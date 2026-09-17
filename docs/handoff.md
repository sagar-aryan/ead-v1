# Project Handoff

## Project
EAD V1: a right-leg, barefoot, wearable error-augmentation system for stroke
gait. Hardware: XIAO ESP32-S3, two IMUs (foot + shank); a haptic band is
specified but not fitted. The desktop dashboard is Tauri 2.
Contract: `ead_agent_docs_v2/` (authoritative, read-only).

## Current Objective
Build the real device data path and a fully functional research dashboard, in
verified milestones. The user asked for genuinely engineered work: no filler, no
placeholder or fake data, no features the spec doesn't need.

## Current State (2026-09-17, after M0)
- Firmware is bring-up code.
  - Both IMUs (MPU6500 silicon) are configured, and the configuration is
    verified by readback, including the accel filter (PROB-003).
  - Mount maps are proper rotations enforced at compile time (PROB-002).
  - Motor pins are LOW first thing at boot.
  - Output is 10 Hz text over USB. There is no protocol, Wi-Fi, calibration or
    orientation yet.
- Dashboard: the Tauri Rust shell builds, with icons, capabilities and a strict CSP.
  The React UI is still the old placeholder skeleton.
- Docs are current as of M0.
- On-body gravity check (TEST-014) is pending: it needs the user to wear the
  device and stand still.

## Architecture
See `docs/architecture.md`. In one line: data-ready-driven 100 Hz acquisition →
portable C++ processing (`lib/ead_core`) → PSRAM message ring → Wi-Fi WebSocket
and USB links → Rust backend with SQLite → React views and exports.

## Important Files
| File | Purpose |
|---|---|
| `ead_agent_docs_v2/` | Contract. Never modify |
| `docs/hardware.md` | As-built hardware, mount maps, what is verified |
| `docs/decisions.md` | DEC-001–DEC-012 |
| `docs/problems.md` | PROB-001–PROB-004 |
| `docs/testing.md` | Executed tests with measured results |
| `firmware/include/config_v1.h` | Fixed V1 constants and mount maps |
| `firmware/src/main.cpp` | Bring-up firmware (replaced in M1) |
| `firmware/lib/ead_codec/` | WS header, EAD1 block header, CRC32 (not yet linked) |
| `dashboard/src-tauri/` | Tauri shell (`main.rs`, `tauri.conf.json`, `capabilities/`, `icons/`) |
| `dashboard/app-icon.svg` | Icon source; regenerate with `npx tauri icon app-icon.svg` and keep only the desktop sizes |
| `tools/orient_viewer.py` | Bring-up orientation viewer; retired in M1 |
| `.claude/skills/` | Installed agent skills: `frontend-design`, `vercel-react-best-practices` |

## Completed Work
- Phase 1 skeleton and Phase 2 bring-up (see `docs/progress.md`).
- Full audit and V1 build plan (2026-09-17).
- M0 baseline fixes and cleanup (2026-09-17).

## Current Work
M0 is complete except TEST-014. M1 is next.

## Milestone Plan
Every milestone ends with its verification actually run, measured numbers in
`docs/testing.md`, docs updated, and a local commit. Push only when the user asks.

1. **M1 — Acquisition, protocol, links.**
   - Build:
     - Interrupt-count test on GPIO7/8 first; if INT is not wired, stop and ask
       the user.
     - `lib/ead_core` with codec, COBS, CRC32 and message ring, plus native Unity
       tests and golden vectors in `protocol/vectors/`.
     - IRAM data-ready acquisition with µs timestamps and self-test.
     - HELLO / STATUS / RAW_SAMPLE_BATCH / CONFIG_GET / ERROR / BACKFILL over Wi-Fi
       (`esp_http_server`) and USB (COBS).
     - `docs/protocol.md`.
     - `tools/eadprobe.py`: an independent decoder with stats and recording.
     - `tools/check_config.py`.
   - Verify:
     - `pio test -e native`.
     - 30-minute zero-gap runs over USB and Wi-Fi.
     - The user runs the Wi-Fi dropout and backfill tests offline.
2. **M2 — Dashboard foundation.**
   - Build:
     - Rust protocol, link, device manager, SQLite store and 20 Hz live channel.
     - Shell with the device-state bar, device drawer, LIVE raw charts and health.
     - Patients, recording sessions, SESSIONS tree, RAW view.
   - Verify: 60-minute recording is exact; backfill works; RAW query < 100 ms.
   - **Stop for the user's UI review.**
3. **M3 — Calibration and orientation.**
   - Build:
     - Gyro bias + gravity alignment.
     - Mahony, relative orientation, Canvas2D orientation view.
     - Guided mounting check; host replay tool.
   - Verify: host replay bit-exact with the device; doc 13 §2 checks.
   - The user records walking datasets on a measured course.
4. **M4 — Gait and ZUPT.**
   - Build:
     - Gait state machine and IC/TO/foot-flat events with adaptive thresholds.
     - ZUPT error-state Kalman filter and cycle features.
     - EVENTS/CYCLES/TRENDS views.
   - Verify: stride count ±1; distance ±5 %; cadence against a metronome.
5. **M5 — Reference and error engine.**
   - Build:
     - Device-built reference (≥30 cycles); dashboard versioning and locking.
     - 10-cycle reference check.
     - Doc 06 error score, classes and confidence.
     - Segmentation; REFERENCES/SESSIONS workflow.
   - Undefined analysis values become DECs reviewed with the user.
6. **M6 — Exports.**
   - CSV package, `metadata.json`, Level-5 `session.mat` (checked with scipy),
     PDF report (krilla).
7. **M7 — On-device flash storage and recovery.** Needs a storage DEC first
   (1.5 MB data partition).

## Known Problems
- PROB-002: shank map fixed in firmware; on-body confirmation pending (TEST-014).
- PROB-004: GPIO43 may toggle during ROM boot. Unverified; matters only once
  motor drivers are fitted.
- The ±4 g / ±500 °/s ranges may clip during walking. Measure in M3 before
  proposing any change.

## Failed Approaches
- The shank map derived by assuming a left-handed anatomical frame (PROB-002
  Attempt 1). Never accept a mount map with determinant −1.
- Gating IMU init on WHO_AM_I 0x68 only (PROB-001). Clone boards carry MPU6500.

## Important Decisions
- DEC-005 USB fallback link.
- DEC-006 no haptic code.
- DEC-007 raw = chip-frame counts.
- DEC-008 session kinds.
- DEC-009 per-sensor mount maps.
- DEC-010 `esp_http_server`.
- DEC-011 dashboard stack.
- DEC-012 device-built reference.

## Environment
- Linux (Ubuntu 24.04), node 24.13.1, npm 11.8.0, rustc/cargo 1.95.0.
- PlatformIO 6.2.0 with `espressif32 @ 7.1.3` (Arduino-ESP32 2.0.17).
- Python 3.12.3 with pyserial 3.5, numpy 1.26.4, scipy 1.11.4.
- WebKitGTK 4.1 and libudev are installed. The user is in the `dialout` and
  `plugdev` groups.
- The device enumerates as `303a:1001` on `/dev/ttyACM0`.
- **The laptop has Wi-Fi only.** Joining the device AP drops internet. Tests the
  agent runs use USB; Wi-Fi tests are run by the user.

## How To Run
- Firmware build: `cd firmware && pio run`.
- Flash: `cd firmware && pio run -t upload`.
- Serial (bring-up text): `pio device monitor -b 115200`. Send `c` for register
  diagnostics.
- Orientation viewer (bring-up only): `python3 tools/orient_viewer.py`.
- Dashboard backend: `cd dashboard/src-tauri && cargo build`.
- Dashboard frontend: `cd dashboard && npm install && npm run build`.
- Dashboard app: `cd dashboard && npx tauri dev`. Not yet exercised; the UI is a
  placeholder until M2.

## How To Verify
- Builds: the three commands above succeed with no warnings from project code.
- Device:
  - The boot log shows "Foot OK  Shank OK".
  - `c` shows `ACCEL2=0x03` on both sensors.
  - Standing still, both sensors read anatomical a ≈ (0, 0, +1) g.

## Next Steps
1. TEST-014: the user wears the device and stands still; capture anatomical gravity.
2. M1 step 1: data-ready interrupt-count test on GPIO7/8.
3. Continue M1 as above.

## Warnings
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- No battery, switch, BLE, FSR, BNO086 or haptic code (contract + DEC-006).
- Keep motor GPIOs LOW. Driving them has no effect today, but PROB-004 and
  future drivers make any change safety-relevant.
- The root file `wide_infographic_diagram_on_a_white_background_sho.png` is an
  untracked, byte-identical copy of the contract placement image.
- Record a test as passed only after running it.
