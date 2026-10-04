# Project Handoff

## Project
EAD V1: a right-leg, barefoot, wearable error-augmentation system for stroke
gait. A XIAO ESP32-S3 with two IMUs (foot + shank) and a six-motor haptic band,
plus a Tauri 2 research dashboard. Contract: `ead_agent_docs_v2/`
(authoritative, read-only).

## Current Objective
Bring the rebuilt device (two BNO086 on SPI, DEC-016) back to a validated gait
instrument: the firmware and the `ead --check` service view are done and verified
over USB; what remains needs the leg (mount maps, gait thresholds, a new reference)
and the user (motor pulses felt). The user wants engineered work only: no filler, no
placeholder or fake data, no features the spec does not need.

## Current State (2026-10-02)

- **Hardware.** The XIAO (MAC 44:B1:76:AF:FB:7C) is wired to DEC-016 with
  everything soldered: both BNO086, the separate ERM driver PCB
  (`~/Documents/ead pcb/`) with its motors and two HT7833 rails, battery
  (user, 2026-10-02). See `docs/hardware.md` "Current build".
- **Product firmware, schema 5** (`5b8dd28` and later), flashed on the board:
  our own SH-2 driver for both sensors (DEC-017), per-wire sensor check, 100 Hz
  gyro-clocked frames, motor service pulses (DEC-018). Verified over USB: every
  check step PASS on both sensors (TEST-047); 60 s at 100.142 Hz with 0 missing,
  0 dropped, 0 bus errors (TEST-046).
- **`ead --check`** opens the dashboard on the Check view: the per-wire table,
  live rates to tell foot from shank, per-motor pulses with a "felt" answer
  (TEST-049). All six motors are enabled (PROB-020 resolved).
- All six motors felt at 50 %, and the foot and shank cables confirmed not swapped
  (user, TEST-049).
- **Not yet measured with the BNO086 sensors:** mount maps (identity for now) and gait
  thresholds; both need the leg.
- **On the MPU6500 build, everything M0–M6 worked and was verified:** 100 Hz
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
- **Dashboard:** every doc 11 view except HAPTICS (the master switch is in the top
  bar; cues are stored and exported), plus Check. It reads sessions of
  both builds (configuration formats 1 and 2, store schema 10; the database on this
  machine was migrated with a backup beside it). Known gaps against doc 11 are under
  Next Steps.

## Architecture
See `docs/architecture.md`. In one line: INT-driven 100 Hz acquisition over SPI →
portable C++ processing (`lib/ead_core`) → PSRAM message ring → Wi-Fi WebSocket
and USB links → Rust backend with SQLite → React views and exports.

## Important Files
| File | Purpose |
|---|---|
| `ead_agent_docs_v2/` | Contract. Never modify |
| `docs/hardware.md` | Current DEC-016 build, previous MPU6500 build, mount maps, what is verified |
| `docs/wiring_reference.md` | Every DEC-016 connection with its evidence; firmware rules the wiring depends on |
| `docs/decisions.md` | DEC-001–DEC-018 |
| `docs/problems.md` | PROB-001–PROB-022 (no PROB-008) |
| `docs/testing.md` | TEST-001–TEST-050, with measured results |
| `docs/protocol.md` | Wire protocol (schema 5): payloads, framing, backfill, SERVICE_TEST, enumerations |
| `docs/clinical_requirements.md` | The researcher's four requirements vs what is built |
| `firmware/include/config_v1.h` | Fixed V1 constants, the DEC-016 pins with compile-time pin checks, BNO086 scales, mount maps |
| `firmware/lib/ead_core/` | Portable codec, COBS, CRC-32, ring, SH-2 codec, motor guard, calibration, Mahony, gait, reference, error engine |
| `firmware/src/` | Device services: BNO086 driver, acquisition, motors, links, calibration, orientation, gait, sessions |
| `firmware/bench/bno086/` | BNO086 bench test on the DEC-016 pins (SparkFun library, one sensor per build) |
| `firmware/bench/padstate/` | Pad-state and JTAG eFuse reader (TEST-041) |
| `protocol/vectors/` | 22 golden vectors (incl. the format-1 config section); `generate.py` rebuilds them |
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

## Current Work
Nothing in flight.

## Milestone Plan
1. **On the leg:** the mounting check to measure both BNO086 mount maps (as TEST-027),
   confirm foot vs shank, calibrate, then new ground-truth walks into `recordings/` and
   re-validate the gait thresholds by replay.
2. **Walking validation:** a new reference, a check, an evaluation, an export read end
   to end.
3. **Haptic feedback** (DEC-023): built and tested on the host (TEST-060); not yet
   flashed or felt. Bench: `eadprobe haptics on`, then an evaluation with
   `eadprobe score --evaluate --haptics`; then worn, with the dashboard's switch.
4. **M7**, on-device storage: needs a DEC (the default partition leaves 1.5 MB).

## Known Problems
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
- `EAD_GAIT_LOWPASS_HZ` (6 Hz) is reported in CONFIG_GET but not applied. The
  20 Hz event path is applied only to the ZUPT stillness test (DEC-019).
- **PROB-022 (open):** still on the desk, both gyroscopes show simultaneous episodes of
  a few °/s; a 2 s calibration was once rejected as "moved". Calibrate again.
- On the BNO086 build: shank repeats 1.9 % of frames and the frame period sd is
  200 µs (TEST-046); after a sensor reset one ~150 ms pause follows the first report.
- Unknown on the DEC-016 build: whether the §13 pre-power checks were done, and
  whether the cell has its own protection board.

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

## Important Decisions
DEC-005 USB link; DEC-006 no haptic code (superseded by DEC-023); DEC-007 raw =
chip-frame counts;
DEC-008 session kinds; DEC-009 per-sensor mount maps; DEC-010 `esp_http_server`;
DEC-011 dashboard stack; DEC-012 device-built reference; DEC-013 doc 06's open
values (provisional); DEC-014 host-side segments; DEC-015 haptic band on the
calf; DEC-016 final pin map, all 15 GPIOs used, GPIO39 never a motor; DEC-017 BNO086
firmware (own SH-2 driver, Mahony kept, calibrated reports as counts); DEC-018 motor
service-test pulses; DEC-019–021 soft footfalls, the user's decisions, the 200 Hz feed;
DEC-022 gait events from the shank's swing; DEC-023 haptic feedback during evaluations.

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
- `pio test -d firmware -e native` (98), `pio run -d firmware`,
  `pio run -d firmware/bench/bno086 -e foot -e shank`
- `cd dashboard/src-tauri && cargo test` (73, 4 ignored) and
  `cargo clippy --all-targets -- -D warnings`
- `cd dashboard && npm test` (28) and `npm run build`
- `python3 tools/eadprobe.py vectors` (26, 0 failures)

On hardware over USB, device still: `eadprobe check` (all PASS), `eadprobe stats
--seconds 60` (100.14 Hz, 0 missing), `cargo test -- --ignored --test-threads=1` (both
hardware tests; the second resets the board).

## Next Steps
0. Schema 6 (DEC-021): 200 Hz frames with the BNO086's own orientation and a native
   250 Hz accelerometer stream; store schema 8; ZUPT gyroscope limit 30 °/s (DEC-020).
   Verified on the board over USB on the desk (TEST-055) and on the fixtures by replay
   (TEST-056); on the leg over Wi-Fi, barefoot, with video (TEST-058): 0 frames lost.
   Seven video-synced fixtures in `recordings/*-2026-10-04.eadlog`; the replay needs
   `--still-from` for them (README). Contact detection splits strides there
   (PROB-024). Step detection now comes from the shank's swing (DEC-022, TEST-059):
   counting error 21 → 3 over 176 landings on the fixtures; not yet flashed or
   walked. Score any change with `python3 tools/replay/walks.py /tmp/eadreplay`.
1. On the leg: mount maps measured and checked (TEST-051). First 10 m walks
   (TEST-052) found PROB-023, fixed in `gait.cpp` and flashed (0.1.0+77ae282).
   PROB-024: contacts fixed (DEC-019, TEST-053); open are slow strides reading
   30 % short and five stances with no zero-velocity update (the 25 °/s limit
   is the user's decision). Check any detector change with
   `tools/replay/walks.py`; a dashboard session becomes a fixture with
   `tools/session2eadlog.py`.
2. Dashboard gaps against doc 11: LIVE metrics, the seven TRENDS, CYCLES → RAW and
   the haptic column and lane are done (implementation.md, not yet seen on screen).
   Left: symmetry proxy on LIVE, RAW cycle/error filters, PAUSE/RESUME. The "roll the
   sole inward" step is dropped: the toes and still steps fix the foot's Y and Z,
   and a mount map must be a proper rotation (`static_assert` in config_v1.h), so X
   follows; a sole roll could not fail where the other two pass.
3. M7 storage DEC, then the code.

## Warnings
- **Only firmware on the DEC-016 pins goes on this board** (product firmware from
  `5b8dd28`, `firmware/bench/*`). Any firmware for it drives the enabled motor gates
  LOW first (a doubtful channel stays out of the enable mask), never asserts both chip
  selects, never starts UART0, and never puts a motor on GPIO39
  (`docs/wiring_reference.md` firmware rules).
- The master switch output feeds the XIAO's 5V pin (PROB-021). **The battery switch is
  never ON while USB is plugged in** (user, 2026-10-05). A motor test that needs battery
  power runs over Wi-Fi with USB unplugged.
- `firmware/bench/padstate` briefly pulls each motor gate up at boot (0.3 ms).
- **The repository is public.** Never commit `firmware/include/ead_secrets.h`;
  never put the patient's name in docs (use the ID, `67`).
- Never modify `ead_agent_docs_v2/`. Contract values override library defaults.
- Calibration lives in device RAM only; recalibrate after every reset or flash.
- Record a test as passed only after running it, with its numbers.
