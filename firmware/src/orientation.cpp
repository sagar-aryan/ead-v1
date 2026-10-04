#include "orientation.h"

#include <freertos/FreeRTOS.h>

#include "calibration_service.h"
#include "config_v1.h"
#include "ead/calibration.h"
#include "ead/feed.h"
#include "ead/mahony.h"
#include "gait_service.h"

namespace orientation {
namespace {

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;

ead::CalibrationRecord s_calibration{};
bool s_calibrated = false;
float s_footMount[4];
float s_shankMount[4];

}  // namespace

void adopt() {
  ead::CalibrationRecord record;
  const bool have = calibration::record(&record) && record.reject == 0;
  ead::matrixToQuaternion(kEadFootMount.m, s_footMount);
  ead::matrixToQuaternion(kEadShankMount.m, s_shankMount);
  portENTER_CRITICAL(&s_mux);
  s_calibrated = have;
  if (have) s_calibration = record;
  portEXIT_CRITICAL(&s_mux);
  // A new calibration invalidates whatever the gait engine was mid-way through.
  gait::reset();
}

void process(ead::RawFrame* frame) {
  portENTER_CRITICAL(&s_mux);
  const bool calibrated = s_calibrated;
  portEXIT_CRITICAL(&s_mux);
  constexpr uint16_t kMissing = ead::kRawFootRvMissing | ead::kRawShankRvMissing;
  if (!calibrated || (frame->status & kMissing) != 0) {
    frame->status &= ~uint16_t(ead::kRawOrientationValid);
    return;
  }
  // The BNO086's own fusion (DEC-021), carried into the segment the same way
  // as every measurement: mount map, then the calibration's alignment, which
  // takes the strap's tilt out (the foot board sits about 40 degrees off
  // upright on the instep, TEST-029).
  float chip[4];
  float segment[4];
  ead::rotationVectorToQuaternion(frame->rv_foot, chip);
  ead::segmentOrientation(chip, s_footMount, s_calibration.foot.alignment, segment);
  ead::quaternionToQ15(segment, frame->q_foot);
  ead::rotationVectorToQuaternion(frame->rv_shank, chip);
  ead::segmentOrientation(chip, s_shankMount, s_calibration.shank.alignment, segment);
  ead::quaternionToQ15(segment, frame->q_shank);
  frame->status |= uint16_t(ead::kRawOrientationValid);
}

bool valid() {
  portENTER_CRITICAL(&s_mux);
  const bool value = s_calibrated;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

}  // namespace orientation
