#pragma once
// Mahony 6-DoF attitude estimator, one per IMU (doc 04 §5).
//
// Gravity corrects roll and pitch; yaw is gyro-integrated only, because the
// MPU6500 has no magnetometer. No absolute heading is claimed anywhere, and the
// kinematic quantity that matters is the relative foot/shank orientation
// (doc 04 §6), which is unaffected by a heading both sensors share.
//
// float32 with the same operation order on device and host, so a replay of the
// same samples reproduces the same quaternions.

#include <cstdint>

namespace ead {

/// Hamilton product: the rotation `b` followed by `a`, as (w, x, y, z).
void quaternionMultiply(const float a[4], const float b[4], float out[4]);
/// Inverse of a unit quaternion.
void quaternionConjugate(const float q[4], float out[4]);
void quaternionNormalize(float q[4]);

/// Relative orientation of the foot with respect to the shank (doc 04 §6):
/// `q_relative = inverse(q_shank) * q_foot`.
void relativeOrientation(const float qShank[4], const float qFoot[4], float out[4]);

/// Q15 encoding used by RAW_SAMPLE_BATCH: ±1.0 maps to ±32767.
void quaternionToQ15(const float q[4], int16_t out[4]);

class Mahony {
 public:
  /// Starts from the given orientation, clearing the integral term. Passing the
  /// calibration's alignment quaternion starts the estimator already upright,
  /// so the first seconds of a recording are not spent converging.
  void reset(const float q0[4]);

  /// One update. `gyroDps` and `accelG` are anatomical-frame, bias-removed.
  /// A sample with no usable acceleration (free fall, or a failed read) still
  /// integrates the gyroscope but applies no correction.
  void update(const float gyroDps[3], const float accelG[3], float dtSeconds, float kp, float ki);

  const float* quaternion() const { return q_; }

 private:
  float q_[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  float integral_[3] = {0.0f, 0.0f, 0.0f};
};

}  // namespace ead
