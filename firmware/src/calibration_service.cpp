#include "calibration_service.h"

#include <freertos/FreeRTOS.h>

#include "config_v1.h"

namespace calibration {
namespace {

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;

ead::CalibrationAccumulator s_foot;
ead::CalibrationAccumulator s_shank;
ead::CalibrationRecord s_record{};
bool s_haveRecord = false;
bool s_completed = false;
ead::CalibrationState s_state = ead::CalibrationState::None;
uint32_t s_samples = 0;
uint32_t s_wanted = 0;
uint16_t s_reject = 0;

/// Chip counts to anatomical g, applying the sensor's mount map.
void anatomicalAccel(const EadMountMap& map, const int16_t raw[6], float out[3]) {
  for (int axis = 0; axis < 3; ++axis) {
    float sum = 0.0f;
    for (int source = 0; source < 3; ++source) {
      if (map.m[axis][source] == 0) continue;
      sum += float(map.m[axis][source]) * float(raw[source]);
    }
    out[axis] = sum / EAD_ACCEL_LSB_PER_G;
  }
}

/// Chip counts to deg/s, left in the chip frame: the bias is a property of the
/// gyroscope, so it is subtracted before the mount map is applied.
void chipGyro(const int16_t raw[6], float out[3]) {
  for (int i = 0; i < 3; ++i) out[i] = float(raw[3 + i]) / EAD_GYRO_LSB_PER_DPS;
}

}  // namespace

bool start(uint16_t durationMs) {
  portENTER_CRITICAL(&s_mux);
  const bool busy = s_state == ead::CalibrationState::Collecting;
  if (!busy) {
    s_foot.reset();
    s_shank.reset();
    s_samples = 0;
    s_reject = 0;
    s_completed = false;
    // Frames arrive at the sensor's own rate, close enough to the nominal one
    // over a few seconds; the record reports the count that was actually used.
    s_wanted = (uint32_t(durationMs) * EAD_SAMPLE_HZ) / 1000u;
    s_state = ead::CalibrationState::Collecting;
  }
  portEXIT_CRITICAL(&s_mux);
  return !busy;
}

void cancel() {
  portENTER_CRITICAL(&s_mux);
  if (s_state == ead::CalibrationState::Collecting) {
    s_state = s_haveRecord ? ead::CalibrationState::Ready : ead::CalibrationState::None;
    s_samples = s_haveRecord ? s_record.samples : 0;
  }
  portEXIT_CRITICAL(&s_mux);
}

void consume(const ead::RawFrame& frame) {
  portENTER_CRITICAL(&s_mux);
  const bool collecting = s_state == ead::CalibrationState::Collecting;
  portEXIT_CRITICAL(&s_mux);
  if (!collecting) return;

  // A frame whose sensor read failed or repeated says nothing about stillness.
  constexpr uint16_t kUnusable = ead::kRawFootReadFail | ead::kRawShankReadFail |
                                 ead::kRawFootRepeated | ead::kRawShankRepeated;
  if ((frame.status & kUnusable) != 0) return;

  float accel[3];
  float gyro[3];
  anatomicalAccel(kEadFootMount, frame.foot, accel);
  chipGyro(frame.foot, gyro);
  s_foot.add(accel, gyro);
  anatomicalAccel(kEadShankMount, frame.shank, accel);
  chipGyro(frame.shank, gyro);
  s_shank.add(accel, gyro);

  const uint32_t collected = s_foot.samples();
  portENTER_CRITICAL(&s_mux);
  s_samples = collected;
  portEXIT_CRITICAL(&s_mux);
  if (collected < s_wanted) return;

  ead::CalibrationRecord finished{};
  finished.samples = collected;
  finished.reject = uint16_t(s_foot.finish(&finished.foot) | s_shank.finish(&finished.shank));

  portENTER_CRITICAL(&s_mux);
  s_record = finished;
  s_haveRecord = true;
  s_completed = true;
  s_reject = finished.reject;
  s_state = finished.reject == 0 ? ead::CalibrationState::Ready : ead::CalibrationState::Rejected;
  portEXIT_CRITICAL(&s_mux);
}

ead::CalibrationState state() {
  portENTER_CRITICAL(&s_mux);
  const ead::CalibrationState value = s_state;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

uint32_t samples() {
  portENTER_CRITICAL(&s_mux);
  const uint32_t value = s_samples;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

uint16_t reject() {
  portENTER_CRITICAL(&s_mux);
  const uint16_t value = s_reject;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

bool record(ead::CalibrationRecord* out) {
  portENTER_CRITICAL(&s_mux);
  const bool have = s_haveRecord;
  if (have) *out = s_record;
  portEXIT_CRITICAL(&s_mux);
  return have;
}

bool takeCompletion() {
  portENTER_CRITICAL(&s_mux);
  const bool completed = s_completed;
  s_completed = false;
  portEXIT_CRITICAL(&s_mux);
  return completed;
}

}  // namespace calibration
