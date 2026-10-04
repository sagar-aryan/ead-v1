# Recordings

Real device recordings kept as fixtures, so the detector can be changed and
re-checked against measured ground truth instead of against a synthetic signal.

Replay one with:

```
g++ -std=gnu++17 -O2 -I firmware/lib/ead_core/src -I firmware/include \
    -o /tmp/eadreplay tools/replay/main.cpp \
    firmware/lib/ead_core/src/ead/{calibration,mahony,gait,protocol,crc32,cobs,feed}.cpp
/tmp/eadreplay recordings/walk6m-2026-09-18.eadlog --still-seconds 5 --mpu6500
```

A recording made by `eadprobe stats --record` since schema 5 carries the device's
configuration, and the replay converts its counts with it. Older recordings carry
none: `--mpu6500` supplies the MPU6500 build's scales and measured mount maps,
and without it the replay refuses rather than guess. Expected for the walk below:
5 valid cycles, 5.83 m (DEC-022; it read 6 and 6.39 m when the first step's
push-off was taken for a contact).

| File | Ground truth | Notes |
|---|---|---|
| `walk10m-normal1-2026-10-02.eadlog` | 10.00 m course out and back; right-foot landings 9 out, 9 back | BNO086 build, normal pace. 6,500 frames. |
| `walk10m-normal2-2026-10-02.eadlog` | as above, 9 and 9 | Normal pace. 5,940 frames. Two short steps in the turn. |
| `walk10m-normal3-2026-10-02.eadlog` | as above, 9 and 9 | Normal pace. 6,660 frames. |
| `walk10m-slow-2026-10-02.eadlog` | as above, 11 and 11 | Slow pace. 6,050 frames. |
| `walk10m-fast-2026-10-02.eadlog` | as above, 8 and 8 | Fast pace. 4,770 frames. |
| `walk10m-normal1-2026-10-04.eadlog` | 10.00 m out and back; 8 right-foot landings per leg (walker's count) | Schema 6, 200 Hz, Wi-Fi, barefoot on carpet. `--still-from 8`. Video = data + 9.1 s. |
| `walk10m-normal2-2026-10-04.eadlog` | as above, 8 and 8 | `--still-from 7`. Video = data − 2.9 s. |
| `walk10m-normal3-2026-10-04.eadlog` | as above, 8 and 8 | `--still-from 0`. Video = data − 3.1 s. |
| `walk10m-slow1-2026-10-04.eadlog` | as above, 9–10 per leg; shank swings 10 and 9 | `--still-from 51.5`. Video = data − 2.7 s. |
| `walk10m-slow2-2026-10-04.eadlog` | as above, 9–10 per leg; shank swings 9 and 8 | `--still-from 33`. Video = data − 2.8 s. No stand between the stamps and the walk, or at the far end. |
| `walk10m-fast1-2026-10-04.eadlog` | as above, 7 per leg | `--still-from 37`. Video = data − 2.7 s. No stand at the far end. |
| `walk3min-2026-10-04.eadlog` | back and forth over the 10 m course for about 3 min; no per-leg count | `--still-from 187`. Video = data − 2.6 s. |
| `walk6m-2026-09-18.eadlog` (MPU6500 build: `--mpu6500`) | 6.00 m course, 11–12 alternating steps, 6 right-foot strides | 40 s over Wi-Fi: 5 s standing, the walk, then standing. 4,010 frames, no gaps. Heel strikes at 7.51, 9.17, 10.94, 12.74, 14.33 and 15.92 s; the acceleration peak at 6.95 s that TEST-030 also listed is the first step's push-off (TEST-059). |

The `walk10m-*` files are dashboard sessions written out by
`tools/session2eadlog.py` (TEST-052): the session's configuration, then its raw
frames, so the replay needs no flag. Worn by the developer (ID `DEV-1`), firmware
0.1.0+9c91b7d with the measured BNO086 maps; each starts and ends with about 5 s
standing, and has a stand, a turn and a stand between the legs. Replay with
`--still-seconds 5`, or all of them, with the 2026-10-04 walks, against their counts with
`tools/replay/walks.py /path/to/eadreplay`. Results per engine version are in
docs/testing.md (TEST-052 onward).

The `2026-10-04` files (TEST-058) start with the walker on the way from the laptop
to the start line, so calibrate from a still window with `--still-from S` (the
first 3 s window the replay accepts). Each walk has three right-heel stamps at the
start line before the first step, for syncing with its video; the stamps are not
steps. The videos are kept off the repository; the offsets above line a frame's
time in the video up with the recording's time.
