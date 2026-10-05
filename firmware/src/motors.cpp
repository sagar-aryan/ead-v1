#include "motors.h"

#include <Arduino.h>
#include <esp_timer.h>
#include <freertos/FreeRTOS.h>
#include <freertos/semphr.h>

#include "config_v1.h"

namespace motors {

namespace {

constexpr uint8_t kPins[EAD_MOTOR_COUNT] = {EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO,
                                            EAD_MOTOR_M3_GPIO, EAD_MOTOR_M4_GPIO,
                                            EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO};

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;
ead::MotorGuard s_guard({EAD_MOTOR_MIN_DUTY, EAD_MOTOR_MAX_DUTY, EAD_HAPTIC_MAX_ON_S * 1000u,
                         EAD_HAPTIC_ROLL_WIN_S * 1000u, EAD_HAPTIC_ROLL_DUTY_LIM,
                         EAD_MOTOR_ENABLED_MASK});
esp_timer_handle_t s_stop = nullptr;
// One owner for the outputs at a time: a cue starting on the processing task, an
// OFF from the link task and the stop timer's own task. Without it, an OFF that
// landed while a cue was being written read the old active set and missed the
// new outputs, which then ran to the end of the cue (audit I10).
SemaphoreHandle_t s_outputs = nullptr;
uint8_t s_active = 0;  // bit n-1: motor n's LEDC channel (n-1) is driven
int64_t s_endUs = 0;   // when the running output is due to end
bool s_ready = false;

bool enabled(int channel) { return (EAD_MOTOR_ENABLED_MASK & (1u << channel)) != 0; }

/// Every driven output to 0. The caller holds s_outputs.
void off() {
  for (int channel = 0; channel < EAD_MOTOR_COUNT; channel++) {
    if (s_active & (1u << channel)) ledcWrite(channel, 0);
  }
  s_active = 0;
}

void stopPulse(void*) {
  xSemaphoreTake(s_outputs, portMAX_DELAY);
  // A callback that waited on the lock while a new output started belongs to
  // the old one: leave the new one (at least 100 ms long) to its own timer. The
  // 1 ms margin keeps a timer that fires a hair early from leaving one running.
  if (esp_timer_get_time() >= s_endUs - 1000) off();
  xSemaphoreGive(s_outputs);
}

/// Drives the accepted motors for `durationMs`, ending whatever ran before.
/// False, with every motor off, if the stop timer could not be started: an
/// output nothing would end must not run.
bool run(const uint8_t motor[2], const uint8_t duty[2], uint32_t durationMs) {
  xSemaphoreTake(s_outputs, portMAX_DELAY);
  esp_timer_stop(s_stop);
  off();
  for (int i = 0; i < 2; i++) {
    if (motor[i] == 0) continue;
    ledcWrite(motor[i] - 1, duty[i]);
    s_active |= uint8_t(1u << (motor[i] - 1));
  }
  s_endUs = esp_timer_get_time() + int64_t(durationMs) * 1000;
  const bool timed = esp_timer_start_once(s_stop, uint64_t(durationMs) * 1000u) == ESP_OK;
  if (!timed) off();
  xSemaphoreGive(s_outputs);
  return timed;
}

}  // namespace

bool begin() {
  for (int channel = 0; channel < EAD_MOTOR_COUNT; channel++) {
    if (!enabled(channel)) continue;  // its pin stays as main() left it
    if (ledcSetup(channel, EAD_HAPTIC_PWM_HZ, EAD_HAPTIC_RES_BITS) == 0) return false;
    ledcAttachPin(kPins[channel], channel);
    ledcWrite(channel, 0);
  }
  s_outputs = xSemaphoreCreateMutex();
  if (s_outputs == nullptr) return false;
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
  return run(motor, duty, request.durationMs) ? Refusal::None : Refusal::Disabled;
}

ead::MotorGuard::Refusal cue(const ead::HapticCue& cue) {
  using Refusal = ead::MotorGuard::Refusal;
  if (!s_ready) return Refusal::Disabled;
  portENTER_CRITICAL(&s_mux);
  const Refusal refusal =
      s_guard.requestCue(cue.motor, cue.duty, cue.durationMs, uint32_t(millis()));
  portEXIT_CRITICAL(&s_mux);
  if (refusal != Refusal::None) return refusal;
  return run(cue.motor, cue.duty, cue.durationMs) ? Refusal::None : Refusal::Disabled;
}

void stopAll() {
  if (!s_ready) return;
  xSemaphoreTake(s_outputs, portMAX_DELAY);
  esp_timer_stop(s_stop);
  off();
  xSemaphoreGive(s_outputs);
}

}  // namespace motors
