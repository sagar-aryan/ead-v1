//! Hardware-in-the-loop checks for the dashboard backend: protocol, link,
//! device manager and store against a real device over USB.
//!
//! Ignored by default because they need the device attached. Run with:
//!   cargo test -- --ignored --nocapture

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::device::{self, Device, LinkState, Sink};
use crate::link::LinkTarget;
use crate::protocol::{AccelSample, RawFrame, Status};
use crate::store::{DeviceIdentity, SessionKind, SignalGroup, Store};

struct StoreSink(Arc<Store>);

impl Sink for StoreSink {
    fn raw_frames(&self, frames: &[RawFrame]) {
        self.0.record_frames(frames);
    }
    fn raw_accel(&self, samples: &[AccelSample]) {
        self.0.record_accel(samples);
    }
    fn status(&self, _status: &Status) {}
    fn gait(&self, cycles: &[crate::protocol::GaitCycle], events: &[crate::protocol::GaitEvent]) {
        self.0.record_gait(cycles, events);
    }
    fn device_restarted(&self) -> Option<String> {
        self.0.end_session_at_restart().expect("end session").map(|id| format!("ended {id}"))
    }
}

/// Starts a USB link task; send `true` on the returned sender to stop it.
fn connect_usb(
    runtime: &tokio::runtime::Runtime,
    device: &Arc<Device>,
) -> (tokio::sync::watch::Sender<bool>, tokio::task::JoinHandle<()>) {
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let link = runtime.spawn(device::run(device.clone(), LinkTarget::Usb { port: None }, stop_rx));
    wait_for(Duration::from_secs(15), "device connection", || {
        device.snapshot().link_state == LinkState::Connected && device.is_live()
    });
    (stop_tx, link)
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
    assert!(snapshot.haptics_fitted, "schema 7 runs error-driven feedback (DEC-023)");
    assert!(!snapshot.haptic_switch_on, "the haptic master switch is off after boot");
    assert_eq!(snapshot.faults, Vec::<&str>::new(), "device reports faults");

    // The configuration must be the as-built one (docs/hardware.md): the BNO086
    // build with its measured maps (TEST-051).
    let config = device.config().expect("configuration");
    assert!(matches!(config.imu.bus, crate::protocol::config::SensorBus::Bno086 { .. }));
    assert_eq!(config.imu.foot_mount, [[0, -1, 0], [1, 0, 0], [0, 0, 1]]);
    assert_eq!(config.imu.shank_mount, [[0, 0, 1], [1, 0, 0], [0, 1, 0]]);
    assert_eq!(config.imu.sample_hz, 200, "200 Hz frames (DEC-021)");
    assert!(config.haptics.fitted, "the configuration agrees with HELLO (DEC-023)");

    // The boot-time sensor check, decoded from the device's own bytes: every
    // step passes on both sensors (TEST-043 wiring).
    let seen = device.request_sensor_check(false).expect("request sensor check");
    wait_for(Duration::from_secs(3), "sensor check", || device.service_replies() > seen);
    let check = device.sensor_check().expect("sensor check");
    for (name, sensor) in [("foot", &check.foot), ("shank", &check.shank)] {
        assert_eq!(sensor.passed, crate::protocol::CHECK_STEPS.to_vec(), "{name} check");
        assert_eq!(sensor.part_number, 10_004_563, "{name} part");
    }

    // Opening the port mid-stream leaves a partial frame in the buffer, which is
    // discarded by design. What matters is that nothing is corrupted afterwards.
    let rejected_at_start = device.snapshot().rejected_frames;

    store.create_patient("HW-TEST", "Hardware test").expect("create patient");
    let identity = DeviceIdentity { calibration: None,
        firmware: snapshot.firmware.clone(),
        config_sha256: device
            .config_sha256()
            .map(|d| d.iter().map(|b| format!("{b:02x}")).collect()),
        mac: snapshot.mac.clone(),
        boot_id: snapshot.boot_id,
        config_section: device.config_section(),
    };
    let session = store
        .start_session("HW-TEST", SessionKind::Recording, &identity, None, None)
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
    // The configured rate, minus up to one batch (10 frames) at each end.
    let expected = (seconds * u64::from(config.imu.sample_hz)) as i64;
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
    // Both accelerometers at their native rate (RAW_ACCEL_BATCH); the rate is
    // printed, not asserted, until a measurement says what to expect.
    let accel = store.accel_count(&stopped.session_id).unwrap();
    println!("{accel} accelerometer samples, {:.0} per second", accel as f64 / seconds as f64);
    assert!(accel > 0, "no accelerometer samples were stored");

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
        .raw_window(&stopped.session_id, &[SignalGroup::FootAccel], first, last, 400)
        .unwrap();
    println!(
        "raw window: {} frames -> {} points, bucket {}, {} ms",
        last - first + 1,
        window.points,
        window.bucket,
        window.query_ms
    );
    assert!(window.points > 0);
    assert_eq!(window.signals[0].axes.len(), 3);
    assert!(window.anatomical, "the session must carry its configuration");
    assert_eq!(window.signals[0].unit, "g");
    assert!(window.time_s[0] == 0.0, "time is relative to the session's first frame");

    // Every bucket must bracket 1 g: the extremes of a still sensor are still
    // about 1 g, which is the check that decimation preserves the signal.
    for point in 0..window.points {
        for extreme in [0usize, 1] {
            let axes: Vec<f32> = (0..3)
                .map(|axis| {
                    let a = &window.signals[0].axes[axis];
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
    let reparsed = stored.parse().expect("parse stored configuration");
    assert_eq!(reparsed.imu.shank_mount, config.imu.shank_mount);
}

/// TEST-042's hardware steps and PROB-019: a replug keeps the calibration and the
/// recording; a reset of the board ends both, and the calibration gate blocks.
#[test]
#[ignore = "requires the EAD device attached over USB and still; resets it"]
fn a_device_reset_ends_the_session_and_its_calibration() {
    let dir = TempDir::new();
    let store = Store::open(dir.0.join("ead.sqlite3")).expect("open store");
    let device = Arc::new(Device::new(Arc::new(StoreSink(store.clone()))));
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();

    let (stop, link) = connect_usb(&runtime, &device);
    let first_boot = device.snapshot().boot_id.expect("boot id");
    // Still on the desk, a 2 s window is now and then rejected as "moved"
    // (PROB-022); an operator would simply calibrate again, and so does this.
    let mut record = None;
    for _ in 0..3 {
        device.start_calibration(2000).expect("start calibration");
        std::thread::sleep(Duration::from_millis(2500));
        wait_for(Duration::from_secs(5), "calibration record", || {
            device.snapshot().status.is_some_and(|s| s.calibration_state != 1)
        });
        record = device.snapshot().calibration;
        if record.is_some_and(|r| r.usable()) {
            break;
        }
    }
    let record = record.expect("record");
    assert!(
        record.usable(),
        "calibration rejected three times ({:?}): the device must be still",
        crate::protocol::calibration_rejections(record.reject)
    );
    store.create_patient("HW-RESET", "Hardware test").expect("create patient");
    let session = store
        .start_session("HW-RESET", SessionKind::Recording, &DeviceIdentity::default(), None, None)
        .expect("start session");
    wait_for(Duration::from_secs(5), "frames stored", || {
        store.flush().unwrap();
        store.frame_count(&session.session_id).unwrap() > 0
    });

    // A replug of the same boot: nothing ends.
    stop.send(true).unwrap();
    runtime.block_on(link).unwrap();
    let (stop, link) = connect_usb(&runtime, &device);
    assert_eq!(device.snapshot().boot_id, Some(first_boot));
    assert!(device.snapshot().calibration.is_some(), "a replug lost the calibration");
    assert_eq!(store.recording_session(), Some(session.session_id.clone()));

    // A reset, as `pio run -t upload` does it, with the link closed so the port is free.
    stop.send(true).unwrap();
    runtime.block_on(link).unwrap();
    let esptool = std::env::var("HOME").unwrap() + "/.platformio/packages/tool-esptoolpy/esptool.py";
    let reset = std::process::Command::new("python3")
        .args([esptool.as_str(), "--chip", "esp32s3", "--after", "hard_reset", "read_mac"])
        .output()
        .expect("run esptool");
    assert!(reset.status.success(), "esptool: {}", String::from_utf8_lossy(&reset.stderr));
    let (stop, link) = connect_usb(&runtime, &device);
    wait_for(Duration::from_secs(10), "the new boot's HELLO", || {
        device.snapshot().boot_id.is_some_and(|id| id != first_boot)
    });

    // The session ended where the old boot's data ends, and said so.
    assert_eq!(store.recording_session(), None);
    assert_eq!(store.ended_by_restart(), Some(session.session_id.clone()));
    assert!(store.session(&session.session_id).unwrap().stopped_at.is_some());
    let snapshot = device.snapshot();
    assert_eq!(snapshot.session_kind, None);
    assert_eq!(snapshot.last_error, Some(format!("ended {}", session.session_id)));
    // The new boot has no calibration, so the session gate blocks (PROB-018).
    assert_eq!(snapshot.calibration, None);

    stop.send(true).unwrap();
    runtime.block_on(link).unwrap();
}
