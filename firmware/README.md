# EAD-V1 Firmware

Seeed XIAO ESP32-S3 with two IMUs (foot `0x68`, shank `0x69`). Contract:
`../ead_agent_docs_v2/` (`CONFIG_V1.json` holds the fixed values). As-built
hardware and verification status: `../docs/hardware.md`. Wire protocol:
`../docs/protocol.md`.

## Current state

Acquires both IMUs at 100 Hz, clocked by the foot sensor's data-ready interrupt,
and streams the binary protocol over Wi-Fi and USB. Roughly 12 minutes of
telemetry is held in PSRAM so a host can recover anything it missed.

No calibration, orientation or gait analysis yet (milestones M3–M5): the
quaternion fields in each frame are identity and session commands answer
NotSupported. The motor GPIOs are held LOW — no drivers are fitted and there is
no haptic code (DEC-006).

## Layout

```text
firmware/
  platformio.ini        seeed_xiao_esp32s3 + native test env; gnu++17, no external libs
  include/config_v1.h   fixed V1 values; mount maps with compile-time checks
  lib/ead_core/         portable: codec, COBS, CRC-32, message ring, config section
  src/imu.*             register driver, readback verification, bus recovery
  src/acquisition.*     data-ready interrupt, self-test, frame assembly
  src/telemetry.*       durable message ring (PSRAM) and the processing task
  src/link.*            protocol endpoint: replies, status, streaming, backfill
  src/link_usb.*        USB Serial/JTAG transport (COBS framing)
  src/link_wifi.*       access point + WebSocket transport
  src/device.*          identity, faults, counters, HELLO/STATUS/CONFIG builders
  test/                 Unity tests, run on the host against the golden vectors
  scripts/              build-time headers (firmware version, AP passphrase)
```

## Build, test and flash

```bash
pio run                 # build for the device
pio test -e native      # unit tests on the host
pio run -t upload       # flash over USB-C
```

## Talking to it

The USB port carries binary protocol frames, not text. Nothing in the firmware
may print to USB: a stray byte corrupts a frame (`../docs/problems.md` PROB-006),
which is why `CORE_DEBUG_LEVEL=0` is a build flag and Arduino `Serial` is unused.

```bash
python3 ../tools/eadprobe.py hello     # identity, sequence window
python3 ../tools/eadprobe.py config    # configuration, hash-verified
python3 ../tools/eadprobe.py stats --seconds 60
```

Expect zero missing frames, zero dropped, zero rejected, and |a| ≈ 1.02 g on a
resting device.

## Wi-Fi access point

`EAD-V1-<last two MAC bytes>`, WPA2, channel 6, device at `192.168.4.1`,
WebSocket at `ws://192.168.4.1:8080/ws`. The passphrase is generated at first
build into `include/ead_secrets.h`, which git does not track; delete that file to
roll a new one.

## Wiring reference (docs 02/03 — summary, see docs for authority)

- I²C bus: XIAO GPIO5 (D4, SDA) + GPIO6 (D5, SCL), 400 kHz, shared by both IMUs.
  XIAO 3V3/GND to both MPU VCC/GND; on-board breakout pull-ups used.
- Foot MPU AD0→GND = `0x68`; shank MPU AD0→3V3 = `0x69`.
- Data-ready interrupts: foot→GPIO7 (D8), shank→GPIO8 (D9). Both verified
  (TEST-015).
- Motors M1–M6 → GPIO 1, 2, 4, 9, 43, 44. Held LOW as the first action at boot;
  no drivers fitted. See PROB-004 before fitting any.

## Mount maps

`anat = M · chip`, applied to accel and gyro. Foot: identity. Shank:
`X = −chipZ, Y = +chipX, Z = −chipY`. A map that is not a proper rotation fails
to compile. See DEC-009 and PROB-002.
