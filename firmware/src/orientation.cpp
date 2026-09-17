#include "orientation.h"

#include <freertos/FreeRTOS.h>

#include "anatomical.h"
#include "calibration_service.h"
#include "ead/calibration.h"
#include "ead/mahony.h"
#include "gait_service.h"

namespace orientation {
namespace {

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;

ead::Mahony s_foot;
ead::Mahony s_shank;
ead::CalibrationRecord s_calibration{};
bool s_calibrated = false;
uint64_t s_lastTimestampUs = 0;

/// Sample period limits. A gap outside these means frames were lost or the
/// clock jumped, and integrating across it would inject a false rotation.
constexpr float kMinDt = 0.5f / float(EAD_SAMPLE_HZ);
constexpr float kMaxDt = 2.0f / float(EAD_SAMPLE_HZ);

}  // namespace

void adopt() {
  ead::CalibrationRecord record;
  const bool have = calibration::record(&record) && record.reject == 0;
  portENTER_CRITICAL(&s_mux);
  s_calibrated = have;
  s_lastTimestampUs = 0;
  if (have) {
    s_calibration = record;
    // The alignment is applied to the measurements, not used as a starting
    // attitude, so the estimators describe the segments rather than the boards
    // and both start upright (doc 04 §4).
    constexpr float kIdentity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
    s_foot.reset(kIdentity);
    s_shank.reset(kIdentity);
  }
  portEXIT_CRITICAL(&s_mux);
  // A new calibration invalidates whatever the gait engine was mid-way through.
  gait::reset();
}

void process(ead::RawFrame* frame) {
  portENTER_CRITICAL(&s_mux);
  const bool calibrated = s_calibrated;
  portEXIT_CRITICAL(&s_mux);
  if (!calibrated) {
    frame->status &= ~uint16_t(ead::kRawOrientationValid);
    return;
  }

  const uint64_t previous = s_lastTimestampUs;
  s_lastTimestampUs = frame->timestamp_us;
  if (previous == 0) return;  // no interval yet; the next frame has one
  const float dt = float(frame->timestamp_us - previous) / 1e6f;
  if (dt < kMinDt || dt > kMaxDt) {
    // Restart rather than integrate across the gap: a wrong orientation that
    // looks valid is worse than a few frames of convergence.
    constexpr float kIdentity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
    s_foot.reset(kIdentity);
    s_shank.reset(kIdentity);
    frame->status &= ~uint16_t(ead::kRawOrientationValid);
    return;
  }

  // Mount map, then bias, then the measured alignment: what reaches the
  // estimator is the segment's motion, with the strap's tilt taken out. Without
  // the last step the mounting angle appears as a permanent joint angle (the
  // foot board sits about 40 degrees off upright on the instep, TEST-029).
  float accel[3];
  float gyro[3];
  float aligned[3];
  anatomical::accel(kEadFootMount, frame->foot, accel);
  anatomical::gyro(kEadFootMount, frame->foot, s_calibration.foot.gyroBiasDps, gyro);
  ead::rotateByQuaternion(s_calibration.foot.alignment, accel, aligned);
  float alignedGyro[3];
  ead::rotateByQuaternion(s_calibration.foot.alignment, gyro, alignedGyro);
  s_foot.update(alignedGyro, aligned, dt, EAD_MAHONY_KP, EAD_MAHONY_KI);

  anatomical::accel(kEadShankMount, frame->shank, accel);
  anatomical::gyro(kEadShankMount, frame->shank, s_calibration.shank.gyroBiasDps, gyro);
  ead::rotateByQuaternion(s_calibration.shank.alignment, accel, aligned);
  ead::rotateByQuaternion(s_calibration.shank.alignment, gyro, alignedGyro);
  s_shank.update(alignedGyro, aligned, dt, EAD_MAHONY_KP, EAD_MAHONY_KI);

  ead::quaternionToQ15(s_foot.quaternion(), frame->q_foot);
  ead::quaternionToQ15(s_shank.quaternion(), frame->q_shank);
  frame->status |= uint16_t(ead::kRawOrientationValid);
}

bool valid() {
  portENTER_CRITICAL(&s_mux);
  const bool value = s_calibrated;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

}  // namespace orientation
