# EAD V1 — Engineering Documentation

Right-leg, barefoot, wearable error-augmentation system for stroke gait.
Source of implementation truth: `ead_agent_docs_v2/` (read-only contract, do not modify).
This `docs/` tree records the engineering process per `AGENTS.md`.

## Index

- `architecture.md` — System overview, components, data flow, constraints.
- `implementation.md` — Phase 1 skeleton status and module implementation state.
- `decisions.md` — DEC-001 through DEC-004.
- `problems.md` — Bug/failure log (currently empty template).
- `testing.md` — Replay determinism and validation plan (from doc 13).
- `progress.md` — Chronological engineering log.
- `handoff.md` — Current state, how to run, next steps.

## Authoritative references

- `ead_agent_docs_v2/00_README.md` — V1 identity and non-negotiable rule.
- `ead_agent_docs_v2/01_SYSTEM_SPEC.md` — Product definition, real-time loop.
- `ead_agent_docs_v2/02_HARDWARE_WIRING.md` — I2C bus, interrupts, motor channels.
- `ead_agent_docs_v2/07_FIRMWARE_ARCHITECTURE.md` — Module structure, timing, states.
- `ead_agent_docs_v2/13_TEST_AND_VALIDATION_PLAN.md` — Verification requirements.
- `ead_agent_docs_v2/14_IMPLEMENTATION_PLAN.md` — Phase 1–12 plan.
- `ead_agent_docs_v2/15_DECISION_LOG_AND_TRACEABILITY.md` — Fixed V1 decisions.

## V1 identity (summary)

- Right leg only, barefoot. 2 × MPU6050 (foot `0x68`, shank `0x69`), 100 Hz, 400 kHz I2C.
- 6 × ERM motors, circumferential lower-shank band, ESP32-S3 direct PWM/MOSFET.
- Wi-Fi AP + WebSocket primary link. No BLE, no FSR, no BNO086, no battery ADC in V1.
- Desktop: Tauri 2 + React + TypeScript + Rust (observation/configuration/export only).
- ESP32 remains in the haptic control loop even when Wi-Fi is unavailable.
