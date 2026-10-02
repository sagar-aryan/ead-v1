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
| `walk10m-normal1-2026-10-02.eadlog` | 10.00 m course out and back; right-foot landings 9 out, 9 back | BNO086 build, normal pace. 6,500 frames. |
| `walk10m-normal2-2026-10-02.eadlog` | as above, 9 and 9 | Normal pace. 5,940 frames. Two short steps in the turn. |
| `walk10m-normal3-2026-10-02.eadlog` | as above, 9 and 9 | Normal pace. 6,660 frames. |
| `walk10m-slow-2026-10-02.eadlog` | as above, 11 and 11 | Slow pace. 6,050 frames. |
| `walk10m-fast-2026-10-02.eadlog` | as above, 8 and 8 | Fast pace. 4,770 frames. |
| `walk6m-2026-09-18.eadlog` (MPU6500 build: `--mpu6500`) | 6.00 m course, 11–12 alternating steps, 6 right-foot strides | 40 s over Wi-Fi: 5 s standing, the walk, then standing. 4,010 frames, no gaps. Heel strikes verified at 6.95, 7.51, 9.17, 10.94, 12.74, 14.33 and 15.92 s by finding the acceleration peaks independently of the detector (TEST-030). |

The `walk10m-*` files are dashboard sessions written out by
`tools/session2eadlog.py` (TEST-052): the session's configuration, then its raw
frames, so the replay needs no flag. Worn by the developer (ID `DEV-1`), firmware
0.1.0+9c91b7d with the measured BNO086 maps; each starts and ends with about 5 s
standing, and has a stand, a turn and a stand between the legs. Replay with
`--still-seconds 5`. Results per engine version are in docs/testing.md (TEST-052
onward).
