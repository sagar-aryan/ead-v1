#pragma once
// Gait state machine, events, ZUPT and per-cycle features (doc 05).
//
// Only the right leg is instrumented, so the repeating unit is the gait cycle:
// right initial contact to the next right initial contact (doc 05 §1). Nothing
// here is called a step time.
//
// Thresholds come from doc 05 where the document gives numbers (the ZUPT window
// and the temporal guards) and are provisional where it does not (the impact and
// swing thresholds), to be replaced from recorded walks. Every provisional value
// is named below so it can be found and changed in one place.

#include <cstdint>

namespace ead {

/// Doc 05 §2.
enum class GaitState : uint8_t {
  Init = 0,
  Swing = 1,
  ContactTransition = 2,
  Stance = 3,
  FootFlatZv = 4,
  PreSwing = 5,
  Fault = 6,
};

enum class GaitEventType : uint8_t {
  InitialContact = 1,
  ToeOff = 2,
  FootFlat = 3,
  ZuptStart = 4,
  ZuptEnd = 5,
};

struct GaitEvent {
  GaitEventType type;
  uint64_t timeUs;
  uint32_t frameIndex;
};

/// One frame's inputs, already calibrated and aligned: acceleration in g and
/// angular rate in deg/s, both in the segment's anatomical frame, plus the
/// orientation the estimator produced for that frame.
struct GaitSample {
  uint64_t timeUs;
  uint32_t frameIndex;
  float footAccelG[3];
  float footGyroDps[3];
  float shankGyroDps[3];
  /// Foot orientation, world from body, used to remove gravity for distance.
  float footQuaternion[4];
  /// Foot with respect to shank: the ankle angles come from this.
  float relativeQuaternion[4];
};

/// Doc 05 §11. Angles are degrees, times seconds, distance metres.
struct GaitCycle {
  uint32_t startFrame;
  uint32_t endFrame;
  uint64_t startUs;
  uint64_t endUs;
  float cycleTimeS;
  float stanceTimeS;
  float swingTimeS;
  float stanceRatio;
  float swingRatio;
  float cadenceStepsPerMin;
  float peakShankRateDps;
  /// Most dorsiflexed the foot gets during swing, relative to the shank.
  float peakDorsiflexionDeg;
  /// Sagittal angle at initial contact; negative is plantarflexion-related.
  float contactSagittalDeg;
  float peakInversionDeg;
  /// Estimated cycle distance. Only meaningful when `zuptQuality` is adequate.
  float distanceM;
  float speedMps;
  /// Fraction of the cycle spent in an accepted zero-velocity window, 0…1.
  float zuptQuality;
  /// False when a temporal guard rejected the cycle (doc 05 §4).
  bool valid;
};

// ---- thresholds ------------------------------------------------------------
// Given by doc 05 §6, except the gyroscope limit: 30 deg/s, not 25, by the
// user's decision (DEC-020; PROB-024 attempt 5: at 25 the foot of a fast step
// often never slows enough for long enough).
constexpr float kZuptAccelToleranceG = 0.15f;
constexpr float kZuptGyroDps = 30.0f;
constexpr float kZuptHoldMs = 60.0f;
constexpr float kZuptEntryHysteresisMs = 20.0f;
// Given by doc 05 §4:
constexpr float kMinCycleS = 0.45f;
constexpr float kMaxCycleS = 3.00f;

// Step detection from the shank (DEC-022). In the right shank's anatomical frame
// (+X forward, +Y medial, +Z up) the shank swinging forward turns about -Y, so
// the forward swing rate is -shankGyroDps[1]. Every stride has one large
// positive peak of it at mid-swing; heel strike follows within about 50 ms of
// the rate crossing zero on the way down; toe-off is the deepest negative rate
// just before the swing (Aminian 2002, Salarian 2004). Values from the
// 2026-10-04 walks (TEST-058, TEST-059):
/// Mid-swing: strides peaked at 140–290 deg/s; turn shuffles and the walk to the
/// start line at 40–100.
constexpr float kMidSwingDps = 100.0f;
/// How long after the downward zero crossing the heel strike is searched for.
/// The foot's impact peak came 35–60 ms after it (median), 90 % within 90 ms.
constexpr float kContactSearchS = 0.15f;
/// The contact's own transient: no toe-off is looked for this soon after it.
constexpr float kContactSettleS = 0.15f;
/// A swing with no downward zero crossing for this long is abandoned.
constexpr float kMaxSwingS = 1.0f;

/// Doc 04 §7's event path: a 2nd-order Butterworth low-pass at 20 Hz
/// (CONFIG_V1.json event_path_lowpass_hz) for the stillness test (PROB-024).
constexpr float kEventPathLowPassHz = 20.0f;
/// The device's frame rate (DEC-021); a replay of an older recording sets its own.
constexpr float kFrameRateHz = 200.0f;

/// 2nd-order Butterworth low-pass, bilinear transform, transposed direct form II.
class LowPass2 {
 public:
  LowPass2(float cutoffHz, float sampleHz);
  /// Settles the filter at a constant input, with no transient.
  void reset(float value);
  float step(float x);

 private:
  float b0_, b1_, b2_, a1_, a2_;
  float z1_ = 0.0f, z2_ = 0.0f;
};

/// The tunable part of the detector. Doc 05 §3 asks for adaptive thresholds
/// built from recorded cycles; keeping them in a struct means a recording can be
/// replayed with different values without rebuilding the firmware, and the
/// values that win can then be defaulted here.
struct GaitConfig {
  float midSwingDps = kMidSwingDps;
  float contactSearchS = kContactSearchS;
  float zuptAccelToleranceG = kZuptAccelToleranceG;
  float zuptGyroDps = kZuptGyroDps;
  /// Frames per second, for the event-path filters.
  float sampleHz = kFrameRateHz;
};

class GaitEngine {
 public:
  void reset();
  /// Replaces the thresholds. Takes effect on the next sample.
  void configure(const GaitConfig& config) {
    config_ = config;
    stillAccel_ = LowPass2(kEventPathLowPassHz, config.sampleHz);
    swingRate_ = LowPass2(kEventPathLowPassHz, config.sampleHz);
  }
  const GaitConfig& config() const { return config_; }

  /// Consumes one frame. Events and completed cycles are queued; drain them
  /// with `takeEvent` and `takeCycle` after each call.
  void update(const GaitSample& sample);

  GaitState state() const { return state_; }
  bool inZupt() const { return zuptActive_; }

  bool takeEvent(GaitEvent* out);
  bool takeCycle(GaitCycle* out);

 private:
  void emit(GaitEventType type, uint64_t timeUs, uint32_t frameIndex);
  void startCycle(uint64_t timeUs, uint32_t frameIndex);
  void closeCycle(uint64_t timeUs, uint32_t frameIndex);
  void claimContact(uint64_t timeUs, uint32_t frameIndex, float sagittal);
  void integrate(const GaitSample& sample, float dt);
  void applyZeroVelocity();
  void removeSegmentDrift(uint64_t nowUs);

  GaitConfig config_{};
  GaitState state_ = GaitState::Init;
  bool zuptActive_ = false;
  float stillMs_ = 0.0f;

  uint64_t lastTimeUs_ = 0;
  LowPass2 stillAccel_{kEventPathLowPassHz, kFrameRateHz};
  /// The shank's forward swing rate on the event path.
  LowPass2 swingRate_{kEventPathLowPassHz, kFrameRateHz};
  uint64_t lastContactUs_ = 0;
  /// Deepest negative swing rate of the current run below zero: the toe-off.
  bool belowZero_ = false;
  float toeOffRate_ = 0.0f;
  uint64_t toeOffCandidateUs_ = 0;
  uint32_t toeOffCandidateFrame_ = 0;
  /// The swing in progress: its peak, whether it is past half of it, and when
  /// the rate crossed zero on the way down (0: not yet).
  uint64_t midSwingUs_ = 0;
  float swingPeakDps_ = 0.0f;
  bool descending_ = false;
  uint64_t zeroCrossUs_ = 0;
  /// Strongest foot impact since the swing passed half its peak: the contact.
  float bestImpact_ = 0.0f;
  uint64_t bestImpactUs_ = 0;
  uint32_t bestImpactFrame_ = 0;

  // Current cycle in progress.
  bool cycleOpen_ = false;
  GaitCycle cycle_{};
  uint64_t toeOffUs_ = 0;
  uint32_t zuptSamples_ = 0;
  uint32_t cycleSamples_ = 0;

  // Foot velocity and position, world frame, metres.
  float velocity_[3] = {0, 0, 0};
  float displacement_[3] = {0, 0, 0};
  /// Error-state covariance for the velocity substate (doc 05 §7), diagonal.
  float velocityVariance_[3] = {0, 0, 0};
  /// Start of the current moving segment, for the drift correction below.
  uint64_t movingSinceUs_ = 0;

  static constexpr int kQueue = 16;
  GaitEvent events_[kQueue]{};
  int eventHead_ = 0;
  int eventCount_ = 0;
  GaitCycle cycles_[4]{};
  int cycleHead_ = 0;
  int cycleCount_ = 0;
};

}  // namespace ead
