# Architecture

This is the target design. What is implemented today is tracked in
`docs/implementation.md`; as-built hardware facts are in `docs/hardware.md`.

## Overview

EAD V1 is a right-leg, barefoot, wearable error-augmentation system for gait
monitoring and retraining after stroke (`ead_agent_docs_v2/01_SYSTEM_SPEC.md`).
Two IMUs (foot + shank) feed a 200 Hz real-time loop on a XIAO ESP32-S3 (100 Hz
before schema 6). Orientation comes from each BNO086's own fusion (game rotation
vector, DEC-021; Mahony 6-DoF on the ESP32 before schema 6). The loop detects gait events, applies
foot-only ZUPT correction, and scores each gait cycle against a patient-specific
reference. A Tauri 2 desktop dashboard observes, configures, records, analyses
and exports; it never closes a control loop.

Haptic feedback is part of the V1 contract, but no error-driven feedback exists
(DEC-006); the motors run only as service-test pulses (DEC-018).

The hardware is the DEC-016 build: two BNO086 on SPI plus six motor channels on a
separate ERM driver board, with the haptic band on the calf (DEC-015). The product
firmware supports it since schema 5 (DEC-017). The MPU6500/I²C build it replaced is
described at the end of the components, because every recording before 2026-10-02
came from it.

## Components

### Foot and shank sensors (BNO086, DEC-016/017)
- Foot: dorsum of the right foot; CS GPIO43, INT GPIO39. Shank: anterior shin,
  10–15 cm below the knee; CS GPIO44, INT GPIO40.
- One SPI bus (SCK 7, MISO 8, MOSI 9, 1 MHz, mode 3); RST (41) and WAKE (3) shared.
- Calibrated accelerometer (±8 g, 250 Hz), gyroscope (±2000 °/s, 200 Hz) and game
  rotation vector (200 Hz) reports, read on INT by our own SH-2 code
  (`src/bno086.cpp`, `lib/ead_core/src/ead/sh2.cpp`); rates from TEST-054.
- Mount maps: measured on the leg (TEST-051; DEC-009: per sensor).

### Haptic band (no haptic code)
Contract: six ERMs around the lower shank (moved to the calf by DEC-015), low-side
IRLML6344 switches on GPIO 1, 2, 4, 9, 43, 44 (doc 03; GPIO 1, 2, 42, 4, 5, 6 under
DEC-016), 200 Hz PWM limited to 20–80 % duty, 5 s maximum on-time,
50 % rolling duty over 10 s. Current build: all six motor GPIOs are driven LOW at boot;
the only drive is a service-test pulse (DEC-018), timed on the device within those
limits.

### ESP32-S3 real-time controller
- **Acquisition** (core 1, highest priority): each sensor's INT (IRAM-safe handler)
  timestamps its packet; the task reads it over SPI within the 1 ms rule, gives each
  sample its own time from the SH-2 timestamps, and emits a frame per foot gyroscope
  sample with the shank's latest. It also runs the per-wire sensor check at boot and on
  request (SERVICE_TEST).
- **Processing** (core 1): calibration, orientation, gait state machine and
  events, ZUPT, cycle features, reference builder/loader, error score, classes
  and confidence. Portable C++ in `lib/ead_core`, also built on the host for
  deterministic replay tests.
- **Links** (core 0): Wi-Fi access point serving the WebSocket endpoint, and USB
  carrying the same binary messages (DEC-005). A PSRAM ring keeps recent
  telemetry so a reconnecting dashboard can backfill gaps.
- **Device states** (doc 07 §6): BOOT, SELF_TEST, CALIBRATING,
  REFERENCE_CAPTURE, READY, RUNNING, PAUSED, FAULT, RECOVERY. A reference check
  runs in REFERENCE_CAPTURE; RUNNING is an evaluation session only (DEC-008).
- **Priority:** acquisition > safety/fault checks > gait/orientation > storage >
  links. Links never block acquisition.
- **Motors** (`src/motors.cpp`): LEDC at 200 Hz, 8 bit; a pulse is accepted by the
  portable `MotorGuard` and ended by an `esp_timer`, so it stops whatever the link does.
- No battery, charger, switch, BLE or FSR code (contract).

### Previous build: MPU6500 on I²C (until 2026-10-01)
Foot `0x68` (INT GPIO7, identity map) and shank `0x69` (INT GPIO8, map
`X = −chipZ, Y = +chipY, Z = +chipX`, measured in TEST-027) on one I²C bus (GPIO5/6,
400 kHz), ±4 g, ±500 °/s, 42 Hz DLPF, frames clocked by the foot data-ready interrupt.
Its sessions store configuration format 1; the replay tool reads them with `--mpu6500`.

### Dashboard (Tauri 2 + React + TypeScript + Rust)
- **Rust backend:**
  - protocol codec;
  - link layer (WebSocket, USB);
  - device manager (handshake, keepalive, command acknowledgements, gap
    backfill);
  - SQLite store (patients, references, sessions, segments, raw frames with
    min/max summaries, cycles, events);
  - 20 Hz live aggregation;
  - exports (CSV, metadata JSON, Level-5 `.mat`, PDF).
- **React frontend:** the doc 11 areas (LIVE, TRENDS, CYCLES, RAW, EVENTS,
  HAPTICS, REFERENCES, SESSIONS, EXPORT), a device drawer, calibration and
  mounting-check flows. Stack choices: DEC-011.
- Raw data is canonical; the UI only receives downsampled or summarised data.

## Data flow

```text
foot DRDY ISR ──► acquisition: read foot + shank, µs timestamp, frame index
                     │  raw chip-frame counts (DEC-007)
                     ▼
processing: mount map → calibration → segment orientation from each game rotation
            vector (`ead/feed`, DEC-021) → relative orientation
            → gait state machine + events (shank swing, DEC-022) → ZUPT (foot)
            → cycle features
            → reference comparison → error score, class, confidence
                     │
                     ▼
durable message ring (RAW_SAMPLE_BATCH / EVENT_BATCH / STEP_BATCH, sequence numbers)
      │                                 │
      ▼                                 ▼
Wi-Fi AP WebSocket                  USB COBS frames
192.168.4.1:8080/ws                 /dev/ttyACM*
      └───────────────┬─────────────────┘
                      ▼
dashboard backend: decode → SQLite (raw, summaries, cycles, events)
                      ├─► 20 Hz live channel → React views
                      └─► export package (CSV, metadata.json, session.mat, report.pdf)
                      └─► exports: CSV package, session.mat, PDF report
```

Commands flow the other way: SESSION_START (with kind, segment limits and locked
reference), SESSION_STOP, PAUSE, RESUME, CONFIG_GET/SET and BACKFILL_REQUEST.
They take effect between frames, and the ACK reports the effective frame index.

## Interfaces

| Interface | Detail |
|---|---|
| I²C | 400 kHz, foot `0x68`, shank `0x69` |
| GPIO interrupts | Foot INT → GPIO7, shank INT → GPIO8; both verified at 100 Hz (TEST-015) |
| Motor GPIOs | 1, 2, 4, 9, 43, 44 held LOW (not fitted) |
| Wi-Fi | Device AP, `ws://192.168.4.1:8080/ws`, binary frames (doc 08 header) |
| USB | Native USB Serial/JTAG `303a:1001`: flashing, and the same binary protocol framed `00 \| COBS(msg ‖ CRC32) \| 00` |
| Protocol payloads | `docs/protocol.md` |
| Dashboard IPC | Tauri commands; live data over `ipc::Channel`; RAW ranges as binary responses |

## Dependencies

| Component | Dependency | Why |
|---|---|---|
| Firmware | PlatformIO 6.2.0, `espressif32 @ 7.1.3` (Arduino-ESP32 2.0.17, ESP-IDF 4.4) | DEC-001 |
| Firmware | No external libraries | IMU driver is register-level; WebSocket via ESP-IDF `esp_http_server` (DEC-010) |
| Dashboard | Tauri 2.11, React 18, TypeScript 5, Vite 5 | Doc 11 |
| Dashboard | rusqlite, tokio-tungstenite, serialport, uPlot, krilla | DEC-011; each added in the milestone that first uses it |
| Verification | Python 3 with pyserial, numpy, scipy | Independent protocol decoder, reference maths, `.mat` reader |

## Constraints

- Right leg only. No bilateral symmetry claim: the metric is the unilateral cycle
  symmetry proxy. No knee/hip angle, ground reaction force or plantar pressure claims.
- No magnetometer and no absolute yaw claim.
- Raw timestamps are preserved end to end. Every feature, event and error traces
  to raw frames and a cycle ID.
- Loss of Wi-Fi never stops device processing.
- Default partition table leaves 1.5 MB for on-device data; the storage
  milestone (M7) needs its own decision before implementation.
