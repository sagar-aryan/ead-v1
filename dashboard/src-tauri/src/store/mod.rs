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

use rusqlite::Connection;

use crate::protocol::{GaitCycle, GaitEvent, RawFrame};

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
}

impl SessionKind {
    fn as_str(self) -> &'static str {
        match self {
            SessionKind::Recording => "recording",
        }
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
}

/// Device identity recorded with a session, so every dataset carries the
/// firmware and configuration that produced it (doc 18).
#[derive(Debug, Clone, Default)]
pub struct DeviceIdentity {
    pub firmware: Option<String>,
    pub config_sha256: Option<String>,
    pub mac: Option<String>,
    pub boot_id: Option<u32>,
    /// The device's configuration section, stored so a recording of raw counts
    /// is self-describing (`docs/protocol.md` §5.5).
    pub config_section: Option<Vec<u8>>,
}

/// A gait cycle as stored, in physical units (doc 05 §11).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
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
    Gait { session_id: String, cycles: Vec<GaitCycle>, events: Vec<GaitEvent> },
    Flush(mpsc::Sender<()>),
    Stop,
}

pub struct Store {
    path: PathBuf,
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

        let (tx, rx) = mpsc::channel::<WriteCommand>();
        std::thread::Builder::new()
            .name("store-writer".into())
            .spawn(move || writer_loop(connection, rx))
            .expect("spawn store writer");

        Ok(Arc::new(Self {
            path,
            writer: Mutex::new(Some(tx)),
            recording: Mutex::new(None),
        }))
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

    pub fn start_session(
        &self,
        patient_id: &str,
        kind: SessionKind,
        identity: &DeviceIdentity,
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
                boot_id, config_section)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ],
        )?;
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
                    MIN(f.frame_index), MAX(f.frame_index), COUNT(f.frame_index)
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
                    MIN(f.frame_index), MAX(f.frame_index), COUNT(f.frame_index)
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
                    contact_sag_deg, peak_inv_deg, distance_m, speed_mps, zupt_quality, valid
             FROM cycles WHERE session_id = ?1 ORDER BY start_frame",
        )?;
        let rows = statement.query_map([session_id], |row| {
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
    })
}

fn writer_loop(mut connection: Connection, rx: mpsc::Receiver<WriteCommand>) {
    let mut pending: Vec<(String, Vec<RawFrame>)> = Vec::new();
    let mut pending_gait: PendingGait = Vec::new();
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
            Ok(WriteCommand::Flush(done)) => {
                commit(&mut connection, &mut pending, &mut pending_gait);
                last_commit = Instant::now();
                let _ = done.send(());
                continue;
            }
            Ok(WriteCommand::Stop) => {
                commit(&mut connection, &mut pending, &mut pending_gait);
                return;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                commit(&mut connection, &mut pending, &mut pending_gait);
                return;
            }
        }
        commit(&mut connection, &mut pending, &mut pending_gait);
        last_commit = Instant::now();
    }
}

type PendingGait = Vec<(String, Vec<GaitCycle>, Vec<GaitEvent>)>;

fn commit(
    connection: &mut Connection,
    pending: &mut Vec<(String, Vec<RawFrame>)>,
    pending_gait: &mut PendingGait,
) {
    if pending.is_empty() && pending_gait.is_empty() {
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
                    // OR REPLACE: a backfilled cycle is the same measurement.
                    cycle.execute(rusqlite::params![
                        session_id, c.start_frame, c.end_frame, c.start_us as i64,
                        c.cycle_time_s, c.stance_time_s, c.swing_time_s, c.stance_ratio,
                        c.swing_ratio, c.cadence_steps_per_min, c.peak_shank_rate_dps,
                        c.peak_dorsiflexion_deg, c.contact_sagittal_deg, c.peak_inversion_deg,
                        c.distance_m, c.speed_mps, c.zupt_quality, c.valid as i64,
                    ])?;
                }
                for e in events {
                    event.execute(rusqlite::params![
                        session_id, e.frame_index, e.timestamp_us as i64, e.event_type.name(),
                    ])?;
                }
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
