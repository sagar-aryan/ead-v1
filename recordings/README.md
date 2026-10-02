# Recordings

Real device recordings kept as fixtures, so the detector can be changed and
re-checked against measured ground truth instead of against a synthetic signal.

Replay one with:

```
g++ -std=gnu++17 -O2 -I firmware/lib/ead_core/src -I firmware/include \
    -o /tmp/eadreplay tools/replay/main.cpp \
    firmware/lib/ead_core/src/ead/{calibration,mahony,gait,protocol,crc32,cobs}.cpp
/tmp/eadreplay recordings/walk6m-2026-09-18.eadlog --still-seconds 5 --mpu6500
```

A recording made by `eadprobe stats --record` since schema 5 carries the device's
configuration, and the replay converts its counts with it. Older recordings carry
none: `--mpu6500` supplies the MPU6500 build's scales and measured mount maps,
and without it the replay refuses rather than guess. Expected for the walk below:
6 valid cycles, 6.39 m.

| File | Ground truth | Notes |
|---|---|---|
| `walk6m-2026-09-18.eadlog` (MPU6500 build: `--mpu6500`) | 6.00 m course, 11–12 alternating steps, 6 right-foot strides | 40 s over Wi-Fi: 5 s standing, the walk, then standing. 4,010 frames, no gaps. Heel strikes verified at 6.95, 7.51, 9.17, 10.94, 12.74, 14.33 and 15.92 s by finding the acceleration peaks independently of the detector (TEST-030). |
