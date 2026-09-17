# EAD V1 — Engineering Documentation

Right-leg, barefoot, wearable error-augmentation system for stroke gait.
Source of implementation truth: `ead_agent_docs_v2/` (read-only contract).
This `docs/` tree records the engineering process: what was built, why, what
failed, and what was verified. Update it alongside every significant change.

## Index

- `handoff.md` — Start here: current state, milestone plan, how to run and verify.
- `protocol.md` — The device wire protocol: payloads, framing, backfill.
- `clinical_requirements.md` — The four clinical requirements: where each is
  specified, what is built, what is proven.
- `architecture.md` — Target system design, data flow, interfaces, dependencies.
- `implementation.md` — What exists in code today, per feature.
- `hardware.md` — As-built hardware: silicon, register config, pins, mount maps.
- `decisions.md` — DEC-001 to DEC-012.
- `problems.md` — PROB-001 to PROB-007, including failed approaches.
- `testing.md` — Executed tests with measured results; doc-13 acceptance suites.
- `progress.md` — Chronological engineering log.

## Rules for contributors

- Contract values in `ead_agent_docs_v2/` override library defaults and intuition.
  Deviations need a DEC entry.
- Record failed approaches; never delete them.
- Record a test as passed only if it was run; include the measured numbers.
- Mark hardware facts as verified or unverified.

## Agent tooling

Project-level agent skills live in `.claude/skills/` (installed with the
`skills` CLI; pinned in `skills-lock.json`):
- `frontend-design`: UI design guidance.
- `vercel-react-best-practices`: React rendering-performance rules.

## V1 identity (summary)

- Right leg only, barefoot. Two IMUs (foot `0x68`, shank `0x69`; MPU6500
  silicon), 100 Hz, 400 kHz I²C.
- Six ERM motors on a lower-shank band are specified; drivers are not fitted and
  there is no haptic code (DEC-006).
- The wire protocol between device and dashboard is implemented three times over
  (firmware, dashboard, `tools/eadprobe.py`), all checked against the same golden
  vectors in `protocol/vectors/`.
- Wi-Fi AP + WebSocket primary link, plus the same binary protocol over USB
  (DEC-005). No BLE, FSR, BNO086 or battery monitoring.
- Desktop: Tauri 2 + React + TypeScript + Rust, for observation, configuration,
  storage, analysis and export.
