#include "ead/calibration.h"

#include <cmath>

namespace ead {
namespace {

constexpr float kRadToDeg = 57.295779513f;

float dot(const float a[3], const float b[3]) {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

float norm(const float v[3]) {
  return std::sqrt(dot(v, v));
}

/// Shortest rotation taking `from` (unit) to anatomical +Z, as (w, x, y, z).
///
/// The half-angle form (1 + cos, axis) is used rather than an angle and an
/// axis: it needs no acos, and it degrades gracefully as the rotation gets
/// small, which is the normal case for a sensor that is roughly upright.
void alignToUp(const float from[3], float out[4]) {
  const float cosine = from[2];  // dot with (0, 0, 1)
  // Cross product with +Z: (from.y * 1 - 0, 0 - from.x * 1, 0).
  float axis[3] = {from[1], -from[0], 0.0f};
  const float axisLength = norm(axis);
  if (axisLength < 1e-6f) {
    // Already up, or exactly inverted. Inverted is rejected elsewhere, but a
    // half turn about X is still the right answer for it.
    out[0] = cosine >= 0.0f ? 1.0f : 0.0f;
    out[1] = cosine >= 0.0f ? 0.0f : 1.0f;
    out[2] = 0.0f;
    out[3] = 0.0f;
    return;
  }
  const float w = 1.0f + cosine;
  const float length = std::sqrt(w * w + axisLength * axisLength);
  out[0] = w / length;
  out[1] = axis[0] / length;
  out[2] = axis[1] / length;
  out[3] = axis[2] / length;
}

}  // namespace

void rotateByQuaternion(const float q[4], const float v[3], float out[3]) {
  // out = v + 2 * q_vec x (q_vec x v + w * v)
  const float x = q[1], y = q[2], z = q[3], w = q[0];
  const float tx = 2.0f * (y * v[2] - z * v[1]);
  const float ty = 2.0f * (z * v[0] - x * v[2]);
  const float tz = 2.0f * (x * v[1] - y * v[0]);
  out[0] = v[0] + w * tx + (y * tz - z * ty);
  out[1] = v[1] + w * ty + (z * tx - x * tz);
  out[2] = v[2] + w * tz + (x * ty - y * tx);
}

void CalibrationAccumulator::reset() {
  samples_ = 0;
  for (int i = 0; i < 3; ++i) {
    accelSum_[i] = 0.0f;
    gyroSum_[i] = 0.0f;
    gyroSquareSum_[i] = 0.0f;
  }
}

void CalibrationAccumulator::add(const float accelG[3], const float gyroDps[3]) {
  for (int i = 0; i < 3; ++i) {
    accelSum_[i] += accelG[i];
    gyroSum_[i] += gyroDps[i];
    gyroSquareSum_[i] += gyroDps[i] * gyroDps[i];
  }
  ++samples_;
}

uint16_t CalibrationAccumulator::finish(CalibrationSensor* out) const {
  *out = CalibrationSensor{};
  out->alignment[0] = 1.0f;
  if (samples_ < kCalibMinSamples) {
    return kCalibTooFewSamples;
  }

  const float count = static_cast<float>(samples_);
  float mean[3];
  for (int i = 0; i < 3; ++i) {
    mean[i] = accelSum_[i] / count;
    out->gyroBiasDps[i] = gyroSum_[i] / count;
    // Variance about the measured mean; negative only through rounding.
    const float variance = gyroSquareSum_[i] / count - out->gyroBiasDps[i] * out->gyroBiasDps[i];
    const float deviation = variance > 0.0f ? std::sqrt(variance) : 0.0f;
    if (deviation > out->gyroStdDps) out->gyroStdDps = deviation;
  }

  const float magnitude = norm(mean);
  out->accelMagnitudeG = magnitude;
  uint16_t reject = 0;
  if (out->gyroStdDps > kCalibMaxGyroStdDps) reject |= kCalibMoved;
  if (magnitude < 1e-3f || std::fabs(magnitude - 1.0f) > kCalibMaxAccelErrorG) {
    reject |= kCalibNotGravity;
    if (magnitude < 1e-3f) return reject;  // no direction to report
  }

  for (int i = 0; i < 3; ++i) out->up[i] = mean[i] / magnitude;
  if (out->up[2] < kCalibMinUpZ) reject |= kCalibUpsideDown;

  alignToUp(out->up, out->alignment);
  out->tiltDeg = std::acos(out->up[2] > 1.0f ? 1.0f : out->up[2]) * kRadToDeg;
  return reject;
}

}  // namespace ead
