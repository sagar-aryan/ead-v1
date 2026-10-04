#include "ead/motor_guard.h"

namespace ead {

uint32_t MotorGuard::onTimeWithin(uint8_t motor, uint32_t fromMs, uint32_t toMs) const {
  uint32_t total = 0;
  for (const Pulse& p : history_) {
    if (p.motor != motor || p.endMs <= fromMs || p.startMs >= toMs) continue;
    const uint32_t start = p.startMs > fromMs ? p.startMs : fromMs;
    const uint32_t end = p.endMs < toMs ? p.endMs : toMs;
    total += end - start;
  }
  return total;
}

uint8_t MotorGuard::running(uint32_t nowMs) const {
  for (const Pulse& p : history_) {
    if (p.motor != 0 && p.startMs <= nowMs && nowMs < p.endMs) return p.motor;
  }
  return 0;
}

MotorGuard::Refusal MotorGuard::check(uint8_t motor, uint8_t duty, uint32_t durationMs,
                                      uint32_t nowMs) const {
  if (motor < 1 || motor > 6) return Refusal::BadMotor;
  if ((limits_.enabledMask & (1u << (motor - 1))) == 0) return Refusal::Disabled;
  if (duty < limits_.minDuty || duty > limits_.maxDuty) return Refusal::BadDuty;
  if (durationMs < kMinPulseMs || durationMs > limits_.maxOnMs) return Refusal::BadDuration;

  // The window ending when this pulse ends holds the most on-time: windows
  // ending later hold less of the past, and earlier ones gain past on-time only
  // as fast as they lose this pulse.
  const uint32_t endMs = nowMs + durationMs;
  const uint32_t fromMs = endMs > limits_.windowMs ? endMs - limits_.windowMs : 0;
  const uint32_t allowedMs = uint32_t(float(limits_.windowMs) * limits_.windowOnFraction);
  if (onTimeWithin(motor, fromMs, endMs) + durationMs > allowedMs) return Refusal::RollingLimit;
  return Refusal::None;
}

void MotorGuard::record(uint8_t motor, uint32_t startMs, uint32_t endMs) {
  history_[next_] = Pulse{motor, startMs, endMs};
  next_ = (next_ + 1) % kHistory;
}

MotorGuard::Refusal MotorGuard::request(uint8_t motor, uint8_t duty, uint32_t durationMs,
                                        uint32_t nowMs) {
  const Refusal refusal = check(motor, duty, durationMs, nowMs);
  if (refusal != Refusal::None) return refusal;
  if (running(nowMs) != 0) return Refusal::Busy;
  record(motor, nowMs, nowMs + durationMs);
  return Refusal::None;
}

MotorGuard::Refusal MotorGuard::requestCue(const uint8_t motor[2], const uint8_t duty[2],
                                           uint32_t durationMs, uint32_t nowMs) {
  for (int i = 0; i < 2; ++i) {
    if (i == 1 && motor[1] == 0) break;
    const Refusal refusal = check(motor[i], duty[i], durationMs, nowMs);
    if (refusal != Refusal::None) return refusal;
  }
  if (motor[1] == motor[0]) return Refusal::BadMotor;
  if (running(nowMs) != 0) return Refusal::Busy;
  record(motor[0], nowMs, nowMs + durationMs);
  if (motor[1] != 0) record(motor[1], nowMs, nowMs + durationMs);
  return Refusal::None;
}

}  // namespace ead
