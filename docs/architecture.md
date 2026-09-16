# Architecture

## Overview

EAD V1 is a right-leg, barefoot, wearable error-augmentation system for gait
monitoring/retraining after stroke (per `ead_agent_docs_v2/01_SYSTEM_SPEC.md`).
Two MPU6050 IMUs (foot + shank) feed a 100 Hz real-time loop on an ESP32-S3
(XIAO). The loop estimates orientation (Mahony 6-DoF), detects gait events
(FSM), applies foot-only ZUPT correction, scores per-cycle deviation against a
patient-specific reference, and drives spatially directional vibrotactile
feedback. A Tauri 2 desktop dashboard observes, configures, stores, and
exports data; it never closes the haptic loop during patient RUNNING.

Source docs: `ead_agent_docs_v2/01_SYSTEM_SPEC.md`,
`ead_agent_docs_v2/02_HARDWARE_WIRING.md`,
`ead_agent_docs_v2/07_FIRMWARE_ARCHITECTURE.md`.

## Components

### Foot IMU module
- Location: dorsum/top of right foot, midfoot region, rigid flat mounting.
- Sensor: MPU6050, I2C address `0x68` (AD0 → GND).
- Axes: X forward toward toes, Y medial/left, Z up. Same orientation as shank board.
- Config: ±4 g, ±500 °/s, DLPF 42 Hz (DLPF_CFG=3), divider 9, PLL X-gyro clock.
- INT → XIAO GPIO7/D8 (data-ready).

### Shank IMU module
- Location: anterior/anteromedial right lower shank, 10–15 cm below knee.
- Sensor: MPU6050, I2C address `0x69` (AD0 → 3V3).
- Same axis orientation and sensor config as foot IMU.
- INT → XIAO GPIO8/D9 (data-ready).
- Shared bus: XIAO GPIO5/D4 (SDA) + GPIO6/D5 (SCL) at 400 kHz. Onboard
  breakout pull-ups used; no additional network unless verified missing.

### Haptic band
- 6 × ERM coin motors, circumferential lower-shank band at 60° increments.
- Physically separated from IMU boards to reduce vibration contamination.
- Each motor low-side switched by IRLML6344 (100 Ω gate series, 100 kΩ
  gate pulldown), 1N5819W flyback diode (cathode → 3V3_HAPTIC).
- Power: two HT7833 rails (M1–M3 rail A, M4–M6 rail B; outputs never tied),
  220 µF bulk per rail, 1 µF in/out decoupling per regulator.
- 10 nF suppression cap footprint allowed, DNP by default.
- Drive: 200 Hz PWM, 8-bit, active duty 20–80% (51–204). Max 5 s continuous
  ON per motor; rolling limit 50% duty over any 10 s window.

### ESP32-S3 real-time controller (XIAO)
- Runs deterministic 100 Hz acquisition timer; both sensors read per frame
  with one master monotonic timestamp + frame sequence number.
- Processing priority: acquisition > safety/fault > gait/orientation >
  haptic control > storage flush > Wi-Fi telemetry.
- Wi-Fi/storage must never block acquisition or haptics.
- Device states: BOOT, SELF_TEST, CALIBRATING, REFERENCE_CAPTURE, READY,
  RUNNING, PAUSED, FAULT, RECOVERY. Motors OFF except RUNNING with valid
  haptic conditions. Service/test mode only outside patient sessions;
  events tagged `SERVICE_TEST`.
- Local recovery: internal flash, LittleFS, append-only 4096-byte CRC
  blocks, current/latest session only.
- Common `ImuSample` interface (sensor_id, imu_timestamp_us, ax/ay/az,
  gx/gy/gz, quaternion, calibration_state, health_flags); MPU6050 is the
  V1 backend, BNO086 reserved as future backend.
- No battery ADC, fuel gauge, charger, or switch-status code in V1.

### Dashboard (Tauri 2 + React + TypeScript + Rust)
- Roles: live observation, researcher configuration, session storage,
  analysis, export. Raw data canonical; live view uses display aggregates
  at render rate only.
- Reference capture/lock workflow, session segmentation, event markers,
  raw-to-error traceability, CSV / MATLAB `.mat` / metadata JSON / PDF export.
- Direct motor control forbidden during patient RUNNING.

## Data Flow

```text
100 Hz acquisition (both MPU6050, ESP32 timestamp + seq)
  -> calibration -> filtering -> Mahony orientation
  -> gait FSM (IC/toe-off/foot-flat) -> foot-only ZUPT error-state update
  -> cycle finalization + feature extraction
  -> patient-specific reference comparison
  -> error score [0,1] + confidence [0,1] + error class
  -> haptic controller (gating, hysteresis/deadband, spatial interpolation,
     safety limits) -> PWM outputs
  -> LittleFS block writer || WebSocket binary telemetry
  -> dashboard (Rust backend store -> React views -> CSV/.mat/PDF export)
```

- Transport: ESP32 local AP, `192.168.4.1:8080/ws`, binary framed protocol,
  reconnection + backfill from storage, no duplicate cycle IDs.
- Segmentation: researcher enters cycle/error limits in dashboard UI;
  firmware does not invent limits; whichever limit hits first ends segment.

## Interfaces

- I2C (400 kHz): ESP32 ↔ foot `0x68` / shank `0x69` MPU6050.
- GPIO interrupts: foot INT → GPIO7, shank INT → GPIO8.
- PWM: 6 ESP32 outputs → MOSFET gates (pin map per `03_GPIO_PIN_MAP.md`).
- USB-C CDC: debug/log/flash console.
- WebSocket `/ws` binary: telemetry up, commands down (service test only
  outside patient sessions).
- Dashboard IPC: React TS frontend ↔ Rust backend (storage, `.mat`/CSV/PDF).

## Dependencies

- Firmware: PlatformIO + Arduino framework (`espressif32`), Adafruit MPU6050
  lib, WebSockets lib, LittleFS (board_build.filesystem).
- Dashboard: Tauri 2, React, TypeScript, Rust backend.
- Host: Linux build/flash environment.

## Constraints

- Right leg only; no bilateral symmetry claim — unilateral cycle-repeatability
  proxy only. No direct knee/hip angle, GRF, or plantar pressure claims.
- No magnetometer; no absolute yaw claim (Mahony 6-DoF).
- Haptic safety limits enforced by construction (5 s / 50%-per-10 s /
  20–80% duty / confidence gating / hysteresis).
- Raw timestamps preserved end-to-end; every feature/event/error/haptic
  command traceable to raw timestamps + step/cycle IDs.
- Wi-Fi loss must not stop haptic control; recovery must survive
  interrupted writes.
