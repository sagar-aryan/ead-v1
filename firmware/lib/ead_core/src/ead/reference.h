#pragma once
// Patient-specific reference gait: the envelope every later cycle is judged
// against (doc 01 §68, doc 06 §2).
//
// There is no healthy-population reference. A patient's own walking, captured
// once and then locked, is the baseline — so the same person's later cycles are
// compared with how they walked on the day the reference was taken, not with a
// population they may not belong to.
//
// Statistics are median and 1.4826 × MAD rather than mean and standard
// deviation, because a capture of thirty cycles will contain a stumble or a
// turn and a mean would follow it. Each spread has a floor, so a feature that
// happened not to vary cannot divide by zero.

#include <cstddef>
#include <cstdint>

#include "ead/gait.h"

namespace ead {

/// The features doc 06 §3 weighs. The order is the order of the weight table.
enum class GaitFeature : uint8_t {
  SwingDorsiflexion = 0,
  ContactPlantarflexion = 1,
  Inversion = 2,
  CycleTime = 3,
  StanceRatio = 4,
  CycleDistance = 5,
  ShankDynamics = 6,
};

constexpr size_t kFeatureCount = 7;

/// Doc 06 §3. Kept as one table rather than scattered constants.
constexpr float kFeatureWeights[kFeatureCount] = {
    0.25f,  // swing dorsiflexion
    0.15f,  // initial-contact plantarflexion
    0.15f,  // inversion / eversion
    0.15f,  // cycle timing
    0.10f,  // stance/swing ratio
    0.10f,  // cycle distance
    0.10f,  // shank dynamics
};

/// Spread floors, in each feature's own unit (doc 06 §2 "minimum floor").
/// Provisional: they are the smallest difference worth calling a deviation,
/// and want revisiting once several patients have been captured.
constexpr float kFeatureSpreadFloor[kFeatureCount] = {
    0.5f,    // degrees
    0.5f,    // degrees
    0.5f,    // degrees
    0.010f,  // seconds
    0.005f,  // ratio
    0.020f,  // metres
    5.0f,    // degrees per second
};

/// Doc 05 §11 features, read off a cycle. One place where a feature's meaning
/// and its source field are connected.
float featureValue(const GaitCycle& cycle, GaitFeature feature);

struct ReferenceFeature {
  float median;
  /// 1.4826 × MAD, floored. Never zero.
  float spread;
};

struct ReferenceProfile {
  ReferenceFeature features[kFeatureCount];
  /// Valid cycles the profile was built from.
  uint16_t cycles;
  /// Set by the dashboard, which owns versioning; the device leaves it 0.
  uint16_t version;
};

/// Doc 12: a reference needs at least this many valid cycles.
constexpr uint16_t kReferenceMinCycles = 30;
/// Cycles kept for the statistics. Beyond this the oldest are dropped, so a
/// long capture describes the end of itself rather than its beginning.
constexpr uint16_t kReferenceMaxCycles = 64;

class ReferenceBuilder {
 public:
  void reset();
  /// Invalid cycles are ignored: a reference must not be built from cycles the
  /// detector already rejected.
  void add(const GaitCycle& cycle);
  uint16_t count() const { return count_; }
  bool ready() const { return count_ >= kReferenceMinCycles; }
  /// Fills `out` and returns false when there are too few cycles.
  bool build(ReferenceProfile* out) const;

 private:
  float values_[kReferenceMaxCycles][kFeatureCount] = {};
  uint16_t count_ = 0;
  uint16_t next_ = 0;
};

}  // namespace ead
