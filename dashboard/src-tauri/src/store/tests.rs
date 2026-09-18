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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();

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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();

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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();

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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();
    assert!(matches!(
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None),
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
            .start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None)
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
        // A version-1 store: sessions without the configuration column. Only
        // the columns the upgrade and startup touch are recreated; every real
        // v1 store also had raw_frames, which startup reads to close sessions
        // an exit left open.
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE raw_frames (session_id TEXT NOT NULL, frame_index INTEGER NOT NULL,
                                          timestamp_us INTEGER NOT NULL,
                                          PRIMARY KEY (session_id, frame_index)) WITHOUT ROWID;
                 CREATE TABLE patients (patient_id TEXT PRIMARY KEY, name TEXT NOT NULL,
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
    let identity = DeviceIdentity { calibration: None,
        firmware: Some("0.1.0+test".into()),
        config_sha256: Some("abc123".into()),
        mac: Some("44:B1:76:AF:FB:7C".into()),
        boot_id: Some(7),
        config_section: Some(vec![1, 2, 3, 4]),
    };
    let session = store.start_session("P-001", SessionKind::Recording, &identity, None, None).unwrap();
    assert_eq!(store.session_config(&session.session_id).unwrap(), Some(vec![1, 2, 3, 4]));
    assert_eq!(store.session(&session.session_id).unwrap().firmware.as_deref(), Some("0.1.0+test"));
}

#[test]
fn raw_window_returns_every_frame_when_the_range_is_small() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();
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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();

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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();
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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();

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
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();
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

#[test]
fn gait_cycles_and_events_round_trip() {
    use crate::protocol::{GaitCycle, GaitEvent, GaitEventType};
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();

    let cycle = GaitCycle {
        start_frame: 1200,
        end_frame: 1360,
        start_us: 12_000_000,
        cycle_time_s: 1.66,
        stance_time_s: 0.83,
        swing_time_s: 0.83,
        stance_ratio: 0.5,
        swing_ratio: 0.5,
        cadence_steps_per_min: 72.3,
        peak_shank_rate_dps: 412.5,
        peak_dorsiflexion_deg: 16.1,
        contact_sagittal_deg: -4.2,
        peak_inversion_deg: 3.1,
        distance_m: 1.13,
        speed_mps: 0.68,
        zupt_quality: 0.22,
        valid: true,
        error_score: 0.42,
        confidence: 0.86,
        confidence_subscores: [1.0, 1.0, 1.0, 0.75, 0.58],
        deviations: [0.91, 0.12, 0.05, 0.10, 0.08, 0.22, 0.30],
        active_classes: 0b10,
        primary_class: 1,
    };
    let events = [
        GaitEvent {
            event_type: GaitEventType::InitialContact,
            frame_index: 1200,
            timestamp_us: 12_000_000,
        },
        GaitEvent { event_type: GaitEventType::ToeOff, frame_index: 1283, timestamp_us: 12_830_000 },
    ];
    store.record_gait(&[cycle], &events);
    // A backfilled batch repeats what was already stored.
    store.record_gait(&[cycle], &events);
    store.flush();

    let stored = store.cycles(&session.session_id).unwrap();
    assert_eq!(stored.len(), 1, "a repeated cycle must not duplicate");
    assert_eq!(stored[0].start_frame, 1200);
    assert_eq!(stored[0].cadence_steps_per_min, 72.3);
    assert_eq!(stored[0].distance_m, 1.13);
    assert!(stored[0].valid);

    let stored_events = store.events(&session.session_id).unwrap();
    assert_eq!(stored_events.len(), 2);
    assert_eq!(stored_events[0].kind, "initial_contact");
    assert_eq!(stored_events[1].kind, "toe_off");
}

#[test]
fn gait_is_only_stored_while_recording() {
    use crate::protocol::{GaitCycle, GaitEvent, GaitEventType};
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session =
        store.start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None).unwrap();
    store.stop_session().unwrap();

    store.record_gait(
        &[GaitCycle {
            start_frame: 1,
            end_frame: 2,
            start_us: 0,
            cycle_time_s: 1.0,
            stance_time_s: 0.6,
            swing_time_s: 0.4,
            stance_ratio: 0.6,
            swing_ratio: 0.4,
            cadence_steps_per_min: 120.0,
            peak_shank_rate_dps: 0.0,
            peak_dorsiflexion_deg: 0.0,
            contact_sagittal_deg: 0.0,
            peak_inversion_deg: 0.0,
            distance_m: 0.0,
            speed_mps: 0.0,
            zupt_quality: 0.0,
            valid: true,
            ..Default::default()
        }],
        &[GaitEvent {
            event_type: GaitEventType::FootFlat,
            frame_index: 1,
            timestamp_us: 0,
        }],
    );
    store.flush();
    assert!(store.cycles(&session.session_id).unwrap().is_empty());
    assert!(store.events(&session.session_id).unwrap().is_empty());
}

#[test]
fn schema_upgrades_from_version_2_keeping_frames() {
    let dir = tempdir::TempDir::new();
    let path = dir.path().join("ead.sqlite3");
    {
        let store = Store::open(&path).expect("create");
        store.create_patient("P-OLD", "Earlier study").unwrap();
        let session = store
            .start_session("P-OLD", SessionKind::Recording, &DeviceIdentity::default(), None, None)
            .unwrap();
        store.record_frames(&[frame(0), frame(1)]);
        store.flush();
        let connection = store.reader().unwrap();
        // Pretend this store predates the gait tables: undo everything schemas
        // 3, 4, 5 and 6 added, so it really looks like a v2 store.
        connection
            .execute_batch(
                "DROP TABLE status_changes;
                 ALTER TABLE sessions DROP COLUMN calibration;
                 DROP TABLE segments;
                 ALTER TABLE sessions DROP COLUMN reference_id;
                 ALTER TABLE sessions DROP COLUMN max_cycles_per_segment;
                 ALTER TABLE sessions DROP COLUMN max_errors_per_segment;
                 DROP TRIGGER reference_profiles_locked;
                 DROP TABLE reference_profiles;
                 DROP TABLE events;
                 DROP TABLE cycles;
                 PRAGMA user_version = 2;",
            )
            .unwrap();
        drop(connection);
        assert_eq!(store.frame_count(&session.session_id).unwrap(), 2);
    }
    let store = Store::open(&path).expect("migrate");
    let connection = store.reader().unwrap();
    let version: i32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, schema::SCHEMA_VERSION);
    let sessions = store.sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(store.frame_count(&sessions[0].session_id).unwrap(), 2, "frames survive");
    assert!(store.cycles(&sessions[0].session_id).unwrap().is_empty());
}

fn sample_profile(cycles: u16) -> crate::protocol::ReferenceProfile {
    use crate::protocol::{ReferenceFeature, ReferenceProfile};
    let mut profile = ReferenceProfile { cycles, version: 0, ..Default::default() };
    // The medians measured on the 6 m walk, with plausible spreads.
    let values = [
        (16.0, 1.6),
        (-4.0, 1.2),
        (3.0, 0.9),
        (1.70, 0.05),
        (0.52, 0.02),
        (1.10, 0.08),
        (400.0, 22.0),
    ];
    for (feature, (median, spread)) in profile.features.iter_mut().zip(values) {
        *feature = ReferenceFeature { median, spread };
    }
    profile
}

#[test]
fn reference_versions_count_up_per_patient() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    store.create_patient("P-002", "Another").unwrap();

    let first = store.add_reference("P-001", None, &sample_profile(34)).unwrap();
    let second = store.add_reference("P-001", None, &sample_profile(41)).unwrap();
    let other = store.add_reference("P-002", None, &sample_profile(30)).unwrap();
    assert_eq!((first.version, second.version, other.version), (1, 2, 1));
    assert_eq!(second.profile.version, 2, "the stored profile carries its version");
    assert_eq!(store.references("P-001").unwrap().len(), 2);
    // Newest first, so the view offers the current one.
    assert_eq!(store.references("P-001").unwrap()[0].version, 2);
}

#[test]
fn a_reference_below_the_minimum_is_refused() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let thin = store.add_reference("P-001", None, &sample_profile(29));
    assert!(matches!(thin, Err(StoreError::Rejected(_))), "29 cycles is not a reference");
    assert!(store.references("P-001").unwrap().is_empty());
}

#[test]
fn a_locked_reference_cannot_be_changed() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let reference = store.add_reference("P-001", None, &sample_profile(34)).unwrap();
    assert!(!reference.locked);
    store.lock_reference(&reference.reference_id).unwrap();
    assert!(store.reference(&reference.reference_id).unwrap().locked);

    // The database refuses the edit itself, not just the code paths that know
    // to check: an evaluation's reference must mean the same thing forever.
    let connection = store.reader().unwrap();
    let changed = connection.execute(
        "UPDATE reference_profiles SET payload = ?1 WHERE reference_id = ?2",
        rusqlite::params![vec![0u8; 64], reference.reference_id],
    );
    assert!(changed.is_err(), "a locked profile must be immutable");
    assert_eq!(
        store.reference(&reference.reference_id).unwrap().profile,
        reference.profile,
        "and must still read back unchanged"
    );
}

#[test]
fn a_reference_round_trips_through_its_stored_bytes() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let saved = store.add_reference("P-001", None, &sample_profile(34)).unwrap();
    let read_back = store.reference(&saved.reference_id).unwrap();
    // What goes back to the device for an evaluation is what was captured.
    assert_eq!(read_back.profile, saved.profile);
    assert_eq!(read_back.profile.features[0].median, 16.0);
    assert_eq!(read_back.profile.features[6].spread, 22.0);
}

/// A cycle the segment counter will see: `valid` and, when `class` is nonzero
/// and the confidence is high enough, an error (DEC-014).
fn scored_cycle(start_frame: u32, valid: bool, class: u8, confidence: f32) -> GaitCycle {
    GaitCycle {
        start_frame,
        end_frame: start_frame + 100,
        start_us: 10_000 * start_frame as u64,
        cycle_time_s: 1.0,
        valid,
        primary_class: class,
        confidence,
        ..Default::default()
    }
}

#[test]
fn segments_close_on_whichever_limit_comes_first() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let limits = SegmentLimits { max_cycles: 3, max_errors: 2 };
    let session = store
        .start_session(
            "P-001",
            SessionKind::Evaluation,
            &DeviceIdentity::default(),
            None,
            Some(limits),
        )
        .unwrap();

    // Three valid, clean cycles reach the cycle limit.
    store.record_gait(
        &[scored_cycle(0, true, 0, 0.9), scored_cycle(1, true, 0, 0.9), scored_cycle(2, true, 0, 0.9)],
        &[],
    );
    // Then two errors reach the error limit inside the next segment, with an
    // invalid cycle between them that counts toward neither.
    store.record_gait(
        &[scored_cycle(3, true, 1, 0.9), scored_cycle(4, false, 0, 0.9), scored_cycle(5, true, 3, 0.9)],
        &[],
    );
    // A classified cycle nobody may be shown does not count as an error.
    store.record_gait(&[scored_cycle(6, true, 1, 0.2)], &[]);
    store.flush();

    let segments = store.segments(&session.session_id).unwrap();
    assert_eq!(segments.len(), 3, "two closed segments and the one now open");
    assert_eq!(segments[0].closed_by.as_deref(), Some("cycle_limit"));
    assert_eq!(segments[0].valid_cycles, 3);
    assert_eq!(segments[1].closed_by.as_deref(), Some("error_limit"));
    assert_eq!(segments[1].errors, 2);
    assert_eq!(segments[1].valid_cycles, 2, "the invalid cycle counts toward neither");
    assert_eq!(segments[2].closed_by, None);
    assert_eq!(segments[2].errors, 0, "0.2 confidence is below the display gate");

    let cycles = store.cycles(&session.session_id).unwrap();
    let indices: Vec<i64> = cycles.iter().map(|c| c.segment_index).collect();
    assert_eq!(indices, vec![0, 0, 0, 1, 1, 1, 2], "the cycle that trips a limit closes it");

    store.stop_session().unwrap();
    let closed = store.segments(&session.session_id).unwrap();
    assert_eq!(closed[2].closed_by.as_deref(), Some("session_stopped"));
}

#[test]
fn a_recording_has_no_segments() {
    let (store, _dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session = store
        .start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None)
        .unwrap();
    store.record_gait(&[scored_cycle(0, true, 1, 0.9)], &[]);
    store.flush();
    assert!(store.segments(&session.session_id).unwrap().is_empty());
    assert_eq!(store.cycles(&session.session_id).unwrap()[0].segment_index, 0);
}

/// A session with everything an export has to carry: frames, cycles, events,
/// a reference, segments and a fault.
fn exportable_session() -> (Arc<Store>, tempdir::TempDir, String) {
    let (store, dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let reference = store.add_reference("P-001", None, &sample_profile(34)).unwrap();
    let session = store
        .start_session(
            "P-001",
            SessionKind::Evaluation,
            &DeviceIdentity {
                calibration: Some(r#"{"kind":1,"reject":0,"samples":500}"#.into()),
                firmware: Some("0.1.0+test".into()),
                mac: Some("aa:bb:cc:dd:ee:ff".into()),
                ..Default::default()
            },
            Some(&reference.reference_id),
            Some(SegmentLimits { max_cycles: 2, max_errors: 5 }),
        )
        .unwrap();
    store.record_frames(&[frame(0), frame(1), frame(2)]);
    let mut cycles = Vec::new();
    for i in 0..3u32 {
        let mut c = scored_cycle(i, true, if i == 1 { 1 } else { 0 }, 0.9);
        c.cycle_time_s = 1.0 + i as f32 * 0.05;
        c.stance_ratio = 0.6;
        c.peak_dorsiflexion_deg = 15.0 + i as f32;
        c.distance_m = 1.1;
        c.zupt_quality = 0.5;
        cycles.push(c);
    }
    store.record_gait(
        &cycles,
        &[GaitEvent { event_type: crate::protocol::GaitEventType::InitialContact, frame_index: 0, timestamp_us: 0 }],
    );
    store.record_status(&crate::protocol::Status {
        device_state: 5,
        frame_index: 1,
        faults: 0,
        ..Default::default()
    });
    store.record_status(&crate::protocol::Status {
        device_state: 7,
        frame_index: 2,
        faults: 1,
        ..Default::default()
    });
    store.flush();
    let id = session.session_id.clone();
    (store, dir, id)
}

#[test]
fn the_export_package_matches_the_database() {
    let (store, dir, session_id) = exportable_session();
    let target = dir.path().join("package");
    let summary = crate::export::export_session(&store, &session_id, &target).unwrap();

    // Doc 10 §2: two rows per frame, one per IMU.
    assert_eq!(summary.raw_rows, store.frame_count(&session_id).unwrap() as usize * 2);
    let raw = std::fs::read_to_string(target.join("raw.csv")).unwrap();
    assert_eq!(raw.lines().count(), summary.raw_rows + 1, "one header row");
    assert!(raw.lines().next().unwrap().starts_with("timestamp_us,frame_index,sensor,"));

    let gait = std::fs::read_to_string(target.join("gait.csv")).unwrap();
    assert_eq!(gait.lines().count(), store.cycles(&session_id).unwrap().len() + 1);
    // The first valid cycle has no previous one, so its proxy cell is empty
    // rather than zero; the second has both.
    let rows: Vec<&str> = gait.lines().skip(1).collect();
    assert_eq!(rows[0].split(',').nth(11).unwrap(), "", "no previous cycle to compare with");
    assert_ne!(rows[1].split(',').nth(11).unwrap(), "");
    assert_eq!(rows[1].split(',').nth(14).unwrap(), "insufficient_dorsiflexion");

    // Doc 10 §4: the derived event types are present alongside the device's.
    let events = std::fs::read_to_string(target.join("events.csv")).unwrap();
    for kind in ["INITIAL_CONTACT", "CYCLE_START", "CYCLE_END", "ERROR_ACTIVE", "FAULT"] {
        assert!(events.contains(kind), "events.csv is missing {kind}");
    }
    assert!(!events.contains("SERVICE_TEST"), "no haptics, so no service test happened");

    // Doc 10 §5: the header alone, so a reader sees no haptic event occurred
    // rather than finding no file at all.
    let haptics = std::fs::read_to_string(target.join("haptics.csv")).unwrap();
    assert_eq!(haptics.lines().count(), 1);

    let metadata: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(target.join("metadata.json")).unwrap())
            .unwrap();
    for key in ["patient", "session", "device", "calibration", "reference_profile",
                "segmentation", "haptics", "storage_recovery", "coordinate_convention"] {
        assert!(!metadata[key].is_null(), "metadata.json is missing {key}");
    }
    assert_eq!(metadata["haptics"]["fitted"], serde_json::json!(false));
    assert_eq!(metadata["device"]["firmware_version"], serde_json::json!("0.1.0+test"));
    assert_eq!(metadata["segmentation"]["max_valid_cycles_per_segment"], serde_json::json!(2));

    assert!(target.join("session.mat").metadata().unwrap().len() > 256);

    // The report's contents are checked by pdftotext in TEST-037; here, only
    // that it is a PDF of three pages, which is what the composer promises.
    let report = std::fs::read(target.join("report.pdf")).unwrap();
    assert_eq!(&report[..5], b"%PDF-");
    let text = String::from_utf8_lossy(&report);
    assert!(text.contains("/Count 3"), "the report is three pages");
}

#[test]
fn an_unscored_session_exports_empty_cells_not_zeroes() {
    let (store, dir) = temp_store();
    store.create_patient("P-001", "Reference Walker").unwrap();
    let session = store
        .start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None)
        .unwrap();
    store.record_gait(&[scored_cycle(0, true, 0, 0.0), scored_cycle(1, true, 0, 0.0)], &[]);
    store.flush();
    let target = dir.path().join("package");
    crate::export::export_session(&store, &session.session_id, &target).unwrap();

    let gait = std::fs::read_to_string(target.join("gait.csv")).unwrap();
    for row in gait.lines().skip(1) {
        let cells: Vec<&str> = row.split(',').collect();
        assert_eq!(cells[11], "", "symmetry proxy needs a reference's spreads");
        assert_eq!(cells[12], "", "an unscored cycle has no error score");
        assert_eq!(cells[13], "", "an unscored cycle has no confidence");
        assert_eq!(cells[14], "", "an unscored cycle has no class");
    }
    let metadata: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(target.join("metadata.json")).unwrap())
            .unwrap();
    assert!(metadata["reference_profile"].as_str().unwrap().starts_with("none"));
}

/// Writes the same package to `target/export-sample` so `tools/check_mat.py`
/// can read it back with scipy — an independent implementation of the Level-5
/// format, which is the only thing that can tell us the hand-rolled writer
/// produces a file MATLAB would actually open.
///
/// Ignored by default because it writes outside a temporary directory. Run:
///   cargo test --manifest-path dashboard/src-tauri/Cargo.toml -- --ignored export_sample
///   python3 tools/check_mat.py dashboard/src-tauri/target/export-sample
#[test]
#[ignore]
fn export_sample_for_the_mat_checker() {
    let (store, _dir, session_id) = exportable_session();
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/export-sample");
    let _ = std::fs::remove_dir_all(&target);
    let summary = crate::export::export_session(&store, &session_id, &target).unwrap();
    println!("{} files in {}", summary.files.len(), summary.directory);
}

#[test]
fn a_session_left_open_by_a_crash_is_closed_at_the_next_start() {
    let dir = tempdir::TempDir::new();
    let path = dir.path().join("ead.sqlite3");
    let session_id = {
        let store = Store::open(&path).unwrap();
        store.create_patient("P-001", "Reference Walker").unwrap();
        let session = store
            .start_session("P-001", SessionKind::Recording, &DeviceIdentity::default(), None, None)
            .unwrap();
        // Frames 0 and 100 are 1.000 s apart in device time (10 ms each).
        store.record_frames(&[frame(0), frame(100)]);
        store.flush();
        session.session_id
        // Dropped without stop_session: the app exited mid-recording.
    };
    let store = Store::open(&path).unwrap();
    let session = store.session(&session_id).unwrap();
    let stopped = session.stopped_at.expect("closed at startup");
    let started = &session.started_at;
    // Same second-of-minute arithmetic as the stored strings: one second later.
    let seconds = |t: &str| t[17..23].parse::<f64>().unwrap();
    let elapsed = (seconds(&stopped) - seconds(started)).rem_euclid(60.0);
    assert!((elapsed - 1.0).abs() < 0.01, "{started} -> {stopped}");
}
