#pragma once
// Register-level driver for the MPU6050/MPU6500 IMUs (docs/hardware.md).
// Only the acquisition task uses the I2C bus after boot.

#include <cstdint>

namespace imu {

// Frees the bus before the I2C driver starts. A reset during a read can leave
// an IMU mid-byte holding SDA low; clocking SCL until SDA is released and then
// sending STOP returns every device to idle.
void recoverBus(uint8_t sdaPin, uint8_t sclPin);

struct ConfigResult {
  uint8_t who;    // WHO_AM_I read, 0 when the device did not answer
  bool present;   // WHO_AM_I is 0x68 (MPU6050) or 0x70 (MPU6500)
  bool configOk;  // every configured register read back as written
};

// Writes the doc-00 configuration (including ACCEL_CONFIG2 on MPU6500) and
// verifies it by readback.
ConfigResult configure(uint8_t addr);

// Burst-reads INT_STATUS, accel, temperature and gyro (0x3A..0x48).
// out = chip-frame counts {ax, ay, az, gx, gy, gz}; `fresh` is the data-ready
// flag, false when this sample was already read. Returns false on I2C failure.
bool readSample(uint8_t addr, int16_t out[6], bool* fresh);

// PWR_MGMT_1 still holds the configured value. A brown-out resets the IMU to
// its sleep default, after which the data registers stop changing.
bool powerOk(uint8_t addr);

}  // namespace imu
