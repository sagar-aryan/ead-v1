# 14 — Implementation Plan

## Phase 1 — Repository skeleton
Create firmware and dashboard projects with the module structure in `07_FIRMWARE_ARCHITECTURE.md`.

## Phase 2 — Sensor layer
Implement:
- I²C bus at 400 kHz;
- two MPU6050 devices;
- fixed addresses `0x68`/`0x69`;
- data-ready interrupts;
- 100 Hz synchronized acquisition;
- raw logging;
- startup calibration.

Acceptance: 30-minute acquisition with zero dropped internal frames and correct synchronized timestamps.

## Phase 3 — Orientation
Implement Mahony filter with fixed V1 gains and coordinate transform. Add replay tests using recorded static/rotational data.

## Phase 4 — Gait + ZUPT
Implement FSM, initial-contact/toe-off/foot-flat, ZUPT error-state update, cycle metrics, speed and unilateral symmetry proxy.

Acceptance: replay suite passes and events are deterministic.

## Phase 5 — Reference + error engine
Implement profile capture, median/MAD reference distribution, feature normalization, confidence and the exact weighted error score.

## Phase 6 — Haptics
Implement:
- six PWM outputs;
- 200 Hz PWM;
- 8-bit duty;
- spatial interpolation;
- error/confidence gating;
- hysteresis;
- safety time/duty limits;
- event logging.

## Phase 7 — Storage
Implement LittleFS, 4096-byte CRC blocks, current-session recovery, session footer, capacity guard.

## Phase 8 — WebSocket
Implement binary protocol in `08_WIRELESS_PROTOCOL.md`, including reconnection and backfill.

## Phase 9 — Dashboard
Implement Tauri 2 shell, React views and Rust backend. Keep raw data in storage and only send display aggregates at live-render rate.

## Phase 10 — Export
Implement raw CSV, gait CSV, event CSV, haptic CSV, metadata JSON, MATLAB Level-5 `.mat`, PDF report.

## Phase 11 — Verification
Execute `13_TEST_AND_VALIDATION_PLAN.md` using replay datasets first, then bench walking tests.

## Phase 12 — Release gate
Do not call the V1 implementation complete until:
- all fixed pins and addresses match;
- no battery/switch code exists;
- haptics are safe by construction;
- raw timestamps are preserved;
- Wi-Fi loss does not stop haptic control;
- recovery survives an interrupted write;
- replay determinism is verified;
- exports are traceable to raw data.
