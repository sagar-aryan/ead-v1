# Implementation

Phase reference: `ead_agent_docs_v2/14_IMPLEMENTATION_PLAN.md` Phase 1
(Repository skeleton). Module structure reference:
`ead_agent_docs_v2/07_FIRMWARE_ARCHITECTURE.md`.

## Phase 1 — Repository skeleton

### Objective
Create firmware and dashboard projects with the module structure in doc 07,
so later phases (sensor → orientation → gait/ZUPT → error → haptics →
storage → WebSocket → dashboard → export → verification) have a place to land.

### Status (2026-09-16)
- Observed: `firmware/` exists with `platformio.ini`, `src/`, `include/`,
  `lib/`, `test/` (parallel scaffolding by main agent).
- `firmware/platformio.ini` targets `seeed_xiao_esp32s3` (Arduino framework),
  LittleFS enabled, USB CDC on boot, Adafruit MPU6050 + WebSockets deps.
- `docs/` scaffolding (this task) is new; dashboard skeleton not yet present.
- No Phase 2+ algorithm code exists. Status: skeleton In Progress, no
  functional firmware behavior verified.

### Intended firmware module layout (per doc 07, to be created under `firmware/src/`)
- `hardware/` — mpu6050, pwm_haptics, gpio, timer
- `sensor/` — acquisition, calibration, filtering, orientation
- `gait/` — state_machine, events, zupt, metrics
- `reference/` — profile, builder, loader
- `error/` — score, confidence, classifier, hysteresis
- `storage/` — littlefs, block_writer, recovery, session_manager
- `protocol/` — websocket, codec, commands
- `safety/` — sensor_watchdog, haptic_limits
- `app_state/` — device_state

### Important Files
- `firmware/platformio.ini` — board/env/deps/filesystem config.
- `ead_agent_docs_v2/07_FIRMWARE_ARCHITECTURE.md` — authoritative module map.
- `ead_agent_docs_v2/03_GPIO_PIN_MAP.md` — fixed pin assignments (not to be invented).

### Edge Cases
- None handled yet; acquisition must eventually tolerate dropped frames,
  invalid samples, and static periods (tested in Phase 11 replay suite).

### Limitations
- Skeleton only. No sensor acquisition, orientation, gait, haptic, storage,
  or protocol code. No dashboard project yet.

### Verification
- Not verified. Phase 1 acceptance is implicit: project builds/flashes and
  module paths exist. `pio run` has not been executed in this task.
