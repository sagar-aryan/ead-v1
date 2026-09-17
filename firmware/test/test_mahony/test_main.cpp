#include <unity.h>

#include <cmath>

#include "ead/mahony.h"

void setUp() {}
void tearDown() {}

namespace {

constexpr float kDt = 0.01f;  // 100 Hz (doc 04 §5)
constexpr float kKp = 2.0f;
constexpr float kKi = 0.05f;
constexpr float kRadToDeg = 57.295779513f;

/// Rotates the world +Z axis into the body frame: where the estimator thinks
/// down is. Comparing this with the measured acceleration is the honest check —
/// it avoids asserting on a quaternion's sign convention.
void predictedGravity(const float q[4], float out[3]) {
  const float w = q[0], x = q[1], y = q[2], z = q[3];
  out[0] = 2.0f * (x * z - w * y);
  out[1] = 2.0f * (w * x + y * z);
  out[2] = w * w - x * x - y * y + z * z;
}

float angleBetween(const float a[3], const float b[3]) {
  const float dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const float la = std::sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]);
  const float lb = std::sqrt(b[0] * b[0] + b[1] * b[1] + b[2] * b[2]);
  float cosine = dot / (la * lb);
  if (cosine > 1.0f) cosine = 1.0f;
  if (cosine < -1.0f) cosine = -1.0f;
  return std::acos(cosine) * kRadToDeg;
}

void run(ead::Mahony* filter, const float gyro[3], const float accel[3], int steps) {
  for (int i = 0; i < steps; ++i) filter->update(gyro, accel, kDt, kKp, kKi);
}

}  // namespace

static void test_a_still_upright_sensor_stays_upright() {
  ead::Mahony filter;
  const float identity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  filter.reset(identity);
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  const float accel[3] = {0.0f, 0.0f, 1.0f};
  run(&filter, gyro, accel, 500);

  float gravity[3];
  predictedGravity(filter.quaternion(), gravity);
  TEST_ASSERT_FLOAT_WITHIN(0.01f, 1.0f, gravity[2]);
}

static void test_gravity_pulls_a_wrong_orientation_back() {
  // The estimator starts upright but the sensor is tilted 30 degrees about X.
  ead::Mahony filter;
  const float identity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  filter.reset(identity);
  const float tilt = 30.0f / kRadToDeg;
  const float accel[3] = {0.0f, std::sin(tilt), std::cos(tilt)};
  const float gyro[3] = {0.0f, 0.0f, 0.0f};

  float gravity[3];
  predictedGravity(filter.quaternion(), gravity);
  TEST_ASSERT_FLOAT_WITHIN(0.5f, 30.0f, angleBetween(gravity, accel));

  // Kp = 2.0 brings it in within a couple of seconds and holds it there.
  run(&filter, gyro, accel, 400);
  predictedGravity(filter.quaternion(), gravity);
  TEST_ASSERT_TRUE(angleBetween(gravity, accel) < 1.0f);
}

static void test_the_gyroscope_integrates_a_turn() {
  // 90 deg/s about Z for one second is a quarter turn. Yaw has no gravity
  // reference, so this tests the integration alone.
  ead::Mahony filter;
  const float identity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  filter.reset(identity);
  const float gyro[3] = {0.0f, 0.0f, 90.0f};
  const float accel[3] = {0.0f, 0.0f, 1.0f};
  run(&filter, gyro, accel, 100);

  // The rotation angle of the quaternion: 2 * acos(w).
  const float angle = 2.0f * std::acos(filter.quaternion()[0]) * kRadToDeg;
  TEST_ASSERT_FLOAT_WITHIN(1.0f, 90.0f, angle);
  // The turn is about Z, so gravity is unmoved.
  float gravity[3];
  predictedGravity(filter.quaternion(), gravity);
  TEST_ASSERT_FLOAT_WITHIN(0.01f, 1.0f, gravity[2]);
}

static void test_free_fall_leaves_the_estimate_to_the_gyroscope() {
  ead::Mahony filter;
  const float identity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  filter.reset(identity);
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  const float none[3] = {0.0f, 0.0f, 0.0f};
  run(&filter, gyro, none, 100);
  // No acceleration, no rotation: the estimate must not drift or blow up.
  TEST_ASSERT_FLOAT_WITHIN(1e-5f, 1.0f, filter.quaternion()[0]);
}

static void test_the_quaternion_stays_normalized() {
  ead::Mahony filter;
  const float identity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  filter.reset(identity);
  const float gyro[3] = {120.0f, -80.0f, 200.0f};
  const float accel[3] = {0.2f, -0.3f, 0.9f};
  run(&filter, gyro, accel, 1000);

  const float* q = filter.quaternion();
  const float length = std::sqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.0f, length);
}

static void test_relative_orientation_cancels_a_shared_heading() {
  // Both segments yawed by the same unknown amount: the relative orientation
  // must not change, which is why doc 04 §6 uses it instead of absolute yaw.
  const float yaw = 40.0f / kRadToDeg;
  const float shared[4] = {std::cos(yaw / 2), 0.0f, 0.0f, std::sin(yaw / 2)};
  const float pitch = 15.0f / kRadToDeg;
  const float footLocal[4] = {std::cos(pitch / 2), 0.0f, std::sin(pitch / 2), 0.0f};

  float qShank[4] = {shared[0], shared[1], shared[2], shared[3]};
  float qFoot[4];
  ead::quaternionMultiply(shared, footLocal, qFoot);

  float relative[4];
  ead::relativeOrientation(qShank, qFoot, relative);
  for (int i = 0; i < 4; ++i) TEST_ASSERT_FLOAT_WITHIN(1e-5f, footLocal[i], relative[i]);

  // Identical segments give the identity rotation.
  ead::relativeOrientation(qFoot, qFoot, relative);
  TEST_ASSERT_FLOAT_WITHIN(1e-5f, 1.0f, relative[0]);
}

static void test_q15_encoding_is_symmetric_and_clamped() {
  const float q[4] = {1.0f, -1.0f, 0.5f, 0.0f};
  int16_t out[4];
  ead::quaternionToQ15(q, out);
  TEST_ASSERT_EQUAL_INT16(32767, out[0]);
  TEST_ASSERT_EQUAL_INT16(-32767, out[1]);
  TEST_ASSERT_EQUAL_INT16(16384, out[2]);
  TEST_ASSERT_EQUAL_INT16(0, out[3]);
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_a_still_upright_sensor_stays_upright);
  RUN_TEST(test_gravity_pulls_a_wrong_orientation_back);
  RUN_TEST(test_the_gyroscope_integrates_a_turn);
  RUN_TEST(test_free_fall_leaves_the_estimate_to_the_gyroscope);
  RUN_TEST(test_the_quaternion_stays_normalized);
  RUN_TEST(test_relative_orientation_cancels_a_shared_heading);
  RUN_TEST(test_q15_encoding_is_symmetric_and_clamped);
  return UNITY_END();
}
