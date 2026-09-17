//! Database schema and the raw-frame row mapping.
//!
//! `user_version` records the schema version, so an older store is detected
//! rather than silently misread (doc 18: version every persisted format).

use rusqlite::{Connection, Result};

use crate::protocol::RawFrame;

pub const SCHEMA_VERSION: i32 = 5;

pub fn migrate(connection: &mut Connection) -> Result<()> {
    // WAL keeps readers (UI queries) from blocking the writer thread.
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "busy_timeout", 5000)?;

    let mut version: i32 =
        connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap_or(0);
    if version > SCHEMA_VERSION {
        // Refuse rather than risk misreading a newer layout.
        return Err(rusqlite::Error::InvalidQuery);
    }
    // A new store is created at the current schema; CREATE_SCHEMA is always the
    // latest layout, so the migrations below never run against it.
    if version == 0 {
        let transaction = connection.transaction()?;
        transaction.execute_batch(CREATE_SCHEMA)?;
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        transaction.commit()?;
        return Ok(());
    }
    // An existing store is upgraded one step at a time, so research data is
    // never dropped to simplify a schema change.
    while version < SCHEMA_VERSION {
        let transaction = connection.transaction()?;
        match version {
            1 => transaction.execute_batch(MIGRATE_1_TO_2)?,
            2 => transaction.execute_batch(MIGRATE_2_TO_3)?,
            3 => transaction.execute_batch(MIGRATE_3_TO_4)?,
            4 => transaction.execute_batch(MIGRATE_4_TO_5)?,
            other => unreachable!("no migration from schema {other}"),
        }
        version += 1;
        transaction.pragma_update(None, "user_version", version)?;
        transaction.commit()?;
    }
    Ok(())
}

const CREATE_SCHEMA: &str = r#"
CREATE TABLE patients (
  patient_id  TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  created_at  TEXT NOT NULL
);

CREATE TABLE sessions (
  session_id     TEXT PRIMARY KEY,          -- YYYYMMDD-HHMMSS-<4hex>, doc 10 §1
  patient_id     TEXT NOT NULL REFERENCES patients(patient_id),
  kind           TEXT NOT NULL,
  started_at     TEXT NOT NULL,             -- host UTC; device time stays in the frames
  stopped_at     TEXT,
  firmware       TEXT,
  config_sha256  TEXT,
  device_mac     TEXT,
  boot_id        INTEGER,
  -- The device's own configuration section as sent (docs/protocol.md §5.5).
  -- Raw counts mean nothing without its scale factors and mount maps, so each
  -- recording carries the description of the device that produced it.
  config_section BLOB
);

CREATE INDEX sessions_by_patient ON sessions(patient_id, started_at DESC);

-- Raw frames exactly as received: chip-frame ADC counts and Q15 quaternions
-- (doc 09 §6, DEC-007). Mount maps and scale factors live in the session's
-- device configuration, so a corrected map never invalidates a recording.
CREATE TABLE raw_frames (
  session_id   TEXT NOT NULL REFERENCES sessions(session_id),
  frame_index  INTEGER NOT NULL,
  timestamp_us INTEGER NOT NULL,
  fax INTEGER NOT NULL, fay INTEGER NOT NULL, faz INTEGER NOT NULL,
  fgx INTEGER NOT NULL, fgy INTEGER NOT NULL, fgz INTEGER NOT NULL,
  sax INTEGER NOT NULL, say INTEGER NOT NULL, saz INTEGER NOT NULL,
  sgx INTEGER NOT NULL, sgy INTEGER NOT NULL, sgz INTEGER NOT NULL,
  fqw INTEGER NOT NULL, fqx INTEGER NOT NULL, fqy INTEGER NOT NULL, fqz INTEGER NOT NULL,
  sqw INTEGER NOT NULL, sqx INTEGER NOT NULL, sqy INTEGER NOT NULL, sqz INTEGER NOT NULL,
  status INTEGER NOT NULL,
  PRIMARY KEY (session_id, frame_index)
) WITHOUT ROWID;

-- One row per completed gait cycle, as the device measured it (doc 05 §11).
-- Derived values, not raw data: a corrected detector produces different rows for
-- the same recording, which is why the frames are kept separately.
CREATE TABLE cycles (
  session_id       TEXT NOT NULL REFERENCES sessions(session_id),
  start_frame      INTEGER NOT NULL,
  end_frame        INTEGER NOT NULL,
  start_us         INTEGER NOT NULL,
  cycle_time_s     REAL NOT NULL,
  stance_time_s    REAL NOT NULL,
  swing_time_s     REAL NOT NULL,
  stance_ratio     REAL NOT NULL,
  swing_ratio      REAL NOT NULL,
  cadence          REAL NOT NULL,
  peak_shank_dps   REAL NOT NULL,
  peak_dorsi_deg   REAL NOT NULL,
  contact_sag_deg  REAL NOT NULL,
  peak_inv_deg     REAL NOT NULL,
  distance_m       REAL NOT NULL,
  speed_mps        REAL NOT NULL,
  zupt_quality     REAL NOT NULL,
  valid            INTEGER NOT NULL,
  PRIMARY KEY (session_id, start_frame)
) WITHOUT ROWID;

-- Gait events, keyed by the frame they are attributed to. An event can refer to
-- a frame already stored: initial contact is timestamped at the strongest impact
-- inside its window, not at the moment the detector decided (doc 05 §3).
CREATE TABLE events (
  session_id   TEXT NOT NULL REFERENCES sessions(session_id),
  frame_index  INTEGER NOT NULL,
  timestamp_us INTEGER NOT NULL,
  kind         TEXT NOT NULL,
  PRIMARY KEY (session_id, frame_index, kind)
) WITHOUT ROWID;

-- Error engine output, added in schema 4. The score, confidence and class are
-- columns because they are what a reader filters and plots on; the subscores
-- and per-feature deviations are JSON because they are only ever read whole.
-- All zero means the cycle was not scored — no reference was loaded — which is
-- not the same as agreeing with the reference.
ALTER TABLE cycles ADD COLUMN error_score REAL NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN confidence REAL NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN active_classes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN primary_class INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN confidence_subscores TEXT NOT NULL DEFAULT '[]';
ALTER TABLE cycles ADD COLUMN deviations TEXT NOT NULL DEFAULT '[]';

-- Versioned, patient-specific reference profiles (doc 12 §2-§3). A profile is
-- immutable once locked: a trigger refuses the update rather than trusting
-- every caller to remember, because a reference edited after use would silently
-- invalidate every evaluation made against it.
CREATE TABLE reference_profiles (
  reference_id TEXT PRIMARY KEY,
  patient_id   TEXT NOT NULL REFERENCES patients(patient_id),
  version      INTEGER NOT NULL,
  created_at   TEXT NOT NULL,
  session_id   TEXT REFERENCES sessions(session_id),
  cycles       INTEGER NOT NULL,
  locked       INTEGER NOT NULL DEFAULT 0,
  -- The 64-byte profile exactly as the device sent it (docs/protocol.md §5.13),
  -- so what is sent back for an evaluation is what was captured.
  payload      BLOB NOT NULL,
  UNIQUE (patient_id, version)
);

CREATE TRIGGER reference_profiles_locked
BEFORE UPDATE ON reference_profiles
WHEN old.locked = 1 AND (new.payload IS NOT old.payload OR new.cycles IS NOT old.cycles)
BEGIN
  SELECT RAISE(ABORT, 'a locked reference profile cannot be modified');
END;

-- Segments (doc 12 §5), added in schema 5. The researcher enters both limits
-- before an evaluation may start and the segment closes on whichever is reached
-- first. Segmentation is host-side (DEC-014): nothing on the device changes
-- behaviour at a segment boundary in V1, so the counter lives where the limits
-- were entered and a replay of the stored cycles reproduces it exactly.
ALTER TABLE sessions ADD COLUMN reference_id TEXT REFERENCES reference_profiles(reference_id);
ALTER TABLE sessions ADD COLUMN max_cycles_per_segment INTEGER;
ALTER TABLE sessions ADD COLUMN max_errors_per_segment INTEGER;

ALTER TABLE cycles ADD COLUMN segment_index INTEGER NOT NULL DEFAULT 0;

CREATE TABLE segments (
  session_id    TEXT NOT NULL REFERENCES sessions(session_id),
  segment_index INTEGER NOT NULL,
  started_at    TEXT NOT NULL,
  closed_at     TEXT,
  -- 'cycle_limit', 'error_limit' or 'session_stopped'; null while open.
  closed_by     TEXT,
  valid_cycles  INTEGER NOT NULL DEFAULT 0,
  errors        INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (session_id, segment_index)
) WITHOUT ROWID;
"#;

const MIGRATE_4_TO_5: &str = r#"
-- Segments (doc 12 §5), added in schema 5. The researcher enters both limits
-- before an evaluation may start and the segment closes on whichever is reached
-- first. Segmentation is host-side (DEC-014): nothing on the device changes
-- behaviour at a segment boundary in V1, so the counter lives where the limits
-- were entered and a replay of the stored cycles reproduces it exactly.
ALTER TABLE sessions ADD COLUMN reference_id TEXT REFERENCES reference_profiles(reference_id);
ALTER TABLE sessions ADD COLUMN max_cycles_per_segment INTEGER;
ALTER TABLE sessions ADD COLUMN max_errors_per_segment INTEGER;

ALTER TABLE cycles ADD COLUMN segment_index INTEGER NOT NULL DEFAULT 0;

CREATE TABLE segments (
  session_id    TEXT NOT NULL REFERENCES sessions(session_id),
  segment_index INTEGER NOT NULL,
  started_at    TEXT NOT NULL,
  closed_at     TEXT,
  -- 'cycle_limit', 'error_limit' or 'session_stopped'; null while open.
  closed_by     TEXT,
  valid_cycles  INTEGER NOT NULL DEFAULT 0,
  errors        INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (session_id, segment_index)
) WITHOUT ROWID;
"#;

const MIGRATE_1_TO_2: &str = r#"
ALTER TABLE sessions ADD COLUMN config_section BLOB;
"#;

const MIGRATE_3_TO_4: &str = r#"
-- Error engine output, added in schema 4. The score, confidence and class are
-- columns because they are what a reader filters and plots on; the subscores
-- and per-feature deviations are JSON because they are only ever read whole.
-- All zero means the cycle was not scored — no reference was loaded — which is
-- not the same as agreeing with the reference.
ALTER TABLE cycles ADD COLUMN error_score REAL NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN confidence REAL NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN active_classes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN primary_class INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cycles ADD COLUMN confidence_subscores TEXT NOT NULL DEFAULT '[]';
ALTER TABLE cycles ADD COLUMN deviations TEXT NOT NULL DEFAULT '[]';

-- Versioned, patient-specific reference profiles (doc 12 §2-§3). A profile is
-- immutable once locked: a trigger refuses the update rather than trusting
-- every caller to remember, because a reference edited after use would silently
-- invalidate every evaluation made against it.
CREATE TABLE reference_profiles (
  reference_id TEXT PRIMARY KEY,
  patient_id   TEXT NOT NULL REFERENCES patients(patient_id),
  version      INTEGER NOT NULL,
  created_at   TEXT NOT NULL,
  session_id   TEXT REFERENCES sessions(session_id),
  cycles       INTEGER NOT NULL,
  locked       INTEGER NOT NULL DEFAULT 0,
  -- The 64-byte profile exactly as the device sent it (docs/protocol.md §5.13),
  -- so what is sent back for an evaluation is what was captured.
  payload      BLOB NOT NULL,
  UNIQUE (patient_id, version)
);

CREATE TRIGGER reference_profiles_locked
BEFORE UPDATE ON reference_profiles
WHEN old.locked = 1 AND (new.payload IS NOT old.payload OR new.cycles IS NOT old.cycles)
BEGIN
  SELECT RAISE(ABORT, 'a locked reference profile cannot be modified');
END;
"#;

const MIGRATE_2_TO_3: &str = r#"
-- One row per completed gait cycle, as the device measured it (doc 05 §11).
-- Derived values, not raw data: a corrected detector produces different rows for
-- the same recording, which is why the frames are kept separately.
CREATE TABLE cycles (
  session_id       TEXT NOT NULL REFERENCES sessions(session_id),
  start_frame      INTEGER NOT NULL,
  end_frame        INTEGER NOT NULL,
  start_us         INTEGER NOT NULL,
  cycle_time_s     REAL NOT NULL,
  stance_time_s    REAL NOT NULL,
  swing_time_s     REAL NOT NULL,
  stance_ratio     REAL NOT NULL,
  swing_ratio      REAL NOT NULL,
  cadence          REAL NOT NULL,
  peak_shank_dps   REAL NOT NULL,
  peak_dorsi_deg   REAL NOT NULL,
  contact_sag_deg  REAL NOT NULL,
  peak_inv_deg     REAL NOT NULL,
  distance_m       REAL NOT NULL,
  speed_mps        REAL NOT NULL,
  zupt_quality     REAL NOT NULL,
  valid            INTEGER NOT NULL,
  PRIMARY KEY (session_id, start_frame)
) WITHOUT ROWID;

-- Gait events, keyed by the frame they are attributed to. An event can refer to
-- a frame already stored: initial contact is timestamped at the strongest impact
-- inside its window, not at the moment the detector decided (doc 05 §3).
CREATE TABLE events (
  session_id   TEXT NOT NULL REFERENCES sessions(session_id),
  frame_index  INTEGER NOT NULL,
  timestamp_us INTEGER NOT NULL,
  kind         TEXT NOT NULL,
  PRIMARY KEY (session_id, frame_index, kind)
) WITHOUT ROWID;
"#;

pub const INSERT_FRAME: &str = r#"
INSERT OR IGNORE INTO raw_frames
  (session_id, frame_index, timestamp_us,
   fax, fay, faz, fgx, fgy, fgz,
   sax, say, saz, sgx, sgy, sgz,
   fqw, fqx, fqy, fqz, sqw, sqx, sqy, sqz, status)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
        ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
"#;

/// Binds and executes one frame. Parameter order matches `INSERT_FRAME`.
pub fn insert_frame(
    statement: &mut rusqlite::CachedStatement<'_>,
    session_id: &str,
    frame: &RawFrame,
) -> Result<usize> {
    statement.execute(rusqlite::params![
        session_id,
        frame.frame_index,
        frame.timestamp_us as i64,
        frame.foot[0], frame.foot[1], frame.foot[2],
        frame.foot[3], frame.foot[4], frame.foot[5],
        frame.shank[0], frame.shank[1], frame.shank[2],
        frame.shank[3], frame.shank[4], frame.shank[5],
        frame.q_foot[0], frame.q_foot[1], frame.q_foot[2], frame.q_foot[3],
        frame.q_shank[0], frame.q_shank[1], frame.q_shank[2], frame.q_shank[3],
        frame.status,
    ])
}

pub const INSERT_CYCLE: &str = r#"
INSERT OR REPLACE INTO cycles
  (session_id, start_frame, end_frame, start_us,
   cycle_time_s, stance_time_s, swing_time_s, stance_ratio, swing_ratio, cadence,
   peak_shank_dps, peak_dorsi_deg, contact_sag_deg, peak_inv_deg,
   distance_m, speed_mps, zupt_quality, valid,
   error_score, confidence, active_classes, primary_class,
   confidence_subscores, deviations, segment_index)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
        ?19, ?20, ?21, ?22, ?23, ?24, ?25)
"#;

pub const INSERT_EVENT: &str = r#"
INSERT OR IGNORE INTO events (session_id, frame_index, timestamp_us, kind)
VALUES (?1, ?2, ?3, ?4)
"#;
