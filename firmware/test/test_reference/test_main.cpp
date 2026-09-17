#include <unity.h>

#include <cmath>

#include "ead/error_engine.h"
#include "ead/reference.h"

void setUp() {}
void tearDown() {}

namespace {

/// A cycle with the features the reference and the error engine read.
ead::GaitCycle cycleWith(float dorsiflexion, float contact, float inversion, float cycleTime,
                         float stanceRatio, float distance, float shankRate) {
  ead::GaitCycle c{};
  c.valid = true;
  c.peakDorsiflexionDeg = dorsiflexion;
  c.contactSagittalDeg = contact;
  c.peakInversionDeg = inversion;
  c.cycleTimeS = cycleTime;
  c.stanceRatio = stanceRatio;
  c.distanceM = distance;
  c.peakShankRateDps = shankRate;
  c.zuptQuality = 0.3f;
  return c;
}

/// A patient walking consistently: the numbers measured on the 6 m course
/// (TEST-030), varied by a degree or two the way real cycles vary.
ead::GaitCycle typical(int i) {
  const float wobble = float(i % 5) - 2.0f;  // -2 … +2
  return cycleWith(16.0f + wobble, -4.0f + wobble * 0.5f, 3.0f + wobble * 0.2f,
                   1.70f + wobble * 0.02f, 0.52f + wobble * 0.005f, 1.10f + wobble * 0.02f,
                   400.0f + wobble * 10.0f);
}

ead::ReferenceProfile buildTypicalReference(int cycles = 40) {
  ead::ReferenceBuilder builder;
  builder.reset();
  for (int i = 0; i < cycles; ++i) builder.add(typical(i));
  ead::ReferenceProfile profile{};
  TEST_ASSERT_TRUE(builder.build(&profile));
  return profile;
}

const ead::ConfidenceInputs kGoodInputs{1.0f, 1.0f};

}  // namespace

static void test_a_reference_needs_thirty_valid_cycles() {
  ead::ReferenceBuilder builder;
  builder.reset();
  for (int i = 0; i < 29; ++i) builder.add(typical(i));
  ead::ReferenceProfile profile{};
  TEST_ASSERT_FALSE_MESSAGE(builder.ready(), "29 cycles is not a reference");
  TEST_ASSERT_FALSE(builder.build(&profile));

  builder.add(typical(29));
  TEST_ASSERT_TRUE(builder.ready());
  TEST_ASSERT_TRUE(builder.build(&profile));
  TEST_ASSERT_EQUAL_UINT16(30, profile.cycles);
}

static void test_invalid_cycles_never_reach_the_reference() {
  ead::ReferenceBuilder builder;
  builder.reset();
  for (int i = 0; i < 40; ++i) {
    ead::GaitCycle c = typical(i);
    if (i % 2 == 0) {
      c.valid = false;
      c.peakDorsiflexionDeg = 90.0f;  // nonsense the detector already rejected
    }
    builder.add(c);
  }
  TEST_ASSERT_EQUAL_UINT16(20, builder.count());
  ead::ReferenceProfile profile{};
  TEST_ASSERT_FALSE_MESSAGE(builder.build(&profile), "20 valid cycles is still too few");
}

static void test_the_median_is_robust_to_a_stumble() {
  // Thirty-five ordinary cycles and five wild ones: a mean would move, the
  // median must not. This is why doc 06 §2 specifies median and MAD.
  ead::ReferenceBuilder builder;
  builder.reset();
  for (int i = 0; i < 35; ++i) builder.add(typical(i));
  for (int i = 0; i < 5; ++i) {
    builder.add(cycleWith(60.0f, -40.0f, 30.0f, 2.90f, 0.95f, 3.0f, 900.0f));
  }
  ead::ReferenceProfile profile{};
  TEST_ASSERT_TRUE(builder.build(&profile));
  const float dorsiflexion =
      profile.features[size_t(ead::GaitFeature::SwingDorsiflexion)].median;
  TEST_ASSERT_FLOAT_WITHIN_MESSAGE(2.0f, 16.0f, dorsiflexion,
                                   "five stumbles must not move the median");
}

static void test_a_flat_feature_gets_the_spread_floor() {
  ead::ReferenceBuilder builder;
  builder.reset();
  // Every cycle identical: MAD is zero, and dividing by it later would be
  // infinite deviation for the smallest difference.
  for (int i = 0; i < 30; ++i) builder.add(cycleWith(16, -4, 3, 1.7f, 0.52f, 1.1f, 400));
  ead::ReferenceProfile profile{};
  TEST_ASSERT_TRUE(builder.build(&profile));
  for (size_t f = 0; f < ead::kFeatureCount; ++f) {
    TEST_ASSERT_TRUE_MESSAGE(profile.features[f].spread >= ead::kFeatureSpreadFloor[f],
                             "spread must never be zero");
  }
}

static void test_a_cycle_like_the_reference_scores_near_zero() {
  const ead::ReferenceProfile profile = buildTypicalReference();
  const ead::ErrorResult result = ead::scoreCycle(typical(2), profile, kGoodInputs);
  TEST_ASSERT_TRUE_MESSAGE(result.score < 0.2f, "a typical cycle is not an error");
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::ErrorClass::None), uint8_t(result.primaryClass));
  TEST_ASSERT_EQUAL_UINT16(0, result.activeClasses);
}

static void test_too_little_dorsiflexion_is_named() {
  const ead::ReferenceProfile profile = buildTypicalReference();
  // Half the usual dorsiflexion, everything else ordinary: the classic
  // foot-drop presentation this device exists to measure.
  ead::GaitCycle c = typical(2);
  c.peakDorsiflexionDeg = 2.0f;
  const ead::ErrorResult result = ead::scoreCycle(c, profile, kGoodInputs);
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::ErrorClass::InsufficientDorsiflexion),
                          uint8_t(result.primaryClass));
  TEST_ASSERT_TRUE(result.activeClasses &
                   ead::errorClassBit(ead::ErrorClass::InsufficientDorsiflexion));
  TEST_ASSERT_TRUE(result.score > 0.2f);
  // More dorsiflexion than the reference is not an error in V1.
  c.peakDorsiflexionDeg = 30.0f;
  const ead::ErrorResult more = ead::scoreCycle(c, profile, kGoodInputs);
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::ErrorClass::None), uint8_t(more.primaryClass));
  TEST_ASSERT_TRUE_MESSAGE(more.score > 0.2f, "it still counts toward the score");
}

static void test_inversion_and_eversion_are_opposite_classes() {
  const ead::ReferenceProfile profile = buildTypicalReference();
  ead::GaitCycle rolled = typical(2);
  rolled.peakInversionDeg = 20.0f;
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::ErrorClass::InversionDeviation),
                          uint8_t(ead::scoreCycle(rolled, profile, kGoodInputs).primaryClass));
  rolled.peakInversionDeg = -15.0f;
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::ErrorClass::EversionDeviation),
                          uint8_t(ead::scoreCycle(rolled, profile, kGoodInputs).primaryClass));
}

static void test_the_score_is_the_weighted_mean_of_deviations() {
  const ead::ReferenceProfile profile = buildTypicalReference();
  // Push one feature to the maximum deviation and leave the rest at the median.
  ead::GaitCycle c = cycleWith(profile.features[0].median, profile.features[1].median,
                               profile.features[2].median, profile.features[3].median,
                               profile.features[4].median, profile.features[5].median,
                               profile.features[6].median);
  c.peakDorsiflexionDeg = profile.features[0].median - 100.0f * profile.features[0].spread;
  const ead::ErrorResult result = ead::scoreCycle(c, profile, kGoodInputs);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.0f, result.deviations[0]);
  // Weight 0.25 of a total 1.00, everything else at zero deviation.
  TEST_ASSERT_FLOAT_WITHIN_MESSAGE(1e-4f, 0.25f, result.score,
                                   "one feature at full deviation carries its weight");
}

static void test_an_unmeasurable_feature_leaves_the_denominator() {
  const ead::ReferenceProfile profile = buildTypicalReference();
  ead::GaitCycle c = typical(2);
  c.zuptQuality = 0.0f;  // no zero-velocity window: distance means nothing
  const ead::ErrorResult result = ead::scoreCycle(c, profile, kGoodInputs);
  TEST_ASSERT_FALSE(result.active[size_t(ead::GaitFeature::CycleDistance)]);
  // Feature completeness falls, and with it confidence — though one missing
  // feature out of seven is not on its own enough to make a cycle unusable.
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 6.0f / 7.0f, result.subscores[2]);
  const ead::ErrorResult complete = ead::scoreCycle(typical(2), profile, kGoodInputs);
  TEST_ASSERT_TRUE_MESSAGE(result.confidence < complete.confidence,
                           "a feature that could not be measured must cost confidence");
}

static void test_confidence_weights_and_gates() {
  const ead::ReferenceProfile profile = buildTypicalReference(60);
  ead::GaitCycle c = typical(2);
  c.zuptQuality = 1.0f;
  const ead::ErrorResult best = ead::scoreCycle(c, profile, {1.0f, 1.0f});
  TEST_ASSERT_FLOAT_WITHIN_MESSAGE(1e-4f, 1.0f, best.confidence,
                                   "every subscore at 1 gives confidence 1");
  TEST_ASSERT_TRUE(best.confidence >= ead::kConfidenceForFeedback);

  // A cycle with failed sensor reads: doc 06 §5 puts 0.30 on sensor quality.
  const ead::ErrorResult poor = ead::scoreCycle(c, profile, {0.0f, 1.0f});
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.70f, poor.confidence);
  const ead::ErrorResult worse = ead::scoreCycle(c, profile, {0.0f, 0.0f});
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 0.45f, worse.confidence);
  TEST_ASSERT_TRUE_MESSAGE(worse.confidence < ead::kConfidenceForDisplay,
                           "below 0.50 the cycle is not worth displaying as a measurement");
}

static void test_several_deviations_with_no_leader_are_overall() {
  const ead::ReferenceProfile profile = buildTypicalReference();
  ead::GaitCycle c = typical(2);
  // Dorsiflexion (0.25) and inversion (0.15) both far out, scaled so their
  // weighted contributions are within a tenth of each other.
  // Deviations 0.6 and 1.0, weights 0.25 and 0.15: contributions 0.150 each.
  c.peakDorsiflexionDeg = profile.features[0].median - 1.8f * profile.features[0].spread;
  c.peakInversionDeg = profile.features[2].median + 3.0f * profile.features[2].spread;
  const ead::ErrorResult result = ead::scoreCycle(c, profile, kGoodInputs);
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::ErrorClass::OverallDeviation),
                          uint8_t(result.primaryClass));
  // The specific classes stay in the log (doc 06 §4).
  TEST_ASSERT_TRUE(result.activeClasses &
                   ead::errorClassBit(ead::ErrorClass::InsufficientDorsiflexion));
  TEST_ASSERT_TRUE(result.activeClasses &
                   ead::errorClassBit(ead::ErrorClass::InversionDeviation));
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_a_reference_needs_thirty_valid_cycles);
  RUN_TEST(test_invalid_cycles_never_reach_the_reference);
  RUN_TEST(test_the_median_is_robust_to_a_stumble);
  RUN_TEST(test_a_flat_feature_gets_the_spread_floor);
  RUN_TEST(test_a_cycle_like_the_reference_scores_near_zero);
  RUN_TEST(test_too_little_dorsiflexion_is_named);
  RUN_TEST(test_inversion_and_eversion_are_opposite_classes);
  RUN_TEST(test_the_score_is_the_weighted_mean_of_deviations);
  RUN_TEST(test_an_unmeasurable_feature_leaves_the_denominator);
  RUN_TEST(test_confidence_weights_and_gates);
  RUN_TEST(test_several_deviations_with_no_leader_are_overall);
  return UNITY_END();
}
