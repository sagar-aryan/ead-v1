#pragma once
// The schema-6 sensor feed (DEC-021): segment orientation from the BNO086's own
// game rotation vector, and accelerometer samples brought to the frame time.
// Portable: built for the device, the replay and the host tests.

#include <cstddef>
#include <cstdint>

namespace ead {

/// The quaternion of a proper-rotation matrix `m` (row = output axis), so that
/// rotateByQuaternion(q, v) equals m * v. For a mount map, chip to anatomical.
void matrixToQuaternion(const int8_t m[3][3], float q[4]);

/// Segment orientation (segment to world) from a sensor's rotation vector
/// (chip to world): the chip frame is carried into anatomical axes by `mount`,
/// then the calibration's `alignment` takes the strap's tilt out, as for every
/// other measurement (doc 04 §4). All quaternions are (w, x, y, z).
void segmentOrientation(const float qChip[4], const float mount[4], const float alignment[4],
                        float out[4]);

/// A rotation vector as the frame carries it (real, i, j, k in Q14) to a unit
/// quaternion (w, x, y, z).
void rotationVectorToQuaternion(const int16_t rv[4], float q[4]);

/// The furthest a sample may be from the frame that uses it. A sensor that stops
/// leaves its last sample behind, and nearest-in-time would carry it into every
/// later frame as current; past this it is reported missing instead (doc 06 §12,
/// "stale sample"). Ten frame periods: well beyond the timing jitter, and well
/// inside the 250 ms after which the sensor is flagged as giving no data.
constexpr int64_t kMaxSampleAgeUs = 50000;

/// Doc 06 §12: whether a frame stops feedback. A failed read, a missing or stale
/// rotation vector, no orientation, a gap before the frame (`gap`), or any device
/// fault; saturation and held or repeated samples are measurement conditions,
/// flagged in the frame but not faults.
bool feedbackFault(uint16_t frameStatus, bool gap, uint16_t deviceFaults);

struct AccelPoint {
  int64_t us;
  int16_t v[3];
};

/// The accelerometer at time `t`, linearly interpolated between the samples
/// either side of it (`points` oldest first). True when it was; false when no
/// sample brackets `t`, and `out` holds the nearest one (zeros if none).
bool interpolateAccel(const AccelPoint* points, size_t count, int64_t t, int16_t out[3]);

}  // namespace ead
