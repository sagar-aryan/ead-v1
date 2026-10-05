//! Session store. Raw frames are canonical (doc 01 §7, DEC-007): they are
//! written exactly as the device sent them, in chip-frame ADC counts, and every
//! derived value is computed from them.
//!
//! One writer thread owns the connection; readers open their own. Writes are
//! batched into transactions so a 200 Hz stream costs a few commits per second.

pub mod raw;
mod schema;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension};

use crate::protocol::{AccelSample, GaitCycle, GaitEvent, HapticRecord, RawFrame, Status};

pub use raw::{RawWindow, SignalGroup};

/// At most this long between a frame arriving and being durable.
const COMMIT_INTERVAL: Duration = Duration::from_millis(250);
/// Frames and accelerometer samples held for retry while commits fail: five
/// minutes of a recording (200 frames and 500 samples a second), about 30 MB.
/// Past it the oldest are dropped and counted, rather than memory growing until
/// the app dies and takes everything with it.
const MAX_RETAINED_ROWS: usize = 5 * 60 * 700;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Rejected(String),
}

type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// Raw capture with no analysis; builds replay datasets (M3).
    Recording,
    /// Collects the patient's own cycles into a new reference profile.
    ReferenceCapture,
    /// A short walk read against a stored profile, with no haptics (doc 12 §3).
    ReferenceCheck,
    /// A scored, segmented session against a locked profile (doc 12 §4).
    Evaluation,
}

impl SessionKind {
    fn as_str(self) -> &'static str {
        match self {
            SessionKind::Recording => "recording",
            SessionKind::ReferenceCapture => "reference_capture",
            SessionKind::ReferenceCheck => "reference_check",
            SessionKind::Evaluation => "evaluation",
        }
    }
}

/// Doc 12 §5. Both are required before an evaluation may start; the segment
/// closes when either is reached first. There is no default: the spec forbids
/// inventing one.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct SegmentLimits {
    pub max_cycles: u32,
    pub max_errors: u32,
}

/// A moment where the device's state or fault mask changed during a session.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StatusChange {
    pub frame_index: i64,
    pub at: String,
    pub device_state: u8,
    pub faults: u16,
}

/// One segment of an evaluation, as closed or still open.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StoredSegment {
    pub segment_index: i64,
    pub started_at: String,
    pub closed_at: Option<String>,
    /// `cycle_limit`, `error_limit`, `session_stopped` or `device_restarted`; null
    /// while open.
    pub closed_by: Option<String>,
    pub valid_cycles: i64,
    pub errors: i64,
}

/// What counts as an error against `max_errors_per_segment` (DEC-014): a scored
/// cycle the engine named a class for, confident enough that doc 06 §7 would
/// let it be displayed. Below that confidence the engine says not to show the
/// classification at all, so counting it toward a limit would be worse.
pub fn counts_as_error(cycle: &GaitCycle) -> bool {
    cycle.primary_class != 0 && cycle.confidence >= CONFIDENCE_FOR_DISPLAY
}

/// Doc 06 §7, mirrored from `ead::kConfidenceForDisplay`.
const CONFIDENCE_FOR_DISPLAY: f32 = 0.50;

/// Mirrored from `ead::kDistanceMinZuptQuality` (doc 05 §8): below this, a
/// cycle's distance is not a measurement and is left out of any comparison.
pub const DISTANCE_MIN_ZUPT_QUALITY: f32 = 0.15;

/// Doc 06 §3 weights, in feature order, mirrored from `ead::kFeatureWeights`.
const FEATURE_WEIGHTS: [f32; 7] = [0.25, 0.15, 0.15, 0.15, 0.10, 0.10, 0.10];
/// Doc 06 §2: three spreads out is a deviation of 1.
const DEVIATION_SCALE: f32 = 3.0;

/// `unilateral_cycle_symmetry_proxy` (doc 05 §10): cycle repeatability between
/// consecutive valid right-leg cycles, `1 - normalized_difference`, using the
/// error engine's normalization — the difference in spreads, clamped at three,
/// weighted by doc 06 §3.
///
/// This is never a left-versus-right symmetry claim. Only the right leg is
/// instrumented, so the name it is given in the export is the one doc 05 §10
/// insists on.
///
/// Returns None when no feature could be compared: a cycle distance with no
/// zero-velocity window is dropped from both cycles, exactly as the error
/// engine drops it, rather than scored as perfect repeatability.
fn symmetry_proxy(current: &StoredCycle, previous: &StoredCycle, spreads: &[f32; 7]) -> Option<f32> {
    let mut weighted = 0.0;
    let mut active = 0.0;
    for feature in 0..7 {
        if spreads[feature] <= 0.0 {
            continue;
        }
        if feature == 5
            && (current.zupt_quality < DISTANCE_MIN_ZUPT_QUALITY
                || previous.zupt_quality < DISTANCE_MIN_ZUPT_QUALITY)
        {
            continue;
        }
        let difference = (feature_value(current, feature)
            - feature_value(previous, feature))
        .abs();
        let d = (difference / spreads[feature] / DEVIATION_SCALE).clamp(0.0, 1.0);
        weighted += FEATURE_WEIGHTS[feature] * d;
        active += FEATURE_WEIGHTS[feature];
    }
    (active > 0.0).then(|| 1.0 - weighted / active)
}

/// The feature values doc 06 §3 weighs, in its order, read off a stored row.
/// Mirrors `ead::featureValue`.
fn feature_value(cycle: &StoredCycle, feature: usize) -> f32 {
    match feature {
        0 => cycle.peak_dorsiflexion_deg,
        1 => cycle.contact_sagittal_deg,
        2 => cycle.peak_inversion_deg,
        3 => cycle.cycle_time_s,
        4 => cycle.stance_ratio,
        5 => cycle.distance_m,
        _ => cycle.peak_shank_rate_dps,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Patient {
    pub patient_id: String,
    pub name: String,
    pub created_at: String,
    pub session_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Session {
    pub session_id: String,
    pub patient_id: String,
    pub patient_name: String,
    pub kind: String,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub firmware: Option<String>,
    pub config_sha256: Option<String>,
    pub device_mac: Option<String>,
    pub boot_id: Option<i64>,
    pub first_frame_index: Option<i64>,
    pub last_frame_index: Option<i64>,
    pub frames_stored: i64,
    /// Frames the device produced in this range but that never arrived.
    pub frames_missing: i64,
    /// The profile this session was scored against, for a check or evaluation.
    pub reference_id: Option<String>,
    /// Segment limits as entered; both null unless this is an evaluation.
    pub max_cycles_per_segment: Option<i64>,
    pub max_errors_per_segment: Option<i64>,
    /// The name the user gave the session, if any.
    pub label: Option<String>,
    /// The device's payload schema when it recorded (HELLO); null for sessions
    /// stored before store schema 11, when it was not kept.
    pub payload_schema: Option<i64>,
}

/// Device identity recorded with a session, so every dataset carries the
/// firmware and configuration that produced it (doc 18).
#[derive(Debug, Clone, Default)]
pub struct DeviceIdentity {
    pub firmware: Option<String>,
    /// The calibration record in force when the session started, as JSON. It
    /// lives only in device RAM, so a session exported later has no other way
    /// to say what its orientation estimate rested on (doc 10 §6).
    pub calibration: Option<String>,
    pub config_sha256: Option<String>,
    pub mac: Option<String>,
    pub boot_id: Option<u32>,
    /// The payload schema the device declared in HELLO.
    pub payload_schema: Option<u16>,
    /// The device's configuration section, stored so a recording of raw counts
    /// is self-describing (`docs/protocol.md` §5.5).
    pub config_section: Option<crate::protocol::ConfigSection>,
}

/// A service test as recorded (doc 07 §7).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StoredServiceTest {
    pub test_id: i64,
    pub at: String,
    pub kind: String,
    pub motor: Option<u8>,
    pub duty: Option<u8>,
    pub duration_ms: Option<u16>,
    pub felt: Option<bool>,
    pub report: Option<String>,
}

/// A gait cycle as stored, in physical units (doc 05 §11).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StoredCycle {
    pub start_frame: i64,
    pub end_frame: i64,
    pub start_us: i64,
    pub cycle_time_s: f32,
    pub stance_time_s: f32,
    pub swing_time_s: f32,
    pub stance_ratio: f32,
    pub swing_ratio: f32,
    pub cadence_steps_per_min: f32,
    pub peak_shank_rate_dps: f32,
    pub peak_dorsiflexion_deg: f32,
    pub contact_sagittal_deg: f32,
    pub peak_inversion_deg: f32,
    pub distance_m: f32,
    pub speed_mps: f32,
    pub zupt_quality: f32,
    pub valid: bool,
    /// All zero when the cycle was not scored: no reference was loaded. That is
    /// not the same as agreeing with the reference, and a reader must not treat
    /// a zero score as a good cycle.
    pub error_score: f32,
    pub confidence: f32,
    pub confidence_subscores: Vec<f32>,
    pub deviations: Vec<f32>,
    pub active_classes: u16,
    pub primary_class: u8,
    /// Which segment of the session it fell in; 0 when the session has none.
    pub segment_index: i64,
    /// Doc 05 §10, `unilateral_cycle_symmetry_proxy`: cycle repeatability
    /// against the previous valid cycle, never a left-versus-right claim.
    /// Null for the first valid cycle, for rejected cycles, and for a session
    /// with no reference — the normalization needs the reference's spreads.
    pub symmetry_proxy: Option<f32>,
}

/// A versioned reference profile (doc 12 §2–§3).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StoredReference {
    pub reference_id: String,
    pub patient_id: String,
    pub version: i64,
    pub created_at: String,
    /// The capture session it was built from, when that session is still stored.
    pub session_id: Option<String>,
    pub cycles: i64,
    /// Locked profiles are immutable: they have been used to judge a session.
    pub locked: bool,
    pub profile: crate::protocol::ReferenceProfile,
}

/// A gait event as stored. `kind` is the protocol's name for it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StoredEvent {
    pub frame_index: i64,
    pub timestamp_us: i64,
    pub kind: String,
}

/// A feedback cue or episode end as stored (doc 06 §13, DEC-023). Motor 0 is
/// none; duty 0 means the cue did not run.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StoredHaptic {
    pub device_time_us: i64,
    pub event: String,
    pub cycle_start_frame: i64,
    pub reason: String,
    pub motor_a: i64,
    pub duty_a: i64,
    pub motor_b: i64,
    pub duty_b: i64,
    pub duration_ms: i64,
    pub error_class: String,
    pub error_score: f32,
    pub confidence: f32,
}

enum WriteCommand {
    Frames { session_id: String, frames: Vec<RawFrame> },
    Haptics { session_id: String, records: Vec<HapticRecord> },
    Accel { session_id: String, samples: Vec<AccelSample> },
    Status { session_id: String, frame_index: i64, device_state: u8, faults: u16 },
    Gait { session_id: String, cycles: Vec<GaitCycle>, events: Vec<GaitEvent> },
    Flush(mpsc::Sender<()>),
    Stop,
}

/// What the writer could not store (PROB-029). Shared with the writer thread.
#[derive(Default)]
struct WriteHealth {
    /// The last commit's error while commits are failing; cleared by a success.
    failing: Option<String>,
    /// Frames and accelerometer samples dropped for good since the recording
    /// started, and the error that cost them.
    lost_rows: usize,
    lost_because: Option<String>,
}

pub struct Store {
    path: PathBuf,
    health: Arc<Mutex<WriteHealth>>,
    /// The last (state, faults) written, so only changes are stored.
    last_status: Mutex<Option<(u8, u16)>>,
    writer: Mutex<Option<mpsc::Sender<WriteCommand>>>,
    /// Session currently recording, if any.
    recording: Mutex<Option<String>>,
    /// The session a device restart ended, until the next one starts (PROB-019).
    ended_by_restart: Mutex<Option<String>>,
}

impl Store {
    /// Opens (creating if needed) the store and starts the writer thread.
    pub fn open(path: impl AsRef<Path>) -> Result<Arc<Self>> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut connection = Connection::open(&path)?;
        schema::migrate(&mut connection)?;
        close_orphaned_sessions(&connection)?;

        let (tx, rx) = mpsc::channel::<WriteCommand>();
        let health = Arc::new(Mutex::new(WriteHealth::default()));
        let writer_health = health.clone();
        std::thread::Builder::new()
            .name("store-writer".into())
            .spawn(move || writer_loop(connection, rx, writer_health))
            .expect("spawn store writer");

        Ok(Arc::new(Self {
            path,
            health,
            last_status: Mutex::new(None),
            writer: Mutex::new(Some(tx)),
            recording: Mutex::new(None),
            ended_by_restart: Mutex::new(None),
        }))
    }

    /// Where exports go by default: an `exports` folder beside the database,
    /// so a package is somewhere findable without a file dialog.
    pub fn export_root(&self) -> PathBuf {
        self.path.parent().unwrap_or_else(|| Path::new(".")).join("exports")
    }

    fn reader(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "busy_timeout", 5000)?;
        Ok(connection)
    }

    // ---- service tests (doc 07 §7, DEC-018) ---------------------------------

    /// Records a sensor check; `report` is its per-sensor result as JSON.
    pub fn record_sensor_check(&self, identity: &DeviceIdentity, report: &str) -> Result<i64> {
        self.insert_service_test(identity, "sensor_check", None, Some(report))
    }

    /// Records a motor pulse the device accepted; whether it was felt comes later.
    pub fn record_motor_pulse(
        &self,
        identity: &DeviceIdentity,
        pulse: &crate::protocol::MotorPulse,
    ) -> Result<i64> {
        self.insert_service_test(identity, "motor_pulse", Some(pulse), None)
    }

    fn insert_service_test(
        &self,
        identity: &DeviceIdentity,
        kind: &str,
        pulse: Option<&crate::protocol::MotorPulse>,
        report: Option<&str>,
    ) -> Result<i64> {
        let connection = self.reader()?;
        connection.execute(
            "INSERT INTO service_tests
               (at, device_mac, boot_id, kind, motor, duty, duration_ms, report)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                now_utc(),
                &identity.mac,
                identity.boot_id.map(i64::from),
                kind,
                pulse.map(|p| p.motor),
                pulse.map(|p| p.duty),
                pulse.map(|p| p.duration_ms),
                report,
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }

    /// The operator's answer to "did you feel it?" for one motor pulse.
    pub fn set_motor_felt(&self, test_id: i64, felt: bool) -> Result<()> {
        let connection = self.reader()?;
        let changed = connection.execute(
            "UPDATE service_tests SET felt = ?2 WHERE test_id = ?1 AND kind = 'motor_pulse'",
            (test_id, felt),
        )?;
        if changed == 0 {
            return Err(StoreError::Rejected(format!("no motor pulse {test_id}")));
        }
        Ok(())
    }

    /// The latest service tests, newest first.
    pub fn service_tests(&self, limit: usize) -> Result<Vec<StoredServiceTest>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT test_id, at, kind, motor, duty, duration_ms, felt, report
             FROM service_tests ORDER BY test_id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], |row| {
            Ok(StoredServiceTest {
                test_id: row.get(0)?,
                at: row.get(1)?,
                kind: row.get(2)?,
                motor: row.get(3)?,
                duty: row.get(4)?,
                duration_ms: row.get(5)?,
                felt: row.get(6)?,
                report: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---- patients ----------------------------------------------------------

    pub fn create_patient(&self, patient_id: &str, name: &str) -> Result<Patient> {
        let patient_id = patient_id.trim();
        let name = name.trim();
        if patient_id.is_empty() || name.is_empty() {
            return Err(StoreError::Rejected("patient ID and name are required".into()));
        }
        let created_at = now_utc();
        let connection = self.reader()?;
        connection
            .execute(
                "INSERT INTO patients (patient_id, name, created_at) VALUES (?1, ?2, ?3)",
                (patient_id, name, &created_at),
            )
            .map_err(|err| match err {
                rusqlite::Error::SqliteFailure(e, _)
                    if e.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    StoreError::Rejected(format!("patient ID {patient_id} already exists"))
                }
                other => StoreError::Sqlite(other),
            })?;
        Ok(Patient {
            patient_id: patient_id.to_string(),
            name: name.to_string(),
            created_at,
            session_count: 0,
        })
    }

    pub fn patients(&self) -> Result<Vec<Patient>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT p.patient_id, p.name, p.created_at,
                    (SELECT COUNT(*) FROM sessions s WHERE s.patient_id = p.patient_id)
             FROM patients p ORDER BY p.created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Patient {
                patient_id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                session_count: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ---- sessions ----------------------------------------------------------

    /// Opens a recording. `reference_id` and `limits` are what a check or an
    /// evaluation was started against; a plain recording passes neither.
    pub fn start_session(
        &self,
        patient_id: &str,
        kind: SessionKind,
        identity: &DeviceIdentity,
        reference_id: Option<&str>,
        limits: Option<SegmentLimits>,
    ) -> Result<Session> {
        let mut recording = self.recording.lock().expect("recording");
        if let Some(open) = recording.as_ref() {
            return Err(StoreError::Rejected(format!("session {open} is already recording")));
        }
        let connection = self.reader()?;
        if let Some(reference_id) = reference_id {
            // A patient is judged against their own walking only (doc 12 §2). The
            // UI lists the patient's references, but the store did not check, so
            // any caller could pair a patient with another's (PROB-032).
            let owner: Option<String> = connection
                .query_row(
                    "SELECT patient_id FROM reference_profiles WHERE reference_id = ?1",
                    [reference_id],
                    |row| row.get(0),
                )
                .optional()?;
            match owner {
                None => return Err(StoreError::Rejected(format!("no reference {reference_id}"))),
                Some(owner) if owner != patient_id => {
                    return Err(StoreError::Rejected(format!(
                        "reference {reference_id} belongs to patient {owner}, not {patient_id}"
                    )))
                }
                Some(_) => {}
            }
        }
        let session_id = new_session_id(&connection)?;
        connection.execute(
            "INSERT INTO sessions
               (session_id, patient_id, kind, started_at, firmware, config_sha256, device_mac,
                boot_id, config_section, config_format, reference_id,
                max_cycles_per_segment, max_errors_per_segment, calibration, payload_schema)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                &session_id,
                patient_id,
                kind.as_str(),
                now_utc(),
                &identity.firmware,
                &identity.config_sha256,
                &identity.mac,
                identity.boot_id.map(i64::from),
                identity.config_section.as_ref().map(|c| &c.bytes),
                identity.config_section.as_ref().map_or(1, |c| c.format),
                reference_id,
                limits.map(|l| i64::from(l.max_cycles)),
                limits.map(|l| i64::from(l.max_errors)),
                &identity.calibration,
                identity.payload_schema.map(i64::from),
            ],
        )?;
        // A segmented session always has a first segment open, so the UI has
        // something to count into before the first cycle arrives.
        if limits.is_some() {
            connection.execute(
                "INSERT INTO segments (session_id, segment_index, started_at)
                 VALUES (?1, 0, ?2)",
                (&session_id, now_utc()),
            )?;
        }
        *self.last_status.lock().expect("last status") = None;
        *self.ended_by_restart.lock().expect("restart notice") = None;
        {
            let mut health = self.health.lock().expect("write health");
            health.lost_rows = 0;
            health.lost_because = None;
        }
        *recording = Some(session_id.clone());
        drop(recording);
        self.session(&session_id)
    }

    pub fn stop_session(&self) -> Result<Option<Session>> {
        match self.close_recording("session_stopped")? {
            Some(session_id) => self.session(&session_id).map(Some),
            None => Ok(None),
        }
    }

    /// Ends the open recording because the device restarted (PROB-019). The new
    /// boot restarts frame indices, so its frames would collide with the ones
    /// already stored; the session ends where the old boot's data does. Returns
    /// the session ended, if one was recording.
    pub fn end_session_at_restart(&self) -> Result<Option<String>> {
        let ended = self.close_recording("device_restarted")?;
        if ended.is_some() {
            *self.ended_by_restart.lock().expect("restart notice") = ended.clone();
        }
        Ok(ended)
    }

    /// The session a device restart ended, until another session starts.
    pub fn ended_by_restart(&self) -> Option<String> {
        self.ended_by_restart.lock().expect("restart notice").clone()
    }

    fn close_recording(&self, closed_by: &str) -> Result<Option<String>> {
        let session_id = match self.recording.lock().expect("recording").take() {
            Some(id) => id,
            None => return Ok(None),
        };
        // The session closes whatever the flush says; a failure is reported after.
        let flushed = self.flush();
        let connection = self.reader()?;
        connection.execute(
            "UPDATE sessions SET stopped_at = ?2 WHERE session_id = ?1",
            (&session_id, now_utc()),
        )?;
        connection.execute(
            "UPDATE segments SET closed_at = ?2, closed_by = ?3
             WHERE session_id = ?1 AND closed_at IS NULL",
            (&session_id, now_utc(), closed_by),
        )?;
        flushed.map_err(|e| {
            StoreError::Rejected(format!("session {session_id} stopped, but {e}"))
        })?;
        Ok(Some(session_id))
    }

    /// Names a session, or clears its name when `label` is blank.
    pub fn set_session_label(&self, session_id: &str, label: &str) -> Result<()> {
        let label = label.trim();
        let changed = self.reader()?.execute(
            "UPDATE sessions SET label = ?2 WHERE session_id = ?1",
            rusqlite::params![session_id, (!label.is_empty()).then_some(label)],
        )?;
        if changed == 0 {
            return Err(StoreError::Rejected(format!("no session {session_id}")));
        }
        Ok(())
    }

    pub fn recording_session(&self) -> Option<String> {
        self.recording.lock().expect("recording").clone()
    }

    pub fn session(&self, session_id: &str) -> Result<Session> {
        let connection = self.reader()?;
        Ok(connection.query_row(
            "SELECT s.session_id, s.patient_id, COALESCE(p.name, ''), s.kind, s.started_at,
                    s.stopped_at, s.firmware, s.config_sha256, s.device_mac, s.boot_id,
                    MIN(f.frame_index), MAX(f.frame_index), COUNT(f.frame_index),
                    s.reference_id, s.max_cycles_per_segment, s.max_errors_per_segment,
                    s.label, s.payload_schema
             FROM sessions s
             LEFT JOIN patients p ON p.patient_id = s.patient_id
             LEFT JOIN raw_frames f ON f.session_id = s.session_id
             WHERE s.session_id = ?1
             GROUP BY s.session_id",
            [session_id],
            session_from_row,
        )?)
    }

    pub fn sessions(&self) -> Result<Vec<Session>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT s.session_id, s.patient_id, COALESCE(p.name, ''), s.kind, s.started_at,
                    s.stopped_at, s.firmware, s.config_sha256, s.device_mac, s.boot_id,
                    MIN(f.frame_index), MAX(f.frame_index), COUNT(f.frame_index),
                    s.reference_id, s.max_cycles_per_segment, s.max_errors_per_segment,
                    s.label, s.payload_schema
             FROM sessions s
             LEFT JOIN patients p ON p.patient_id = s.patient_id
             LEFT JOIN raw_frames f ON f.session_id = s.session_id
             GROUP BY s.session_id
             ORDER BY s.started_at DESC",
        )?;
        let rows = statement.query_map([], session_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ---- frames ------------------------------------------------------------

    /// Queues frames for the session currently recording. Returns immediately.
    pub fn record_frames(&self, frames: &[RawFrame]) {
        let Some(session_id) = self.recording_session() else { return };
        let writer = self.writer.lock().expect("writer");
        if let Some(writer) = writer.as_ref() {
            let _ = writer.send(WriteCommand::Frames { session_id, frames: frames.to_vec() });
        }
    }

    /// Queues accelerometer samples for the session currently recording, under
    /// the same rule as frames: whatever arrives while it records, live or
    /// backfilled, is its data.
    pub fn record_accel(&self, samples: &[AccelSample]) {
        let Some(session_id) = self.recording_session() else { return };
        let writer = self.writer.lock().expect("writer");
        if let Some(writer) = writer.as_ref() {
            let _ = writer.send(WriteCommand::Accel { session_id, samples: samples.to_vec() });
        }
    }

    /// Stores gait cycles and events for the recording session, if any.
    pub fn record_gait(&self, cycles: &[GaitCycle], events: &[GaitEvent]) {
        if cycles.is_empty() && events.is_empty() {
            return;
        }
        let Some(session_id) = self.recording_session() else { return };
        let writer = self.writer.lock().expect("writer");
        if let Some(writer) = writer.as_ref() {
            let _ = writer.send(WriteCommand::Gait {
                session_id,
                cycles: cycles.to_vec(),
                events: events.to_vec(),
            });
        }
    }

    /// Stores feedback records for the recording session, if any.
    pub fn record_haptics(&self, records: &[HapticRecord]) {
        if records.is_empty() {
            return;
        }
        let Some(session_id) = self.recording_session() else { return };
        let writer = self.writer.lock().expect("writer");
        if let Some(writer) = writer.as_ref() {
            let _ = writer.send(WriteCommand::Haptics { session_id, records: records.to_vec() });
        }
    }

    /// Every feedback record stored for a session, in time order.
    pub fn haptics(&self, session_id: &str) -> Result<Vec<StoredHaptic>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT device_time_us, event, cycle_start_frame, reason, motor_a, duty_a, motor_b,
                    duty_b, duration_ms, error_class, error_score, confidence
             FROM haptics WHERE session_id = ?1 ORDER BY device_time_us, event",
        )?;
        let rows = statement.query_map([session_id], |row| {
            Ok(StoredHaptic {
                device_time_us: row.get(0)?,
                event: row.get(1)?,
                cycle_start_frame: row.get(2)?,
                reason: row.get(3)?,
                motor_a: row.get(4)?,
                duty_a: row.get(5)?,
                motor_b: row.get(6)?,
                duty_b: row.get(7)?,
                duration_ms: row.get(8)?,
                error_class: row.get(9)?,
                error_score: row.get(10)?,
                confidence: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Every cycle stored for a session, in time order.
    pub fn cycles(&self, session_id: &str) -> Result<Vec<StoredCycle>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT start_frame, end_frame, start_us, cycle_time_s, stance_time_s, swing_time_s,
                    stance_ratio, swing_ratio, cadence, peak_shank_dps, peak_dorsi_deg,
                    contact_sag_deg, peak_inv_deg, distance_m, speed_mps, zupt_quality, valid,
                    confidence_subscores, deviations, error_score, confidence,
                    active_classes, primary_class, segment_index
             FROM cycles WHERE session_id = ?1 ORDER BY start_frame",
        )?;
        let rows = statement.query_map([session_id], |row| {
            let subscores: String = row.get(17)?;
            let deviations: String = row.get(18)?;
            Ok(StoredCycle {
                start_frame: row.get(0)?,
                end_frame: row.get(1)?,
                start_us: row.get(2)?,
                cycle_time_s: row.get(3)?,
                stance_time_s: row.get(4)?,
                swing_time_s: row.get(5)?,
                stance_ratio: row.get(6)?,
                swing_ratio: row.get(7)?,
                cadence_steps_per_min: row.get(8)?,
                peak_shank_rate_dps: row.get(9)?,
                peak_dorsiflexion_deg: row.get(10)?,
                contact_sagittal_deg: row.get(11)?,
                peak_inversion_deg: row.get(12)?,
                distance_m: row.get(13)?,
                speed_mps: row.get(14)?,
                zupt_quality: row.get(15)?,
                valid: row.get::<_, i64>(16)? != 0,
                confidence_subscores: serde_json::from_str(&subscores).unwrap_or_default(),
                deviations: serde_json::from_str(&deviations).unwrap_or_default(),
                error_score: row.get(19)?,
                confidence: row.get(20)?,
                active_classes: row.get::<_, i64>(21)? as u16,
                primary_class: row.get::<_, i64>(22)? as u8,
                segment_index: row.get(23)?,
                symmetry_proxy: None,
            })
        })?;
        let mut cycles = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        self.fill_symmetry_proxy(&connection, session_id, &mut cycles)?;
        Ok(cycles)
    }

    /// Fills in each valid cycle's repeatability against the previous one.
    /// Silent when the session had no reference: the normalization has no
    /// spreads to work from, and inventing some would make an unfounded number
    /// look like a measurement (doc 05 §10).
    fn fill_symmetry_proxy(
        &self,
        connection: &Connection,
        session_id: &str,
        cycles: &mut [StoredCycle],
    ) -> Result<()> {
        let reference_id: Option<String> = connection.query_row(
            "SELECT reference_id FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?;
        let Some(reference_id) = reference_id else { return Ok(()) };
        let profile = self.reference(&reference_id)?.profile;
        let mut spreads = [0.0f32; 7];
        for (spread, feature) in spreads.iter_mut().zip(profile.features.iter()) {
            *spread = feature.spread;
        }
        let mut previous: Option<StoredCycle> = None;
        for cycle in cycles.iter_mut() {
            if !cycle.valid {
                continue;
            }
            if let Some(last) = previous.as_ref() {
                cycle.symmetry_proxy = symmetry_proxy(cycle, last, &spreads);
            }
            previous = Some(cycle.clone());
        }
        Ok(())
    }

    /// Streams every stored frame of a session in frame order, handing each to
    /// `visit`. Streamed rather than collected: an hour at 200 Hz is 720 000
    /// frames, and the export writes them straight out.
    pub fn for_each_frame(
        &self,
        session_id: &str,
        mut visit: impl FnMut(&RawFrame),
    ) -> Result<usize> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT frame_index, timestamp_us,
                    fax, fay, faz, fgx, fgy, fgz,
                    sax, say, saz, sgx, sgy, sgz,
                    fqw, fqx, fqy, fqz, sqw, sqx, sqy, sqz, status,
                    rfw, rfx, rfy, rfz, rsw, rsx, rsy, rsz
             FROM raw_frames WHERE session_id = ?1 ORDER BY frame_index",
        )?;
        let mut rows = statement.query([session_id])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            let mut frame = RawFrame {
                frame_index: row.get::<_, i64>(0)? as u32,
                timestamp_us: row.get::<_, i64>(1)? as u64,
                status: row.get::<_, i64>(22)? as u16,
                ..Default::default()
            };
            for (i, value) in frame.foot.iter_mut().enumerate() {
                *value = row.get::<_, i64>(2 + i)? as i16;
            }
            for (i, value) in frame.shank.iter_mut().enumerate() {
                *value = row.get::<_, i64>(8 + i)? as i16;
            }
            for (i, value) in frame.q_foot.iter_mut().enumerate() {
                *value = row.get::<_, i64>(14 + i)? as i16;
            }
            for (i, value) in frame.q_shank.iter_mut().enumerate() {
                *value = row.get::<_, i64>(18 + i)? as i16;
            }
            // NULL in a frame recorded before device schema 6.
            let rotation = |first: usize| -> rusqlite::Result<Option<[i16; 4]>> {
                let mut q = [0i16; 4];
                for (i, value) in q.iter_mut().enumerate() {
                    match row.get::<_, Option<i64>>(first + i)? {
                        Some(v) => *value = v as i16,
                        None => return Ok(None),
                    }
                }
                Ok(Some(q))
            };
            frame.rv_foot = rotation(23)?;
            frame.rv_shank = rotation(27)?;
            visit(&frame);
            count += 1;
        }
        Ok(count)
    }

    /// Streams every stored accelerometer sample of a session, in device-time
    /// order, foot before shank at an equal time.
    pub fn for_each_accel(
        &self,
        session_id: &str,
        mut visit: impl FnMut(&AccelSample),
    ) -> Result<usize> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT timestamp_us, sensor, sequence, ax, ay, az
             FROM raw_accel WHERE session_id = ?1 ORDER BY timestamp_us, sensor, sequence",
        )?;
        let mut rows = statement.query([session_id])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            visit(&AccelSample {
                timestamp_us: row.get::<_, i64>(0)? as u64,
                sensor: row.get(1)?,
                sequence: row.get(2)?,
                accel: [row.get(3)?, row.get(4)?, row.get(5)?],
            });
            count += 1;
        }
        Ok(count)
    }

    /// The calibration record in force when a session started, as stored JSON.
    pub fn session_calibration(&self, session_id: &str) -> Result<Option<String>> {
        let connection = self.reader()?;
        Ok(connection.query_row(
            "SELECT calibration FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?)
    }

    /// Every status change recorded during a session, in order.
    pub fn status_changes(&self, session_id: &str) -> Result<Vec<StatusChange>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT frame_index, at, device_state, faults
             FROM status_changes WHERE session_id = ?1 ORDER BY frame_index",
        )?;
        let rows = statement.query_map([session_id], |row| {
            Ok(StatusChange {
                frame_index: row.get(0)?,
                at: row.get(1)?,
                device_state: row.get::<_, i64>(2)? as u8,
                faults: row.get::<_, i64>(3)? as u16,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Records a status whose state or fault mask differs from the last one
    /// stored. Called from the link task, so it writes through the writer
    /// thread like everything else.
    pub fn record_status(&self, status: &Status) {
        let Some(session_id) = self.recording_session() else { return };
        let mut last = self.last_status.lock().expect("last status");
        let current = (status.device_state, status.faults);
        if *last == Some(current) {
            return;
        }
        *last = Some(current);
        drop(last);
        let writer = self.writer.lock().expect("writer");
        if let Some(writer) = writer.as_ref() {
            let _ = writer.send(WriteCommand::Status {
                session_id,
                frame_index: i64::from(status.frame_index),
                device_state: status.device_state,
                faults: status.faults,
            });
        }
    }

    /// Every segment of a session, in order. Empty unless it was segmented.
    pub fn segments(&self, session_id: &str) -> Result<Vec<StoredSegment>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT segment_index, started_at, closed_at, closed_by, valid_cycles, errors
             FROM segments WHERE session_id = ?1 ORDER BY segment_index",
        )?;
        let rows = statement.query_map([session_id], |row| {
            Ok(StoredSegment {
                segment_index: row.get(0)?,
                started_at: row.get(1)?,
                closed_at: row.get(2)?,
                closed_by: row.get(3)?,
                valid_cycles: row.get(4)?,
                errors: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Every event stored for a session, in time order.
    pub fn events(&self, session_id: &str) -> Result<Vec<StoredEvent>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT frame_index, timestamp_us, kind FROM events
             WHERE session_id = ?1 ORDER BY timestamp_us",
        )?;
        let rows = statement.query_map([session_id], |row| {
            Ok(StoredEvent {
                frame_index: row.get(0)?,
                timestamp_us: row.get(1)?,
                kind: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Stores a profile the device built, as the next version for that patient.
    ///
    /// Versions are per patient and never reused, so an evaluation can always
    /// name the profile it was judged against even after a newer capture.
    pub fn add_reference(
        &self,
        patient_id: &str,
        session_id: Option<&str>,
        profile: &crate::protocol::ReferenceProfile,
    ) -> Result<StoredReference> {
        if profile.cycles < crate::protocol::REFERENCE_MIN_CYCLES {
            return Err(StoreError::Rejected(format!(
                "a reference needs at least {} valid cycles, got {}",
                crate::protocol::REFERENCE_MIN_CYCLES,
                profile.cycles
            )));
        }
        let connection = self.reader()?;
        let version: i64 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM reference_profiles WHERE patient_id = ?1",
            [patient_id],
            |row| row.get(0),
        )?;
        let created_at = now_utc();
        let reference_id = format!("{patient_id}-v{version}");
        let mut stored = *profile;
        stored.version = version as u16;
        connection.execute(
            "INSERT INTO reference_profiles
               (reference_id, patient_id, version, created_at, session_id, cycles, locked, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            rusqlite::params![
                reference_id,
                patient_id,
                version,
                created_at,
                session_id,
                stored.cycles as i64,
                crate::protocol::encode_reference(&stored),
            ],
        )?;
        Ok(StoredReference {
            reference_id,
            patient_id: patient_id.to_string(),
            version,
            created_at,
            session_id: session_id.map(str::to_string),
            cycles: stored.cycles as i64,
            locked: false,
            profile: stored,
        })
    }

    pub fn references(&self, patient_id: &str) -> Result<Vec<StoredReference>> {
        let connection = self.reader()?;
        let mut statement = connection.prepare(
            "SELECT reference_id, patient_id, version, created_at, session_id, cycles, locked,
                    payload
             FROM reference_profiles WHERE patient_id = ?1 ORDER BY version DESC",
        )?;
        let rows = statement.query_map([patient_id], |row| {
            let payload: Vec<u8> = row.get(7)?;
            Ok(StoredReference {
                reference_id: row.get(0)?,
                patient_id: row.get(1)?,
                version: row.get(2)?,
                created_at: row.get(3)?,
                session_id: row.get(4)?,
                cycles: row.get(5)?,
                locked: row.get::<_, i64>(6)? != 0,
                profile: crate::protocol::parse_reference(&payload).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn reference(&self, reference_id: &str) -> Result<StoredReference> {
        let connection = self.reader()?;
        let patient: String = connection
            .query_row(
                "SELECT patient_id FROM reference_profiles WHERE reference_id = ?1",
                [reference_id],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Rejected(format!("no reference {reference_id}")))?;
        self.references(&patient)?
            .into_iter()
            .find(|r| r.reference_id == reference_id)
            .ok_or_else(|| StoreError::Rejected(format!("no reference {reference_id}")))
    }

    /// Locks a profile. Called when it is first used to judge a session, after
    /// which the database itself refuses to change it.
    pub fn lock_reference(&self, reference_id: &str) -> Result<()> {
        let connection = self.reader()?;
        let changed = connection.execute(
            "UPDATE reference_profiles SET locked = 1 WHERE reference_id = ?1",
            [reference_id],
        )?;
        if changed == 0 {
            return Err(StoreError::Rejected(format!("no reference {reference_id}")));
        }
        Ok(())
    }

    /// Blocks until every queued write is committed. An error when they were
    /// not: a commit is failing, data was lost, or the writer did not answer.
    pub fn flush(&self) -> Result<()> {
        {
            let guard = self.writer.lock().expect("writer");
            let Some(writer) = guard.as_ref() else { return Ok(()) };
            let (tx, rx) = mpsc::channel();
            if writer.send(WriteCommand::Flush(tx)).is_err() {
                return Err(StoreError::Rejected("the database writer has stopped".into()));
            }
            // Not held while waiting: recording threads queue through it.
            drop(guard);
            if rx.recv_timeout(Duration::from_secs(10)).is_err() {
                return Err(StoreError::Rejected(
                    "the database writer did not finish within 10 s".into(),
                ));
            }
        }
        match self.write_problem() {
            Some(problem) => Err(StoreError::Rejected(problem)),
            None => Ok(()),
        }
    }

    /// Why recorded data is not (all) in the database, if it is not: commits
    /// failing now, or rows dropped since the recording started.
    pub fn write_problem(&self) -> Option<String> {
        let health = self.health.lock().expect("write health");
        if health.lost_rows > 0 {
            return Some(format!(
                "{} frames and accelerometer samples of this recording could not be saved: {}",
                health.lost_rows,
                health.lost_because.as_deref().unwrap_or("unknown error")
            ));
        }
        health
            .failing
            .as_ref()
            .map(|e| format!("saving to the database is failing, data is held and retried: {e}"))
    }

    /// Decimated signal window for the raw view, in anatomical physical units
    /// when the session recorded the device configuration.
    pub fn raw_window(
        &self,
        session_id: &str,
        groups: &[SignalGroup],
        first_frame: i64,
        last_frame: i64,
        max_points: usize,
    ) -> Result<RawWindow> {
        let stored = self.session_config(session_id)?;
        let config = stored.and_then(|section| section.parse().ok());
        let connection = self.reader()?;
        raw::read_window(
            &connection,
            session_id,
            groups,
            first_frame,
            last_frame,
            max_points,
            config.as_ref(),
        )
    }

    /// The device configuration stored with a session, if the device reported
    /// one when it started.
    pub fn session_config(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::protocol::ConfigSection>> {
        let connection = self.reader()?;
        let (bytes, format): (Option<Vec<u8>>, u16) = connection.query_row(
            "SELECT config_section, config_format FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(bytes.map(|bytes| crate::protocol::ConfigSection { format, bytes }))
    }

    #[cfg(test)]
    pub fn accel_count(&self, session_id: &str) -> Result<i64> {
        let connection = self.reader()?;
        Ok(connection.query_row(
            "SELECT COUNT(*) FROM raw_accel WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?)
    }

    #[cfg(test)]
    pub fn frame_count(&self, session_id: &str) -> Result<i64> {
        let connection = self.reader()?;
        Ok(connection.query_row(
            "SELECT COUNT(*) FROM raw_frames WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.lock().expect("writer").take() {
            let _ = writer.send(WriteCommand::Stop);
        }
    }
}

fn session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    let first: Option<i64> = row.get(10)?;
    let last: Option<i64> = row.get(11)?;
    let stored: i64 = row.get(12)?;
    let missing = match (first, last) {
        (Some(first), Some(last)) => (last - first + 1 - stored).max(0),
        _ => 0,
    };
    Ok(Session {
        session_id: row.get(0)?,
        patient_id: row.get(1)?,
        patient_name: row.get(2)?,
        kind: row.get(3)?,
        started_at: row.get(4)?,
        stopped_at: row.get(5)?,
        firmware: row.get(6)?,
        config_sha256: row.get(7)?,
        device_mac: row.get(8)?,
        boot_id: row.get(9)?,
        first_frame_index: first,
        last_frame_index: last,
        frames_stored: stored,
        frames_missing: missing,
        reference_id: row.get(13)?,
        max_cycles_per_segment: row.get(14)?,
        max_errors_per_segment: row.get(15)?,
        label: row.get(16)?,
        payload_schema: row.get(17)?,
    })
}

/// Writes waiting for the next commit.
#[derive(Default)]
struct Pending {
    frames: Vec<(String, Vec<RawFrame>)>,
    accel: Vec<(String, Vec<AccelSample>)>,
    gait: Vec<(String, Vec<GaitCycle>, Vec<GaitEvent>)>,
    status: Vec<(String, i64, u8, u16)>,
    haptics: Vec<(String, Vec<HapticRecord>)>,
}

impl Pending {
    fn is_empty(&self) -> bool {
        self.frames.is_empty()
            && self.accel.is_empty()
            && self.gait.is_empty()
            && self.status.is_empty()
            && self.haptics.is_empty()
    }

    fn rows(&self) -> usize {
        self.frames.iter().map(|(_, f)| f.len()).sum::<usize>()
            + self.accel.iter().map(|(_, a)| a.len()).sum::<usize>()
    }

    /// Drops the oldest frame and accelerometer batches until at most `limit`
    /// rows remain; returns how many rows went. Cycles, events, status and
    /// haptics are a few rows a second and are kept.
    fn trim(&mut self, limit: usize) -> usize {
        let mut dropped = 0;
        while self.rows() > limit {
            // From the longer queue, so both streams keep their most recent data.
            if !self.frames.is_empty() && self.frames.len() >= self.accel.len() {
                dropped += self.frames.remove(0).1.len();
            } else if !self.accel.is_empty() {
                dropped += self.accel.remove(0).1.len();
            } else {
                break;
            }
        }
        dropped
    }
}

fn writer_loop(
    mut connection: Connection,
    rx: mpsc::Receiver<WriteCommand>,
    health: Arc<Mutex<WriteHealth>>,
) {
    let mut pending = Pending::default();
    let mut last_commit = Instant::now();
    loop {
        match rx.recv_timeout(COMMIT_INTERVAL) {
            Ok(WriteCommand::Haptics { session_id, records }) => {
                pending.haptics.push((session_id, records));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Gait { session_id, cycles, events }) => {
                pending.gait.push((session_id, cycles, events));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Frames { session_id, frames }) => {
                pending.frames.push((session_id, frames));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Accel { session_id, samples }) => {
                pending.accel.push((session_id, samples));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Status { session_id, frame_index, device_state, faults }) => {
                pending.status.push((session_id, frame_index, device_state, faults));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Flush(done)) => {
                commit(&mut connection, &mut pending, &health);
                last_commit = Instant::now();
                let _ = done.send(());
                continue;
            }
            Ok(WriteCommand::Stop) => {
                commit(&mut connection, &mut pending, &health);
                return;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                commit(&mut connection, &mut pending, &health);
                return;
            }
        }
        commit(&mut connection, &mut pending, &health);
        last_commit = Instant::now();
    }
}

/// Commits everything pending in one transaction. On failure nothing is lost
/// yet: the batches stay pending and the next commit retries them (the
/// transaction rolled back, so a retry writes each row once). Until 2026-10-05 a
/// failure was printed to stderr and the batches thrown away (PROB-029).
fn commit(connection: &mut Connection, pending: &mut Pending, health: &Mutex<WriteHealth>) {
    if pending.is_empty() {
        return;
    }
    let result = write_pending(connection, pending);
    let mut health = health.lock().expect("write health");
    match result {
        Ok(()) => {
            *pending = Pending::default();
            health.failing = None;
        }
        Err(err) => {
            let dropped = pending.trim(MAX_RETAINED_ROWS);
            if dropped > 0 {
                health.lost_rows += dropped;
                health.lost_because = Some(err.to_string());
            }
            health.failing = Some(err.to_string());
        }
    }
}

fn write_pending(connection: &mut Connection, pending: &Pending) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    {
        let mut insert = transaction.prepare_cached(schema::INSERT_FRAME)?;
        for (session_id, frames) in pending.frames.iter() {
            for frame in frames {
                // INSERT OR IGNORE: a backfilled frame may already be stored.
                schema::insert_frame(&mut insert, session_id, frame)?;
            }
        }
        let mut insert = transaction.prepare_cached(schema::INSERT_ACCEL)?;
        for (session_id, samples) in pending.accel.iter() {
            for sample in samples {
                schema::insert_accel(&mut insert, session_id, sample)?;
            }
        }
    }
    {
        let mut cycle = transaction.prepare_cached(schema::INSERT_CYCLE)?;
        let mut event = transaction.prepare_cached(schema::INSERT_EVENT)?;
        for (session_id, cycles, events) in pending.gait.iter() {
            for c in cycles {
                let segment = roll_segment(&transaction, session_id, c)?;
                // OR REPLACE: a backfilled cycle is the same measurement.
                cycle.execute(rusqlite::params![
                    session_id, c.start_frame, c.end_frame, c.start_us as i64,
                    c.cycle_time_s, c.stance_time_s, c.swing_time_s, c.stance_ratio,
                    c.swing_ratio, c.cadence_steps_per_min, c.peak_shank_rate_dps,
                    c.peak_dorsiflexion_deg, c.contact_sagittal_deg, c.peak_inversion_deg,
                    c.distance_m, c.speed_mps, c.zupt_quality, c.valid as i64,
                    c.error_score, c.confidence, c.active_classes as i64,
                    c.primary_class as i64,
                    serde_json::to_string(&c.confidence_subscores).unwrap_or_default(),
                    serde_json::to_string(&c.deviations).unwrap_or_default(),
                    segment,
                ])?;
            }
            for e in events {
                event.execute(rusqlite::params![
                    session_id, e.frame_index, e.timestamp_us as i64, e.event_type.name(),
                ])?;
            }
        }
    }
    {
        let mut insert = transaction.prepare_cached(schema::INSERT_HAPTIC)?;
        for (session_id, records) in pending.haptics.iter() {
            for h in records {
                let motor = |i: usize| h.motors.get(i).copied().unwrap_or((0, 0));
                let (motor_a, duty_a) = motor(0);
                let (motor_b, duty_b) = motor(1);
                insert.execute(rusqlite::params![
                    session_id, h.device_time_us as i64, h.event, h.cycle_start_frame,
                    h.reason, motor_a, duty_a, motor_b, duty_b, h.duration_ms,
                    h.error_class, h.error_score, h.confidence,
                ])?;
            }
        }
    }
    {
        let mut status = transaction.prepare_cached(schema::INSERT_STATUS_CHANGE)?;
        for (session_id, frame_index, device_state, faults) in pending.status.iter() {
            status.execute(rusqlite::params![
                session_id,
                frame_index,
                now_utc(),
                i64::from(*device_state),
                i64::from(*faults),
            ])?;
        }
    }
    transaction.commit()
}

/// Closes sessions left open when the app last exited mid-recording.
///
/// Nothing can be recording at startup, so any session without a stop time
/// was interrupted — and would otherwise read "recording" forever. It is given
/// the stop time its own data supports: the start plus the device-time span of
/// its frames, or the start itself if none were stored. Its open segment is
/// closed as `session_stopped` for the same reason.
fn close_orphaned_sessions(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "UPDATE segments SET closed_at = COALESCE(
             (SELECT strftime('%Y-%m-%dT%H:%M:%fZ', julianday(s.started_at)
                 + COALESCE((SELECT (MAX(timestamp_us) - MIN(timestamp_us)) / 86400e6
                             FROM raw_frames f WHERE f.session_id = s.session_id), 0))
              FROM sessions s WHERE s.session_id = segments.session_id), started_at),
             closed_by = 'session_stopped'
         WHERE closed_at IS NULL
           AND session_id IN (SELECT session_id FROM sessions WHERE stopped_at IS NULL);
         UPDATE sessions SET stopped_at = strftime('%Y-%m-%dT%H:%M:%fZ', julianday(started_at)
                 + COALESCE((SELECT (MAX(timestamp_us) - MIN(timestamp_us)) / 86400e6
                             FROM raw_frames f WHERE f.session_id = sessions.session_id), 0))
         WHERE stopped_at IS NULL;",
    )
}

/// Counts a cycle into the session's open segment and closes that segment when
/// either researcher-entered limit is reached (doc 12 §5, DEC-014). Returns the
/// segment the cycle belongs to — the one that was open when it arrived, so the
/// cycle that trips a limit belongs to the segment it closed.
///
/// Returns 0 without touching the table for an unsegmented session, which is
/// every recording, capture and check.
///
/// ponytail: four small statements per cycle rather than a cached counter.
/// Cycles arrive at roughly 1 Hz, and reading the row back each time means a
/// backfilled batch cannot double-count against a stale in-memory total.
fn roll_segment(
    transaction: &rusqlite::Transaction<'_>,
    session_id: &str,
    cycle: &GaitCycle,
) -> rusqlite::Result<i64> {
    let limits: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT max_cycles_per_segment, max_errors_per_segment
             FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .map(|(cycles, errors)| cycles.zip(errors))?;
    let Some((max_cycles, max_errors)) = limits else { return Ok(0) };

    let open: Option<(i64, i64, i64)> = transaction
        .query_row(
            "SELECT segment_index, valid_cycles, errors FROM segments
             WHERE session_id = ?1 AND closed_at IS NULL
             ORDER BY segment_index LIMIT 1",
            [session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    // Nothing open means the session was stopped between the cycle arriving and
    // this commit; attribute it to the last segment rather than reopening one.
    let Some((index, valid_cycles, errors)) = open else {
        return transaction
            .query_row(
                "SELECT MAX(segment_index) FROM segments WHERE session_id = ?1",
                [session_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map(|last| last.unwrap_or(0));
    };

    let valid_cycles = valid_cycles + i64::from(cycle.valid);
    let errors = errors + i64::from(counts_as_error(cycle));
    transaction.execute(
        "UPDATE segments SET valid_cycles = ?3, errors = ?4
         WHERE session_id = ?1 AND segment_index = ?2",
        rusqlite::params![session_id, index, valid_cycles, errors],
    )?;

    // Whichever limit is reached first closes the segment (doc 12 §5).
    let closed_by = if valid_cycles >= max_cycles {
        Some("cycle_limit")
    } else if errors >= max_errors {
        Some("error_limit")
    } else {
        None
    };
    if let Some(reason) = closed_by {
        transaction.execute(
            "UPDATE segments SET closed_at = ?3, closed_by = ?4
             WHERE session_id = ?1 AND segment_index = ?2",
            rusqlite::params![session_id, index, now_utc(), reason],
        )?;
        transaction.execute(
            "INSERT INTO segments (session_id, segment_index, started_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![session_id, index + 1, now_utc()],
        )?;
    }
    Ok(index)
}

fn now_utc() -> String {
    // RFC 3339 in UTC, computed from the system clock without a date crate.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format_utc(now.as_secs(), now.subsec_millis())
}

fn format_utc(unix_seconds: u64, millis: u32) -> String {
    let (year, month, day, hour, minute, second) = civil_from_unix(unix_seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Days-from-civil algorithm (Howard Hinnant), valid for all Gregorian dates.
fn civil_from_unix(unix_seconds: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (unix_seconds / 86_400) as i64;
    let seconds_of_day = (unix_seconds % 86_400) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d, seconds_of_day / 3600, (seconds_of_day / 60) % 60, seconds_of_day % 60)
}

/// `YYYYMMDD-HHMMSS-<4hex>` (doc 10 §1), unique within the store.
fn new_session_id(connection: &Connection) -> Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let (year, month, day, hour, minute, second) = civil_from_unix(now.as_secs());
    let stamp = format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}");
    for attempt in 0..16u32 {
        // Suffix from the sub-second clock, varied per attempt on collision.
        let suffix = (now.subsec_nanos() ^ attempt.wrapping_mul(0x9E37)) & 0xFFFF;
        let candidate = format!("{stamp}-{suffix:04x}");
        let taken: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sessions WHERE session_id = ?1",
            [&candidate],
            |row| row.get(0),
        )?;
        if taken == 0 {
            return Ok(candidate);
        }
    }
    Err(StoreError::Rejected("could not allocate a session ID".into()))
}
