#include "ead/feed.h"

#include <cmath>

#include "ead/mahony.h"
#include "ead/protocol.h"

namespace ead {

void matrixToQuaternion(const int8_t m[3][3], float q[4]) {
  const float r00 = m[0][0], r01 = m[0][1], r02 = m[0][2];
  const float r10 = m[1][0], r11 = m[1][1], r12 = m[1][2];
  const float r20 = m[2][0], r21 = m[2][1], r22 = m[2][2];
  const float trace = r00 + r11 + r22;
  if (trace > 0.0f) {
    const float s = std::sqrt(trace + 1.0f) * 2.0f;
    q[0] = 0.25f * s;
    q[1] = (r21 - r12) / s;
    q[2] = (r02 - r20) / s;
    q[3] = (r10 - r01) / s;
  } else if (r00 > r11 && r00 > r22) {
    const float s = std::sqrt(1.0f + r00 - r11 - r22) * 2.0f;
    q[0] = (r21 - r12) / s;
    q[1] = 0.25f * s;
    q[2] = (r01 + r10) / s;
    q[3] = (r02 + r20) / s;
  } else if (r11 > r22) {
    const float s = std::sqrt(1.0f + r11 - r00 - r22) * 2.0f;
    q[0] = (r02 - r20) / s;
    q[1] = (r01 + r10) / s;
    q[2] = 0.25f * s;
    q[3] = (r12 + r21) / s;
  } else {
    const float s = std::sqrt(1.0f + r22 - r00 - r11) * 2.0f;
    q[0] = (r10 - r01) / s;
    q[1] = (r02 + r20) / s;
    q[2] = (r12 + r21) / s;
    q[3] = 0.25f * s;
  }
}

void segmentOrientation(const float qChip[4], const float mount[4], const float alignment[4],
                        float out[4]) {
  // chip -> segment is the mount map then the alignment; the segment's
  // orientation is the chip's followed by the inverse of that.
  float chipToSegment[4];
  quaternionMultiply(alignment, mount, chipToSegment);
  float segmentToChip[4];
  quaternionConjugate(chipToSegment, segmentToChip);
  quaternionMultiply(qChip, segmentToChip, out);
  quaternionNormalize(out);
}

void rotationVectorToQuaternion(const int16_t rv[4], float q[4]) {
  for (int i = 0; i < 4; ++i) q[i] = float(rv[i]) / 16384.0f;
  quaternionNormalize(q);
}

bool interpolateAccel(const AccelPoint* points, size_t count, int64_t t, int16_t out[3]) {
  if (count == 0) {
    out[0] = out[1] = out[2] = 0;
    return false;
  }
  for (size_t i = 1; i < count; ++i) {
    const AccelPoint& a = points[i - 1];
    const AccelPoint& b = points[i];
    if (a.us <= t && t <= b.us && b.us > a.us) {
      const float f = float(t - a.us) / float(b.us - a.us);
      for (int k = 0; k < 3; ++k) {
        out[k] = int16_t(std::lround(float(a.v[k]) + f * float(b.v[k] - a.v[k])));
      }
      return true;
    }
  }
  // Nothing either side: the latest sample not after t, else the earliest.
  const AccelPoint* nearest = &points[0];
  for (size_t i = 0; i < count; ++i) {
    if (points[i].us <= t) nearest = &points[i];
  }
  for (int k = 0; k < 3; ++k) out[k] = nearest->v[k];
  return false;
}

bool feedbackFault(uint16_t frameStatus, bool gap, uint16_t deviceFaults) {
  constexpr uint16_t kFault = kRawFootReadFail | kRawShankReadFail | kRawFootRvMissing |
                              kRawShankRvMissing;
  return gap || deviceFaults != 0 || (frameStatus & kFault) != 0 ||
         (frameStatus & kRawOrientationValid) == 0;
}

}  // namespace ead
