// EAD-V1 firmware: data-ready-clocked 100 Hz acquisition of the foot and shank
// IMUs, streamed as binary protocol messages over Wi-Fi and USB.
// Architecture: docs/architecture.md. Protocol: docs/protocol.md.
#include <Arduino.h>
#include <Wire.h>
#include <esp_log.h>

#include "acquisition.h"
#include "config_v1.h"
#include "device.h"
#include "ead/protocol.h"
#include "imu.h"
#include "link_usb.h"
#include "link_wifi.h"
#include "telemetry.h"

static const uint8_t kMotorGpios[EAD_MOTOR_COUNT] = {
    EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO,
    EAD_MOTOR_M4_GPIO, EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO};

void setup() {
  // Motor outputs first: doc 03 §4 requires them LOW before anything else.
  // No drivers are fitted and nothing else touches these pins (DEC-006).
  for (uint8_t pin : kMotorGpios) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, LOW);
  }

  // USB carries only binary frames (DEC-005) through link_usb.cpp, which drives
  // the USB Serial/JTAG FIFO itself; Arduino Serial is never started. Silence
  // ESP-IDF logging, which would otherwise write into the same FIFO.
  esp_log_level_set("*", ESP_LOG_NONE);

  imu::recoverBus(EAD_PIN_I2C_SDA, EAD_PIN_I2C_SCL);
  Wire.begin(EAD_PIN_I2C_SDA, EAD_PIN_I2C_SCL, EAD_I2C_HZ);
  Wire.setTimeOut(5);

  const bool psramRing = telemetry::begin();
  if (!psramRing) device::raiseFault(ead::kFaultNoPsram);

  const QueueHandle_t frames = xQueueCreate(64, sizeof(ead::RawFrame));
  const acquisition::BootReport boot = acquisition::start(frames);
  device::captureIdentity(boot.whoFoot, boot.whoShank, psramRing);
  telemetry::startProcessing(frames);
  startUsbLink();
  startWifiLink();
  device::markBooted();
}

void loop() {
  // All work runs in dedicated tasks; the Arduino loop task is not needed.
  vTaskDelete(nullptr);
}
