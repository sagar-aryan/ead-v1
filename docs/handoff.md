# Project Handoff

## Project
EAD V1 — right-leg, barefoot, wearable error-augmentation system for stroke
gait (ESP32-S3 + dual MPU6050 + 6-ERM shank band + Tauri 2 dashboard).
Contract: `ead_agent_docs_v2/` (authoritative, read-only).

## Current Objective
Stand up the repository skeleton (doc 14 Phase 1) with engineering docs and
build scaffolding, so sensor-layer work (Phase 2) can begin.

## Current State
- `ead_agent_docs_v2/` contract present and untouched.
- `docs/` engineering records created (this scaffolding task).
- `.gitignore` created; repo has NOT been `git init`ed (main agent handles it).
- `firmware/` PlatformIO skeleton observed (parallel work): `platformio.ini`
  (`seeed_xiao_esp32s3`, Arduino, LittleFS, USB-CDC flags, MPU6050/WebSocket
  deps) + empty `src/include/lib/test`. No algorithm code, no dashboard.
- Nothing built or tested yet.

## Architecture
100 Hz dual-IMU acquisition (foot `0x68` dorsum, shank `0x69` lower shank,
400 kHz I2C, INT GPIO7/8) → Mahony orientation → gait FSM + foot-only ZUPT →
patient-specific reference compare → error [0,1] + confidence [0,1] → 6-ch
ERM control (200 Hz PWM, 20–80%, 5 s / 50%-per-10 s limits) → LittleFS +
Wi-Fi AP (`192.168.4.1:8080/ws`) binary telemetry → Tauri 2 dashboard
(observe/configure/store/export CSV/`.mat`/PDF). Full detail:
`docs/architecture.md`. Decisions: `docs/decisions.md` (DEC-001–DEC-004).

## Important Files
- `ead_agent_docs_v2/00_README.md` — V1 identity, non-negotiable rule.
- `ead_agent_docs_v2/01…13_*.md` — system/hardware/algorithm/protocol/test specs.
- `ead_agent_docs_v2/14_IMPLEMENTATION_PLAN.md` — Phase 1–12 plan.
- `ead_agent_docs_v2/15_DECISION_LOG_AND_TRACEABILITY.md` — fixed decisions.
- `firmware/platformio.ini` — build definition.
- `docs/` — README, architecture, implementation, decisions, problems,
  testing, progress, handoff.
- `.gitignore` — PIO/Tauri/`.ead`/`dist` excludes.

## Completed Work
- Engineering docs skeleton + `.gitignore` (2026-09-16, this task).

## Current Work
- Phase 1 skeleton in progress: firmware `platformio.ini` exists; module
  sources, dashboard project, and build verification outstanding.

## Known Problems
- None logged (`docs/problems.md` is an empty template).

## Failed Approaches
- None.

## Important Decisions
- DEC-001 PlatformIO+Arduino on Linux; DEC-002 USB-C CDC debug;
  DEC-003 Rust-native `.mat` export; DEC-004 researcher-entered segment
  limits. See `docs/decisions.md`.

## Environment
- Linux host. Firmware: PlatformIO (`espressif32`), Seeed XIAO ESP32-S3,
  Arduino framework. Dashboard (planned): Tauri 2 + React + TS + Rust.
  Tool versions unverified — record exact `pio --version` / `rustc` /
  `node` versions when first used.

## How To Run
- Firmware build (not yet verified): `pio run -d firmware`
  (env `seeed_xiao_esp32s3`); monitor: `pio device monitor -d firmware`.
- Dashboard: not scaffolded — no run steps yet.
- Tests: none implemented; replay plan in `docs/testing.md`.

## How To Verify
- Skeleton: `pio run -d firmware` succeeds; both MPU6050 enumerate at
  `0x68`/`0x69` on hardware (doc 13 §1).
- Later: deterministic replay suite must pass before bench tests count
  (doc 14 Phase 12 gate).

## Next Steps
1. `git init` + initial commit (main agent).
2. `pio run -d firmware` to verify skeleton builds; fix env issues.
3. Create firmware module files per `docs/implementation.md` layout.
4. Scaffold Tauri 2 dashboard (Phase 9 location TBD — recommend `dashboard/`).
5. Begin Phase 2 sensor layer with versioned replay fixtures.

## Warnings
- Never modify `ead_agent_docs_v2/`; spec values there override library defaults.
- Do not add battery/switch/BLE/FSR/BNO086 code — explicitly out of V1 scope.
- Keep motor outputs OFF except valid RUNNING haptic conditions.
- Do not claim tests passed unless actually executed.
