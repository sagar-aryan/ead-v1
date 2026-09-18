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

## Current State (2026-09-18, after M6)

Every milestone from M0 to M6 is built. What separates "built" from "working"
here is a single thing: **nobody has walked thirty cycles yet**, so the reference
profile, the error scores, the segments and every export file have only ever seen
synthetic or single-walk data.

- **Firmware streams real data.** 100 Hz acquisition clocked by the foot IMU's
  data-ready interrupt, every frame timestamped and indexed. The doc-08 binary
  protocol runs over both the Wi-Fi access point and USB; roughly 12 minutes of
  telemetry is held in PSRAM for backfill. A 30-minute run lost none of 180,250
  frames (TEST-018).
- **Calibration and orientation work on hardware.** A 5 s still window gives gyro
  bias and gravity alignment; Mahony runs on the device with the alignment
  applied to both accelerometer and gyroscope before each update (PROB-010).
  A flat foot reads −0.11°.
- **Gait detection is measured, not assumed.** On a 6.00 m course the device read
  6.39 m (+6.5 %) with the stride count matching (TEST-030). The thresholds were
  tuned by replaying `recordings/walk6m-2026-09-18.eadlog` through the device's
  own code, never by guessing.
- **The reference workflow and the error engine are complete in code.** The
  device builds a profile from at least thirty of the patient's own valid cycles
  and refuses below that; the dashboard versions it, locks it the moment it
  judges a session, and a SQLite trigger refuses to edit a locked one. Scoring,
  the six classes and the five confidence subscores match hand-computed vectors
  (TEST-031). **None of it has run against a real patient profile.**
- **Segments are counted on the host** (DEC-014, superseding the firmware half of
  DEC-004), because nothing on the device behaves differently at a boundary in
  V1.
- **The export package is complete and independently checked.** `raw.csv`,
  `gait.csv`, `events.csv`, `haptics.csv`, `metadata.json`, `session.mat` and
  `report.pdf`. `tools/check_mat.py` reads the MATLAB file with scipy and
  `tools/check_pdf.py` reads the report with poppler; both found real defects on
  their first run (TEST-036, TEST-037).
- **The dashboard has every doc 11 view** except HAPTICS, which is absent because
  no drivers are fitted: LIVE, CYCLES, EVENTS, RAW, REFERENCES, SESSIONS, EXPORT,
  DEVICE.
- **Not built:** M7, on-device flash storage and recovery. It needs its own
  decision about the 1.5 MB partition first.
- **Never run end to end:** a walk recorded through the dashboard. The gait chain
  has been verified by replay and by `eadprobe`, but cycles reaching the database
  and appearing in the Cycles view has not been watched happen.

## Architecture
See `docs/architecture.md`. In one line: data-ready-driven 100 Hz acquisition →
portable C++ processing (`lib/ead_core`) → PSRAM message ring → Wi-Fi WebSocket
and USB links → Rust backend with SQLite → React views and exports.

## Important Files
| File | Purpose |
|---|---|
| `ead_agent_docs_v2/` | Contract. Never modify |
| `docs/hardware.md` | As-built hardware, mount maps, what is verified |
| `docs/decisions.md` | DEC-001–DEC-014 |
| `docs/problems.md` | PROB-001–PROB-012 |
| `docs/testing.md` | Executed tests with measured results |
| `firmware/include/config_v1.h` | Fixed V1 constants and mount maps |
| `docs/protocol.md` | Wire protocol: payloads, framing, backfill, enumerations |
| `docs/clinical_requirements.md` | The researcher's four requirements vs what is built |
| `firmware/lib/ead_core/` | Portable codec, COBS, CRC-32, message ring, config section |
| `firmware/src/acquisition.cpp` | Data-ready interrupt, self-test, frame assembly |
| `firmware/src/link.cpp` | Per-link protocol endpoint (replies, streaming, backfill) |
| `protocol/vectors/` | Golden vectors; `generate.py` rebuilds them |
| `tools/eadprobe.py` | Independent host decoder: `hello`, `config`, `stats`, `reopen`, `calibrate`, `walk`, `capture`, `score`, `vectors` |
| `tools/replay/main.cpp` | Replays a `.eadlog` through the device's own code; how gait thresholds are changed |
| `tools/check_mat.py` | Reads an exported `session.mat` with scipy and checks it against the CSVs |
| `tools/check_pdf.py` | Reads an exported `report.pdf` with poppler and checks doc 10 §8 |
| `recordings/` | Walks with verified ground truth, and what each one is |
| `firmware/lib/ead_core/src/ead/gait.{h,cpp}` | Gait state machine, ZUPT, cycle features |
| `firmware/lib/ead_core/src/ead/reference.{h,cpp}` | Feature table, weights, median/MAD, the builder |
| `firmware/lib/ead_core/src/ead/error_engine.{h,cpp}` | Score, six classes, five confidence subscores |
| `dashboard/src-tauri/src/export/` | `csv.rs`, `mat.rs`, `pdf.rs` and the package orchestration |
| `dashboard/src-tauri/assets/fonts/` | IBM Plex latin subsets as TTF, embedded in the report |
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
- M3 mounting check, calibration and orientation (2026-09-18).
- M4 gait events, ZUPT and cycle distance, tuned on a measured course (2026-09-18).
- M5 reference profiles, the error engine, the session gate and segments (2026-09-18).
- M6 the whole doc 10 export package including the PDF report (2026-09-18).

## Current Work
Nothing is in progress. The next work is not code: it is the hardware
verification that M5 and M6 have never had. See Next Steps.

## Milestone Plan
M0–M6 are done; `docs/progress.md` has each one's entry. What is left:

1. **Hardware verification of M5 and M6** (needs the user — see Next Steps).
2. **M7 — on-device flash storage and recovery.** Blocked on a storage decision:
   the default partition table leaves about 1.5 MB, and doc 09 §10 expects the
   device to survive a session without a host. Write the DEC before any code.
3. **Adaptive thresholds.** Doc 05 §3 wants the gait thresholds adapted per
   patient; today they are named constants fitted to one recording of one
   person's walking.

## Known Problems
- **The biggest one is not a bug:** the reference workflow, the error engine, the
  segments and every export file have only been exercised against synthetic
  cycles. Until somebody walks thirty cycles through the dashboard, they are
  untested against reality.
- The gait thresholds were fitted to a single 6 m walk by one person at one
  speed (TEST-030). They are named constants and overridable at run time, but
  nothing yet says they generalise.
- PROB-002: shank map fixed in firmware and confirmed by the mounting check.
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
- DEC-013 the four values doc 06 leaves open, chosen and justified.
- DEC-014 segments counted on the host, superseding the firmware half of DEC-004.

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
- **On a new machine:** one script per OS in `scripts/`, and one command. With
  no argument each script checks the dependencies, asks once, installs what is
  missing, checks again, clones or updates the repository into `~/ead-v1`, runs
  `npm ci` and starts the dashboard — only once nothing is missing.
  - Linux: `curl -fsSL https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-linux.sh | bash` (apt, dnf or pacman for
    system libraries; nvm and rustup for Node and Rust; adds you to the serial
    group, which needs a log-out to take effect)
  - macOS: `curl -fsSL https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-macos.sh | bash` (Xcode command line tools;
    Node from Homebrew, or nvm without it; rustup)
  - Windows: `irm https://raw.githubusercontent.com/sagar-aryan/ead-v1/main/scripts/setup-windows.ps1 -OutFile $env:TEMP\ead-setup.ps1`, then
    `powershell -ExecutionPolicy Bypass -File $env:TEMP\ead-setup.ps1` (winget).
    Not a piped scriptblock: the script's `exit` would close the terminal.
  Arguments: `check` (report only, change nothing) and `build` (an installer).
  A script run while a dashboard is open says so instead of failing on port 1420.
  Linux verified end to end; macOS and Windows not run (TEST-038).
- Firmware: `cd firmware && pio run` to build, `pio run -t upload` to flash.
- Firmware unit tests: `cd firmware && pio test -e native`.
- Talk to the device without the dashboard:
  - `python3 tools/eadprobe.py hello` — identity and sequence window
  - `python3 tools/eadprobe.py config` — configuration, hash-verified
  - `python3 tools/eadprobe.py stats --seconds 1800 --record run.eadlog`
  - `python3 tools/eadprobe.py calibrate --seconds 5` — still window, prints the record
  - `python3 tools/eadprobe.py reopen --cycles 20` — port reopen must not reset
  - `python3 tools/eadprobe.py walk --seconds 30` — cycles the device detected
  - `python3 tools/eadprobe.py capture --seconds 60 --save profile.bin` — build a
    reference profile; exits 1 if the device refuses (fewer than 30 valid cycles)
  - `python3 tools/eadprobe.py score --profile profile.bin --seconds 30` — a
    reference check; add `--evaluate` for a scored evaluation
  - `python3 tools/eadprobe.py vectors` — decode the golden vectors with this
    tool's own decoders. **Run this after every protocol change**; nothing else
    checks the Python decoder, and it silently fell a schema behind once.
  - add `--ws ws://192.168.4.1:8080/ws` for the Wi-Fi link
- Regenerate the protocol vectors after a protocol change:
  `python3 protocol/vectors/generate.py`, then rerun both test suites.
- Dashboard: `cd dashboard && npm install`, then `npx tauri dev` to run,
  `npm run build` and `(cd src-tauri && cargo build)` to build.
- Frontend logic tests: `cd dashboard && npm test` (Node 22.6+ runs the
  TypeScript directly; keep test syntax erasable — no enums).
- Replay a recording through the device's own code:
  `g++ -std=gnu++17 -O2 -I firmware/lib/ead_core/src -I firmware/include -o /tmp/eadreplay
   tools/replay/main.cpp firmware/lib/ead_core/src/ead/{calibration,mahony,gait,protocol,crc32,cobs}.cpp`
  then `/tmp/eadreplay recordings/walk6m-2026-09-18.eadlog --still-seconds 5`.
  This is how gait thresholds are changed: against a recording with known ground
  truth, never by guessing.
- Dashboard tests: `cd dashboard/src-tauri && cargo test`; add
  `-- --ignored` to run the hardware test with the device attached.
- The dashboard's database lives at
  `~/.local/share/com.ead.dashboard/ead.sqlite3`, and exports default to
  `exports/<session id>` beside it.
- Check an export package end to end:
  `cd dashboard/src-tauri && cargo test -- --ignored export_sample`, then
  `python3 tools/check_mat.py dashboard/src-tauri/target/export-sample` and
  `python3 tools/check_pdf.py dashboard/src-tauri/target/export-sample`.
  Both need scipy and poppler respectively.

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
In order. The first four need the user; nothing useful comes before them.

1. **A walk recorded through the dashboard.** The one part of the chain never
   watched end to end is cycles reaching the database and appearing in the
   Cycles view. Start a recording in SESSIONS, walk, stop, open CYCLES.
2. **A reference capture of about a minute.** Thirty valid cycles is the floor
   the device enforces; a minute of walking is comfortably above it. Do it in
   REFERENCES → Capture. Until this happens no reference profile has ever
   existed, and requirements 2 and 3 are code that has never seen a patient.
3. **A reference check, then an evaluation** against that profile, with segment
   limits entered. Then export the session and read the report.
4. **Walks at different speeds**, and one deliberately faster walk to settle
   whether ±4 g clips at heel strike (PROB-011).
5. Then: adaptive thresholds (doc 05 §3), and the M7 storage decision.

## Warnings
- **The repository is public** (since 2026-09-18). Everything tracked is
  visible, including history: the contract package `ead_agent_docs_v2/`, the
  researcher's `Critical Clinical Insights.docx` and the walk recording. Never
  commit `firmware/include/ead_secrets.h` (the access-point passphrase); it is
  gitignored and has never been in any commit.
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- No battery, switch, BLE, FSR, BNO086 or haptic code (contract + DEC-006).
- Keep motor GPIOs LOW. Driving them has no effect today, but PROB-004 and
  future drivers make any change safety-relevant.
- The root file `wide_infographic_diagram_on_a_white_background_sho.png` is an
  untracked, byte-identical copy of the contract placement image.
- Record a test as passed only after running it.
