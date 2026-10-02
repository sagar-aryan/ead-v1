// EAD-V1 firmware: 100 Hz acquisition of the foot and shank BNO086 sensors,
// streamed as binary protocol messages over Wi-Fi and USB.
// Architecture: docs/architecture.md. Protocol: docs/protocol.md.
#include <Arduino.h>
#include <esp_log.h>

#include "acquisition.h"
#include "config_v1.h"
#include "device.h"
#include "ead/protocol.h"
#include "link_usb.h"
#include "link_wifi.h"
#include "motors.h"
#include "telemetry.h"

static const uint8_t kMotorGpios[EAD_MOTOR_COUNT] = {
    EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO,
    EAD_MOTOR_M4_GPIO, EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO};

void setup() {
  // Motor outputs first: doc 03 §4 and wiring rule 1 require them LOW before
  // anything else. A channel switched off in EAD_MOTOR_ENABLED_MASK stays as
  // reset leaves it, held off by its gate pulldown: driving a pin is safe only if
  // it really reaches a gate (PROB-020).
  for (int m = 0; m < EAD_MOTOR_COUNT; m++) {
    if ((EAD_MOTOR_ENABLED_MASK & (1u << m)) == 0) continue;
    pinMode(kMotorGpios[m], OUTPUT);
    digitalWrite(kMotorGpios[m], LOW);
  }

  // USB carries only binary frames (DEC-005) through link_usb.cpp, which drives
  // the USB Serial/JTAG FIFO itself; Arduino Serial is never started. Silence
  // ESP-IDF logging, which would otherwise write into the same FIFO.
  esp_log_level_set("*", ESP_LOG_NONE);

  const bool psramRing = telemetry::begin();
  if (!psramRing) device::raiseFault(ead::kFaultNoPsram);

  const QueueHandle_t frames = xQueueCreate(64, sizeof(ead::RawFrame));
  acquisition::start(frames);
  const bool motorTest = motors::begin();
  device::captureIdentity(acquisition::sensorAnswered(0) ? ead::kSensorAnswered : 0,
                          acquisition::sensorAnswered(1) ? ead::kSensorAnswered : 0, psramRing,
                          motorTest);
  telemetry::startProcessing(frames);
  startUsbLink();
  startWifiLink();
  device::markBooted();
}

void loop() {
  // All work runs in dedicated tasks; the Arduino loop task is not needed.
  vTaskDelete(nullptr);
}
