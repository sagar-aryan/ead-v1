//! Database schema and the raw-frame row mapping.
//!
//! `user_version` records the schema version, so an older store is detected
//! rather than silently misread (doc 18: version every persisted format).

use rusqlite::{Connection, Result};

use crate::protocol::RawFrame;

pub const SCHEMA_VERSION: i32 = 1;

pub fn migrate(connection: &mut Connection) -> Result<()> {
    // WAL keeps readers (UI queries) from blocking the writer thread.
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "busy_timeout", 5000)?;

    let version: i32 =
        connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap_or(0);
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    if version > SCHEMA_VERSION {
        // Refuse rather than risk misreading a newer layout.
        return Err(rusqlite::Error::InvalidQuery);
    }
    if version == 0 {
        connection.execute_batch(CREATE_SCHEMA)?;
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
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
  boot_id        INTEGER
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
