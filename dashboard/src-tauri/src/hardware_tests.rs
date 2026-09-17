//! Hardware-in-the-loop checks for the dashboard backend: protocol, link,
//! device manager and store against a real device over USB.
//!
//! Ignored by default because they need the device attached. Run with:
//!   cargo test -- --ignored --nocapture

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::device::{self, Device, LinkState, Sink};
use crate::link::LinkTarget;
use crate::protocol::{RawFrame, Status};
use crate::store::{DeviceIdentity, SessionKind, SignalGroup, Store};

struct StoreSink(Arc<Store>);

impl Sink for StoreSink {
    fn raw_frames(&self, frames: &[RawFrame]) {
        self.0.record_frames(frames);
    }
    fn status(&self, _status: &Status) {}
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        let unique =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("ead-hw-test-{unique}"));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Waits for `condition`, polling every 50 ms.
fn wait_for(timeout: Duration, label: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {label}");
}

#[test]
#[ignore = "requires the EAD device attached over USB"]
fn records_a_session_from_a_real_device() {
    let dir = TempDir::new();
    let store = Store::open(dir.0.join("ead.sqlite3")).expect("open store");
    let device = Arc::new(Device::new(Arc::new(StoreSink(store.clone()))));

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let link = runtime.spawn(device::run(
        device.clone(),
        LinkTarget::Usb { port: None },
        stop_rx,
    ));

    wait_for(Duration::from_secs(10), "device connection", || {
        device.snapshot().link_state == LinkState::Connected && device.is_live()
    });
    wait_for(Duration::from_secs(10), "device configuration", || device.config().is_some());

    let snapshot = device.snapshot();
    println!(
        "connected: {} firmware {:?} mac {:?} capabilities {:?}",
        snapshot.link_description.clone().unwrap_or_default(),
        snapshot.firmware,
        snapshot.mac,
        snapshot.capabilities
    );
    assert!(!snapshot.schema_mismatch, "device and host protocol schemas differ");
    assert!(!snapshot.haptics_fitted, "no ERM drivers are fitted (DEC-006)");
    assert_eq!(snapshot.faults, Vec::<&str>::new(), "device reports faults");

    // The configuration must be the as-built one (docs/hardware.md).
    let config = device.config().expect("configuration");
    assert_eq!(config.imu.foot_mount, [[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    assert_eq!(config.imu.shank_mount, [[0, 0, -1], [1, 0, 0], [0, -1, 0]]);
    assert_eq!(config.imu.sample_hz, 100);
    assert!(!config.haptics.fitted);

    // Opening the port mid-stream leaves a partial frame in the buffer, which is
    // discarded by design. What matters is that nothing is corrupted afterwards.
    let rejected_at_start = device.snapshot().rejected_frames;

    store.create_patient("HW-TEST", "Hardware test").expect("create patient");
    let identity = DeviceIdentity {
        firmware: snapshot.firmware.clone(),
        config_sha256: device
            .config_sha256()
            .map(|d| d.iter().map(|b| format!("{b:02x}")).collect()),
        mac: snapshot.mac.clone(),
        boot_id: snapshot.boot_id,
        config_section: device.config_section(),
    };
    let session = store
        .start_session("HW-TEST", SessionKind::Recording, &identity)
        .expect("start session");

    let seconds = 5;
    std::thread::sleep(Duration::from_secs(seconds));
    let stopped = store.stop_session().expect("stop session").expect("a session was recording");
    stop_tx.send(true).unwrap();
    runtime.block_on(async { let _ = link.await; });

    println!(
        "session {} stored {} frames, {} missing; link rejected {} byte runs \
         ({} before recording started)",
        stopped.session_id,
        stopped.frames_stored,
        stopped.frames_missing,
        device.snapshot().rejected_frames,
        rejected_at_start
    );
    assert_eq!(stopped.session_id, session.session_id);
    assert_eq!(stopped.frames_missing, 0, "frames were lost between device and store");
    // 100 Hz, minus up to one batch (10 frames) at each end.
    let expected = (seconds * 100) as i64;
    assert!(
        stopped.frames_stored > expected - 40 && stopped.frames_stored <= expected + 20,
        "stored {} frames, expected about {expected}",
        stopped.frames_stored
    );
    assert_eq!(
        device.snapshot().rejected_frames,
        rejected_at_start,
        "link corrupted data while streaming (PROB-006)"
    );
    assert_eq!(identity.config_sha256, stopped.config_sha256, "session must record the configuration");

    // Stored counts are the device's own, unmodified (DEC-007), and physical
    // units come from the device configuration.
    let connection = rusqlite::Connection::open(dir.0.join("ead.sqlite3")).unwrap();
    let (fax, fay, faz): (i64, i64, i64) = connection
        .query_row(
            "SELECT fax, fay, faz FROM raw_frames WHERE session_id = ?1 ORDER BY frame_index LIMIT 1",
            [&stopped.session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let counts = [fax as i16, fay as i16, faz as i16, 0, 0, 0];
    let (accel, _) = config.foot_anatomical(&counts);
    let magnitude = (accel[0] * accel[0] + accel[1] * accel[1] + accel[2] * accel[2]).sqrt();
    println!("first foot sample: {counts:?} counts -> {accel:?} g, |a| = {magnitude:.3}");
    assert!(
        (0.8..1.2).contains(&magnitude),
        "|a| = {magnitude:.3} g at rest; expected about 1 g"
    );

    // The raw view reads this session back through the same query the UI uses.
    let first = stopped.first_frame_index.expect("frames");
    let last = stopped.last_frame_index.expect("frames");
    let window = store
        .raw_window(&stopped.session_id, SignalGroup::FootAccel, first, last, 400)
        .unwrap();
    println!(
        "raw window: {} frames -> {} points, bucket {}, {} ms",
        last - first + 1,
        window.points,
        window.bucket,
        window.query_ms
    );
    assert!(window.points > 0);
    assert_eq!(window.axes.len(), 3);
    assert!(window.anatomical, "the session must carry its configuration");
    assert_eq!(window.unit, "g");
    assert!(window.time_s[0] == 0.0, "time is relative to the session's first frame");

    // Every bucket must bracket 1 g: the extremes of a still sensor are still
    // about 1 g, which is the check that decimation preserves the signal.
    for point in 0..window.points {
        for extreme in [0usize, 1] {
            let axes: Vec<f32> = (0..3)
                .map(|axis| {
                    let a = &window.axes[axis];
                    if extreme == 0 { a.min[point] } else { a.max[point] }
                })
                .collect();
            let m = (axes[0].powi(2) + axes[1].powi(2) + axes[2].powi(2)).sqrt();
            assert!(
                (0.7..1.3).contains(&m),
                "bucket {point} extreme {extreme} gives |a| = {m:.3} g"
            );
        }
    }

    // The session carries the configuration that produced it, so it reads
    // correctly with no device attached.
    let stored = store.session_config(&stopped.session_id).unwrap().expect("stored configuration");
    let reparsed = crate::protocol::parse_section(&stored).expect("parse stored configuration");
    assert_eq!(reparsed.imu.shank_mount, config.imu.shank_mount);
}
