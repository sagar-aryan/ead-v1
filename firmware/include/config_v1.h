#pragma once
// EAD-V1 fixed configuration values.
// Source of truth: ead_agent_docs_v2/CONFIG_V1.json
// (mirrors 00_README.md / 01_SYSTEM_SPEC.md / 03_GPIO_PIN_MAP.md /
//  04_SENSOR_CALIBRATION_AND_ORIENTATION.md / 05 / 06 / 08 / 09).
// Do NOT invent new sensors, GPIOs, thresholds, or mechanisms.
// Explicitly OUT OF SCOPE in V1: battery ADC, switch GPIO, BLE, BNO086, FSRs.

#include <stdint.h>

// ---- Device identity ----
#define EAD_DEVICE_NAME       "EAD-V1"
#define EAD_LIMB              "RIGHT_LEG_ONLY"
#define EAD_FOOTWEAR          "BAREFOOT"

// ---- GPIO map (doc 03, CONFIG_V1.json "pins") ----
#define EAD_PIN_I2C_SDA            5
#define EAD_PIN_I2C_SCL            6
#define EAD_PIN_FOOT_IMU_INT       7
#define EAD_PIN_SHANK_IMU_INT      8
#define EAD_MOTOR_M1_GPIO          1
#define EAD_MOTOR_M2_GPIO          2
#define EAD_MOTOR_M3_GPIO          4
#define EAD_MOTOR_M4_GPIO          9
#define EAD_MOTOR_M5_GPIO          43
#define EAD_MOTOR_M6_GPIO          44
#define EAD_MOTOR_COUNT            6

// ---- MPU6050 (docs 01/02/04, CONFIG_V1.json "mpu6050") ----
#define EAD_FOOT_MPU_ADDR          0x68
#define EAD_SHANK_MPU_ADDR         0x69
#define EAD_I2C_HZ                 400000u
#define EAD_SAMPLE_HZ              100u
#define EAD_ACCEL_RANGE_G          4
#define EAD_GYRO_RANGE_DPS         500
#define EAD_MPU_DLPF_HZ            42
#define EAD_MPU_DLPF_CFG           3
#define EAD_MPU_SMPLRT_DIV         9
// Clock source: PLL with X-axis gyro reference (see MPU6050 driver setup).
// Sensitivity at the configured ranges (same for MPU6050 and MPU6500).
#define EAD_ACCEL_LSB_PER_G        8192.0f  // +-4 g
#define EAD_GYRO_LSB_PER_DPS       65.5f    // +-500 dps

// ---- Coordinate frame (doc 04) ----
// X+ forward toward toes, Y+ medial/left (right leg), Z+ up.
// This anatomical frame is right-handed (X x Y = Z), and so is the MPU chip
// frame. Any rigid mounting is therefore a proper rotation: a mount map with
// determinant -1 is physically impossible and would make gyro rates disagree
// with accel-derived tilt (see docs/problems.md PROB-002).

// ---- Mount maps: anatomical = M * chip, for accel AND gyro ----
struct EadMountMap {
  int8_t m[3][3];
};

constexpr int eadMountDet(const EadMountMap& a) {
  return a.m[0][0] * (a.m[1][1] * a.m[2][2] - a.m[1][2] * a.m[2][1]) -
         a.m[0][1] * (a.m[1][0] * a.m[2][2] - a.m[1][2] * a.m[2][0]) +
         a.m[0][2] * (a.m[1][0] * a.m[2][1] - a.m[1][1] * a.m[2][0]);
}

// True when M * M^T = I, i.e. every row and column holds exactly one +-1.
constexpr bool eadMountIsSignedPermutation(const EadMountMap& a) {
  for (int r = 0; r < 3; r++) {
    for (int c = 0; c < 3; c++) {
      int dot = 0;
      for (int k = 0; k < 3; k++) dot += a.m[r][k] * a.m[c][k];
      if (dot != (r == c ? 1 : 0)) return false;
    }
  }
  return true;
}

// Foot (0x68): board flat on the dorsum, chip X toward the toes, chip Z up.
constexpr EadMountMap kEadFootMount = {{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}};

// Shank (0x69): board on the anterior shin, derived from measurement on the leg
// rather than from the placement image, which does not match how the board is
// actually strapped (TEST-027, 2026-09-17).
//
// The earlier map (anatX = -chipZ, anatY = +chipX, anatZ = -chipY) put gravity
// on anatomical Y (+0.99 g standing still) and put a knee extension on
// anatomical Z (+43 deg/s), i.e. Y and Z were interchanged. Correcting that
// measurement (Z_true = Y_measured, Y_true = -Z_measured) gives:
//   anatX = -chipZ   chip +Z points posteriorly, toward the bone
//   anatY = +chipY   chip +Y points medially
//   anatZ = +chipX   chip +X points up the leg, so gravity reads +Z
constexpr EadMountMap kEadShankMount = {{{0, 0, -1}, {0, 1, 0}, {1, 0, 0}}};

static_assert(eadMountIsSignedPermutation(kEadFootMount) &&
                  eadMountDet(kEadFootMount) == 1,
              "foot mount map must be a proper rotation");
static_assert(eadMountIsSignedPermutation(kEadShankMount) &&
                  eadMountDet(kEadShankMount) == 1,
              "shank mount map must be a proper rotation");

// int32 output: negating a raw -32768 count must not overflow.
inline void eadMountApply(const EadMountMap& map, int32_t cx, int32_t cy,
                          int32_t cz, int32_t out[3]) {
  for (int r = 0; r < 3; r++) {
    out[r] = map.m[r][0] * cx + map.m[r][1] * cy + map.m[r][2] * cz;
  }
}

// ---- Calibration / orientation (doc 04) ----
#define EAD_CAL_STATIC_S           5u
#define EAD_MAHONY_KP              2.0f
#define EAD_MAHONY_KI              0.05f
#define EAD_GAIT_LOWPASS_HZ        6.0f
#define EAD_EVENT_PATH_LOWPASS_HZ  20.0f

// ---- Gait guards (doc 05, CONFIG_V1.json "gait") ----
#define EAD_MIN_CYCLE_S            0.45f
#define EAD_MAX_CYCLE_S            3.0f
#define EAD_EVENT_CONTACT_GUARD_MS 30u
#define EAD_TOEOFF_GUARD_MS        40u
#define EAD_IC_CANDIDATE_WINDOW_MS 120u

// ---- ZUPT, foot-only (doc 05, CONFIG_V1.json "zupt") ----
#define EAD_ZUPT_ACCEL_TOL_G       0.15f
#define EAD_ZUPT_GYRO_THRESH_DPS   25.0f
#define EAD_ZUPT_MIN_DUR_MS        60u
#define EAD_ZUPT_ENTRY_HYST_MS     20u

// ---- Error score (doc 06, CONFIG_V1.json "error") ----
#define EAD_SCORE_MIN              0.0f
#define EAD_SCORE_MAX              1.0f
#define EAD_CLASS_ACTIVATION_TH    0.35f
#define EAD_HAPTIC_ON_TH           0.35f
#define EAD_HAPTIC_OFF_TH          0.25f
#define EAD_CONF_HAPTIC_TH         0.75f
#define EAD_ROBUST_Z_CLIP          3.0f
// Error weights: swing_dorsi .25, IC_plantar .15, inv Ever .15,
// timing .15, stance/swing .10, cycle_dist .10, shank_dyn .10.
#define EAD_W_SWING_DORSI          0.25f
#define EAD_W_IC_PLANTAR           0.15f
#define EAD_W_INV_EVER             0.15f
#define EAD_W_TIMING               0.15f
#define EAD_W_STANCE_SWING         0.10f
#define EAD_W_CYCLE_DIST           0.10f
#define EAD_W_SHANK_DYN            0.10f

// ---- Haptics (doc 06, CONFIG_V1.json "haptics") ----
#define EAD_HAPTIC_PWM_HZ          200u
#define EAD_HAPTIC_RES_BITS        8u
#define EAD_HAPTIC_MIN_DUTY        51u    // 20%
#define EAD_HAPTIC_MAX_DUTY        204u   // 80%
#define EAD_HAPTIC_INT_EXP         1.5f
#define EAD_HAPTIC_MAX_ON_S        5u
#define EAD_HAPTIC_ROLL_WIN_S      10u
#define EAD_HAPTIC_ROLL_DUTY_LIM   0.50f
// Motor positions (deg, circumferential).
#define EAD_MOTOR_M1_DEG           0u
#define EAD_MOTOR_M2_DEG           60u
#define EAD_MOTOR_M3_DEG           120u
#define EAD_MOTOR_M4_DEG           180u
#define EAD_MOTOR_M5_DEG           240u
#define EAD_MOTOR_M6_DEG           300u
// ERM driver channels are not fitted in this build (docs/decisions.md DEC-006).
#define EAD_HAPTICS_FITTED         0u

// ---- Network (doc 08) ----
#define EAD_NET_AP_IP              "192.168.4.1"
#define EAD_NET_AP_IP_0            192
#define EAD_NET_AP_IP_1            168
#define EAD_NET_AP_IP_2            4
#define EAD_NET_AP_IP_3            1
#define EAD_NET_PORT               8080
#define EAD_NET_WS_PATH            "/ws"
#define EAD_WS_PROTO_VER           1u
#define EAD_SAMPLE_BATCH_FRAMES    10u

// ---- Storage (doc 09) ----
#define EAD_STORAGE_BLOCK_BYTES    4096u
#define EAD_STORAGE_FLUSH_MS       500u
#define EAD_STORAGE_FREE_FLOOR     0.15f

// ---- Reference capture (doc 12) ----
#define EAD_REF_MIN_CYCLES         30u
#define EAD_REF_CHECK_CYCLES       10u
// Preferred valid cycles: 50-100+. Haptics OFF during reference capture.
