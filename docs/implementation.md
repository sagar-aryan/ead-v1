# Implementation

What exists in the code today. Target design: `docs/architecture.md`.
Milestone plan: `docs/handoff.md`.

## Status by milestone

| Milestone | Scope | Status |
|---|---|---|
| M0 | Baseline fixes and cleanup | Complete except on-body gravity check (TEST-014) |
| M1 | Data-ready acquisition, protocol, Wi-Fi + USB links, backfill ring | Not started |
| M2 | Dashboard foundation: backend, shell, LIVE, recording, RAW, SESSIONS | Not started |
| M3 | Calibration, Mahony orientation, mounting check, datasets | Not started |
| M4 | Gait events + ZUPT, EVENTS/CYCLES/TRENDS | Not started |
| M5 | Reference, error engine, session workflow | Not started |
| M6 | CSV, `.mat`, PDF exports | Not started |
| M7 | On-device flash storage and recovery | Not planned in detail (needs a DEC) |

## Sensor mount maps

### Objective
Express both IMUs in the doc 04 anatomical frame.

### Design
`anat = M · chip`, one signed-permutation matrix per sensor, applied identically
to accel and gyro. Both frames are right-handed, so `M` must be a proper rotation
(DEC-009, PROB-002).

### Implementation
`firmware/include/config_v1.h`:
- `EadMountMap`, `kEadFootMount` (identity), `kEadShankMount`.
- `eadMountDet()` and `eadMountIsSignedPermutation()` are `constexpr` and
  enforced with `static_assert`, so an invalid map is a build error.
- `eadMountApply()` writes `int32_t` so negating a raw −32768 cannot overflow.

### Verification
TEST-009 (negative and positive compile tests). On-body check TEST-014 pending.

## IMU initialisation (bring-up firmware)

### Objective
Configure both sensors to the doc 00 values and prove the configuration took effect.

### Implementation
`firmware/src/main.cpp`, `mpuInit()`:
- Accepts WHO_AM_I 0x68 (MPU6050) and 0x70 (MPU6500).
- Writes PWR_MGMT_1, SMPLRT_DIV, CONFIG, GYRO_CONFIG, ACCEL_CONFIG, INT_PIN_CFG,
  INT_ENABLE, and ACCEL_CONFIG2 when the part is an MPU6500 (PROB-003).
- Reads every register back; any mismatch fails init and prints the register,
  value read and value expected.

### Boot order
`setup()` drives the six motor GPIOs LOW before starting USB serial, then I²C,
scan and init. There is no PWM or haptic code (DEC-006).

### Edge cases
ACCEL_CONFIG2 is compared through a 0x0F mask; the upper bits are reserved.

### Limitations
This remains bring-up firmware until M1:
- `millis()`-polled reads with no timestamps or frame index;
- 10 Hz text output for `tools/orient_viewer.py`;
- data-ready interrupts not used;
- no protocol, links, calibration or orientation.
`lib/ead_codec` (WS header, EAD1 block header, CRC32) exists but is not yet
linked into the application.

### Verification
TEST-008, TEST-010, TEST-011.

## Dashboard shell

### Implementation
- `dashboard/src-tauri/src/main.rs` is a bare Tauri builder; no commands yet.
- `tauri.conf.json`: strict CSP (`default-src 'self'`, IPC allowed); bundle icons
  generated from `dashboard/app-icon.svg` with `tauri icon`, keeping only desktop
  sizes.
- `capabilities/default.json` grants `core:default` to the `main` window.
- The React frontend (`src/App.tsx`, `src/types.ts`) is still the skeleton with
  placeholder data; M2 replaces it entirely.

### Verification
TEST-012, TEST-013. The app window has not been launched yet; first launch is part of M2.
