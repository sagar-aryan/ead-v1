// EAD-V1 bring-up — MPU6050 x2 + 6x ERM safe test.
// Refs: docs 02 (wiring), 03 (GPIO map), 04 (sensor config).
#include <Arduino.h>
#include <Wire.h>
#include "config_v1.h"

static const uint8_t kMotorGpios[EAD_MOTOR_COUNT] = {
  EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO,
  EAD_MOTOR_M4_GPIO, EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO
};

// MPU6050 registers
#define MPU_WHOAMI 0x75
#define MPU_PWR1 0x6B
#define MPU_SMPLRT 0x19
#define MPU_CONFIG 0x1A
#define MPU_GYRO_CFG 0x1B
#define MPU_ACCEL_CFG 0x1C
#define MPU_INT_PIN 0x37
#define MPU_INT_EN 0x38
#define MPU_ACCEL_X 0x3B

static uint8_t mpuReadReg(uint8_t addr, uint8_t reg) {
  Wire.beginTransmission(addr);
  Wire.write(reg);
  Wire.endTransmission(false);
  Wire.requestFrom(addr, (uint8_t)1);
  return Wire.available() ? Wire.read() : 0xFF;
}
static void mpuWriteReg(uint8_t addr, uint8_t reg, uint8_t val) {
  Wire.beginTransmission(addr);
  Wire.write(reg);
  Wire.write(val);
  Wire.endTransmission(true);
}
static bool mpuInit(uint8_t addr, const char* name) {
  uint8_t who = mpuReadReg(addr, MPU_WHOAMI);
  Serial.printf("%s 0x%02X WHO_AM_I=0x%02X %s\n", name, addr, who,
                (who == 0x68) ? "OK" : "FAIL");
  if (who != 0x68) return false;
  mpuWriteReg(addr, MPU_PWR1, 0x01);      // PLL X gyro ref
  delay(50);
  mpuWriteReg(addr, MPU_SMPLRT, 9);       // 1kHz/10 = 100Hz
  mpuWriteReg(addr, MPU_CONFIG, 3);       // DLPF 42Hz
  mpuWriteReg(addr, MPU_GYRO_CFG, 0x08);  // +-500 dps
  mpuWriteReg(addr, MPU_ACCEL_CFG, 0x08); // +-4g
  mpuWriteReg(addr, MPU_INT_PIN, 0x00);
  mpuWriteReg(addr, MPU_INT_EN, 0x01);    // DRDY
  return true;
}
struct Raw6 { int16_t ax, ay, az, gx, gy, gz; };
static bool mpuRead6(uint8_t addr, Raw6& r, int16_t& t) {
  Wire.beginTransmission(addr);
  Wire.write(MPU_ACCEL_X);
  if (Wire.endTransmission(false) != 0) return false;
  if (Wire.requestFrom(addr, (uint8_t)14) != 14) return false;
  r.ax = (Wire.read() << 8) | Wire.read();
  r.ay = (Wire.read() << 8) | Wire.read();
  r.az = (Wire.read() << 8) | Wire.read();
  t = (Wire.read() << 8) | Wire.read();
  r.gx = (Wire.read() << 8) | Wire.read();
  r.gy = (Wire.read() << 8) | Wire.read();
  r.gz = (Wire.read() << 8) | Wire.read();
  return true;
}

static void motorsSafeInit() {
  for (uint8_t i = 0; i < EAD_MOTOR_COUNT; i++) {
    pinMode(kMotorGpios[i], OUTPUT);
    digitalWrite(kMotorGpios[i], LOW);
  }
  analogWriteFrequency(200);
}

void setup() {
  Serial.begin(115200);
  uint32_t t0 = millis();
  while (!Serial && (millis() - t0) < 1500) {}
  motorsSafeInit();
  pinMode(EAD_PIN_FOOT_IMU_INT, INPUT);
  pinMode(EAD_PIN_SHANK_IMU_INT, INPUT);
  Wire.begin(EAD_PIN_I2C_SDA, EAD_PIN_I2C_SCL, EAD_I2C_HZ);

  Serial.println(F("EAD-V1 bring-up"));
  // I2C scan
  Serial.println(F("I2C scan:"));
  for (uint8_t a = 1; a < 127; a++) {
    Wire.beginTransmission(a);
    if (Wire.endTransmission() == 0) Serial.printf("  found 0x%02X\n", a);
  }
  bool fok = mpuInit(EAD_FOOT_MPU_ADDR, "Foot");
  bool sok = mpuInit(EAD_SHANK_MPU_ADDR, "Shank");
  Serial.printf("Foot %s  Shank %s\n", fok ? "OK" : "FAIL", sok ? "OK" : "FAIL");
  if (!fok || !sok) Serial.println(F("FAULT: check AD0 wiring (foot->GND, shank->3V3)"));
  else Serial.println(F("Send 'm' to run 6-motor safe test (0.6s each @30%). Motors otherwise OFF."));
}

static uint32_t s_lastMs = 0, s_n = 0;
static void motorTest() {
  for (uint8_t i = 0; i < EAD_MOTOR_COUNT; i++) {
    Serial.printf("MOTOR M%d GPIO%d ON 0.6s\n", i + 1, kMotorGpios[i]);
    analogWrite(kMotorGpios[i], 80);  // ~30%
    delay(600);
    analogWrite(kMotorGpios[i], 0);
    digitalWrite(kMotorGpios[i], LOW);
    delay(400);
  }
  Serial.println(F("MOTOR TEST DONE (all OFF)"));
}

void loop() {
  if (Serial.available()) {
    char c = Serial.read();
    if (c == 'm' || c == 'M') motorTest();
  }
  uint32_t now = millis();
  if (now - s_lastMs >= 10) {  // 100Hz read
    s_lastMs = now;
    Raw6 f, s;
    int16_t tf, ts;
    bool okf = mpuRead6(EAD_FOOT_MPU_ADDR, f, tf);
    bool oks = mpuRead6(EAD_SHANK_MPU_ADDR, s, ts);
    s_n++;
    if (s_n % 10 == 0) {  // print @10Hz
      int fi = digitalRead(EAD_PIN_FOOT_IMU_INT);
      int si = digitalRead(EAD_PIN_SHANK_IMU_INT);
      // +-4g => 8192 LSB/g ; +-500dps => 65.5 LSB/dps
      Serial.printf("F[%d] a=%.2f,%.2f,%.2f g g=%.0f,%.0f,%.0f dps INT=%d | S[%d] a=%.2f,%.2f,%.2f g g=%.0f,%.0f,%.0f dps INT=%d\n",
        okf, f.ax/8192.0, f.ay/8192.0, f.az/8192.0, f.gx/65.5, f.gy/65.5, f.gz/65.5, fi,
        oks, s.ax/8192.0, s.ay/8192.0, s.az/8192.0, s.gx/65.5, s.gy/65.5, s.gz/65.5, si);
      // Machine-readable line for tools/orient_viewer.py (physical units).
      Serial.printf("CSV,%.4f,%.4f,%.4f,%.2f,%.2f,%.2f,%.4f,%.4f,%.4f,%.2f,%.2f,%.2f,%d,%d\n",
        f.ax/8192.0, f.ay/8192.0, f.az/8192.0, f.gx/65.5, f.gy/65.5, f.gz/65.5,
        s.ax/8192.0, s.ay/8192.0, s.az/8192.0, s.gx/65.5, s.gy/65.5, s.gz/65.5, fi, si);
    }
  }
}
