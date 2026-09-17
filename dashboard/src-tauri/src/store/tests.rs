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
fn schema_upgrades_from_version_1_without_losing_data() {
    let dir = tempdir::TempDir::new();
    let path = dir.path().join("ead.sqlite3");
    {
        // A version-1 store: sessions without the configuration column.
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE patients (patient_id TEXT PRIMARY KEY, name TEXT NOT NULL,
                                        created_at TEXT NOT NULL);
                 CREATE TABLE sessions (session_id TEXT PRIMARY KEY, patient_id TEXT NOT NULL,
                                        kind TEXT NOT NULL, started_at TEXT NOT NULL,
                                        stopped_at TEXT, firmware TEXT, config_sha256 TEXT,
                                        device_mac TEXT, boot_id INTEGER);
                 INSERT INTO patients VALUES ('P-OLD', 'Earlier study', '2026-01-01T00:00:00.000Z');
                 INSERT INTO sessions (session_id, patient_id, kind, started_at)
                   VALUES ('20260101-000000-abcd', 'P-OLD', 'recording', '2026-01-01T00:00:00.000Z');
                 PRAGMA user_version = 1;",
            )
            .unwrap();
    }

    let store = Store::open(&path).expect("migrate");
    let connection = store.reader().unwrap();
    let version: i32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, schema::SCHEMA_VERSION);
    // The earlier rows survive, and the new column exists and reads as absent.
    assert_eq!(store.patients().unwrap()[0].patient_id, "P-OLD");
    assert_eq!(store.session_config("20260101-000000-abcd").unwrap(), None);
}

#[test]
fn session_records_the_device_configuration() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let identity = DeviceIdentity {
        firmware: Some("0.1.0+test".into()),
        config_sha256: Some("abc123".into()),
        mac: Some("44:B1:76:AF:FB:7C".into()),
        boot_id: Some(7),
        config_section: Some(vec![1, 2, 3, 4]),
    };
    let session = store.start_session("P-001", SessionKind::Recording, &identity).unwrap();
    assert_eq!(store.session_config(&session.session_id).unwrap(), Some(vec![1, 2, 3, 4]));
    assert_eq!(store.session(&session.session_id).unwrap().firmware.as_deref(), Some("0.1.0+test"));
}

#[test]
fn raw_window_returns_every_frame_when_the_range_is_small() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();
    let frames: Vec<RawFrame> = (0..300).map(frame).collect();
    store.record_frames(&frames);
    store.flush();

    let window = store
        .raw_window(&session.session_id, &[SignalGroup::FootAccel], 0, 299, 1000)
        .unwrap();
    assert_eq!(window.bucket, 1);
    assert_eq!(window.points, 300);
    // Time is relative to the session's first frame; frames are 10 ms apart.
    assert_eq!(window.time_s[0], 0.0);
    assert!((window.time_s[299] - 2.99).abs() < 1e-9);
    assert_eq!(window.signals[0].axes.len(), 3);
    // No stored configuration: raw counts in the sensor's own axes.
    assert!(!window.anatomical);
    assert_eq!(window.signals[0].unit, "counts");
    assert_eq!(window.signals[0].axes[0].min[0], 1.0);
    assert_eq!(window.signals[0].axes[2].max[0], 8192.0);
}

#[test]
fn decimation_preserves_a_single_sample_transient() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();

    // 20,000 quiet frames with one spike, the shape of a heel strike.
    let mut frames: Vec<RawFrame> = (0..20_000).map(frame).collect();
    frames[12_345].foot[0] = 30_000;
    frames[12_345].status = 0x0008; // foot accel saturated
    store.record_frames(&frames);
    store.flush();

    let window = store
        .raw_window(&session.session_id, &[SignalGroup::FootAccel], 0, 19_999, 500)
        .unwrap();
    assert!(window.bucket > 1, "a 20,000-frame range must be decimated");
    assert!(window.points <= 520, "returned {} points", window.points);
    // Sampling every Nth frame would lose the spike; min/max cannot.
    assert_eq!(window.signals[0].axes[0].max.iter().copied().fold(f32::MIN, f32::max), 30_000.0);
    assert!(window.status.iter().any(|s| s & 0x0008 != 0), "flagged frame must survive");

    // Zooming in reaches the exact sample.
    let zoom = store
        .raw_window(&session.session_id, &[SignalGroup::FootAccel], 12_300, 12_400, 1000)
        .unwrap();
    assert_eq!(zoom.bucket, 1);
    assert_eq!(zoom.signals[0].axes[0].max[45], 30_000.0);
    assert_eq!(zoom.frame_index[45], 12_345);
}

#[test]
fn raw_window_rejects_a_backwards_range() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();
    assert!(matches!(
        store.raw_window(&session.session_id, &[SignalGroup::FootAccel], 100, 10, 100),
        Err(StoreError::Rejected(_))
    ));
}

/// Decides whether precomputed summary tables are needed: if a full-session
/// query over an hour of data is fast enough, they are not.
#[test]
#[ignore = "performance measurement; run with --ignored --nocapture"]
fn raw_window_query_time_on_an_hour_of_data() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();

    // One hour at 100 Hz, with values that vary like real signals rather than
    // compressing to a constant.
    let total = 360_000u32;
    let mut lcg: u32 = 99;
    for chunk in 0..(total / 1000) {
        let frames: Vec<RawFrame> = (0..1000)
            .map(|i| {
                let index = chunk * 1000 + i;
                let mut f = frame(index);
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (lcg >> 20) as i16;
                f.foot[0] = 8192 + noise % 400;
                f.foot[2] = noise % 1200;
                f.shank[1] = -8000 + noise % 600;
                f
            })
            .collect();
        store.record_frames(&frames);
    }
    let write_start = std::time::Instant::now();
    store.flush();
    println!("wrote {total} frames, flush took {} ms", write_start.elapsed().as_millis());

    for (label, first, last) in [
        ("whole session", 0i64, total as i64 - 1),
        ("10 minutes", 0, 60_000),
        ("1 minute", 100_000, 106_000),
        ("10 seconds", 200_000, 201_000),
    ] {
        let window =
            store.raw_window(&session.session_id, &[SignalGroup::FootAccel], first, last, 1400).unwrap();
        println!(
            "{label:14} {:>7} frames -> {:>5} points, bucket {:>4}, {} ms",
            last - first + 1,
            window.points,
            window.bucket,
            window.query_ms
        );
        assert!(window.query_ms < 1000, "{label} took {} ms", window.query_ms);
    }

    // What the raw view actually costs now that it draws every signal at once:
    // four detail queries plus the whole-session overview strip.
    let groups = [
        SignalGroup::FootAccel,
        SignalGroup::FootGyro,
        SignalGroup::ShankAccel,
        SignalGroup::ShankGyro,
    ];
    for (label, first, last) in
        [("whole session", 0i64, total as i64 - 1), ("1 minute", 100_000, 106_000)]
    {
        let started = std::time::Instant::now();
        let window = store.raw_window(&session.session_id, &groups, first, last, 1400).unwrap();
        assert_eq!(window.signals.len(), 4);
        let detail = started.elapsed().as_millis();
        let started = std::time::Instant::now();
        store.raw_window(&session.session_id, &groups[..1], 0, total as i64 - 1, 700).unwrap();
        println!(
            "{label:14} four signals {detail} ms + overview {} ms",
            started.elapsed().as_millis()
        );
    }

    let size = std::fs::metadata(_dir.path().join("ead.sqlite3")).unwrap().len();
    println!("database {} MB for one hour", size / 1_048_576);
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

#[test]
fn raw_window_reads_several_signals_on_one_time_base() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default()).unwrap();
    let frames: Vec<RawFrame> = (0..300)
        .map(|i| {
            let mut f = frame(i);
            f.foot[0] = 100;
            f.shank[5] = -200;
            f
        })
        .collect();
    store.record_frames(&frames);
    store.flush();

    let groups = [SignalGroup::ShankGyro, SignalGroup::FootAccel, SignalGroup::ShankGyro];
    let window = store.raw_window(&session.session_id, &groups, 0, 299, 1000).unwrap();
    // Duplicates collapse; the caller's order is kept, since it is the stacking order.
    let order: Vec<_> = window.signals.iter().map(|s| s.group).collect();
    assert_eq!(order, [SignalGroup::ShankGyro, SignalGroup::FootAccel]);
    // Each signal reads its own columns, not its neighbour's.
    assert_eq!(window.signals[0].axes[2].min[0], -200.0);
    assert_eq!(window.signals[1].axes[0].max[0], 100.0);
    assert!(window.signals.iter().all(|s| s.axes[0].min.len() == window.points));

    assert!(matches!(
        store.raw_window(&session.session_id, &[], 0, 299, 1000),
        Err(StoreError::Rejected(_))
    ));
}
