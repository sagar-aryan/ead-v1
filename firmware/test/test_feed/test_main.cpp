// The schema-6 feed (DEC-021): mount-map quaternions, segment orientation from
// a rotation vector, and accelerometer interpolation to the frame time.
#include <cmath>
#include <initializer_list>

#include <unity.h>

#include "config_v1.h"
#include "ead/calibration.h"
#include "ead/feed.h"
#include "ead/mahony.h"

void setUp() {}
void tearDown() {}

namespace {

void assertVector(const float expected[3], const float actual[3]) {
  for (int i = 0; i < 3; ++i) TEST_ASSERT_FLOAT_WITHIN(1e-5f, expected[i], actual[i]);
}

}  // namespace

static void test_the_mount_quaternion_rotates_like_the_map() {
  // The measured BNO086 maps (TEST-051) and a 180-degree one, which has a
  // negative trace and takes the other branches.
  const EadMountMap flip = {{{-1, 0, 0}, {0, -1, 0}, {0, 0, 1}}};
  for (const EadMountMap& map : {kEadFootMount, kEadShankMount, flip}) {
    float q[4];
    ead::matrixToQuaternion(map.m, q);
    const float axes[3][3] = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}};
    for (const auto& v : axes) {
      float expected[3];
      for (int r = 0; r < 3; ++r) {
        expected[r] = map.m[r][0] * v[0] + map.m[r][1] * v[1] + map.m[r][2] * v[2];
      }
      float rotated[3];
      ead::rotateByQuaternion(q, v, rotated);
      assertVector(expected, rotated);
    }
  }
}

static void test_a_chip_held_as_its_segment_reads_identity() {
  // When the chip's attitude is exactly the mount map then the alignment, the
  // segment is level and facing world X: identity.
  float mount[4];
  ead::matrixToQuaternion(kEadShankMount.m, mount);
  const float alignment[4] = {0.9659258f, 0.2588190f, 0.0f, 0.0f};  // 30 deg about X
  float chip[4];
  ead::quaternionMultiply(alignment, mount, chip);
  float segment[4];
  ead::segmentOrientation(chip, mount, alignment, segment);
  TEST_ASSERT_FLOAT_WITHIN(1e-5f, 1.0f, std::fabs(segment[0]));
  for (int i = 1; i < 4; ++i) TEST_ASSERT_FLOAT_WITHIN(1e-5f, 0.0f, segment[i]);
}

static void test_segment_orientation_maps_gravity_like_the_measurements() {
  // A still sensor: its accelerometer, carried into the segment by mount map and
  // alignment, then into the world by the segment orientation, must read the
  // same as the chip accelerometer carried into the world by the chip's own
  // rotation vector.
  float mount[4];
  ead::matrixToQuaternion(kEadFootMount.m, mount);
  const float alignment[4] = {0.9238795f, 0.0f, 0.3826834f, 0.0f};  // 45 deg about Y
  float chip[4] = {0.8f, 0.2f, -0.4f, 0.4f};
  ead::quaternionNormalize(chip);
  const float accelChip[3] = {0.3f, -0.5f, 0.81f};
  float worldDirect[3];
  ead::rotateByQuaternion(chip, accelChip, worldDirect);

  float anatomical[3];
  ead::rotateByQuaternion(mount, accelChip, anatomical);
  float aligned[3];
  ead::rotateByQuaternion(alignment, anatomical, aligned);
  float segment[4];
  ead::segmentOrientation(chip, mount, alignment, segment);
  float worldViaSegment[3];
  ead::rotateByQuaternion(segment, aligned, worldViaSegment);
  assertVector(worldDirect, worldViaSegment);
}

static void test_a_rotation_vector_becomes_a_unit_quaternion() {
  const int16_t rv[4] = {16384, 0, 0, 0};
  float q[4];
  ead::rotationVectorToQuaternion(rv, q);
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, 1.0f, q[0]);
  const int16_t half[4] = {11585, 11585, 0, 0};  // 90 deg about X, Q14
  ead::rotationVectorToQuaternion(half, q);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.7071068f, q[0]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.7071068f, q[1]);
}

static void test_the_accelerometer_is_interpolated_to_the_frame_time() {
  const ead::AccelPoint points[] = {
      {1000, {0, 100, -100}}, {5000, {400, 300, 100}}, {9000, {800, 300, 100}}};
  int16_t out[3];
  // A quarter of the way from 1000 to 5000 us.
  TEST_ASSERT_TRUE(ead::interpolateAccel(points, 3, 2000, out));
  TEST_ASSERT_EQUAL_INT16(100, out[0]);
  TEST_ASSERT_EQUAL_INT16(150, out[1]);
  TEST_ASSERT_EQUAL_INT16(-50, out[2]);
  // Exactly on a sample.
  TEST_ASSERT_TRUE(ead::interpolateAccel(points, 3, 5000, out));
  TEST_ASSERT_EQUAL_INT16(400, out[0]);
  // After the last sample: held at the latest, and said so.
  TEST_ASSERT_FALSE(ead::interpolateAccel(points, 3, 9500, out));
  TEST_ASSERT_EQUAL_INT16(800, out[0]);
  // Before the first: the earliest.
  TEST_ASSERT_FALSE(ead::interpolateAccel(points, 3, 500, out));
  TEST_ASSERT_EQUAL_INT16(0, out[0]);
  TEST_ASSERT_FALSE(ead::interpolateAccel(points, 0, 500, out));
  TEST_ASSERT_EQUAL_INT16(0, out[2]);
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_the_mount_quaternion_rotates_like_the_map);
  RUN_TEST(test_a_chip_held_as_its_segment_reads_identity);
  RUN_TEST(test_segment_orientation_maps_gravity_like_the_measurements);
  RUN_TEST(test_a_rotation_vector_becomes_a_unit_quaternion);
  RUN_TEST(test_the_accelerometer_is_interpolated_to_the_frame_time);
  return UNITY_END();
}
