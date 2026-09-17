# EAD-V1 Firmware

Seeed XIAO ESP32-S3 with two IMUs (foot `0x68`, shank `0x69`). Contract:
`../ead_agent_docs_v2/` (`CONFIG_V1.json` holds the fixed values). As-built
hardware and verification status: `../docs/hardware.md`.

## Current state

Bring-up firmware (milestone M0). It configures both IMUs, verifies the
configuration by readback, and prints anatomical-frame accel/gyro at 10 Hz over
USB. The motor GPIOs are held LOW; no motor drivers are fitted and there is no
haptic code (DEC-006). Milestone M1 replaces `src/main.cpp` with data-ready
acquisition and the binary protocol.

## Layout

```text
firmware/
  platformio.ini        env seeed_xiao_esp32s3: espressif32 @ 7.1.3, gnu++17, no external libs
  src/main.cpp          bring-up: IMU init + readback, 10 Hz text output, 'c' diagnostics
  include/config_v1.h   fixed V1 values; per-sensor mount maps with compile-time checks
  lib/ead_codec/        WS frame header, EAD1 block header, CRC32 (not yet linked)
  test/                 placeholder; real Unity tests arrive in M1
```

## Build and flash

```bash
pio run                      # build
pio run -t upload            # flash over USB-C
pio device monitor -b 115200 # text output; send 'c' for register diagnostics
```

Expected boot log: I²C scan finds `0x68` and `0x69`, both report
`WHO_AM_I=0x70 (MPU6500)`, then `Foot OK  Shank OK`. A readback mismatch prints
the register, value read and value expected.

## Output lines

- `F[ok] a=… g g=… dps INT=… | S[ok] …` — human-readable, anatomical frame.
- `RAW,fax,fay,faz,sax,say,saz` — chip-frame accel in g (diagnoses mounting).
- `CSV,fax,fay,faz,fgx,fgy,fgz,sax,say,saz,sgx,sgy,sgz,fi,si` — anatomical frame,
  consumed by `../tools/orient_viewer.py`.

The `INT` columns are 10 Hz `digitalRead` samples of 50 µs pulses; they say
nothing about whether the interrupt lines work (tested in M1).

## Mount maps

`anat = M · chip`, applied to accel and gyro. Foot: identity. Shank:
`X = −chipZ, Y = +chipX, Z = −chipY`. A map that is not a proper rotation fails
to compile. See DEC-009 and PROB-002.
