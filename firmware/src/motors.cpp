#include "motors.h"

#include <Arduino.h>
#include <esp_timer.h>

#include "config_v1.h"

namespace motors {

namespace {

constexpr uint8_t kPins[EAD_MOTOR_COUNT] = {EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO,
                                            EAD_MOTOR_M3_GPIO, EAD_MOTOR_M4_GPIO,
                                            EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO};

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;
ead::MotorGuard s_guard({EAD_HAPTIC_MIN_DUTY, EAD_HAPTIC_MAX_DUTY, EAD_HAPTIC_MAX_ON_S * 1000u,
                         EAD_HAPTIC_ROLL_WIN_S * 1000u, EAD_HAPTIC_ROLL_DUTY_LIM,
                         EAD_MOTOR_ENABLED_MASK});
esp_timer_handle_t s_stop = nullptr;
uint8_t s_active = 0;  // bit n-1: motor n's LEDC channel (n-1) is driven
bool s_ready = false;

bool enabled(int channel) { return (EAD_MOTOR_ENABLED_MASK & (1u << channel)) != 0; }

void stopPulse(void*) {
  portENTER_CRITICAL(&s_mux);
  const uint8_t active = s_active;
  s_active = 0;
  portEXIT_CRITICAL(&s_mux);
  for (int channel = 0; channel < EAD_MOTOR_COUNT; channel++) {
    if (active & (1u << channel)) ledcWrite(channel, 0);
  }
}

/// Drives the accepted motors for `durationMs`, ending whatever ran before.
void run(const uint8_t motor[2], const uint8_t duty[2], uint32_t durationMs) {
  // The guard has ruled the previous output over; make sure it is, before the
  // timer that would have ended it is reused.
  esp_timer_stop(s_stop);
  stopPulse(nullptr);
  uint8_t active = 0;
  for (int i = 0; i < 2; i++) {
    if (motor[i] == 0) continue;
    ledcWrite(motor[i] - 1, duty[i]);
    active |= uint8_t(1u << (motor[i] - 1));
  }
  portENTER_CRITICAL(&s_mux);
  s_active = active;
  portEXIT_CRITICAL(&s_mux);
  esp_timer_start_once(s_stop, uint64_t(durationMs) * 1000u);
}

}  // namespace

bool begin() {
  for (int channel = 0; channel < EAD_MOTOR_COUNT; channel++) {
    if (!enabled(channel)) continue;  // its pin stays as main() left it
    if (ledcSetup(channel, EAD_HAPTIC_PWM_HZ, EAD_HAPTIC_RES_BITS) == 0) return false;
    ledcAttachPin(kPins[channel], channel);
    ledcWrite(channel, 0);
  }
  const esp_timer_create_args_t args = {stopPulse, nullptr, ESP_TIMER_TASK, "motor_stop", false};
  if (esp_timer_create(&args, &s_stop) != ESP_OK) return false;
  s_ready = true;
  return true;
}

ead::MotorGuard::Refusal pulse(const ead::MotorPulse& request) {
  using Refusal = ead::MotorGuard::Refusal;
  if (!s_ready) return Refusal::Disabled;
  portENTER_CRITICAL(&s_mux);
  const Refusal refusal =
      s_guard.request(request.motor, request.duty, request.durationMs, uint32_t(millis()));
  portEXIT_CRITICAL(&s_mux);
  if (refusal != Refusal::None) return refusal;
  const uint8_t motor[2] = {request.motor, 0};
  const uint8_t duty[2] = {request.duty, 0};
  run(motor, duty, request.durationMs);
  return Refusal::None;
}

ead::MotorGuard::Refusal cue(const ead::HapticCue& cue) {
  using Refusal = ead::MotorGuard::Refusal;
  if (!s_ready) return Refusal::Disabled;
  portENTER_CRITICAL(&s_mux);
  const Refusal refusal =
      s_guard.requestCue(cue.motor, cue.duty, cue.durationMs, uint32_t(millis()));
  portEXIT_CRITICAL(&s_mux);
  if (refusal != Refusal::None) return refusal;
  run(cue.motor, cue.duty, cue.durationMs);
  return Refusal::None;
}

void stopAll() {
  if (!s_ready) return;
  esp_timer_stop(s_stop);
  stopPulse(nullptr);
}

}  // namespace motors
