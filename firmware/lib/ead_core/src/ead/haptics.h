#pragma once
// Error-driven haptic cues (doc 06 §6–§13, DEC-023).
//
// One cue per scored cycle, decided when the cycle closes: the right foot has
// just landed (DEC-020: the buzz is the step's error, after the step lands). The
// engine decides only; the device turns a cue into PWM through the motor guard,
// whose limits (5 s on, 50 % per 10 s) bind every motor whatever this decides.

#include <cstdint>

#include "ead/error_engine.h"
#include "ead/gait.h"

namespace ead {

/// Doc 06 §7–§11 as data (§8: "the motor map as data/configuration").
struct HapticConfig {
  /// Motor n+1's position around the shank, degrees clockwise from anterior,
  /// seen from above (doc 06 §7).
  float motorDeg[6] = {0.0f, 60.0f, 120.0f, 180.0f, 240.0f, 300.0f};
  float onScore = 0.35f;            ///< §11: an episode starts at or above
  float offScore = 0.25f;           ///< §11: and ends below
  float startConfidence = 0.75f;    ///< §5: a new episode needs this
  float keepConfidence = 0.50f;     ///< §5: below this, no haptic at all
  uint8_t minDuty = 51;             ///< §10, of 255
  uint8_t maxDuty = 204;
  float intensityExponent = 1.5f;
  /// How long each cue runs. Doc 06 gives none; provisional until worn.
  uint16_t cueMs = 250;
};

/// What a log record says about the episode (docs/protocol.md HAPTIC_BATCH).
enum class HapticEvent : uint8_t { On = 1, Update = 2, Off = 3 };

/// Why an episode ended, or why a cue did not run (doc 06 §13).
enum class HapticReason : uint8_t {
  None = 0,
  BelowThreshold = 1,   ///< score fell under the off threshold
  LowConfidence = 2,    ///< confidence under keepConfidence
  InvalidStep = 3,      ///< the cycle failed the temporal guards
  NoDirection = 4,      ///< no class with a direction (distance or shank only)
  SwitchedOff = 5,      ///< the dashboard's master switch
  SessionEnded = 6,
  SensorFault = 7,      ///< a read failure or a gap in the frames (doc 06 §12)
  Refused = 8,          ///< the motor guard refused the cue (rolling limit)
  LinkLost = 9,         ///< held back: no message from the laptop (DEC-027)
};

struct HapticCue {
  HapticEvent event;
  HapticReason reason;
  /// Up to two motors, 1..6; 0 is unused. Duty 0 for an Off record.
  uint8_t motor[2];
  uint8_t duty[2];
  uint16_t durationMs;
  ErrorClass errorClass;
  float score;
  float confidence;
  /// The cycle that produced it: its opening contact, and when it closed.
  uint32_t cycleStartFrame;
  uint64_t timeUs;
};

class HapticEngine {
 public:
  explicit HapticEngine(const HapticConfig& config = {}) : config_(config) {}

  /// One scored cycle. Returns true and fills `cue` when there is a cue to run
  /// or an episode to close; false when nothing happens.
  bool onCycle(const GaitCycle& cycle, const ErrorResult& score, HapticCue* cue);

  /// Ends a running episode. Returns true and fills an Off record if one ran.
  bool stop(HapticReason reason, uint64_t timeUs, HapticCue* cue);

  bool active() const { return active_; }

 private:
  /// The correction angle for `score`, or false for the timing pattern.
  bool direction(const ErrorResult& score, float* degrees) const;
  void place(float degrees, uint8_t duty, HapticCue* cue) const;
  uint8_t intensity(float score, float confidence) const;

  HapticConfig config_;
  bool active_ = false;
  ErrorClass class_ = ErrorClass::None;
  /// Timing has no direction: M1 and M4 take turns, one per cue (doc 06 §8).
  bool timingFront_ = true;
};

}  // namespace ead
