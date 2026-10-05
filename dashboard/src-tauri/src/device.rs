//! Device manager: owns the link, performs the handshake, keeps the connection
//! alive, and repairs telemetry gaps by backfill (`docs/protocol.md` §7).
//!
//! One instance per app. The UI reads `snapshot()`; decoded telemetry is handed
//! to the `Sink` the caller provides.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::link::{self, LinkEvent, LinkTarget};
use crate::protocol::{
    self, config::DeviceConfigSection, AccelSample, Hello, MsgType, RawFrame, Status,
};

const KEEPALIVE: Duration = Duration::from_millis(500);
/// No STATUS for this long means the device or link is unhealthy.
const STALE_AFTER: Duration = Duration::from_secs(2);
const RECONNECT_MIN: Duration = Duration::from_millis(500);
const RECONNECT_MAX: Duration = Duration::from_secs(5);
/// Bound on a single backfill request, so one gap cannot monopolise the link.
const MAX_BACKFILL_SPAN: u32 = 2000;
/// How long a session command waits for the device's ACK or ERROR.
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);
/// Numbers for commands that expect a reply, apart from the link task's own
/// (keepalive, HELLO, backfill), which count up from 1.
const FIRST_COMMAND_SEQ: u32 = 0x8000_0000;
/// How long the device must report a session no recording owns before it is
/// stopped: longer than a STATUS sent before a stop takes to arrive.
const ORPHAN_AFTER: Duration = Duration::from_secs(1);

/// Where decoded telemetry goes. Implemented by the session store (M2) and by
/// tests.
pub trait Sink: Send + Sync + 'static {
    fn raw_frames(&self, frames: &[RawFrame]);
    /// Every accelerometer sample at its native rate (schema 6).
    fn raw_accel(&self, samples: &[AccelSample]);
    fn status(&self, status: &Status);
    /// Gait cycles and events, whichever the device sent (schema 3).
    fn gait(&self, cycles: &[protocol::GaitCycle], events: &[protocol::GaitEvent]);
    /// Feedback cues and episode ends (schema 7, DEC-023).
    fn haptics(&self, _records: &[protocol::HapticRecord]) {}
    /// The device restarted (a new boot_id after an earlier one). Returns what
    /// the operator must be told, if anything (PROB-019).
    fn device_restarted(&self) -> Option<String> {
        None
    }
    /// Whether a recording is open to receive what a device session produces.
    /// A session nobody records (the app was closed or crashed while it ran) is
    /// stopped (PROB-033).
    fn owns_session(&self) -> bool {
        true
    }
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
    /// The payload schema the device declared in HELLO.
    pub schema: Option<u16>,
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
    /// The session kind this host last started, by name; null when none.
    pub session_kind: Option<&'static str>,
    /// Valid cycles since that session started.
    pub session_valid_cycles: u32,
    /// The device can pulse its motors for the service test (DEC-018).
    pub motor_service_test: bool,
    /// The latest sensor check from this boot, when one has been asked for.
    pub sensor_check: Option<protocol::SensorCheckReport>,
    /// The haptic master switch as the device reports it (off at every boot).
    pub haptic_switch_on: bool,
    /// A feedback episode is running on the device.
    pub haptic_episode: bool,
    /// The latest cycle the device reported on this link, valid or not.
    pub last_cycle: Option<protocol::GaitCycle>,
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
    /// The session this host started and the device acknowledged. STATUS
    /// `session` (schema 8) is the device's own account.
    session_kind: Option<u8>,
    /// Valid cycles seen since the session started. During a capture this is
    /// the same rule the device's builder applies, so it tracks the count the
    /// thirty-cycle gate will use — up to the builder's sixty-four cycle ring.
    session_valid_cycles: u32,
    sensor_check: Option<protocol::SensorCheckReport>,
    /// The latest cycle the device reported, for LIVE (doc 11).
    last_cycle: Option<protocol::GaitCycle>,
    /// SERVICE_TEST replies received, so a command can tell its own from older.
    service_replies: u64,
    frames_received: u64,
    missing_messages: u32,
    rejected_frames: u64,
    schema_mismatch: bool,
    config: Option<Arc<DeviceConfigSection>>,
    config_sha256: Option<[u8; 32]>,
    config_section: Option<protocol::ConfigSection>,
    /// Answers to numbered commands, oldest first: the ACK's boundary sequence or
    /// the ERROR's text (schema 8).
    replies: std::collections::VecDeque<(u32, std::result::Result<u32, String>)>,
    /// Every durable message up to this sequence has arrived (or is gone).
    received_through: u32,
    /// Messages up to this sequence belonged to a recording already closed; any
    /// that turn up late are not handed on (PROB-033).
    floor: Option<u32>,
    /// Since when STATUS has shown a device session no recording owns.
    orphan_since: Option<Instant>,
    /// A stop for that session is due on the link task.
    stop_orphan: bool,
}

pub struct Device {
    state: Arc<Mutex<State>>,
    commands: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    sink: Arc<dyn Sink>,
    /// The last connection's receive progress, between connections.
    tracker: Mutex<Option<Tracker>>,
    next_command: std::sync::atomic::AtomicU32,
}

impl Device {
    pub fn new(sink: Arc<dyn Sink>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            commands: Mutex::new(None),
            sink,
            tracker: Mutex::new(None),
            next_command: std::sync::atomic::AtomicU32::new(FIRST_COMMAND_SEQ),
        }
    }

    /// The tracker a connection uses: the previous connection's, so a reconnect
    /// to the same boot knows what it already has and asks for what it missed.
    /// Until 2026-10-05 every connection started a new one, and the device's
    /// three-minute history was never asked for after a dropout (PROB-030).
    fn take_tracker(&self) -> Tracker {
        self.tracker
            .lock()
            .expect("tracker")
            .take()
            .unwrap_or_else(|| Tracker::new(self.sink.clone(), self.state.clone()))
    }

    fn keep_tracker(&self, tracker: Tracker) {
        *self.tracker.lock().expect("tracker") = Some(tracker);
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
            schema: state.hello.as_ref().map(|h| h.schema),
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
            session_kind: if connected {
                state.session_kind.and_then(|k| protocol::SESSION_KINDS.get(k as usize).copied())
            } else {
                None
            },
            session_valid_cycles: state.session_valid_cycles,
            motor_service_test: state.hello.as_ref().is_some_and(|h| h.motor_service_test()),
            sensor_check: if connected { state.sensor_check.clone() } else { None },
            haptic_switch_on: status.is_some_and(|s| s.haptics & protocol::STATUS_HAPTICS_SWITCH_ON != 0),
            haptic_episode: status.is_some_and(|s| s.haptics & protocol::STATUS_HAPTICS_EPISODE != 0),
            last_cycle: if connected { state.last_cycle } else { None },
        }
    }

    /// The device's own configuration, once CONFIG_GET has been answered.
    /// Asks the device to start a still window. The record arrives later as a
    /// SESSION_STOP message; progress is visible in STATUS meanwhile.
    pub async fn start_calibration(&self, duration_ms: u16) -> Result<(), String> {
        let payload = protocol::session_start(protocol::SESSION_KIND_CALIBRATION, duration_ms);
        self.command(MsgType::SessionStart, &payload).await.map(|_| ())
    }

    /// Cancels a running window; the partial record is discarded.
    pub async fn cancel_calibration(&self) -> Result<(), String> {
        self.command(MsgType::SessionStop, &[]).await.map(|_| ())
    }

    /// Starts collecting valid cycles for a new reference profile, once the
    /// device has accepted it.
    pub async fn start_reference_capture(&self) -> Result<(), String> {
        let payload = protocol::session_start(protocol::SESSION_KIND_REFERENCE_CAPTURE, 0);
        self.command(MsgType::SessionStart, &payload).await?;
        self.note_session(Some(protocol::SESSION_KIND_REFERENCE_CAPTURE));
        Ok(())
    }

    /// Records which session the host started, and restarts the cycle count.
    fn note_session(&self, kind: Option<u8>) {
        let mut state = self.state.lock().expect("device state");
        state.session_kind = kind;
        state.session_valid_cycles = 0;
    }

    /// Starts a check or an evaluation against a locked profile. The profile
    /// travels with the command, so the device judges against the one the
    /// operator chose rather than whatever it saw last.
    pub async fn start_scored_session(
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
        self.command(MsgType::SessionStart, &payload).await?;
        self.note_session(Some(kind));
        Ok(())
    }

    /// Ends the running session. Returns the boundary: the last durable message
    /// the session produced. A capture's profile follows, collected with
    /// `take_reference`.
    pub async fn stop_session(&self) -> Result<u32, String> {
        let boundary = self.command(MsgType::SessionStop, &[]).await;
        self.note_session(None);
        boundary
    }

    /// Stops the device session from a thread that cannot await, as the app
    /// closes: the device would otherwise go on scoring, and vibrating, with
    /// nothing recording it (PROB-033). Waits up to `timeout` for the ACK.
    pub fn stop_session_blocking(&self, timeout: Duration) -> Result<(), String> {
        let seq = self.send_command(MsgType::SessionStop, &[])?;
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(reply) = self.take_reply(seq) {
                return reply.map(|_| ());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err("the device did not confirm the stop".into())
    }

    /// Waits until every durable message up to `boundary` has arrived, or
    /// `timeout`; true when they all did. Then closes the stream there: anything
    /// up to it that arrives later is not handed on, so it cannot land in the
    /// next recording (PROB-033).
    pub async fn drain_through(&self, boundary: u32, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let drained = loop {
            if self.state.lock().expect("device state").received_through >= boundary {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        let mut state = self.state.lock().expect("device state");
        state.floor = Some(state.floor.map_or(boundary, |f| f.max(boundary)));
        drained
    }

    /// Sends a session command and waits for the device's answer: the boundary
    /// sequence of its ACK, or its ERROR as the error. Until schema 8 the device
    /// answered only refusals, and 800 ms of silence was taken as acceptance
    /// (PROB-033).
    async fn command(&self, msg_type: MsgType, payload: &[u8]) -> Result<u32, String> {
        let seq = self.send_command(msg_type, payload)?;
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            if let Some(reply) = self.take_reply(seq) {
                return reply;
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "the device did not answer within {} s",
                    REPLY_TIMEOUT.as_secs()
                ));
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    fn take_reply(&self, seq: u32) -> Option<std::result::Result<u32, String>> {
        let mut state = self.state.lock().expect("device state");
        let at = state.replies.iter().position(|(s, _)| *s == seq)?;
        state.replies.remove(at).map(|(_, reply)| reply)
    }

    /// Asks for the sensor check. With `rerun` the device resets both sensors
    /// and checks every line again, which stops frames for about two seconds.
    /// Returns the reply count to wait past.
    pub fn request_sensor_check(&self, rerun: bool) -> Result<u64, String> {
        self.request_service(&protocol::sensor_check_request(rerun))
    }

    /// Asks for one motor pulse; the device ends it by itself.
    pub fn request_motor_pulse(&self, pulse: &protocol::MotorPulse) -> Result<u64, String> {
        self.request_service(&protocol::motor_pulse_request(pulse))
    }

    fn request_service(&self, payload: &[u8]) -> Result<u64, String> {
        let seen = self.service_replies();
        self.clear_error();
        self.send_now(MsgType::ServiceTest, payload)?;
        Ok(seen)
    }

    /// The haptic master switch (CONFIG_SET, DEC-023). The device echoes it and
    /// reports it in STATUS; a refusal arrives as the last error.
    pub fn set_haptic_feedback(&self, on: bool) -> Result<(), String> {
        self.clear_error();
        self.send_now(MsgType::ConfigSet, &protocol::haptic_switch_request(on))
    }

    pub fn service_replies(&self) -> u64 {
        self.state.lock().expect("device state").service_replies
    }

    pub fn sensor_check(&self) -> Option<protocol::SensorCheckReport> {
        self.state.lock().expect("device state").sensor_check.clone()
    }

    /// Clears the last error, so that one arriving next can be attributed to
    /// the command about to be sent.
    pub fn clear_error(&self) {
        self.state.lock().expect("device state").last_error = None;
    }

    pub fn last_error(&self) -> Option<String> {
        self.state.lock().expect("device state").last_error.clone()
    }

    /// Takes the profile from the last completed capture, clearing it.
    pub fn take_reference(&self) -> Option<protocol::ReferenceProfile> {
        self.state.lock().expect("device state").reference.take()
    }

    /// Queues one command on the link task.
    fn send_now(&self, msg_type: MsgType, payload: &[u8]) -> Result<(), String> {
        self.send_command(msg_type, payload).map(|_| ())
    }

    /// Queues one command under its own sequence number, which the device
    /// echoes in an ACK or ERROR; returns the number.
    fn send_command(&self, msg_type: MsgType, payload: &[u8]) -> Result<u32, String> {
        let seq = self.next_command.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let commands = self.commands.lock().expect("commands");
        let Some(sender) = commands.as_ref() else {
            return Err("not connected".into());
        };
        sender
            .try_send(protocol::encode(msg_type, seq, 0, payload))
            .map_err(|_| "the device link is busy".to_string())?;
        Ok(seq)
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
    pub fn config_section(&self) -> Option<protocol::ConfigSection> {
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
    events: mpsc::Receiver<LinkEvent>,
    commands: mpsc::Sender<Vec<u8>>,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> bool {
    let mut tracker = device.take_tracker();
    let healthy = connection(device, &mut tracker, events, commands, stop).await;
    device.keep_tracker(tracker);
    healthy
}

async fn connection(
    device: &Arc<Device>,
    tracker: &mut Tracker,
    mut events: mpsc::Receiver<LinkEvent>,
    commands: mpsc::Sender<Vec<u8>>,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> bool {
    tracker.hello_seen = false;
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
                if std::mem::take(&mut device.state.lock().expect("device state").stop_orphan) {
                    command_seq += 1;
                    if send(&commands, MsgType::SessionStop, command_seq, &[]).await.is_err() {
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
    /// This connection's HELLO has been read. The tracker outlives connections,
    /// and until HELLO says which boot this is, its missing set may belong to a
    /// boot that no longer exists: asking a reset device for it got
    /// BackfillUnavailable (TEST-065).
    hello_seen: bool,
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
            hello_seen: false,
        }
    }

    fn handle(&mut self, bytes: &[u8]) {
        let Ok((header, payload)) = protocol::parse(bytes) else { return };
        let Some(msg_type) = MsgType::from_u8(header.msg_type) else { return };
        match msg_type {
            MsgType::Hello => self.on_hello(payload),
            MsgType::Status => self.on_status(payload),
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
            MsgType::ServiceTest => match protocol::parse_service_test(payload) {
                Ok(reply) => {
                    let mut state = self.state.lock().expect("device state");
                    // A pulse reply only says it was accepted: the count is enough.
                    if let protocol::ServiceReply::SensorCheck(report) = reply {
                        state.sensor_check = Some(report);
                    }
                    state.service_replies += 1;
                }
                Err(e) => self.note_error(format!("SERVICE_TEST: {e}")),
            },
            MsgType::ConfigSet => {
                // The echo only confirms; STATUS carries the switch from here on.
                if let Err(e) = protocol::parse_haptic_switch(payload) {
                    self.note_error(format!("CONFIG_SET: {e}"));
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
            MsgType::Ack => match protocol::parse_ack(payload) {
                Ok(ack) => self.reply(ack.cmd_seq, Ok(header.sequence)),
                Err(e) => self.note_error(format!("ACK: {e}")),
            },
            MsgType::Error => {
                if let Ok(error) = protocol::parse_device_error(payload) {
                    if error.cmd_seq != 0 {
                        self.reply(error.cmd_seq, Err(error.to_string()));
                    }
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
        self.hello_seen = true;
        // A new boot_id means the device restarted: sequence numbers restarted
        // with it, and nothing from the previous boot can be backfilled.
        if self.boot_id != Some(hello.boot_id) {
            self.boot_id = Some(hello.boot_id);
            self.highest_seq = None;
            self.missing.clear();
            let mut state = self.state.lock().expect("device state");
            state.received_through = 0;
            state.floor = None;
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
        // The device keeps its calibration in RAM only, so a new boot has none,
        // whatever this host heard before (PROB-018). Compared with the last
        // HELLO rather than `self.boot_id`, which a first connection starts
        // without: a replug of the same boot keeps its calibration.
        let previous = state.hello.as_ref().map(|h| h.boot_id);
        let restarted = previous.is_some() && previous != Some(hello.boot_id);
        if previous != Some(hello.boot_id) {
            state.calibration = None;
            state.sensor_check = None;  // a check describes the boot it ran in
        }
        if restarted {
            // Whatever session this host started ended with the old boot.
            state.session_kind = None;
            state.session_valid_cycles = 0;
        }
        state.schema_mismatch = hello.schema != protocol::SCHEMA_VERSION;
        state.hello = Some(hello);
        drop(state);
        if restarted {
            if let Some(notice) = self.sink.device_restarted() {
                self.note_error(notice);
            }
        }
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
        let section =
            protocol::ConfigSection { format: response.format, bytes: response.section.clone() };
        match section.parse() {
            Ok(config) => {
                state.config = Some(Arc::new(config));
                state.config_sha256 = Some(digest);
                state.config_section = Some(section);
            }
            Err(err) => state.last_error = Some(format!("device configuration: {err}")),
        }
    }

    fn reply(&self, cmd_seq: u32, reply: std::result::Result<u32, String>) {
        let mut state = self.state.lock().expect("device state");
        // Unclaimed answers (a timed-out wait, a capture's refusal after its
        // ACK) would otherwise pile up.
        if state.replies.len() == 16 {
            state.replies.pop_front();
        }
        state.replies.push_back((cmd_seq, reply));
    }

    fn note_error(&self, message: String) {
        self.state.lock().expect("device state").last_error = Some(message);
    }

    fn on_status(&mut self, payload: &[u8]) {
        let Ok(status) = protocol::parse_status(payload) else { return };
        self.oldest_stored = status.oldest_seq;
        self.forget_evicted();
        self.sink.status(&status);
        let orphaned = status.session != 0 && !self.sink.owns_session();
        let mut state = self.state.lock().expect("device state");
        if !orphaned {
            state.orphan_since = None;
        } else if state.orphan_since.is_none() {
            state.orphan_since = Some(Instant::now());
        } else if state.orphan_since.is_some_and(|t| t.elapsed() >= ORPHAN_AFTER) {
            state.orphan_since = None;
            state.stop_orphan = true;
            state.last_error = Some(format!(
                "the device was running a {} that no recording owns, and has been stopped",
                protocol::SESSION_KINDS.get(status.session as usize).copied().unwrap_or("session")
            ));
        }
        state.status = Some(status);
        state.status_at = Some(Instant::now());
        state.missing_messages = self.missing.len() as u32;
    }

    fn on_durable(&mut self, sequence: u32, msg_type: u8, payload: &[u8]) {
        match self.highest_seq {
            Some(highest) if sequence > highest + 1 => self.note_missing_range(highest + 1, sequence - 1),
            // Already received: a backfill can repeat what the live stream
            // delivered. Handing it on again counted its cycles twice toward a
            // segment's limits (PROB-031); the store's OR IGNORE only covers rows.
            Some(highest) if sequence <= highest && !self.missing.contains(&sequence) => return,
            _ => {}
        }
        self.missing.remove(&sequence);
        self.highest_seq = Some(self.highest_seq.map_or(sequence, |h| h.max(sequence)));
        {
            let mut state = self.state.lock().expect("device state");
            let highest = self.highest_seq.unwrap_or(0);
            state.received_through = self.missing.first().map_or(highest, |m| m - 1);
            if state.floor.is_some_and(|floor| sequence <= floor) {
                return;
            }
        }

        if msg_type == MsgType::RawSampleBatch as u8 {
            if let Ok(frames) = protocol::parse_raw_batch(payload) {
                let mut state = self.state.lock().expect("device state");
                state.frames_received += frames.len() as u64;
                state.missing_messages = self.missing.len() as u32;
                drop(state);
                self.sink.raw_frames(&frames);
            }
        } else if msg_type == MsgType::RawAccelBatch as u8 {
            match protocol::parse_accel_batch(payload) {
                Ok(samples) => self.sink.raw_accel(&samples),
                Err(e) => self.note_error(format!("RAW_ACCEL_BATCH: {e}")),
            }
        } else if msg_type == MsgType::EventBatch as u8 {
            // Events and cycles are durable too. Handled ahead of this, they
            // never counted toward gap detection, so each one was requested again,
            // and a backfilled copy was dropped: gait data inside a real gap was lost.
            match protocol::parse_event_batch(payload) {
                Ok(events) => self.sink.gait(&[], &events),
                Err(e) => self.note_error(format!("EVENT_BATCH: {e}")),
            }
        } else if msg_type == MsgType::StepBatch as u8 {
            match protocol::parse_step_batch(payload) {
                Ok(cycles) => {
                    let valid = cycles.iter().filter(|c| c.valid).count() as u32;
                    let mut state = self.state.lock().expect("device state");
                    if valid > 0 && state.session_kind.is_some() {
                        state.session_valid_cycles += valid;
                    }
                    // A backfilled batch is older than what LIVE already shows.
                    if let Some(last) = cycles.last() {
                        if state.last_cycle.is_none_or(|c| last.start_us >= c.start_us) {
                            state.last_cycle = Some(*last);
                        }
                    }
                    drop(state);
                    self.sink.gait(&cycles, &[]);
                }
                Err(e) => self.note_error(format!("STEP_BATCH: {e}")),
            }
        } else if msg_type == MsgType::HapticBatch as u8 {
            match protocol::parse_haptic_batch(payload) {
                Ok(records) => self.sink.haptics(&records),
                Err(e) => self.note_error(format!("HAPTIC_BATCH: {e}")),
            }
        }
    }

    fn note_missing_range(&mut self, first: u32, last: u32) {
        // The device ring holds about 3 minutes (schema 6); a gap larger than
        // this cap is unrecoverable anyway, and the bound keeps the set small.
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
        if !self.hello_seen {
            return Vec::new();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::tests::vector;

    struct NoSink;
    impl Sink for NoSink {
        fn raw_frames(&self, _: &[RawFrame]) {}
        fn raw_accel(&self, _: &[AccelSample]) {}
        fn status(&self, _: &Status) {}
        fn gait(&self, _: &[protocol::GaitCycle], _: &[protocol::GaitEvent]) {}
    }

    fn hello(boot_id: u32) -> Vec<u8> {
        let msg = vector("hello_info.hex");
        let (_, payload) = protocol::parse(&msg).unwrap();
        let mut payload = payload.to_vec();
        payload[4..8].copy_from_slice(&boot_id.to_le_bytes());
        protocol::encode(MsgType::Hello, 1, 0, &payload)
    }

    /// A connection's tracker, as `session()` takes it. Dropped rather than
    /// handed back with `keep_tracker`, the next connection starts a new one.
    fn connect(device: &Device) -> Tracker {
        device.state.lock().unwrap().link_state = Some(LinkState::Connected);
        device.take_tracker()
    }

    /// A HELLO whose device history runs from `oldest` to `last`.
    fn hello_holding(boot_id: u32, oldest: u32, last: u32) -> Vec<u8> {
        let msg = vector("hello_info.hex");
        let (_, payload) = protocol::parse(&msg).unwrap();
        let mut payload = payload.to_vec();
        payload[4..8].copy_from_slice(&boot_id.to_le_bytes());
        payload[49..53].copy_from_slice(&oldest.to_le_bytes());
        payload[53..57].copy_from_slice(&last.to_le_bytes());
        protocol::encode(MsgType::Hello, 1, 0, &payload)
    }

    /// The vector's message under another durable sequence number.
    fn resequenced(name: &str, sequence: u32) -> Vec<u8> {
        let msg = vector(name);
        let (header, payload) = protocol::parse(&msg).unwrap();
        protocol::encode(MsgType::from_u8(header.msg_type).unwrap(), sequence, header.time_us, payload)
    }

    fn backfill(messages: &[Vec<u8>]) -> Vec<u8> {
        let first = protocol::parse(&messages[0]).unwrap().0.sequence;
        let last = protocol::parse(messages.last().unwrap()).unwrap().0.sequence;
        let mut payload = Vec::new();
        payload.extend_from_slice(&11u32.to_le_bytes());
        payload.extend_from_slice(&first.to_le_bytes());
        payload.extend_from_slice(&last.to_le_bytes());
        payload.push(0);
        for m in messages {
            payload.extend_from_slice(m);
        }
        protocol::encode(MsgType::BackfillData, 9, 0, &payload)
    }

    // PROB-030, audit I02: received through 100, dropped, back on the same boot
    // with history through 150. Nothing from 101 to 150 was asked for.
    #[test]
    fn a_reconnect_to_the_same_boot_requests_what_it_missed() {
        let device = Device::new(Arc::new(NoSink));
        let mut tracker = connect(&device);
        tracker.handle(&hello_holding(7, 1, 0));
        for sequence in 1..=100 {
            tracker.handle(&resequenced("raw_batch.hex", sequence));
        }
        assert!(tracker.take_backfill_requests().is_empty());
        device.keep_tracker(tracker);

        let mut tracker = connect(&device);
        tracker.handle(&hello_holding(7, 1, 150));
        assert_eq!(tracker.take_backfill_requests(), vec![(101, 150)]);

        // Another boot: its numbers start again and nothing earlier exists. Until
        // its HELLO is read, the old boot's gap is not asked for either (TEST-065).
        tracker.handle(&resequenced("raw_batch.hex", 200)); // 151-199 now missing
        device.keep_tracker(tracker);
        let mut tracker = connect(&device);
        tracker.hello_seen = false; // as `connection()` starts each one
        assert!(tracker.take_backfill_requests().is_empty());
        tracker.handle(&hello_holding(8, 1, 20));
        assert!(tracker.take_backfill_requests().is_empty());
    }

    fn calibrated(boot_id: u32) -> Device {
        let device = Device::new(Arc::new(NoSink));
        let mut tracker = connect(&device);
        tracker.handle(&hello(boot_id));
        tracker.handle(&vector("calibration_record.hex"));
        assert!(device.snapshot().calibration.is_some_and(|c| c.usable()));
        device
    }

    // PROB-018: evaluations on 2026-09-18 ran on a reflashed device with the
    // previous boot's calibration still accepted by the session gate.
    #[test]
    fn a_device_reboot_discards_the_calibration() {
        let device = calibrated(0xA1B2_C3D4);
        connect(&device).handle(&hello(0x0102_0304));
        assert_eq!(device.snapshot().calibration, None);
    }

    /// Counts restarts, as the store would end a session for each.
    struct RestartSink(std::sync::atomic::AtomicUsize);
    impl Sink for RestartSink {
        fn raw_frames(&self, _: &[RawFrame]) {}
        fn raw_accel(&self, _: &[AccelSample]) {}
        fn status(&self, _: &Status) {}
        fn gait(&self, _: &[protocol::GaitCycle], _: &[protocol::GaitEvent]) {}
        fn device_restarted(&self) -> Option<String> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some("session ended at the restart".into())
        }
    }

    // PROB-019: a reboot ends the host's session; a replug of the same boot,
    // or the first HELLO this host ever sees, does not.
    #[test]
    fn only_a_device_reboot_ends_the_session() {
        let sink = Arc::new(RestartSink(std::sync::atomic::AtomicUsize::new(0)));
        let device = Device::new(sink.clone());
        let restarts = || sink.0.load(std::sync::atomic::Ordering::SeqCst);

        connect(&device).handle(&hello(0xA1B2_C3D4));
        assert_eq!(restarts(), 0);
        device.note_session(Some(protocol::SESSION_KIND_REFERENCE_CAPTURE));

        connect(&device).handle(&hello(0xA1B2_C3D4));
        assert_eq!(restarts(), 0);
        assert!(device.snapshot().session_kind.is_some());

        connect(&device).handle(&hello(0x0102_0304));
        assert_eq!(restarts(), 1);
        let snapshot = device.snapshot();
        assert_eq!(snapshot.session_kind, None);
        assert_eq!(snapshot.last_error.as_deref(), Some("session ended at the restart"));
    }

    /// Keeps what the tracker hands on.
    #[derive(Default)]
    struct Collect(Mutex<(Vec<RawFrame>, Vec<AccelSample>, usize)>);
    impl Sink for Collect {
        fn raw_frames(&self, frames: &[RawFrame]) {
            self.0.lock().unwrap().0.extend_from_slice(frames);
        }
        fn raw_accel(&self, samples: &[AccelSample]) {
            self.0.lock().unwrap().1.extend_from_slice(samples);
        }
        fn status(&self, _: &Status) {}
        fn gait(&self, cycles: &[protocol::GaitCycle], _: &[protocol::GaitEvent]) {
            self.0.lock().unwrap().2 += cycles.len();
        }
    }

    // RAW_ACCEL_BATCH is durable: its sequence counts toward gap detection, and
    // a gap it reveals is repaired by backfill like any other.
    #[test]
    fn accel_batches_are_durable_and_reach_the_sink() {
        let sink = Arc::new(Collect::default());
        let device = Device::new(sink.clone());
        let mut tracker = connect(&device);
        tracker.handle(&hello(0xA1B2_C3D4));
        tracker.handle(&vector("raw_batch.hex")); // sequence 3
        tracker.handle(&vector("raw_accel_batch.hex")); // sequence 5
        assert_eq!(tracker.missing.iter().copied().collect::<Vec<_>>(), vec![4]);
        assert_eq!(tracker.take_backfill_requests(), vec![(4, 4)]);

        tracker.handle(&vector("backfill_data.hex")); // sequences 3 and 4
        assert!(tracker.missing.is_empty());
        let (frames, samples, _) = &*sink.0.lock().unwrap();
        // Sequence 3 live, then 3 again and 4 from the backfill: the repeat of 3
        // is dropped here (PROB-031), only 4 is handed on.
        let indices: Vec<u32> = frames.iter().map(|f| f.frame_index).collect();
        assert_eq!(indices, vec![100, 101, 101]);
        assert_eq!(samples.len(), 3);
        assert_eq!((samples[1].sensor, samples[1].sequence), (1, 255));
    }

    // HAPTIC_BATCH is durable as well: the episode log must survive a gap.
    #[test]
    fn haptic_batches_are_durable_and_reach_the_sink() {
        #[derive(Default)]
        struct Haptics(Mutex<Vec<protocol::HapticRecord>>);
        impl Sink for Haptics {
            fn raw_frames(&self, _: &[RawFrame]) {}
            fn raw_accel(&self, _: &[AccelSample]) {}
            fn status(&self, _: &Status) {}
            fn gait(&self, _: &[protocol::GaitCycle], _: &[protocol::GaitEvent]) {}
            fn haptics(&self, records: &[protocol::HapticRecord]) {
                self.0.lock().unwrap().extend_from_slice(records);
            }
        }
        let sink = Arc::new(Haptics::default());
        let device = Device::new(sink.clone());
        let mut tracker = connect(&device);
        tracker.handle(&hello(0xA1B2_C3D4));
        tracker.handle(&vector("raw_batch.hex")); // sequence 3
        tracker.handle(&vector("haptic_batch.hex")); // sequence 61
        assert_eq!(tracker.missing.iter().copied().collect::<Vec<_>>(), (4..=60).collect::<Vec<_>>());
        let records = sink.0.lock().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!((records[0].event, records[1].reason), ("on", "switched_off"));
    }

    // Event and step batches count toward gap detection like any durable
    // message, and a backfilled cycle reaches the sink.
    #[test]
    fn gait_batches_are_durable_and_reach_the_sink() {
        let sink = Arc::new(Collect::default());
        let device = Device::new(sink.clone());
        let mut tracker = connect(&device);
        tracker.handle(&hello(0xA1B2_C3D4));
        tracker.handle(&vector("raw_batch.hex")); // sequence 3
        tracker.handle(&vector("event_batch.hex")); // sequence 51
        tracker.handle(&vector("step_batch.hex")); // sequence 52
        assert_eq!(tracker.highest_seq, Some(52));
        assert_eq!(tracker.missing.iter().copied().collect::<Vec<_>>(), (4..=50).collect::<Vec<_>>());
        assert_eq!(sink.0.lock().unwrap().2, 1);
        let shown = device.state.lock().unwrap().last_cycle.expect("LIVE's last cycle");
        assert_eq!(shown.start_frame, protocol::parse_step_batch(
            protocol::parse(&vector("step_batch.hex")).unwrap().1).unwrap().last().unwrap().start_frame);

        // A missing cycle delivered by backfill is handed on.
        tracker.handle(&backfill(&[resequenced("step_batch.hex", 10)]));
        assert_eq!(sink.0.lock().unwrap().2, 2);
        assert!(!tracker.missing.contains(&10));

        // One already received is not, live or backfilled: its cycles were
        // counted twice toward a segment's limits (PROB-031, audit I06).
        tracker.handle(&backfill(&[vector("step_batch.hex")]));
        tracker.handle(&resequenced("step_batch.hex", 10));
        assert_eq!(sink.0.lock().unwrap().2, 2);
    }

    #[test]
    fn a_reconnect_to_the_same_boot_keeps_the_calibration() {
        let device = calibrated(0xA1B2_C3D4);
        connect(&device).handle(&hello(0xA1B2_C3D4));
        assert!(device.snapshot().calibration.is_some_and(|c| c.usable()));
    }

    /// A device on the other end of `device.commands`: the command it was sent.
    fn linked(device: &Device) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(8);
        *device.commands.lock().unwrap() = Some(tx);
        rx
    }

    fn ack(cmd_seq: u32, cmd_type: MsgType, kind: u8, boundary: u32) -> Vec<u8> {
        let mut payload = cmd_seq.to_le_bytes().to_vec();
        payload.extend_from_slice(&[cmd_type as u8, kind]);
        protocol::encode(MsgType::Ack, boundary, 0, &payload)
    }

    fn error_for(cmd_seq: u32) -> Vec<u8> {
        let mut payload = cmd_seq.to_le_bytes().to_vec();
        payload.push(MsgType::SessionStart as u8);
        payload.extend_from_slice(&4u16.to_le_bytes());
        payload.push(7);
        payload.extend_from_slice(b"running");
        protocol::encode(MsgType::Error, 0, 0, &payload)
    }

    // PROB-033, audit I09: a session counted as started when 800 ms passed
    // without an error. Now the device's ACK or ERROR for that command decides.
    #[tokio::test]
    async fn a_session_command_waits_for_its_own_answer() {
        let device = Arc::new(Device::new(Arc::new(NoSink)));
        let mut sent = linked(&device);
        let mut tracker = connect(&device);
        let waiting = tokio::spawn({
            let device = device.clone();
            async move { device.stop_session().await }
        });
        let command = sent.recv().await.unwrap();
        let (header, _) = protocol::parse(&command).unwrap();
        assert!(header.sequence >= FIRST_COMMAND_SEQ);
        // Someone else's answer is not this one's.
        tracker.handle(&ack(header.sequence + 1000, MsgType::SessionStop, 4, 99));
        tracker.handle(&ack(header.sequence, MsgType::SessionStop, 4, 120));
        assert_eq!(waiting.await.unwrap(), Ok(120));

        let waiting = tokio::spawn({
            let device = device.clone();
            async move { device.start_reference_capture().await }
        });
        let (header, _) = protocol::parse(&sent.recv().await.unwrap()).unwrap();
        tracker.handle(&error_for(header.sequence));
        assert!(waiting.await.unwrap().unwrap_err().contains("running"));
        assert_eq!(device.snapshot().session_kind, None, "a refused start is not a session");
    }

    // PROB-033, audit I05: the recording closes once the stop's data is in, and
    // what turns up later from before the stop is not handed on.
    #[tokio::test]
    async fn a_stop_drains_to_its_boundary_and_closes_the_stream_there() {
        let sink = Arc::new(Collect::default());
        let device = Device::new(sink.clone());
        let mut tracker = connect(&device);
        tracker.handle(&hello_holding(7, 1, 0));
        tracker.handle(&resequenced("raw_batch.hex", 1));
        tracker.handle(&resequenced("raw_batch.hex", 3)); // 2 is missing
        assert!(!device.drain_through(3, Duration::from_millis(50)).await, "2 never came");
        // 2 arrives by backfill after the recording closed: it is not handed on.
        let before = sink.0.lock().unwrap().0.len();
        tracker.handle(&backfill(&[resequenced("raw_batch.hex", 2)]));
        assert_eq!(sink.0.lock().unwrap().0.len(), before);
        // After the boundary, everything flows.
        tracker.handle(&resequenced("raw_batch.hex", 4));
        assert_eq!(sink.0.lock().unwrap().0.len(), before + 2);
        assert!(device.drain_through(4, Duration::from_millis(50)).await);
    }

    /// Owns no recording: as the store after the app restarted.
    struct Unowned;
    impl Sink for Unowned {
        fn raw_frames(&self, _: &[RawFrame]) {}
        fn raw_accel(&self, _: &[AccelSample]) {}
        fn status(&self, _: &Status) {}
        fn gait(&self, _: &[protocol::GaitCycle], _: &[protocol::GaitEvent]) {}
        fn owns_session(&self) -> bool {
            false
        }
    }

    // PROB-033: an evaluation the closed app left running is stopped, but only
    // after it has been reported for longer than a stale STATUS could explain.
    #[test]
    fn a_device_session_no_recording_owns_is_stopped() {
        let device = Device::new(Arc::new(Unowned));
        let mut tracker = connect(&device);
        tracker.handle(&hello(0xA1B2_C3D4));
        let status = vector("status.hex"); // an evaluation running
        tracker.handle(&status);
        tracker.handle(&status);
        assert!(!device.state.lock().unwrap().stop_orphan);
        std::thread::sleep(ORPHAN_AFTER + Duration::from_millis(50));
        tracker.handle(&status);
        assert!(device.state.lock().unwrap().stop_orphan);
        assert!(device.last_error().unwrap().contains("no recording owns"));
    }

}
