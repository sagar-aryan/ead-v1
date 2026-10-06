# Project Handoff

## Project
EAD V1: a right-leg, barefoot, wearable error-augmentation system for stroke
gait. A XIAO ESP32-S3 with two IMUs (foot + shank) and a six-motor haptic band,
plus a Tauri 2 research dashboard. Contract: `ead_agent_docs_v2/`
(authoritative, read-only).

## Current Objective
Make the device trustworthy for a real reference → check → evaluation walk with
feedback. On 2026-10-07 a second external audit (`EAD_AUDIT_REPORT.md`, untracked) was
checked finding by finding; the user answered its questions (cue strength graded
60–100 %, cues held back after 5 s without the laptop, the patient's name in every
export, baseline as the target, right leg only, no external sync system) and the
code-level findings were fixed (DEC-026–028, PROB-037–042). Next: flash, then the
user's evaluation walk. The user wants engineered work only: no filler, no placeholder
or fake data, no features the spec does not need.

## Current State (2026-10-07)

- **Hardware.** The XIAO (MAC 44:B1:76:AF:FB:7C) wired to DEC-016, both BNO086 on SPI,
  the separate ERM driver PCB (`hardware/erm-driver-pcb/`, tight variant) with six motors and two HT7833
  rails, battery. See `docs/hardware.md`. **Battery switch never ON with USB.**
- **Firmware `0.1.0+ae4b4b5`, protocol schema 9, flashed (TEST-069):** cues graded
  60–100 % for 500 ms (DEC-026), held back after 5 s without the laptop (DEC-027),
  calibration record kept until sent (PROB-037). Earlier features as below.
- **Firmware as of `29c560c`, protocol schema 8:** 200 Hz frames clocked by
  the foot gyroscope, the BNO086's own orientation and a native 250 Hz accelerometer
  stream (DEC-021); gait events from the shank's swing (DEC-022); haptic feedback in
  evaluations behind a master switch, off at boot (DEC-023); ACK for session commands
  and the running session in STATUS (DEC-024); sensor faults and stale samples stop
  feedback (PROB-026); every calibration adopted (PROB-027).
- **Verified on the board:** 200.29 Hz, 0 dropped (TEST-064); ACK and STATUS `session`
  (TEST-064); a device reset ends the session, a recalibration after re-strapping is
  used (worn, TEST-065); 10 m walks over Wi-Fi at 200 Hz with 0 frames lost
  (TEST-058). Step counting on the seven video-synced fixtures: 3 errors over 176
  landings (TEST-059).
- **Walked worn on battery over Wi-Fi (TEST-066, TEST-067):** the full workflow ran,
  0 frames lost. Cues were faint and normal walking buzzed against the steady v5
  reference; DEC-025 answers both, to be confirmed on a walk.
- **No trustworthy reference profile exists yet.** Patient 67's v1–v3 (2026-09-18) were
  built while PROB-016 split strides; v4 (2026-10-05, TEST-066) is half free walking
  around a room. v5 (`20261005-151103-5aad`, the 10 m straight only) is the one to
  use; scored under DEC-025's floors, it needs no recapture.
- **Dashboard (`ead`, rebuilt 2026-10-07):** every doc 11 view except a
  separate HAPTICS page, plus Check; protocol schema 9, store schema 12. The database on
  this machine migrates to 12 when the app next opens (migration tested on a copy: 41
  status rows kept, integrity ok); backup `ead.sqlite3.schema11-backup-2026-10-07`
  beside it. Every CSV carries `patient_name`; `gait.csv` has `valid`. Session commands wait for the device's answer; stops drain to the device's
  boundary; save failures show in the state bar; a live session cannot be exported.

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
| `docs/decisions.md` | DEC-001–DEC-028 |
| `docs/problems.md` | PROB-001–PROB-042 (no PROB-008) |
| `docs/testing.md` | TEST-001–TEST-069, with measured results |
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
- Real-person sessions with patient 67 (2026-09-18), which produced PROB-013 to
  PROB-016.
- BNO086 on the bench, genuine part confirmed (TEST-039, TEST-040, 2026-09-23).
- Haptic band placement on the calf (DEC-015, 2026-09-23).
- Final pin map DEC-016, wiring reference, pad states measured on the chip
  (TEST-041), 2026-10-01.
- 2026-10-02: PROB-018 fixed and PROB-019 fixed (a device reboot ends the host
  session), both verified on hardware (TEST-042); `ead` command; both BNO086 proven with the bench
  driver (TEST-043); the BNO086 product firmware, SERVICE_TEST and `ead --check`
  (DEC-017, DEC-018, TEST-045–050).
- 2026-10-03/04: mount maps measured (TEST-051), 200 Hz feed (DEC-021), video-synced
  walks over Wi-Fi (TEST-058), shank-swing step detection (DEC-022), haptic feedback
  (DEC-023), schema 7 on the board (TEST-062).
- 2026-10-05: power budget estimate (`hardware.md`); the external audit checked
  claim by claim and fixed: PROB-026–035, DEC-024, TEST-063–065. First walks with
  feedback (TEST-066–068), DEC-025; the ERM driver PCB project into the repository.
- 2026-10-07: second external audit verified; the user's answers (DEC-026–028); fixes
  PROB-037–042; mirrored score constants tested against the firmware headers; root
  README.

## Current Work
Nothing in flight. The audit list's remaining items are under Next Steps.

## Known Problems
- **No trustworthy reference profile:** v1–v3 carry the PROB-016 fault.
- **Distance:** slow 10 m walks read about 6.2 and 7.0 m (PROB-024, open); the
  error-state filter is parked until floor-mark ground truth exists.
- **Shank swing axis** is 9–14° off anatomical Y (TEST-061); functional calibration
  deferred.
- **PROB-015:** inversion shifted about 15° after re-wearing; root cause unknown. A
  reference is only trustworthy within the wearing and calibration it was captured in;
  nothing checks that a reference matches the current mounting.
- **Per-cycle windows run late:** contact is decided up to ~0.15 s after the swing
  reverses; the angle is taken at the impact (PROB-028), but peaks, distance and ZUPT
  counts still run to the deciding sample.
- **`device_state` never reports REFERENCE_CAPTURE or RUNNING** (`device.cpp`
  `currentState`); the running session is STATUS byte 59 (schema 8).
- `EAD_GAIT_LOWPASS_HZ` (6 Hz) is reported in CONFIG_GET but not applied.
- **PROB-022 (open):** still on the desk, both gyroscopes show episodes of a few °/s; a
  2 s calibration was once rejected as "moved". Calibrate again.
- **Not yet fixed from the audits:** anyone on the access point can control the device
  (no pairing; left as is by the user's decision, DEC-024); the Rust dependency
  notices (wait on Tauri and krilla, PROB-036); `session.mat` built in memory.
  Second audit, open: the confidence gate cannot block a cue (F-02, floor 0.796 vs
  gate 0.75); velocity is not bounded when no zero-velocity window fires and those
  cycles stay valid (F-04); no reference quality gate or mounting check (F-05); shank
  gyro and rotation vectors resampled to the foot's frames (F-06); slow-walk distance,
  toe-off/stance/swing timing and thresholds unvalidated beyond one healthy wearer
  (F-08, F-09, F-15); worn accelerometer rate 224–238 Hz and ~0.5 ms frame-time jitter,
  causes unknown (F-14, F-32); pause/resume and on-device storage not built (F-17);
  cues at cycles under ~1 s on one motor can be refused by the 50 % limit (F-18, never
  observed); no part numbers in the PCB BOM (F-24); Windows/macOS setup never run
  (F-26). By the user's choice: OFF = ON (F-12), Wi-Fi control (F-40).

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
- Clocking BNO086 frames on the accelerometer: it runs at 125 Hz, not 100 (TEST-046).
- Converting replayed counts with `config_v1.h`'s constants: they describe the firmware
  being built, not the recording (TEST-050).
- `pkill -f ead-dashboard` from the agent's shell kills the shell itself; use a PID.
- Taking 800 ms without an ERROR as a session's acceptance (PROB-033): a late refusal
  or an unrelated error decided wrongly. Session commands now wait for ACK or ERROR.
- Sending backfill requests on a new connection before its HELLO is read: after a
  reset the old boot's gaps went to the new boot (TEST-065).
- `cargo fmt` on the dashboard: it reformats the whole crate (not rustfmt-formatted).
- Clearing the dashboard's held calibration record at each calibration start
  (PROB-037): a cancelled window keeps the device's old calibration.

## Important Decisions
DEC-005 USB link; DEC-006 no haptic code (superseded by DEC-023); DEC-007 raw =
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
laptop (schema 9); DEC-028 the patient's name in every exported file.

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
- **Product firmware (DEC-016 build):** `pio run -d firmware`,
  `pio run -d firmware -t upload`. Then `python3 tools/eadprobe.py check` (per-wire
  check; `--rerun` resets the sensors) and `eadprobe pulse 1 --duty 102` (one motor).
- Firmware unit tests: `pio test -d firmware -e native`.
- Check view: `ead --check`.
- Talk to the device (product firmware): `python3 tools/eadprobe.py hello`,
  `config`, `stats --seconds 60`, `calibrate --seconds 5`, `walk`, `capture`,
  `score`, `reopen`; add `--ws ws://192.168.4.1:8080/ws` for Wi-Fi.
- After any protocol change: `python3 protocol/vectors/generate.py`, both test
  suites, `python3 tools/eadprobe.py vectors`, and reflash (PROB-013).
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
1. **Evaluation walk (user, battery, Wi-Fi):** calibrate, evaluation against v5 with the
   switch on; normal lengths and deliberate deviations. Is a slight error felt at 60 %,
   a large one stronger? Then switch the laptop's Wi-Fi off mid-walk: the buzz should
   stop within 5 s and resume after reconnecting (`link_lost` records in haptics.csv).
2. **Second audit, design work:** a confidence measure that can block a cue (F-02);
   bounding velocity or invalidating distance when no zero-velocity window fires
   (F-04); a reference quality gate and mounting check (F-05). Each needs recorded walks
   to judge.
3. Distance (PROB-024) with floor-mark ground truth; functional calibration of the
   shank swing axis (TEST-061); event timing against the synced videos (F-09).
4. Dashboard gaps against doc 11: symmetry proxy on LIVE, RAW cycle/error filters,
   PAUSE/RESUME.
5. M7 storage DEC, then the code.

## Warnings
- **Only firmware on the DEC-016 pins goes on this board** (product firmware from
  `5b8dd28`, `firmware/bench/*`). Any firmware for it drives the enabled motor gates
  LOW first (a doubtful channel stays out of the enable mask), never asserts both chip
  selects, never starts UART0, and never puts a motor on GPIO39
  (`docs/wiring_reference.md` firmware rules).
- PROB-021: the user reports the power path fixed (2026-10-06; method not recorded).
  **The battery switch is still never ON while USB is plugged in** (user, 2026-10-05).
  A motor test that needs battery power runs over Wi-Fi with USB unplugged.
- `firmware/bench/padstate` briefly pulls each motor gate up at boot (0.3 ms).
- **The repository is public.** Never commit `firmware/include/ead_secrets.h`;
  never put the patient's name in docs (use the ID, `67`).
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- Calibration lives in device RAM only; recalibrate after every reset or flash.
- Record a test as passed only after running it, with its numbers.
