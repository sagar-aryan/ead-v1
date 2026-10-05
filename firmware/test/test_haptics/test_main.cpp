#include <unity.h>

#include "ead/haptics.h"
#include "ead/reference.h"

using ead::ErrorClass;
using ead::HapticCue;
using ead::HapticEngine;
using ead::HapticEvent;
using ead::HapticReason;

void setUp() {}
void tearDown() {}

namespace {

ead::GaitCycle validCycle() {
  ead::GaitCycle c{};
  c.valid = true;
  c.startFrame = 100;
  c.endUs = 2'000'000;
  return c;
}

ead::ErrorResult scored(ErrorClass primary, float score, float confidence) {
  ead::ErrorResult r{};
  r.primaryClass = primary;
  r.activeClasses = ead::errorClassBit(primary);
  r.score = score;
  r.confidence = confidence;
  return r;
}

}  // namespace

static void test_an_episode_needs_the_on_threshold_and_full_confidence() {
  HapticEngine engine;
  HapticCue cue;
  const auto cycle = validCycle();
  // Doc 06 §11 and §5: below 0.35, or below 0.75 confidence, nothing starts.
  TEST_ASSERT_FALSE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.34f, 1.0f), &cue));
  TEST_ASSERT_FALSE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.9f, 0.74f), &cue));
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.35f, 0.75f), &cue));
  TEST_ASSERT_EQUAL(HapticEvent::On, cue.event);
  TEST_ASSERT_EQUAL_UINT8(4, cue.motor[0]);  // posterior cue (doc 06 §8)
  TEST_ASSERT_EQUAL_UINT8(0, cue.motor[1]);
  TEST_ASSERT_EQUAL_UINT16(250, cue.durationMs);
  TEST_ASSERT_EQUAL_UINT32(100, cue.cycleStartFrame);
  TEST_ASSERT_EQUAL_UINT64(2'000'000, cue.timeUs);
}

static void test_the_episode_holds_down_to_the_off_threshold() {
  HapticEngine engine;
  HapticCue cue;
  const auto cycle = validCycle();
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::ExcessPlantarflexion, 0.5f, 0.9f), &cue));
  // Between the thresholds, and at reduced confidence, it carries on.
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::ExcessPlantarflexion, 0.30f, 0.6f), &cue));
  TEST_ASSERT_EQUAL(HapticEvent::Update, cue.event);
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::ExcessPlantarflexion, 0.24f, 0.9f), &cue));
  TEST_ASSERT_EQUAL(HapticEvent::Off, cue.event);
  TEST_ASSERT_EQUAL(HapticReason::BelowThreshold, cue.reason);
  TEST_ASSERT_EQUAL_UINT8(0, cue.duty[0]);
  TEST_ASSERT_FALSE(engine.active());
  // A new episode must reach the on threshold again (§11).
  TEST_ASSERT_FALSE(engine.onCycle(cycle, scored(ErrorClass::ExcessPlantarflexion, 0.30f, 0.9f), &cue));
}

static void test_low_confidence_or_an_invalid_step_ends_it() {
  HapticEngine engine;
  HapticCue cue;
  auto cycle = validCycle();
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.6f, 0.9f), &cue));
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.6f, 0.49f), &cue));
  TEST_ASSERT_EQUAL(HapticReason::LowConfidence, cue.reason);

  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.6f, 0.9f), &cue));
  cycle.valid = false;
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.6f, 0.9f), &cue));
  TEST_ASSERT_EQUAL(HapticEvent::Off, cue.event);
  TEST_ASSERT_EQUAL(HapticReason::InvalidStep, cue.reason);
  TEST_ASSERT_EQUAL(ErrorClass::InsufficientDorsiflexion, cue.errorClass);
}

static void test_medial_and_lateral_cues_use_the_two_motors_either_side() {
  HapticEngine engine;
  HapticCue cue;
  TEST_ASSERT_TRUE(engine.onCycle(validCycle(), scored(ErrorClass::InversionDeviation, 1.0f, 1.0f), &cue));
  // 270 degrees: halfway between M5 (240) and M6 (300), equal duty.
  TEST_ASSERT_TRUE((cue.motor[0] == 5 && cue.motor[1] == 6) || (cue.motor[0] == 6 && cue.motor[1] == 5));
  TEST_ASSERT_EQUAL_UINT8(204, cue.duty[0]);
  TEST_ASSERT_EQUAL_UINT8(204, cue.duty[1]);

  HapticEngine lateral;
  TEST_ASSERT_TRUE(lateral.onCycle(validCycle(), scored(ErrorClass::EversionDeviation, 1.0f, 1.0f), &cue));
  TEST_ASSERT_TRUE((cue.motor[0] == 2 && cue.motor[1] == 3) || (cue.motor[0] == 3 && cue.motor[1] == 2));
}

static void test_overall_points_between_the_active_cues() {
  // Too little dorsiflexion (posterior, 180) and inversion (medial, 270), the
  // inversion three times as large: the sum points at about 252 degrees, nearest
  // M5 (240), with M6 (300) taking the smaller share.
  HapticEngine engine;
  HapticCue cue;
  ead::ErrorResult r = scored(ErrorClass::OverallDeviation, 1.0f, 1.0f);
  r.activeClasses = ead::errorClassBit(ErrorClass::InsufficientDorsiflexion) |
                    ead::errorClassBit(ErrorClass::InversionDeviation) |
                    ead::errorClassBit(ErrorClass::OverallDeviation);
  r.deviations[size_t(ead::GaitFeature::SwingDorsiflexion)] = 0.3f;
  r.deviations[size_t(ead::GaitFeature::Inversion)] = 0.9f;
  TEST_ASSERT_TRUE(engine.onCycle(validCycle(), r, &cue));
  TEST_ASSERT_EQUAL_UINT8(5, cue.motor[0]);
  TEST_ASSERT_EQUAL_UINT8(204, cue.duty[0]);
  // 251.6 degrees: 11.6 from M5, 48.4 from M6, so M6 runs at 11.6 / 48.4 of M5's duty,
  // 49, under the 51 minimum: dropped.
  TEST_ASSERT_EQUAL_UINT8(0, cue.motor[1]);
}

static void test_timing_alternates_front_and_back() {
  HapticEngine engine;
  HapticCue cue;
  const auto cycle = validCycle();
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::TimingDeviation, 0.8f, 0.9f), &cue));
  TEST_ASSERT_EQUAL_UINT8(1, cue.motor[0]);
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::TimingDeviation, 0.8f, 0.9f), &cue));
  TEST_ASSERT_EQUAL_UINT8(4, cue.motor[0]);
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::TimingDeviation, 0.8f, 0.9f), &cue));
  TEST_ASSERT_EQUAL_UINT8(1, cue.motor[0]);
}

static void test_intensity_follows_score_and_confidence() {
  // Doc 06 §10: 51 + 153 * score^1.5 * confidence, clamped to 51..204.
  HapticEngine engine;
  HapticCue cue;
  TEST_ASSERT_TRUE(engine.onCycle(validCycle(), scored(ErrorClass::InsufficientDorsiflexion, 0.35f, 0.75f), &cue));
  TEST_ASSERT_EQUAL_UINT8(75, cue.duty[0]);  // 51 + 153 * 0.2071 * 0.75 = 74.8
  TEST_ASSERT_TRUE(engine.onCycle(validCycle(), scored(ErrorClass::InsufficientDorsiflexion, 1.0f, 1.0f), &cue));
  TEST_ASSERT_EQUAL_UINT8(204, cue.duty[0]);
}

static void test_a_score_with_no_class_has_no_direction() {
  // A large score from distance or shank dynamics alone names no class, so it
  // has nowhere to point: no episode starts, and a running one ends.
  HapticEngine engine;
  HapticCue cue;
  const auto cycle = validCycle();
  TEST_ASSERT_FALSE(engine.onCycle(cycle, scored(ErrorClass::None, 0.9f, 1.0f), &cue));
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::InsufficientDorsiflexion, 0.9f, 1.0f), &cue));
  TEST_ASSERT_TRUE(engine.onCycle(cycle, scored(ErrorClass::None, 0.9f, 1.0f), &cue));
  TEST_ASSERT_EQUAL(HapticReason::NoDirection, cue.reason);
}

static void test_stopping_from_outside_closes_only_a_running_episode() {
  HapticEngine engine;
  HapticCue cue;
  TEST_ASSERT_FALSE(engine.stop(HapticReason::SwitchedOff, 5, &cue));
  TEST_ASSERT_TRUE(engine.onCycle(validCycle(), scored(ErrorClass::EversionDeviation, 0.6f, 0.9f), &cue));
  TEST_ASSERT_TRUE(engine.stop(HapticReason::SwitchedOff, 7, &cue));
  TEST_ASSERT_EQUAL(HapticEvent::Off, cue.event);
  TEST_ASSERT_EQUAL(HapticReason::SwitchedOff, cue.reason);
  TEST_ASSERT_EQUAL(ErrorClass::EversionDeviation, cue.errorClass);
  TEST_ASSERT_EQUAL_UINT64(7, cue.timeUs);
  TEST_ASSERT_FALSE(engine.active());
}

static void test_the_built_cue_is_full_strength_whatever_the_score() {
  // DEC-025, the user: every cue at 100 % for 500 ms. With the minimum and maximum
  // duty both 255, doc 06 §10's formula gives 255 for any score; a second motor
  // runs only at the same full share, so OVERALL picks the nearest one.
  ead::HapticConfig config;
  config.minDuty = 255;
  config.maxDuty = 255;
  config.cueMs = 500;
  HapticEngine engine(config);
  HapticCue cue;
  TEST_ASSERT_TRUE(engine.onCycle(validCycle(), scored(ErrorClass::InsufficientDorsiflexion, 0.36f, 0.75f), &cue));
  TEST_ASSERT_EQUAL_UINT8(255, cue.duty[0]);
  TEST_ASSERT_EQUAL_UINT16(500, cue.durationMs);
  HapticEngine lateral(config);
  TEST_ASSERT_TRUE(lateral.onCycle(validCycle(), scored(ErrorClass::EversionDeviation, 0.4f, 0.9f), &cue));
  TEST_ASSERT_EQUAL_UINT8(255, cue.duty[0]);
  TEST_ASSERT_EQUAL_UINT8(255, cue.duty[1]);  // halfway between M2 and M3: both, full
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_an_episode_needs_the_on_threshold_and_full_confidence);
  RUN_TEST(test_the_episode_holds_down_to_the_off_threshold);
  RUN_TEST(test_low_confidence_or_an_invalid_step_ends_it);
  RUN_TEST(test_medial_and_lateral_cues_use_the_two_motors_either_side);
  RUN_TEST(test_overall_points_between_the_active_cues);
  RUN_TEST(test_timing_alternates_front_and_back);
  RUN_TEST(test_intensity_follows_score_and_confidence);
  RUN_TEST(test_a_score_with_no_class_has_no_direction);
  RUN_TEST(test_stopping_from_outside_closes_only_a_running_episode);
  RUN_TEST(test_the_built_cue_is_full_strength_whatever_the_score);
  return UNITY_END();
}
