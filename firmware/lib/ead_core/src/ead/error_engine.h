#pragma once
// Error score, classes and confidence for one gait cycle (doc 06).
//
// The score is an engineering research metric in [0,1], not a clinical severity
// scale, and nothing here should be presented as one: 0 means the cycle sat
// inside the reference envelope, 1 means it was at or beyond the maximum
// normalized deviation.
//
// Confidence is reported separately and is never folded into the score. A cycle
// can deviate a great deal and be worth little (a failed sensor read, no
// zero-velocity window), and the two facts must stay legible apart.

#include <cstdint>

#include "ead/gait.h"
#include "ead/reference.h"

namespace ead {

/// Doc 06 §4. The first five are feature-specific; the last is the fallback
/// when several features deviate and none dominates.
enum class ErrorClass : uint8_t {
  None = 0,
  InsufficientDorsiflexion = 1,
  ExcessPlantarflexion = 2,
  InversionDeviation = 3,
  EversionDeviation = 4,
  TimingDeviation = 5,
  OverallDeviation = 6,
};

/// Bit per class, for the set of classes active on a cycle.
constexpr uint16_t errorClassBit(ErrorClass c) {
  return uint16_t(1u << uint8_t(c));
}

/// A feature counts as deviating at or above this (doc 06 §4).
constexpr float kClassActiveDeviation = 0.35f;
/// Doc 06 §2: d = clamp(z / 3, 0, 1).
constexpr float kDeviationScale = 3.0f;

/// Doc 06 §5 weights, in subscore order.
constexpr float kConfidenceWeights[5] = {0.30f, 0.25f, 0.20f, 0.15f, 0.10f};

/// What the caller knows about the cycle that the features themselves do not.
struct ConfidenceInputs {
  /// Fraction of the cycle's frames with no read failure or saturation.
  float sensorQuality;
  /// How cleanly the cycle's events were detected: 1 when initial contact and
  /// toe-off were both accepted inside their guards.
  float eventQuality;
};

/// Doc 06 §5 gating.
constexpr float kConfidenceForFeedback = 0.75f;
constexpr float kConfidenceForDisplay = 0.50f;

/// Doc 05 §8: below this zero-velocity quality, distance and speed are reported
/// as low-confidence rather than as measurements. The engine drops cycle
/// distance below it for the same reason — on 2026-09-18 cycles at quality
/// 0.02–0.04 reported 15–48 m strides and scored full deviation (PROB-015).
constexpr float kDistanceMinZuptQuality = 0.15f;

struct ErrorResult {
  /// [0,1]; the weighted mean of the active features' deviations.
  float score;
  float deviations[kFeatureCount];
  /// Which features were included in the denominator.
  bool active[kFeatureCount];
  uint16_t activeClasses;
  ErrorClass primaryClass;
  float confidence;
  /// Sensor, event, feature completeness, reference stability, ZUPT.
  float subscores[5];
};

/// Scores one cycle against a locked reference.
ErrorResult scoreCycle(const GaitCycle& cycle, const ReferenceProfile& reference,
                       const ConfidenceInputs& inputs);

const char* errorClassName(ErrorClass c);

}  // namespace ead
