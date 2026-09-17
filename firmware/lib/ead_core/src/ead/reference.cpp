#include "ead/reference.h"

#include <cmath>

namespace ead {
namespace {

/// Median of `count` values, sorted in place. `count` is at most 64, so an
/// insertion sort is both the smallest code and fast enough at this size.
float medianOf(float* values, uint16_t count) {
  for (uint16_t i = 1; i < count; ++i) {
    const float key = values[i];
    int16_t j = int16_t(i) - 1;
    while (j >= 0 && values[j] > key) {
      values[j + 1] = values[j];
      --j;
    }
    values[j + 1] = key;
  }
  const uint16_t middle = count / 2;
  // Even counts take the lower of the two middle values rather than their mean,
  // so the median is always a value the patient actually walked.
  return values[middle - (count % 2 == 0 ? 1 : 0)];
}

}  // namespace

float featureValue(const GaitCycle& cycle, GaitFeature feature) {
  switch (feature) {
    case GaitFeature::SwingDorsiflexion:
      return cycle.peakDorsiflexionDeg;
    case GaitFeature::ContactPlantarflexion:
      return cycle.contactSagittalDeg;
    case GaitFeature::Inversion:
      return cycle.peakInversionDeg;
    case GaitFeature::CycleTime:
      return cycle.cycleTimeS;
    case GaitFeature::StanceRatio:
      return cycle.stanceRatio;
    case GaitFeature::CycleDistance:
      return cycle.distanceM;
    case GaitFeature::ShankDynamics:
      return cycle.peakShankRateDps;
  }
  return 0.0f;
}

void ReferenceBuilder::reset() {
  count_ = 0;
  next_ = 0;
}

void ReferenceBuilder::add(const GaitCycle& cycle) {
  if (!cycle.valid) return;
  for (size_t f = 0; f < kFeatureCount; ++f) {
    values_[next_][f] = featureValue(cycle, GaitFeature(f));
  }
  next_ = uint16_t((next_ + 1) % kReferenceMaxCycles);
  if (count_ < kReferenceMaxCycles) ++count_;
}

bool ReferenceBuilder::build(ReferenceProfile* out) const {
  *out = ReferenceProfile{};
  if (count_ < kReferenceMinCycles) return false;

  float scratch[kReferenceMaxCycles];
  float deviations[kReferenceMaxCycles];
  for (size_t f = 0; f < kFeatureCount; ++f) {
    for (uint16_t i = 0; i < count_; ++i) scratch[i] = values_[i][f];
    const float median = medianOf(scratch, count_);
    for (uint16_t i = 0; i < count_; ++i) {
      deviations[i] = std::fabs(values_[i][f] - median);
    }
    const float mad = medianOf(deviations, count_);
    float spread = 1.4826f * mad;
    if (spread < kFeatureSpreadFloor[f]) spread = kFeatureSpreadFloor[f];
    out->features[f] = ReferenceFeature{median, spread};
  }
  out->cycles = count_;
  return true;
}

}  // namespace ead
