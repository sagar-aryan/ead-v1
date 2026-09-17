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

## Current State (2026-09-17, after M1 and most of M2)
- **Firmware streams real data.** 100 Hz acquisition clocked by the foot IMU's
  data-ready interrupt, every frame timestamped and indexed. The doc-08 binary
  protocol runs over both the Wi-Fi access point and USB; roughly 12 minutes of
  telemetry is held in PSRAM so a host can backfill anything it missed.
  Verified by a 30-minute run with zero missing, dropped or corrupted frames
  (TEST-018).
- **Dashboard connects, shows and records.** The Tauri app detects the device by
  USB ID, performs the handshake, verifies the device's configuration against the
  hash the device reports, shows live sensor traces, and records sessions into
  SQLite with every raw sample preserved (TEST-022, TEST-023).
- **Recordings can be read back.** The raw view draws a whole session as a
  min/max envelope per bucket, in anatomical physical units taken from the
  configuration stored with that session (TEST-025).
- **Not yet built:** calibration, orientation, gait analysis, the error engine
  and exports (M3–M6). The device answers SESSION_* commands with NotSupported.
- On-body gravity check (TEST-014) is still pending: it needs the user to wear
  the device and stand still.
- The Wi-Fi link is verified as far as this machine can go: the access point
  advertises correctly (TEST-019) and the WebSocket code path is shared with USB,
  but an end-to-end Wi-Fi session has not been run (see Environment).

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
| `docs/problems.md` | PROB-001–PROB-007 |
| `docs/testing.md` | Executed tests with measured results |
| `firmware/include/config_v1.h` | Fixed V1 constants and mount maps |
| `docs/protocol.md` | Wire protocol: payloads, framing, backfill, enumerations |
| `docs/clinical_requirements.md` | The researcher's four requirements vs what is built |
| `firmware/lib/ead_core/` | Portable codec, COBS, CRC-32, message ring, config section |
| `firmware/src/acquisition.cpp` | Data-ready interrupt, self-test, frame assembly |
| `firmware/src/link.cpp` | Per-link protocol endpoint (replies, streaming, backfill) |
| `protocol/vectors/` | Golden vectors; `generate.py` rebuilds them |
| `tools/eadprobe.py` | Independent host decoder: `hello`, `config`, `stats`, `reopen` |
| `dashboard/src-tauri/src/` | protocol, link, device, store, live, app |
| `dashboard/src/` | React UI: api, useDevice, components, views |
| `dashboard/src-tauri/` | Tauri shell (`main.rs`, `tauri.conf.json`, `capabilities/`, `icons/`) |
| `dashboard/app-icon.svg` | Icon source; regenerate with `npx tauri icon app-icon.svg` and keep only the desktop sizes |
| `.claude/skills/` | Installed agent skills: `frontend-design`, `vercel-react-best-practices` |

## Completed Work
- Phase 1 skeleton and Phase 2 bring-up (see `docs/progress.md`).
- Full audit and V1 build plan (2026-09-17).
- M0 baseline fixes and cleanup (2026-09-17).
- M1 firmware acquisition, protocol and both links (2026-09-17).
- M2 dashboard foundation including the raw view (2026-09-17).

## Current Work
M2 is complete. Next is M3: the guided mounting check (which settles PROB-002),
then calibration and orientation.

## Milestone Plan
Every milestone ends with its verification actually run, measured numbers in
`docs/testing.md`, docs updated, and a local commit. Push only when the user asks.

1. **M3 — Mounting check, calibration and orientation.**
   - Build, in this order so the open risk closes first:
     - Guided mounting check (stand still, raise toes, extend the knee) giving a
       pass/fail per axis. This settles PROB-002 without asking the user to read
       numbers aloud.
     - 5 s static calibration: gyro bias and gravity alignment.
     - Mahony orientation and relative foot/shank orientation; orientation view.
     - Host replay tool.
   - Verify: host replay reproduces device quaternions bit-exactly; doc 13 §2.
   - The user records walking datasets on a measured course.
3. **M4 — Gait and ZUPT.**
   - Build:
     - Gait state machine and IC/TO/foot-flat events with adaptive thresholds.
     - ZUPT error-state Kalman filter and cycle features.
     - EVENTS/CYCLES/TRENDS views.
   - Verify: stride count ±1; distance ±5 %; cadence against a metronome.
4. **M5 — Reference and error engine.**
   - Build:
     - Device-built reference (≥30 cycles); dashboard versioning and locking.
     - 10-cycle reference check.
     - Doc 06 error score, classes and confidence.
     - Segmentation; REFERENCES/SESSIONS workflow.
   - Undefined analysis values become DECs reviewed with the user.
5. **M6 — Exports.**
   - CSV package, `metadata.json`, Level-5 `session.mat` (checked with scipy),
     PDF report (krilla).
6. **M7 — On-device flash storage and recovery.** Needs a storage DEC first
   (1.5 MB data partition).

## Known Problems
- PROB-002: shank map fixed in firmware; on-body confirmation pending (TEST-014).
- PROB-004: GPIO43 may toggle during ROM boot. Unverified; matters only once
  motor drivers are fitted.
- The ±4 g / ±500 °/s ranges may clip during walking. Nothing saturated while
  resting (TEST-018); measure during the M3 walking recordings before proposing
  any change.
- The sample rate is 100.145 Hz, not exactly 100 Hz: it comes from the sensor's
  own oscillator. Analysis must use the device timestamps, never an assumed rate.
- The two IMUs' clocks differ by 0.14 %, so the shank repeats a sample roughly
  every 3.7 s. Those frames are flagged, and ZUPT/orientation work in M3–M4 must
  treat them as what they are rather than as new data.

## Failed Approaches
- The shank map derived by assuming a left-handed anatomical frame (PROB-002
  Attempt 1). Never accept a mount map with determinant −1.
- Gating IMU init on WHO_AM_I 0x68 only (PROB-001). Clone boards carry MPU6500.
- Arduino `Serial` (HWCDC) for the USB link, and silencing only the ESP-IDF
  logger (PROB-006 attempts 1 and 2). The Arduino `log_*` macros write to the
  same USB endpoint and must be compiled out with `CORE_DEBUG_LEVEL=0`; the
  firmware now drives the endpoint directly.
- Reading the IMUs immediately on the data-ready edge (PROB-007). Always leave a
  guard delay.

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
- **The laptop has Wi-Fi only.** Joining the device access point drops its
  internet connection, so the agent tests over USB and the user runs the Wi-Fi
  tests. The access point is `EAD-V1-<last two MAC bytes>`; its WPA2 passphrase
  is generated at first build into `firmware/include/ead_secrets.h`, which is not
  tracked by git. To test:
  `python3 tools/eadprobe.py --ws ws://192.168.4.1:8080/ws stats --seconds 300`.
- Screenshotting the app needs `GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1`
  and `xwd`; the XWD decoder must skip the colormap (`ncolors × 12` bytes) or the
  image comes out shifted sideways. Driving it with `xdotool` needs XTEST (no
  `--window`: WebKit ignores synthetic events) and the client-area origin from
  `xwininfo`, not `xdotool getwindowgeometry`, which includes the title bar.

## How To Run
- Firmware: `cd firmware && pio run` to build, `pio run -t upload` to flash.
- Firmware unit tests: `cd firmware && pio test -e native`.
- Talk to the device without the dashboard:
  - `python3 tools/eadprobe.py hello` — identity and sequence window
  - `python3 tools/eadprobe.py config` — configuration, hash-verified
  - `python3 tools/eadprobe.py stats --seconds 1800 --record run.eadlog`
  - `python3 tools/eadprobe.py reopen --cycles 20` — port reopen must not reset
  - add `--ws ws://192.168.4.1:8080/ws` for the Wi-Fi link
- Regenerate the protocol vectors after a protocol change:
  `python3 protocol/vectors/generate.py`, then rerun both test suites.
- Dashboard: `cd dashboard && npm install`, then `npx tauri dev` to run,
  `npm run build` and `(cd src-tauri && cargo build)` to build.
- Frontend logic tests: `cd dashboard && npm test` (Node 22.6+ runs the
  TypeScript directly; keep test syntax erasable — no enums).
- Dashboard tests: `cd dashboard/src-tauri && cargo test`; add
  `-- --ignored` to run the hardware test with the device attached.
- The dashboard's database lives at
  `~/.local/share/com.ead.dashboard/ead.sqlite3`.

## How To Verify
- Builds and tests: `pio run`, `pio test -e native`, `cargo test`,
  `npm run build` — all clean, no warnings from project code.
- Device, 60 seconds over USB: `python3 tools/eadprobe.py stats --seconds 60`
  should report 0 missing frames, 0 dropped, 0 rejected, |a| ≈ 1.02 g on both
  sensors, and no faults.
- Application: `npx tauri dev`, connect over USB, and check the Live view shows
  about 1 g and near-zero angular rate on a still device.
- Still outstanding: standing still while wearing both sensors must read
  anatomical a ≈ (0, 0, +1) g (TEST-014).

## Next Steps
0. User visually checks the raw view navigation (overview strip, drag-select,
   pan/zoom, four stacked signals) — not yet seen by the agent.
1. Build the guided mounting check, then have the user run it. It replaces
   TEST-014 and settles PROB-002, the last open question on the mount maps.
2. Ask the user to run the Wi-Fi link tests (command in Environment above).
3. Calibration and Mahony orientation; then walking recordings for M4.

## Warnings
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- No battery, switch, BLE, FSR, BNO086 or haptic code (contract + DEC-006).
- Keep motor GPIOs LOW. Driving them has no effect today, but PROB-004 and
  future drivers make any change safety-relevant.
- The root file `wide_infographic_diagram_on_a_white_background_sho.png` is an
  untracked, byte-identical copy of the contract placement image.
- Record a test as passed only after running it.
