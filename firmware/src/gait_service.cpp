#include "gait_service.h"

#include <freertos/FreeRTOS.h>

#include "anatomical.h"
#include "calibration_service.h"
#include "config_v1.h"
#include "ead/gait.h"
#include "ead/mahony.h"
#include "orientation.h"
#include "session_service.h"
#include "telemetry.h"

namespace gait {
namespace {

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;

ead::GaitEngine s_engine;
ead::CalibrationRecord s_calibration{};
bool s_haveCalibration = false;
uint32_t s_cycles = 0;

/// Drained on the processing task between frames, so a batch is built once.
ead::GaitEvent s_events[ead::kMaxEventsPerBatch];
size_t s_eventCount = 0;
ead::GaitCycle s_cycles_[ead::kMaxCyclesPerBatch];
ead::ErrorResult s_scores[ead::kMaxCyclesPerBatch];
bool s_scored = false;
size_t s_cycleCount = 0;

/// Q15 back to float: the frame carries the estimate the device just made.
void fromQ15(const int16_t q[4], float out[4]) {
  for (int i = 0; i < 4; ++i) out[i] = float(q[i]) / 32767.0f;
}

}  // namespace

void reset() {
  portENTER_CRITICAL(&s_mux);
  s_engine.reset();
  s_eventCount = 0;
  s_cycleCount = 0;
  portEXIT_CRITICAL(&s_mux);
}

void consume(const ead::RawFrame& frame) {
  if ((frame.status & ead::kRawOrientationValid) == 0) return;
  if (!s_haveCalibration) {
    ead::CalibrationRecord record;
    if (!calibration::record(&record) || record.reject != 0) return;
    s_calibration = record;
    s_haveCalibration = true;
    s_engine.reset();
  }

  ead::GaitSample sample{};
  sample.timeUs = frame.timestamp_us;
  sample.frameIndex = frame.frame_index;
  // The same conversion orientation uses: mount map, bias, then the measured
  // alignment, so the engine sees the segment rather than the board.
  float accel[3];
  float gyro[3];
  anatomical::accel(kEadFootMount, frame.foot, accel);
  anatomical::gyro(kEadFootMount, frame.foot, s_calibration.foot.gyroBiasDps, gyro);
  ead::rotateByQuaternion(s_calibration.foot.alignment, accel, sample.footAccelG);
  ead::rotateByQuaternion(s_calibration.foot.alignment, gyro, sample.footGyroDps);
  anatomical::gyro(kEadShankMount, frame.shank, s_calibration.shank.gyroBiasDps, gyro);
  ead::rotateByQuaternion(s_calibration.shank.alignment, gyro, sample.shankGyroDps);

  fromQ15(frame.q_foot, sample.footQuaternion);
  float shankQuaternion[4];
  fromQ15(frame.q_shank, shankQuaternion);
  ead::relativeOrientation(shankQuaternion, sample.footQuaternion, sample.relativeQuaternion);

  s_engine.update(sample);

  ead::GaitEvent event;
  while (s_engine.takeEvent(&event)) {
    if (s_eventCount < ead::kMaxEventsPerBatch) s_events[s_eventCount++] = event;
  }
  ead::GaitCycle cycle;
  while (s_engine.takeCycle(&cycle)) {
    // The session decides what a cycle means: a capture collects it, a check or
    // an evaluation scores it against the locked reference.
    const ead::ErrorResult* score = session::consume(cycle);
    if (s_cycleCount < ead::kMaxCyclesPerBatch) {
      if (score != nullptr) {
        s_scores[s_cycleCount] = *score;
        s_scored = true;
      } else {
        s_scores[s_cycleCount] = ead::ErrorResult{};
      }
      s_cycles_[s_cycleCount++] = cycle;
    }
    portENTER_CRITICAL(&s_mux);
    ++s_cycles;
    portEXIT_CRITICAL(&s_mux);
  }
}

void publish() {
  if (s_eventCount > 0) {
    uint8_t payload[2 + ead::kMaxEventsPerBatch * ead::kEventRecordSize];
    const size_t len = ead::encodeEventBatchPayload(s_events, s_eventCount, payload, sizeof payload);
    telemetry::append(ead::MsgType::EventBatch, s_events[0].timeUs, payload, len);
    s_eventCount = 0;
  }
  if (s_cycleCount > 0) {
    uint8_t payload[2 + ead::kMaxCyclesPerBatch * ead::kCycleRecordSize];
    const size_t len = ead::encodeStepBatchPayload(s_cycles_, s_scored ? s_scores : nullptr,
                                                   s_cycleCount, payload, sizeof payload);
    telemetry::append(ead::MsgType::StepBatch, s_cycles_[0].startUs, payload, len);
    s_cycleCount = 0;
    s_scored = false;
  }
}

uint8_t state() {
  return uint8_t(s_engine.state());
}

uint32_t cyclesCompleted() {
  portENTER_CRITICAL(&s_mux);
  const uint32_t value = s_cycles;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

}  // namespace gait
