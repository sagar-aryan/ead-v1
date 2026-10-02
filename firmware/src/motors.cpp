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
int s_active = -1;  // LEDC channel of the running pulse; channel n-1 drives motor n
bool s_ready = false;

bool enabled(int channel) { return (EAD_MOTOR_ENABLED_MASK & (1u << channel)) != 0; }

void stopPulse(void*) {
  portENTER_CRITICAL(&s_mux);
  const int channel = s_active;
  s_active = -1;
  portEXIT_CRITICAL(&s_mux);
  if (channel >= 0) ledcWrite(channel, 0);
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

  // The guard has ruled the previous pulse over; make sure its output is too,
  // before the timer that would have ended it is reused.
  esp_timer_stop(s_stop);
  stopPulse(nullptr);
  const int channel = request.motor - 1;
  ledcWrite(channel, request.duty);
  portENTER_CRITICAL(&s_mux);
  s_active = channel;
  portEXIT_CRITICAL(&s_mux);
  esp_timer_start_once(s_stop, uint64_t(request.durationMs) * 1000u);
  return Refusal::None;
}

}  // namespace motors
