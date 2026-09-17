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
