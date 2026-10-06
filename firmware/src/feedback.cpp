#include "feedback.h"

#include <esp_timer.h>

#include <atomic>

#include "config_v1.h"
#include "device.h"
#include "ead/haptics.h"
#include "ead/protocol.h"
#include "motors.h"
#include "session_service.h"
#include "telemetry.h"

namespace feedback {
namespace {

ead::HapticConfig contractConfig() {
  ead::HapticConfig c;
  const float degrees[6] = {EAD_MOTOR_M1_DEG, EAD_MOTOR_M2_DEG, EAD_MOTOR_M3_DEG,
                            EAD_MOTOR_M4_DEG, EAD_MOTOR_M5_DEG, EAD_MOTOR_M6_DEG};
  for (int i = 0; i < 6; ++i) c.motorDeg[i] = degrees[i];
  c.onScore = EAD_HAPTIC_ON_TH;
  c.offScore = EAD_HAPTIC_OFF_TH;
  c.startConfidence = EAD_CONF_HAPTIC_TH;
  c.keepConfidence = ead::kConfidenceForDisplay;
  c.minDuty = EAD_HAPTIC_MIN_DUTY;
  c.maxDuty = EAD_HAPTIC_MAX_DUTY;
  c.intensityExponent = EAD_HAPTIC_INT_EXP;
  c.cueMs = EAD_HAPTIC_CUE_MS;
  return c;
}

std::atomic<bool> s_enabled{false};
std::atomic<bool> s_episode{false};
ead::HapticEngine s_engine(contractConfig());
ead::HapticCue s_log[ead::kMaxHapticsPerBatch];
size_t s_logCount = 0;

void log(const ead::HapticCue& cue) {
  if (s_logCount < ead::kMaxHapticsPerBatch) s_log[s_logCount++] = cue;
}

void end(ead::HapticReason reason, uint64_t timeUs) {
  motors::stopAll();
  ead::HapticCue cue;
  if (s_engine.stop(reason, timeUs, &cue)) log(cue);
  s_episode = false;
}

bool evaluating() {
  return session::active() && session::kind() == ead::SessionKind::Evaluation;
}

}  // namespace

void setEnabled(bool on) {
  s_enabled = on;
  if (!on) motors::stopAll();
}

bool enabled() { return s_enabled; }

bool episodeActive() { return s_episode; }

void onCycle(const ead::GaitCycle& cycle, const ead::ErrorResult* score) {
  if (!s_enabled || score == nullptr || !evaluating()) return;
  ead::HapticCue cue;
  if (!s_engine.onCycle(cycle, *score, &cue)) return;
  if (cue.event == ead::HapticEvent::Off) {
    motors::stopAll();
  } else if (device::hostQuietFor(esp_timer_get_time(), EAD_HAPTIC_LINK_TIMEOUT_MS * 1000LL)) {
    // DEC-027: no one is watching. Logged as it would have run, with no duty;
    // the episode carries on, and cues resume when the laptop is heard again.
    cue.reason = ead::HapticReason::LinkLost;
    cue.duty[0] = cue.duty[1] = 0;
  } else if (motors::cue(cue) != ead::MotorGuard::Refusal::None) {
    // Logged as it would have run, with no duty: it did not.
    cue.reason = ead::HapticReason::Refused;
    cue.duty[0] = cue.duty[1] = 0;
  }
  log(cue);
  s_episode = s_engine.active();
}

void fault(uint64_t timeUs) {
  if (s_engine.active()) end(ead::HapticReason::SensorFault, timeUs);
}

void poll(uint64_t timeUs) {
  if (!s_engine.active()) return;
  if (!s_enabled) {
    end(ead::HapticReason::SwitchedOff, timeUs);
  } else if (!evaluating()) {
    end(ead::HapticReason::SessionEnded, timeUs);
  }
}

void publish() {
  if (s_logCount == 0) return;
  uint8_t payload[2 + ead::kMaxHapticsPerBatch * ead::kHapticRecordSize];
  const size_t len = ead::encodeHapticBatchPayload(s_log, s_logCount, payload, sizeof payload);
  telemetry::append(ead::MsgType::HapticBatch, s_log[0].timeUs, payload, len);
  s_logCount = 0;
}

}  // namespace feedback
