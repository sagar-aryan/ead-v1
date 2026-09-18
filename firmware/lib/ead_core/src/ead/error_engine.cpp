#include "ead/error_engine.h"

#include <cmath>

namespace ead {
namespace {

float clamp01(float v) {
  if (v < 0.0f) return 0.0f;
  if (v > 1.0f) return 1.0f;
  return v;
}

/// Which class a feature raises, and in which direction. Doc 06 §4 names five
/// feature-specific classes; the sign says which side of the median counts.
/// Returns None for features that have no class of their own — they still
/// contribute to the score, and can still produce OVERALL_DEVIATION.
ErrorClass classFor(GaitFeature feature, float value, float median) {
  switch (feature) {
    case GaitFeature::SwingDorsiflexion:
      // Only too little dorsiflexion has a class; more than the reference is
      // not a V1 error (doc 06 §8 lists EXCESS_DORSIFLEXION as internal only).
      return value < median ? ErrorClass::InsufficientDorsiflexion : ErrorClass::None;
    case GaitFeature::ContactPlantarflexion:
      // The sagittal angle at contact: more negative is more plantarflexed.
      return value < median ? ErrorClass::ExcessPlantarflexion : ErrorClass::None;
    case GaitFeature::Inversion:
      return value > median ? ErrorClass::InversionDeviation : ErrorClass::EversionDeviation;
    case GaitFeature::CycleTime:
    case GaitFeature::StanceRatio:
      return ErrorClass::TimingDeviation;
    case GaitFeature::CycleDistance:
    case GaitFeature::ShankDynamics:
      return ErrorClass::None;
  }
  return ErrorClass::None;
}

}  // namespace

const char* errorClassName(ErrorClass c) {
  switch (c) {
    case ErrorClass::None:
      return "none";
    case ErrorClass::InsufficientDorsiflexion:
      return "insufficient_dorsiflexion";
    case ErrorClass::ExcessPlantarflexion:
      return "excess_plantarflexion";
    case ErrorClass::InversionDeviation:
      return "inversion_deviation";
    case ErrorClass::EversionDeviation:
      return "eversion_deviation";
    case ErrorClass::TimingDeviation:
      return "timing_deviation";
    case ErrorClass::OverallDeviation:
      return "overall_deviation";
  }
  return "unknown";
}

ErrorResult scoreCycle(const GaitCycle& cycle, const ReferenceProfile& reference,
                       const ConfidenceInputs& inputs) {
  ErrorResult result{};
  result.primaryClass = ErrorClass::None;

  float weighted = 0.0f;
  float activeWeight = 0.0f;
  float bestContribution = 0.0f;
  ErrorClass bestClass = ErrorClass::None;
  int activeFeatures = 0;
  int activeClassCount = 0;

  for (size_t f = 0; f < kFeatureCount; ++f) {
    const GaitFeature feature = GaitFeature(f);
    const float value = featureValue(cycle, feature);
    const ReferenceFeature& ref = reference.features[f];

    // A feature the device could not measure is dropped from the denominator
    // rather than scored as zero deviation (doc 06 §3): distance means nothing
    // without a zero-velocity window to correct it.
    const bool measured =
        !(feature == GaitFeature::CycleDistance && cycle.zuptQuality < kDistanceMinZuptQuality) &&
        ref.spread > 0.0f;
    result.active[f] = measured;
    if (!measured) continue;
    ++activeFeatures;

    const float z = std::fabs(value - ref.median) / ref.spread;
    const float d = clamp01(z / kDeviationScale);
    result.deviations[f] = d;
    weighted += kFeatureWeights[f] * d;
    activeWeight += kFeatureWeights[f];

    if (d >= kClassActiveDeviation) {
      const ErrorClass raised = classFor(feature, value, ref.median);
      if (raised != ErrorClass::None) {
        result.activeClasses |= errorClassBit(raised);
        const float contribution = kFeatureWeights[f] * d;
        if (contribution > bestContribution) {
          bestContribution = contribution;
          bestClass = raised;
        }
        ++activeClassCount;
      }
    }
  }

  result.score = activeWeight > 0.0f ? weighted / activeWeight : 0.0f;

  // Doc 06 §4: the largest weighted contribution is primary; OVERALL_DEVIATION
  // is for when several deviate and none leads.
  if (bestClass != ErrorClass::None) {
    result.primaryClass = bestClass;
    if (activeClassCount > 1) {
      // True when another class comes close enough that calling the leader
      // primary would overstate it (doc 06 §4: "no single class dominates").
      bool contested = false;
      for (size_t f = 0; f < kFeatureCount; ++f) {
        if (!result.active[f] || result.deviations[f] < kClassActiveDeviation) continue;
        const ErrorClass raised =
            classFor(GaitFeature(f), featureValue(cycle, GaitFeature(f)), reference.features[f].median);
        if (raised == ErrorClass::None || raised == bestClass) continue;
        // Another class within a tenth of the leader means nothing dominates.
        if (kFeatureWeights[f] * result.deviations[f] >= bestContribution * 0.9f) {
          contested = true;
        }
      }
      if (contested) {
        result.primaryClass = ErrorClass::OverallDeviation;
        result.activeClasses |= errorClassBit(ErrorClass::OverallDeviation);
      }
    }
  }

  // Doc 06 §5.
  result.subscores[0] = clamp01(inputs.sensorQuality);
  result.subscores[1] = clamp01(inputs.eventQuality);
  result.subscores[2] = float(activeFeatures) / float(kFeatureCount);
  // Reference stability: how much evidence the reference rests on, saturating
  // at twice the minimum. Thirty cycles is the floor, sixty is as good as it
  // gets for V1.
  result.subscores[3] =
      clamp01(float(reference.cycles) / float(kReferenceMinCycles * 2));
  result.subscores[4] = clamp01(cycle.zuptQuality);

  float confidence = 0.0f;
  for (int i = 0; i < 5; ++i) confidence += kConfidenceWeights[i] * result.subscores[i];
  result.confidence = clamp01(confidence);
  return result;
}

}  // namespace ead
