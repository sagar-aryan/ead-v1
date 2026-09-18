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
// Given by doc 05 §6:
constexpr float kZuptAccelToleranceG = 0.15f;
constexpr float kZuptGyroDps = 25.0f;
constexpr float kZuptHoldMs = 60.0f;
constexpr float kZuptEntryHysteresisMs = 20.0f;
// Given by doc 05 §3–4:
constexpr float kContactSustainMs = 30.0f;
constexpr float kContactWindowMs = 120.0f;
constexpr float kToeOffSustainMs = 40.0f;
constexpr float kMinCycleS = 0.45f;
constexpr float kMaxCycleS = 3.00f;
// Provisional, to be set from recorded walks (doc 05 §3 adaptive thresholds):
/// Opens a contact candidate window: any dynamics worth looking at.
constexpr float kImpactG = 0.45f;
/// Confirms it. Measured on a real walk (TEST-030): heel strikes peaked at
/// 2.3–5.3 g while push-off and mid-swing peaked at 1.5–1.9 g, so the confirm
/// level sits between them. Doc 05 §3 wants this derived per patient from
/// recorded cycles; this is the starting value until that exists.
constexpr float kContactConfirmG = 1.1f;
constexpr float kSwingGyroDps = 90.0f;   ///< foot rate that marks the start of swing
/// A second contact cannot be accepted sooner than the shortest legal cycle: a
/// footfall produces several impact features (heel, then forefoot), and without
/// this each one opens a cycle (TEST-030 measured 20 contacts for 12 steps).
constexpr float kContactRefractoryS = kMinCycleS;
/// Swing must have been under way this long before a contact can end it. The
/// push-off spike arrives within ~100 ms of toe-off and was being read as the
/// next footfall, splitting every stride in two (TEST-030).
constexpr float kMinSwingS = 0.20f;
/// Stance must have lasted this long after a contact before toe-off can be
/// declared. The foot slapping flat after a heel strike turns as fast as a
/// lift-off; read as toe-off, it let the next push-off pass as a contact and
/// split about half the strides of a real walk (PROB-016). Chosen by replay:
/// any value from 0.15 s fixes the split; 0.25 s is where the benefit levels
/// off and still leaves margin for fast walking, whose stance is ~0.35-0.4 s.
/// The 6 m ground-truth course is unchanged (6 cycles, 6.39 m).
constexpr float kMinStanceS = 0.25f;
/// Doc 05 §3: a contact candidate requires the angular speed to be decreasing
/// toward contact. Push-off is the opposite — the foot is speeding up — so a
/// candidate is only accepted once the rate has fallen this far below the peak
/// reached during the swing.
/// 1.0 disables the test. Doc 05 §3 suggests requiring the angular speed to be
/// decreasing toward contact, but measured on the leg the foot's rate peaks
/// within 20 ms of heel strike (TEST-030), so the test rejected real contacts.
/// The impact confirm level does the same job without the assumption.
constexpr float kContactRateFallRatio = 1.0f;

/// The tunable part of the detector. Doc 05 §3 asks for adaptive thresholds
/// built from recorded cycles; keeping them in a struct means a recording can be
/// replayed with different values without rebuilding the firmware, and the
/// values that win can then be defaulted here.
struct GaitConfig {
  float impactG = kImpactG;
  float contactConfirmG = kContactConfirmG;
  float swingGyroDps = kSwingGyroDps;
  float minSwingS = kMinSwingS;
  float minStanceS = kMinStanceS;
  float contactRateFallRatio = kContactRateFallRatio;
  float contactRefractoryS = kContactRefractoryS;
  float zuptAccelToleranceG = kZuptAccelToleranceG;
  float zuptGyroDps = kZuptGyroDps;
};

class GaitEngine {
 public:
  void reset();
  /// Replaces the thresholds. Takes effect on the next sample.
  void configure(const GaitConfig& config) { config_ = config; }
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
  void integrate(const GaitSample& sample, float dt);
  void applyZeroVelocity();
  void removeSegmentDrift(uint64_t nowUs);

  GaitConfig config_{};
  GaitState state_ = GaitState::Init;
  bool zuptActive_ = false;
  float stillMs_ = 0.0f;
  float movingMs_ = 0.0f;
  float contactMs_ = 0.0f;
  float toeOffMs_ = 0.0f;

  uint64_t lastTimeUs_ = 0;
  /// Strongest impact feature seen inside the current contact window.
  float bestImpact_ = 0.0f;
  uint64_t bestImpactUs_ = 0;
  uint32_t bestImpactFrame_ = 0;
  uint64_t contactWindowUs_ = 0;
  uint64_t lastContactUs_ = 0;
  /// Highest foot angular rate seen since toe-off, for the decreasing-rate test.
  float swingPeakRateDps_ = 0.0f;

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
