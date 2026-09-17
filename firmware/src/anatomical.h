#pragma once
// Chip counts to anatomical physical units.
//
// Shared by the calibration and orientation services so there is one place
// where a mount map is applied on the device. The raw path is untouched: these
// are derived values, and the frame keeps its chip-frame counts (doc 04 §7).

#include <cstdint>

#include "config_v1.h"

namespace anatomical {

/// Applies a mount map to three chip-frame values and scales them.
inline void map(const EadMountMap& mount, const int16_t chip[3], float scale, float out[3]) {
  for (int axis = 0; axis < 3; ++axis) {
    float sum = 0.0f;
    for (int source = 0; source < 3; ++source) {
      if (mount.m[axis][source] == 0) continue;
      sum += float(mount.m[axis][source]) * float(chip[source]);
    }
    out[axis] = sum / scale;
  }
}

/// Acceleration in g, anatomical frame. `raw` is one sensor's six counts.
inline void accel(const EadMountMap& mount, const int16_t raw[6], float out[3]) {
  map(mount, raw, EAD_ACCEL_LSB_PER_G, out);
}

/// Angular rate in deg/s, chip frame: a gyroscope's bias belongs to the part,
/// so it is subtracted here, before the mount map is applied.
inline void chipGyro(const int16_t raw[6], const float biasDps[3], float out[3]) {
  for (int i = 0; i < 3; ++i) out[i] = float(raw[3 + i]) / EAD_GYRO_LSB_PER_DPS - biasDps[i];
}

/// Angular rate in deg/s, anatomical frame, with the bias already removed.
inline void gyro(const EadMountMap& mount, const int16_t raw[6], const float biasDps[3],
                 float out[3]) {
  float chip[3];
  chipGyro(raw, biasDps, chip);
  for (int axis = 0; axis < 3; ++axis) {
    float sum = 0.0f;
    for (int source = 0; source < 3; ++source) {
      if (mount.m[axis][source] == 0) continue;
      sum += float(mount.m[axis][source]) * chip[source];
    }
    out[axis] = sum;
  }
}

}  // namespace anatomical
