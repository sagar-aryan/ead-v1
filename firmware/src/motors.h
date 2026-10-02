#pragma once
// Motor outputs for the service test only (DEC-018): one pulse at a time, its
// length timed here on the device, within the contract's limits. No error-driven
// haptic feedback exists (DEC-006).

#include "ead/motor_guard.h"
#include "ead/protocol.h"

namespace motors {

/// PWM on every enabled motor pin at the contract's frequency and resolution,
/// duty 0. False if the PWM peripheral could not be set up; pulses are then
/// refused.
bool begin();

/// Starts a pulse, or says why not. It ends by itself after its duration.
ead::MotorGuard::Refusal pulse(const ead::MotorPulse& request);

}  // namespace motors
