# Engineering Progress

## 2026-09-16 — Engineering docs + git scaffolding

### Objective
Create `docs/` engineering records (README, architecture, implementation,
decisions, problems, testing, progress, handoff) plus `.gitignore`
scaffolding, without touching `ead_agent_docs_v2/`, and without running
`git init` (main agent handles it).

### Investigation
Read `AGENTS.md` doc-structure rules (§2, §4–§9, §19–§20) and the contract
docs `ead_agent_docs_v2/00_README.md` (V1 identity), `14_IMPLEMENTATION_PLAN.md`
(Phase 1–12), `15_DECISION_LOG_AND_TRACEABILITY.md` (fixed decisions), plus
`01_SYSTEM_SPEC.md`, `02_HARDWARE_WIRING.md`, `07_FIRMWARE_ARCHITECTURE.md`,
and `13_TEST_AND_VALIDATION_PLAN.md` for architecture/testing content.
Listed the workdir: only `ead_agent_docs_v2/` and one image existed at task
start; `firmware/` with `platformio.ini` (+ `src/include/lib/test`) appeared
during the task via parallel main-agent scaffolding — read `platformio.ini`
to report Phase 1 status accurately.

### Approach
Wrote the eight `docs/` files to the AGENTS.md formats, grounding all numeric
claims (addresses, rates, PWM limits, IPs) in the contract docs. Recorded the
four requested decisions (DEC-001–DEC-004) with context/options/reason/
trade-offs. Left `problems.md` as an empty template, `testing.md` as planned
(not-run) tests. Added `.gitignore` for PlatformIO/Tauri/`.ead`/`dist` outputs.

### Changes
Added:
- `docs/README.md`
- `docs/architecture.md`
- `docs/implementation.md`
- `docs/decisions.md`
- `docs/problems.md`
- `docs/testing.md`
- `docs/progress.md` (this file)
- `docs/handoff.md`
- `.gitignore`

Modified: none. Touched `ead_agent_docs_v2/`: no.

### Problems
None. No build or test executed in this task.

### Diagnosis
N/A.

### Solution
N/A.

### Verification
Listed created files; confirmed `ead_agent_docs_v2/` unmodified. No code
compiled, no tests run — `testing.md` entries explicitly marked NOT RUN.

### Current Status
Completed (docs + ignore scaffolding). Implementation remains at Phase 1
skeleton; dashboard skeleton and all Phase 2+ code outstanding.

### Next Steps
1. ~~Main agent runs `git init` + initial commit.~~ DONE 2026-09-17: repo at https://github.com/sagar-aryan/ead-v1, branch main.
2. ~~Verify `pio run` for `firmware/` (Phase 1 build check).~~ DONE: SUCCESS (309s, RAM 5.7%, Flash 8.0%).
3. ~~Scaffold Tauri 2 dashboard project per doc 14 Phase 9.~~ DONE: `npm run build` PASS (tsc + vite, 30 modules).
4. Begin Phase 2 sensor layer with replay fixtures.

## 2026-09-17 — Phase 1 skeletons verified + repo live

### Objective
Parallel-scaffold firmware + dashboard + docs, verify builds, push private repo.

### Investigation
pio missing (installed 6.2.0 via pip --break-system-packages); node 24, rustc 1.95, gh authed as sagar-aryan.

### Approach
3 parallel subagents scaffolded firmware / dashboard / docs; main installed pio, built, init git, pushed private repo, npm-built dashboard.

### Changes
Added firmware/, dashboard/, docs/, .gitignore. Verified, committed 378469c.

### Verification
- `pio run -d firmware`: SUCCESS, elf->bin, RAM 18740/327680, Flash 266301/3342336.
- `npm run build` in dashboard: tsc + vite SUCCESS.
- Remote: https://github.com/sagar-aryan/ead-v1 (private, main).

### Current Status
Phase 1 complete. Next: Phase 2 sensor layer (MPU6050 drivers, INT, 100Hz sync, cal).

## 2026-09-17 — Phase 2 bring-up firmware and orientation tester (reconstructed)

Reconstructed on 2026-09-17 from git history (`e89f1c2`, `58c418c`, `eab57d7`,
`d052a9c`, `6230d34`) and `docs/problems.md`; no progress entry was written at the time.

### Objective
First hardware bring-up: read both IMUs, exercise the motor outputs, and check sensor
mounting visually.

### Changes
- `firmware/src/main.cpp`: register-level IMU init, `millis()`-polled ~100 Hz reads,
  10 Hz text output (`F[..]`, `RAW,…`, `CSV,…`), `m` motor test (analogWrite 30 %),
  `c` register diagnostics.
- `tools/orient_viewer.py` + `tools/README_ORIENT.md`: matplotlib 3D viewer of the
  `CSV,` stream with a five-pose check.
- `firmware/include/config_v1.h`: shank axis remap macros.

### Problems
- PROB-001: accel read ~2 g at rest; the boards are MPU6500 (WHO 0x70) and init had
  returned early. Resolved in `d052a9c`.
- PROB-002: shank mapping; a candidate map was committed while still investigating.

### Current Status
Superseded by the 2026-09-17 audit and M0 entries below.

## 2026-09-17 — Project audit and V1 build plan

### Objective
Understand the whole project (contract docs, engineering docs, firmware, dashboard,
tools), agree open questions with the user, and plan the build to a fully functional
research dashboard.

### Investigation
- Read all of `ead_agent_docs_v2/`, `docs/`, firmware, dashboard and tools.
- Checked toolchains: node 24.13.1, npm 11.8.0, rustc 1.95.0, PlatformIO 6.2.0,
  Python 3.12.3 with scipy 1.11.4, WebKitGTK 4.1, libudev.
- Confirmed the device on `/dev/ttyACM0`.
- Verified against the installed Arduino-ESP32 2.0.17 sources:
  - `attachInterrupt` is not IRAM-safe;
  - `WiFiClient::write` retries 10 × 1 s;
  - `esp_http_server` WebSocket support is compiled in;
  - `HWCDC::write` timeout logic;
  - the board definition enables PSRAM.
- Ran an independent design review against the contract and cross-checked its
  source-level claims.

### Findings
Shank map is a reflection (PROB-002). MPU6500 accel DLPF not configured (PROB-003).
GPIO43 boot-time risk (PROB-004). Motor pins driven LOW only after a 1.5 s USB wait.
Tauri Rust side never compiled. Contract docs untracked in git. Stale docs.

### Decisions with the user
USB fallback link (DEC-005); no haptic code (DEC-006); shank chip +Z toward the bone;
device is battery-powered and wearable.

### Approach
Milestones M0 (baseline fixes) → M1 (acquisition, protocol, both links) → M2
(dashboard foundation, user UI review) → M3 (calibration, orientation, mounting
check, datasets) → M4 (gait + ZUPT) → M5 (reference, error engine, workflow) →
M6 (exports); M7 flash storage later. Each milestone: verified, documented,
committed locally.

### Current Status
Completed.

## 2026-09-17 — M0 baseline fixes and cleanup

### Objective
Fix verified defects and make every component build before new features.

### Changes
Modified:
- `firmware/include/config_v1.h` — mount maps as matrices with compile-time proper-rotation checks; shank Y sign fixed.
- `firmware/src/main.cpp` — motor GPIOs LOW first; ACCEL_CONFIG2 write; full config readback; `m` motor test and PWM frequency call removed.
- `firmware/platformio.ini` — platform pinned to 7.1.3, gnu++17, unused libraries and duplicated USB flags removed.
- `dashboard/src-tauri/Cargo.toml`, `src/main.rs`, `tauri.conf.json` — shell plugin and placeholder commands removed, strict CSP, icon list.
- `.gitignore` — `__pycache__/`, `dashboard/src-tauri/gen/`.
- `docs/decisions.md`, `problems.md`, `testing.md`, `progress.md`, `architecture.md`, `implementation.md`, `handoff.md`, `README.md`; `firmware/README.md`, `dashboard/README.md`, `tools/README_ORIENT.md`.

Added:
- `docs/hardware.md`
- `dashboard/app-icon.svg` and generated desktop icons
- `dashboard/src-tauri/capabilities/default.json`
- `dashboard/src-tauri/Cargo.lock`
- `.claude/skills/frontend-design`, `.claude/skills/vercel-react-best-practices`, `skills-lock.json`

Committed separately first (`fdc4a69`): `ead_agent_docs_v2/`, `dashboard/package-lock.json`.

### Problems
None blocking. The on-body standing check (TEST-014) needs the user to wear the device.

### Verification
TEST-008 to TEST-013 passed. Details in `docs/testing.md`.

### Current Status
Completed except TEST-014 (on-body gravity check), which needs the user.

### Next Steps
M1: interrupt-count test on GPIO7/8, then DRDY acquisition, protocol and both links.

## 2026-09-17 — M1 firmware: acquisition, protocol and both links

### Objective
Replace the polled bring-up loop with a real data path: data-ready-clocked
100 Hz acquisition, the doc-08 binary protocol over Wi-Fi and USB, and recovery
of telemetry lost in transit.

### Investigation
Started with the gating question: are the data-ready lines even wired? A 20 s
interrupt-count test measured 100.143 Hz (foot) and 100.000 Hz (shank), so both
are connected — and the 0.14 % difference between the two sensors' oscillators
became a design input (TEST-015).

Read the installed Arduino-ESP32 2.0.17 sources rather than trusting assumptions:
`attachInterrupt` is not IRAM-safe in this build, `WiFiClient::write` retries
10 × 1 s (so links2004 WebSockets can stall a sender for ~10 s), and ESP-IDF's
`esp_http_server` WebSocket support is already compiled in.

### Approach
`lib/ead_core` holds everything portable (codec, COBS, CRC-32, message ring,
configuration section) and builds for both the device and the host, so the same
code is unit-tested off-target. Golden vectors are generated by a Python script
from `docs/protocol.md` and the contract JSON, then checked by the firmware
tests, the Rust dashboard and `tools/eadprobe.py` independently.

### Changes
Added: `firmware/lib/ead_core/`, `firmware/src/{imu,acquisition,telemetry,device,link,link_usb,link_wifi}.*`,
`firmware/scripts/generate_headers.py`, `firmware/test/test_{crc_cobs,protocol,msg_ring}/`,
`protocol/vectors/`, `docs/protocol.md`, `tools/eadprobe.py`.
Removed: `firmware/lib/ead_codec/` (folded into ead_core), the 10 Hz text CSV
output, and `tools/orient_viewer.py` with its guide — the viewer read that text
stream, and the dashboard's Live view replaces it.

### Problems
PROB-005 (opening the USB port reset the device), PROB-006 (USB frames corrupted
by Arduino core logging sharing the endpoint), PROB-007 (I²C reads failing when
started on the data-ready edge). All three resolved; see `docs/problems.md` for
the diagnosis of each.

### Verification
TEST-015 to TEST-021. The acceptance run (TEST-018) streamed 180,250 of 180,250
frames over 30 minutes with zero missing, zero dropped, zero corrupted frames and
zero I²C errors, at a mean period of 9985.5 µs (σ 0.5 µs).

### Current Status
Complete. The Wi-Fi link is verified as far as this machine can go without
leaving the network: the access point advertises correctly (TEST-019), and the
WebSocket path is exercised by the same code as USB. An end-to-end Wi-Fi test is
for the user to run.

### Next Steps
M2 dashboard foundation.

## 2026-09-17 — M2 dashboard foundation

### Objective
A dashboard that connects to the device, shows what the sensors are doing, and
records datasets that can be trusted.

### Approach
The Rust backend re-implements the protocol independently of the firmware; both
are held to the same golden vectors, so a mistake has to be made twice in two
languages to reach the data. The device manager fetches the device's own
configuration and verifies it against the hash the device reports, so every
recording carries the exact firmware and settings that produced it.

Raw frames go to SQLite through a single writer thread, batched into transactions
every 250 ms, with `INSERT OR IGNORE` so backfilled frames cannot duplicate live
ones. Live updates are aggregated to 20 Hz for the UI (doc 11 §3) while the store
keeps every 100 Hz sample.

The UI is built as an instrument panel: one loud element (the state bar), flat
panels, measurements in tabular mono numerals, and a persistent colour identity
for the two sensors. Only the three views whose measurements exist are present —
Live, Sessions, Device. The chart palette was validated with the dataviz skill's
checker (foot/shank ΔE 24.7 protan on this surface).

### Changes
Added: `dashboard/src-tauri/src/{protocol,link,store,device,live,app,hardware_tests}`,
`dashboard/src/{api,useDevice,styles}.*`, `dashboard/src/components/`,
`dashboard/src/views/`. Removed the placeholder `dashboard/src/types.ts` and the
demo data in `App.tsx`.

### Problems
Two bugs were caught by tooling rather than by chance: the golden vectors
rejected a backfill parser that assumed one message per buffer, and a dead-code
warning revealed that USB commands were never COBS-framed. A third was found by
looking at the running app: the first live updates arrive before the device
configuration, so they carry raw counts; the chart history now resets when units
change.

### Verification
23 Rust tests plus a hardware test (TEST-022) and the application itself running
against the device (TEST-023).

### Current Status
In progress. The RAW view is still outstanding; everything else in the M2 scope
works against real hardware.

### Next Steps
Build the RAW view over stored frames, then hand the dashboard to the user for
the look-and-feel review the plan calls for before M3.

## 2026-09-17 — M2 complete: raw-data view

### Objective
Read recorded sessions back: whole-session overview, zoom to individual samples,
in physical units.

### Approach
One decimated query groups frames into buckets and reports each bucket's minimum
and maximum, so a single-frame impact survives decimation; below 4,000 frames the
rows are returned exactly.

Measured before building machinery: a full-session query over an hour takes
309 ms and zoomed views 42–89 ms (TEST-024), so the planned summary tables were
not built. They would have cost write-path work and a rebuild after every
backfill to save a third of a second once.

### Changes
- `store/raw.rs`: decimated window query and the counts-to-anatomical conversion.
- `store/schema.rs`: schema 2 adds `sessions.config_section`, so a recording of
  raw counts is self-describing; migration from schema 1 tested.
- `views/Raw.tsx`, `api.ts`, `app.rs`, `main.rs`.

### Problems
Two defects found by looking at the rendered chart rather than by testing:
1. The x axis was drawn as wall-clock dates from the epoch, because uPlot treats
   the x scale as time by default. Elapsed seconds now declared explicitly.
2. The counts-to-units conversion lived in the frontend, where the project has no
   test runner, so the case that matters — a negative mount-map sign swapping a
   bucket's minimum and maximum — could not be tested. Moved into Rust and tested.
   The view is now a renderer with no domain logic.

Also encountered: GUI automation under XWayland needs XTEST rather than synthetic
events, and the client-area origin from `xwininfo` rather than the frame origin.
Recorded in the handoff, since it cost an hour.

### Verification
30 Rust tests. TEST-024 (query time) and TEST-025 (the view over 30 minutes of
real recorded data, cross-checked against SQL).

### Current Status
M2 complete.

### Next Steps
M3, starting with the guided mounting check so PROB-002 closes first.

## 2026-09-17 — Raw view: timeline navigation and simultaneous signals

### Objective
The user, looking at the raw view zoomed to 101 frames at t ≈ 899.5 s, asked how
a researcher scrolls the timeline, and that all the data be visible at once.

### Investigation
Confirmed: the view had only "Zoom in" (always to the centre) and "Whole
session". There was no way to move along a recording, and only one signal group
was drawn at a time, so a foot impact could not be compared with the shank at
the same instant.

### Approach
- Focus plus context: whole-session overview strip with the window marked; drag
  to select on any chart; pan and zoom buttons plus arrow/+/− keys.
- All four signal groups stacked by default, toggled by checkbox, shared time
  axis, synced cursor.
- Window arithmetic extracted to `timeline.ts` and tested with `node --test`,
  because the M2 lesson was that untested frontend logic is where bugs hide.

### Changes
- Added: `dashboard/src/timeline.ts`, `dashboard/src/timeline.test.ts`.
- Modified: `views/Raw.tsx`, `styles.css`, `api.ts`, `tsconfig.json`
  (`allowImportingTsExtensions`), `package.json` (`test` script,
  `@types/node` dev dependency).
- Modified: `store/raw.rs` (one query for several groups, `RawSignal`),
  `store/mod.rs`, `app.rs`, `store/tests.rs`, `hardware_tests.rs`.

### Problems
1. First version issued one query per signal. Measured on an hour of data:
   542 ms for four signals on the whole session. All four read the same rows, so
   they were merged into one pass: 291 ms, zoomed 78 → 35 ms (TEST-026).
2. `cargo clippy -- -D warnings` failed on two M2 lines in `raw.rs`
   (`needless_lifetimes`, `needless_range_loop`). M2's "zero warnings" was from
   `cargo build`, not clippy. Fixed.
3. The running app is now a native Wayland window, so the XTEST/xwd screenshot
   route used in M2 no longer reaches it, and Xvfb is not installed. The new view
   has not been looked at by the agent; the user is asked to check it.

### Verification
`npm test` 10 pass; `npm run build` clean; `cargo test` 31 pass, 2 ignored;
`cargo clippy --all-targets -D warnings` clean; TEST-026 measured.
Not verified: the rendered view (see Problem 3).

### Current Status
Implemented; visual check pending with the user.

### Follow-up after the user's check
- Every chart now prints its own time axis. The first version printed it only
  under the bottom chart to save vertical space, which made the upper charts
  unreadable in isolation.
- A dropdown for choosing how many graphs to show was started and abandoned on
  the user's instruction: the checkbox row is what they want.

### Next Steps
Guided mounting check (M3).

## 2026-09-17 — M3: guided mounting check

### Objective
Settle PROB-002 (the shank mount map) with a measurement the user can perform,
rather than by asking them to read axis values aloud.

### Approach
Three guided moves judged in the dashboard against the anatomical frame: stand
still (gravity must read +Z on both sensors), raise the toes (foot must turn
about −Y), seated knee extension (shank must turn about −Y). No firmware change:
the live stream already carries anatomical accelerations and rates.

### Changes
- Added `dashboard/src/mounting.ts` (verdicts, thresholds) and its tests.
- Added `dashboard/src/views/MountingCheck.tsx`; rendered from the Device view.
- `styles.css`: a verdict style whose colour only reinforces the word.

### Problems
The first version collected samples from the live tick React state, which
`useDevice` throttles to 5 Hz for the numeric readouts — fast enough for the
still step's mean but not to catch the peak of a short movement. The rotation
steps now read the 20 Hz rings instead.

### Verification
`npm test` 21 pass (11 new); `npm run build` clean. Not run on hardware: it needs
the device worn on the leg, which is the user's step.

### Current Status
Implemented, awaiting a run on the leg (TEST-027).

### Next Steps
User runs the check; the result decides whether PROB-002 closes or the shank map
changes. Then static calibration and Mahony orientation.

## 2026-09-17 — TEST-027: the mounting check finds the shank map error

### Objective
Run the mounting check on the leg and act on the result.

### Investigation
All three steps failed. Two shank measurements agreed: standing still put gravity
on anatomical Y (+0.99 g) and a seated knee extension turned about anatomical Z
(+43 °/s), i.e. anatomical Y and Z were interchanged. The foot read
(−0.43, +0.35, +0.86) g: gravity dominant on +Z with the board tilted 33° on the
instep — the map is right, the check was too strict. The foot rotation step
captured only 4 °/s, so it says nothing yet.

### Approach
Correct the measurement rather than re-derive from assumptions: with
`Z_true = Y_measured` and `Y_true = −Z_measured`, the new shank map is
`X = −chipZ, Y = +chipY, Z = +chipX`, still a proper rotation. The assumption
that failed in the earlier derivation was that chip +Y points down the leg; the
board is strapped with chip +X up the leg.

### Changes
- `firmware/include/config_v1.h`: new shank map with the derivation from measured
  values.
- `protocol/vectors/generate.py` + regenerated vectors; `protocol/tests.rs`
  expectations.
- `dashboard/src/mounting.ts`: the still step now checks axis and sign and
  reports the tilt angle (`tiltDegrees`), instead of demanding near-zero tilt.

### Verification
Firmware 20/20 native tests, Rust 31 pass, frontend 23 pass, all builds clean.
Flashed, and the device reports the new map over USB
(`eadprobe config`: `shank_mount [0,0,-1, 0,1,0, 1,0,0]`).

### Current Status
Resolved. The re-run on the leg passed all three steps (user-reported), so
PROB-002 is closed and both mount maps are confirmed against the hardware.

### Next Steps
Static calibration: gyro bias and gravity alignment from five seconds of
stillness, which is what removes the 33° instep tilt from the measurements.

## 2026-09-18 — Static calibration on the device (schema 2)

### Objective
Measure each gyroscope's resting bias and the direction of gravity, so drift and
mounting tilt can be removed from everything that follows.

### Approach
On the device, not the host: the firmware already has every sample, and the
orientation estimator that will consume the record runs there too. Computing it
on the host would mean writing it twice and keeping two implementations
bit-identical.

The samples are the frames already being acquired, so calibration adds no I2C
traffic and does not interrupt streaming. This is the documented path
(SESSION_START kind CALIBRATION), so no new message types were invented; it does
require payload schema 2, which is what `docs/protocol.md` said session control
would need.

### Changes
- `ead_core`: `calibration.{h,cpp}` — accumulator, rejection rules, and the
  quaternion that takes measured gravity to anatomical +Z. Seven native tests.
- `protocol.{h,cpp}`: schema 2 — STATUS gains calibration state/samples/reject,
  plus SESSION_START and the 128-byte record payload.
- `firmware/src/calibration_service.{h,cpp}`: runs the window, guarded by a
  critical section because frames arrive on one task and the record is read by
  another. `link.cpp` handles the commands and emits the record.
- `protocol/vectors`: `session_start.hex`, `calibration_record.hex`; Rust and
  firmware both test against them.
- Dashboard: `protocol/mod.rs` parsing, `device.rs` state, two Tauri commands,
  and a Calibration panel in the Device view.
- `tools/eadprobe.py`: `calibrate` subcommand.

### Problems
1. The first hardware run returned a record for the *previous* window: a
   completion that finished while no host was listening was delivered to the
   next host to connect, which then read it as the answer to its own request.
   The device now discards an unsent completion when a host says HELLO; STATUS
   still reports that a record is held, so nothing is hidden.
2. Then no record arrived at all for windows longer than three seconds. The
   device stops streaming to a host that has been quiet for 3 s (by design), and
   `eadprobe`'s request helper sent nothing while waiting. The probe now sends
   the 1 Hz keepalive the protocol requires while a window runs. The dashboard
   was never affected: it keepalives already.

### Verification
TEST-028 on hardware: 3 s → 300 samples, 8 s → 800 samples, bias repeatable
within 0.03 °/s across runs, a disturbed window rejected as `moved`.
Firmware 29 native tests, Rust 34, frontend 23; clippy clean.

### Current Status
Calibration works end to end over USB and is visible in the dashboard, which the
user has not yet looked at.

### Next Steps
Mahony orientation on the device using the record, then the orientation view.

## 2026-09-18 — Mahony orientation on the device

### Objective
Turn the two sensors into foot and shank orientations, and the pair into ankle
angles, using the calibration record.

### Approach
Doc 04 §5 to the letter: a 6-DoF Mahony estimator per IMU at 100 Hz, Kp 2.0,
Ki 0.05, gravity correcting roll and pitch, yaw gyro-integrated with no heading
claimed, and the relative orientation `inverse(q_shank) * q_foot` as the
kinematic quantity (doc 04 §6). The estimator starts from the alignment the
calibration measured rather than from identity, so a recording does not begin
with seconds of convergence.

Two refusals are built in, because a plausible-looking orientation is worse than
none: without a calibration record the quaternions stay identity and the new
`orientation_valid` status bit stays clear; a frame whose interval is outside
half to twice the nominal period restarts the estimator instead of integrating
across the gap.

### Changes
- `ead_core/mahony.{h,cpp}` + 7 native tests.
- `firmware/src/orientation.{h,cpp}`, run on the processing task;
  `src/anatomical.h` now holds the one chip-to-anatomical conversion, which
  `calibration_service.cpp` also uses (it had its own copy).
- `protocol.h`, `docs/protocol.md`: `orientation_valid`, bit 8 of the frame status.
- Dashboard: `src-tauri/src/orientation.rs` (relative orientation and the three
  ankle angles, 7 tests), orientation in `LiveTick`, and an `Orientation` panel
  drawing the sagittal view on a canvas.

### Problems
Verification on hardware is blocked on the device's physical position, not on
code: with the boards lying as they are on the bench the shank sensor is 73° off
upright, so calibration rejects the window as `upside_down` and orientation
correctly stays unavailable. Both refusals were observed working
(0/410 frames valid without a record).

### Verification
Firmware 36 native tests, Rust 41, frontend 23; clippy clean; all four builds
clean. Orientation itself is not yet verified against the hardware.

### Current Status
Verified on the leg after one fix. The first on-body run exposed PROB-010: the
calibration's alignment was used as the estimator's starting attitude but never
applied to the measurements, so the estimator described the board rather than the
segment and the 40.7° instep mounting appeared as a permanent ankle angle. With
the alignment applied to the acceleration and angular rate, a flat foot reads
−0.11°.

### Next Steps
M4: gait events and ZUPT.

Orientation is confirmed on the leg: four toe raises peaked at +24.9°, +25.9°,
+26.5° and +26.7°, repeating within 0.2°, with no accumulation between lifts.

## 2026-09-18 — M4: gait events, ZUPT and cycle distance

### Objective
Detect gait cycles and estimate stride distance, then check both against a
measured course.

### Approach
Doc 05 in `ead_core/gait.{h,cpp}`: the seven-state machine, initial contact
timestamped at the strongest impact inside its window, toe-off after sustained
rotation, zero-velocity windows at the document's thresholds, per-cycle features,
and a velocity error-state update rather than a hard reset. Schema 3 carries the
result as EVENT_BATCH and STEP_BATCH, both durable.

Then a recording, because thresholds cannot be guessed: `tools/replay` runs the
device's own code over a `.eadlog` on the host, so a change can be tested against
real signals in seconds instead of asking for another walk.

### Problems
Four, all found by measurement (TEST-030, PROB-011, PROB-012):

1. Every impact opened a cycle — 20 contacts for 12 steps. A footfall produces
   several impacts; contacts within one minimum cycle are now the same footfall.
2. Zero-velocity was never detected on a real walk. PRE_SWING had been left out
   of the states allowed to hold a window, and a single moving sample during
   stance locked the detector out for the whole cycle.
3. Push-off was being read as the next footfall, splitting every stride into a
   1.0 s "mostly stance" cycle and a 0.6 s "mostly swing" one. Measured, heel
   strikes peak at 2.3–5.3 g and push-off at 1.5–1.9 g, so a confirm threshold
   at 1.1 g above rest separates them. Doc 05's suggested test — angular speed
   decreasing toward contact — does not hold for this mounting: the foot's rate
   peaks within 20 ms of heel strike.
4. One real contact was rejected for lasting 29.96 ms against a 30 ms
   requirement: three samples at the device's actual 100.147 Hz. An impact above
   the confirm level is now decisive regardless of duration.

### Verification
TEST-030: 6 cycles for 6 strides, contact times matching the ground truth
exactly, total distance 6.39 m against a 6.00 m course (+6.5 %), ZUPT quality
0.22–0.29, stance ratio 0.50–0.60. Firmware 47 native tests, Rust 44.

### Current Status
The gait engine works on real walking. The firmware on the device is one version
behind: it needs a USB connection to flash.

### Next Steps
1. Flash the device with the corrected detector and confirm live (needs USB).
2. More walks, at different speeds, before trusting the thresholds (they are
   fitted to one recording).

## 2026-09-18 — Gait cycles in the dashboard

### Objective
Store what the device measures per cycle and show it.

### Changes
- Schema 3: `cycles` and `events` tables, with a tested migration from 2.
- `Sink::gait` carries EVENT_BATCH and STEP_BATCH from the link into the store.
- `Cycles` view: session summary, per-cycle table, and eight trends.
- Live view shows the device's gait state and cycle count from STATUS.

### Verification
47 Rust tests (3 new), 23 frontend tests, clippy clean, frontend builds.
Not yet seen with real cycles in the database: that needs the corrected firmware
flashed and a recorded walk.

### Current Status
Complete as far as it can go without the device on USB.

### Next Steps
Flash, record a walk through the dashboard, and read the Cycles view.

## 2026-09-18 — M5: reference profiles and the error engine

### Objective
Build the patient-specific reference and the per-cycle error score, classes and
confidence that doc 06 specifies.

### Approach
Portable core first, in `ead_core`, because DEC-012 puts reference building on
the device and the same code has to run in the host replay. `reference.{h,cpp}`
holds the feature table, the weights and the median/MAD statistics;
`error_engine.{h,cpp}` holds the scoring, the six classes and the five
confidence subscores.

Doc 06 leaves four values undefined — the spread floors, the reference-stability
subscore, what "no single class dominates" means, and which features may be
missing. They are chosen, justified and recorded in DEC-013 rather than buried.

### Verification
TEST-031: 11 native cases against hand-computed answers, including the ones that
matter for honesty — a reference refuses to build from 29 cycles, a stumble does
not move the median, a flat feature cannot divide by zero, and a cycle with no
zero-velocity window loses its distance feature instead of scoring it as perfect.

Firmware 58 native tests.

### Current Status
The core is done and tested. Not yet wired: the session kinds that capture and
check a reference (schema 4), storage and locking in the dashboard, and the
REFERENCES view.

### Next Steps
Schema 4 (session kinds carrying a reference, error fields in STEP_BATCH), then
the dashboard side.

## 2026-09-18 — M5 user interface: references, errors, events, segments

### Objective
Finish M5: the dashboard side of the reference workflow, the error engine's
output where a reader can see it, and the doc 12 session gate and segmentation.
The core and the protocol were done the same day (schema 4, commit `adc059d`);
none of it was visible in the UI.

### Investigation
The backend already had everything the views needed: `references`,
`add_reference`, `lock_reference`, `parse_reference`, the error columns on
`cycles`, and `FEATURE_NAMES` / `ERROR_CLASSES` in the vocabulary. Two things
were missing.

First, a capture's progress. The device does not report its session kind or its
builder's cycle count in STATUS, so the UI had no way to show progress toward
the thirty-cycle gate. Rather than a fifth protocol schema for a counter, the
host now records which session it started and counts valid cycles out of the
STEP_BATCH stream it is already decoding. The snapshot says so in its field
comment: it is what this host started, not an echo from the device.

Second, segments did not exist at all — no table, no limits on the session, no
rollover. DEC-004 had planned firmware enforcement.

### Approach
Segmentation moved to the host (DEC-014). What made the decision was asking what
the device does differently at a segment boundary in V1: nothing. No haptics, no
flash storage, no state change. Firmware enforcement would have cost protocol
schema 5 — two fields in SESSION_START, a segment index in every cycle record,
regenerated vectors, four codebases — for a counter. The host applies the rule to
the stored cycle stream instead, which is also more reproducible.

Capture and check now run inside a recording session, so the walk that built a
profile is still on disk and can be replayed against a corrected detector. That
also gave the check its readout for free: it reads back the cycles the store
already has.

The session gate is computed in the backend (`session_blockers`) and only
displayed by the UI, so the button and the command cannot disagree about whether
a session may start.

### Changes
Added:
- `dashboard/src/views/References.tsx` — capture with progress against the
  thirty-cycle gate, version list with lock state, per-feature median and
  spread, and the ten-cycle check with median error and per-feature deviation.
- `dashboard/src/views/Events.tsx` — Canvas2D lanes for initial contact, toe
  off, foot flat, zero-velocity bands and cycle bars, with drag to zoom.
- `dashboard/src/events.ts`, `dashboard/src/events.test.ts` — ZUPT pairing,
  5 tests (TEST-033).

Modified:
- `dashboard/src-tauri/src/store/schema.rs` — schema 5: `segments`,
  `cycles.segment_index`, and `reference_id` plus both limits on `sessions`.
- `dashboard/src-tauri/src/store/mod.rs` — `SessionKind` gains
  `ReferenceCapture`, `ReferenceCheck`, `Evaluation`; `SegmentLimits`,
  `StoredSegment`, `counts_as_error`, `roll_segment`, `segments()`.
- `dashboard/src-tauri/src/app.rs` — `session_blockers`,
  `start_reference_capture`, `finish_reference_capture`, `start_scored_session`,
  `stop_scored_session`, `segments`.
- `dashboard/src-tauri/src/device.rs` — records the session this host started
  and counts valid cycles in it.
- `dashboard/src/views/Cycles.tsx` — score, confidence, class and segment
  columns; a class distribution; the segments table; error trends.
- `dashboard/src/views/Sessions.tsx` — the doc 12 §4 evaluation gate.
- `dashboard/src/App.tsx`, `api.ts`, `styles.css`.

### Problems
The migration test simulates a v2 store by dropping what later schemas added;
schema 5 added columns to `sessions`, so the test failed with `duplicate column
name: reference_id` until it dropped those too. Nothing else failed.

### Verification
51 Rust tests (2 new, TEST-032), 28 frontend tests (5 new, TEST-033), zero
clippy warnings, `npm run build` clean. None of it has been exercised against
the device yet: nobody has walked thirty cycles, so no real reference profile
exists and the check, the evaluation gate and the segment rollover have been
run against synthetic cycles only.

### Current Status
M5 complete in code. Unverified on hardware.

### Next Steps
`eadprobe` commands for capture, check and evaluation so the workflow can be
driven without the GUI; then M6, the export package. The hardware verification
needs one walk of about a minute — thirty valid cycles — recorded through the
dashboard.

## 2026-09-18 — eadprobe: reference workflow and a vector self-check

### Objective
Drive the doc 12 workflow from the command line, so a capture, a check and an
evaluation can be run against the device without the GUI.

### Changes
`tools/eadprobe.py` gains three commands:
- `capture --seconds N [--save FILE]` — runs a REFERENCE_CAPTURE, counts valid
  cycles as they arrive, stops, and prints the profile the device built. Writes
  the raw 64-byte profile when asked. Exits 1 when the device refuses to build
  one, which it does below thirty valid cycles.
- `score --profile FILE [--evaluate] [--seconds N]` — sends the saved profile
  inside SESSION_START as a REFERENCE_CHECK or an EVALUATION and prints each
  cycle's score, confidence and class, then the median score and the median
  deviation per feature.
- `vectors [--verbose]` — decodes every golden vector with eadprobe's own
  decoders.

### Problems
`vectors` was written to close a hole, and immediately found what was in it. The
protocol design says the golden vectors are checked by three independent
implementations: firmware, Rust and this tool. The first two check themselves in
their test suites. Nothing ran the third. eadprobe's `CYCLE_RECORD` was still the
schema-3 72-byte layout, so it had been unable to decode a STEP_BATCH since the
schema-4 commit earlier the same day, and the claim of three-way agreement in
that commit was not true.

### Diagnosis
`python3 tools/eadprobe.py vectors` → `step_batch.hex FAILED: unpack requires a
buffer of 72 bytes`.

### Solution
Extended the struct to the 132-byte schema-4 record with a size assertion at
import, added `decode_reference` and `decode_session_start`, and made `vectors`
a command that has to pass.

### Verification
TEST-034: 18 vectors (17: a miscount, corrected in TEST-034 on 2026-10-02), 0 failures, values spot-checked against each file's stated
contents.

### Current Status
Complete. The workflow commands themselves have not been run against the device
— that needs someone to walk.

### Next Steps
M6: the export package.

## 2026-09-18 — M6: the export package

### Objective
Clinical requirement 4 and doc 10: `raw.csv`, `gait.csv`, `events.csv`,
`haptics.csv`, `metadata.json` and `session.mat`.

### Investigation
Three things doc 10 requires were not being kept, and had to be built before
anything could be exported.

1. **The unilateral cycle symmetry proxy** (doc 05 §10) had never been
   implemented, although `gait.csv` and the doc 11 LIVE panel both name it. It is
   `1 - normalized_difference` between consecutive valid cycles "using the same
   robust feature normalization used by the error engine", which means it needs
   the reference's spreads. Computed on the host in `Store::cycles`, for the same
   reason segments are (DEC-014): it is a pure function of the stored cycle
   stream. Null — not zero — for the first valid cycle and for a session with no
   reference, because without spreads there is nothing to normalize by.
2. **The calibration record** lived only in device RAM. Doc 10 §6 requires the
   calibration parameters and their quality in `metadata.json`, so a session
   exported after the device was unplugged would have had nothing to say. Now
   stored as JSON on the session row when the session starts, and only when the
   record is usable.
3. **Faults** were not recorded at all. STATUS arrives at 5 Hz; only changes of
   state or fault mask are stored, so a clean hour costs no rows.

Schema 6.

### Approach
`events.csv` was the one place where the doc asks for more than the device
produces: ten event types against the device's five. The ones that can be
derived honestly are derived — cycle bounds from the cycles, ERROR_ACTIVE and
ERROR_RESOLVED from the class transitions of displayable cycles, FAULT from the
status changes. SERVICE_TEST never appears, and `metadata.json` says why rather
than leaving a reader to wonder. ZUPT_END is written although doc 10 §4 does not
list it: the device reports windows, not instants, and dropping the end would
make the export hold less than the store.

`raw.csv` carries the stored ADC counts, not physical units (DEC-007), with the
scale factors and both mount maps in `metadata.json`. That keeps it consistent
with doc 10 §7's rule that the raw integers survive into the `.mat`.

### Problems
`scipy.io.loadmat` refused the first `session.mat` with `buffer is too small for
requested array`. The writer tagged char arrays `18`, which is `miUTF32`; the
correct code for UTF-16 is `17`. A reader given the wrong width runs off the end
of the buffer instead of failing cleanly, so the file looked structurally
plausible and was simply unreadable.

### Diagnosis
`tools/check_mat.py`, written for exactly this purpose: the `.mat` writer is
hand-rolled, so the only thing that can confirm it is an implementation somebody
else wrote.

### Verification
TEST-035 (2 Rust cases) and TEST-036 (scipy, every check passing, 0 failures).
57 Rust tests, zero clippy warnings, frontend builds.

### Current Status
CSV, metadata and `.mat` complete and verified. The PDF report (doc 10 §8) is
not written yet.

### Next Steps
The PDF report, then the hardware verification that every one of these needs: a
walk long enough to build a real reference.

## 2026-09-18 — M6: the PDF report

### Objective
Doc 10 §8: the research report, and with it the last file of the export package.

### Approach
krilla 0.8 (DEC-003 and the build plan) for the PDF, with a fixed-grid composer:
every element at a measured coordinate on A4, no layout engine, because the
report's shape never changes and a page that always looks the same is one a
reader can scan. Charts are drawn directly as polylines and rectangles rather
than rasterised, so the file stays around 22 kB and the text is selectable.

Fonts: the dashboard bundles IBM Plex as woff, which krilla cannot read, so the
latin subsets were converted to TTF once with fontTools and committed under
`dashboard/src-tauri/assets/fonts` (116 kB for three faces). Embedding the same
faces the UI uses means a printed number lines up with the one on screen, and
subsetting means a patient name outside Latin-1 still renders — which a base-14
font would have turned into question marks.

Three pages: session identity and measures with the error-score distribution and
class counts; the seven trend plots; the event timeline, segments and data
quality. Every page carries "Research and engineering report. Not a validated
clinical diagnostic report."

Nothing in the report passes or fails a patient. Doc 06 defines no threshold
separating a good cycle from a bad one, so the report shows distributions and
says that no threshold is drawn.

### Problems
The composer has no layout engine, and `pdftotext` showed exactly what that
costs: paragraphs ran off the right margin and were cut mid-sentence, and the
seventh trend plot needed 844 pt on an 841.89 pt page. Both fixed — a wrapping
paragraph helper, and charts at 72 pt with 20 pt gaps.

### Verification
TEST-037: `tools/check_pdf.py`, 41 checks, 0 failures. The Rust test additionally
asserts the file starts `%PDF-` and declares three pages.
55 Rust tests, zero clippy warnings, frontend builds.

### Current Status
M6 complete: `raw.csv`, `gait.csv`, `events.csv`, `haptics.csv`,
`metadata.json`, `session.mat` and `report.pdf`, with an EXPORT view that
reports what was written rather than announcing success.

### Next Steps
Everything left needs the device and somebody to walk. Nothing in M5 or M6 has
been exercised against real data: the reference workflow, the error scores, the
segments and every one of these files have only ever seen synthetic cycles.

## 2026-09-18 — Documentation brought back to the actual state

### Objective
`docs/handoff.md` still said "after M1 and most of M2" while M6 was finished. A
handoff that describes an idealised state is worse than none.

### Changes
- `docs/handoff.md`: current state rewritten for M0–M6, with the honest headline
  that nobody has walked thirty cycles yet; important files, decisions, known
  problems, how-to-run and next steps updated. The milestone plan collapsed to
  what actually remains.
- `docs/clinical_requirements.md`: requirement 4 marked implemented with its
  evidence; the status summary now says which two requirements have never seen a
  patient.
- `docs/implementation.md`: host-side segmentation, the symmetry proxy and the
  export package.
- `docs/architecture.md`: the export path added to the data-flow diagram.

### Verification
Full suite, all clean: 60 firmware native tests, 55 Rust tests, 28 frontend
tests, 18 protocol vectors, zero clippy warnings, firmware and dashboard build.

### Current Status
V1 is code-complete except M7. Everything remaining needs the device and somebody
to walk.

### Next Steps
`docs/handoff.md` → Next Steps. In short: a walk recorded through the dashboard,
then a one-minute reference capture, then a check and an evaluation, then the
export read end to end.

## 2026-09-18 — The repository is public

### Objective
The user made `sagar-aryan/ead-v1` public. Adjust to that, and check what is now
exposed.

### Investigation
- Secrets: `firmware/include/ead_secrets.h` holds the access-point passphrase. It
  is gitignored and `git log --all` shows it in **no** commit. The only match for
  "passphrase" anywhere in history is the code that generates it.
- **A correction:** in this session I told the user `Critical Clinical
  Insights.docx` was not in git, and `docs/clinical_requirements.md` said the
  same. Both were wrong: it was committed on 2026-09-17 in `9238d1c`, so it is now
  public, in history as well as at HEAD. Whether to remove it is the user's call —
  removing it from history means rewriting history and force-pushing.
- Also public by virtue of being tracked: the contract package
  `ead_agent_docs_v2/`, the 6 m walk recording, and the committer address in the
  git log.

### Changes
- The three setup scripts run straight from GitHub (`curl … | bash -s -- check`;
  on Windows, download to a file and run it with `-File`). The private-repo
  authentication paths (`gh repo clone`, token notes) are gone.
- Under `curl | bash`, stdin is the script, so the macOS install prompt reads
  from `/dev/tty`.
- The Windows one-liner downloads to a file rather than invoking a scriptblock:
  a scriptblock's `exit` closes the user's terminal.
- `setup-windows.ps1` is pure ASCII — Windows PowerShell 5.1 reads a BOM-less
  UTF-8 file as ANSI — and `.gitattributes` keeps `.ps1` CRLF and `.sh` LF on
  checkout.

### Verification
See TEST-038.

## 2026-09-18 — A failing test was committed

Commit `193edff` (close sessions left open at exit) was verified by running only
its own new test (`cargo test crash`). The full suite was not run, and
`schema_upgrades_from_version_1_without_losing_data` failed from then on: its
hand-built v1 store had no `raw_frames` table, which every real v1 store had and
which the new startup step reads. `146d044` then went out on top of it, because
the command chain carried on after `cargo test` reported the failure, and its
message claimed "59 Rust tests" when the suite had 56, one failing.

Fixed by giving the fixture the table. The full suite, 56 tests, passes.
Lesson: run the whole suite before every commit, and do not chain a commit onto a
test command whose failure does not stop the chain.

## 2026-09-23 — One BNO086 on the bench: wiring check, identity, tilt viewer

### Objective

The user wired one BNO086 and asked for software that shows how the sensor is being
tilted, after first checking that the part is a genuine BNO086 and that every GPIO
going to it works.

### Approach

A separate PlatformIO project, `firmware/bench/bno086`, using SparkFun's BNO08x
library (which carries CEVA's SH-2 driver). Deliberately a third-party driver: if the
bench test fails, the wiring is at fault, not new code of ours. The product firmware
keeps its no-external-dependency rule, and the product SH-2 driver is written after
the wiring is proven.

The firmware runs three things in order, and prints one line per result:

1. **Wiring.** Every line is read back with the ESP32's pull-up and then its
   pull-down, with the sensor held in reset, which finds shorts and misplaced wires.
   Then reset is released and the time for INT to assert is measured, which exercises
   RST and INT together. Then, once SH-2 is up and before any report is enabled,
   WAKE is pulled low and INT must answer, which exercises WAKE.
2. **Identity.** The SH-2 product ID request, and a probe that tries to enable the
   features only genuine CEVA firmware has.
3. **Data.** Rotation vector, accelerometer and gyro at 100 Hz, as one CSV line per
   orientation report, plus a two-second at-rest sanity check on gravity and the gyro.

`tools/bno_view.py` reads those lines, prints the checks, and draws a block that
tilts with the sensor (roll/pitch/yaw, accelerometer, gyro, report rate). Its
quaternion maths is checked against scipy over 200 random quaternions.

### Changes

Added:
- `firmware/bench/bno086/platformio.ini`, `firmware/bench/bno086/src/main.cpp`
- `tools/bno_view.py`

Documented: `docs/hardware.md` (bench pin table and measurements), `docs/testing.md`
(TEST-039), `docs/problems.md` (PROB-017).

### Problems

1. **Nothing at all came out over USB**, for the bench firmware, a bare
   `Serial.println` sketch, and the product firmware alike. PROB-017: the chip was
   sitting in the ROM bootloader after DTR/RTS had been toggled by hand on the open
   port, and the device node had moved to `/dev/ttyACM1`. Recovered with
   `esptool --after watchdog_reset`.
2. **The SH-2 driver blocks waiting for INT**, so a dead INT line hung the sketch
   inside `beginSPI` with nothing on the serial port. The firmware now refuses to
   call the driver unless its own INT check passed, and says so.
3. **The checks run once at boot, but a monitor is attached later**, so every line is
   kept in a transcript and re-sent whenever the host sends a byte; `bno_view.py`
   sends one on connect.

### Verification

TEST-039, on the hardware: all checks PASS, 99.9 Hz for 30 s, |a| = 9.752 m/s² and
|ω| = 0.005 rad/s at rest, SH-2 firmware 3.12.6 with four product-ID entries and the
full feature set. The plotting window was not exercised here (no display in this
session); its maths was checked against scipy.

### Current Status

Complete for one sensor on the bench. The product BNO086 driver is not started.

### Next Steps

1. Watch the tilt view on the bench and confirm the sensor follows the hand.
2. Decide and record the two-sensor SPI pin map as a DEC (it replaces the doc 03 I²C
   map), then write the product SH-2 driver behind the existing sensor abstraction.
3. Decide the ERM motor mapping against the new motor GPIOs.

## 2026-09-23 — Checking that the part is a BNO086, not just a BNO08x

### Objective

The user found the CEVA datasheet's list of BNO086-only features — 14-bit
accelerometer fusion, lower idle power, Interactive Calibration (Motion Intent and
Motion Request) — and asked whether they can be verified.

### Approach

Two additions to the bench firmware:

- `checkIdentity()` reads what the part says about its own hardware: each sensor's
  SH-2 metadata (the vendor's part number for the MEMS behind it, range, resolution,
  q points), the FRS serial-number record, and the oscillator type.
- `check086()` exercises the datasheet's 086-only features: the Motion Intent
  command, the Motion Request report, and the accelerometer's real resolution,
  measured from raw counts rather than taken on trust.

### Problems

1. The raw accelerometer report produced nothing, and then the at-rest check lost its
   accelerometer reports too. Cause: the probe ran before any report was enabled, and
   the raw report only flows while the calibrated accelerometer is running. Moved the
   probe to after the reports are up; both then worked.
2. First reading of the resolution test was wrong on its own terms. It looked for
   counts that were not multiples of 4, called the multiples-of-4 result a failure,
   and would have reported a 14-bit part as 12-bit. The BMA280 puts its 14-bit value
   left-aligned in a 16-bit field, so multiples of 4 are exactly what a 14-bit part
   gives. Replaced with the measurement that actually answers the question: steps per
   g against the ±8 g full scale the metadata declares.

### Verification

TEST-040, on the hardware: Bosch BMA280 / BMI055 / BMM150 declared, external crystal,
Motion Intent accepted, Motion Request report accepted, 1120 steps per g over ±8 g =
14.1 bits. At rest |a| = 9.805 m/s², |ω| = 0.0004 rad/s.

### Current Status

Complete, with two honest limits: idle power needs a current meter, and no readable
value can rule out a BNO085 physically relabelled as a BNO086 — only the package
marking can.

### Next Steps

Unchanged. One addition worth remembering: Interactive Calibration exists to remove
gyro zero-rate offset more often, which is the heading-drift problem in PROB-015.

## 2026-10-01 — Final wiring reference; the motor band needs 8 wires, not 7

### Objective

The builder confirmed a 10 kΩ pull-up on RST on the BNO086 breakout and asked for
a final wiring PDF with no guessed connections.

### Approach

Every connection in `docs/wiring_reference.md` now carries its evidence:
datasheet, bench measurement, builder confirmation, contract, or decision.
Sources read for this revision: the CEVA BNO08X datasheet (pin functions,
SPI-mode strapping, supply current, absolute maxima), the 7Semi breakout manual
(exact pad names), the Infineon IRLML6344 datasheet (SOT-23 pinout: 1 gate,
2 source, 3 drain; threshold 0.5–1.1 V), the ESP32-S3 datasheet, Seeed's XIAO
wiki (BAT pad polarity, 3V3 current, charge current), the Shenzhen Airupton
HT78XX datasheet and Holtek's LDO application note.

### Problems

1. **The motor-band wire count was wrong in three places** — the previous
   wiring reference, `docs/hardware.md`, and the generated assembly image — all
   saying 7 conductors, six returns plus one shared positive. The contract keeps
   the two HT7833 outputs apart (doc 02 §5), so motors 1–3 and 4–6 each need
   their own positive: **8 conductors**. A shared positive would have tied the
   two regulator outputs together. Corrected in the reference and in
   `docs/hardware.md`; the image is left as it is, with the error stated in its
   caption.
2. **The HT7833 pinout cannot be stated without knowing the part's maker.** At
   least two manufacturers sell a 3.3 V 500 mA LDO marked HT7833 (Holtek, and
   Shenzhen Airupton). Airupton's datasheet was read; Holtek's official datasheet
   link returned "file not found". The reference prints the Airupton table and
   says not to solder by it until the package and seller are known.
3. **The XIAO's charge current** appears in a mis-formatted row of Seeed's spec
   table. Read from the table's HTML: 50 mA fast charge for the plain XIAO
   ESP32-S3 (the Plus variant is 100 mA). Either way the XIAO's charger is too
   slow for a 2000 mAh cell, so the MCP73833 module is the charger.

### Changes

- `docs/wiring_reference.md` rewritten: evidence marks on every connection;
  exact 7Semi pad names; IRLML6344 pin numbers; SPI robustness for the long foot
  cable (33 Ω series on SCK and MOSI, a ground each side of SCK, 1 MHz bus);
  conductor-by-conductor harness tables; power and charging wiring; firmware
  rules the wiring depends on; and §16, five questions that each block one
  specific connection.
- `docs/hardware.md`: wire count corrected, image caption updated.
- RST open item closed: two boards' 10 kΩ pull-ups in parallel give 5 kΩ, which
  GPIO41 pulls low with 0.66 mA.

### Current Status

Every signal connection is final. Five items remain, all about specific parts:
the HT7833 package and seller, the ERM motor part, the MCP73833 board, whether
the cell has a protection circuit, and whether there is a power switch.

## 2026-10-02 — PROB-018: the dashboard kept a calibration across device reboots

### Objective

Fix the session-gate defect found by reading code while writing the handoff
(`handoff/`, untracked): after a device reboot or reflash the dashboard still
treated the previous boot's calibration as in force.

### Investigation

- Database (read-only): two 2026-09-18 evaluations on a new boot stored the previous
  boot's calibration record and have no frame with valid orientation (table in
  PROB-018).
- Code: `Tracker::on_hello` never cleared `State::calibration`; `Tracker` is rebuilt
  on every reconnect, so its `boot_id` cannot tell a reboot from a replug.

### Approach

Regression tests first, through the same `Tracker` the link uses, fed the golden
HELLO and calibration vectors. Then the smallest change in the one place every
HELLO passes through.

### Changes

Modified:
- `dashboard/src-tauri/src/device.rs`: `on_hello` clears the calibration when the
  boot_id differs from the last HELLO's; test module with two tests.
- `dashboard/src-tauri/src/protocol/mod.rs`, `protocol/tests.rs`: the golden-vector
  loader `vector()` made `pub(crate)` so the device tests reuse it.
- `docs/problems.md` (PROB-018), `docs/testing.md` (TEST-042).

### Problems

The first proposed fix (clear in the tracker's boot_id branch) also wiped the
calibration on every reconnect of the same boot. Caught by the second test before
commit; recorded as PROB-018 Attempt 1.

### Verification

Full suite: firmware native 63/63; `pio run` SUCCESS (RAM 20.9 %, flash 21.8 %);
`cargo test` 59 passed, 3 ignored; clippy clean; `npm test` 28/28;
`eadprobe vectors` 17/17, 0 failures.

Not run on hardware. The user reported on 2026-10-02 that the only board is now
wired to DEC-016 per `~/Downloads/EAD_V1_XIAO_connections.pdf`, with everything
soldered (both BNO086, motor channels, motors, rails, battery). The product
firmware still uses the doc-03 pins and would drive motor gates and both chip
selects on that wiring, so it must not be flashed until the pin map moves.

### Current Status

Completed in code. Hardware steps of TEST-042 pending the DEC-016 product firmware.

### Next Steps

P1 of the handoff: bring the stale files in `docs/` back to the actual state,
including the board now being the DEC-016 build.

## 2026-10-02 — Bench firmware moved to the DEC-016 pins

### Objective

The user reported the XIAO is now wired to DEC-016 with everything soldered,
including the motors. Have a firmware ready that can check the sensors on that
wiring without driving a motor gate, for when the board is next on USB.

### Investigation

- `firmware/bench/bno086` drove D0/D1/D3 (GPIO1, 2, 4) as WAKE, RST and CS; on
  DEC-016 those are motor gates 1, 2 and 4.
- The SparkFun BNO08x library (v1.0.6, the bench's known-good driver) keeps CS, INT
  and RST in file-scope globals, and CEVA's `sh2.c` inside it has a single global
  `sh2_t`. Two sensors cannot be driven at once with it.
- The product firmware cannot be used either: it still has the doc-03 pins.

### Approach

Keep the known-good driver and test one sensor per build, holding the other's CS
high. A two-sensor bring-up needs our own SH-2 driver, which is the P3/DEC-017
question for the user. Adapted the existing project rather than adding one, so the
repository no longer holds a bench firmware with motor-gate pins.

### Changes

Modified:
- `firmware/bench/bno086/platformio.ini`: envs `foot` and `shank`
  (`default_envs = foot`, so an upload without `-e` flashes one, not both).
- `firmware/bench/bno086/src/main.cpp`: DEC-016 pins; motors LOW and both CS high
  first; pull-ups on both INT lines; SPI 1 MHz; other-sensor INT report;
  `static_assert` that no sensor line is a motor pin.
- `docs/hardware.md`, `docs/testing.md` (TEST-043, not run).

### Problems

The first draft timed the other sensor's INT from after the selected sensor's INT
had arrived, so its "ms after reset" figure would have been wrong. Replaced with a
level report (high in reset, low after release) before building.

### Verification

Both envs build (RAM 6.6 %, flash 8.6 %). The `static_assert` was checked by moving
RST to GPIO42 (motor 3): the build failed with the assertion; restored. Not run on
hardware.

### Current Status

Built, not run. TEST-043 holds the procedure.

### Next Steps

Run TEST-043 when the board is on USB.

## 2026-10-02 — Docs brought back to the actual state; PROB-019 found

### Objective

Bring every stale file in `docs/` (and the READMEs and comments that described
the code wrongly) back to the actual state, as the documentation protocol
requires.

### Investigation

Each staleness listed in the untracked handoff notes was checked against the
code, the git history or the database before editing:
- `docs/handoff.md` described the 2026-09-18 state ("nobody has walked thirty
  cycles"; the two-sensor pin map "not yet a DEC").
- `docs/architecture.md` and `firmware/README.md` gave the old shank map
  (`Y = +chipX, Z = −chipY`); `config_v1.h` has `Y = +chipY, Z = +chipX` (TEST-027).
- `docs/implementation.md` had M3–M6 "Not started".
- `docs/protocol.md` §1 and §5.2 said schema 1 (code: 4); the catalogue said
  SESSION_START was CALIBRATION only (all four kinds are handled).
- DEC-016 cited the conflict check as §15 of the wiring reference; it is §14.
- TEST-034 and its progress note said 18 golden vectors. At TEST-034's own commit
  (`01a69fe`) `protocol/vectors/` held 17 `.hex` files, as it does now: a
  miscount, corrected visibly in place.
- `firmware/src/link.cpp` error texts said "schema 1"; `protocol.h` likewise.

### Changes

Modified: `docs/handoff.md` (rewritten), `README.md`, `architecture.md`,
`implementation.md`, `protocol.md`, `decisions.md`, `clinical_requirements.md`,
`hardware.md` (new "Current build: DEC-016" section; the MPU6500 sections labelled
as the previous build), `testing.md` (summary table to TEST-043), `problems.md`
(PROB-019); `firmware/README.md`, `firmware/test/README.md`, `dashboard/README.md`;
`dashboard/src/views/Live.tsx` header comment; `firmware/src/link.cpp` (the two
error texts no longer name a schema number, which HELLO already carries) and the
`protocol.h` header comment.

### Problems

- This log has no entries for the 2026-09-18 afternoon (the patient-67 sessions
  that produced PROB-013 to PROB-016), for DEC-015 and the worn-assembly image
  (2026-09-23), or for TEST-041 (2026-10-01). They were recorded at the time only
  in `problems.md`, `decisions.md`, `testing.md` and `hardware.md`, and are not
  reconstructed here after the fact.
- While fixing PROB-018, reading what else the host keeps across a device reboot
  found PROB-019: a reboot during a recording drops or mixes frames and overwrites
  colliding cycles. Recorded as open; the fix is the user's decision.

### Verification

Full suite after the edits (see the commit). The `link.cpp` texts reach the device
only with the next product-firmware flash, which must wait for DEC-016 support.

### Current Status

Completed.

### Next Steps

TEST-043 when the board is on USB; the user's decisions on PROB-019, DEC-017 and
DEC-006.

## 2026-10-02 — `ead` command; check mode requested

### Objective

The user asked to start the dashboard by typing `ead`, and for an `ead --check`
mode that tests every wire of both BNO086s and each ERM motor individually.

### Investigation

- `~/.local/bin` is on the user's PATH; no `ead` command existed.
- `tauri build --no-bundle` builds the release binary with the frontend embedded
  and installs nothing.
- The contract already defines the motor half: a service-test mode, one motor at
  a time by default, hard safety limits always on, refused during a patient
  session, events marked `SERVICE_TEST` (doc 07 §7, doc 08 0x12, doc 11 §6).
  Doc 13 §1.7 asks for flyback-diode polarity to be checked by hand before any
  motor test.
- The user reported the motors were tested with a plain PWM signal, and is not
  sure which XIAO is in the build.

### Approach

`ead` is a symlink from `~/.local/bin` to the release binary in the repository,
so a rebuild updates it and nothing needs root. The check mode is not built: it
needs device firmware that does not exist yet (the DEC-016 pins, our own SH-2
driver for two sensors, motor output). It waits for the user's DEC-017 and
DEC-006 decisions.

### Verification

`ead` started under X11 (`GDK_BACKEND=x11`, only so `xdotool` could see the
window): window present after 0.5 s, the process holding `ead.sqlite3` open.
Launched natively under Wayland it ran 20 s with nothing on stderr; that window
could not be observed with the tools available.

### Current Status

`ead` done on this machine. `ead --check` designed only (questions to the user).

## 2026-10-02 — Board on USB; the ERM driver PCB; motor 3 pin held low

### Objective

The user plugged the DEC-016 build in, pointed to `~/Documents/ead pcb/` for the
ERM driver, and delegated the sensor-driver choices (DEC-017) to the agent.

### Investigation

- Read the PCB project's handoff, hardware, testing and progress docs, its BOM, its
  bring-up procedure and `erm_channel_test.ino`. It answers several open hardware
  questions (wiring reference §16): Holtek HT7833 SOT-89 with the pinout read from
  the Holtek datasheet; motors rated 3 V, 90 mA, 120 mA at start; an external master
  switch after the charger. It also differs from the wiring reference: the 10 nF
  across each motor is fitted, and its bench sketch drove 20 kHz PWM where the
  contract says 200 Hz. Its 2026-09-24 log has a stuck-motor event with no
  established root cause and a host-silence failsafe never exercised on hardware.
- USB descriptor: the XIAO is `44:B1:76:AF:FB:7C`, the TEST-041 chip.
- A single `?` returned `padstate`'s stored report from a boot on the wired board
  (TEST-044). Sensor lines look as expected; GPIO42 (motor 3) is held low, unlike
  the other five motor pins (PROB-020).

### Approach

Stop before flashing. The bench firmware drives GPIO42 LOW, which is harmful only
if GPIO42 is wired to a drain or rail; that has to be measured, not assumed.

### Changes

`docs/wiring_reference.md` (§6, §7, §15, §16), `docs/hardware.md`,
`docs/testing.md` (TEST-044), `docs/problems.md` (PROB-020).

### Current Status

Blocked on the user's measurement of GPIO42 (PROB-020) before TEST-043.

### Next Steps

Measure GPIO42; then TEST-043; then DEC-017 (delegated: our own two-sensor SH-2
driver, Mahony on the BNO086's calibrated accel and gyro, the MPU6500 build retired,
a protocol schema bump accepted).

## 2026-10-02 — Both BNO086s pass on the real wiring; the power wiring hazard

### Objective

Run TEST-043 on the assembled build; answer the user's questions on the GPIO42
measurement, the power wiring, the motor test and the PWM frequency.

### Investigation

- The user asked why GPIO42 must be measured. The sensor test does not need motor 3,
  so the bench firmware now leaves GPIO42 undriven (its reset state) instead of
  driving it LOW: harmless whatever it is wired to. The measurement is needed only
  before motor 3 is driven.
- The user described the power wiring: the switched LOAD+ feeds the ERM driver and
  the XIAO's 5V pin. Seeed's wiki says that pin is USB VBUS and needs an external
  diode as an input: PROB-021.
- Holtek HT78xx Rev 1.51 (downloaded): SOT-89 pins 1 GND, 2 VIN, 3 VOUT (as the PCB
  was soldered); input up to 8 V, absolute maximum 8.5 V.
- User answers: master switch OFF; test pulses approved (DEC-018); foot cable about
  30 cm; the motor that kept running during the PCB bring-up stopped when the user
  disconnected and reconnected the dashboard; the sensor-driver choices were
  delegated (DEC-017).

### Changes

`firmware/bench/bno086/src/main.cpp` (GPIO42 left undriven); docs: TEST-043 results,
PROB-021, DEC-017, DEC-018, `hardware.md`, `wiring_reference.md` §15.

### Verification

TEST-043: PASS for both sensors (each flashed in turn, checks read back over USB).

### Current Status

Sensors proven. Motor 3 waits for PROB-020; any motor test waits for the switch rule
of PROB-021 to be understood by the operator.

### Next Steps

The DEC-017 driver and the DEC-018 service test, then `ead --check`.

## 2026-10-02 — BNO086 product firmware, SERVICE_TEST, `ead --check` (schema 5)

### Objective
The main build: product firmware for the DEC-016 wiring with our own two-sensor SH-2
driver (DEC-017), motor service-test pulses (DEC-018), and the `ead --check` view.

### Changes
- Firmware: `ead/sh2.{h,cpp}` (SHTP/SH-2 codec), `ead/motor_guard.{h,cpp}` (contract
  motor limits), `src/bno086.{h,cpp}` (SPI transport, per-wire check), acquisition
  rewritten (gyro-clocked frames), `src/motors.{h,cpp}` (LEDC 200 Hz, device-timed
  pulses), SERVICE_TEST in `link.cpp`; `config_v1.h` on DEC-016 pins and BNO086
  scales; MPU6500 driver removed. Protocol schema 5, config format 2.
- Vectors: format 2, four SERVICE_TEST vectors, `config_section_format1.hex`
  (byte-identical to the old section, kept for stored sessions).
- Dashboard: both config formats, store schema 7 (`config_format`, `service_tests`),
  service-test commands, Check view, `--check` launch mode. `eadprobe`: schema 5,
  `check`, `pulse`, scale from the device config, recordings carry CONFIG.

### Problems
- Frames first came at 125 Hz: the BNO086 runs the accelerometer at its nearest rate,
  125 Hz, while the gyroscope runs at 100 Hz. Frames are now gyro-clocked.
- After a sensor reset, one 152 ms pause follows the first gyro report (sequence +1):
  sensor start-up behaviour, shown by the timestamps.
- Shank repeats 1.9 % (clusters every ~2.4 s, phase crossings); period sd 200 us.
- Check view first showed errors at the page top, far from the button: fixed per panel.
- Synthetic X clicks on scrolled content do not reach WebKit here; keyboard works.

### Verification
Native 76/76, Rust 62 (+ hardware test PASS on the device), frontend 28/28, vectors 22.
On hardware over USB: all 7 check steps PASS on both sensors; 60 s at 100.142 Hz, 0
missing, 0 dropped, 0 bus errors, |a| 1.013 / 1.002 g; re-check resumes streaming;
GUI: Check table all PASS, M3 pulse refused with the PROB-020 text. Database migrated
to schema 7 (backup `ead.sqlite3.schema6-backup-2026-10-02`), 18 sessions intact.
Not done: an accepted motor pulse felt by a person; protocol.md / DEC-017 detail /
TEST entries / handoff for this work; mount maps and gait on the leg.

## 2026-10-02 — Replay regression fixed; Check view errors verified; docs for schema 5

### Objective
Close what the previous entry left open.

### Problems
- The replay tool converted counts with `config_v1.h`'s constants, which since schema 5
  are the BNO086 scale and identity maps: the MPU6500 ground-truth walk replayed to 0
  valid cycles. Found by running it, not by a test.

### Solution
The replay tool takes the scale and maps from the recording's CONFIG_GET message
(`eadprobe stats --record` writes one since schema 5), or `--mpu6500` for older
recordings, and refuses otherwise.

### Verification
Walk: 6 valid cycles, 6.39 m with `--mpu6500`; refused without. A new BNO086 recording
replays with its own conversion. The Check view's motor refusal now shows in the Motors
panel (re-verified in the app). `protocol.md` (schema 5, §5.14, §6.11), DEC-017's
implementation choices, TEST-045–050, `implementation.md`, `architecture.md`,
`hardware.md` and `handoff.md` written.

### Current Status
Completed. Open: motor pulses felt by a person, PROB-019, PROB-020, the leg work.

## 2026-10-02 — PROB-019 fixed: a device reboot ends the host session

### Objective
The user's default for PROB-019 (no objection): end the recording at a device reboot and
say so. Also run TEST-042's hardware steps, which waited for DEC-016 firmware.

### Changes
`device.rs` (a changed boot_id after an earlier one clears the session kind and calls
`Sink::device_restarted`), `app.rs` (the sink ends the store's recording; command
`ended_by_restart`), `store/mod.rs` (`end_session_at_restart`, segments closed as
`device_restarted`, the notice cleared by the next session), `StateBar.tsx`, `App.tsx`,
two Rust tests, and the ignored hardware test
`a_device_reset_ends_the_session_and_its_calibration`.

### Verification
Rust 64 passed; the hardware test PASS on the device (replug keeps everything; a board
reset ends the session and the calibration). TEST-042 PASS; PROB-018 and PROB-019
resolved.

## 2026-10-02 — A flaky calibration in the hardware test: PROB-022

### Problems
Running both hardware tests serially, the reset test's calibration was rejected as
"moved" (foot gyro sd 3.82 °/s) with the device untouched. Alone it passed.

### Diagnosis
The test printed nothing useful at first; it now reports the reject reasons. The
"settling after power-up" hypothesis was tested and not supported (eight calibrations
from 2.8 s after a reset all accepted). A 40 s still recording showed episodes of up to
3.5 °/s (foot) and 1.2 °/s (shank) at the same moments in both sensors. Cause unknown:
PROB-022.

### Solution
The test retries a rejected calibration up to three times, as an operator would.

### Verification
Both hardware tests passed in two consecutive serial runs.

## 2026-10-02 — Motor 3 enabled (PROB-020 resolved)

### Investigation
The user measured the MTMS (GPIO42) pad to GND on the unpowered board: about 100 kΩ,
steady, which is the gate network every motor pin sees. Every faulty-wiring hypothesis
in PROB-020 predicts a different reading. The earlier "held low" readback is within
specification for a pin pulled to about 2.3 V.

### Changes
`config_v1.h` (`EAD_MOTOR_ENABLED_MASK` 0x3F), `main.cpp` comment, the bench firmware's
GPIO42 exception removed, `link.cpp` refusal text without the motor-3 case.

### Verification
Full suite; flashed: motor service test available, sensor check 14/14 PASS, 15 s at
100.143 Hz, 0 missing, no faults. Motor 3 turning is for the user to feel.

## 2026-10-02 — BNO086 on the leg: mount maps, first walks, PROB-023

### Objective
Measure the BNO086 mount maps, then the first walks against a measured course.

### Approach
Mount maps from a standing / toe-raise / knee-extension recording, cross-checked
with the user's description of the silkscreen axes; flashed and confirmed with
the dashboard mounting check (TEST-051). Five 10 m out-and-back walks recorded by
the user over Wi-Fi, replayed on the host from the dashboard database.

### Problems
Cycles of 16–29 m. Traced to the gait engine staying in swing through a stop when
the last step had no impact (PROB-023). Fixed with an exit from swing on
sustained stillness; one unit test. Two contacts the old engine reported after a
turn are now missed, along with every other first landing from standing (PROB-024).

### Changes
- `firmware/lib/ead_core/src/ead/gait.cpp`, `firmware/test/test_gait/test_main.cpp`
- `tools/session2eadlog.py` (new): dashboard session → .eadlog for the replay.

### Current Status
The fix is verified in replay and by unit test, and flashed (0.1.0+77ae282, streaming
at 100.14 Hz, no faults). PROB-024 open.

### Next Steps
Flash over USB; replay sweeps for PROB-024.

## 2026-10-02 — PROB-024: soft footfalls and the event-path stillness test

### Objective
Work through PROB-024 on the five 10 m walks (now fixtures in `recordings/`).

### Approach
Replay sweeps against the counted landings and the course, with the 6 m walk as a
regression check; traces of the failing stances; a scratch build instrumenting
the drift correction.

### Changes
- `gait.{h,cpp}`: soft footfalls confirmed by stillness; `LowPass2` and the 20 Hz
  event path for the stillness test (DEC-019). Two unit tests.
- `tools/replay/walks.py` (new): per-leg comparison with the ground truth.

### Problems
Lowering the confirm level or the swing-start rate split strides (rejected).
Slow strides still read 30 % short; cause unknown.

### Verification
TEST-053.

### Current Status
In progress: contacts done; distance partial (PROB-024).

### Next Steps
Flash; the user decides on the 25 °/s limit; the slow-stride shortfall.

## 2026-10-04 — Barefoot walks over Wi-Fi at 200 Hz, with video (TEST-058)

### Objective
Ground truth for the schema-6 feed on the leg: seven walks, each with a video.

### Investigation
Sessions read from the dashboard database; each exported with `tools/session2eadlog.py`
and replayed. Videos synced by motion energy, then by the heel stamps.

### Problems
- The replay calibrates from the first seconds, but these recordings start with the
  walker on the way to the start line; stands of 2–3 s fail the 2 °/s stillness test.
  Added `--still-from S` to `tools/replay/main.cpp`; the end stand (≥ 5 s) calibrates
  where the start does not.
- Audio sync failed: the stamps are not audible above the carpet's noise (peak/median
  ≤ 3). Motion-energy correlation over a whole recording locked onto the repeating
  out-and-back pattern (offsets of 20–90 s); restricted to the first 25 s it agreed
  with the stamps on six of seven; fast 1 was synced by eye from the stamps.

### Changes
- `tools/replay/main.cpp`: `--still-from`.
- `recordings/`: seven 2026-10-04 fixtures, README rows with sync offsets.
- `.gitignore`: `recrdings/` (the videos stay local).

### Verification
TEST-058: 0 frames lost over Wi-Fi at 200 Hz; detector findings in PROB-024.

### Current Status
Completed. Contact detection open (PROB-024).

### Next Steps
Shank-based step detection, scored against the 2026-10-04 fixtures.

## 2026-10-04 — Step detection from the shank's swing (DEC-022, TEST-059)

### Objective
DEC-020 6a: replace the foot-impact contact detector, which split strides on the
barefoot walks (TEST-058).

### Investigation
The aligned shank gyro added to the replay's TRACE line. Forward swing rate
(−gy, right leg) per stride: one peak of 140–290 °/s at mid-swing, a sharp dip at
heel strike, a broad backward dip at push-off. Timing survey on 14 recordings:
the foot's impact peak follows the downward zero crossing by 35–60 ms (median);
the shank minimum is later and wider-spread (to 190 ms), so it was not used as
the contact's clock.

### Approach
Count from the shank, time from the foot (DEC-022). Kept: ZUPT, integration,
cycle features, the stillness fallback of PROB-023.

### Changes
- `gait.{h,cpp}`: new contact and toe-off logic; `GaitConfig` reduced to
  `midSwingDps`, `contactSearchS`, the ZUPT limits and the rate.
- `test_gait`: the synthetic gait gains a realistic shank (stance roll, push-off
  dip); two tests replaced by one for a step into a stop; a new split-stride test.
- `tools/replay/main.cpp`: shank gyro in TRACE, `--mid-swing`, `--contact-search`;
  the old detector's flags removed.
- `tools/replay/walks.py`: all eleven 10 m fixtures, the 2026-10-04 ones by leg
  windows.

### Problems
- The first split-stride test passed on the old engine too: a lone impact in
  stance was never the failure. Rebuilt from the real mechanism (the foot turning
  fast in stance, then an impact); it now fails on the old engine.
- The scorer dropped each recording's last contact (it paired contacts with
  cycles); fixed by counting contacts directly.
- The 6 m course read 5 cycles, not 6: TEST-030's first "heel strike" was a
  push-off (correction recorded under TEST-030).

### Verification
TEST-059: counting error 21 → 3 over 176 landings; 13 gait tests pass (3 fail on
the old engine); firmware builds.

### Current Status
Completed on the host. Not flashed; not tried on the leg.

### Next Steps
Flash and walk; then the distance filter (DEC-020 6c).

## 2026-10-04 — Haptic feedback during walking (DEC-023, TEST-060)

### Objective
DEC-020 item 2: vibration during walking, a master switch on the dashboard.

### Investigation
Doc 06 §5–§13 against what existed: the error engine scores each cycle of a check or
an evaluation on the device; the motor guard enforced the limits for one-motor service
pulses; HAPTIC_BATCH and CONFIG_SET were reserved message types; every export and UI
said "no haptics (DEC-006)".

### Approach
On the device, one cue per scored cycle (DEC-023). Pure logic in `ead_core` with unit
tests; the device module only wires it to the switch, the motors and the log.

### Changes
Firmware, protocol schema 7 (vectors regenerated; format-1 config keeps
`haptics_fitted` 0), eadprobe, dashboard store schema 10, export, UI. Files in
docs/implementation.md "Haptic feedback".

### Problems
- `HapticEngine::stop` first copied the cue's earlier fields into the OFF record:
  garbage when called from outside a cycle (switch off). Rewritten to build a clean
  record; `onCycle` adds the cycle's fields.
- The guard test's rolling-limit case was wrong (see TEST-060).
- Clippy: two functions at 8 arguments. `metadata_json` reads the haptics from the
  store as it does the config; `session_mat` takes one argument per table and carries
  an `allow` saying so.

### Verification
TEST-060.

### Current Status
Completed on the host. Not flashed, not felt.

### Next Steps
Flash; bench test with the motors visible; then worn.

## 2026-10-05 — Fixing the external audit's findings

### Objective
An external code audit of `982a350` (pasted by the user) reported 8 reproduced defects
and more. The user asked whether it was correct, then to fix the list in order of
harm; power wiring and battery runtime are out of scope by the user's decision.

### Investigation
Each claim was checked against the code. Confirmed as stated: I01 (writer drops
batches on a failed commit), I02 (reconnect forgets backfill progress), I03
(recalibration ignored), I04 (contact angle read late), I06 (duplicates counted
twice), I07 (reference ownership not checked), I09 (session acceptance inferred from
800 ms of silence), I11 (stale export checkers), I15 (stale handoff). Corrected:
I10's frame-level faults did stop feedback, the gap was device-level faults and stale
rotation vectors; the motor race is bounded by the 250 ms cue; I08's 10,000 vs
10,300 cannot be metadata against raw.csv (same pass), the MAT pass can differ; I05's
realistic effect is the tail dropped at stop, since I02 stops any cross-reconnect
backfill.

### Changes (firmware)
PROB-026 (feedback faults, motor and session races), PROB-027 (recalibration),
PROB-028 (contact angle). Files: `lib/ead_core/src/ead/{feed,gait}.{h,cpp}`,
`src/{acquisition,calibration_service,device,gait_service,main,motors,orientation,
session_service,telemetry}`, `test/test_{feed,gait}`.

### Verification
TEST-063.

### Changes (dashboard)
PROB-029 (failed commits kept and retried, reported in the state bar; `flush` returns
a result). Files: `dashboard/src-tauri/src/{store/mod.rs,store/tests.rs,app.rs,main.rs,
hardware_tests.rs}`, `dashboard/src/{api.ts,App.tsx,components/StateBar.tsx}`.

PROB-030 (tracker kept between connections) and PROB-031 (repeats dropped):
`dashboard/src-tauri/src/device.rs`.

PROB-032 (reference ownership): `store/mod.rs`, `app.rs`, `views/Sessions.tsx`.

PROB-033 (protocol schema 8: ACK for session commands, STATUS `session`; the
dashboard waits for answers, drains to the stop's boundary, stops a session at close
or when nothing records it): firmware `src/{link,device}.cpp`, `ead_core` protocol,
vectors, `eadprobe`, dashboard `device.rs`, `app.rs`, `protocol/`, `api.ts`.

Flashed and checked on the board, worn over USB (TEST-064, TEST-065); the reset test
found the old boot's backfill being requested from the new one, fixed (PROB-030
update). PROB-034 (export checkers), PROB-035 (exports: live sessions refused, the
recorded schema kept as store schema 11, CSVs streamed).

### Current Status
In progress. Board on `29c560c` (schema 8); `ead` rebuilt after each dashboard change.

Docs brought to the current state (audit I15): `handoff.md` (current state, known
problems, failed approaches, how to verify, next steps), `hardware.md` Haptics,
`clinical_requirements.md` (status, per-cycle reading of "per step", what "native
rate" covers), `architecture.md` interfaces (SPI). History kept and labelled.

PROB-036 (audit I14): Vite 5 → 8 and its React plugin 4 → 6, `npm audit` clean; the
Rust notices all come through Tauri's GTK3 stack and krilla's newest release, so they
wait on upstream. The release app was launched against an empty data folder and
renders (screenshot).

Store: bounded writer queue and foreign keys on every connection (PROB-029 update).
Not changed, for the user to decide: Wi-Fi controller ownership (audit I12), which
cannot be tested from this laptop.

## 2026-10-05 — First walks with feedback; DEC-025

### Objective
The full workflow worn on battery over Wi-Fi, then the user's two reports: cues faint,
and frequent cues while walking normally.

### Investigation
TEST-066 and TEST-067 from the stored sessions: normal steps score about 0.23 (OFF was
0.25); reference v5's spreads were narrow (stance 0.012), and a 7 % slower evaluation
walk scored as error. Rescored offline with wider floors before changing anything.

### Changes
DEC-025: cues at 255 for 500 ms (`config_v1.h`, `feedback.cpp`), guard range separate
(`EAD_MOTOR_*`), OFF threshold 0.35, spread floors raised and applied at scoring
(`reference.h`, `error_engine.cpp`), the dashboard's symmetry proxy floored alike,
Check view up to 100 %, vectors (config section) regenerated. Tests:
`test_a_narrow_old_profile_is_scored_under_todays_floors` (0.83 without the scoring
floor, as in TEST-067), `test_the_built_cue_is_full_strength_whatever_the_score`, the
guard accepting 255.

### Current Status
Built and tested on the host; to flash and walk.

## 2026-10-05 — The ERM driver PCB into the repository

The PCB project (`~/Documents/ead pcb/`, never in git) copied to
`hardware/erm-driver-pcb/` at the user's request: four KiCad variants, generator and
check scripts, outputs, the board's own docs, the bench firmware. Left out, also in
`.gitignore`: the three purchase invoices (the user), KiCad `.kicad_prl` view state and
lock files. Scanned before publishing: no email addresses, phone numbers, secrets or
absolute paths; the PDFs carry no author. The docs keep the supplier and an order
number as the source of the parts list. Current docs point to the new path; dated
entries keep the old one.

## 2026-10-07 — Second external audit: verified, the user's answers, fixes

### Objective
The user pasted the findings list (F-01 to F-40) of a second external audit
(`EAD_AUDIT_REPORT.md`, untracked, run read-only against `3a08651`) and asked which
exist in the current version, which need fixing, and which stop the device working.

### Investigation
Every finding re-checked in the code, not taken from the report. All confirmed against
`3a08651` except: F-11 partly done (one test already checked the shipped duty); F-18
never observed (no `refused` record in any stored session); F-12 and F-01 are the user's
DEC-025 choices; F-40 the user's DEC-024 choice. The one finding that can make a session
run wrong unnoticed is F-27 (a calibration record dropped on a busy Wi-Fi socket).

### The user's answers (2026-10-07)
- F-01: cue strength scaled by the score again, 60–100 % (DEC-026).
- F-10: hold back cues after 5 s without the laptop, resume when it is heard (DEC-027).
- F-19/F-38: the patient's name in every export, every CSV included (DEC-028).
- F-07: no external reference system available; no sync work.
- Q-1 (target vs baseline): the patient's own baseline, as now.
- Left leg: not needed; right leg only.
- The remaining fixes: go ahead.

### Changes (first group)
- DEC-026: `EAD_HAPTIC_MIN_DUTY` 153; test builds the shipped configuration from
  `config_v1.h` (`test_the_shipped_cue_is_graded_from_60_to_100_percent`, replaces the
  full-strength test). StateBar tooltip.
- DEC-027: `device::noteHostMessage` / `hostQuietFor` (fed by `Link::onMessage`),
  `feedback::onCycle` holds back a cue after `EAD_HAPTIC_LINK_TIMEOUT_MS`; haptic reason
  9 `link_lost`; protocol schema 9 (firmware, dashboard, eadprobe, vectors). Cycles view
  shows "no laptop"; the PDF haptics line counts held-back cues; `check_pdf.py` checks it.
- DEC-028 and F-31: `patient_name` in all five CSVs; `valid` column at the end of
  `gait.csv`. `check_mat.py` compares `valid` with `session.mat` and every CSV's names
  with `metadata.json`.
- `docs/protocol.md`: schema 9, reason 9, and the stale 0.25 OFF threshold in §5.17/§6.12.

### Verification
Full suite: pio 103/103, firmware build, cargo 84 (4 ignored), npm 28/28, build, vectors
0 failures; `export_sample` then `check_mat.py` and `check_pdf.py`, 0 failures each.
The link-loss hold-back is not host-testable (Arduino side); to confirm on the device.

### Changes (second group)
- Wi-Fi and attribution (PROB-037 to PROB-039): calibration completion taken on
  commit; sessions also gated on STATUS calibration state; 5 s silence ends a Wi-Fi
  connection; one backfill range per tick; reply queue 8; held profile cleared at a
  capture start and a new boot; a start's ACK raises the delivery floor; service-test
  refusals matched by command sequence.
- Store schema 12 (PROB-040): status changes keyed by arrival; FAULT rows timed by
  their frame.
- Non-finite guards (PROB-041); calibration minimum 2 s at 200 Hz (PROB-042).
- F-16: one Rust test reads the firmware headers and compares every mirrored score
  constant; the frontend's copies removed, weights and the distance threshold now come
  from the backend's vocabulary.
- F-21/F-23/F-25: `wiring_reference.md` build status and capacitor rows, docs index,
  the duplicated export line in `architecture.md`; the GPU dump untracked and ignored
  (kept locally); a root `README.md`.

### Failed approach
Clearing the dashboard's held calibration record at each calibration start (for
F-27): backed out before commit, because a cancelled window keeps the device's old
calibration and the dashboard would then block sessions on a calibrated device. The
firmware fix removes the cause; the STATUS gate covers the rest.

`cargo fmt` was run once on one file and reformatted the whole crate (the project is
not rustfmt-formatted); the files were restored and only the intended edits re-applied.

### Verification (second group)
Full suite: pio 104/104, firmware and both bench builds, cargo 91 (4 ignored),
clippy `-D warnings` clean, npm 28/28, build, vectors 0 failures; export checks 0
failures; walk replays unchanged (3/176). New regression tests fail with their fixes
removed (F-30, F-33, F-13 checked).
