# 06 — Error Score, Confidence and Haptic Engine

## 1. Error principle
Every valid right-leg gait cycle produces exactly one primary quantitative `error_score` in `[0,1]`.
- `0` = within the accepted reference envelope.
- `1` = very large deviation / maximum normalized deviation.

The score is not a clinical severity scale. It is an engineering research metric used to drive feedback.

## 2. Reference normalization
For each feature:
- reference center = median;
- reference spread = `1.4826 × MAD` with a minimum floor so zero-variance features do not divide by zero;
- robust deviation score:
```text
z = abs(x - median) / max(spread, epsilon)
d = clamp(z / 3.0, 0, 1)
```

## 3. Feature weights
Use this V1 weighted composition:
| Feature group | Weight |
|---|---:|
| Swing dorsiflexion deviation | 0.25 |
| Initial-contact plantarflexion deviation | 0.15 |
| Inversion/eversion deviation | 0.15 |
| Cycle timing deviation | 0.15 |
| Stance/swing ratio deviation | 0.10 |
| Cycle distance/step-length proxy deviation | 0.10 |
| Shank dynamic deviation | 0.10 |

`error_score = weighted_sum(d_i) / weighted_sum(active_weights)`.

Missing/low-quality features are removed from the denominator and reduce confidence.

## 4. Error classes
V1 uses exactly these classes:
1. `INSUFFICIENT_DORSIFLEXION`
2. `EXCESS_PLANTARFLEXION`
3. `INVERSION_DEVIATION`
4. `EVERSION_DEVIATION`
5. `TIMING_DEVIATION`
6. `OVERALL_DEVIATION`

The first five are feature-specific. `OVERALL_DEVIATION` is used when multiple feature deviations exist but no single class dominates.

A feature-specific class is active when its normalized deviation `d >= 0.35` and confidence is sufficient. If multiple classes are active, the class with the largest weighted contribution is the primary class; all active classes remain in the event log.

## 5. Confidence score
Compute:
- sensor quality: 0.30
- event quality: 0.25
- feature completeness: 0.20
- reference stability: 0.15
- ZUPT quality: 0.10

Each subscore is `[0,1]`.
```text
confidence = weighted_sum(subscores)
```

Feedback gating:
- `confidence >= 0.75`: haptic control allowed.
- `0.50 <= confidence < 0.75`: log/display error but do not start new haptic feedback.
- `< 0.50`: invalid/low-confidence cycle for feedback; no haptic.

## 6. Haptic philosophy
Use spatial feedback around the shank. The tactile cue represents the **direction in which the user should move away from the vibration**. This is consistent with directional vibrotactile gait-retraining approaches using medial/lateral shank cues. citeturn644476search10turn644476search13

## 7. Motor positions
Six motors evenly spaced around the shank circumference:
- M1 = anterior (0°)
- M2 = anterolateral (60°)
- M3 = posterolateral (120°)
- M4 = posterior (180°)
- M5 = posteromedial (240°)
- M6 = anteromedial (300°)

This arrangement gives a full 360° tactile direction basis and follows published six-vibrotactor lower-leg/shank array concepts. citeturn644476search2turn644476search7

## 8. Error -> correction direction
The correction vector is encoded by the opposite-side tactile cue:
- `INSUFFICIENT_DORSIFLEXION`: posterior cue -> M4.
- `EXCESS_PLANTARFLEXION`: posterior cue -> M4 (desired correction is dorsiflexion).
- `EXCESS_DORSIFLEXION` when separately represented in internal features: anterior cue -> M1.
- `INVERSION_DEVIATION`: medial cue -> M6 + M5.
- `EVERSION_DEVIATION`: lateral cue -> M2 + M3.
- `TIMING_DEVIATION`: M1 + M4 alternating pulse, because timing has no single spatial correction vector.
- `OVERALL_DEVIATION`: use the vector sum of all active correction vectors and distribute vibration to the two nearest directional motors.

The agent must implement the motor map as data/configuration, not scattered constants.

## 9. Direction interpolation
For directional errors, map the desired correction angle onto the six motor basis directions. Activate the two nearest motors with barycentric/linear weights. This allows intermediate tactile directions rather than a crude one-motor classifier.

## 10. Haptic PWM
- PWM frequency: **200 Hz**.
- Resolution: **8-bit** (`0..255`).
- Minimum active duty: **20%** (`51`).
- Maximum duty: **80%** (`204`).
- At error/confidence activation, intensity:
```text
p = pow(error_score, 1.5)
pwm = 51 + (204 - 51) * p * confidence
pwm = clamp(round(pwm), 51, 204)
```

## 11. Haptic persistence
When an error remains active:
- continuously re-evaluate every control cycle;
- reduce/increase intensity with the updated score;
- remain ON while score stays above the off threshold;
- stop once the score falls into the off band.

Hysteresis:
- ON threshold: `error_score >= 0.35`.
- OFF threshold: `error_score < 0.25`.
- A new haptic episode cannot begin until the previous episode has actually entered OFF state.

## 12. Safety limits
- Maximum continuous ON time per motor: **5 s**.
- Rolling duty limit: **50% across any 10 s window**.
- Any sensor fault, stale sample, invalid step, watchdog reset or initialization fault -> all motors OFF.
- No dashboard command can directly force a motor ON during a live rehabilitation session.

## 13. Haptic event logging
Every ON/update/OFF episode records:
- timestamp;
- session/segment/cycle/step ID;
- motor IDs;
- PWM duty;
- duration;
- error class;
- error score;
- confidence;
- safety/fault reason if interrupted.
