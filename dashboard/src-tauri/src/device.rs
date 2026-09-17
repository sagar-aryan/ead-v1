//! Device manager: owns the link, performs the handshake, keeps the connection
//! alive, and repairs telemetry gaps by backfill (`docs/protocol.md` §7).
//!
//! One instance per app. The UI reads `snapshot()`; decoded telemetry is handed
//! to the `Sink` the caller provides.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::link::{self, LinkEvent, LinkTarget};
use crate::protocol::{self, config::DeviceConfigSection, Hello, MsgType, RawFrame, Status};

const KEEPALIVE: Duration = Duration::from_millis(500);
/// No STATUS for this long means the device or link is unhealthy.
const STALE_AFTER: Duration = Duration::from_secs(2);
const RECONNECT_MIN: Duration = Duration::from_millis(500);
const RECONNECT_MAX: Duration = Duration::from_secs(5);
/// Bound on a single backfill request, so one gap cannot monopolise the link.
const MAX_BACKFILL_SPAN: u32 = 2000;

/// Where decoded telemetry goes. Implemented by the session store (M2) and by
/// tests.
pub trait Sink: Send + Sync + 'static {
    fn raw_frames(&self, frames: &[RawFrame]);
    fn status(&self, status: &Status);
    /// Gait cycles and events, whichever the device sent (schema 3).
    fn gait(&self, cycles: &[protocol::GaitCycle], events: &[protocol::GaitEvent]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Disconnected,
    Connecting,
    Connected,
}

/// What the UI needs to render the connection and device state.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Snapshot {
    pub link_state: LinkState,
    pub link_description: Option<String>,
    pub last_error: Option<String>,
    /// Device state name, or "disconnected" when there is no live link.
    pub device_state: &'static str,
    pub faults: Vec<&'static str>,
    pub status: Option<Status>,
    pub firmware: Option<String>,
    pub mac: Option<String>,
    pub haptics_fitted: bool,
    pub capabilities: Vec<&'static str>,
    pub boot_id: Option<u32>,
    pub schema_mismatch: bool,
    /// Durable messages the host has not yet recovered.
    pub missing_messages: u32,
    /// Corrupted byte runs on the link, cumulative (USB only).
    pub rejected_frames: u64,
    pub frames_received: u64,
    pub status_age_ms: Option<u64>,
    /// Latest calibration record the device has reported, if any.
    pub calibration: Option<protocol::Calibration>,
    /// Why that record was rejected, in words; empty when it is usable.
    pub calibration_rejections: Vec<&'static str>,
}

#[derive(Default)]
struct State {
    link_state: Option<LinkState>,
    link_description: Option<String>,
    last_error: Option<String>,
    hello: Option<Hello>,
    status: Option<Status>,
    status_at: Option<Instant>,
    calibration: Option<protocol::Calibration>,
    /// The profile from the last completed capture, until it is saved.
    reference: Option<protocol::ReferenceProfile>,
    frames_received: u64,
    missing_messages: u32,
    rejected_frames: u64,
    schema_mismatch: bool,
    config: Option<Arc<DeviceConfigSection>>,
    config_sha256: Option<[u8; 32]>,
    config_section: Option<Vec<u8>>,
}

pub struct Device {
    state: Arc<Mutex<State>>,
    commands: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    sink: Arc<dyn Sink>,
}

impl Device {
    pub fn new(sink: Arc<dyn Sink>) -> Self {
        Self { state: Arc::new(Mutex::new(State::default())), commands: Mutex::new(None), sink }
    }

    pub fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().expect("device state");
        let link_state = state.link_state.unwrap_or(LinkState::Disconnected);
        let connected = link_state == LinkState::Connected;
        let status = if connected { state.status } else { None };
        Snapshot {
            link_state,
            link_description: state.link_description.clone(),
            last_error: state.last_error.clone(),
            device_state: match status {
                Some(s) => protocol::DEVICE_STATES
                    .get(s.device_state as usize)
                    .copied()
                    .unwrap_or("unknown"),
                None => "disconnected",
            },
            faults: status.map(|s| protocol::fault_names(s.faults)).unwrap_or_default(),
            status,
            firmware: state.hello.as_ref().map(|h| h.fw_version.clone()),
            mac: state.hello.as_ref().map(|h| h.mac_string()),
            haptics_fitted: state.hello.as_ref().is_some_and(|h| h.haptics_fitted()),
            capabilities: state
                .hello
                .as_ref()
                .map(|h| h.capability_names())
                .unwrap_or_default(),
            boot_id: state.hello.as_ref().map(|h| h.boot_id),
            schema_mismatch: state.schema_mismatch,
            missing_messages: state.missing_messages,
            rejected_frames: state.rejected_frames,
            frames_received: state.frames_received,
            status_age_ms: if connected {
                state.status_at.map(|at| at.elapsed().as_millis() as u64)
            } else {
                None
            },
            calibration: if connected { state.calibration } else { None },
            calibration_rejections: match state.calibration {
                Some(record) if connected && !record.usable() => {
                    protocol::calibration_rejections(record.reject)
                }
                _ => Vec::new(),
            },
        }
    }

    /// The device's own configuration, once CONFIG_GET has been answered.
    /// Asks the device to start a still window. The record arrives later as a
    /// SESSION_STOP message; progress is visible in STATUS meanwhile.
    pub fn start_calibration(&self, duration_ms: u16) -> Result<(), String> {
        let payload = protocol::session_start(protocol::SESSION_KIND_CALIBRATION, duration_ms);
        self.send_now(MsgType::SessionStart, &payload)
    }

    /// Cancels a running window; the partial record is discarded.
    pub fn cancel_calibration(&self) -> Result<(), String> {
        self.send_now(MsgType::SessionStop, &[])
    }

    /// Starts collecting valid cycles for a new reference profile.
    pub fn start_reference_capture(&self) -> Result<(), String> {
        let payload = protocol::session_start(protocol::SESSION_KIND_REFERENCE_CAPTURE, 0);
        self.send_now(MsgType::SessionStart, &payload)
    }

    /// Starts a check or an evaluation against a locked profile. The profile
    /// travels with the command, so the device judges against the one the
    /// operator chose rather than whatever it saw last.
    pub fn start_scored_session(
        &self,
        check: bool,
        profile: &protocol::ReferenceProfile,
    ) -> Result<(), String> {
        let kind = if check {
            protocol::SESSION_KIND_REFERENCE_CHECK
        } else {
            protocol::SESSION_KIND_EVALUATION
        };
        let payload = protocol::session_start_with_reference(kind, profile);
        self.send_now(MsgType::SessionStart, &payload)
    }

    /// Ends the running session. A capture answers with its profile, which
    /// arrives later and is collected with `take_reference`.
    pub fn stop_session(&self) -> Result<(), String> {
        self.send_now(MsgType::SessionStop, &[])
    }

    /// Takes the profile from the last completed capture, clearing it.
    pub fn take_reference(&self) -> Option<protocol::ReferenceProfile> {
        self.state.lock().expect("device state").reference.take()
    }

    /// Queues one command on the link task. Commands carry sequence 0: only
    /// backfill and HELLO need a sequence the device echoes.
    fn send_now(&self, msg_type: MsgType, payload: &[u8]) -> Result<(), String> {
        let commands = self.commands.lock().expect("commands");
        let Some(sender) = commands.as_ref() else {
            return Err("not connected".into());
        };
        sender
            .try_send(protocol::encode(msg_type, 0, 0, payload))
            .map_err(|_| "the device link is busy".to_string())
    }

    pub fn config(&self) -> Option<Arc<DeviceConfigSection>> {
        self.state.lock().expect("device state").config.clone()
    }

    /// Hash of the configuration section, recorded with every session (doc 18).
    pub fn config_sha256(&self) -> Option<[u8; 32]> {
        self.state.lock().expect("device state").config_sha256
    }

    /// The configuration section exactly as the device sent it, stored with a
    /// session so the recording describes the device that produced it.
    pub fn config_section(&self) -> Option<Vec<u8>> {
        self.state.lock().expect("device state").config_section.clone()
    }

    /// True while the device is connected and reporting.
    pub fn is_live(&self) -> bool {
        let state = self.state.lock().expect("device state");
        state.link_state == Some(LinkState::Connected)
            && state.status_at.is_some_and(|at| at.elapsed() < STALE_AFTER)
    }
}

/// Connects and keeps reconnecting until `stop` is triggered.
pub async fn run(device: Arc<Device>, target: LinkTarget, mut stop: tokio::sync::watch::Receiver<bool>) {
    let mut backoff = RECONNECT_MIN;
    loop {
        if *stop.borrow() {
            break;
        }
        {
            let mut state = device.state.lock().expect("device state");
            state.link_state = Some(LinkState::Connecting);
            state.link_description = Some(target.describe());
        }

        let (cmd_tx, cmd_rx) = mpsc::channel::<Vec<u8>>(64);
        let (ev_tx, ev_rx) = mpsc::channel::<LinkEvent>(256);
        *device.commands.lock().expect("commands") = Some(cmd_tx.clone());

        let link = tokio::spawn(link::run(target.clone(), cmd_rx, ev_tx));
        let clean = session(&device, ev_rx, cmd_tx, &mut stop).await;
        link.abort();
        let _ = link.await;

        *device.commands.lock().expect("commands") = None;
        {
            let mut state = device.state.lock().expect("device state");
            state.link_state = Some(LinkState::Disconnected);
            state.status = None;
            state.status_at = None;
        }
        if *stop.borrow() {
            break;
        }
        backoff = if clean { RECONNECT_MIN } else { (backoff * 2).min(RECONNECT_MAX) };
        tokio::select! {
            _ = tokio::time::sleep(backoff) => {}
            _ = stop.changed() => {}
        }
    }
    let mut state = device.state.lock().expect("device state");
    state.link_state = Some(LinkState::Disconnected);
}

/// Drives one connection. Returns true when it ran long enough to count as a
/// healthy session (so the backoff resets).
async fn session(
    device: &Arc<Device>,
    mut events: mpsc::Receiver<LinkEvent>,
    commands: mpsc::Sender<Vec<u8>>,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> bool {
    let mut tracker = Tracker::new(device.sink.clone(), device.state.clone());
    let mut command_seq: u32 = 0;
    let mut keepalive = tokio::time::interval(KEEPALIVE);
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let started = Instant::now();
    let mut connected = false;

    loop {
        tokio::select! {
            _ = stop.changed() => return true,
            _ = keepalive.tick(), if connected => {
                command_seq += 1;
                // STATUS with an empty payload is the host keepalive.
                if send(&commands, MsgType::Status, command_seq, &[]).await.is_err() {
                    return false;
                }
                if device.config().is_none() && device.snapshot().firmware.is_some() {
                    command_seq += 1;
                    if send(&commands, MsgType::ConfigGet, command_seq, &[]).await.is_err() {
                        return false;
                    }
                }
                for (first, last) in tracker.take_backfill_requests() {
                    command_seq += 1;
                    let payload = protocol::backfill_request(first, last);
                    if send(&commands, MsgType::BackfillRequest, command_seq, &payload)
                        .await
                        .is_err()
                    {
                        return false;
                    }
                }
            }
            event = events.recv() => match event {
                Some(LinkEvent::Connected { description }) => {
                    connected = true;
                    {
                        let mut state = device.state.lock().expect("device state");
                        state.link_state = Some(LinkState::Connected);
                        state.link_description = Some(description);
                        state.last_error = None;
                    }
                    command_seq += 1;
                    if send(&commands, MsgType::Hello, command_seq, &protocol::hello_request())
                        .await
                        .is_err()
                    {
                        return false;
                    }
                }
                Some(LinkEvent::Message(bytes)) => tracker.handle(&bytes),
                Some(LinkEvent::RejectedFrames(count)) => {
                    device.state.lock().expect("device state").rejected_frames = count;
                }
                Some(LinkEvent::Disconnected { reason }) => {
                    let mut state = device.state.lock().expect("device state");
                    state.last_error = Some(reason);
                    return started.elapsed() > Duration::from_secs(10);
                }
                None => return started.elapsed() > Duration::from_secs(10),
            },
        }
    }
}

async fn send(
    commands: &mpsc::Sender<Vec<u8>>,
    msg_type: MsgType,
    sequence: u32,
    payload: &[u8],
) -> Result<(), ()> {
    let msg = protocol::encode(msg_type, sequence, 0, payload);
    // Host time is never substituted for device time (doc 08 §6), hence 0.
    commands.send(msg).await.map_err(|_| ())
}

/// Decodes messages, tracks durable sequence numbers, and feeds the sink.
struct Tracker {
    sink: Arc<dyn Sink>,
    state: Arc<Mutex<State>>,
    boot_id: Option<u32>,
    highest_seq: Option<u32>,
    missing: std::collections::BTreeSet<u32>,
    oldest_stored: u32,
}

impl Tracker {
    fn new(sink: Arc<dyn Sink>, state: Arc<Mutex<State>>) -> Self {
        Self {
            sink,
            state,
            boot_id: None,
            highest_seq: None,
            missing: std::collections::BTreeSet::new(),
            oldest_stored: 0,
        }
    }

    fn handle(&mut self, bytes: &[u8]) {
        let Ok((header, payload)) = protocol::parse(bytes) else { return };
        let Some(msg_type) = MsgType::from_u8(header.msg_type) else { return };
        match msg_type {
            MsgType::Hello => self.on_hello(payload),
            MsgType::Status => self.on_status(payload),
            MsgType::EventBatch => match protocol::parse_event_batch(payload) {
                Ok(events) => self.sink.gait(&[], &events),
                Err(e) => self.note_error(format!("EVENT_BATCH: {e}")),
            },
            MsgType::StepBatch => match protocol::parse_step_batch(payload) {
                Ok(cycles) => self.sink.gait(&cycles, &[]),
                Err(e) => self.note_error(format!("STEP_BATCH: {e}")),
            },
            MsgType::ConfigGet => self.on_config(payload),
            MsgType::SessionStop => {
                // Two things arrive here, told apart by length: a calibration
                // record when a still window completes, and a reference profile
                // when a capture ends.
                if payload.len() == protocol::REFERENCE_PAYLOAD_SIZE {
                    match protocol::parse_reference(payload) {
                        Ok(profile) => {
                            self.state.lock().expect("device state").reference = Some(profile)
                        }
                        Err(e) => self.note_error(format!("reference profile: {e}")),
                    }
                } else {
                    match protocol::parse_calibration(payload) {
                        Ok(record) => {
                            self.state.lock().expect("device state").calibration = Some(record)
                        }
                        Err(e) => self.note_error(format!("calibration record: {e}")),
                    }
                }
            }
            MsgType::BackfillData => {
                if let Ok(chunk) = protocol::parse_backfill_data(payload) {
                    for message in chunk.messages {
                        // Backfilled messages are originals; decode them the same way.
                        if let Ok((inner, inner_payload)) = protocol::parse(&message) {
                            self.on_durable(inner.sequence, inner.msg_type, inner_payload);
                        }
                    }
                }
            }
            MsgType::Error => {
                if let Ok(error) = protocol::parse_device_error(payload) {
                    let mut state = self.state.lock().expect("device state");
                    state.last_error = Some(error.to_string());
                }
            }
            other if other.is_durable() => self.on_durable(header.sequence, header.msg_type, payload),
            _ => {}
        }
    }

    fn on_hello(&mut self, payload: &[u8]) {
        let Ok(hello) = protocol::parse_hello(payload) else { return };
        // A new boot_id means the device restarted: sequence numbers restarted
        // with it, and nothing from the previous boot can be backfilled.
        if self.boot_id != Some(hello.boot_id) {
            self.boot_id = Some(hello.boot_id);
            self.highest_seq = None;
            self.missing.clear();
            let mut state = self.state.lock().expect("device state");
            state.config = None;
            state.config_sha256 = None;
            state.config_section = None;
        }
        self.oldest_stored = hello.oldest_seq;
        if let Some(highest) = self.highest_seq {
            // Recover whatever the device still holds from before the gap.
            self.note_missing_range(highest + 1, hello.last_seq);
        }
        let mut state = self.state.lock().expect("device state");
        state.schema_mismatch = hello.schema != protocol::SCHEMA_VERSION;
        state.hello = Some(hello);
    }

    fn on_config(&mut self, payload: &[u8]) {
        let Ok(response) = protocol::parse_config(payload) else { return };
        // The section must hash to what the device reported in HELLO, or the
        // configuration recorded with a session would not describe the device.
        use sha2::{Digest, Sha256};
        let digest: [u8; 32] = Sha256::digest(&response.section).into();
        let mut state = self.state.lock().expect("device state");
        if digest != response.sha256 {
            state.last_error = Some("device configuration failed its own hash check".into());
            return;
        }
        if let Some(hello) = state.hello.as_ref() {
            if hello.config_sha256 != digest {
                state.last_error =
                    Some("device configuration does not match the hash in HELLO".into());
                return;
            }
        }
        match protocol::parse_section(&response.section) {
            Ok(config) => {
                state.config = Some(Arc::new(config));
                state.config_sha256 = Some(digest);
                state.config_section = Some(response.section.clone());
            }
            Err(err) => state.last_error = Some(format!("device configuration: {err}")),
        }
    }

    fn note_error(&self, message: String) {
        self.state.lock().expect("device state").last_error = Some(message);
    }

    fn on_status(&mut self, payload: &[u8]) {
        let Ok(status) = protocol::parse_status(payload) else { return };
        self.oldest_stored = status.oldest_seq;
        self.forget_evicted();
        self.sink.status(&status);
        let mut state = self.state.lock().expect("device state");
        state.status = Some(status);
        state.status_at = Some(Instant::now());
        state.missing_messages = self.missing.len() as u32;
    }

    fn on_durable(&mut self, sequence: u32, msg_type: u8, payload: &[u8]) {
        match self.highest_seq {
            Some(highest) if sequence > highest + 1 => self.note_missing_range(highest + 1, sequence - 1),
            _ => {}
        }
        self.missing.remove(&sequence);
        self.highest_seq = Some(self.highest_seq.map_or(sequence, |h| h.max(sequence)));

        if msg_type == MsgType::RawSampleBatch as u8 {
            if let Ok(frames) = protocol::parse_raw_batch(payload) {
                let mut state = self.state.lock().expect("device state");
                state.frames_received += frames.len() as u64;
                state.missing_messages = self.missing.len() as u32;
                drop(state);
                self.sink.raw_frames(&frames);
            }
        }
    }

    fn note_missing_range(&mut self, first: u32, last: u32) {
        // The device ring holds ~12 minutes; a gap larger than this cap is
        // unrecoverable anyway, and the bound keeps the set small.
        const MAX_TRACKED: usize = 200_000;
        for sequence in first..=last {
            if self.missing.len() >= MAX_TRACKED {
                break;
            }
            self.missing.insert(sequence);
        }
        self.forget_evicted();
    }

    /// Messages the device no longer stores can never arrive; stop asking.
    fn forget_evicted(&mut self) {
        if self.oldest_stored > 0 {
            self.missing = self.missing.split_off(&self.oldest_stored);
        }
    }

    /// Contiguous ranges to request, oldest first.
    fn take_backfill_requests(&mut self) -> Vec<(u32, u32)> {
        self.forget_evicted();
        let mut ranges = Vec::new();
        let mut iter = self.missing.iter().copied();
        let Some(mut first) = iter.next() else { return ranges };
        let mut last = first;
        for sequence in iter {
            if sequence == last + 1 && last - first + 1 < MAX_BACKFILL_SPAN {
                last = sequence;
                continue;
            }
            ranges.push((first, last));
            if ranges.len() == 4 {
                return ranges; // one request per keepalive tick is enough
            }
            first = sequence;
            last = sequence;
        }
        ranges.push((first, last));
        ranges
    }
}
