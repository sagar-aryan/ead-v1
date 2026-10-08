# Project Handoff

## Project
EAD V1: a right-leg, barefoot, wearable error-augmentation system for stroke
gait. A XIAO ESP32-S3 with two IMUs (foot + shank) and a six-motor haptic band,
plus a Tauri 2 research dashboard. Contract: `ead_agent_docs_v2/`
(authoritative, read-only).

## Current Objective
Demonstrate the full reference → check → evaluation walk with feedback. The user set
the cue strength (graded 60–100 %), cues paused after 5 s without the laptop, the
patient's name in every export, the wearer's own baseline as the target and the right
leg as the focus (DEC-026–028). Next: the motor-position check and the demo walk. The
user wants engineered work only: no filler, no placeholder or fake data, no features
the spec does not need.

## Current State (2026-10-07)

- **Hardware.** The XIAO (MAC 44:B1:76:AF:FB:7C) wired to DEC-016, both BNO086 on SPI,
  the separate ERM driver PCB (`hardware/erm-driver-pcb/`, tight variant) with six motors and two HT7833
  rails, battery. See `docs/hardware.md`. **Battery switch never ON with USB.**
- **Firmware `0.1.0+d59ccb7` flashed 2026-10-08:** M5/M6 order updated (DEC-029),
  timestamp handling and stride-plausibility checks. Checked over USB:
  `eadprobe hello` reports the version and schema 9, `config` motor pins m1–m6 = 4, 6,
  42, 5, 2, 1, `check` passes on both sensors. Next: the user's motor-position check
  on battery over Wi-Fi.
- **Firmware `0.1.0+ae4b4b5`, protocol schema 9, flashed (TEST-069):** cues graded
  60–100 % for 500 ms (DEC-026), held back after 5 s without the laptop (DEC-027),
  calibration record kept until sent. Earlier features as below.
- **Firmware as of `29c560c`, protocol schema 8:** 200 Hz frames clocked by
  the foot gyroscope, the BNO086's own orientation and a native 250 Hz accelerometer
  stream (DEC-021); gait events from the shank's swing (DEC-022); haptic feedback in
  evaluations behind a master switch, off at boot (DEC-023); ACK for session commands
  and the running session in STATUS (DEC-024); cues pause on a sensor fault or a stale
  sample; every calibration adopted.
- **Verified on the board:** 200.29 Hz, 0 dropped (TEST-064); ACK and STATUS `session`
  (TEST-064); a device reset ends the session, a recalibration after re-strapping is
  used (worn, TEST-065); 10 m walks over Wi-Fi at 200 Hz with 0 frames lost
  (TEST-058). Step counting on the seven video-synced fixtures: 173 of 176
  landings matched (TEST-059).
- **Walked worn on battery over Wi-Fi (TEST-066, TEST-067):** the full workflow ran,
  0 frames lost; cue strength and spread floors tuned on these walks (DEC-025, DEC-026).
- **Reference profiles:** patient 67's current baseline is v6, recorded 2026-10-07 on
  a 10 m straight and used for the deliberate-error walks (TEST-071). Earlier versions
  (v1–v5) are kept for the record.
- **Dashboard (`ead`, rebuilt 2026-10-07):** every doc 11 view except a
  separate HAPTICS page, plus Check; protocol schema 9, store schema 12. The database on
  this machine migrates to 12 when the app next opens (migration tested on a copy: 41
  status rows kept, integrity ok); backup `ead.sqlite3.schema11-backup-2026-10-07`
  beside it. Every CSV carries `patient_name`; `gait.csv` has `valid`. Session commands wait for the device's answer; stops drain to the device's
  boundary; save status shows in the state bar; sessions export once ended.

## Architecture
See `docs/architecture.md`. In one line: INT-driven 200 Hz acquisition over SPI →
portable C++ processing (`lib/ead_core`) → PSRAM message ring → Wi-Fi WebSocket
and USB links → Rust backend with SQLite → React views and exports.

## Important Files
| File | Purpose |
|---|---|
| `ead_agent_docs_v2/` | Contract. Never modify |
| `docs/hardware.md` | Current DEC-016 build, previous MPU6500 build, mount maps, what is verified |
| `docs/wiring_reference.md` | Every DEC-016 connection with its evidence; firmware rules the wiring depends on |
| `docs/protocol.md` | Wire protocol (schema 9): payloads, framing, backfill, ACK, SERVICE_TEST, haptics, enumerations |
| `docs/clinical_requirements.md` | The researcher's four requirements vs what is built |
| `firmware/include/config_v1.h` | Fixed V1 constants, the DEC-016 pins with compile-time pin checks, BNO086 scales, mount maps |
| `firmware/lib/ead_core/` | Portable codec, COBS, CRC-32, ring, SH-2 codec, motor guard, calibration, Mahony, gait, reference, error engine |
| `firmware/src/` | Device services: BNO086 driver, acquisition, motors, links, calibration, orientation, gait, sessions |
| `firmware/bench/bno086/` | BNO086 bench test on the DEC-016 pins (SparkFun library, one sensor per build) |
| `firmware/bench/padstate/` | Pad-state and JTAG eFuse reader (TEST-041) |
| `protocol/vectors/` | 27 golden vectors (incl. the format-1 config section); `generate.py` rebuilds them |
| `tools/eadprobe.py` | Independent host client and decoder; `check`, `pulse` |
| `tools/replay/main.cpp` | Replays a `.eadlog` through the device's own code; how gait thresholds are changed |
| `tools/check_mat.py`, `tools/check_pdf.py` | Independent export checkers (scipy, poppler) |
| `tools/bno_view.py` | BNO086 bench viewer: checks and a tilt window |
| `recordings/` | Walks with verified ground truth |
| `dashboard/src-tauri/src/` | protocol, link, device, store, live, orientation, export, app |
| `dashboard/src/` | React UI: api, useDevice, components, views (`Check.tsx` for `ead --check`) |
| `handoff/` | Untracked handoff notes from 2026-10-02 (the user asked not to commit them) |

## Completed Work
- M0–M2 (2026-09-17): baseline fixes, acquisition, protocol, both links, the
  dashboard foundation and raw view.
- M3–M6 (2026-09-18): calibration and orientation, gait and ZUPT, reference and
  error engine, the export package.
- First sessions with patient 67 (2026-09-18).
- BNO086 on the bench, genuine part confirmed (TEST-039, TEST-040, 2026-09-23).
- Haptic band placement on the calf (DEC-015, 2026-09-23).
- Final pin map DEC-016, wiring reference, pad states measured on the chip
  (TEST-041), 2026-10-01.
- 2026-10-02: calibration and session handling across device reboots, verified on
  hardware (TEST-042); `ead` command; both BNO086 proven with the bench
  driver (TEST-043); the BNO086 product firmware, SERVICE_TEST and `ead --check`
  (DEC-017, DEC-018, TEST-045–050).
- 2026-10-03/04: mount maps measured (TEST-051), 200 Hz feed (DEC-021), video-synced
  walks over Wi-Fi (TEST-058), shank-swing step detection (DEC-022), haptic feedback
  (DEC-023), schema 7 on the board (TEST-062).
- 2026-10-05: power budget estimate (`hardware.md`); an external code review worked
  through (DEC-024, TEST-063–065). First walks with feedback (TEST-066–068), DEC-025;
  the ERM driver PCB project into the repository.
- 2026-10-07: second external code review worked through; the user's choices
  (DEC-026–028); mirrored score constants tested against the firmware headers; root
  README; deliberate-error walks against v6 (TEST-071).
- 2026-10-08: firmware `d59ccb7` on the board.

## Current Work
Nothing in flight. Next is the demo walk (Next Steps).

## Roadmap
- Cue refinement: timing judged from cycle time, a finer dorsiflexion threshold.
- Per-step distance refinement, against floor-mark ground truth.
- Confidence-weighted cue gating; automatic baseline-to-strapping matching.
- Functional calibration of the shank swing axis.
- Pause/resume and on-device storage.
- Studies with more wearers.

## Working Practices
- Measure mount maps on the leg; a map must be a proper rotation (determinant +1).
- Identify the IMU by its product ID, not WHO_AM_I alone.
- Drive the USB Serial/JTAG endpoint directly; Arduino `Serial` stays unused.
- Read the IMUs 1–2 ms after the data-ready edge.
- Reset the board with `esptool --after watchdog_reset` or a replug, not by toggling DTR/RTS by hand.
- Change gait thresholds only by replaying recordings with ground truth
  (`tools/replay/`).
- Run the whole suite before every commit.
- Replayed counts are converted with the recording's own configuration.
- Stop the dashboard by PID (`pkill -f` matches the calling shell).
- Session commands wait for the device's ACK or ERROR.
- Backfill requests go out only after the connection's HELLO.
- Don't run `cargo fmt` on the dashboard (the crate is not rustfmt-formatted).

## Important Decisions
Key design choices: DEC-005 USB link; DEC-007 raw =
chip-frame counts;
DEC-008 session kinds; DEC-009 per-sensor mount maps; DEC-010 `esp_http_server`;
DEC-011 dashboard stack; DEC-012 device-built reference; DEC-013 doc 06's open
values (provisional); DEC-014 host-side segments; DEC-015 haptic band on the
calf; DEC-016 final pin map, all 15 GPIOs used, GPIO39 never a motor; DEC-017 BNO086
firmware (own SH-2 driver, Mahony kept, calibrated reports as counts); DEC-018 motor
service-test pulses; DEC-019–021 soft footfalls, the user's decisions, the 200 Hz feed;
DEC-022 gait events from the shank's swing; DEC-023 haptic feedback during evaluations;
DEC-024 session commands acknowledged, a session outlives the link but not the app;
DEC-025 0.5 s cues, OFF at 0.35, realistic spread floors (its fixed 100 % duty
superseded by DEC-026 graded 60–100 %); DEC-027 cues held back after 5 s without the
laptop (schema 9); DEC-028 the patient's name in every exported file; DEC-029 motors
rewired by the user to a new order over the same six pins.

## Environment
- Linux (Ubuntu 24.04), node 24.13.1, npm 11.8.0, rustc/cargo 1.95.0.
- PlatformIO 6.2.0 with `espressif32 @ 7.1.3` (Arduino-ESP32 2.0.17).
- Python 3.12.3 with pyserial 3.5, numpy 1.26.4, scipy 1.11.4, matplotlib.
- WebKitGTK 4.1, libudev, poppler-utils. The user is in `dialout` and `plugdev`.
- The device enumerates as `303a:1001`; the `/dev/ttyACM*` number changes after
  resets.
- **The laptop has Wi-Fi only.** Joining the device access point drops its
  internet connection, so the agent tests over USB and the user runs the Wi-Fi
  tests. The access point is `EAD-V1-<last two MAC bytes>`; its passphrase is
  generated into `firmware/include/ead_secrets.h`, which is not tracked.
- Screenshotting the app needs `GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1`
  and `xwd` (skip the colormap, `ncolors × 12` bytes); `xdotool` needs XTEST
  and the client-area origin from `xwininfo`.

## How To Run
- **New machine:** `scripts/setup-linux.sh`, `setup-macos.sh`,
  `setup-windows.ps1` (one command each, from GitHub; arguments `check` and
  `build`). Linux verified; the macOS and Windows scripts are included (TEST-038).
- **BNO086 bench (the DEC-016 build):**
  `pio run -d firmware/bench/bno086 -e foot -t upload` (or `-e shank`), then
  `python3 tools/bno_view.py --checks-only`, or without the flag for the tilt
  window. If USB goes silent, start the application with `esptool --before no_reset --after watchdog_reset` (or replug).
- **Product firmware (DEC-016 build):** `pio run -d firmware`,
  `pio run -d firmware -t upload`. Then `python3 tools/eadprobe.py check` (per-wire
  check; `--rerun` resets the sensors) and `eadprobe pulse 1 --duty 102` (one motor).
- Firmware unit tests: `pio test -d firmware -e native`.
- Check view: `ead --check`.
- Talk to the device (product firmware): `python3 tools/eadprobe.py hello`,
  `config`, `stats --seconds 60`, `calibrate --seconds 5`, `walk`, `capture`,
  `score`, `reopen`; add `--ws ws://192.168.4.1:8080/ws` for Wi-Fi.
- After any protocol change: `python3 protocol/vectors/generate.py`, both test
  suites, `python3 tools/eadprobe.py vectors`, and reflash.
- Dashboard: `ead` from any terminal (a link in `~/.local/bin` to the release
  build; set up and rebuilt as in `dashboard/README.md`), or
  `cd dashboard && npm install && npx tauri dev` for development. Database:
  `~/.local/share/com.ead.dashboard/ead.sqlite3`; exports beside it.
- Replay: build `tools/replay/main.cpp` with the `ead_core` sources (command in
  `recordings/README.md`), then
  `/tmp/eadreplay recordings/walk6m-2026-09-18.eadlog --still-seconds 5 --mpu6500`
  (expect 6 cycles, 6.39 m). New recordings carry their configuration: no flag.
- Export check: `cd dashboard/src-tauri && cargo test -- --ignored
  export_sample`, then `tools/check_mat.py` and `tools/check_pdf.py` on
  `target/export-sample`.

## How To Verify
The whole suite, before every commit:
- `pio test -d firmware -e native` (104), `pio run -d firmware`,
  `pio run -d firmware/bench/bno086 -e foot -e shank`
- `cd dashboard/src-tauri && cargo test` (91, 4 ignored) and
  `cargo clippy --all-targets -- -D warnings`
- `cd dashboard && npm test` (28) and `npm run build`
- `python3 tools/eadprobe.py vectors` (27, 0 failures)

On hardware over USB, battery off: `eadprobe stats --seconds 60` (200.3 Hz, 0
dropped), `cargo test hardware -- --ignored --test-threads=1` (both hardware tests;
the second resets the board and needs both sensors right way up, or worn standing).

## Next Steps
1. **Motor positions (user, battery, Wi-Fi; DEC-029):** pulse M1–M6 from the Check
   view and confirm each at its band position (M1 front, M2 front-outer, M3 back-outer,
   M4 back, M5 back-inner, M6 front-inner).
2. **Demo walk (user, battery, Wi-Fi):** calibrate, evaluation against v6 with the
   switch on; confirm graded strength (60 % vs 100 %) and that cues pause within 5 s
   when the laptop's Wi-Fi is off (`link_lost` records in haptics.csv).
3. The Roadmap items above, each judged on recorded walks with ground truth.
4. Event timing against the synced videos.
5. Dashboard additions from doc 11: symmetry proxy on LIVE, RAW cycle/error filters,
   PAUSE/RESUME.
6. M7 storage DEC, then the code.

## Warnings
- **Only firmware on the DEC-016 pins goes on this board** (product firmware from
  `5b8dd28`, `firmware/bench/*`). Any firmware for it drives the enabled motor gates
  LOW first (a doubtful channel stays out of the enable mask), never asserts both chip
  selects, never starts UART0, and never puts a motor on GPIO39
  (`docs/wiring_reference.md` firmware rules).
- **The battery switch is never ON while USB is plugged in** (user, 2026-10-05).
  A motor test that needs battery power runs over Wi-Fi with USB unplugged.
- `firmware/bench/padstate` briefly pulls each motor gate up at boot (0.3 ms).
- **The repository is public.** Never commit `firmware/include/ead_secrets.h`;
  never put the patient's name in docs (use the ID, `67`).
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- Calibration lives in device RAM only; recalibrate after every reset or flash.
- Record a test as passed only after running it, with its numbers.
