#include <unity.h>

#include "config_v1.h"
#include "ead/motor_guard.h"

using ead::MotorGuard;
using Refusal = MotorGuard::Refusal;

void setUp() {}
void tearDown() {}

// The contract's haptic limits, with every motor enabled.
static MotorGuard contractGuard(uint8_t enabledMask = 0x3F) {
  return MotorGuard({EAD_HAPTIC_MIN_DUTY, EAD_HAPTIC_MAX_DUTY, EAD_HAPTIC_MAX_ON_S * 1000u,
                     EAD_HAPTIC_ROLL_WIN_S * 1000u, EAD_HAPTIC_ROLL_DUTY_LIM, enabledMask});
}

static void test_refuses_requests_outside_the_contract() {
  MotorGuard guard = contractGuard(0x3F & ~(1u << 2));  // motor 3 switched off
  TEST_ASSERT_EQUAL(Refusal::BadMotor, guard.request(0, 128, 1000, 0));
  TEST_ASSERT_EQUAL(Refusal::BadMotor, guard.request(7, 128, 1000, 0));
  TEST_ASSERT_EQUAL(Refusal::Disabled, guard.request(3, 128, 1000, 0));
  TEST_ASSERT_EQUAL(Refusal::BadDuty, guard.request(1, 50, 1000, 0));
  TEST_ASSERT_EQUAL(Refusal::BadDuty, guard.request(1, 205, 1000, 0));
  TEST_ASSERT_EQUAL(Refusal::BadDuration, guard.request(1, 128, 99, 0));
  TEST_ASSERT_EQUAL(Refusal::BadDuration, guard.request(1, 128, 5001, 0));
  TEST_ASSERT_EQUAL(0, guard.running(0));
}

static void test_one_motor_at_a_time_and_the_pulse_ends_on_its_own() {
  MotorGuard guard = contractGuard();
  TEST_ASSERT_EQUAL(Refusal::None, guard.request(1, 128, 1000, 0));
  TEST_ASSERT_EQUAL(1, guard.running(500));
  TEST_ASSERT_EQUAL(Refusal::Busy, guard.request(2, 128, 1000, 500));
  TEST_ASSERT_EQUAL(0, guard.running(1000));
  TEST_ASSERT_EQUAL(Refusal::None, guard.request(2, 204, 1000, 1000));
}

static void test_at_most_half_of_any_ten_seconds_on_per_motor() {
  MotorGuard guard = contractGuard();
  TEST_ASSERT_EQUAL(Refusal::None, guard.request(1, 128, 5000, 0));
  TEST_ASSERT_EQUAL(Refusal::RollingLimit, guard.request(1, 128, 100, 5000));
  TEST_ASSERT_EQUAL(Refusal::None, guard.request(2, 128, 100, 5000));  // per motor
  // Window 1000..11000 holds 4000 ms of the first pulse, plus 1000: exactly half.
  TEST_ASSERT_EQUAL(Refusal::None, guard.request(1, 128, 1000, 10000));
}

static void test_many_short_pulses_still_count() {
  // A hundred alternating 100 ms pulses fill ten seconds. Motor 1 then holds
  // 4900 ms of the window 200..10200, so 200 ms more breaks the limit; a history
  // shorter than a window's worth of pulses would have forgotten some of them.
  MotorGuard guard = contractGuard();
  for (uint32_t i = 0; i < 100; i++) {
    TEST_ASSERT_EQUAL(Refusal::None, guard.request(i % 2 == 0 ? 1 : 2, 128, 100, i * 100));
  }
  TEST_ASSERT_EQUAL(Refusal::RollingLimit, guard.request(1, 128, 200, 10000));
  TEST_ASSERT_EQUAL(Refusal::None, guard.request(1, 128, 100, 10000));
}

static void test_a_cue_runs_two_motors_together_within_each_limit() {
  MotorGuard guard = contractGuard(0x3F & ~(1u << 2));  // motor 3 switched off
  const uint8_t pair[2] = {5, 6};
  const uint8_t duties[2] = {204, 120};
  TEST_ASSERT_EQUAL(Refusal::None, guard.requestCue(pair, duties, 250, 0));
  TEST_ASSERT_EQUAL(Refusal::Busy, guard.requestCue(pair, duties, 250, 100));
  TEST_ASSERT_EQUAL(Refusal::Busy, guard.request(1, 128, 200, 100));  // a pulse waits too
  const uint8_t single[2] = {4, 0};
  const uint8_t singleDuty[2] = {80, 0};
  TEST_ASSERT_EQUAL(Refusal::None, guard.requestCue(single, singleDuty, 250, 250));
  // Either motor failing a limit refuses the cue.
  const uint8_t withOff[2] = {2, 3};
  TEST_ASSERT_EQUAL(Refusal::Disabled, guard.requestCue(withOff, duties, 250, 1000));
  const uint8_t weak[2] = {204, 50};
  TEST_ASSERT_EQUAL(Refusal::BadDuty, guard.requestCue(pair, weak, 250, 1000));
  const uint8_t same[2] = {5, 5};
  TEST_ASSERT_EQUAL(Refusal::BadMotor, guard.requestCue(same, duties, 250, 1000));
  // The rolling limit counts each motor of a cue: twenty cues from 0 to 9.5 s
  // put 5 s of motor 6 in the first 10 s, the most allowed, so one more ending
  // inside them is refused.
  MotorGuard rolling = contractGuard();
  for (uint32_t t = 0; t < 10000; t += 500) {
    TEST_ASSERT_EQUAL(Refusal::None, rolling.requestCue(pair, duties, 250, t));
  }
  const uint8_t six[2] = {6, 0};
  TEST_ASSERT_EQUAL(Refusal::RollingLimit, rolling.requestCue(six, duties, 250, 9750));
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_refuses_requests_outside_the_contract);
  RUN_TEST(test_one_motor_at_a_time_and_the_pulse_ends_on_its_own);
  RUN_TEST(test_at_most_half_of_any_ten_seconds_on_per_motor);
  RUN_TEST(test_many_short_pulses_still_count);
  RUN_TEST(test_a_cue_runs_two_motors_together_within_each_limit);
  return UNITY_END();
}
