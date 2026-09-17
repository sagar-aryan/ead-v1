//! Application state and the command surface the UI calls.

use std::sync::{Arc, Mutex};

use tauri::ipc::Channel;

use crate::device::{self, Device, Sink, Snapshot};
use crate::link::{usb::UsbPortInfo, LinkTarget};
use crate::live::{LiveHub, LiveTick};
use crate::protocol::{config::DeviceConfigSection, RawFrame, Status};
use crate::store::{
    DeviceIdentity, Patient, RawWindow, SegmentLimits, Session, SessionKind, SignalGroup, Store,
};

/// Routes decoded telemetry to the store and the live view.
struct Telemetry {
    store: Arc<Store>,
    live: Arc<LiveHub>,
}

impl Sink for Telemetry {
    fn raw_frames(&self, frames: &[RawFrame]) {
        self.store.record_frames(frames);
        self.live.push(frames);
    }

    fn status(&self, _status: &Status) {}

    fn gait(&self, cycles: &[crate::protocol::GaitCycle], events: &[crate::protocol::GaitEvent]) {
        self.store.record_gait(cycles, events);
    }
}

pub struct App {
    pub device: Arc<Device>,
    pub store: Arc<Store>,
    pub live: Arc<LiveHub>,
    /// Stops the current link task; `None` when disconnected.
    link_stop: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
    config: Mutex<Option<Arc<DeviceConfigSection>>>,
    shutdown: tokio::sync::watch::Sender<bool>,
}

impl App {
    pub fn new(store: Arc<Store>) -> Arc<Self> {
        let live = LiveHub::new();
        let telemetry = Arc::new(Telemetry { store: store.clone(), live: live.clone() });
        let (shutdown, _) = tokio::sync::watch::channel(false);
        Arc::new(Self {
            device: Arc::new(Device::new(telemetry)),
            store,
            live,
            link_stop: Mutex::new(None),
            config: Mutex::new(None),
            shutdown,
        })
    }

    pub fn shutdown_signal(&self) -> tokio::sync::watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
        self.disconnect();
        self.store.flush();
    }

    fn connect(self: &Arc<Self>, target: LinkTarget) {
        self.disconnect();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        *self.link_stop.lock().expect("link stop") = Some(stop_tx);
        let device = self.device.clone();
        let app = self.clone();
        tauri::async_runtime::spawn(async move {
            device::run(device, target, stop_rx).await;
            app.live.set_config(None);
            *app.config.lock().expect("config") = None;
        });
    }

    fn disconnect(&self) {
        if let Some(stop) = self.link_stop.lock().expect("link stop").take() {
            let _ = stop.send(true);
        }
    }

    fn device_identity(&self) -> DeviceIdentity {
        let snapshot = self.device.snapshot();
        DeviceIdentity {
            firmware: snapshot.firmware,
            config_sha256: self.config_hash(),
            mac: snapshot.mac,
            boot_id: snapshot.boot_id,
            config_section: self.device.config_section(),
        }
    }

    fn config_hash(&self) -> Option<String> {
        self.device.config_sha256().map(|digest| {
            digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
        })
    }
}

/// Commands return plain strings on failure: the UI shows them verbatim.
type CommandResult<T> = Result<T, String>;

fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
pub fn list_usb_ports() -> Vec<UsbPortInfo> {
    crate::link::usb::list_ports()
}

#[tauri::command]
pub fn connect_device(app: tauri::State<'_, Arc<App>>, target: LinkTarget) {
    app.inner().connect(target);
}

#[tauri::command]
pub fn disconnect_device(app: tauri::State<'_, Arc<App>>) {
    app.disconnect();
}

#[tauri::command]
pub fn device_snapshot(app: tauri::State<'_, Arc<App>>) -> Snapshot {
    app.device.snapshot()
}

#[tauri::command]
pub fn device_config(app: tauri::State<'_, Arc<App>>) -> Option<Arc<DeviceConfigSection>> {
    let config = app.device.config();
    if let Some(config) = config.clone() {
        app.live.set_config(Some(config.clone()));
        *app.config.lock().expect("config") = Some(config);
    }
    config
}

/// Names the UI uses to decode status and fault bits, so the protocol
/// vocabulary lives in one place (`docs/protocol.md` §6).
#[derive(serde::Serialize)]
pub struct Vocabulary {
    pub device_states: [&'static str; 9],
    pub fault_names: [&'static str; 10],
    pub raw_status_names: [&'static str; 9],
    /// Gait state names in device order (docs/protocol.md §6.6).
    pub gait_states: [&'static str; 7],
    /// Feature order doc 06 §3 weighs, and the error classes it names.
    pub feature_names: [&'static str; 7],
    pub error_classes: [&'static str; 7],
    pub default_wifi_url: &'static str,
}

#[tauri::command]
pub fn vocabulary() -> Vocabulary {
    Vocabulary {
        device_states: crate::protocol::DEVICE_STATES,
        fault_names: crate::protocol::FAULT_NAMES,
        raw_status_names: crate::protocol::RAW_STATUS_NAMES,
        gait_states: crate::protocol::GAIT_STATES,
        feature_names: crate::protocol::FEATURE_NAMES,
        error_classes: crate::protocol::ERROR_CLASSES,
        default_wifi_url: crate::link::ws::DEFAULT_URL,
    }
}

#[tauri::command]
pub fn subscribe_live(app: tauri::State<'_, Arc<App>>, channel: Channel<LiveTick>) {
    app.live.subscribe(channel);
}

#[tauri::command]
pub fn unsubscribe_live(app: tauri::State<'_, Arc<App>>) {
    app.live.unsubscribe();
}

#[tauri::command]
pub fn create_patient(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
    name: String,
) -> CommandResult<Patient> {
    app.store.create_patient(&patient_id, &name).map_err(failed)
}

#[tauri::command]
pub fn patients(app: tauri::State<'_, Arc<App>>) -> CommandResult<Vec<Patient>> {
    app.store.patients().map_err(failed)
}

#[tauri::command]
pub fn start_recording(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
) -> CommandResult<Session> {
    if !app.device.is_live() {
        return Err("device is not connected".into());
    }
    let identity = app.device_identity();
    app.store
        .start_session(&patient_id, SessionKind::Recording, &identity, None, None)
        .map_err(failed)
}

#[tauri::command]
pub fn stop_recording(app: tauri::State<'_, Arc<App>>) -> CommandResult<Option<Session>> {
    app.store.stop_session().map_err(failed)
}

#[tauri::command]
pub fn recording_session(app: tauri::State<'_, Arc<App>>) -> Option<String> {
    app.store.recording_session()
}

#[tauri::command]
pub fn sessions(app: tauri::State<'_, Arc<App>>) -> CommandResult<Vec<Session>> {
    app.store.sessions().map_err(failed)
}

#[tauri::command]
pub fn session(app: tauri::State<'_, Arc<App>>, session_id: String) -> CommandResult<Session> {
    app.store.session(&session_id).map_err(failed)
}

/// Decimated signals over a frame range, in physical units, for the raw view.
#[tauri::command]
pub fn raw_window(
    app: tauri::State<'_, Arc<App>>,
    session_id: String,
    groups: Vec<SignalGroup>,
    first_frame: i64,
    last_frame: i64,
    max_points: usize,
) -> CommandResult<RawWindow> {
    app.store
        .raw_window(&session_id, &groups, first_frame, last_frame, max_points)
        .map_err(failed)
}

/// Why a session may not start, in the researcher's words. Empty means it may.
///
/// Doc 12 §4 requires patient, locked reference, both segment limits and healthy
/// sensors before RUNNING. The gate is computed here rather than in the UI so
/// that a command cannot be issued past it.
#[tauri::command]
pub fn session_blockers(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
    reference_id: String,
    limits: Option<SegmentLimits>,
    kind: SessionKind,
) -> Vec<String> {
    let mut blockers = Vec::new();
    let snapshot = app.device.snapshot();
    if snapshot.link_state != crate::device::LinkState::Connected {
        blockers.push("the device is not connected".into());
    }
    if patient_id.is_empty() {
        blockers.push("no patient is selected".into());
    }
    let scored = kind == SessionKind::ReferenceCheck || kind == SessionKind::Evaluation;
    if scored && reference_id.is_empty() {
        blockers.push("no reference profile is selected".into());
    }
    match snapshot.calibration {
        Some(record) if record.usable() => {}
        Some(_) => blockers.push("the last calibration was rejected".into()),
        None => blockers.push("the device has not been calibrated since it started".into()),
    }
    for fault in &snapshot.faults {
        blockers.push(format!("the device reports a fault: {fault}"));
    }
    if app.store.recording_session().is_some() {
        blockers.push("another session is already recording".into());
    }
    if kind == SessionKind::Evaluation {
        // Doc 12 §5: no default is invented when the researcher leaves these
        // blank, so a missing or zero limit blocks the start.
        match limits {
            Some(l) if l.max_cycles > 0 && l.max_errors > 0 => {}
            Some(_) => blockers.push("both segment limits must be greater than zero".into()),
            None => blockers.push("the segment limits have not been entered".into()),
        }
    }
    blockers
}

/// Starts a reference capture (doc 12 §2): a recording session, and the device
/// session that collects the patient's own cycles into a profile. Either both
/// start or neither does.
#[tauri::command]
pub fn start_reference_capture(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
) -> CommandResult<Session> {
    let blockers = session_blockers(
        app.clone(),
        patient_id.clone(),
        String::new(),
        None,
        SessionKind::ReferenceCapture,
    );
    if let Some(first) = blockers.first() {
        return Err(first.clone());
    }
    let identity = app.device_identity();
    let session = app
        .store
        .start_session(&patient_id, SessionKind::ReferenceCapture, &identity, None, None)
        .map_err(failed)?;
    if let Err(e) = app.device.start_reference_capture() {
        let _ = app.store.stop_session();
        return Err(e);
    }
    Ok(session)
}

/// Ends a capture and stores what the device built as the patient's next
/// version. Returns null when the device sent no profile — it refuses to build
/// one from fewer than thirty valid cycles (doc 12 §2), and replies with an
/// error instead. The recording is stopped either way: the walk is still data.
#[tauri::command]
pub async fn finish_reference_capture(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
) -> CommandResult<Option<crate::store::StoredReference>> {
    let session_id = app.store.recording_session();
    let stop = app.device.stop_session();
    // The profile arrives as a SESSION_STOP reply, a link round trip later.
    let mut profile = None;
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        profile = app.device.take_reference();
        if profile.is_some() {
            break;
        }
    }
    let _ = app.store.stop_session();
    stop?;
    let Some(profile) = profile else { return Ok(None) };
    app.store
        .add_reference(&patient_id, session_id.as_deref(), &profile)
        .map(Some)
        .map_err(failed)
}

/// Starts a check (a short walk read against a stored profile, doc 12 §3) or an
/// evaluation (scored and segmented, doc 12 §4). Both lock the profile: once it
/// has judged a session it is immutable.
#[tauri::command]
pub fn start_scored_session(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
    reference_id: String,
    check: bool,
    limits: Option<SegmentLimits>,
) -> CommandResult<Session> {
    let kind = if check { SessionKind::ReferenceCheck } else { SessionKind::Evaluation };
    let blockers =
        session_blockers(app.clone(), patient_id.clone(), reference_id.clone(), limits, kind);
    if let Some(first) = blockers.first() {
        return Err(first.clone());
    }
    let reference = app.store.reference(&reference_id).map_err(failed)?;
    let identity = app.device_identity();
    let session = app
        .store
        .start_session(
            &patient_id,
            kind,
            &identity,
            Some(&reference_id),
            if check { None } else { limits },
        )
        .map_err(failed)?;
    if let Err(e) = app.device.start_scored_session(check, &reference.profile) {
        let _ = app.store.stop_session();
        return Err(e);
    }
    app.store.lock_reference(&reference_id).map_err(failed)?;
    Ok(session)
}

/// Ends a check or an evaluation: the device stops scoring and the recording
/// closes, which closes the open segment.
#[tauri::command]
pub fn stop_scored_session(app: tauri::State<'_, Arc<App>>) -> CommandResult<Option<Session>> {
    let stop = app.device.stop_session();
    let session = app.store.stop_session().map_err(failed)?;
    stop?;
    Ok(session)
}

/// Every segment of a session, in order; empty unless it was an evaluation.
#[tauri::command]
pub fn segments(
    app: tauri::State<'_, Arc<App>>,
    session_id: String,
) -> CommandResult<Vec<crate::store::StoredSegment>> {
    app.store.segments(&session_id).map_err(failed)
}

#[tauri::command]
pub fn references(
    app: tauri::State<'_, Arc<App>>,
    patient_id: String,
) -> CommandResult<Vec<crate::store::StoredReference>> {
    app.store.references(&patient_id).map_err(failed)
}

/// Every gait cycle stored for a session, in time order.
#[tauri::command]
pub fn cycles(
    app: tauri::State<'_, Arc<App>>,
    session_id: String,
) -> CommandResult<Vec<crate::store::StoredCycle>> {
    app.store.cycles(&session_id).map_err(failed)
}

/// Every gait event stored for a session, in time order.
#[tauri::command]
pub fn events(
    app: tauri::State<'_, Arc<App>>,
    session_id: String,
) -> CommandResult<Vec<crate::store::StoredEvent>> {
    app.store.events(&session_id).map_err(failed)
}

/// Starts a still window on the device. The record arrives in a later STATUS
/// and in `device_snapshot`; five seconds is the documented default.
#[tauri::command]
pub fn start_calibration(app: tauri::State<'_, Arc<App>>, duration_ms: u16) -> CommandResult<()> {
    app.device.start_calibration(duration_ms).map_err(failed)
}

#[tauri::command]
pub fn cancel_calibration(app: tauri::State<'_, Arc<App>>) -> CommandResult<()> {
    app.device.cancel_calibration().map_err(failed)
}

/// The configuration recorded with a session, so stored counts can be shown in
/// physical units even when no device is connected.
#[tauri::command]
pub fn session_config(
    app: tauri::State<'_, Arc<App>>,
    session_id: String,
) -> CommandResult<Option<DeviceConfigSection>> {
    let section = app.store.session_config(&session_id).map_err(failed)?;
    match section {
        Some(bytes) => crate::protocol::parse_section(&bytes).map(Some).map_err(failed),
        None => Ok(None),
    }
}


