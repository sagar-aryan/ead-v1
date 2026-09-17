# 05 — Gait State, Events, ZUPT, Speed and Cycle Metrics

## 1. Naming
Because only the right leg is instrumented, V1 uses **gait cycle** as the authoritative repeating unit: right initial-contact to the next right initial-contact. Do not call that interval a conventional bilateral “step time.”

## 2. State machine
Use these states:
- `INIT`
- `SWING`
- `CONTACT_TRANSITION`
- `STANCE`
- `FOOT_FLAT_ZV`
- `PRE_SWING`
- `FAULT`

State transitions are driven by both IMUs and temporal guards. The detector must not rely on one noisy channel.

## 3. Event detector
Use the shank angular velocity plus foot dynamics as the primary event signals.

### Initial-contact / heel-strike estimate
Candidate condition:
- current state is SWING;
- foot/shank angular speed is decreasing toward contact;
- foot vertical/forward dynamics show a local impact feature;
- contact candidate is sustained for at least 30 ms.

Choose the event timestamp as the strongest local impact/contact feature within the accepted 120 ms candidate window.

### Toe-off estimate
Candidate condition:
- current state is STANCE/FOOT_FLAT;
- foot/shank angular velocity rises above adaptive swing threshold;
- acceleration dynamics leave the stance envelope;
- candidate persists for at least 40 ms.

### Adaptive thresholds
Build initial event thresholds from the first valid cycles of the current reference/calibration data using robust statistics:
- center = median;
- spread = `1.4826 × MAD`;
- dynamic threshold = center + `3 × spread` where a high threshold is required;
- dynamic threshold = center - `3 × spread` where a low threshold is required.
Clamp thresholds to physically reasonable bounds to prevent a corrupted calibration from creating an extreme threshold.

## 4. Temporal guards
- Minimum accepted right-leg cycle time: 0.45 s.
- Maximum accepted right-leg cycle time: 3.00 s.
- Reject overlapping/duplicate event candidates.
- Do not emit a new cycle until a complete next initial-contact is detected.

## 5. Stance/swing
- Stance begins at accepted initial contact.
- Swing begins at accepted toe-off.
- Stance ends at toe-off.
- Swing ends at next initial contact.

## 6. ZUPT detector
ZUPT is a **zero-velocity update**, not a physical sensor reset.

Foot-only detector inputs:
- calibrated acceleration magnitude;
- calibrated gyro magnitude;
- low-pass foot dynamics;
- gait state.

A candidate zero-velocity window requires:
- `| |a| - 1 g | <= 0.15 g`;
- gyro magnitude `<= 25 °/s`;
- both conditions continuously satisfied for at least 60 ms;
- candidate occurs in STANCE/FOOT_FLAT context.

Use 20 ms entry hysteresis and require the conditions to remain valid for the full 60 ms before applying the update.

## 7. ZUPT state update
Use the standard zero-velocity measurement model with a foot-velocity state constrained toward `[0,0,0]` during a valid ZUPT interval. For V1, use an error-state Kalman update for the velocity substate rather than simply hard-setting a value on every sample.

The integrated position is not blindly reset at every ZUPT. ZUPT primarily removes accumulated velocity drift; position remains the integral of the corrected state.

## 8. Speed and distance
The authoritative V1 speed is **mean walking speed over valid gait cycles**:
```text
cycle_speed = estimated_cycle_distance / cycle_time
session_speed = distance_sum / elapsed_valid_walking_time
```
Use only cycles with adequate ZUPT quality. Report:
- instantaneous/rolling speed;
- session mean speed;
- total estimated walking distance;
- speed quality/confidence.

When ZUPT quality is inadequate, flag speed/distance as low-confidence instead of inventing a corrected number.

## 9. Cadence
For a unilateral instrumented leg:
```text
cadence_steps_per_min = 120 / cycle_time_seconds
```
This converts a right-foot-to-right-foot cycle into conventional steps/minute under the assumption of alternating left/right steps.

## 10. Unilateral symmetry proxy
True left-vs-right symmetry is unavailable. V1 therefore reports **cycle repeatability / unilateral symmetry proxy**, computed between consecutive valid right-leg cycles:
```text
proxy = 1 - normalized_difference(current_cycle_features, previous_cycle_features)
```
Use the same robust feature normalization used by the error engine. Label this metric explicitly as `unilateral_cycle_symmetry_proxy` everywhere.

## 11. Feature set per cycle
Primary features:
- cycle time;
- stance time;
- swing time;
- stance ratio;
- swing ratio;
- peak/terminal dorsiflexion-related foot angle in swing;
- initial-contact plantarflexion-related angle;
- peak inversion/eversion-related angle;
- peak shank angular speed;
- ZUPT quality;
- estimated cycle distance/step-length proxy where valid.
