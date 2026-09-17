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
