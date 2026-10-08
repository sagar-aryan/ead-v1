# EAD-V1 Firmware

Seeed XIAO ESP32-S3 with two BNO086 IMUs on SPI (foot and shank) and six ERM motors
through the driver PCB, wired to DEC-016 (`../docs/wiring_reference.md`). Contract:
`../ead_agent_docs_v2/` (`CONFIG_V1.json` holds the fixed values). As-built hardware:
`../docs/hardware.md`. Wire protocol: `../docs/protocol.md`.

## Current state

Acquires both IMUs at 200 Hz, clocked by the foot gyroscope, with each sensor's own
orientation and a native 250 Hz accelerometer stream, and streams the binary
protocol over Wi-Fi and USB. About 4.5 minutes of telemetry is held in PSRAM so a host
can recover anything it missed.

On the device: static calibration, orientation per sensor, gait events from the
shank's swing, ZUPT and stride distance, cycle features, the reference builder, the
error engine and graded directional haptic cues (portable C++ in `lib/ead_core`, also
built on the host for tests and replay). Session kinds CALIBRATION,
REFERENCE_CAPTURE, REFERENCE_CHECK and EVALUATION; SERVICE_TEST pulses single motors
for the Check view.

## Layout

```text
firmware/
  platformio.ini        seeed_xiao_esp32s3 + native test env; gnu++17, no external libs
  include/config_v1.h   fixed V1 values, DEC-016 pins with compile-time checks, mount maps
  lib/ead_core/         portable: codec, COBS, CRC-32, message ring, config section,
                        SH-2 codec, calibration, Mahony, gait, reference builder,
                        error engine, haptics, motor guard
  src/bno086.*          SH-2 over SPI driver for both sensors
  src/acquisition.*     sensor reports, frame assembly
  src/motors.*          motor gates (LOW from boot), service-test pulses
  src/feedback.*        haptic cues during evaluations, on-time limits
  src/telemetry.*       durable message ring (PSRAM) and the processing task
  src/link.*            protocol endpoint: replies, status, streaming, backfill
  src/link_usb.*        USB Serial/JTAG transport (COBS framing)
  src/link_wifi.*       access point + WebSocket transport
  src/device.*          identity, faults, counters, HELLO/STATUS/CONFIG builders
  src/calibration_service.*  still-window calibration
  src/orientation.*     Mahony per sensor, quaternions into each frame
  src/gait_service.*    gait engine, EVENT_BATCH and STEP_BATCH
  src/session_service.* session kinds, reference capture, scoring
  bench/                not product: bno086 (DEC-016 pins), padstate
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
may print to USB: a stray byte would corrupt a frame,
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

## Wiring

DEC-016, every pin with its evidence: `../docs/wiring_reference.md`. Motors M1–M6 on
GPIO 4, 6, 42, 5, 2, 1 (DEC-029), held LOW as the first action at boot.

## Mount maps

`anat = M · chip`, applied to accel and gyro. Per-sensor maps are
measured on the leg (TEST-051) and kept in `include/config_v1.h`. A map that is not a
proper rotation fails to compile (DEC-009).
