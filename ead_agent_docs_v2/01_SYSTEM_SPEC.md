# 01 — System Specification

## 1. Product definition
EAD V1 is a right-leg, barefoot, wearable error-augmentation system for gait monitoring/retraining after stroke. It estimates gait-state and movement features from two MPU6050 IMUs and applies spatially directional vibrotactile feedback through six ERM motors. The device itself makes the haptic decision; the desktop application is for observation, configuration, storage, analysis and export.

## 2. Physical configuration
### Foot module
- Location: dorsum/top of the right foot, approximately the midfoot region.
- Mounting: rigid, flat, repeatable.
- Orientation: X forward toward toes; Y medial/left; Z upward away from the ground.
- I²C address: `0x68`.

### Shank module
- Location: anterior/anteromedial right lower shank, on/near the tibial shaft, approximately 10–15 cm below the knee.
- Mounting: rigid, flat, repeatable.
- Same axis orientation as the foot IMU.
- I²C address: `0x69`.

### Haptic band
- Circumferential band around the lower shank.
- Six ERMs equally distributed around the circumference at 60° increments.
- Haptics are intentionally separated from the IMU boards to reduce motion/vibration contamination.

## 3. What V1 directly measures/estimates
- Raw acceleration and angular velocity from both IMUs.
- Relative and segment orientation features.
- Heel-strike/initial-contact estimate.
- Toe-off estimate.
- Foot-flat/zero-velocity intervals.
- Stance and swing durations.
- Right-leg gait cycle time.
- Cadence estimate.
- ZUPT-corrected velocity/step-length/distance estimates where confidence is sufficient.
- Foot dorsiflexion/plantarflexion-related deviation.
- Foot inversion/eversion-related deviation.
- Timing deviation.
- Overall cycle deviation.
- Within-right-leg cycle-repeatability metric, explicitly labelled as a **unilateral symmetry proxy**, not bilateral left-vs-right symmetry.

## 4. Explicit non-claims
V1 MUST NOT claim direct measurement of:
- true knee joint angle;
- true hip joint angle;
- bilateral left/right step symmetry;
- ground-reaction force;
- plantar pressure.

The original project description includes knee-angle estimation, but with only foot + shank sensors this is not a directly observable quantity; a third thigh IMU would be required. fileciteturn9file1L88-L92

## 5. Real-time loop
```text
100 Hz acquisition
  -> calibration
  -> filtering
  -> orientation estimation
  -> gait state/event detector
  -> ZUPT/state correction
  -> step/cycle finalization
  -> feature extraction
  -> reference comparison
  -> error score + confidence
  -> haptic controller
  -> storage + wireless telemetry
```
The haptic controller must not wait for Wi-Fi or the desktop application.

## 6. Reference principle
Every evaluation session uses a versioned, patient-specific reference gait. No generic healthy-population reference is used by default. The reference remains locked during evaluation unless the researcher explicitly starts a new reference-capture workflow.

## 7. Research data principle
Raw data is canonical. Every processed feature, gait event, error and haptic command must be traceable to raw timestamps and step/cycle IDs.
