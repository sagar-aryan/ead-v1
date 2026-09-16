// EAD-V1 skeleton — Seeed XIAO ESP32-S3, Arduino framework.
// Refs: docs 02 (wiring), 03 (GPIO map), 07 (firmware architecture).
// Scope: USB-CDC debug, motor-GPIO safe init, I2C stub, 100 Hz timer placeholder.
// Explicitly NOT in V1 skeleton: battery/switch ADC, BLE, BNO086, FSR code.

#include <Arduino.h>
#include <Wire.h>
#include "config_v1.h"

// ---- Device states (doc 07, section 6) ----
enum class DeviceState : uint8_t {
  BOOT = 0,
  SELF_TEST,
  CALIBRATING,
  REFERENCE_CAPTURE,
  READY,
  RUNNING,
  PAUSED,
  FAULT,
  RECOVERY
};

static const char* stateName(DeviceState s) {
  switch (s) {
    case DeviceState::BOOT:              return "BOOT";
    case DeviceState::SELF_TEST:         return "SELF_TEST";
    case DeviceState::CALIBRATING:       return "CALIBRATING";
    case DeviceState::REFERENCE_CAPTURE: return "REFERENCE_CAPTURE";
    case DeviceState::READY:             return "READY";
    case DeviceState::RUNNING:           return "RUNNING";
    case DeviceState::PAUSED:            return "PAUSED";
    case DeviceState::FAULT:             return "FAULT";
    case DeviceState::RECOVERY:          return "RECOVERY";
    default:                             return "?";
  }
}

static DeviceState g_state = DeviceState::BOOT;

static const uint8_t kMotorGpios[EAD_MOTOR_COUNT] = {
  EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO,
  EAD_MOTOR_M4_GPIO, EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO
};

// ---- 100 Hz timer placeholder ----
// TODO(EAD-V1): replace this polling stub with a hw_timer_t 10 ms
// acquisition backbone (doc 07 section 2: deterministic 100 Hz timer,
// dual data-ready INTs on GPIO7/8, one master timestamp + frame seq
// per synchronized frame). Keep Wi-Fi/storage off the acquisition path.
static uint32_t s_lastTickMs = 0;
static uint32_t s_frameSeq = 0;

static void motorsSafeInit() {
  // Doc 03 section 4: all six motor GPIOs as OUTPUT, driven LOW,
  // before PWM timers are enabled. Outputs stay OFF until READY
  // and all sensor health checks pass.
  for (uint8_t i = 0; i < EAD_MOTOR_COUNT; i++) {
    pinMode(kMotorGpios[i], OUTPUT);
    digitalWrite(kMotorGpios[i], LOW);
  }
}

static void i2cStubInit() {
  // Doc 02 section 2: shared bus, SDA=GPIO5, SCL=GPIO6, 400 kHz.
  // Stub only: full MPU6050 bring-up (WHO_AM_I, DLPF_CFG=3,
  // SMPLRT_DIV=9, +/-4g, +/-500dps, DRDY INTs) lands in hardware/mpu6050.
  Wire.begin(EAD_PIN_I2C_SDA, EAD_PIN_I2C_SCL, EAD_I2C_HZ);
}

void setup() {
  // Native USB-CDC debug/log port (doc 03: GPIO43/44 free because
  // UART0 unused; GPIO19/20 left for USB).
  Serial.begin(115200);
  // Brief wait so the boot banner is visible on USB-CDC monitors.
  uint32_t t0 = millis();
  while (!Serial && (millis() - t0) < 1500) { /* wait */ }

  motorsSafeInit();
  i2cStubInit();

  g_state = DeviceState::SELF_TEST;  // skeleton parks here; real bring-up later
  Serial.println(F("EAD-V1 boot"));
  Serial.print(F("state="));
  Serial.println(stateName(g_state));
  Serial.print(F("motors: 6x OUTPUT LOW on GPIOs "));
  for (uint8_t i = 0; i < EAD_MOTOR_COUNT; i++) {
    if (i) Serial.print(',');
    Serial.print(kMotorGpios[i]);
  }
  Serial.println();
  Serial.println(F("I2C: SDA=GPIO5 SCL=GPIO6 400kHz (stub)"));
  Serial.println(F("timer: 100Hz acquisition placeholder (stub)"));
}

void loop() {
  // 100 Hz placeholder tick: real acquisition ISR/task replaces this.
  uint32_t now = millis();
  if (now - s_lastTickMs >= 10) {
    s_lastTickMs = now;
    s_frameSeq++;
    // TODO(EAD-V1): read foot (0x68) + shank (0x69) frame,
    // stamp master time_us + frame seq, hand to pipeline.
  }
}
