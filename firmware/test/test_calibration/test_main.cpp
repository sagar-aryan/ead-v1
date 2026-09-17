#include <unity.h>

#include <cmath>

#include "ead/calibration.h"

void setUp() {}
void tearDown() {}

namespace {

/// Feeds `count` identical samples, the still case the calibration assumes.
void feed(ead::CalibrationAccumulator* acc, const float accel[3], const float gyro[3],
          uint32_t count) {
  for (uint32_t i = 0; i < count; ++i) acc->add(accel, gyro);
}

void expectUnitVector(const float v[3]) {
  const float length = std::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.0f, length);
}

}  // namespace

static void test_bias_is_the_mean_rate_while_still() {
  ead::CalibrationAccumulator acc;
  acc.reset();
  const float accel[3] = {0.0f, 0.0f, 1.0f};
  // A real gyro sits at a small constant offset with noise either side of it.
  const float low[3] = {1.4f, -0.6f, 0.1f};
  const float high[3] = {1.6f, -0.4f, 0.3f};
  for (uint32_t i = 0; i < 250; ++i) acc.add(accel, i % 2 == 0 ? low : high);

  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(0, acc.finish(&out));
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.5f, out.gyroBiasDps[0]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, -0.5f, out.gyroBiasDps[1]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.2f, out.gyroBiasDps[2]);
  TEST_ASSERT_FLOAT_WITHIN(1e-3f, 0.1f, out.gyroStdDps);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.0f, out.accelMagnitudeG);
  TEST_ASSERT_FLOAT_WITHIN(1e-3f, 0.0f, out.tiltDeg);
}

static void test_alignment_rotates_measured_gravity_onto_plus_z() {
  // The foot board as actually strapped: gravity on +Z but tilted (TEST-027).
  ead::CalibrationAccumulator acc;
  acc.reset();
  const float accel[3] = {-0.43f, 0.35f, 0.86f};
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  feed(&acc, accel, gyro, 500);

  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(0, acc.finish(&out));
  expectUnitVector(out.up);
  TEST_ASSERT_FLOAT_WITHIN(0.5f, 33.0f, out.tiltDeg);

  float upright[3];
  ead::rotateByQuaternion(out.alignment, out.up, upright);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.0f, upright[0]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.0f, upright[1]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.0f, upright[2]);
}

static void test_alignment_is_identity_when_already_upright() {
  ead::CalibrationAccumulator acc;
  acc.reset();
  const float accel[3] = {0.0f, 0.0f, 1.0f};
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  feed(&acc, accel, gyro, 300);

  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(0, acc.finish(&out));
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, 1.0f, out.alignment[0]);
  for (int i = 1; i < 4; ++i) TEST_ASSERT_FLOAT_WITHIN(1e-6f, 0.0f, out.alignment[i]);
}

static void test_movement_during_the_window_is_rejected() {
  ead::CalibrationAccumulator acc;
  acc.reset();
  const float accel[3] = {0.0f, 0.0f, 1.0f};
  const float still[3] = {0.0f, 0.0f, 0.0f};
  const float moved[3] = {0.0f, 0.0f, 40.0f};
  for (uint32_t i = 0; i < 300; ++i) acc.add(accel, i < 290 ? still : moved);

  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(ead::kCalibMoved, acc.finish(&out));
}

static void test_a_short_window_is_rejected_without_a_verdict() {
  ead::CalibrationAccumulator acc;
  acc.reset();
  const float accel[3] = {0.0f, 0.0f, 1.0f};
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  feed(&acc, accel, gyro, ead::kCalibMinSamples - 1);

  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(ead::kCalibTooFewSamples, acc.finish(&out));
  // The record must not look usable: identity alignment, zero bias.
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, 1.0f, out.alignment[0]);
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, 0.0f, out.gyroBiasDps[0]);
}

static void test_wrong_magnitude_and_upside_down_are_rejected() {
  // 1.3 g: either still moving, or the scale factor is wrong.
  ead::CalibrationAccumulator heavy;
  heavy.reset();
  const float strong[3] = {0.0f, 0.0f, 1.3f};
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  feed(&heavy, strong, gyro, 300);
  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(ead::kCalibNotGravity, heavy.finish(&out));

  ead::CalibrationAccumulator inverted;
  inverted.reset();
  const float upsideDown[3] = {0.0f, 0.0f, -1.0f};
  feed(&inverted, upsideDown, gyro, 300);
  TEST_ASSERT_EQUAL_UINT16(ead::kCalibUpsideDown, inverted.finish(&out));
}

static void test_a_sensor_on_its_side_is_rejected_as_not_up() {
  ead::CalibrationAccumulator acc;
  acc.reset();
  const float sideways[3] = {0.0f, 0.94f, 0.34f};
  const float gyro[3] = {0.0f, 0.0f, 0.0f};
  feed(&acc, sideways, gyro, 300);
  ead::CalibrationSensor out;
  TEST_ASSERT_EQUAL_UINT16(ead::kCalibUpsideDown, acc.finish(&out));
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_bias_is_the_mean_rate_while_still);
  RUN_TEST(test_alignment_rotates_measured_gravity_onto_plus_z);
  RUN_TEST(test_alignment_is_identity_when_already_upright);
  RUN_TEST(test_movement_during_the_window_is_rejected);
  RUN_TEST(test_a_short_window_is_rejected_without_a_verdict);
  RUN_TEST(test_wrong_magnitude_and_upside_down_are_rejected);
  RUN_TEST(test_a_sensor_on_its_side_is_rejected_as_not_up);
  return UNITY_END();
}
