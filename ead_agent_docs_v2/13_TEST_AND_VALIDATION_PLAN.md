# 13 — Test and Validation Plan

## 1. Hardware bring-up tests
1. Verify both MPU6050 devices enumerate at `0x68` and `0x69`.
2. Verify no I²C address collision.
3. Verify 100 Hz frame cadence over a 5-minute run.
4. Verify both INT lines generate data-ready events.
5. Verify all six PWM outputs stay LOW at boot.
6. Verify each motor independently.
7. Verify flyback diode polarity physically before motor testing.
8. Verify haptic safety cutoff after 5 s continuous command.

## 2. Sensor tests
- 5-minute static log: accel magnitude should remain close to 1 g.
- gyro bias stability check.
- controlled axis rotations to verify X/Y/Z signs.
- verify both sensors use the same anatomical orientation.
- compare relative orientation during known foot/shank rotations.

## 3. Gait algorithm tests
Use recorded datasets and replay them deterministically.
Test:
- normal walking;
- slow walking;
- variable cadence;
- intentional dorsiflexion deviation;
- plantarflexion deviation;
- inversion/eversion deviation;
- temporary sensor noise;
- dropped samples;
- static periods;
- invalid sensor values.

## 4. ZUPT tests
Confirm that:
- valid foot-flat intervals are detected;
- false ZUPTs are rejected during swing;
- velocity drift is materially reduced;
- poor-quality sessions are flagged instead of silently reporting fabricated distance.

## 5. Haptic tests
Confirm:
- no haptic on low confidence;
- larger error yields larger PWM;
- intensity falls as error falls;
- OFF threshold is below ON threshold;
- correct spatial motor pair is selected;
- timing error uses temporal dual-pole cue;
- 5 s maximum continuous ON is enforced;
- rolling 10 s duty limit is enforced.

## 6. Wireless tests
- disconnect Wi-Fi for 60 s while walking;
- verify gait/haptic continues;
- reconnect;
- verify missing telemetry is backfilled from storage when available;
- verify no duplicate cycle IDs.

## 7. Storage tests
- power/reset during block write;
- corrupt payload CRC;
- truncate final block;
- reboot;
- verify automatic recovery to last valid block.

## 8. Dashboard tests
- live charts;
- event markers;
- raw-to-error traceability;
- export CSV;
- export `.mat`;
- PDF report;
- session segmentation;
- reference lock.

## 9. Replay requirement
A saved raw dataset plus fixed configuration must reproduce the same:
- gait events;
- cycle features;
- error score;
- error class;
- haptic decisions.

This deterministic replay requirement is mandatory for algorithm development and regression testing.
