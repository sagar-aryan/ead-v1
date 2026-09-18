#include <unity.h>

#include <cmath>
#include <vector>

#include "ead/gait.h"

void setUp() {}
void tearDown() {}

namespace {

constexpr float kHz = 100.0f;
constexpr uint32_t kDtUs = 10000;
const float kIdentity[4] = {1.0f, 0.0f, 0.0f, 0.0f};

/// Feeds samples and collects what the engine produces.
struct Run {
  ead::GaitEngine engine;
  std::vector<ead::GaitEvent> events;
  std::vector<ead::GaitCycle> cycles;
  uint64_t timeUs = 1'000'000;
  uint32_t frame = 0;

  void push(const float accel[3], const float footGyro[3], float shankRate,
            const float relative[4] = kIdentity) {
    ead::GaitSample s{};
    s.timeUs = timeUs;
    s.frameIndex = frame;
    for (int i = 0; i < 3; ++i) {
      s.footAccelG[i] = accel[i];
      s.footGyroDps[i] = footGyro[i];
      s.shankGyroDps[i] = 0.0f;
    }
    s.shankGyroDps[1] = shankRate;
    for (int i = 0; i < 4; ++i) {
      s.footQuaternion[i] = kIdentity[i];
      s.relativeQuaternion[i] = relative[i];
    }
    engine.update(s);
    ead::GaitEvent e;
    while (engine.takeEvent(&e)) events.push_back(e);
    ead::GaitCycle c;
    while (engine.takeCycle(&c)) cycles.push_back(c);
    timeUs += kDtUs;
    ++frame;
  }

  void still(float seconds) {
    const float accel[3] = {0.0f, 0.0f, 1.0f};
    const float gyro[3] = {0.0f, 0.0f, 0.0f};
    for (int i = 0; i < int(seconds * kHz); ++i) push(accel, gyro, 0.0f);
  }

  /// Swing: the foot turns fast and the acceleration wanders, but never enough
  /// to look like an impact.
  void swing(float seconds) {
    const int steps = int(seconds * kHz);
    for (int i = 0; i < steps; ++i) {
      const float phase = float(i) / float(steps);
      const float accel[3] = {0.25f * std::sin(phase * 6.28f), 0.0f, 1.0f};
      const float gyro[3] = {0.0f, 200.0f * std::sin(phase * 3.14f), 0.0f};
      push(accel, gyro, 250.0f * std::sin(phase * 3.14f));
    }
  }

  /// Swing with rotation but no forward acceleration: the foot turns in place.
  void swingInPlace(float seconds) {
    const int steps = int(seconds * kHz);
    for (int i = 0; i < steps; ++i) {
      const float phase = float(i) / float(steps);
      const float accel[3] = {0.0f, 0.0f, 1.0f};
      const float gyro[3] = {0.0f, 200.0f * std::sin(phase * 3.14f), 0.0f};
      push(accel, gyro, 200.0f * std::sin(phase * 3.14f));
    }
  }

  /// The impact feature at initial contact: three frames of a hard spike.
  void contact() {
    const float gyro[3] = {0.0f, 60.0f, 0.0f};
    for (int i = 0; i < 3; ++i) {
      const float accel[3] = {0.0f, 0.0f, 1.0f + (i == 1 ? 1.4f : 0.7f)};
      push(accel, gyro, 80.0f);
    }
  }

  int count(ead::GaitEventType type) const {
    int n = 0;
    for (const auto& e : events) {
      if (e.type == type) ++n;
    }
    return n;
  }
};

/// One second per cycle: 0.6 s of stance beginning at contact, 0.4 s of swing.
void walk(Run* run, int cycles) {
  for (int i = 0; i < cycles; ++i) {
    run->contact();
    run->still(0.57f);
    run->swing(0.40f);
  }
}

}  // namespace

static void test_a_still_foot_produces_a_zupt_and_no_cycles() {
  Run run;
  run.still(2.0f);
  TEST_ASSERT_EQUAL_INT(1, run.count(ead::GaitEventType::ZuptStart));
  TEST_ASSERT_EQUAL_INT(0, run.count(ead::GaitEventType::InitialContact));
  TEST_ASSERT_EQUAL_size_t(0, run.cycles.size());
  TEST_ASSERT_TRUE(run.engine.inZupt());
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::GaitState::FootFlatZv), uint8_t(run.engine.state()));
}

static void test_a_walk_produces_one_cycle_per_stride() {
  Run run;
  run.still(1.0f);  // stand before walking
  walk(&run, 5);
  // The first contact arrives from standing, not from swing, so it is not an
  // initial contact: four of the five are. Each detected contact opens a cycle
  // and closes the one before, so three cycles complete.
  TEST_ASSERT_EQUAL_INT(4, run.count(ead::GaitEventType::InitialContact));
  TEST_ASSERT_EQUAL_INT(5, run.count(ead::GaitEventType::ToeOff));
  TEST_ASSERT_TRUE(run.cycles.size() >= 3);
}

static void test_cycle_timing_stance_ratio_and_cadence() {
  Run run;
  run.still(1.0f);
  walk(&run, 5);
  TEST_ASSERT_TRUE(run.cycles.size() >= 3);
  for (const auto& c : run.cycles) {
    TEST_ASSERT_TRUE_MESSAGE(c.valid, "a one-second cycle must pass the temporal guards");
    TEST_ASSERT_FLOAT_WITHIN(0.03f, 1.0f, c.cycleTimeS);
    // Stance runs from contact to toe-off. Toe-off needs 40 ms of sustained
    // rotation before it is accepted, so stance measures a little long and
    // swing a little short against the 0.60/0.40 the generator produces.
    TEST_ASSERT_FLOAT_WITHIN(0.06f, 0.63f, c.stanceRatio);
    TEST_ASSERT_FLOAT_WITHIN(0.06f, 0.37f, c.swingRatio);
    TEST_ASSERT_FLOAT_WITHIN(0.02f, c.cycleTimeS, c.stanceTimeS + c.swingTimeS);
    // Doc 05 §9: 120 / cycle time.
    TEST_ASSERT_FLOAT_WITHIN(4.0f, 120.0f, c.cadenceStepsPerMin);
    TEST_ASSERT_FLOAT_WITHIN(30.0f, 250.0f, c.peakShankRateDps);
    // Most of stance is still, so ZUPT covers more than half the cycle.
    TEST_ASSERT_TRUE_MESSAGE(c.zuptQuality > 0.4f, "stance should be mostly zero-velocity");
  }
}

static void test_a_second_impact_soon_after_is_the_same_footfall() {
  // A real footfall gives several impact features — heel, then forefoot. Each
  // one used to open a cycle: 20 contacts for 12 steps on the first walk
  // (TEST-030). Contacts closer together than the shortest legal cycle are one.
  Run run;
  run.still(1.0f);
  run.contact();
  run.still(0.50f);
  run.swing(0.40f);
  run.contact();            // the footfall
  run.still(0.08f);
  run.contact();            // the forefoot, 0.11 s later
  run.still(0.30f);
  TEST_ASSERT_EQUAL_INT_MESSAGE(1, run.count(ead::GaitEventType::InitialContact),
                                "one footfall is one initial contact");
}

static void test_a_cycle_longer_than_the_guard_is_marked_invalid() {
  // Standing between two footfalls: over the 3.00 s maximum (doc 05 §4).
  Run run;
  run.still(1.0f);
  run.contact();
  run.still(0.50f);
  run.swing(0.40f);
  run.contact();            // opens the cycle
  run.still(3.20f);         // a long pause
  run.swing(0.40f);
  run.contact();            // closes it
  run.still(0.20f);
  TEST_ASSERT_TRUE(run.cycles.size() >= 1);
  const ead::GaitCycle& slow = run.cycles.back();
  TEST_ASSERT_TRUE(slow.cycleTimeS > ead::kMaxCycleS);
  TEST_ASSERT_FALSE_MESSAGE(slow.valid, "a cycle over 3 s must not be valid");
}

static void test_the_push_off_spike_is_not_a_contact() {
  // Measured on the leg (TEST-030): every stride was being split in two because
  // the push-off acceleration early in swing was read as the next footfall.
  Run run;
  run.still(1.0f);
  run.contact();
  run.still(0.50f);
  // Toe-off, then a hard spike 80 ms into swing while the foot is speeding up.
  for (int i = 0; i < 8; ++i) {
    const float accel[3] = {0.0f, 0.0f, 1.0f};
    const float gyro[3] = {0.0f, 120.0f + 20.0f * float(i), 0.0f};
    run.push(accel, gyro, 150.0f);
  }
  for (int i = 0; i < 3; ++i) {
    const float accel[3] = {0.0f, 0.0f, 2.1f};  // push-off
    const float gyro[3] = {0.0f, 280.0f, 0.0f};
    run.push(accel, gyro, 300.0f);
  }
  run.swing(0.30f);
  run.contact();
  run.still(0.30f);
  TEST_ASSERT_EQUAL_INT_MESSAGE(1, run.count(ead::GaitEventType::InitialContact),
                                "push-off must not be counted as a footfall");
}

static void test_the_foot_slapping_flat_is_not_a_toe_off() {
  // Measured on the leg (PROB-016): right after a heel strike the foot rotates
  // flat as fast as it lifts off. Taken for toe-off, it put the engine in swing
  // during stance, and the push-off impact then passed as the next contact.
  Run run;
  run.still(1.0f);
  run.swing(0.40f);  // contacts are only accepted out of a swing
  run.contact();
  for (int i = 0; i < 15; ++i) {  // 150 ms of foot slap, fast plantarflexion
    const float accel[3] = {0.2f, 0.0f, 1.3f};
    const float gyro[3] = {0.0f, -220.0f, 0.0f};
    run.push(accel, gyro, 60.0f);
  }
  run.still(0.45f);
  for (int i = 0; i < 3; ++i) {  // push-off: a hard spike as the heel rises
    const float accel[3] = {0.0f, 0.0f, 2.3f};
    const float gyro[3] = {0.0f, 260.0f, 0.0f};
    run.push(accel, gyro, 250.0f);
  }
  run.swing(0.35f);
  run.contact();
  run.still(0.30f);
  // Two heel strikes and nothing between them. Without the stance floor the
  // slap becomes a toe-off 160 ms after contact and the push-off a third
  // contact. Toe-offs: the warm-up swing's, and push-off's — not the slap's.
  TEST_ASSERT_EQUAL_INT_MESSAGE(2, run.count(ead::GaitEventType::InitialContact),
                                "one stride: the heel strike, and the next one");
  TEST_ASSERT_EQUAL_INT_MESSAGE(2, run.count(ead::GaitEventType::ToeOff),
                                "a toe-off at push-off, not at the slap");
}

static void test_distance_matches_a_known_motion() {
  // A cycle whose swing is a measured out-and-back acceleration: 3 m/s² for
  // 0.2 s then -3 m/s² for 0.2 s travels 0.12 m and ends at rest.
  Run run;
  run.still(1.0f);
  run.contact();       // from standing: opens nothing
  run.still(0.50f);
  run.swing(0.40f);
  run.contact();       // this one opens the cycle being measured
  run.still(0.57f);
  const float g = 9.80665f;
  for (int phase = 0; phase < 2; ++phase) {
    const float a = phase == 0 ? 3.0f : -3.0f;
    for (int i = 0; i < 20; ++i) {
      const float accel[3] = {a / g, 0.0f, 1.0f};
      const float gyro[3] = {0.0f, 150.0f, 0.0f};
      run.push(accel, gyro, 150.0f);
    }
  }
  run.contact();
  run.still(0.20f);
  TEST_ASSERT_TRUE(run.cycles.size() >= 1);
  const ead::GaitCycle& c = run.cycles.back();
  // Within 15 %: the integrator sees the same profile the test prescribes, so
  // the error here is discretisation, not sensing.
  TEST_ASSERT_FLOAT_WITHIN(0.018f, 0.12f, c.distanceM);
  TEST_ASSERT_TRUE(c.speedMps > 0.0f);
}

static void test_a_foot_that_turns_without_translating_travels_nowhere() {
  // The distance must come from acceleration, not from rotation: a cycle whose
  // swing has no forward acceleration has to measure near zero, or every
  // shuffle in place would be reported as walking.
  Run run;
  run.still(1.0f);
  run.contact();
  run.still(0.50f);
  run.swingInPlace(0.40f);
  run.contact();       // opens the cycle
  run.still(0.60f);
  run.swingInPlace(0.35f);
  run.contact();       // closes it
  run.still(0.20f);
  TEST_ASSERT_TRUE(run.cycles.size() >= 1);
  TEST_ASSERT_TRUE_MESSAGE(run.cycles.back().distanceM < 0.05f,
                           "rotation alone must not be reported as distance");
}

static void test_a_timing_gap_restarts_the_engine() {
  Run run;
  run.still(1.0f);
  run.timeUs += 500000;  // half a second lost
  run.still(0.02f);      // too little to re-establish a zero-velocity window
  TEST_ASSERT_EQUAL_UINT8(uint8_t(ead::GaitState::Init), uint8_t(run.engine.state()));
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_a_still_foot_produces_a_zupt_and_no_cycles);
  RUN_TEST(test_a_walk_produces_one_cycle_per_stride);
  RUN_TEST(test_cycle_timing_stance_ratio_and_cadence);
  RUN_TEST(test_a_second_impact_soon_after_is_the_same_footfall);
  RUN_TEST(test_a_cycle_longer_than_the_guard_is_marked_invalid);
  RUN_TEST(test_the_push_off_spike_is_not_a_contact);
  RUN_TEST(test_the_foot_slapping_flat_is_not_a_toe_off);
  RUN_TEST(test_distance_matches_a_known_motion);
  RUN_TEST(test_a_foot_that_turns_without_translating_travels_nowhere);
  RUN_TEST(test_a_timing_gap_restarts_the_engine);
  return UNITY_END();
}
