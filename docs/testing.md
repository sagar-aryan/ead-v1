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
PARTIAL. Step 1 enumeration verified (TEST-010). Steps 2–5 not run; no motor
drivers are fitted (DEC-006), so motor steps are deferred.

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
