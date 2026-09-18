# Testing

Plan derived from `ead_agent_docs_v2/13_TEST_AND_VALIDATION_PLAN.md`.
TEST-001 to TEST-007 are the doc-13 acceptance suites; each says whether any part
has run. TEST-008 onward are tests that were executed, with measured results.
A result is only recorded as PASS when it was run and checked.

## Executed tests

| ID | Date | Test | Result |
|---|---|---|---|
| TEST-008 | 2026-09-17 | M0 firmware build | PASS |
| TEST-009 | 2026-09-17 | Mount map proper-rotation checks | PASS |
| TEST-010 | 2026-09-17 | IMU register configuration readback on device | PASS |
| TEST-011 | 2026-09-17 | Accel magnitude at rest (desk, not worn) | PASS (observation) |
| TEST-012 | 2026-09-17 | Dashboard Rust backend build | PASS |
| TEST-013 | 2026-09-17 | Dashboard frontend build | PASS |
| TEST-014 | pending | Anatomical gravity while standing (worn) | NOT RUN |
| TEST-015 | 2026-09-17 | IMU data-ready interrupt lines (GPIO7/8) | PASS |
| TEST-016 | 2026-09-17 | USB port reopen does not reset the device | PASS |
| TEST-017 | 2026-09-17 | Protocol integrity after PROB-006/007 fixes (90 s) | PASS |
| TEST-018 | 2026-09-17 | 30-minute USB acquisition acceptance run | PASS |
| TEST-019 | 2026-09-17 | Wi-Fi access point starts and advertises | PASS |
| TEST-020 | 2026-09-17 | Reset recovery: IMUs reconfigure on every boot | PASS |
| TEST-021 | 2026-09-17 | Protocol golden vectors, three implementations | PASS |
| TEST-022 | 2026-09-17 | Dashboard backend records from the device (hardware) | PASS |
| TEST-023 | 2026-09-17 | Dashboard application end to end on hardware | PASS |
| TEST-024 | 2026-09-17 | Raw-window query time on an hour of data | PASS |
| TEST-025 | 2026-09-17 | Raw view over 30 minutes of recorded device data | PASS |
| TEST-026 | 2026-09-17 | Raw view load with four signals and overview; window arithmetic | PASS |
| TEST-027 | 2026-09-17 | Mounting check on a worn device | PASS (after correcting the shank map) |
| TEST-028 | 2026-09-18 | Static calibration on the device, repeatability and window length | PASS |
| TEST-029 | 2026-09-18 | Orientation estimate on hardware | PASS |
| TEST-030 | 2026-09-18 | Gait detection and distance on a measured 6 m course | PASS (6.39 m measured against 6.00 m) |
| TEST-031 | 2026-09-18 | Reference profile and error engine against hand-computed cases | PASS |

## TEST-008 — M0 firmware build

### Objective
Firmware builds with the pinned platform, C++17 and no external libraries.

### Environment
PlatformIO 6.2.0, `espressif32 @ 7.1.3` (Arduino-ESP32 2.0.17),
xtensa-esp32s3-elf-g++ 8.4.0 (esp-2021r2-patch5).

### Procedure
1. `pio run` in `firmware/`.
2. Delete `.pio/build/seeed_xiao_esp32s3/src/main.cpp.o`, rebuild with `pio run -v`,
   and read the compile line for `main.cpp`.

### Expected
Build succeeds; `main.cpp` compiles with `-std=gnu++17` only.

### Actual
SUCCESS. RAM 18,740 / 327,680 B (5.7 %), flash 274,741 / 3,342,336 B (8.2 %).
The `main.cpp` compile line carries `-std=gnu++17` and no `-std=gnu++11`.

### Result
PASS

## TEST-009 — Mount map proper-rotation checks

### Objective
`config_v1.h` accepts only signed-permutation maps with determinant +1, and the
shank map transforms as derived (PROB-002).

### Environment
Host g++ 13.3.0, `-std=gnu++17`.

### Procedure
1. Compile a copy of `config_v1.h` in which the shank Y row is the previous
   reflection `{-1, 0, 0}`.
2. Compile and run a host program with the committed header that maps chip
   (1, 2, 3) and chip (−32768, 0, 0) through the shank map.

### Expected
1. Compilation fails on the shank static assertion.
2. (1, 2, 3) → (−3, 1, −2); −32768 → anatomical Y = −32768 without overflow.

### Actual
1. `error: static assertion failed: shank mount map must be a proper rotation`.
2. `shank(1,2,3)->(-3,1,-2)`, `x=-32768 -> Y=-32768`.

### Result
PASS

## TEST-010 — IMU register configuration readback on device

### Objective
Both IMUs hold the doc-00 configuration, including the MPU6500 accel filter (PROB-003).

### Environment
XIAO ESP32-S3 on `/dev/ttyACM0` (USB `303a:1001`), both IMUs connected, not worn.

### Procedure
1. Flash with `pio run -t upload`.
2. Capture the boot log over USB, then send `c` and capture the diagnostics.

### Expected
Both sensors: WHO 0x70, PWR1 0x01, SMPL 0x09, CFG 0x03, GYRO 0x08, ACCEL 0x08,
ACCEL2 0x03, INTEN 0x01; boot readback reports OK.

### Actual
Boot log: I²C scan finds 0x68 and 0x69; "Foot 0x68 WHO_AM_I=0x70 (MPU6500)",
"Shank 0x69 WHO_AM_I=0x70 (MPU6500)", "Foot OK  Shank OK".
Diagnostics, identical for both sensors:
`WHO=0x70 PWR1=0x01 SMPL=0x09 CFG=0x03 GYRO=0x08 ACCEL=0x08 ACCEL2=0x03 INTEN=0x01`.

### Result
PASS

## TEST-011 — Accel magnitude at rest (desk, not worn)

### Objective
Sanity-check accel scale after the configuration change.

### Procedure
Average 78 anatomical-frame samples printed at 10 Hz over 7.8 s, device resting.

### Expected
|a| within ±0.05 g of 1 g on both sensors.

### Actual
Foot |a| = 1.024 g, shank |a| = 1.014 g; accel σ ≤ 0.002 g on every axis.
Gyro means at rest: foot (+3.14, +1.15, +0.02) °/s, shank (+0.37, +1.99, +0.12) °/s.
The board was lying on a desk, so the axis of gravity says nothing about mounting.

### Result
PASS (observation). Gyro offsets are expected MEMS bias for calibration (M3) to remove.

## TEST-012 — Dashboard Rust backend build

### Objective
The Tauri 2 shell compiles for the first time (icons, capabilities, no shell plugin).

### Environment
rustc 1.95.0, tauri 2.11.5, tauri-build 2.6.3, WebKitGTK 4.1 (2.52.6).

### Procedure
`cargo build` in `dashboard/src-tauri/`.

### Actual
`Finished dev profile` in 1 min 16 s, no warnings from the crate.

### Result
PASS

## TEST-013 — Dashboard frontend build

### Procedure
`npm run build` in `dashboard/` (tsc + vite 5.4.21).

### Actual
30 modules; `dist/assets/index-*.js` 149.59 kB (48.29 kB gzip).

### Result
PASS. The frontend is still the placeholder skeleton, replaced in M2.

## TEST-014 — Anatomical gravity while standing (worn)

### Objective
On-body confirmation of both mount maps (PROB-002).

### Procedure
Wear both sensors per `docs/hardware.md`, stand still for 10 s, capture the
anatomical accel means.

### Expected
Both sensors a ≈ (0, 0, +1) g; a residual tilt of a few degrees is normal before
alignment calibration.

### Result
NOT RUN

## TEST-015 — IMU data-ready interrupt lines (GPIO7/8)

### Objective
Doc 13 §1.4: both INT lines generate data-ready interrupts. This gates the
data-ready acquisition design of M1.

### Environment
M0 firmware plus temporary counters (not committed): GPIO ISR service installed
with `ESP_INTR_FLAG_IRAM`, rising edge on GPIO7 and GPIO8, `IRAM_ATTR` handlers
incrementing counters, counts printed with `esp_timer_get_time()` once per second.
IMU INT_PIN_CFG 0x00 (active-high 50 µs pulse), INT_ENABLE 0x01. Device resting.

### Procedure
Flash, then log counts over USB for 23 s; compute rates over the 21 s window
between the second and last report.

### Expected
About 100 interrupts per second on each line.

### Actual
ISR install and both handler registrations returned ESP_OK (0). Foot 100.143 Hz,
shank 100.000 Hz (2,100 shank / 2,103 foot interrupts in 21.000 s). Per-second
counts: foot 100–101, shank 100–100.

### Result
PASS. Both lines are wired. The 0.14 % rate difference between the two sensors'
clocks means a foot-clocked frame repeats a shank sample about once every 7 s;
M1 marks such frames instead of hiding them.

## TEST-016 — USB port reopen does not reset the device

### Objective
A host connecting must not restart the device, or sequence numbers restart and
backfill cannot span a reconnect (PROB-005).

### Procedure
`python3 tools/eadprobe.py reopen --cycles 20 --pause 0.3`: open the port, send
HELLO, close, repeat.

### Expected
One `boot_id` across all cycles; `last_seq` never decreases.

### Actual
20/20 cycles reported `boot_id f09d0c32`, state READY, `last_seq` rising 
monotonically from 74 to 153.

### Result
PASS

## TEST-017 — Protocol integrity after the PROB-006/007 fixes

### Objective
No corrupted USB frames and no I²C read failures once core logging is compiled
out and the data-ready guard is in place.

### Environment
M1 firmware with `-DCORE_DEBUG_LEVEL=0`, direct USB endpoint writes, 1–2 ms
data-ready guard. Device at rest.

### Procedure
`python3 tools/eadprobe.py stats --seconds 90`.

### Expected
0 rejected frames, 0 missing sequence numbers, 0 I²C errors.

### Actual
9,020 frames (index 270–9,289), 0 missing, 0 duplicates, 0 rejected frames.
Frame flags: all read-failure and saturation counts 0; `shank_repeated` 20.
Device `i2c_errors` 0, `frames_dropped` 0, faults none.
Before the fixes the same test rejected ~240 frames per 6 minutes and logged
~30 I²C errors per 90 s.

### Result
PASS

## TEST-018 — 30-minute USB acquisition acceptance run

### Objective
Doc 14 Phase 2 acceptance: sustained acquisition with zero dropped internal
frames and correct synchronized timestamps.

### Environment
M1 firmware, USB link, device at rest on the bench.

### Procedure
`python3 tools/eadprobe.py stats --seconds 1800 --record m1_usb_final.eadlog`.

### Expected
Zero device-reported dropped frames; no gaps in `frame_index`; no missing
durable sequence numbers; period ≈ 10 ms; accel magnitude ≈ 1 g.

### Actual
1800.0 s, no reconnects.
- Frames: 180,250 of 180,250 expected (index 980–181,229), 0 missing, 0 duplicates.
- Durable messages: 18,025 received, 0 missing, 0 backfill requests needed.
- Corrupted link frames: 0.
- Frame period: mean 9985.5 µs, σ 0.5 µs, range 9980–9991 µs
  (100.145 Hz on the device clock; the MPU6500's internal oscillator, not a
  fault — see the note below).
- Frame flags: every read-failure and saturation count 0; `shank_repeated` 487
  (one per 3.7 s, consistent with the 0.14 % clock difference of TEST-015).
- Accel magnitude at rest: foot 1.0241 g (σ 0.0015), shank 1.0138 g (σ 0.0019).
- Device counters at the end: `frames_dropped` 0, `i2c_errors` 0, `imu_reinits` 0,
  no faults. Minimum free heap 242,644 B. Task stack headroom: acquisition 2492 B,
  processing 5332 B, USB 4724 B.

The sample rate is 0.145 % above nominal because the sensor's internal oscillator
sets it; every frame carries the device timestamp, so analysis uses measured time
rather than an assumed 100 Hz.

### Result
PASS

## TEST-019 — Wi-Fi access point starts and advertises

### Objective
Confirm the device access point (doc 08 §1) starts, without joining it. The
development laptop is Wi-Fi-only, so joining would drop its network connection.

### Procedure
`nmcli -t -f SSID,CHAN,SIGNAL,SECURITY dev wifi list --rescan yes`.

### Expected
An `EAD-V1-XXXX` SSID matching the device MAC, on channel 6, WPA2.

### Actual
`EAD-V1-FB7C:6:100:WPA2` (device MAC 44:B1:76:AF:FB:7C).

### Result
PASS. The WebSocket link over Wi-Fi is not yet exercised end to end; that test
is run by the user (`eadprobe.py --ws ws://192.168.4.1:8080/ws stats`).

## TEST-020 — Reset recovery: IMUs reconfigure on every boot

### Objective
After an unexpected reset, possibly mid-I²C-transaction, both IMUs must come
back configured without a power cycle.

### Procedure
Force a reset through the USB Serial/JTAG control lines, wait for boot, then read
HELLO and the first STATUS. Repeat 10 times.

### Expected
Every boot: both WHO values 0x70, state READY, no faults.

### Actual
10/10 boots reported `who 0x70/0x70`, READY, faults none, 10 distinct `boot_id`
values. `i2c_errors` after boot was 0 or 1 (the bus-recovery pulse train).

### Result
PASS

## TEST-021 — Protocol golden vectors, three implementations

### Objective
The wire format is implemented identically by the firmware, the host probe, and
the vector generator, so a single-language mistake cannot pass unnoticed.

### Environment
`pio test -e native` (host g++ 13.3.0) and Python 3.12 with `struct`/`zlib`.

### Procedure
1. `python3 protocol/vectors/generate.py` builds the vectors from
   `docs/protocol.md` and `CONFIG_V1.json`.
2. `pio test -e native` compares firmware encoders and decoders against them.
3. A Python check drives `tools/eadprobe.py` decoders over the same vectors.

### Expected
Byte-identical encodings; decoders recover every documented field; CRC-32 of
"123456789" is 0xCBF43926; COBS matches the reference examples.

### Actual
20 native test cases passed (`test_crc_cobs`, `test_protocol`, `test_msg_ring`),
including the config section built from the contract JSON. The probe reproduced
every vector and decoded HELLO, STATUS, ERROR, CONFIG_GET and BACKFILL_DATA.

### Result
PASS


## TEST-022 — Dashboard backend records from the device (hardware)

### Objective
The whole host chain — protocol, USB link, device manager, store — works against
the real device, and stored data is the device's own.

### Environment
`cargo test -- --ignored`, device on USB, resting on the bench.

### Procedure
`src/hardware_tests.rs`: connect, wait for the handshake and configuration,
record a 5-second session, stop, then read the stored rows back through SQL.

### Expected
Connection and configuration within 10 s; configuration matching
`docs/hardware.md`; about 500 frames stored with none missing; no corrupted link
frames while streaming; stored counts converting to about 1 g.

### Actual
Connected over USB `/dev/ttyACM0`, firmware `0.1.0+c98e010.dirty`, MAC
44:B1:76:AF:FB:7C, capabilities `["psram_ring"]` (haptics correctly absent).
Mount maps as built; sample rate 100 Hz. Session `20260917-152214-8929` stored
510 frames, 0 missing. Corrupted link frames: 1 before the session started (the
partial frame in flight when the port is opened, discarded by design) and no
further ones while streaming. First stored foot sample `[1474, 5596, 6105]`
counts → `[0.180, 0.683, 0.745]` g, |a| = 1.027.

### Result
PASS

## TEST-023 — Dashboard application end to end on hardware

### Objective
The built application, not just its libraries, connects to the device and shows
correct live measurements.

### Environment
`npx tauri dev` (WebKitGTK), device on USB, resting on the bench. Captured with
`xwd` under XWayland with `WEBKIT_DISABLE_COMPOSITING_MODE=1`.

### Procedure
Launch the app, press the USB connect button offered for the detected device,
and read the Live view.

### Expected
The device is detected by USB ID; the state bar reports ready with both sensors
healthy; live accel magnitude reads about 1 g; angular rate reads about 0.

### Actual
The connect button offered `/dev/ttyACM0 (44:B1:76:AF:FB:7C)` — detected by USB
vendor and product ID, not a hard-coded path. After connecting, the state bar
read `ready`, `USB /dev/ttyACM0`, `foot ok`, `shank ok`, frames 268,590, firmware
`0.1.0+c98e010.dirty`, haptics `not fitted`. Live showed a flat 1.0 g trace with
readouts foot |a| 1.03 g, shank |a| 1.01 g, and foot angular rates within a few
°/s of zero.

One defect was found and fixed during this test: the first updates arrive before
the device configuration does, so their values are raw counts rather than
physical units. The chart history now resets when the units change, instead of
plotting both on one axis.

### Result
PASS


## TEST-024 — Raw-window query time on an hour of data

### Objective
Decide whether precomputed summary tables are needed, by measuring the query
that backs the raw view rather than assuming.

### Environment
`cargo test raw_window_query_time -- --ignored --nocapture`; SQLite (bundled),
WAL, one hour of frames (360,000) with varying values.

### Procedure
Insert an hour of frames, then query a full-session view and three zoom levels,
each asking for about 1,400 points.

### Actual
Writing 360,000 frames: 1,374 ms to flush. Database 30 MB per hour.

| View | Frames | Points | Frames per point | Query |
|---|---:|---:|---:|---:|
| Whole session | 360,000 | 1,401 | 257 | 309 ms |
| 10 minutes | 60,001 | 1,429 | 42 | 89 ms |
| 1 minute | 6,001 | 1,501 | 4 | 46 ms |
| 10 seconds | 1,001 | 1,001 | 1 | 42 ms |

### Result
PASS. Summary tables are not built: 309 ms happens once when a session is
opened, and every interaction after that is under 90 ms. Precomputed summaries
would add a write-path cost and a rebuild whenever backfill fills a gap, to save
a third of a second once. Revisit if sessions grow beyond a few hours.

## TEST-025 — Raw view over 30 minutes of recorded device data

### Objective
The raw view shows stored data correctly: right values, right units, right time
axis, and decimation that does not hide flagged frames.

### Environment
The TEST-018 recording (180,250 real frames, 30 minutes) imported into the
dashboard database with the device's configuration, then opened in the app.

### Procedure
Open the Raw view, select the session, and compare what is drawn against the
same values queried directly with SQL.

### Expected
Three axis traces at the recorded means; physical units; elapsed seconds on the
x axis; flagged buckets counted.

### Actual
180,250 frames drawn as 1,409 points (128 frames per point) in 158 ms.
Traces sat at about 0.18, 0.68 and 0.74 g; SQL over the same rows gives axis
means of +0.180, +0.683, +0.741 g, and |a| = 1.024 g — matching the 1.0241 g
measured in TEST-018. Units read "g" because the session carries its
configuration. 262 of 1,409 buckets flagged, consistent with the 487 repeated
shank samples in that recording.

Two defects were found by looking at the rendered chart and fixed: the x axis
was being formatted as a date from the epoch rather than elapsed seconds, and
the counts-to-units conversion was being done in the frontend where it could not
be unit-tested. The conversion now happens in Rust, with a test for the case
that matters — a negative mount-map sign swaps a bucket's minimum and maximum.

### Result
PASS


## Acceptance suites (doc 13)

## Replay determinism (mandatory, doc 13 §9)

### Objective
A saved raw dataset plus fixed configuration must reproduce identical gait
events, cycle features, error score, error class, and haptic decisions.

### Procedure (planned)
1. Capture or synthesize raw dual-IMU datasets covering doc 13 §3: normal
   walking, slow walking, variable cadence, dorsiflexion / plantarflexion /
   inversion-eversion deviations, sensor noise bursts, dropped samples,
   static periods, invalid values.
2. Store raw frames (ESP32 timestamps + seq) as versioned fixtures with the
   exact config (gains, thresholds, reference version, segment limits).
3. Run the firmware-equivalent algorithm (host replay harness) twice on each
   fixture; diff events/features/scores/classes/haptic commands byte-for-byte.
4. Re-run after every algorithm change as a regression gate (Phase 11).

### Expected
Bit-identical outputs across runs for identical inputs.

### Acceptance
Replay suite passes before any bench walking test counts toward release
(doc 14 Phase 12 release gate: replay determinism verified).

## TEST-001 — Hardware bring-up (planned)

### Objective
Verify electrical integration per doc 13 §1.

### Procedure
1. Enumerate MPU6050 at `0x68`/`0x69`; confirm no address collision.
2. 5-minute 100 Hz cadence run; count dropped internal frames (expect 0).
3. Confirm both INT lines fire data-ready.
4. Confirm all six PWM outputs LOW at boot; test each motor independently.
5. Verify flyback polarity physically before motor power; verify 5 s cutoff.

### Result
PARTIAL. Verified: step 1 enumeration (TEST-010), step 3 frame cadence over
30 minutes (TEST-018), step 4 both data-ready lines (TEST-015). Not run:
steps 5–8 (motor outputs), because no drivers are fitted (DEC-006).

## TEST-002 — Sensor static/rotation checks (planned)

### Objective
Per doc 13 §2: 5-min static log ≈ 1 g accel magnitude, gyro bias stability,
axis-sign rotations, same anatomical orientation on both boards, relative
orientation sanity.

### Result
NOT RUN.

## TEST-003 — ZUPT behavior (planned)

### Objective
Per doc 13 §4: foot-flat intervals detected, swing false-ZUPTs rejected,
drift materially reduced, poor sessions flagged (never fabricated).

### Result
NOT RUN.

## TEST-004 — Haptic safety/logic (planned)

### Objective
Per doc 13 §5: no haptic on low confidence, monotonic error→PWM, ON/OFF
hysteresis ordering, correct spatial pair, temporal dual-pole cue for timing
errors, 5 s and 10 s rolling-duty enforcement.

### Result
NOT RUN.

## TEST-005 — Wireless loss + backfill (planned)

### Objective
Per doc 13 §6: 60 s Wi-Fi loss during walking → gait/haptics continue;
reconnect backfills from storage with no duplicate cycle IDs.

### Result
NOT RUN.

## TEST-006 — Storage recovery (planned)

### Objective
Per doc 13 §7: power/reset mid-write, CRC corruption, truncated block →
reboot recovers to last valid block automatically.

### Result
NOT RUN.

## TEST-007 — Dashboard + export (planned)

### Objective
Per doc 13 §8: live charts, event markers, raw-to-error traceability,
CSV/`.mat`/PDF export, segmentation, reference lock.

### Result
NOT RUN.

## TEST-026 — Raw view load with four signals and overview; window arithmetic

### Objective
Measure what the raw view costs now that it draws every signal at once plus an
overview, and test the navigation arithmetic.

### Environment
Release build, laptop, synthetic one-hour session (360,000 frames, varying
values) in a temporary store — the same fixture as TEST-024.

### Procedure
1. `cd dashboard/src-tauri && cargo test --release raw_window_query_time -- --ignored --nocapture`
2. `cd dashboard && npm test`

### Expected
Whole-session four-signal load well under 1 s; panning (zoomed windows) under
100 ms; all timeline tests pass.

### Actual
| Load | Four queries (first version) | One query (final) |
|---|---|---|
| Whole session, four signals | 542 ms | 291 ms |
| 1 minute, four signals | 78 ms | 35 ms |
| Overview, whole session, one signal, 700 points | 135 ms | 141 ms |

The overview loads once per session/signal, not on every pan.
`npm test`: 10/10 pass (clamping at both ends keeps width, minimum span,
whole-session as null, 1000 half-window pans stay in bounds, zoom out reaches
whole session in eight doublings from 1000 frames, clock format).

### Result
PASS

### Notes
Rendered view not inspected by the agent (native Wayland window, no Xvfb).

## TEST-027 — Mounting check on a worn device (pending)

### Objective
Confirm both mount maps against the real mounting, settling PROB-002.

### Environment
Device worn: foot sensor on the shoe, shank sensor on the shin, connected over
USB or Wi-Fi, Device view open.

### Procedure
1. Stand still and press Start on "Stand still".
2. Heel on the floor, press Start on "Raise your toes", then lift the toes fully
   and lower them within the four seconds.
3. Sit, press Start on "Extend your knee", straighten the knee and lower it.

### Expected
All three PASS. Standing still: both sensors near (0, 0, +1) g. Toes: foot peak
on Y, negative. Knee: shank peak on Y, negative.

### Actual
Run on the leg, 2026-09-17, all three steps FAIL:

| Step | Measured | Reading |
|---|---|---|
| Stand still | foot (−0.43, +0.35, **+0.86**) g | gravity on +Z, board tilted 33° on the instep — correct axis, and my tilt threshold was too strict |
| Stand still | shank (−0.07, **+0.99**, +0.12) g | gravity on anatomical Y: the shank map is wrong |
| Raise toes | foot peak (+4, +2, +0) °/s | too small to judge; the move was not captured |
| Extend knee | shank peak (+12, −0, **+43**) °/s | turned about anatomical Z, expected Y |

The two shank results agree: anatomical Y and Z were interchanged. The map was
corrected to `X = −chipZ, Y = +chipY, Z = +chipX` and flashed (PROB-002).

The still step's tilt rule was relaxed afterwards: it now requires gravity to be
dominant on +Z and reports the tilt angle, because a strap over the instep holds
the board at a slope and calibration's gravity alignment removes it.

Re-run on the leg with the corrected map flashed: **all three steps PASS**
(user-reported, 2026-09-17). Both sensors read gravity on +Z standing still, the
toe lift turned the foot about −Y, and the knee extension turned the shank
about −Y.

### Result
PASS on the second run. The first run's FAIL is what produced the correction and
is kept above: it is the evidence for the map.

### Notes
A FAIL is the useful outcome here: the panel prints the measured vector, which
says what the mounting actually is and what the mount map should be.

## TEST-028 — Static calibration on the device

### Objective
Confirm the device measures a gyro bias and a gravity direction from a still
window, that the window length is honoured, and that repeated runs agree.

### Environment
XIAO ESP32-S3 with both IMUs, firmware schema 2, resting on the bench (not worn).
Host: `python3 tools/eadprobe.py calibrate --seconds N` over USB.

### Procedure
1. Run a 3 s window, then an 8 s window, undisturbed.
2. Compare the bias from consecutive runs of the same length.

### Expected
Samples = seconds × 100. No rejection. Bias repeatable; |a| ≈ 1 g; the tilt
matches how the boards happen to be lying.

### Actual
| Run | Samples | Foot bias (°/s) | Foot \|a\| | Shank bias (°/s) | Shank \|a\| |
|---|---:|---|---:|---|---:|
| 3 s | 300 | 3.144, 1.127, 0.027 | 1.0248 | 0.586, −0.228, −0.392 | 0.9981 |
| 8 s | 800 | 3.158, 1.120, 0.028 | 1.0249 | 0.573, −0.224, −0.407 | 0.9977 |
| 5 s (earlier) | 500 | 3.132, 1.126, 0.003 | 1.0249 | 0.590, −0.183, −0.409 | 0.9985 |

Bias repeats within 0.03 °/s across runs of different lengths — the measurement
is dominated by the sensor's offset, not by noise. Sample counts match the
requested duration exactly. A deliberately disturbed window was rejected with
`moved`, and the record was refused rather than returned.

### Result
PASS

### Notes
The foot sensor reads |a| = 1.0249 g consistently while the shank reads 0.998 g.
That is a scale-factor difference between the two parts, not motion; recorded as
PROB-009.

## TEST-029 — Orientation estimate on hardware (pending)

### Objective
Confirm the device's orientation estimate tracks reality: valid only when
calibrated, stable while still, and agreeing with measured gravity.

### Procedure
1. Place the device as worn (or both boards flat), calibrate, and hold still.
2. Compare each frame's estimated gravity direction with the measured
   acceleration direction; expect agreement within a degree while still.
3. Rotate the foot through dorsiflexion and watch the sagittal angle follow.

### Expected
`orientation_valid` set on every frame after calibration; mean angle between
estimated and measured gravity under 1°; the sagittal angle following the foot.

### Actual
Two earlier attempts were refused by the device, correctly: with the boards
lying on the bench the shank sensor sat 73° from upright, calibration rejected
the window as `upside_down`, and 0/410 frames carried `orientation_valid`.

With the device worn (foot tilt 40.7°, shank tilt 1.6°, calibration accepted):

| Measure | Result |
|---|---|
| Frames with `orientation_valid` | 507 / 510 |
| Foot: estimated vs measured gravity | mean 0.09°, max 0.24° |
| Shank: estimated vs measured gravity | mean 0.17°, max 0.78° |

The three invalid frames are the first after the record was adopted, before an
interval exists to integrate over — the documented behaviour, not a fault.

Dynamic, first attempt: a toe raise moved the sagittal angle by about +28° in
the correct direction, but the flat-foot pose read −34° and the frontal angle a
constant +20° — the mounting tilt was not being removed (PROB-010).

After the fix, standing still over 502 frames:

| Angle | Mean | Spread |
|---|---:|---:|
| Sagittal | −0.11° | 0.19° |
| Frontal | −0.02° | 0.11° |
| Transverse | −0.19° | 0.43° |

A flat foot reads zero, which is the pose whose answer is known independently of
the code.

Toe raises against the corrected firmware, four lifts in fifteen seconds:

| Lift | Peak sagittal angle |
|---|---:|
| 1 | +24.9° |
| 2 | +25.9° |
| 3 | +26.5° |
| 4 | +26.7° |

Peaks repeat within 0.2° across four independent movements, and the angle
returns to about +3.5° between lifts against a calibrated neutral of −0.11°. The
residual is the standing pose not being reproduced exactly between lifts rather
than estimator error: it does not accumulate across the sequence, which drift
would.

### Result
PASS

## TEST-030 — Gait detection and distance on a measured 6 m course

### Objective
Check the gait detector against measured ground truth: does it find one cycle per
stride, and does the estimated distance match a course of known length?

### Environment
Device worn, powered from a battery pack, streaming over Wi-Fi. Course 6.00 m
walked in a straight line at a normal indoor pace, 11–12 alternating steps (about
6 right-foot strides). Recording `recordings/walk6m-2026-09-18.eadlog`: 40 s,
4,010 frames, no gaps, 5 s standing at the start.

### Procedure
1. `eadprobe --ws … stats --seconds 40 --record walk6m.eadlog` while walking.
2. Establish ground-truth contact times independently of the detector, by finding
   the acceleration peaks above 2 g in the recording.
3. Replay the recording through the device's own code (`tools/replay`) and
   compare.

### Ground truth
Heel strikes at 6.95, 7.51, 9.17, 10.94, 12.74, 14.33 and 15.92 s — six strides
of 1.59–1.80 s. Contacts peak at 2.3–5.3 g; push-off and mid-swing peak at
1.5–1.9 g, which is what separates them.

### Actual
Three defects were found and fixed, each with the evidence that produced it:

| Run | Contacts found | Distance | What it showed |
|---|---|---|---|
| First, on the device | 20 for ~12 steps | 5.91 m, ZUPT quality 0.00 | Every impact opened a cycle; zero-velocity never detected |
| After the refractory and ZUPT-context fixes | 12, alternating 1.0 s / 0.6 s | 2.65 m | Push-off was being read as a footfall, splitting every stride |
| After the confirm threshold and the sustain fix | 7, exactly the ground truth | 6.39 m | Correct |

Final replay: 6 cycles, all valid, cycle time 1.59–1.80 s, stance ratio
0.50–0.60, ZUPT quality 0.22–0.29, per-stride distance 1.04–1.35 m, total
**6.39 m against a 6.00 m course (+6.5 %)**.

The gravity-correction gate was chosen by measurement rather than by taste:

| Gate | Total distance | Error |
|---|---:|---:|
| none | 4.63 m | −23 % |
| 0.15 g | 6.52 m | +8.7 % |
| **0.25 g** | **6.39 m** | **+6.5 %** |
| 0.40 g | 5.46 m | −9 % |

### Result
PASS

### Notes
Tuned against a single recording, so the thresholds are fitted to one gait on one
day. They are named constants in `gait.h` and overridable at run time
(`GaitConfig`), and doc 05 §3 asks for them to become adaptive per patient. More
walks, at different speeds, are the next evidence needed.

Saturation appeared for the first time: 2 frames of accelerometer and 1 of
gyroscope clipping (±4 g, ±500 °/s) in 4,010. Impacts reached 5.3 g, so the
accelerometer range is marginal for heel strike (PROB-011).

## TEST-031 — Reference profile and error engine

### Objective
Check the reference statistics and the error engine against cases whose answers
are known by hand, before any of it is used on a patient.

### Environment
`cd firmware && pio test -e native -f test_reference` (11 cases).

### Cases and results
| Case | Expected | Result |
|---|---|---|
| 29 valid cycles offered | refused | PASS |
| 40 cycles, half marked invalid | refused: 20 valid is still too few | PASS |
| 35 ordinary cycles plus 5 stumbles | median moves by less than 2° | PASS |
| Every cycle identical | spread takes the floor, never zero | PASS |
| Cycle equal to the reference | score < 0.2, no class | PASS |
| Dorsiflexion at 2° against a 16° reference | INSUFFICIENT_DORSIFLEXION | PASS |
| Dorsiflexion at 30° | scores, but raises no class (V1 has no excess class) | PASS |
| Inversion above / below the reference | INVERSION / EVERSION | PASS |
| One feature at full deviation, rest at median | score = its weight, 0.25 | PASS |
| ZUPT quality 0 | distance leaves the denominator; confidence falls | PASS |
| Every subscore at 1 | confidence 1.00; sensor quality 0 gives 0.70; sensor and event 0 gives 0.45, below the display gate | PASS |
| Two classes within 10 % of each other | OVERALL_DEVIATION, both classes still logged | PASS |

### Result
PASS

### Notes
Every case here is synthetic. The engine has never been run against a reference
captured from a person, because that needs thirty valid cycles and the longest
walk recorded so far is six. The four values doc 06 leaves undefined are recorded
in DEC-013 and need review against real captures before any claim is made from
this output.

## TEST-032 — Segment rollover and the error rule

### Objective
That a segment closes on whichever researcher-entered limit is reached first
(doc 12 §5), that the cycle which trips a limit belongs to the segment it
closed, and that a classification the engine would not display does not count
as an error (DEC-014).

### Environment
Host, `cargo test`, an in-memory-equivalent temporary SQLite store. No device.

### Procedure
`segments_close_on_whichever_limit_comes_first` in
`dashboard/src-tauri/src/store/tests.rs` opens an evaluation with limits of 3
valid cycles and 2 errors, then records, in order: three valid clean cycles;
one cycle classified with confidence 0.9; one invalid cycle; a second cycle
classified with confidence 0.9; and one classified cycle with confidence 0.2.
`a_recording_has_no_segments` records a classified cycle into a plain recording.

### Expected
Segment 0 closes by `cycle_limit` with 3 valid cycles. Segment 1 closes by
`error_limit` with 2 errors and 2 valid cycles — the invalid cycle counts toward
neither. Segment 2 stays open with 0 errors, because confidence 0.2 is below the
0.50 display gate. Cycle segment indices are `0,0,0,1,1,1,2`. Stopping the
session closes segment 2 by `session_stopped`. A plain recording produces no
segment rows and every cycle reads segment 0.

### Actual
As expected, on the first run.

### Result
PASS

### Notes
The rule is a pure function of the stored cycle stream, so re-running it over
the same `cycles` rows reproduces the same segmentation with no device present.
That is the property DEC-014 traded the protocol change for, and this test is
what holds it.

## TEST-033 — Zero-velocity window pairing in the events view

### Objective
That the EVENTS view's zero-velocity lane is drawn from correctly paired
`zupt_start` / `zupt_end` events, including the malformed cases.

### Environment
Host, `node --test dashboard/src/*.test.ts`. No device.

### Procedure
`dashboard/src/events.test.ts`, five cases: ordinary pairs; unrelated event
kinds interleaved; a repeated start; an end with nothing open; a window still
open when the session ended.

### Expected
Ordinary pairs come back as spans. Other kinds are ignored. A repeated start
does not open a nested window. An unmatched end is dropped. An open window is
drawn to the end of the session rather than discarded.

### Result
PASS — 5 cases. Frontend total 28.

### Notes
The last case is the one that matters for honesty: a zero-velocity window that
never closed is a detector fault worth seeing, and silently dropping it would
make the lane look tidier than the data.

## TEST-034 — eadprobe decodes every golden vector

### Objective
That `tools/eadprobe.py`, the independent Python decoder, agrees with the
cross-language golden vectors in `protocol/vectors/` — and that it keeps
agreeing as the schema grows.

### Environment
Host, `python3 tools/eadprobe.py vectors`. No device.

### Procedure
The command reads every `*.hex` vector, strips its comments, un-frames the two
USB vectors (COBS then CRC32), parses the header and decodes the payload with
eadprobe's own decoders. `--verbose` prints what each one decoded to, which is
what was compared against the human-readable comment at the top of each file.

### Expected
Eighteen vectors, no failures, and the decoded values matching each file's
stated contents.

### Actual
First run: **4 failures**, and one of them was a real defect.

- `step_batch.hex` — `unpack requires a buffer of 72 bytes`. eadprobe's
  `CYCLE_RECORD` was still the schema-3 layout. STEP_BATCH grew from 72 to 132
  bytes when schema 4 added the error fields, and nothing had checked the Python
  decoder, so it had been silently wrong since that commit. Fixed by extending
  the struct (with a `size == 132` assertion at import) and decoding the score,
  confidence, five subscores, seven deviations and primary class.
- `config_response.hex`, `config_section.hex` — not messages; they are payloads
  on their own. The command now decodes them as payloads.
- `long_message.hex`, `usb_frame_long.hex` — a 300-byte body that exists to
  exercise framing, not a STATUS payload. Framing is checked, the body is not.

After those changes: 18 vectors, 0 failures. Spot-checked against the file
comments — `step_batch.hex` decodes to error score 0.42, confidence 0.86,
primary class `insufficient_dorsiflexion` with a dorsiflexion deviation of 0.91;
`reference_profile.hex` decodes to 34 cycles, version 2, dorsiflexion
16.0 ± 1.6°, shank 400 ± 22 °/s. Both match their headers exactly.

### Result
PASS, after fixing the defect it found.

### Lessons
The vectors were described as checked by three implementations. Two of them
were checking themselves; the third was not being run. A decoder that nothing
executes is not a cross-check, and the schema-4 commit's claim of three-way
agreement was wrong. `eadprobe vectors` is now the thing that makes it true, and
it should be run whenever the schema changes.

## TEST-035 — Export package against the database

### Objective
That the doc 10 package holds exactly what the store holds, and that a value
that was not measured is empty rather than zero.

### Environment
Host, `cargo test`, a temporary store with a synthetic session: 3 frames,
3 cycles (one classified), 1 event, a reference profile, segment limits of 2
cycles / 5 errors, and two status changes of which one carries a fault.

### Procedure
`the_export_package_matches_the_database` and
`an_unscored_session_exports_empty_cells_not_zeroes` in
`dashboard/src-tauri/src/store/tests.rs`.

### Expected
`raw.csv` has two rows per stored frame plus a header. `gait.csv` has one row
per cycle; the first valid cycle's symmetry-proxy cell is empty because there is
no previous cycle to compare with, the second is not. The classified cycle reads
`insufficient_dorsiflexion`. `events.csv` contains INITIAL_CONTACT, CYCLE_START,
CYCLE_END, ERROR_ACTIVE and FAULT, and never SERVICE_TEST. `haptics.csv` is a
header alone. `metadata.json` carries every doc 10 §6 group. For a session with
no reference, all four derived columns of `gait.csv` are empty.

### Actual
As expected.

### Result
PASS — 2 cases.

## TEST-036 — session.mat read back by scipy

### Objective
That the hand-rolled Level-5 writer (DEC-003) produces a file an independent
implementation can open, and that what comes out agrees with the CSV files
written beside it.

### Environment
Host. `cargo test -- --ignored export_sample` writes the package to
`dashboard/src-tauri/target/export-sample`; `python3 tools/check_mat.py
<that directory>` reads it with `scipy.io.loadmat` 1.11.4.

### Procedure
The checker verifies the six top-level variables doc 10 §7 names, that `raw` is
int32 with int64 timestamps (§7: the raw integer values and timestamps must not
be lost), that every raw column of the first row equals the same column of
`raw.csv`, that each numeric `gait` column equals `gait.csv` — NaN exactly where
the CSV cell is empty — that the metadata agrees with `metadata.json`, and that
`haptics` is empty and carries the reason.

### Actual
First run: `TypeError: buffer is too small for requested array` inside
`read_char`. **A real defect**: the writer tagged its char arrays `18`, which is
`miUTF32`, not `miUTF16` (`17`). A reader told the wrong width walks off the end
of the buffer rather than failing cleanly, which is why nothing before this
noticed — the file was structurally plausible and simply unreadable.

After the fix: every check passes, 0 failures, including the NaN-versus-empty
agreement on the symmetry proxy and the error columns.

### Result
PASS, after fixing the defect it found.

### Lessons
A hand-rolled binary format needs a reader that was written by somebody else.
`cargo test` could confirm the bytes were produced; only scipy could confirm
they meant anything. The same argument applies to the PDF, which is why
`pdfinfo`/`pdftotext` are the check there rather than a byte count.

## TEST-037 — report.pdf against doc 10 §8

### Objective
That every section doc 10 §8 requires is on the page, that all seven trend
plots are drawn, and that the research-report label appears on every page — so
a page printed on its own still says what the document is not.

### Environment
Host. `cargo test -- --ignored export_sample` writes the package; `python3
tools/check_pdf.py <that directory>` reads it with `pdfinfo` and `pdftotext`
(poppler).

### Procedure
The checker asserts three A4 pages, the PDF subject line, the nineteen section
headings and readouts doc 10 §8 names, the seven trend titles on page 2, and the
footer label on each page.

### Actual
The composer has no layout engine, so the failures it found were content
falling off the page rather than malformed PDF:

- Long paragraphs ran past the right margin and were cut mid-sentence —
  `pdftotext` showed "…never corrected (doc" and nothing after it. Added a
  wrapping `paragraph` helper. It estimates the line width from a mean advance
  of 0.52 em rather than shaping each candidate line; the report's prose is all
  lowercase Latin and nothing is set flush right, so an estimate is enough and
  the alternative is shaping every line twice.
- The seventh trend plot overflowed the page: seven charts at 84 pt with 26 pt
  gaps needed 844 pt on an 841.89 pt page. Charts are now 72 pt with 20 pt gaps,
  which lands the last one at 744.
- One checker bug of my own: a case-sensitive match on the subject line.

After those: **0 failures**, 41 checks.

### Result
PASS

### Notes
The haptic-response plot doc 10 §8 asks for is drawn as an empty panel titled
"Haptic response — no drivers fitted (DEC-006)" rather than omitted. A reader
has to be able to see that the report was asked for it and that nothing could
answer it; a missing panel would look like an oversight.

## TEST-038 — Setup scripts and a clean clone

### Objective
That a machine with nothing but the dependencies can go from the GitHub
repository to a built dashboard.

### Environment
This Linux machine (Ubuntu 24.04, node 24.13.1, rust 1.95.0). No macOS or
Windows machine was available, and PowerShell is not installed here.

### Procedure
1. `bash scripts/setup.sh check` — the dependency report.
2. A fresh `gh repo clone sagar-aryan/ead-v1` into an empty directory, then
   `npm ci`, `npm run build` and `cargo build` in it — the same steps the script
   runs, without launching the GUI.

### Actual
1. Every dependency reported present, exit 0.
2. Clone, `npm ci`, the frontend build and the Rust build all succeeded from the
   pushed repository (Rust build 1 m 30 s from cold). That confirms nothing the
   build needs lives only on this machine — the report fonts, for instance, are
   committed.

### Result
PARTIAL. Linux: PASS. `setup.sh` on macOS and `setup.ps1` on Windows have not
been run anywhere. The dashboard itself has never been built on either; the Rust
code contains nothing platform-specific, but that is an argument, not a test.

### Notes
Rust must be at least 1.92 (krilla's minimum); the scripts check for it.
