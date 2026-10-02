#pragma once
// EAD-V1 fixed configuration values.
// Source of truth: ead_agent_docs_v2/CONFIG_V1.json
// (mirrors 00_README.md / 01_SYSTEM_SPEC.md / 03_GPIO_PIN_MAP.md /
//  04_SENSOR_CALIBRATION_AND_ORIENTATION.md / 05 / 06 / 08 / 09).
// Do NOT invent new sensors, GPIOs, thresholds, or mechanisms.
// Explicitly OUT OF SCOPE in V1: battery ADC, switch GPIO, BLE, FSRs.
// Recorded deviations: the pin map is DEC-016 (two BNO086 on SPI), the sensors
// and their scales DEC-017, motor service pulses DEC-018.

#include <stdint.h>

// ---- Device identity ----
#define EAD_DEVICE_NAME       "EAD-V1"
#define EAD_LIMB              "RIGHT_LEG_ONLY"
#define EAD_FOOTWEAR          "BAREFOOT"

// ---- GPIO map (DEC-016; replaces doc 03 and CONFIG_V1.json "pins") ----
// Both BNO086 boards share SCK, MISO, MOSI, RST and WAKE (= PS0); each has its
// own CS and INT (docs/wiring_reference.md §3).
#define EAD_PIN_SPI_SCK            7
#define EAD_PIN_SPI_MISO           8
#define EAD_PIN_SPI_MOSI           9
#define EAD_PIN_FOOT_CS            43
#define EAD_PIN_SHANK_CS           44
#define EAD_PIN_FOOT_INT           39
#define EAD_PIN_SHANK_INT          40
#define EAD_PIN_SENSOR_RST         41
#define EAD_PIN_SENSOR_WAKE        3
#define EAD_MOTOR_M1_GPIO          1
#define EAD_MOTOR_M2_GPIO          2
#define EAD_MOTOR_M3_GPIO          42
#define EAD_MOTOR_M4_GPIO          4
#define EAD_MOTOR_M5_GPIO          5
#define EAD_MOTOR_M6_GPIO          6
#define EAD_MOTOR_COUNT            6
// Bit n-1 = motor n may be driven. A channel whose wiring is in doubt is switched
// off here: motor 3 was, until GPIO42 measured about 100 kOhm to GND (PROB-020).
#define EAD_MOTOR_ENABLED_MASK     0x3Fu

// ---- Sensors: BNO086 (DEC-017; replaces CONFIG_V1.json "mpu6050") ----
#define EAD_SPI_HZ                 1000000u  // wiring rule 5: 1 MHz until soak-tested
#define EAD_SAMPLE_HZ              100u
#define EAD_REPORT_INTERVAL_US     10000u
#define EAD_ACCEL_RANGE_G          8         // BMA280 inside the BNO086 (TEST-040)
#define EAD_GYRO_RANGE_DPS         2000      // BMI055 inside the BNO086 (TEST-040)
// Calibrated reports: accelerometer Q8 m/s^2, gyroscope Q9 rad/s. As counts per
// g and per deg/s, so the processing chain keeps its units (DEC-007).
#define EAD_ACCEL_LSB_PER_G        2510.5024f  // 256 * 9.80665
#define EAD_GYRO_LSB_PER_DPS       8.9360858f  // 512 * pi / 180
// Full scale in counts, from the sensors' own metadata (TEST-040).
#define EAD_ACCEL_FULL_SCALE       20082
#define EAD_GYRO_FULL_SCALE        17863

// ---- Coordinate frame (doc 04) ----
// X+ forward toward toes, Y+ medial/left (right leg), Z+ up.
// This anatomical frame is right-handed (X x Y = Z), and so is the sensor
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

// Measured on the leg for the BNO086 boards (TEST-051, 2026-10-02): standing
// still gives the up axis, a held toe raise and a held seated knee extension the
// forward axis, Y = Z x X. Maps are measured, never derived from statements or
// images (PROB-002). The MPU6500 build's maps are in docs/hardware.md (TEST-027).
// Foot: X = -chipY, Y = +chipX, Z = +chipZ.
constexpr EadMountMap kEadFootMount = {{{0, -1, 0}, {1, 0, 0}, {0, 0, 1}}};
// Shank: X = +chipZ, Y = +chipX, Z = +chipY.
constexpr EadMountMap kEadShankMount = {{{0, 0, 1}, {1, 0, 0}, {0, 1, 0}}};

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
// No error-driven haptic feedback runs (DEC-006). The driver board is fitted and
// motors can be pulsed for the service test (DEC-018).
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

// ---- Pin safety (DEC-016, docs/wiring_reference.md §10) ----
namespace ead_pins {
constexpr int kMotors[EAD_MOTOR_COUNT] = {EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO,
                                          EAD_MOTOR_M4_GPIO, EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO};
constexpr bool isMotor(int pin) {
  for (int motor : kMotors) {
    if (motor == pin) return true;
  }
  return false;
}
}  // namespace ead_pins
static_assert(!ead_pins::isMotor(EAD_PIN_SPI_SCK) && !ead_pins::isMotor(EAD_PIN_SPI_MISO) &&
                  !ead_pins::isMotor(EAD_PIN_SPI_MOSI) && !ead_pins::isMotor(EAD_PIN_FOOT_CS) &&
                  !ead_pins::isMotor(EAD_PIN_SHANK_CS) && !ead_pins::isMotor(EAD_PIN_FOOT_INT) &&
                  !ead_pins::isMotor(EAD_PIN_SHANK_INT) && !ead_pins::isMotor(EAD_PIN_SENSOR_RST) &&
                  !ead_pins::isMotor(EAD_PIN_SENSOR_WAKE),
              "a sensor line on a motor gate would switch the motor");
// GPIO39 comes out of reset with a pull-up that would hold a gate on (DEC-016).
static_assert(!ead_pins::isMotor(39), "GPIO39 must never drive a motor");
