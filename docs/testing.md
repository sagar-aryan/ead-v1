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
| TEST-032 | 2026-09-18 | Segment rollover and the error rule | PASS |
| TEST-033 | 2026-09-18 | Zero-velocity window pairing in the events view | PASS |
| TEST-034 | 2026-09-18 | eadprobe decodes every golden vector | PASS after fixing the defect it found |
| TEST-035 | 2026-09-18 | Export package against the database | PASS (synthetic session) |
| TEST-036 | 2026-09-18 | `session.mat` read back by scipy | PASS after fixing the defect it found |
| TEST-037 | 2026-09-18 | `report.pdf` against doc 10 §8 | PASS |
| TEST-038 | 2026-09-18 | Setup scripts: install and launch from nothing | Linux PASS; macOS and Windows NOT RUN |
| TEST-039 | 2026-09-23 | One BNO086 over SPI on the bench wiring | PASS |
| TEST-040 | 2026-09-23 | BNO086 identity and the 086-only features | PASS (idle power not measured) |
| TEST-041 | 2026-10-01 | Pad states and JTAG eFuses on the chip | PASS |
| TEST-042 | 2026-10-02 | A device reboot discards the host's calibration record; ends the session (PROB-019) | PASS (unit tests and hardware) |
| TEST-043 | 2026-10-02 | Each BNO086 on the DEC-016 wiring | PASS (both sensors) |
| TEST-044 | 2026-10-02 | Pad readback on the assembled build | PARTIAL (GPIO42 anomaly, PROB-020) |
| TEST-045 | 2026-10-02 | Host tests for the BNO086 build (SH-2 codec, motor guard, schema 5) | PASS |
| TEST-046 | 2026-10-02 | Acquisition on the DEC-016 build, 60 s over USB | PASS after a fix (125 Hz frames) |
| TEST-047 | 2026-10-02 | Sensor check from the product firmware | PASS, both sensors |
| TEST-048 | 2026-10-02 | Dashboard backend recording from the BNO086 build | PASS |
| TEST-049 | 2026-10-02 | `ead --check` in the real app | PASS (all six motors felt at 50 %; cables not swapped) |
| TEST-050 | 2026-10-02 | Replay with the recording's own conversion; schema 7 migration | PASS |

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

**Correction (2026-10-04, TEST-059):** the first "heel strike", 6.95 s, was the
first step's push-off: it falls in the shank's backward dip before that step's
swing (peak at 7.27 s), and a right-foot landing cannot precede the right foot's
swing. The course has six landings and five cycles; the 0.36 m first cycle was
not a stride.

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
stated contents. (Correction 2026-10-02: seventeen. `protocol/vectors/` held 17
`.hex` files at this test's commit, as it does now; "eighteen" was a miscount.)

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

After those changes: 18 vectors, 0 failures (17; see the correction above). Spot-checked against the file
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

## TEST-038 — Setup scripts: install and launch from nothing

### Objective
That one command takes a machine with missing dependencies to a running
dashboard: check, install, check again, clone, install packages, start.

### Environment
This Linux machine (Ubuntu 24.04). No macOS or Windows machine, and no
container runtime, so the system-package step (`apt`, which needs sudo) could
not be exercised on a clean system.

### Procedure
`setup-linux.sh` run under `env -i` with `HOME` set to an empty scratch
directory and `PATH` reduced to `/usr/local/bin:/usr/bin:/bin`. That hides the
machine's own nvm Node and rustup Rust, leaving the distribution's Node 18.19.1
— which also exercises "found, but too old". Run three times.

### Actual
1. **First run.** Check: system libraries ok, `node >= 20 (found v18.19.1)`
   MISSING, `rust >= 1.92` MISSING. Installed Node 22.23.2 through nvm and Rust
   1.98.1 through rustup; the second check passed; cloned from GitHub; `npm ci`
   added 78 packages; `tauri dev` compiled 512 crates in 1 m 59 s and launched —
   then GTK refused to start. **Test-harness fault, not the script:** `env -i`
   passed `DISPLAY` but not `XAUTHORITY`, so X refused the connection.
2. **Second run**, with the display variables passed: everything reported ok,
   `git pull` said already up to date — and then Vite failed with `Port 1420 is
   already in use`, because a dashboard had just been started on this machine
   from the real repository. That process was left alone. **Script change:** a
   run while a dashboard is open now stops with "The dashboard is already running
   (port 1420 is in use)" instead of a port error.
3. **Third run**, port free, display passed: straight through to a running
   dashboard, alive after 2 m 38 s, no panic, no error in the log.

Also seen and fixed: with no controlling terminal, the confirmation prompt
printed `/dev/tty: No such device or address` before defaulting to yes.

The macOS script passes `bash -n`; the Windows script parses under PowerShell
7.4.6's parser. Neither has run on its platform.

### Result
Linux: PASS for the user-level installs and the launch. The `apt` install of
system libraries: not exercised. macOS and Windows: not run.

### Notes
- Rust must be at least 1.92 (krilla's minimum).
- nvm and rustup are allowed to add themselves to the user's shell startup file,
  so `node` and `cargo` work in new terminals too. A first attempt to stop nvm
  editing profiles (`PROFILE=/dev/null`) was reverted before commit for exactly
  that reason: installed but unusable in the next terminal is not installed.

## TEST-039 — One BNO086 over SPI: wiring, identity, orientation stream

### Objective

Before any product driver is written: prove the bench wiring of one BNO086, check
that the part answers as genuine CEVA/Hillcrest silicon, and stream orientation so
the sensor's tilt can be watched.

### Environment

- Seeed XIAO ESP32-S3, wiring as in `docs/hardware.md` "BNO086 bench test".
- `firmware/bench/bno086` (separate PlatformIO project; SparkFun BNO08x library
  1.0.6, which carries CEVA's SH-2 driver). Not the product firmware, which stays
  dependency-free.
- Host: `tools/bno_view.py`, USB CDC at /dev/ttyACM1.
- Sensor lying still on a desk, unmounted.

### Procedure

1. `pio run -d firmware/bench/bno086 -t upload`.
2. `python3 tools/bno_view.py --checks-only --log bench.csv` (the tool asks the
   firmware to re-send the checks, which run once at boot).
3. Let it stream for 30 s, then count samples and report types in the log.

### Expected

Every check PASS; a product ID response; ~100 Hz of rotation-vector samples;
|a| ≈ 9.81 m/s² and |ω| ≈ 0 while still.

### Actual

```
PASS int_idle_in_reset      INT high while RST low
INFO readback_miso          1/1        INFO readback_cs   1/0
INFO readback_sck           1/1        INFO readback_mosi 1/1
PASS rst_int                INT asserted 115 ms after reset released
PASS spi_init               SH-2 opened at 3 MHz, SPI mode 3
id   0,part=10004563,ver=3.12.6,build=62,reset_cause=4
id   1,part=10003606,ver=1.10.10,build=404,reset_cause=0
id   2,part=10004135,ver=5.7.17,build=319,reset_cause=0
id   3,part=10004149,ver=5.3.14,build=226,reset_cause=0
PASS wake                   INT answered the wake request in 1 ms
INFO features               arvr_stabilized_rv=1 gyro_integrated_rv=1
                            stability_classifier=1 magnetometer=1 tap_detector=1
PASS reports                rotation_vector=1 accel=1 gyro=1
PASS still                  |a|=9.752 m/s^2 over 249 accel reports,
                            |w|=0.0050 rad/s over 199 gyro reports
```

Stream: 2952 samples over 29 539 ms = 99.9 Hz, gyro components up to 0.025 rad/s
(desk vibration), rotation-vector accuracy field 180° throughout.

### Result

PASS.

### Notes

- Confirmed: the part runs CEVA SH-2 application firmware 3.12.6, answers the
  product ID request with four entries, accepts the whole SH-2 feature set, and
  fuses correctly (gravity within 0.6 % of 9.81 m/s², gyro at rest ≈ 0).
- Unknown: the datasheet publishes no table of SH-2 part numbers, so these values
  cannot *prove* the die is a BNO086 rather than another BNO08x. Nothing in the
  responses suggests a relabelled or counterfeit part, and 3.12.6 is well past the
  BNO080-era 3.2.x firmware.
- The 180° accuracy field is the uncalibrated magnetometer, not an error; the
  rotation vector is 9-axis. Tilt is correct, heading is unreferenced.
- `tools/bno_view.py` without `--checks-only` draws the tilting block; that view was
  exercised by the user, not in this session (no display here).

## TEST-040 — Is the part really a BNO086? The hardware identity and the 086-only features

### Objective

TEST-039 showed the part runs genuine CEVA SH-2 firmware, but not that the die is a
BNO086 rather than another BNO08x. The CEVA datasheet lists three things as BNO086
only: 14-bit accelerometer fusion, lower idle power, and Interactive Calibration
(the Motion Intent command and the Motion Request report). This tests what can be
reached over SPI.

### Environment

As TEST-039. `firmware/bench/bno086` gained `checkIdentity()` (SH-2 metadata, FRS
serial number, oscillator type) and `check086()` (the 086-only features).

### Procedure

1. Read each sensor's SH-2 metadata, the FRS serial-number record and the
   oscillator type.
2. Send Motion Intent (`sh2_setIZro(SH2_IZRO_MI_STATIONARY_NO_VIBRATION)`) and
   enable the Motion Request report.
3. With the calibrated accelerometer running, enable the raw accelerometer report
   for 2 s, take the greatest common divisor of the counts and their mean vector
   magnitude while the board is still, and work out the step size relative to the
   ±8 g full scale the metadata declares.

### Actual

```
meta_accel            vendor="Bosch Sensortec BMA280" range=20082 res=9 q=8
meta_gyro             vendor="Bosch Sensortec BMI055" range=17863 res=1 q=9
meta_mag              vendor="Bosch Sensortec BMM150" range=32000 res=5 q=4
meta_rotation_vector  vendor=""                       range=16384 res=1 q=14
serial_number         request succeeded, record empty (0 words)
oscillator            1 (external crystal)
izro_motion_intent    PASS  accepted
izro_motion_request   PASS  report enabled (no request arrived in 2 s)
accel_bits            PASS  1120.5 steps per g over +-8 g = 14.13 bits
                            (counts are multiples of 4: a 14-bit field left-aligned in 16)
still                 PASS  |a|=9.805 m/s^2, |w|=0.0004 rad/s
```

### Result

PASS.

### Notes

- Confirmed: the part declares Bosch BMA280 / BMI055 / BMM150 hardware, accepts the
  Motion Intent command and the Motion Request report — both listed as BNO086 only —
  and its accelerometer really carries ~14 bits (a 12-bit part would give about
  280 steps per g, i.e. 12.1 bits).
- Not testable here: lower idle power needs a current meter.
- No Motion Request arrived in the 2 s window. Expected: the hub asks for a
  stationary period only when it wants one, so this is not evidence either way.
- The FRS serial-number record is present but empty on this unit, so it cannot serve
  as identity evidence.
- Still open: nothing above rules out a BNO085 physically relabelled as a BNO086,
  because a relabel would still answer as the die it is — and this die answers as a
  BNO086. The remaining check is the package marking, by eye.
- Useful beyond identity: Interactive Calibration exists to remove gyro zero-rate
  offset more often than opportunistic calibration does, which is exactly the
  heading-drift problem in PROB-015. Worth considering for the product.

## TEST-041 — Pad states and JTAG eFuses on the actual chip

### Objective

DEC-016's pin map rests on the ESP32-S3 datasheet's pad states and on three
JTAG eFuses being at their factory value. Read them from the chip instead of
the datasheet.

### Environment

XIAO ESP32-S3 (ESP32-S3 QFN56, revision v0.2, MAC 44:b1:76:af:fb:7c), bare —
nothing wired to any of the pins tested. `firmware/bench/padstate`, USB.
2026-10-01.

### Procedure

1. Flash `firmware/bench/padstate`. Before user code touches anything, it
   snapshots the IO MUX register of each pad (pull-up, pull-down, input enable,
   drive strength, function) and reads the pad level, then reads the eFuses
   HARD_DIS_JTAG (the datasheet's EFUSE_DIS_PAD_JTAG), DIS_USB_JTAG and
   STRAP_JTAG_SEL.
2. Then read each pad with the internal pull-up and then the pull-down, to see
   what is attached.

### Expected

eFuses all 0. Pad settings as the datasheet's "After Reset" column (Table 2-1):
GPIO1, 2, 3, 9 input-enable only; GPIO4–8 nothing; GPIO39 input-enable plus a
weak pull-up; GPIO40–42 input-enable only; GPIO43/44 pull-up and input-enable.

### Actual

```
EFUSE HARD_DIS_JTAG=0  DIS_USB_JTAG=0  STRAP_JTAG_SEL=0

gpio  WPU WPD IE level   readback
1      0   0   1   0      1/0
2      0   0   1   0      1/0
3      0   0   1   0      1/0
4      0   0   0   0      1/0
5      0   0   0   0      1/0
6      0   0   0   0      1/0
7      0   0   0   0      1/0
8      0   0   0   0      1/0
9      0   0   1   0      1/0
39     0   0   1   1      1/0
40     0   0   1   0      1/0
41     0   0   1   0      1/0
42     0   0   1   0      1/0
43     1   0   1   1      (not disturbed: UART0)
44     1   0   1   1      (not disturbed: UART0)
```

### Result

PASS.

### Notes

- **Confirmed:** the three JTAG eFuses are at factory default. By datasheet
  Table 3-5, JTAG is routed to the USB Serial/JTAG controller, the four back
  pads are not connected to it, and GPIO3's strap value is ignored. The three
  conflict-check rows that depended on this are now measured, not assumed.
- **Confirmed:** every motor pin (GPIO1, 2, 4, 5, 6, 42) has neither pull-up nor
  pull-down; the chip-select pins (43, 44) have their pull-ups and sit high.
- **Observed, GPIO39:** the pull-up bit is already off when user code runs, yet
  the pin reads high. The readback shows nothing is attached to it (1/0). It is
  the only floating pad that reads high; its unpulled neighbours 40–42 read low.
  **Interpretation:** the weak pull-up the datasheet describes was on from reset
  and was switched off by startup code before `setup()`, and the floating pad
  kept the charge. That is the window — power-up to firmware — in which a motor
  gate on GPIO39 would be pulled on, so DEC-016's rule stands. Which startup step
  clears the bit was not identified.
- All pads report drive strength 2 and IO MUX function 0.

## TEST-042 — A device reboot discards the host's calibration record (PROB-018)

### Objective

The session gate must refuse to start when the device has rebooted since its last
calibration, and must not refuse after a reconnect to the same boot.

### Environment

Dashboard Rust backend, `cargo test`, Linux, 2026-10-02. No device: the only board
is wired to DEC-016 and cannot run the product firmware.

### Procedure

1. `device::tests::a_device_reboot_discards_the_calibration`: connect, HELLO with
   boot_id 0xA1B2C3D4, then the golden calibration record, then a new connection
   (new tracker, as `session()` makes one) and HELLO with boot_id 0x01020304.
2. `device::tests::a_reconnect_to_the_same_boot_keeps_the_calibration`: the same,
   but the second HELLO carries 0xA1B2C3D4 again.
3. Hardware (not run): flash the product firmware, calibrate, reset the board,
   attempt a session (must be blocked: "the device has not been calibrated since it
   started"), recalibrate (must unblock), unplug and replug USB without a reset
   (must stay unblocked).

### Expected

1. `snapshot().calibration` is `None`, which `session_blockers` turns into a blocker.
2. The usable record is still there.

### Actual

- Before the fix: test 1 failed (the record survived); test 2 passed.
- With the rejected fix (clearing in the tracker's boot_id branch): test 1 passed,
  test 2 failed.
- With the applied fix: both pass. Full suite: firmware native 63/63, `pio run`
  SUCCESS, `cargo test` 59 passed / 3 ignored, `cargo clippy --all-targets -- -D
  warnings` clean, `npm test` 28/28, `eadprobe vectors` 17 ok / 0 failures.

### Result

PARTIAL on 2026-10-02 morning (unit tests only); PASS once the hardware steps ran (below).

### Hardware steps (later on 2026-10-02)

`cargo test -- --ignored a_device_reset_ends_the_session_and_its_calibration`, product
firmware schema 5 on the DEC-016 build, USB, device still on the desk:
2 s calibration (usable) → recording started, frames stored → link closed and reopened
(same boot_id): calibration kept, still recording → link closed, board reset with
`esptool.py --chip esp32s3 --after hard_reset read_mac` → reconnected, new boot_id:
calibration `None` (the gate's "not calibrated since it started"), recording ended,
`ended_by_restart` = that session, `stopped_at` set, the notice in `last_error`
(PROB-019). PASS in 5.0 s.

## TEST-043 — Each BNO086 on the DEC-016 wiring (bench firmware, one at a time)

### Objective

First power-on check of the soldered DEC-016 build: each sensor and its cable
answers on the shared SPI bus, identifies as the same BNO086 seen in TEST-039/040,
and streams sensible data, with the motor gates held LOW throughout.

### Environment

The DEC-016 build (user, 2026-10-02: everything soldered). `firmware/bench/bno086`
at the commit that moved it to DEC-016, SPI 1 MHz mode 3, USB. Not yet run.

### Procedure

1. Board on USB. `pio run -d firmware/bench/bno086 -e foot -t upload`.
2. `python3 tools/bno_view.py --checks-only`; then without `--checks-only`, tilt the
   foot sensor and watch the block follow.
3. Repeat with `-e shank`.

### Expected

For each sensor, from TEST-039/040 on the bench part:
- `int_idle_in_reset` PASS; `other_rst_int` "high in reset, low after release".
- Readback (sensor in reset): MISO, SCK, MOSI 1/1 (the boards' I2C pull-ups on the
  shared pins); CS 1/0. 0/0 on any line is a fault.
- `rst_int` PASS, INT within about 112–115 ms of reset release.
- `spi_init` PASS at 1 MHz; product ID part 10004563 (version 3.12.6 on the bench
  part; the second part may differ).
- `still`: |a| about 9.8 m/s², |ω| near 0, held still.
- No motor runs at any point.

### Actual

Run 2026-10-02 over USB, master switch OFF, bench firmware with GPIO42 left
undriven (PROB-020).

| Check | Foot (CS 43, INT 39) | Shank (CS 44, INT 40) |
|---|---|---|
| `int_idle_in_reset` | PASS | PASS |
| Readback MISO / CS / SCK / MOSI | 1/1, 1/0, 1/1, 1/1 | 1/1, 1/0, 1/1, 1/1 |
| `other_rst_int` | other INT high in reset, low after release | same |
| `rst_int` | PASS, 112 ms | PASS, 112 ms |
| `spi_init` | PASS, 1 MHz, mode 3 | PASS, 1 MHz, mode 3 |
| Product ID | part 10004563, 3.12.6, build 62, reset cause 4 | same |
| `wake` | PASS, 1 ms | PASS, 1 ms |
| `reports`, `izro_*` | PASS | PASS |
| `accel_bits` | PASS, 1011.2 steps/g, 13.98 bits | PASS, 1024.3 steps/g, 14.00 bits |
| `still` | PASS, \|a\| 9.937 m/s², \|ω\| 0.0000 rad/s | PASS, \|a\| 9.813 m/s², \|ω\| 0.0000 rad/s |
| Stream | about 98 Hz | 99.8 Hz (998 lines over 9.994 s) |

### Result

PASS, both sensors.

### Notes

- Every one of the nine lines per sensor took part in a passing step: 3V3, GND and
  RST (the sensor boots on reset release), INT (idle high in reset, then asserted),
  SCK, MISO, MOSI and CS (an SH-2 request and its answer), WAKE (the 1 ms answer).
- Which physical sensor answers on which CS is not verified by this test: both
  were still. The mounting check on the leg confirms foot vs shank.
- Both runs print `CHK,reset,INFO,sensor reset itself` once, just after streaming
  starts, and stream normally afterwards. Root cause: unknown; most likely the
  library reporting the reset event of its own RST pulse late. Not investigated.
- Motor gates 1, 2, 4, 5, 6 were held LOW and GPIO42 left undriven throughout.

## TEST-044 — Pad readback on the assembled DEC-016 build (padstate's stored report)

### Objective

Learn what the wiring does to each pad before any firmware drives a pin on the
assembled build.

### Environment

The DEC-016 build, XIAO `44:B1:76:AF:FB:7C`, running `firmware/bench/padstate`
since 2026-10-01. USB, opened with DTR/RTS set before open (PROB-005). 2026-10-02.

### Procedure

1. Send `?` once. `padstate` re-sends the report it recorded at its last boot (each
   pad read with the internal pull-up, then the pull-down, 300 µs each). No reset,
   no flash. The boot time of that report, and whether the motor rails were powered
   then, are unknown.

### Expected

From DEC-016 and the PCB design: SCK, MISO, MOSI 1/1 (the boards' pull-ups);
RST and WAKE 1/1; INT lines driven by the sensors; each motor pin the same as the
others (100 Ω to a gate with a 100 kΩ pulldown, against a ~45 kΩ internal pull-up:
about 2.3 V, so the pull-up reading may come out 1).

### Actual

| GPIO | Function (DEC-016) | Readback | TEST-041 (bare chip) |
|---:|---|---|---|
| 1, 2, 4, 5, 6 | Motors 1, 2, 4, 5, 6 | 1/0 | 1/0 |
| 42 | Motor 3 | **0/0** | 1/0 |
| 3 | WAKE | 1/1 | 1/0 |
| 7, 8, 9 | SCK, MISO, MOSI | 1/1 | 1/0 |
| 39 | Foot INT | 0/0 | 1/0 |
| 40 | Shank INT | 1/1 | 1/0 |
| 41 | RST | 1/1 | 1/0 |
| 43, 44 | CS (not disturbed) | level 1, pull-up on | same |

eFuses unchanged: HARD_DIS_JTAG = DIS_USB_JTAG = STRAP_JTAG_SEL = 0.

### Result

PARTIAL — observations only.

### Notes

- **Confirmed:** the report was taken on the wired board (it differs from TEST-041).
- **Consistent with DEC-016:** SPI lines, RST and WAKE held high.
- **Foot INT held low:** most likely the powered foot sensor asserting INT with data
  nobody read; a short to GND reads the same. TEST-043 tells them apart.
- **Shank INT held high:** inconclusive; an unserviced BNO086 deasserts INT after a
  timeout, so one snapshot cannot say.
- **Motors 1, 2, 4, 5, 6 read 1/0:** consistent with the 100 kΩ pulldowns, but the
  same as nothing attached; this test cannot tell them apart.
- **GPIO42 held low:** see PROB-020.

## TEST-045 — Host tests for the BNO086 build

### Objective
The SH-2 codec, the motor limits and the schema 5 payloads, checked on the host before
any hardware.

### Environment
`pio test -e native`, `cargo test`, `eadprobe vectors`, Linux, 2026-10-02.

### Procedure
1. `test_sh2` (7): SHTP headers (valid, empty, undriven MISO 0xFF, bad channel, too long);
   Product ID request and Set Feature bytes; control replies with the product ID TEST-043
   read (part 10004563, 3.12.6, build 62) and feature responses; input reports with base
   timestamp, rebase, delay bits in the status byte, and a stop at an unknown report.
2. `test_motor_guard` (4): refusals (motor, disabled motor 3, duty 50/205, 99/5001 ms),
   one at a time, 5 s per motor in 10 s, and 100 alternating 100 ms pulses.
3. `test_protocol` (+2): SERVICE_TEST requests and replies against four new vectors.
4. Rust: format 2 and format 1 sections, SERVICE_TEST vectors, the service-test log.
5. Mutation check: the history shrunk from 128 to 64 entries must fail step 2's last case.

### Expected
All pass; the mutation fails.

### Actual
Native 76/76; Rust 62 passed, 3 ignored; vectors 22 decoded, 0 failures. With 64 entries,
`test_many_short_pulses_still_count` failed ("Expected 6 Was 0"); restored to 128.
Two hand-typed part-number bytes in the test were wrong at first (0xA6 for 0xA8); caught
before the first run by computing them.

### Result
PASS

## TEST-046 — Acquisition on the DEC-016 build, 60 s over USB

### Objective
TEST-018's measurement on the BNO086 build: lost frames, rate, period, |a| at rest.

### Environment
Product firmware 0.1.0+2340d44.dirty (schema 5), XIAO 44:B1:76:AF:FB:7C, USB, still on a
desk, master switch OFF. `eadprobe stats --seconds 60`. 2026-10-02.

### Expected
About 100 Hz, 0 missing, 0 dropped, 0 bus errors, |a| about 1 g.

### Actual
- First run: 7510 frames in 60 s, **125.104 Hz**, foot_repeated 1498, shank_repeated 1251.
  The accelerometer runs at 125 Hz (the part's nearest rate to the requested 10 ms) and
  the gyroscope at 100.2 Hz (6012 new samples in 60 s); frames were accelerometer-clocked.
  Fixed by clocking frames on the gyroscope.
- After the fix: 6010 of 6010 frames, 0 missing, 0 dropped, 0 bus errors, 0 rejected USB
  frames, **100.142 Hz**, period mean 9985.9 µs, sd 200.6 µs (min 9341, max 10607); foot
  |a| 1.0126 g (sd 0.0050), shank |a| 1.0023 g (sd 0.0041); foot_repeated 3,
  shank_repeated 116 (1.9 %).
- 20 s recording: periods in one peak around 10 000 µs; shank repeats in clusters about
  every 240 frames.
- After `check --rerun`: the check's own 539 ms gap (index +54), then one 152 ms pause
  between consecutive indices right after the first gyroscope report.

### Result
PASS after the fix.

### Notes
- The 200 µs period sd (MPU6500 build: 0.5 µs from the data-ready ISR) is consistent with
  real sample spread inside the BNO086; not established.
- Shank repeats: phase crossings of two independent clocks; flagged per frame.

## TEST-047 — Sensor check from the product firmware

### Objective
Our own driver's per-wire check on the assembled build, at boot and on request.

### Environment
As TEST-046. `eadprobe check`, `eadprobe check --rerun`.

### Expected
Every step PASS on both sensors (TEST-043 proved the wiring with the SparkFun driver).

### Actual
Boot and three re-runs: all seven steps PASS on foot and shank; part 10004563, version
3.12.6, build 62 on both; boot 114 ms; WAKE answered in 454–478 µs. Streaming resumed
after each re-run.

### Result
PASS

## TEST-048 — Dashboard backend recording from the BNO086 build

### Objective
The Rust device link and store against the real device bytes.

### Environment
`cargo test -- --ignored records_a_session_from_a_real_device`, USB.

### Procedure
The existing hardware test, updated: BNO086 configuration, identity maps, then the boot
sensor check decoded from the device (all steps, part number), then the recorded session.

### Actual
PASS in 5.7 s. (Before the update it asserted the shank map from before TEST-027's
correction, so it was already stale.)

### Result
PASS

## TEST-049 — `ead --check` in the real app

### Objective
The Check view end to end on the device.

### Environment
Release build (`tauri build --no-bundle`), launched as `ead --check` under X11
(`GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1`) so it could be driven with
`xdotool` and captured with `xwd`. USB.

### Actual
- Opens on Check; after connecting: device ready, sensors ok, Haptics "no feedback",
  every step PASS for both sensors, motors M1–M6 with GPIO 1, 2, 42, 4, 5, 6.
- "Run the check again" ran a check and logged it (`service_tests` row 1).
- M3 pulse: the device refused with "motor 3 stays off until its wiring is measured
  (PROB-020)"; nothing logged. The message first appeared at the page top, far from the
  button; moved into the Motors panel and re-verified.
- Synthetic clicks on scrolled content never reached WebKit in this setup (not seen with
  unscrolled content); the M3 press was made with Tab and Space instead.
- Not run by the agent: an accepted pulse felt by a person; the "which sensor is which"
  turn. Both were done by the user later the same day (below).

### User run (2026-10-02, 13:52–14:40 UTC, from the service-test log)
- All six motors pulsed for 1 s and marked **felt** at duty 128 (50 %): M1, M2, M4, M5,
  M6 (tests 11–15), M3 after PROB-020 (test 17). Earlier pulses at 75 % were left
  unanswered. The lowest perceivable strength was not tested.
- "Which sensor is which": moving the shank sensor changed the Shank column, and the
  foot sensor the Foot column (user): the cables are not swapped.
- Sensor check from the app: every step PASS on both sensors; boot 120 / 114 ms, WAKE
  454 / 469 µs (user's screenshot).

### Result
PASS

## TEST-050 — Replay with the recording's own conversion; schema 7 migration

### Procedure
1. `eadreplay recordings/walk6m-2026-09-18.eadlog --still-seconds 5` with and without
   `--mpu6500`.
2. A new 8 s BNO086 recording (`eadprobe stats --record`) replayed with no flag.
3. The schema 6 → 7 migration SQL on a copy of the dashboard database, then the real
   database opened by the new app after a backup.

### Actual
1. Before the fix the walk replayed to 0 valid cycles (BNO086 scale applied to MPU6500
   counts). After: with `--mpu6500`, 6 valid cycles, 6.39 m; without, refused.
2. "conversion from the recording: 2510.5024 counts/g"; calibration accepted.
3. Copy: 18 sessions, 258,280 frames, 257 cycles kept, all 18 sections read as format 1
   at 8192 LSB/g. Real database: schema 7, 18 sessions all format 1. Backup
   `ead.sqlite3.schema6-backup-2026-10-02` (schema 6, 18 sessions, 258,280 frames).

### Result
PASS

## TEST-051 — BNO086 mount maps measured on the leg

### Objective
Measure the chip → anatomical map of both BNO086 boards as worn (they were identity,
unmeasured, since the DEC-016 build).

### Environment
Product firmware 0.1.0+943fff3 (identity maps), USB. Session
`20261002-151307-5352` (ID `DEV-1`; the developer wearing the device, not a patient),
10,480 frames, 104.8 s. Foot board on the dorsum, shank board on the anterior shin,
motor band fitted.

### Procedure
1. The user, wearing the device, recorded: stand still 10 s, toe raises with a 2 s
   hold, seated knee extensions with a 2 s hold, stand still 10 s. More than three
   repetitions in places.
2. Raw frames (chip frame, since the maps were identity) read from the dashboard
   database: 1 s mean acceleration and peak rate per axis.

### Actual
- Standing (0–16 s, 95–104 s): foot (0.29, 0.62, 0.76) g, |a| 1.02 g, largest on chip
  +Z, board tilted about 42° on the instep. Shank (−0.12, 0.97, −0.21) g: up is chip +Y.
- Toe raise holds (e.g. 20–22 s): foot chip y 0.62 → 0.36 g, z 0.76 → 0.91 g. Chip −Y
  tilts up with the toes, so forward is chip −Y. Raise onsets peak on chip X negative
  (−19 to −28 °/s), lowerings positive (+21 to +46 °/s).
- Seated knee extension holds (53–54 s, 60–61 s): shank chip Z 0.92 g, the anterior face
  turned up, so forward is chip +Z. Extension onset peaks on chip X at −140 °/s.
- Y = Z × X gives chip +X (medial) for both boards. Both maps are proper rotations
  and match the user's description of the silkscreen axes (docs/hardware.md).
- Foot: `X = −chipY, Y = +chipX, Z = +chipZ`. Shank: `X = +chipZ, Y = +chipX, Z = +chipY`.
  With these, toe raise and knee extension both turn about anatomical −Y, as the
  mounting check expects.
- Outside the protocol: 64–90 s the shank rested partly extended; at 77 s a turn of the
  shank about its long axis (chip Y +160 °/s).
- A static calibration the user ran on this firmware was rejected "gravity was not
  upward": with identity maps the shank's up axis is chip Z, which reads −0.21 g, below
  `kCalibMinUpZ` (0.5). Expected. With the new foot map, up Z is 0.74 (42°), inside the
  60° limit.

- Flashed 0.1.0+9c91b7d over USB; `eadprobe config` reported both new maps.
- Dashboard mounting check on the leg (user, 2026-10-02): stand still PASS (foot
  −0.61 +0.29 +0.77 g, shank −0.22 −0.15 +0.96 g); toe raise PASS (peak −3 −31 −2 °/s,
  −31 °/s about Y); knee extension PASS (peak −1 −109 −32 °/s, −109 °/s about Y). The
  user was told to raise quickly and hold until the window closed: the slow raises of
  the recording peak at 19–28 °/s with the lowering faster, which would fail the
  check's 30 °/s minimum or its sign test.
- Static calibration accepted: foot tilt 42.1°, |a| 1.024 g, gyro σ 0.16 °/s; shank
  tilt 14.4°, |a| 0.996 g, gyro σ 0.51 °/s; 500 frames; gyro bias within ±0.12 °/s.

### Result
PASS

## TEST-052 — First 10 m walks on the BNO086 build

### Objective
Measure contacts, cycles and distance on the leg against a measured course and
counted steps.

### Environment
Firmware 0.1.0+9c91b7d (TEST-051 maps), battery, Wi-Fi, dashboard. Worn by the
developer (ID `DEV-1`). 10.0 m straight course, out and back per recording, 5 s
still at each end, calibration accepted before the walks.

### Procedure
1. The user recorded five sessions, each out and back with the right-foot
   landings counted per leg: `153957-590d`, `154232-3056`, `154336-5c28` normal
   pace (9 and 9), `154509-130f` slow (11 and 11), `154631-d7ed` fast (8 and 8).
2. `tools/session2eadlog.py` wrote each session as an .eadlog;
   `eadreplay --still-seconds 5` replayed it, with `--trace` and `--events` to
   follow the engine.

### Actual
- The device's own cycles: 17–20 per recording; totals 13–33 m; cycles of 4–29 m.
  The replay of the same firmware matches them to 0.01 m from the second cycle.
- Cause of the largest errors: PROB-023, fixed. After the fix, per leg (contacts,
  sum of valid cycles, median cycle):

| Walk | Leg 1 | Leg 2 | Stride from the count |
|---|---|---|---|
| normal 1 | 8, 7.76 m, 1.16 m | 9, 8.92 m, 1.10 m | 1.11–1.25 m |
| normal 2 | 9, 9.00 m, 1.04 m | 11 (2 turn steps), 15.35 m, 1.22 m | 1.11–1.25 m |
| normal 3 | 8, 13.00 m, 1.25 m | 8, 8.07 m, 1.16 m | 1.11–1.25 m |
| slow | 8, 4.92 m, 0.73 m | 9, 5.78 m, 0.65 m | 0.91–1.00 m |
| fast | 8, 15.27 m, 1.30 m | 8, 8.90 m, 1.24 m | 1.25–1.43 m |

  A leg's sum covers the strides between its first and last detected contact, not
  the whole 10 m. "Stride from the count" is 10 m over the counted landings, with
  or without the first and last as half strides.
- Remaining errors: PROB-024.

### Result
PARTIAL. Step timing and normal-pace stride length are close to the count; distance
per cycle is not yet reliable (PROB-024). The five recordings are in `recordings/`
as `walk10m-*-2026-10-02.eadlog`.

## TEST-053 — PROB-024 attempts replayed on the 10 m walks

### Objective
Measure each PROB-024 change against the counted landings and the 10 m course.

### Environment
Host replay (`tools/replay`) of the five `walk10m-*-2026-10-02` fixtures and
`walk6m-2026-09-18` (`--mpu6500`). `tools/replay/walks.py` prints per-leg results.

### Procedure
1. Build the replay, then `python3 tools/replay/walks.py /path/to/eadreplay` (add
   replay flags such as `--confirm 1.0` to sweep a threshold).
2. Compare with the same run at 77ae282.
3. Replay the 6 m walk: expect 6 valid cycles, 6.39 m.

### Actual
At 77ae282: contacts 86 of 92, 8 valid cycles without a zero-velocity update. With
DEC-019 (soft footfalls, 20 Hz event path):

| Walk | Leg 1 (contacts, metres) | Leg 2 | Valid / no ZUPT | Median cycle |
|---|---|---|---|---|
| normal1 | 9/9, 8.54 | 9/9, 8.78 | 16 / 0 | 1.09 m |
| normal2 | 9/9, 9.00 | 11/9 (turn steps), 15.18 | 18 / 2 | 1.10 m |
| normal3 | 9/9, 9.57 | 8/9, 8.01 | 15 / 1 | 1.23 m |
| slow | 10/11, 6.24 | 11/11, 6.16 | 18 / 1 | 0.69 m |
| fast | 8/8, 15.27 | 8/8, 8.78 | 15 / 1 | 1.24 m |

Contacts 92 of 92 counted. 6 m walk: 6 valid cycles, 6.39 m. Unit tests
`test_a_soft_landing_ending_a_swing_is_a_contact` and
`test_accel_jitter_in_foot_flat_does_not_block_zero_velocity` each fail with their
change removed. Rejected sweeps (confirm level, swing rate) and the measured
`--zupt-gyro` sweep are in PROB-024.

### Result
PASS for contact detection on these walks. Distance: PARTIAL (PROB-024: slow
strides short, five uncorrected cycles). Flashed as 0.1.0+156b186 on 2026-10-04.

## TEST-054 — BNO086 report-rate survey, both sensors, product driver

### Objective
Find the fastest rate both sensors deliver completely over the shared 1 MHz SPI
bus, with the accelerometer, gyroscope and game rotation vector all on (DEC-020:
"the rate that gives the most accuracy").

### Environment
DEC-016 build, both BNO086 on the harness, SPI 1 MHz. `firmware/bench/rates`
(new): the product driver (`firmware/src/bno086.cpp`, `ead/sh2`) reading both
sensors as the product does; each report counted for 3 s after a 0.5 s settle,
with gaps from the per-report sequence numbers, and the interval each hub
confirmed in its Get Feature Response.

### Procedure
1. `pio run -d firmware/bench/rates -t upload`; send any byte to the port to read
   the transcript.

### Actual
| Requested | Hub chose (accel / gyro / game RV) | Delivered foot (accel / gyro / RV) | Shank | Gaps, bad packets |
|---|---|---|---|---|
| 10000 µs | 8000 / 10000 / 10000 µs | 125.3 / 100.0 / 100.0 Hz | 127.7 / 99.7 / 99.7 Hz | 0, 0 |
| 5000 µs | 4000 / 5000 / 5000 µs | 250.3 / 200.3 / 200.3 Hz | 255.0 / 199.3 / 199.7 Hz | 0, 0 |
| 4000 µs | 4000 / 2500 / 2500 µs | 92.7 / 306.7 / 306.7 Hz | 104.3 / 302.7 / 302.7 Hz | 0, 0 |
| 2500 µs | 2000 / 2500 / 2500 µs | 57.0 / 301.7 / 302.0 Hz | 61.0 / 300.3 / 300.3 Hz | 0, 0 |
| 2000 µs | 2000 / 2000 / 2000 µs | 18.7 / 65.0 / 65.0 Hz | 19.7 / 69.3 / 69.3 Hz | 0, 0 |

Check flags 0x7f (all steps) on both sensors at the start.

### Result
PASS. 200 Hz gyroscope and game rotation vector with a 250 Hz accelerometer is
the fastest complete setting on this bus. From 400 Hz up the reports arrive at a
fraction of the confirmed rate with no sequence gaps: the hubs send fewer, not
lose them, which points at the bus (hypothesis; 3 MHz SPI would test it, after
the soak test wiring rule 5 requires). The accelerometer and gyroscope never share
a rate: the hub offers the accelerometer 125/250/500 Hz and the gyroscope and
fusion 100/200/400 Hz.

### Notes — failed first attempt
The same survey through the SparkFun library in `firmware/bench/bno086` read
0–415 Hz at random per report and interval, and the sensor reset itself after
it. That library keeps only the last report of a packet carrying several, so it
undercounts when three reports run together. Discarded; the bench file was
restored. Its metadata read is kept as evidence: accelerometer BMA280, minimum
period 2000 µs; gyroscope BMI055.

## TEST-055 — Schema-6 feed on the board: 200 Hz frames, native accelerometer

### Objective
Check DEC-021's feed on hardware: frame rate and completeness, the native
accelerometer stream, the frame-assembly flags, and the rotation vector's world
frame.

### Environment
Product firmware with schema 6 (uncommitted at the time), both sensors on the
desk, still, USB. `eadprobe stats` (schema 6 decoder), 20–40 s runs; a 30 s
recording analysed offline.

### Procedure
1. Flash; `eadprobe stats --seconds 20`.
2. Fix found in step 1 (below), flash, repeat; `--seconds 40`; then
   `--record` 30 s for the timing analysis.

### Actual
- First build: frames 4010 of 4010 at 200.279 Hz, 0 missing; accelerometer streams
  249.7 / 251.4 Hz with no sequence gaps. But 461 of 4010 frames (11.5 %)
  repeated a shank gyroscope sample and 331 (8.3 %) held a stale shank
  accelerometer value. Cause, from the code: a frame waiting for its
  accelerometer samples was forced out by the next foot gyroscope sample, which
  the task reads before the shank's packet already waiting.
- Fix: up to four frames wait; each takes the shank's gyroscope and both rotation
  vectors nearest its time from a short history, and the accelerometers
  interpolated from theirs; a frame is forced out only after three periods
  (15 ms). After: 4010 of 4010 frames, 200.273 Hz; `shank_repeated` 78 (1.9 %,
  the beat of two independent clocks, as at 100 Hz in TEST-046); accelerometer
  held 0 on both; rotation vector missing 0.
- Rotation vector world frame: the foot accelerometer carried into the world by
  the foot's own game rotation vector reads (−0.000, +0.000, +1.031) g. The
  BNO086's world is Z up, as `segmentOrientation` assumes.
- Accelerometer stream, in arrival order: 0 sequence gaps on both sensors over
  40 s (an earlier counter that sorted by time reported 512 "gaps": two
  timestamp inversions, each counted as 255).
- Timing (30 s recording): foot gyroscope frames fit one straight line against
  their index to a residual sd of 157 µs. The accelerometers do not: samples per
  second of host time range 244–251 (foot) and 243–256 (shank); 4.2 % (foot) and
  17.8 % (shank) of consecutive steps lie outside 4.0 ± 0.5 ms, from 2 µs to
  11 ms; 2 timestamp inversions of up to 136 µs in 40 s.

### Result
PASS for completeness, rate and frame assembly. The accelerometers' timing is
recorded as measured.

### Notes
Observed: the accelerometer's own rate varies by a few percent from second to
second while the gyroscope's does not. Hypothesis: the BMA280 inside the BNO086
runs from its own oscillator, not the hub's crystal (the hub reports an external
crystal, TEST-054 transcript). The steps of a few microseconds are not explained
by that and could be the hub's timestamping (unknown). Consequence for the design:
frames interpolate by time, never by sample count, and the native stream keeps
every sample's own time.

## TEST-056 — The 30 °/s ZUPT limit and the schema-6 replay on the 10 m walks

### Objective
Apply DEC-020 item 1 (ZUPT gyroscope limit 30 °/s) and check the replay, now
reading schema-6 recordings, still reproduces the older fixtures.

### Environment
Replay built from the working tree with `ead/feed.cpp`; `tools/replay/walks.py`.

### Procedure
1. Replay at 25 °/s (before the change): compare with TEST-053.
2. Change the limit; replay again; the 6 m walk with `--mpu6500`.

### Actual
- Step 1: identical to TEST-053 on all five walks, and 6 valid cycles, 6.39 m
  on the 6 m walk: the schema-6 reader, the time-based calibration window and
  the per-recording frame rate change nothing for 100 Hz recordings.
- Step 2, per leg (contacts / counted, metres):

| Walk | Leg 1 | Leg 2 | Valid / no ZUPT |
|---|---|---|---|
| normal1 | 9/9, 8.53 | 9/9, 8.73 | 16 / 0 |
| normal2 | 9/9, 8.92 | 11/9 (turn steps), 9.42 | 18 / 0 |
| normal3 | 9/9, 9.54 | 8/9, 8.07 | 15 / 1 |
| slow | 10/11, 6.26 | 10/11, 4.87 | 16 / 1 |
| fast | 8/8, 8.68 | 8/8, 8.64 | 15 / 0 |

  Contacts 91 of 92 (one slow landing fewer than at 25 °/s). 6 m walk: 6 valid
  cycles, 6.34 m.

### Result
PASS. Every normal and fast leg reads 8.5–9.5 m (10 m less the first step from
standing); the two 15 m legs of TEST-053 are gone. Slow strides still read short
(PROB-024).

## TEST-057 — Schema-6 dashboard against the board (USB)

### Objective
The dashboard's own hardware test on the committed schema-6 firmware.

### Environment
Firmware 0.1.0+523b49d, sensors on the desk, USB. Release dashboard rebuilt
(`npx tauri build --no-bundle`); the user's database backed up first as
`ead.sqlite3.schema7-backup-2026-10-04` (schema 7, 24 sessions): opening the new
dashboard migrates it to schema 8.

### Procedure
1. `cargo test records_a_session_from_a_real_device -- --ignored --nocapture`.

### Actual
Connected over USB (firmware 0.1.0+523b49d); a 5 s session stored 1000 frames, 0
missing, 0 rejected byte runs; 2528 accelerometer samples (506 per second, both
sensors); first foot sample |a| = 1.031 g.

### Result
PASS. `a_device_reset_ends_the_session_and_its_calibration` was not run: it needs an
accepted calibration, and with the measured mount maps the shank board lying on the
desk reads upside down (calibration needs the sensors worn). Wi-Fi at the schema-6 rate
(about 23 kB/s) is not yet tested.

## TEST-058 — Barefoot 10 m walks over Wi-Fi at 200 Hz, with video

### Objective
The first walks on the schema-6 feed (DEC-021) over Wi-Fi, barefoot, each with a video
for ground truth (DEC-020: video only, no second person): does the link carry 200 Hz,
and how do the detector's contacts and distances compare with the video and the count?

### Environment
Firmware 0.1.0+523b49d, measured mount maps, worn by the developer (`DEV-1`): foot
sensor on the dorsum with a heel loop, right foot bare, on carpet. Dashboard over the
device's Wi-Fi. Device calibration (Part A, over USB) accepted: foot tilt 35.4°, shank
12.9°. Phone at the start line, knee height, portrait, 1080p 60 fps, facing along the
course (from behind going out, from the front coming back).

### Procedure
1. Mounting check, calibration over USB; then battery on, USB out, Wi-Fi.
2. Per recording: Start recording, start the video, walk to the tape, stand ≥ 3 s,
   three right-heel stamps, stand ≥ 2 s, 10 m out, turn at the marker, 10 m back,
   stand ≥ 5 s, stop the video and the recording.
3. Recordings: normal ×3, slow ×2, fast ×1, a continuous walk of about 3 min
   (`recordings/*-2026-10-04.eadlog`; three retakes without a video were not kept).
4. Sync: video motion energy cross-correlated with gyro magnitude over the first 25 s,
   then checked against the heel stamps on contact sheets (fast 1, normal 1).
5. Replay each with `--still-from` (README); count shank swings (|gyro| peaks
   ≥ 150 °/s, 9-sample mean) and match each to the first contact within 0.8 s.

### Actual
- **Link:** 10 sessions, 128,630 frames, 0 missing (frame count equals the index span
  in every session). Accelerometer: no sequence gaps; 230–238 samples/s per sensor,
  median interval 4.0 ms, 99th percentile 10–12 ms, longest 16 ms.
- **Sync:** video = data − 2.6 to −3.1 s for six recordings, + 9.1 s for normal 1
  (camera started first). Fast 1 by the stamps: landings at 9.10, 9.81, 10.45 s in the
  video, 11.86, 12.53, 13.16 s in the data (−2.73 s). Motion correlation alone gave
  +11.3 s for fast 1: wrong, and not used.
- **Counts:** the walker's count per leg (normal 8, slow 9–10, fast 7) equals the shank
  swing count on every leg checked (normal 2 and 3: 8 and 8; slow 1: 10 and 9).
- **Detector** (current `gait.cpp`): 272 contacts against 254 swings over all seven;
  40 contacts with no swing in the 0.8 s before them, mostly a second contact about
  1.0 s after a swing, splitting a stride in two (normal 1: 20.30, 45.93, 48.60 s).
  The matching is a heuristic: in fast 1 contacts fall before the swing peak and
  count as unmatched.
- **Distance per 10 m leg** (sum of valid cycles, so 10 m less the first step):
  normal 8.66/9.61, 8.55/9.10, 9.34/9.18; slow 9.53/10.25, slow 2 back 9.43.
  Slow 2 out and fast 1 have no stand between legs, so their legs merge.
- **Fast 1 return leg:** contacts every 0.5–0.6 s and one 5.04 m cycle (34.32 s). The
  video (26.5–36 s) shows a normal walk with both sensors and the heel loop in place,
  and the stop on the tape.

### Result
PASS for the link at 200 Hz over Wi-Fi. FAIL for contact detection: split strides
and a broken fast return leg, with the recordings themselves sound (PROB-024).

### Notes
Slow legs no longer read 30 % short (TEST-052: 0.65–0.73 m median cycle; here
1.03–1.18 m). Observed, not explained: schema 6 changed the rate, the orientation
source and the footwear at once.

## TEST-059 — Shank-swing step detection on the recorded walks

### Objective
DEC-022 against every fixture with ground truth, beside the foot-impact engine
it replaces.

### Environment
Host replay (`tools/replay/main.cpp`), the eleven 10 m fixtures (2026-10-02 and
2026-10-04), the 3-minute walk and the 6 m MPU6500 course.

### Procedure
1. Survey the shank's forward swing rate: per-peak height and the time from its
   downward zero crossing to the shank minimum and to the foot's impact peak.
2. `python3 tools/replay/walks.py /tmp/eadreplay`, before (e3d4c01) and after.
3. Sweep `--mid-swing` 50–100.
4. The new gait tests against both engines.

### Actual
- Peaks: strides 140–290 °/s; walk-in, turn shuffles and a few first steps 40–100.
  Zero crossing to impact 35–60 ms median (p90 54–122 ms); to the shank minimum
  40–149 ms median, p90 up to 190 ms.
- Contacts against the counts, sum of |found − counted| over 176 landings:
  foot-impact engine 21, shank engine 3 (2026-10-02 slow 10/11 twice; 10-04 slow 2
  back 9 against a swing count of 8, the walker said 9–10). Every 2026-10-04 normal
  and slow leg matches. Fast 1 (out, turn, back): 15 contacts (was 22), 18.98 m
  (was 23.27 with a 5.04 m cycle); no valid cycle over 3 m.
- `--mid-swing`: 100 → 3, 90/80/70 → 5, 60 → 7, 50 → 8 (lower thresholds count the
  shuffles the walker did not count).
- Per leg, 10 m less the first step: 8.47–9.66 m on all normal, fast and
  2026-10-04 slow legs; 2026-10-02 slow 6.19 and 7.02 m (unchanged, PROB-024).
- 3-minute walk: every leg 8–9 contacts, 8.8–10.0 m; 92 contacts (was 108).
- Stance ratio median 0.54–0.59 (was 0.48–0.50).
- 6 m course: contacts at 7.51, 9.17, 10.94, 12.74, 14.33, 15.92 s, 5 cycles,
  5.83 m. The old engine's extra contact at 6.95 s sits in the backward shank dip
  before the first swing (peak 7.27 s): push-off, not a heel strike.
- Gait tests: 13 pass on the new engine; on the old one 3 fail, among them the
  split-stride case from TEST-058.

### Result
PASS. Step counting and timing on the fixtures; on the device it is unflashed
and untested.

### Notes
All ground truth is one healthy wearer. Distance still comes from the same ZUPT
integrator; the 2026-10-02 slow shortfall is a distance problem, not detection.

## TEST-060 — Haptic feedback on the host

### Objective
DEC-023 end to end without hardware: engine, guard, protocol, store and export.

### Environment
Native tests (`pio test -d firmware -e native`), the dashboard's `cargo test`, the
regenerated vectors (`protocol/vectors`, schema 7) and `eadprobe vectors`.

### Procedure
1. `test_haptics`: gating, hysteresis, low confidence, invalid step, medial/lateral
   pairs, OVERALL direction, TIMING alternation, intensity, no direction, external stop.
2. `test_motor_guard`: two-motor cues within each motor's limits, Busy, refusals, the
   rolling limit counted per motor.
3. `test_protocol`: STATUS 59 bytes, CONFIG_SET decode and echo, HAPTIC_BATCH against
   `haptic_batch.hex`.
4. Dashboard: the vectors decode; a HAPTIC_BATCH counts for gap detection and reaches
   the sink; an export writes the records, stores a backfilled copy once, and counts
   episodes in `metadata.json`; schema 2 and 7 stores upgrade to 10.

### Expected
All pass; a test that inverts a doc 06 rule fails.

### Actual
Native 98 of 98 (9 haptic, 1 new guard, 2 new protocol). Dashboard 73 passed, 4
ignored (hardware). Vectors: 0 failures in eadprobe. Firmware builds (RAM 23.0 %,
flash 22.0 %). One test was wrong on first run: the rolling-limit case expected a
refusal at exactly 5.0 s on in 10 s, which the contract allows; corrected to 5.25 s.

### Result
PASS on the host. Not flashed; no motor has run from a cue; nothing felt.

### Notes
Bench next: `eadprobe haptics on`, a check with a reference, then an evaluation with
`--haptics`, motors watched and felt; then worn, with the dashboard's switch.

## TEST-061 — Shank swing axis against the anatomical Y axis

### Objective
Evidence for or against a per-session functional calibration (DEC-020 6d): how far
the shank's swing axis sits from anatomical Y after mount map and static alignment.

### Environment
Host replay traces (aligned shank gyro) of the 2026-10-04 walks, two 2026-10-02 walks
and the 6 m MPU6500 course.

### Procedure
Principal axis of the shank gyro over samples above 100 °/s; its angle from +Y, split
into a heading part (toward X) and a part toward Z.

### Actual
2026-10-04, barefoot: 8.9–13.6° off Y, heading −3.9 to −6.7°, toward Z −6.8 to −12.1°;
the axis carries 68–75 % of the swing energy. 2026-10-02, shod: 6.9 and 8.7° off Y,
heading −0.4 and −1.9°. 6 m course (other build): 14.5°, heading +12.9°.

### Result
Measured. Detection: unaffected (cos 10° = 0.985). Ankle angles: a 6° heading puts
about 10 % of the sagittal angle into the frontal one, about 1–1.5° of inversion at
peak dorsiflexion; the heading moved by about 5° between the two days.

### Notes
The functional calibration is deferred until a reference shows inversion spreads small
enough for that to raise false INVERSION or EVERSION classes; no trustworthy reference
exists yet. Part of the tilt toward Z may be anatomical (knee axis, tibial rotation),
not mounting.

## TEST-062 — Schema 7 on the board (USB)

### Objective
The DEC-022/DEC-023 firmware on the device, before any walking or felt vibration.

### Environment
Firmware 0.1.0+ec73cec flashed over USB, sensors on the desk, battery switch not used.
Release dashboard rebuilt; the user's database backed up first as
`ead.sqlite3.schema9-backup-2026-10-04` (it migrates to schema 10 on the next open).

### Procedure
1. `eadprobe hello`; `eadprobe stats --seconds 10`; `eadprobe haptics on`, then `off`.
2. `cargo test records_a_session_from_a_real_device -- --ignored --nocapture`.

### Actual
1. Schema 7, both sensors answered (0x86), capabilities haptics_fitted, psram_ring,
   motor_service_test. 10 s: 2010 of 2010 frames, 200.256 Hz, 0 missing, 0 dropped, no
   faults; accelerometers 247.5 and 249.4 Hz, no sequence gaps. STATUS `haptics` empty
   (switch off at boot). CONFIG_SET echoed 1, then 0.
2. First run failed on a stale assertion that the configuration's `haptics.fitted` is
   false (missed earlier because the test needs the board); corrected, then: 1000
   frames stored, 0 missing, 502 accelerometer samples/s, haptics fitted in HELLO and
   in the configuration, switch off.

### Result
PASS. No cue has run: cues need scored steps in an evaluation.

## TEST-063 — Audit fixes: feedback faults, contact angle, recalibration (host)

### Objective
Regression tests for PROB-026 and PROB-028; build check for PROB-027.

### Environment
Host, `pio test -d firmware -e native`; firmware build for the XIAO ESP32-S3.

### Procedure
1. `test_a_sensor_fault_of_any_kind_stops_feedback` (test_feed): each fault kind,
   including a device fault on an otherwise clean frame, stops feedback; saturation,
   held and repeated samples do not.
2. `test_the_contact_angle_is_the_one_at_the_impact` (test_gait): landings 20°
   plantarflexed, flat by the time the contact is decided.
3. The whole suite.

### Expected
Both pass; test 2 fails on the previous engine.

### Actual
Both pass (100 native tests). Test 2 on the previous `gait.cpp`/`gait.h`: "Expected
-20 Was 0". Whole suite: cargo 73 (4 ignored), clippy, npm test and build, 26 vectors.

### Result
PASS on the host. Not run on hardware.

## TEST-064 — Schema 8 and the audit fixes on the board (USB)

### Objective
Flash `29c560c` (PROB-026–033) and check what can be checked over USB, battery off.

### Environment
XIAO ESP32-S3 on USB, both BNO086 on the desk, battery switch off; `eadprobe`, the
hardware tests, a probe script for the ACK.

### Procedure and actual
1. `pio run -d firmware -t upload`; `eadprobe hello`: schema 8, `0.1.0+29c560c`,
   haptics fitted.
2. `eadprobe stats --seconds 20`: 200.29 Hz, 0 frames dropped, 0 faults, `foot_rv_missing`
   and `shank_rv_missing` 0: the 50 ms age limit on rotation vectors (PROB-026) does
   not trip on healthy sensors.
3. SESSION_START capture → ACK {cmd 2, type 0x04, kind 2}; STATUS `session` capture.
   SESSION_STOP → ACK {kind 2}, then ERROR Rejected (fewer than 30 cycles); STATUS
   `session` none. A second stop → ACK {kind 0}.
4. `cargo test hardware -- --ignored --test-threads=1`: `records_a_session_from_a_real_
   device` PASS. `a_device_reset_ends_the_session_and_its_calibration` FAIL: its three
   calibrations were each started and acknowledged, then rejected "gravity was not
   upward". `eadprobe calibrate`: foot tilt 147°, the foot board was lying upside down
   on the desk. Placement, not code; to be rerun with the boards upright.
5. Both gyroscopes read exactly 0 at rest, as recorded in TEST-043/046 (the part's own
   zero-rate removal); not a fault.
6. The release dashboard rebuilt (`npx tauri build --no-bundle`), schema 8.

### Result
PARTIAL. ACK and STATUS `session` verified on the board; the reset test, the
recalibration check (PROB-027) and anything with motors are still to do.
