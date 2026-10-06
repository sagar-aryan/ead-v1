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
- `hardware.md` — As-built hardware: the current DEC-016 build, the previous
  MPU6500 build, register config, pins, mount maps.
- `wiring_reference.md` — Every connection of the DEC-016 build, with its evidence.
- `decisions.md` — DEC-001 to DEC-029.
- `problems.md` — PROB-001 to PROB-042 (there is no PROB-008), including failed
  approaches.
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

- Right leg only, barefoot. Two IMUs at 200 Hz (100 Hz before schema 6, DEC-021). The product firmware drives two
  BNO086s on SPI (DEC-016, DEC-017) since schema 5; sessions recorded before it came
  from two MPU6500s on I²C (foot `0x68`, shank `0x69`).
- Six ERM motors on a calf band (DEC-015), fitted on the DEC-016 build (user,
  2026-10-02). Error-driven feedback during evaluations with the dashboard's switch on
  (DEC-023); service-test pulses outside sessions (DEC-018).
- The wire protocol between device and dashboard is implemented three times over
  (firmware, dashboard, `tools/eadprobe.py`), all checked against the same golden
  vectors in `protocol/vectors/`.
- Wi-Fi AP + WebSocket primary link, plus the same binary protocol over USB
  (DEC-005). No BLE, FSR or battery monitoring.
- Desktop: Tauri 2 + React + TypeScript + Rust, for observation, configuration,
  storage, analysis and export.
