//! Session store. Raw frames are canonical (doc 01 §7, DEC-007): they are
//! written exactly as the device sent them, in chip-frame ADC counts, and every
//! derived value is computed from them.
//!
//! One writer thread owns the connection; readers open their own. Writes are
//! batched into transactions so a 100 Hz stream costs a few commits per second.

pub mod raw;
mod schema;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension};

use crate::protocol::{GaitCycle, GaitEvent, RawFrame, Status};

pub use raw::{RawWindow, SignalGroup};

/// At most this long between a frame arriving and being durable.
const COMMIT_INTERVAL: Duration = Duration::from_millis(250);

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
    /// `cycle_limit`, `error_limit` or `session_stopped`; null while open.
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
    /// The device's configuration section, stored so a recording of raw counts
    /// is self-describing (`docs/protocol.md` §5.5).
    pub config_section: Option<Vec<u8>>,
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

enum WriteCommand {
    Frames { session_id: String, frames: Vec<RawFrame> },
    Status { session_id: String, frame_index: i64, device_state: u8, faults: u16 },
    Gait { session_id: String, cycles: Vec<GaitCycle>, events: Vec<GaitEvent> },
    Flush(mpsc::Sender<()>),
    Stop,
}

pub struct Store {
    path: PathBuf,
    /// The last (state, faults) written, so only changes are stored.
    last_status: Mutex<Option<(u8, u16)>>,
    writer: Mutex<Option<mpsc::Sender<WriteCommand>>>,
    /// Session currently recording, if any.
    recording: Mutex<Option<String>>,
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
        std::thread::Builder::new()
            .name("store-writer".into())
            .spawn(move || writer_loop(connection, rx))
            .expect("spawn store writer");

        Ok(Arc::new(Self {
            path,
            last_status: Mutex::new(None),
            writer: Mutex::new(Some(tx)),
            recording: Mutex::new(None),
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
        let session_id = new_session_id(&connection)?;
        connection.execute(
            "INSERT INTO sessions
               (session_id, patient_id, kind, started_at, firmware, config_sha256, device_mac,
                boot_id, config_section, reference_id, max_cycles_per_segment,
                max_errors_per_segment, calibration)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                &session_id,
                patient_id,
                kind.as_str(),
                now_utc(),
                &identity.firmware,
                &identity.config_sha256,
                &identity.mac,
                identity.boot_id.map(i64::from),
                &identity.config_section,
                reference_id,
                limits.map(|l| i64::from(l.max_cycles)),
                limits.map(|l| i64::from(l.max_errors)),
                &identity.calibration,
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
        *recording = Some(session_id.clone());
        drop(recording);
        self.session(&session_id)
    }

    pub fn stop_session(&self) -> Result<Option<Session>> {
        let session_id = match self.recording.lock().expect("recording").take() {
            Some(id) => id,
            None => return Ok(None),
        };
        self.flush();
        let connection = self.reader()?;
        connection.execute(
            "UPDATE sessions SET stopped_at = ?2 WHERE session_id = ?1",
            (&session_id, now_utc()),
        )?;
        connection.execute(
            "UPDATE segments SET closed_at = ?2, closed_by = 'session_stopped'
             WHERE session_id = ?1 AND closed_at IS NULL",
            (&session_id, now_utc()),
        )?;
        self.session(&session_id).map(Some)
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
                    s.reference_id, s.max_cycles_per_segment, s.max_errors_per_segment
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
                    s.reference_id, s.max_cycles_per_segment, s.max_errors_per_segment
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
    /// `visit`. Streamed rather than collected: an hour at 100 Hz is 360 000
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
                    fqw, fqx, fqy, fqz, sqw, sqx, sqy, sqz, status
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
            visit(&frame);
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

    /// Blocks until every queued write is committed.
    pub fn flush(&self) {
        let writer = self.writer.lock().expect("writer");
        let Some(writer) = writer.as_ref() else { return };
        let (tx, rx) = mpsc::channel();
        if writer.send(WriteCommand::Flush(tx)).is_ok() {
            let _ = rx.recv_timeout(Duration::from_secs(10));
        }
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
        let config = match stored.as_deref() {
            Some(bytes) => crate::protocol::parse_section(bytes).ok(),
            None => None,
        };
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
    pub fn session_config(&self, session_id: &str) -> Result<Option<Vec<u8>>> {
        let connection = self.reader()?;
        Ok(connection.query_row(
            "SELECT config_section FROM sessions WHERE session_id = ?1",
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
    })
}

fn writer_loop(mut connection: Connection, rx: mpsc::Receiver<WriteCommand>) {
    let mut pending: Vec<(String, Vec<RawFrame>)> = Vec::new();
    let mut pending_gait: PendingGait = Vec::new();
    let mut pending_status: Vec<(String, i64, u8, u16)> = Vec::new();
    let mut last_commit = Instant::now();
    loop {
        match rx.recv_timeout(COMMIT_INTERVAL) {
            Ok(WriteCommand::Gait { session_id, cycles, events }) => {
                pending_gait.push((session_id, cycles, events));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Frames { session_id, frames }) => {
                pending.push((session_id, frames));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Status { session_id, frame_index, device_state, faults }) => {
                pending_status.push((session_id, frame_index, device_state, faults));
                if last_commit.elapsed() < COMMIT_INTERVAL {
                    continue;
                }
            }
            Ok(WriteCommand::Flush(done)) => {
                commit(&mut connection, &mut pending, &mut pending_gait, &mut pending_status);
                last_commit = Instant::now();
                let _ = done.send(());
                continue;
            }
            Ok(WriteCommand::Stop) => {
                commit(&mut connection, &mut pending, &mut pending_gait, &mut pending_status);
                return;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                commit(&mut connection, &mut pending, &mut pending_gait, &mut pending_status);
                return;
            }
        }
        commit(&mut connection, &mut pending, &mut pending_gait, &mut pending_status);
        last_commit = Instant::now();
    }
}

type PendingGait = Vec<(String, Vec<GaitCycle>, Vec<GaitEvent>)>;

fn commit(
    connection: &mut Connection,
    pending: &mut Vec<(String, Vec<RawFrame>)>,
    pending_gait: &mut PendingGait,
    pending_status: &mut Vec<(String, i64, u8, u16)>,
) {
    if pending.is_empty() && pending_gait.is_empty() && pending_status.is_empty() {
        return;
    }
    let result = (|| -> rusqlite::Result<()> {
        let transaction = connection.transaction()?;
        {
            let mut insert = transaction.prepare_cached(schema::INSERT_FRAME)?;
            for (session_id, frames) in pending.iter() {
                for frame in frames {
                    // INSERT OR IGNORE: a backfilled frame may already be stored.
                    schema::insert_frame(&mut insert, session_id, frame)?;
                }
            }
        }
        {
            let mut cycle = transaction.prepare_cached(schema::INSERT_CYCLE)?;
            let mut event = transaction.prepare_cached(schema::INSERT_EVENT)?;
            for (session_id, cycles, events) in pending_gait.iter() {
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
            let mut status = transaction.prepare_cached(schema::INSERT_STATUS_CHANGE)?;
            for (session_id, frame_index, device_state, faults) in pending_status.iter() {
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
    })();
    if let Err(err) = result {
        // Losing raw research data silently is not acceptable; surface it.
        eprintln!("store: commit failed, {} batches dropped: {err}", pending.len());
    }
    pending.clear();
    pending_gait.clear();
    pending_status.clear();
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
