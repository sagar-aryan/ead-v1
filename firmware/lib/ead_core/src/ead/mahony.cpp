#include "ead/mahony.h"

#include <cmath>

namespace ead {
namespace {

constexpr float kDegToRad = 0.017453292519943295f;
/// Below this the accelerometer says nothing about which way is down.
constexpr float kMinAccelG = 0.1f;

}  // namespace

void quaternionMultiply(const float a[4], const float b[4], float out[4]) {
  const float w = a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3];
  const float x = a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2];
  const float y = a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1];
  const float z = a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0];
  out[0] = w;
  out[1] = x;
  out[2] = y;
  out[3] = z;
}

void quaternionConjugate(const float q[4], float out[4]) {
  out[0] = q[0];
  out[1] = -q[1];
  out[2] = -q[2];
  out[3] = -q[3];
}

void quaternionNormalize(float q[4]) {
  const float length = std::sqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]);
  if (length < 1e-9f) {
    q[0] = 1.0f;
    q[1] = q[2] = q[3] = 0.0f;
    return;
  }
  for (int i = 0; i < 4; ++i) q[i] /= length;
}

void relativeOrientation(const float qShank[4], const float qFoot[4], float out[4]) {
  float inverse[4];
  quaternionConjugate(qShank, inverse);
  quaternionMultiply(inverse, qFoot, out);
  quaternionNormalize(out);
}

void quaternionToQ15(const float q[4], int16_t out[4]) {
  for (int i = 0; i < 4; ++i) {
    float scaled = q[i] * 32767.0f;
    if (scaled > 32767.0f) scaled = 32767.0f;
    if (scaled < -32767.0f) scaled = -32767.0f;
    out[i] = static_cast<int16_t>(scaled >= 0.0f ? scaled + 0.5f : scaled - 0.5f);
  }
}

void Mahony::reset(const float q0[4]) {
  for (int i = 0; i < 4; ++i) q_[i] = q0[i];
  quaternionNormalize(q_);
  integral_[0] = integral_[1] = integral_[2] = 0.0f;
}

void Mahony::update(const float gyroDps[3], const float accelG[3], float dtSeconds, float kp,
                    float ki) {
  float rate[3] = {gyroDps[0] * kDegToRad, gyroDps[1] * kDegToRad, gyroDps[2] * kDegToRad};

  const float magnitude =
      std::sqrt(accelG[0] * accelG[0] + accelG[1] * accelG[1] + accelG[2] * accelG[2]);
  if (magnitude > kMinAccelG) {
    const float ax = accelG[0] / magnitude;
    const float ay = accelG[1] / magnitude;
    const float az = accelG[2] / magnitude;

    // Gravity as the current orientation predicts it: the body-frame direction
    // of the world +Z axis, which is the third column of the rotation matrix.
    const float w = q_[0], x = q_[1], y = q_[2], z = q_[3];
    const float vx = 2.0f * (x * z - w * y);
    const float vy = 2.0f * (w * x + y * z);
    const float vz = w * w - x * x - y * y + z * z;

    // The error is the cross product of measured and predicted gravity: the
    // rotation that would bring them together.
    const float ex = ay * vz - az * vy;
    const float ey = az * vx - ax * vz;
    const float ez = ax * vy - ay * vx;

    if (ki > 0.0f) {
      integral_[0] += ki * ex * dtSeconds;
      integral_[1] += ki * ey * dtSeconds;
      integral_[2] += ki * ez * dtSeconds;
      rate[0] += integral_[0];
      rate[1] += integral_[1];
      rate[2] += integral_[2];
    }
    rate[0] += kp * ex;
    rate[1] += kp * ey;
    rate[2] += kp * ez;
  }

  // First-order integration of the quaternion derivative, then renormalize.
  const float half = 0.5f * dtSeconds;
  const float w = q_[0], x = q_[1], y = q_[2], z = q_[3];
  q_[0] += half * (-x * rate[0] - y * rate[1] - z * rate[2]);
  q_[1] += half * (w * rate[0] + y * rate[2] - z * rate[1]);
  q_[2] += half * (w * rate[1] - x * rate[2] + z * rate[0]);
  q_[3] += half * (w * rate[2] + x * rate[1] - y * rate[0]);
  quaternionNormalize(q_);
}

}  // namespace ead
