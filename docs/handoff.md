# Project Handoff

## Project
EAD V1: a right-leg, barefoot, wearable error-augmentation system for stroke
gait. A XIAO ESP32-S3 with two IMUs (foot + shank) and a six-motor haptic band,
plus a Tauri 2 research dashboard. Contract: `ead_agent_docs_v2/`
(authoritative, read-only).

## Current Objective
Two tracks. Software: the device data path and the research dashboard, built in
verified milestones (M0–M6 done, M7 not started). Hardware: moving the device
from two MPU6500s on I²C to two BNO086s on SPI (DEC-016), which the product
firmware does not support yet. The user wants engineered work only: no filler,
no placeholder or fake data, no features the spec does not need.

## Current State (2026-10-02)

**The physical device and the product firmware no longer match.**

- **Hardware.** The user reports (2026-10-02) the XIAO is wired to DEC-016
  exactly (`docs/wiring_reference.md`; `EAD_V1_XIAO_connections.pdf`), with
  everything soldered: both BNO086, six motor channels and motors, both HT7833
  rails, battery. It has been powered since assembly. Nothing has been measured
  on the assembled build. See `docs/hardware.md` "Current build".
- **Product firmware** (`firmware/`) implements the previous MPU6500/I²C build
  (doc 03 pins). **It must not be flashed onto the DEC-016 build**: it would
  toggle motor gates 5 and 6 (I²C on GPIO5/6) and drive both chip selects and
  MOSI LOW as "motors". There is no BNO086 driver and no haptic code in it.
- **Bench firmware** `firmware/bench/bno086` now uses the DEC-016 pins, one
  sensor per build (`-e foot`, `-e shank`), motors LOW first. Built, not run
  (TEST-043).
- **On the MPU6500 build, everything M0–M6 works and was verified:** 100 Hz
  data-ready acquisition with 0 lost frames in 30 min (TEST-018); calibration and
  Mahony orientation on the leg (TEST-027–029); gait and ZUPT, 6.39 m on a
  6.00 m course (TEST-030); the reference workflow, error engine, session gate
  and host-side segments (TEST-031–032); the full export package, checked by
  scipy and poppler against synthetic sessions (TEST-035–037).
- **Real-person use (2026-09-18, patient 67):** references v1 (42 cycles), v2
  (33) and v3 (50), seven reference checks and three evaluations, in the
  dashboard database. **All three references were built while PROB-016 split
  strides, so none can be trusted.** The evaluations recorded no cycles; two of
  them because of PROB-018 (fixed in code on 2026-10-02), the third (0 frames)
  for an unknown reason. No trustworthy reference exists.
- **Dashboard:** every doc 11 view except HAPTICS. Known gaps against doc 11
  are listed under Next Steps.

## Architecture
See `docs/architecture.md`. In one line: data-ready-driven 100 Hz acquisition →
portable C++ processing (`lib/ead_core`) → PSRAM message ring → Wi-Fi WebSocket
and USB links → Rust backend with SQLite → React views and exports.

## Important Files
| File | Purpose |
|---|---|
| `ead_agent_docs_v2/` | Contract. Never modify |
| `docs/hardware.md` | Current DEC-016 build, previous MPU6500 build, mount maps, what is verified |
| `docs/wiring_reference.md` | Every DEC-016 connection with its evidence; firmware rules the wiring depends on |
| `docs/decisions.md` | DEC-001–DEC-016 |
| `docs/problems.md` | PROB-001–PROB-019 (no PROB-008) |
| `docs/testing.md` | TEST-001–TEST-043, with measured results |
| `docs/protocol.md` | Wire protocol (schema 4): payloads, framing, backfill, enumerations |
| `docs/clinical_requirements.md` | The researcher's four requirements vs what is built |
| `firmware/include/config_v1.h` | Fixed V1 constants, the doc-03 pin map, mount maps |
| `firmware/lib/ead_core/` | Portable codec, COBS, CRC-32, ring, calibration, Mahony, gait, reference, error engine |
| `firmware/src/` | Device services: acquisition, links, calibration, orientation, gait, sessions |
| `firmware/bench/bno086/` | BNO086 bench test on the DEC-016 pins (SparkFun library, one sensor per build) |
| `firmware/bench/padstate/` | Pad-state and JTAG eFuse reader (TEST-041) |
| `protocol/vectors/` | 17 golden vectors; `generate.py` rebuilds them |
| `tools/eadprobe.py` | Independent host client and decoder |
| `tools/replay/main.cpp` | Replays a `.eadlog` through the device's own code; how gait thresholds are changed |
| `tools/check_mat.py`, `tools/check_pdf.py` | Independent export checkers (scipy, poppler) |
| `tools/bno_view.py` | BNO086 bench viewer: checks and a tilt window |
| `recordings/` | Walks with verified ground truth |
| `dashboard/src-tauri/src/` | protocol, link, device, store, live, orientation, export, app |
| `dashboard/src/` | React UI: api, useDevice, components, views |
| `handoff/` | Untracked handoff notes from 2026-10-02 (the user asked not to commit them) |

## Completed Work
- M0–M2 (2026-09-17): baseline fixes, acquisition, protocol, both links, the
  dashboard foundation and raw view.
- M3–M6 (2026-09-18): calibration and orientation, gait and ZUPT, reference and
  error engine, the export package.
- Real-person sessions with patient 67 (2026-09-18), which produced PROB-013 to
  PROB-016.
- BNO086 on the bench, genuine part confirmed (TEST-039, TEST-040, 2026-09-23).
- Haptic band placement on the calf (DEC-015, 2026-09-23).
- Final pin map DEC-016, wiring reference, pad states measured on the chip
  (TEST-041), 2026-10-01.
- 2026-10-02: PROB-018 fixed (calibration kept across device reboots); bench
  firmware moved to DEC-016; these docs brought back to the actual state.

## Current Work
Nothing in flight. The board was not connected on 2026-10-02, so TEST-043 and
the hardware steps of TEST-042 wait.

## Milestone Plan
1. **TEST-043**: each BNO086 on the DEC-016 wiring with the bench firmware.
2. **DEC-017, the BNO086 product firmware.** Needs the user's decisions first:
   the device's own game rotation vector or the contract's Mahony; our own SH-2
   driver or CEVA's sources (single-instance, would need changes for two
   sensors); the raw data format (DEC-007 assumes int16 counts) and whether that
   means protocol schema 5; keep or retire the MPU6500 build. Then the pin map
   moves to DEC-016 (motors 1, 2, 42, 4, 5, 6), and the mount maps and gait
   thresholds are re-measured on the leg.
3. **Haptics.** DEC-006's condition (drivers fitted) is met; lifting it is the
   user's decision. Doc 06 §6–§10 specifies the engine.
4. **M7**, on-device storage: needs a DEC (the default partition leaves 1.5 MB).
5. **Walking validation** on the new build: a new reference, a check, an
   evaluation, an export read end to end.

## Known Problems
- **PROB-019 (open):** a device reboot during a recording leaves the host
  session open; new-boot frames with an already-stored index are dropped, later
  ones mix with the old boot's, colliding cycles are overwritten. Needs a
  decision (see the entry).
- **No trustworthy reference profile:** v1–v3 carry the PROB-016 fault.
- **Gait thresholds** were fitted to one 6 m walk by one person (TEST-030) and
  to MPU6500 data; doc 05 §3 wants adaptive thresholds.
- **PROB-015:** inversion shifted about 15° after re-wearing; root cause unknown.
  A reference is only trustworthy within the wearing and calibration it was
  captured in.
- **PROB-012:** remaining +6.5 % distance error; the proper fix is a full
  error-state Kalman filter.
- **The device never reports REFERENCE_CAPTURE or RUNNING** in STATUS
  (`device.cpp` `currentState`), unlike DEC-008's description.
- `EAD_GAIT_LOWPASS_HZ` and `EAD_EVENT_PATH_LOWPASS_HZ` are reported in
  CONFIG_GET but no such filter is applied.
- Unknown on the DEC-016 build: whether the §13 pre-power checks were done,
  which HT7833 part was fitted and by which pinout, and which XIAO is in it.

## Failed Approaches
- Mount maps derived from statements or images (PROB-002). Measure on the leg;
  never accept determinant −1.
- Gating IMU init on WHO_AM_I 0x68 only (PROB-001).
- Arduino `Serial`/HWCDC on this chip, and silencing only the IDF logger
  (PROB-006). Drive the USB Serial/JTAG endpoint directly.
- Reading the IMUs on the data-ready edge (PROB-007).
- Hand-toggling DTR/RTS to reset the board: it lands in ROM download mode
  (PROB-017).
- Hinge-axis correction of the ankle frontal angle (PROB-015): unstable.
- Tuning gait thresholds by guessing; always replay recordings with ground
  truth.
- Clearing PROB-018's calibration in the per-connection tracker: it also wiped
  a valid calibration on every replug (PROB-018 Attempt 1).
- Trusting `cargo test <one test>` before a commit; run the whole suite.

## Important Decisions
DEC-005 USB link; DEC-006 no haptic code; DEC-007 raw = chip-frame counts;
DEC-008 session kinds; DEC-009 per-sensor mount maps; DEC-010 `esp_http_server`;
DEC-011 dashboard stack; DEC-012 device-built reference; DEC-013 doc 06's open
values (provisional); DEC-014 host-side segments; DEC-015 haptic band on the
calf; DEC-016 final pin map, all 15 GPIOs used, GPIO39 never a motor.

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
  `build`). Linux verified; macOS and Windows never run (TEST-038).
- **BNO086 bench (the DEC-016 build):**
  `pio run -d firmware/bench/bno086 -e foot -t upload` (or `-e shank`), then
  `python3 tools/bno_view.py --checks-only`, or without the flag for the tilt
  window. If USB goes silent, read PROB-017 first.
- **Product firmware (MPU6500 build only; never onto DEC-016 wiring):**
  `pio run -d firmware`, `pio run -d firmware -t upload`.
- Firmware unit tests: `pio test -d firmware -e native`.
- Talk to the device (product firmware): `python3 tools/eadprobe.py hello`,
  `config`, `stats --seconds 60`, `calibrate --seconds 5`, `walk`, `capture`,
  `score`, `reopen`; add `--ws ws://192.168.4.1:8080/ws` for Wi-Fi.
- After any protocol change: `python3 protocol/vectors/generate.py`, both test
  suites, `python3 tools/eadprobe.py vectors`, and reflash (PROB-013).
- Dashboard: `cd dashboard && npm install && npx tauri dev`. Database:
  `~/.local/share/com.ead.dashboard/ead.sqlite3`; exports beside it.
- Replay: build `tools/replay/main.cpp` with the `ead_core` sources (command in
  `recordings/README.md`), then
  `/tmp/eadreplay recordings/walk6m-2026-09-18.eadlog --still-seconds 5`
  (expect 6 cycles, 6.39 m).
- Export check: `cd dashboard/src-tauri && cargo test -- --ignored
  export_sample`, then `tools/check_mat.py` and `tools/check_pdf.py` on
  `target/export-sample`.

## How To Verify
The whole suite, before every commit:
- `pio test -d firmware -e native` (63), `pio run -d firmware`,
  `pio run -d firmware/bench/bno086 -e foot -e shank`
- `cd dashboard/src-tauri && cargo test` (59, 3 ignored) and
  `cargo clippy --all-targets -- -D warnings`
- `cd dashboard && npm test` (28)
- `python3 tools/eadprobe.py vectors` (17, 0 failures)

On hardware: TEST-043 for the DEC-016 build.

## Next Steps
1. Plug the board in and run TEST-043 (both envs).
2. User decisions: PROB-019's fix; DEC-017's questions (Milestone Plan 2);
   lifting DEC-006.
3. Write DEC-017 and the BNO086 product firmware; then the TEST-042 hardware
   steps and a new reference on the new build.
4. Dashboard gaps against doc 11: live gait metrics on LIVE, the seven TRENDS
   panels, CYCLES → RAW click-through, RAW cycle/error filters, the "roll the
   sole inward" mounting step, PAUSE/RESUME, CONFIG_SET.
5. M7 storage DEC, then the code.

## Warnings
- **Never flash the product firmware onto the DEC-016 build** (see Current
  State). Any firmware for it drives the six motor gates (GPIO1, 2, 42, 4, 5, 6)
  LOW first, never asserts both chip selects, never starts UART0, and never puts
  a motor on GPIO39 (`docs/wiring_reference.md` firmware rules).
- `firmware/bench/padstate` briefly pulls each motor gate up at boot (0.3 ms);
  do not leave it on the board longer than needed.
- **The repository is public.** Never commit `firmware/include/ead_secrets.h`;
  never put the patient's name in docs (use the ID, `67`).
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- Calibration lives in device RAM only; recalibrate after every reset or flash.
- Record a test as passed only after running it, with its numbers.
