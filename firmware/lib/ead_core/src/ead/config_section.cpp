#include "ead/config_section.h"

#include <initializer_list>

#include "config_v1.h"
#include "ead/bytes.h"

namespace ead {

namespace {

void writeMount(ByteWriter& w, const EadMountMap& map) {
  for (const auto& row : map.m) {
    for (int8_t v : row) w.i8(v);
  }
}

}  // namespace

size_t encodeConfigSection(uint8_t* out, size_t cap) {
  ByteWriter w(out, cap);

  // IMUs
  w.u8(EAD_FOOT_MPU_ADDR);
  w.u8(EAD_SHANK_MPU_ADDR);
  w.u32(EAD_I2C_HZ);
  w.u16(EAD_SAMPLE_HZ);
  w.u8(EAD_ACCEL_RANGE_G);
  w.u16(EAD_GYRO_RANGE_DPS);
  w.u8(EAD_MPU_DLPF_HZ);
  w.u8(EAD_MPU_DLPF_CFG);
  w.u8(EAD_MPU_SMPLRT_DIV);
  w.f32(EAD_ACCEL_LSB_PER_G);
  w.f32(EAD_GYRO_LSB_PER_DPS);
  writeMount(w, kEadFootMount);
  writeMount(w, kEadShankMount);

  // Pins
  w.u8(EAD_PIN_I2C_SDA);
  w.u8(EAD_PIN_I2C_SCL);
  w.u8(EAD_PIN_FOOT_IMU_INT);
  w.u8(EAD_PIN_SHANK_IMU_INT);
  for (uint8_t pin : {EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO, EAD_MOTOR_M4_GPIO,
                      EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO}) {
    w.u8(pin);
  }

  // Calibration and orientation
  w.u8(EAD_CAL_STATIC_S);
  w.f32(EAD_MAHONY_KP);
  w.f32(EAD_MAHONY_KI);

  // Gait
  w.f32(EAD_GAIT_LOWPASS_HZ);
  w.f32(EAD_EVENT_PATH_LOWPASS_HZ);
  w.f32(EAD_MIN_CYCLE_S);
  w.f32(EAD_MAX_CYCLE_S);
  w.u16(EAD_EVENT_CONTACT_GUARD_MS);
  w.u16(EAD_TOEOFF_GUARD_MS);
  w.u16(EAD_IC_CANDIDATE_WINDOW_MS);

  // ZUPT
  w.f32(EAD_ZUPT_ACCEL_TOL_G);
  w.f32(EAD_ZUPT_GYRO_THRESH_DPS);
  w.u16(EAD_ZUPT_MIN_DUR_MS);
  w.u16(EAD_ZUPT_ENTRY_HYST_MS);

  // Error engine
  w.f32(EAD_CLASS_ACTIVATION_TH);
  w.f32(EAD_HAPTIC_ON_TH);
  w.f32(EAD_HAPTIC_OFF_TH);
  w.f32(EAD_CONF_HAPTIC_TH);
  w.f32(EAD_ROBUST_Z_CLIP);
  for (float weight : {EAD_W_SWING_DORSI, EAD_W_IC_PLANTAR, EAD_W_INV_EVER, EAD_W_TIMING,
                       EAD_W_STANCE_SWING, EAD_W_CYCLE_DIST, EAD_W_SHANK_DYN}) {
    w.f32(weight);
  }

  // Haptics (contract values; hardware not fitted)
  w.u8(EAD_HAPTICS_FITTED);
  w.u16(EAD_HAPTIC_PWM_HZ);
  w.u8(EAD_HAPTIC_RES_BITS);
  w.u8(EAD_HAPTIC_MIN_DUTY);
  w.u8(EAD_HAPTIC_MAX_DUTY);
  w.f32(EAD_HAPTIC_INT_EXP);
  w.u8(EAD_HAPTIC_MAX_ON_S);
  w.u8(EAD_HAPTIC_ROLL_WIN_S);
  w.f32(EAD_HAPTIC_ROLL_DUTY_LIM);
  for (uint16_t deg : {EAD_MOTOR_M1_DEG, EAD_MOTOR_M2_DEG, EAD_MOTOR_M3_DEG, EAD_MOTOR_M4_DEG,
                       EAD_MOTOR_M5_DEG, EAD_MOTOR_M6_DEG}) {
    w.u16(deg);
  }

  // Network
  for (uint8_t octet : {EAD_NET_AP_IP_0, EAD_NET_AP_IP_1, EAD_NET_AP_IP_2, EAD_NET_AP_IP_3}) {
    w.u8(octet);
  }
  w.u16(EAD_NET_PORT);
  w.u8(EAD_SAMPLE_BATCH_FRAMES);

  // Storage
  w.u16(EAD_STORAGE_BLOCK_BYTES);
  w.u16(EAD_STORAGE_FLUSH_MS);
  w.f32(EAD_STORAGE_FREE_FLOOR);

  // Reference
  w.u16(EAD_REF_MIN_CYCLES);
  w.u16(EAD_REF_CHECK_CYCLES);

  return w.ok() ? w.size() : 0;
}

}  // namespace ead
