#include "imu.h"

#include <Arduino.h>
#include <Wire.h>

#include "config_v1.h"

namespace imu {

namespace {

constexpr uint8_t kWho6050 = 0x68;
constexpr uint8_t kWho6500 = 0x70;

constexpr uint8_t kRegSmplrtDiv = 0x19;
constexpr uint8_t kRegConfig = 0x1A;
constexpr uint8_t kRegGyroConfig = 0x1B;
constexpr uint8_t kRegAccelConfig = 0x1C;
constexpr uint8_t kRegAccelConfig2 = 0x1D;  // MPU6500 only
constexpr uint8_t kRegIntPinCfg = 0x37;
constexpr uint8_t kRegIntEnable = 0x38;
constexpr uint8_t kRegIntStatus = 0x3A;
constexpr uint8_t kRegPwrMgmt1 = 0x6B;
constexpr uint8_t kRegWhoAmI = 0x75;

constexpr uint8_t kPwrPllClock = 0x01;
constexpr uint8_t kGyro500Dps = 0x08;
constexpr uint8_t kAccel4G = 0x08;
constexpr uint8_t kIntPulseActiveHigh = 0x00;
constexpr uint8_t kIntDataReady = 0x01;

bool writeRegister(uint8_t addr, uint8_t reg, uint8_t value) {
  Wire.beginTransmission(addr);
  Wire.write(reg);
  Wire.write(value);
  return Wire.endTransmission(true) == 0;
}

bool readRegisters(uint8_t addr, uint8_t reg, uint8_t* out, uint8_t n) {
  Wire.beginTransmission(addr);
  Wire.write(reg);
  if (Wire.endTransmission(false) != 0) return false;
  if (Wire.requestFrom(addr, n) != n) return false;
  return Wire.readBytes(out, n) == n;
}

int16_t be16(const uint8_t* p) { return int16_t(uint16_t(p[0]) << 8 | p[1]); }

}  // namespace

void recoverBus(uint8_t sdaPin, uint8_t sclPin) {
  constexpr uint32_t kHalfPeriodUs = 5;  // well below 400 kHz; slow enough for any IMU
  pinMode(sdaPin, INPUT_PULLUP);
  pinMode(sclPin, OUTPUT_OPEN_DRAIN);
  digitalWrite(sclPin, HIGH);
  delayMicroseconds(kHalfPeriodUs);
  for (int pulse = 0; pulse < 9 && digitalRead(sdaPin) == LOW; pulse++) {
    digitalWrite(sclPin, LOW);
    delayMicroseconds(kHalfPeriodUs);
    digitalWrite(sclPin, HIGH);
    delayMicroseconds(kHalfPeriodUs);
  }
  // STOP condition: SDA rises while SCL is high.
  pinMode(sdaPin, OUTPUT_OPEN_DRAIN);
  digitalWrite(sdaPin, LOW);
  delayMicroseconds(kHalfPeriodUs);
  digitalWrite(sclPin, HIGH);
  delayMicroseconds(kHalfPeriodUs);
  digitalWrite(sdaPin, HIGH);
  delayMicroseconds(kHalfPeriodUs);
  pinMode(sdaPin, INPUT);
  pinMode(sclPin, INPUT);
}

ConfigResult configure(uint8_t addr) {
  ConfigResult result{0, false, false};
  uint8_t who = 0;
  if (!readRegisters(addr, kRegWhoAmI, &who, 1)) return result;
  result.who = who;
  result.present = (who == kWho6050 || who == kWho6500);
  if (!result.present) return result;
  const bool is6500 = (who == kWho6500);

  struct Setting {
    uint8_t reg;
    uint8_t value;
    uint8_t mask;
  };
  // On MPU6500 CONFIG filters only the gyro; ACCEL_CONFIG2 filters the accel.
  const Setting settings[] = {
      {kRegSmplrtDiv, EAD_MPU_SMPLRT_DIV, 0xFF},
      {kRegConfig, EAD_MPU_DLPF_CFG, 0x07},
      {kRegGyroConfig, kGyro500Dps, 0xFF},
      {kRegAccelConfig, kAccel4G, 0xFF},
      {kRegIntPinCfg, kIntPulseActiveHigh, 0xFF},
      {kRegIntEnable, kIntDataReady, 0xFF},
      {kRegAccelConfig2, EAD_MPU_DLPF_CFG, 0x0F},  // keep last: MPU6500 only
  };
  const size_t count = sizeof settings / sizeof settings[0] - (is6500 ? 0 : 1);

  bool ok = writeRegister(addr, kRegPwrMgmt1, kPwrPllClock);
  delay(50);  // clock source switch settles before further writes
  for (size_t i = 0; i < count; i++) ok &= writeRegister(addr, settings[i].reg, settings[i].value);

  uint8_t value = 0;
  ok &= readRegisters(addr, kRegPwrMgmt1, &value, 1) && value == kPwrPllClock;
  for (size_t i = 0; i < count; i++) {
    ok &= readRegisters(addr, settings[i].reg, &value, 1) &&
          (value & settings[i].mask) == settings[i].value;
  }
  result.configOk = ok;
  return result;
}

bool readSample(uint8_t addr, int16_t out[6], bool* fresh) {
  uint8_t b[15];
  if (!readRegisters(addr, kRegIntStatus, b, sizeof b)) return false;
  *fresh = (b[0] & kIntDataReady) != 0;
  out[0] = be16(b + 1);
  out[1] = be16(b + 3);
  out[2] = be16(b + 5);
  // b[7..8] is temperature
  out[3] = be16(b + 9);
  out[4] = be16(b + 11);
  out[5] = be16(b + 13);
  return true;
}

bool powerOk(uint8_t addr) {
  uint8_t value = 0;
  return readRegisters(addr, kRegPwrMgmt1, &value, 1) && value == kPwrPllClock;
}

}  // namespace imu
