# 04 — Sensor Calibration and Orientation

## 1. Coordinate convention
Both IMUs use the same anatomical frame:
- **X+ = forward, toward the toes**.
- **Y+ = medial/left for the right leg, toward the big-toe/inner-leg side**.
- **Z+ = up, away from the ground**.

The supplied placement image is the physical assembly reference. Do not mirror the shank sensor or rotate one board 180° relative to the other.

## 2. Installation requirement
The boards should be:
- flat against the body segment as much as practical;
- rigidly attached;
- aligned with each other;
- mechanically repeatable between sessions.

Small installation angle error is handled by alignment calibration, but gross misorientation is not acceptable.

## 3. Startup static calibration
### Step A — sensor health
1. Confirm both I²C addresses answer (`0x68`, `0x69`).
2. Read `WHO_AM_I` and verify expected MPU6050 identity.
3. Read multiple samples and reject a sensor if values are frozen, NaN/invalid, or outside physical bounds.

### Step B — stationary bias estimate
Place the user standing still and keep both sensors still for **5 seconds**.
- Require low angular-rate variance.
- Require acceleration magnitude close to 1 g.
- Estimate gyro bias from the stationary mean.
- Estimate accelerometer bias after accounting for the expected gravity vector in the installed coordinate frame.
- Save calibration quality metrics.

If motion occurs during calibration, discard the window and request another 5-second stationary period.

## 4. Alignment calibration
After static bias estimation, derive an alignment rotation from the measured gravity vector and the known anatomical orientation. The alignment should make the gravity vector point to +Z while preserving the defined forward/medial directions. Do not use the magnetometer because MPU6050 has none.

The alignment transform is stored with the session/device calibration record.

## 5. Orientation algorithm
Use a **Mahony 6-DoF attitude estimator** separately for each IMU.
- Input: calibrated accelerometer + calibrated gyroscope.
- Update: 100 Hz.
- Gravity is used to correct roll/pitch.
- Yaw is gyro-integrated and therefore drift-prone.
- No absolute heading is claimed.
- Relative foot/shank orientation is the preferred kinematic representation for rotation features.

V1 gains:
- `Kp = 2.0`
- `Ki = 0.05`

Keep the gain values in configuration so engineering tests can tune them, but ship these values as the V1 default.

## 6. Relative orientation
Represent segment-relative orientation as:
```text
q_relative = inverse(q_shank) * q_foot
```
Use the relative orientation for ankle/foot-segment features. Normalize quaternions after each update.

For V1 analysis, do not use absolute yaw as a clinical feature.

## 7. Filtering layers
Maintain separate paths:
- **raw path:** unmodified calibrated measurements for storage.
- **orientation path:** Mahony input, lightly filtered for estimator stability.
- **gait path:** 2nd-order low-pass IIR/biquad at 6 Hz for gait timing/features.
- **event path:** preserve enough bandwidth for contact/transient detection; use the 20 Hz cleaned sensor path.

Never overwrite the canonical raw data with filtered values.
