#pragma once
// The contract's motor limits (doc 06 §6-§12, CONFIG_V1.json "haptics") applied
// to service-test pulses (doc 07 §7, DEC-018) and to feedback cues (DEC-023). Time is passed in, so the rules
// are tested on the host; the device turns an accepted pulse into PWM and ends it
// itself, whatever happens to the host link.

#include <cstddef>
#include <cstdint>

namespace ead {

class MotorGuard {
 public:
  enum class Refusal : uint8_t {
    None = 0,
    BadMotor,      // not 1..6
    Disabled,      // its channel is switched off in this build
    BadDuty,       // outside the contract's min..max duty
    BadDuration,   // shorter than kMinPulseMs, or longer than the maximum on-time
    Busy,          // another pulse or cue is running
    RollingLimit,  // would exceed the on-time allowed in the rolling window
  };

  struct Limits {
    uint8_t minDuty;
    uint8_t maxDuty;
    uint32_t maxOnMs;
    uint32_t windowMs;
    float windowOnFraction;
    uint8_t enabledMask;  // bit n-1 = motor n may run
  };

  explicit MotorGuard(const Limits& limits) : limits_(limits) {}

  /// Pulses never overlap, except the two motors of one cue, and last at least
  /// this long, so one window holds at most 2 * windowMs / kMinPulseMs entries
  /// (200 at 10 s): a history longer than that never drops one that still counts.
  static constexpr uint32_t kMinPulseMs = 100;

  /// Accepts or refuses a pulse starting at `nowMs`. The rolling limit counts
  /// time on, whatever the duty: the stricter reading of "50 % rolling duty".
  Refusal request(uint8_t motor, uint8_t duty, uint32_t durationMs, uint32_t nowMs);

  /// A feedback cue: one or two motors together (doc 06 §9); `motor[1]` 0 for
  /// one. Each motor is held to every limit; either refused refuses both.
  Refusal requestCue(const uint8_t motor[2], const uint8_t duty[2], uint32_t durationMs,
                     uint32_t nowMs);

  /// The motor running at `nowMs` (1..6), or 0.
  uint8_t running(uint32_t nowMs) const;

 private:
  struct Pulse {
    uint8_t motor;
    uint32_t startMs;
    uint32_t endMs;
  };
  static constexpr size_t kHistory = 256;

  uint32_t onTimeWithin(uint8_t motor, uint32_t fromMs, uint32_t toMs) const;
  /// Every limit except Busy, for one motor.
  Refusal check(uint8_t motor, uint8_t duty, uint32_t durationMs, uint32_t nowMs) const;
  void record(uint8_t motor, uint32_t startMs, uint32_t endMs);

  Limits limits_;
  Pulse history_[kHistory] = {};
  size_t next_ = 0;
};

}  // namespace ead
