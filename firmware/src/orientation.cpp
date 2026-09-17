#include "orientation.h"

#include <freertos/FreeRTOS.h>

#include "anatomical.h"
#include "calibration_service.h"
#include "ead/mahony.h"

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
    s_foot.reset(record.foot.alignment);
    s_shank.reset(record.shank.alignment);
  }
  portEXIT_CRITICAL(&s_mux);
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
    s_foot.reset(s_calibration.foot.alignment);
    s_shank.reset(s_calibration.shank.alignment);
    frame->status &= ~uint16_t(ead::kRawOrientationValid);
    return;
  }

  float accel[3];
  float gyro[3];
  anatomical::accel(kEadFootMount, frame->foot, accel);
  anatomical::gyro(kEadFootMount, frame->foot, s_calibration.foot.gyroBiasDps, gyro);
  s_foot.update(gyro, accel, dt, EAD_MAHONY_KP, EAD_MAHONY_KI);
  anatomical::accel(kEadShankMount, frame->shank, accel);
  anatomical::gyro(kEadShankMount, frame->shank, s_calibration.shank.gyroBiasDps, gyro);
  s_shank.update(gyro, accel, dt, EAD_MAHONY_KP, EAD_MAHONY_KI);

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
