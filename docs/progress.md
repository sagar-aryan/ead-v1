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
TEST-034: 18 vectors, 0 failures, values spot-checked against each file's stated
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
