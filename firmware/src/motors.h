#pragma once
// Motor outputs: service-test pulses (DEC-018) and feedback cues (DEC-023), each
// timed here on the device and held to the contract's limits by one guard.

#include "ead/haptics.h"
#include "ead/motor_guard.h"
#include "ead/protocol.h"

namespace motors {

/// PWM on every enabled motor pin at the contract's frequency and resolution,
/// duty 0. False if the PWM peripheral could not be set up; pulses are then
/// refused.
bool begin();

/// Starts a pulse, or says why not. It ends by itself after its duration.
ead::MotorGuard::Refusal pulse(const ead::MotorPulse& request);

/// Starts a feedback cue on its one or two motors, or says why not. It ends by
/// itself after its duration.
ead::MotorGuard::Refusal cue(const ead::HapticCue& cue);

/// Every motor off now (doc 06 §12). Safe from any task.
void stopAll();

}  // namespace motors
