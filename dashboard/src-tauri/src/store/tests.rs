use super::*;
use crate::protocol::RawFrame;

fn temp_store() -> (Arc<Store>, tempdir::TempDir) {
    let dir = tempdir::TempDir::new();
    let store = Store::open(dir.path().join("ead.sqlite3")).expect("open store");
    (store, dir)
}

fn frame(index: u32) -> RawFrame {
    RawFrame {
        timestamp_us: 10_000 * index as u64,
        frame_index: index,
        foot: [1, -2, 8192, 4, -5, 6],
        shank: [-7, 8, -9, 10, -11, 32767],
        q_foot: [32767, 0, 0, 0],
        q_shank: [32767, 0, 0, -32768],
        status: 0,
    }
}

#[test]
fn schema_version_is_recorded() {
    let (store, _dir) = temp_store();
    let connection = store.reader().unwrap();
    let version: i32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, schema::SCHEMA_VERSION);
}

#[test]
fn patients_are_unique_and_validated() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let duplicate = store.create_patient("P-001", "Someone Else");
    assert!(matches!(duplicate, Err(StoreError::Rejected(_))));
    assert!(matches!(store.create_patient("  ", "No ID"), Err(StoreError::Rejected(_))));
    assert_eq!(store.patients().unwrap().len(), 1);
}

#[test]
fn frames_round_trip_with_exact_values() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();

    let frames: Vec<RawFrame> = (0..250).map(frame).collect();
    store.record_frames(&frames);
    store.flush();

    assert_eq!(store.frame_count(&session.session_id).unwrap(), 250);
    let connection = store.reader().unwrap();
    let (fax, saz, sqz, ts): (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT fax, saz, sqz, timestamp_us FROM raw_frames
             WHERE session_id = ?1 AND frame_index = 7",
            [&session.session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    // Signed counts survive the round trip, including the i16 extremes.
    assert_eq!((fax, saz, sqz, ts), (1, -9, -32768, 70_000));
}

#[test]
fn repeated_frames_do_not_duplicate() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();

    let frames: Vec<RawFrame> = (0..10).map(frame).collect();
    store.record_frames(&frames);
    // A backfill can deliver frames the live stream already stored.
    store.record_frames(&frames);
    store.flush();
    assert_eq!(store.frame_count(&session.session_id).unwrap(), 10);
}

#[test]
fn session_reports_gaps_from_missing_frame_indices() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let started =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();

    // 0..20 with 5..9 never delivered.
    let frames: Vec<RawFrame> =
        (0..20).filter(|i| !(5..10).contains(i)).map(frame).collect();
    store.record_frames(&frames);
    store.flush();

    let session = store.session(&started.session_id).unwrap();
    assert_eq!(session.frames_stored, 15);
    assert_eq!(session.first_frame_index, Some(0));
    assert_eq!(session.last_frame_index, Some(19));
    assert_eq!(session.frames_missing, 5);
}

#[test]
fn only_one_session_records_at_a_time() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let first =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();
    assert!(matches!(
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()),
        Err(StoreError::Rejected(_))
    ));

    // Frames are only stored while a session is recording.
    store.record_frames(&[frame(0)]);
    store.flush();
    let stopped = store.stop_session().unwrap().unwrap();
    assert_eq!(stopped.session_id, first.session_id);
    assert!(stopped.stopped_at.is_some());

    store.record_frames(&[frame(1)]);
    store.flush();
    assert_eq!(store.frame_count(&first.session_id).unwrap(), 1);
    assert!(store.stop_session().unwrap().is_none());
}

#[test]
fn session_ids_follow_the_documented_shape() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let mut ids = Vec::new();
    for _ in 0..8 {
        let session = store
            .start_session("P-001", SessionKind::Recording, &DeviceIdentity::default())
            .unwrap();
        ids.push(session.session_id.clone());
        store.stop_session().unwrap();
    }
    for id in &ids {
        // YYYYMMDD-HHMMSS-xxxx (doc 10 §1)
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "{id}");
        assert_eq!((parts[0].len(), parts[1].len(), parts[2].len()), (8, 6, 4), "{id}");
        assert!(parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit())), "{id}");
    }
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 8, "session IDs must be unique");
}

#[test]
fn civil_from_unix_matches_known_timestamps() {
    // Reference values from `date -u -d @<epoch>`, not from this code.
    assert_eq!(format_utc(0, 0), "1970-01-01T00:00:00.000Z");
    assert_eq!(format_utc(1_000_000_000, 0), "2001-09-09T01:46:40.000Z");
    assert_eq!(format_utc(1_709_164_800, 250), "2024-02-29T00:00:00.250Z"); // leap day
    assert_eq!(format_utc(1_788_912_000, 0), "2026-09-09T00:00:00.000Z");
    assert_eq!(format_utc(1_788_912_000 + 57_845, 7), "2026-09-09T16:04:05.007Z");
}

/// Minimal scoped temporary directory: the store is the only thing that needs
/// one, and it is not worth a dependency.
mod tempdir {
    use std::path::{Path, PathBuf};

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ead-store-test-{unique}"));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
