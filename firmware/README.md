# EAD-V1 Firmware Skeleton

Right-leg, barefoot, dual-MPU6050 gait error-augmentation device on
Seeed XIAO ESP32-S3. Spec: `../ead_agent_docs_v2/` (authoritative;
`CONFIG_V1.json` is the fixed-value source of truth).

## Layout

```text
firmware/
  platformio.ini          env:seeed_xiao_esp32s3, Arduino, LittleFS, USB-CDC flags
  src/main.cpp            setup()/loop() skeleton
  include/config_v1.h     all fixed V1 values from CONFIG_V1.json
  lib/ead_codec/          WS binary header + EAD1 block envelope + CRC32
  test/                   Unity replay-test placeholder (determinism required)
```

## Build

```bash
pip install platformio
pio run                   # builds env:seeed_xiao_esp32s3
pio run -t upload         # flash over USB-C
pio device monitor -b 115200
pio test                  # Unity tests (once written)
```

## Wiring reference (docs 02/03 — summary, see docs for authority)

- I²C bus: XIAO GPIO5 (D4, SDA) + GPIO6 (D5, SCL), 400 kHz, shared by both IMUs.
  XIAO 3V3/GND to both MPU VCC/GND; on-board breakout pull-ups used.
- Foot MPU AD0→GND = `0x68`; shank MPU AD0→3V3 = `0x69`.
- DRDY INTs: foot→GPIO7 (D8), shank→GPIO8 (D9).
- Motors M1–M6 PWM → GPIO 1, 2, 4, 9, 43, 44. Each via 100R→IRLML6344 gate
  (100k gate pulldown), low-side switch, 1N5819W flyback (stripe to ERM+),
  3V3_HAPTIC rail split A (M1–M3) / B (M4–M6), 220 µF bulk per rail.
- Boot: all six motor GPIOs OUTPUT + LOW before PWM start; stay OFF until
  READY with sensor health checks passing. No battery/switch/BLE/BNO/FSR in V1.

## Skeleton scope

Includes: USB-CDC boot banner, `DeviceState` enum (BOOT…RECOVERY),
motor-GPIO safe init, I²C 400 kHz stub, 100 Hz timer placeholder,
`config_v1.h`, `ead_codec` (WS + EAD1 + CRC32).
NOT included yet: MPU6050 driver, Mahony, gait/ZUPT FSM, haptic engine,
LittleFS writer, WebSocket AP/telemetry — per doc 07/14 build order.
