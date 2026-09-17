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
