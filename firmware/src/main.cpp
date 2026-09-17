// EAD-V1 bring-up — foot + shank IMU (MPU6050/MPU6500) read at ~100 Hz.
// Refs: docs 02 (wiring), 03 (GPIO map), 04 (sensor config).
// ERM drivers are not fitted: motor GPIOs are only held LOW (DEC-006).
#include <Arduino.h>
#include <Wire.h>
#include "config_v1.h"

static const uint8_t kMotorGpios[EAD_MOTOR_COUNT] = {
  EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO,
  EAD_MOTOR_M4_GPIO, EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO
};

// MPU6050 / MPU6500 registers
#define MPU_WHOAMI 0x75
#define MPU_PWR1 0x6B
#define MPU_SMPLRT 0x19
#define MPU_CONFIG 0x1A
#define MPU_GYRO_CFG 0x1B
#define MPU_ACCEL_CFG 0x1C
#define MPU_ACCEL_CFG2 0x1D  // MPU6500 only
#define MPU_INT_PIN 0x37
#define MPU_INT_EN 0x38
#define MPU_ACCEL_X 0x3B

#define MPU_WHO_6050 0x68
#define MPU_WHO_6500 0x70

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

struct RegExpect {
  uint8_t reg;
  uint8_t value;
  uint8_t mask;
  const char* name;
};

static bool mpuInit(uint8_t addr, const char* name) {
  uint8_t who = mpuReadReg(addr, MPU_WHOAMI);
  // Genuine MPU6050 = 0x68. Boards sold as MPU6050 often carry MPU6500
  // silicon (WHO = 0x70): same register layout and sensitivities for this
  // configuration (+-4g = 8192 LSB/g, +-500dps = 65.5 LSB/dps), except that
  // CONFIG (0x1A) filters only the gyro and the accel filter lives in
  // ACCEL_CONFIG2 (0x1D), which resets to a ~460 Hz bandwidth.
  bool is6500 = (who == MPU_WHO_6500);
  Serial.printf("%s 0x%02X WHO_AM_I=0x%02X (%s)\n", name, addr, who,
                who == MPU_WHO_6050 ? "MPU6050" : is6500 ? "MPU6500" : "UNKNOWN");
  if (!(who == MPU_WHO_6050 || is6500)) return false;

  mpuWriteReg(addr, MPU_PWR1, 0x01);      // wake, PLL with X gyro reference
  delay(50);
  mpuWriteReg(addr, MPU_SMPLRT, EAD_MPU_SMPLRT_DIV);  // 1 kHz / (1+9) = 100 Hz
  mpuWriteReg(addr, MPU_CONFIG, EAD_MPU_DLPF_CFG);    // gyro DLPF 42 Hz
  if (is6500) mpuWriteReg(addr, MPU_ACCEL_CFG2, EAD_MPU_DLPF_CFG);  // accel ~41 Hz
  mpuWriteReg(addr, MPU_GYRO_CFG, 0x08);  // +-500 dps
  mpuWriteReg(addr, MPU_ACCEL_CFG, 0x08); // +-4 g
  mpuWriteReg(addr, MPU_INT_PIN, 0x00);   // active-high push-pull 50 us pulse
  mpuWriteReg(addr, MPU_INT_EN, 0x01);    // data-ready

  // Never trust a write: a missed config silently rescales every sample
  // (docs/problems.md PROB-001).
  const RegExpect expected[] = {
    {MPU_PWR1, 0x01, 0xFF, "PWR1"},
    {MPU_SMPLRT, EAD_MPU_SMPLRT_DIV, 0xFF, "SMPLRT"},
    {MPU_CONFIG, EAD_MPU_DLPF_CFG, 0x07, "CONFIG"},
    {MPU_GYRO_CFG, 0x08, 0xFF, "GYRO_CFG"},
    {MPU_ACCEL_CFG, 0x08, 0xFF, "ACCEL_CFG"},
    {MPU_INT_PIN, 0x00, 0xFF, "INT_PIN"},
    {MPU_INT_EN, 0x01, 0xFF, "INT_EN"},
    {MPU_ACCEL_CFG2, EAD_MPU_DLPF_CFG, 0x0F, "ACCEL_CFG2"},  // keep last: MPU6500 only
  };
  const size_t count = sizeof(expected) / sizeof(expected[0]) - (is6500 ? 0 : 1);
  bool ok = true;
  for (size_t i = 0; i < count; i++) {
    uint8_t got = mpuReadReg(addr, expected[i].reg);
    if ((got & expected[i].mask) != expected[i].value) {
      Serial.printf("%s readback %s=0x%02X expected 0x%02X\n", name,
                    expected[i].name, got, expected[i].value);
      ok = false;
    }
  }
  return ok;
}

struct Raw6 { int16_t ax, ay, az, gx, gy, gz; };
static bool mpuRead6(uint8_t addr, Raw6& r) {
  Wire.beginTransmission(addr);
  Wire.write(MPU_ACCEL_X);
  if (Wire.endTransmission(false) != 0) return false;
  if (Wire.requestFrom(addr, (uint8_t)14) != 14) return false;
  r.ax = (Wire.read() << 8) | Wire.read();
  r.ay = (Wire.read() << 8) | Wire.read();
  r.az = (Wire.read() << 8) | Wire.read();
  Wire.read();  // temperature, unused
  Wire.read();
  r.gx = (Wire.read() << 8) | Wire.read();
  r.gy = (Wire.read() << 8) | Wire.read();
  r.gz = (Wire.read() << 8) | Wire.read();
  return true;
}

static void motorsHoldLow() {
  for (uint8_t i = 0; i < EAD_MOTOR_COUNT; i++) {
    pinMode(kMotorGpios[i], OUTPUT);
    digitalWrite(kMotorGpios[i], LOW);
  }
}

static void printDiag(uint8_t addr, const char* name) {
  Serial.printf("%s 0x%02X WHO=0x%02X PWR1=0x%02X SMPL=0x%02X CFG=0x%02X GYRO=0x%02X "
                "ACCEL=0x%02X ACCEL2=0x%02X INTEN=0x%02X\n",
                name, addr, mpuReadReg(addr, MPU_WHOAMI), mpuReadReg(addr, MPU_PWR1),
                mpuReadReg(addr, MPU_SMPLRT), mpuReadReg(addr, MPU_CONFIG),
                mpuReadReg(addr, MPU_GYRO_CFG), mpuReadReg(addr, MPU_ACCEL_CFG),
                mpuReadReg(addr, MPU_ACCEL_CFG2), mpuReadReg(addr, MPU_INT_EN));
}

void setup() {
  // Motor outputs first: doc 03 §4 requires them LOW before anything else.
  motorsHoldLow();
  Serial.begin(115200);
  uint32_t t0 = millis();
  while (!Serial && (millis() - t0) < 1500) {}
  pinMode(EAD_PIN_FOOT_IMU_INT, INPUT);
  pinMode(EAD_PIN_SHANK_IMU_INT, INPUT);
  Wire.begin(EAD_PIN_I2C_SDA, EAD_PIN_I2C_SCL, EAD_I2C_HZ);

  Serial.println(F("EAD-V1 bring-up"));
  Serial.println(F("I2C scan:"));
  for (uint8_t a = 1; a < 127; a++) {
    Wire.beginTransmission(a);
    if (Wire.endTransmission() == 0) Serial.printf("  found 0x%02X\n", a);
  }
  bool fok = mpuInit(EAD_FOOT_MPU_ADDR, "Foot");
  bool sok = mpuInit(EAD_SHANK_MPU_ADDR, "Shank");
  Serial.printf("Foot %s  Shank %s\n", fok ? "OK" : "FAIL", sok ? "OK" : "FAIL");
  if (!fok || !sok) Serial.println(F("FAULT: check AD0 wiring (foot->GND, shank->3V3)"));
  Serial.println(F("Send 'c' for register diagnostics."));
}

static uint32_t s_lastMs = 0, s_n = 0;

void loop() {
  if (Serial.available()) {
    char c = Serial.read();
    if (c == 'c' || c == 'C') {
      Serial.println(F("--- DIAG ---"));
      printDiag(EAD_FOOT_MPU_ADDR, "Foot");
      printDiag(EAD_SHANK_MPU_ADDR, "Shank");
    }
  }
  uint32_t now = millis();
  if (now - s_lastMs >= 10) {  // ~100 Hz polled read; replaced by DRDY acquisition in M1
    s_lastMs = now;
    Raw6 f, s;
    bool okf = mpuRead6(EAD_FOOT_MPU_ADDR, f);
    bool oks = mpuRead6(EAD_SHANK_MPU_ADDR, s);
    s_n++;
    if (s_n % 10 == 0) {  // print at 10 Hz in the anatomical frame
      int fi = digitalRead(EAD_PIN_FOOT_IMU_INT);
      int si = digitalRead(EAD_PIN_SHANK_IMU_INT);
      int32_t fa[3], fg[3], sa[3], sg[3];
      eadMountApply(kEadFootMount, f.ax, f.ay, f.az, fa);
      eadMountApply(kEadFootMount, f.gx, f.gy, f.gz, fg);
      eadMountApply(kEadShankMount, s.ax, s.ay, s.az, sa);
      eadMountApply(kEadShankMount, s.gx, s.gy, s.gz, sg);
      const float kAccLsbPerG = 8192.0f;    // +-4 g
      const float kGyroLsbPerDps = 65.5f;   // +-500 dps
      Serial.printf("F[%d] a=%.2f,%.2f,%.2f g g=%.0f,%.0f,%.0f dps INT=%d | "
                    "S[%d] a=%.2f,%.2f,%.2f g g=%.0f,%.0f,%.0f dps INT=%d\n",
                    okf, fa[0] / kAccLsbPerG, fa[1] / kAccLsbPerG, fa[2] / kAccLsbPerG,
                    fg[0] / kGyroLsbPerDps, fg[1] / kGyroLsbPerDps, fg[2] / kGyroLsbPerDps, fi,
                    oks, sa[0] / kAccLsbPerG, sa[1] / kAccLsbPerG, sa[2] / kAccLsbPerG,
                    sg[0] / kGyroLsbPerDps, sg[1] / kGyroLsbPerDps, sg[2] / kGyroLsbPerDps, si);
      // Machine-readable lines for tools/orient_viewer.py (physical units).
      // RAW = unmapped chip axes (diagnoses mounting/scale); CSV = anatomical.
      Serial.printf("RAW,%.4f,%.4f,%.4f,%.4f,%.4f,%.4f\n",
                    f.ax / kAccLsbPerG, f.ay / kAccLsbPerG, f.az / kAccLsbPerG,
                    s.ax / kAccLsbPerG, s.ay / kAccLsbPerG, s.az / kAccLsbPerG);
      Serial.printf("CSV,%.4f,%.4f,%.4f,%.2f,%.2f,%.2f,%.4f,%.4f,%.4f,%.2f,%.2f,%.2f,%d,%d\n",
                    fa[0] / kAccLsbPerG, fa[1] / kAccLsbPerG, fa[2] / kAccLsbPerG,
                    fg[0] / kGyroLsbPerDps, fg[1] / kGyroLsbPerDps, fg[2] / kGyroLsbPerDps,
                    sa[0] / kAccLsbPerG, sa[1] / kAccLsbPerG, sa[2] / kAccLsbPerG,
                    sg[0] / kGyroLsbPerDps, sg[1] / kGyroLsbPerDps, sg[2] / kGyroLsbPerDps,
                    fi, si);
    }
  }
}
