#pragma once
// Static calibration: what five seconds of stillness tells us about a sensor.
//
// Two things are measured per sensor. The gyro bias is the mean angular rate
// while the sensor is still, which would otherwise integrate into drift: 1 deg/s
// unremoved becomes 60 degrees of false rotation in a minute. The gravity
// direction is the mean acceleration while still; the sensor is not mounted
// upright (a strap over the instep holds the foot board about 33 degrees off,
// TEST-027) and this is the rotation that takes the mounted frame to the
// anatomical one.
//
// Accumulation is in float32 with the same operations on device and host, so a
// replay of the same samples produces the same record bit for bit.

#include <cstdint>

namespace ead {

/// Rejection reasons; a record with any bit set must not be used.
enum CalibrationReject : uint16_t {
  kCalibTooFewSamples = 1 << 0,
  kCalibMoved = 1 << 1,          ///< angular rate varied too much to be still
  kCalibNotGravity = 1 << 2,     ///< |a| was not 1 g: moving, or mis-scaled
  kCalibUpsideDown = 1 << 3,     ///< gravity was not on +Z at all
};

/// At least this many samples (2 s at 100 Hz) before a record is usable.
constexpr uint32_t kCalibMinSamples = 200;
/// Standard deviation of any gyro axis, above which the sensor was moving.
constexpr float kCalibMaxGyroStdDps = 2.0f;
/// How far the mean acceleration may sit from 1 g.
constexpr float kCalibMaxAccelErrorG = 0.05f;
/// Gravity must be at least this much of the way up; the rest is mounting tilt.
constexpr float kCalibMinUpZ = 0.5f;

struct CalibrationSensor {
  /// Mean angular rate while still, chip frame, deg/s. Subtract from readings.
  float gyroBiasDps[3];
  /// Measured gravity direction, anatomical frame, unit length.
  float up[3];
  /// Rotation taking the measured up direction to anatomical +Z (w, x, y, z).
  float alignment[4];
  /// Angle between the measured up direction and +Z, degrees.
  float tiltDeg;
  /// Mean |a| in g: 1.00 confirms the scale factor as well as the stillness.
  float accelMagnitudeG;
  /// Largest gyro standard deviation across the three axes, deg/s.
  float gyroStdDps;
};

struct CalibrationRecord {
  CalibrationSensor foot;
  CalibrationSensor shank;
  uint32_t samples;
  uint16_t reject;  ///< CalibrationReject bits; 0 means usable
};

/// Accumulates one sensor's still window. Sums are kept, not the samples.
class CalibrationAccumulator {
 public:
  void reset();
  /// One sample: acceleration in anatomical g, angular rate in chip-frame deg/s.
  void add(const float accelG[3], const float gyroDps[3]);
  uint32_t samples() const { return samples_; }
  /// Fills `out` and returns the rejection bits for this sensor.
  uint16_t finish(CalibrationSensor* out) const;

 private:
  uint32_t samples_ = 0;
  float accelSum_[3] = {0, 0, 0};
  float gyroSum_[3] = {0, 0, 0};
  float gyroSquareSum_[3] = {0, 0, 0};
};

/// Rotates a vector by a quaternion (w, x, y, z). Shared with the tests.
void rotateByQuaternion(const float q[4], const float v[3], float out[3]);

}  // namespace ead
