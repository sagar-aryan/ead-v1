#include "ead/gait.h"

#include <cmath>

#include "ead/calibration.h"
#include "ead/mahony.h"

namespace ead {
namespace {

constexpr float kGravityMps2 = 9.80665f;
/// Process noise on the foot velocity, (m/s)² per second. A walking foot's
/// acceleration error is dominated by orientation error, a degree of which is
/// about 0.17 m/s²; a second of that is the scale used here.
constexpr float kVelocityProcessVariance = 0.03f;
/// Measurement noise of a zero-velocity update. Small but not zero: the foot is
/// nearly, not exactly, still during foot-flat.
constexpr float kZuptMeasurementVariance = 1e-4f;

float magnitude(const float v[3]) {
  return std::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
}

/// Sagittal and frontal angles from the foot-with-respect-to-shank quaternion.
/// Same decomposition as the dashboard's (`orientation.rs`): Ry · Rx · Rz, with
/// dorsiflexion positive.
void ankleAngles(const float q[4], float* sagittalDeg, float* frontalDeg) {
  const float w = q[0], x = q[1], y = q[2], z = q[3];
  const float m02 = 2.0f * (x * z + w * y);
  const float m12 = 2.0f * (y * z - w * x);
  const float m22 = w * w - x * x - y * y + z * z;
  float sine = -m12;
  if (sine > 1.0f) sine = 1.0f;
  if (sine < -1.0f) sine = -1.0f;
  *sagittalDeg = -std::atan2(m02, m22) * 57.295779513f;
  *frontalDeg = std::asin(sine) * 57.295779513f;
}

}  // namespace

void GaitEngine::reset() {
  const GaitConfig config = config_;
  *this = GaitEngine{};
  config_ = config;
}

void GaitEngine::emit(GaitEventType type, uint64_t timeUs, uint32_t frameIndex) {
  if (eventCount_ == kQueue) return;  // the caller is not draining; drop the oldest use
  const int at = (eventHead_ + eventCount_) % kQueue;
  events_[at] = GaitEvent{type, timeUs, frameIndex};
  ++eventCount_;
}

bool GaitEngine::takeEvent(GaitEvent* out) {
  if (eventCount_ == 0) return false;
  *out = events_[eventHead_];
  eventHead_ = (eventHead_ + 1) % kQueue;
  --eventCount_;
  return true;
}

bool GaitEngine::takeCycle(GaitCycle* out) {
  if (cycleCount_ == 0) return false;
  *out = cycles_[cycleHead_];
  cycleHead_ = (cycleHead_ + 1) % 4;
  --cycleCount_;
  return true;
}

void GaitEngine::startCycle(uint64_t timeUs, uint32_t frameIndex) {
  cycle_ = GaitCycle{};
  cycle_.startUs = timeUs;
  cycle_.startFrame = frameIndex;
  cycle_.peakDorsiflexionDeg = -1e9f;
  cycle_.peakInversionDeg = -1e9f;
  cycleOpen_ = true;
  zuptSamples_ = 0;
  cycleSamples_ = 0;
  toeOffUs_ = 0;
  for (int i = 0; i < 3; ++i) displacement_[i] = 0.0f;
}

void GaitEngine::closeCycle(uint64_t timeUs, uint32_t frameIndex) {
  if (!cycleOpen_) return;
  GaitCycle c = cycle_;
  c.endUs = timeUs;
  c.endFrame = frameIndex;
  c.cycleTimeS = float(timeUs - c.startUs) / 1e6f;
  c.stanceTimeS = toeOffUs_ != 0 ? float(toeOffUs_ - c.startUs) / 1e6f : 0.0f;
  c.swingTimeS = toeOffUs_ != 0 ? float(timeUs - toeOffUs_) / 1e6f : 0.0f;
  if (c.cycleTimeS > 0.0f) {
    c.stanceRatio = c.stanceTimeS / c.cycleTimeS;
    c.swingRatio = c.swingTimeS / c.cycleTimeS;
    // Doc 05 §9: a right-to-right cycle is two steps under the alternating
    // assumption, which is why this is 120 and not 60.
    c.cadenceStepsPerMin = 120.0f / c.cycleTimeS;
  }
  c.zuptQuality = cycleSamples_ > 0 ? float(zuptSamples_) / float(cycleSamples_) : 0.0f;
  // Horizontal displacement over the cycle: vertical travel is not distance.
  c.distanceM = std::sqrt(displacement_[0] * displacement_[0] + displacement_[1] * displacement_[1]);
  c.speedMps = c.cycleTimeS > 0.0f ? c.distanceM / c.cycleTimeS : 0.0f;
  if (c.peakDorsiflexionDeg < -1e8f) c.peakDorsiflexionDeg = 0.0f;
  if (c.peakInversionDeg < -1e8f) c.peakInversionDeg = 0.0f;
  // Doc 05 §4: a cycle outside the temporal guards is reported, but marked so
  // nothing downstream treats it as a measurement.
  c.valid = c.cycleTimeS >= kMinCycleS && c.cycleTimeS <= kMaxCycleS && toeOffUs_ != 0;

  if (cycleCount_ < 4) {
    cycles_[(cycleHead_ + cycleCount_) % 4] = c;
    ++cycleCount_;
  }
  cycleOpen_ = false;
}

void GaitEngine::integrate(const GaitSample& sample, float dt) {
  // Acceleration in the world frame, with gravity removed.
  float worldG[3];
  rotateByQuaternion(sample.footQuaternion, sample.footAccelG, worldG);
  const float a[3] = {worldG[0] * kGravityMps2, worldG[1] * kGravityMps2,
                      (worldG[2] - 1.0f) * kGravityMps2};
  for (int i = 0; i < 3; ++i) {
    velocity_[i] += a[i] * dt;
    displacement_[i] += velocity_[i] * dt;
    velocityVariance_[i] += kVelocityProcessVariance * dt;
  }
}

void GaitEngine::removeSegmentDrift(uint64_t nowUs) {
  // The foot is at rest at both ends of a swing, so whatever velocity has
  // accumulated by the end of one is error — measured on the leg it reaches
  // 6–10 m/s, from gravity leaking through the orientation estimate while the
  // foot is accelerating (TEST-030). Attributing it as a linear ramp over the
  // segment and removing it from the displacement is the closed form of
  // re-integrating the corrected velocity: with a uniform sample period the
  // correction is exactly the end velocity times half the segment duration.
  if (movingSinceUs_ == 0 || nowUs <= movingSinceUs_) return;
  const float seconds = float(nowUs - movingSinceUs_) / 1e6f;
  for (int i = 0; i < 3; ++i) displacement_[i] -= velocity_[i] * seconds * 0.5f;
  movingSinceUs_ = 0;
}

void GaitEngine::applyZeroVelocity() {
  // Error-state update on the velocity substate (doc 05 §7): the velocity is
  // pulled toward zero by a Kalman gain rather than hard-set, so a single noisy
  // still sample cannot erase real motion, and the position keeps the integral
  // of the corrected state rather than being reset.
  for (int i = 0; i < 3; ++i) {
    const float gain = velocityVariance_[i] / (velocityVariance_[i] + kZuptMeasurementVariance);
    velocity_[i] -= gain * velocity_[i];
    velocityVariance_[i] *= (1.0f - gain);
  }
}

void GaitEngine::update(const GaitSample& sample) {
  const float dt = lastTimeUs_ == 0 ? 0.0f : float(sample.timeUs - lastTimeUs_) / 1e6f;
  lastTimeUs_ = sample.timeUs;
  if (dt <= 0.0f || dt > 0.1f) {
    // A gap: neither the integrator nor the event timers can be trusted across
    // it, and an open cycle would be measured over an interval that was not
    // observed. Everything restarts, including any zero-velocity window.
    state_ = GaitState::Init;
    for (int i = 0; i < 3; ++i) {
      velocity_[i] = 0.0f;
      velocityVariance_[i] = 0.0f;
    }
    if (zuptActive_) {
      zuptActive_ = false;
      emit(GaitEventType::ZuptEnd, sample.timeUs, sample.frameIndex);
    }
    stillMs_ = movingMs_ = contactMs_ = toeOffMs_ = 0.0f;
    cycleOpen_ = false;
    return;
  }
  const float ms = dt * 1000.0f;

  const float accelMagnitude = magnitude(sample.footAccelG);
  const float footRate = magnitude(sample.footGyroDps);
  const float shankRate = magnitude(sample.shankGyroDps);
  const bool stillNow = std::fabs(accelMagnitude - 1.0f) <= config_.zuptAccelToleranceG &&
                        footRate <= config_.zuptGyroDps;

  stillMs_ = stillNow ? stillMs_ + ms : 0.0f;
  movingMs_ = stillNow ? 0.0f : movingMs_ + ms;

  integrate(sample, dt);

  // ---- zero-velocity windows ----------------------------------------------
  // Anything but swing may hold a zero-velocity window. PRE_SWING was excluded
  // at first, which locked the detector out for a whole cycle whenever a single
  // moving sample during stance flipped the state: measured ZUPT quality was
  // 0.00 on almost every real cycle (TEST-030).
  const bool stanceContext = state_ != GaitState::Swing;
  // Doc 05 §6: the hysteresis is on entry — 20 ms of stillness before the 60 ms
  // hold begins to count. Exit is immediate, because a foot that has started
  // moving is not still, and holding the update even briefly discards real
  // acceleration at the start of every swing.
  if (!zuptActive_ && stillNow && stanceContext &&
      stillMs_ >= kZuptHoldMs + kZuptEntryHysteresisMs) {
    removeSegmentDrift(sample.timeUs);
    zuptActive_ = true;
    emit(GaitEventType::ZuptStart, sample.timeUs, sample.frameIndex);
    if (state_ != GaitState::Init) {
      state_ = GaitState::FootFlatZv;
      emit(GaitEventType::FootFlat, sample.timeUs, sample.frameIndex);
    }
  } else if (zuptActive_ && !stillNow) {
    zuptActive_ = false;
    movingSinceUs_ = sample.timeUs;
    emit(GaitEventType::ZuptEnd, sample.timeUs, sample.frameIndex);
  }
  if (zuptActive_) {
    applyZeroVelocity();
    ++zuptSamples_;
  }
  if (cycleOpen_) ++cycleSamples_;

  // ---- features ------------------------------------------------------------
  float sagittal = 0.0f;
  float frontal = 0.0f;
  ankleAngles(sample.relativeQuaternion, &sagittal, &frontal);
  if (cycleOpen_) {
    if (shankRate > cycle_.peakShankRateDps) cycle_.peakShankRateDps = shankRate;
    if (frontal > cycle_.peakInversionDeg) cycle_.peakInversionDeg = frontal;
    // Doc 05 §11 asks for the dorsiflexion peak in swing specifically.
    if (state_ == GaitState::Swing && sagittal > cycle_.peakDorsiflexionDeg) {
      cycle_.peakDorsiflexionDeg = sagittal;
    }
  }

  // ---- state machine -------------------------------------------------------
  switch (state_) {
    case GaitState::Init:
      // Wait for the foot to be still before claiming to know anything.
      if (zuptActive_) state_ = GaitState::FootFlatZv;
      break;

    case GaitState::Stance:
    case GaitState::FootFlatZv:
    case GaitState::PreSwing: {
      const bool leaving = footRate > config_.swingGyroDps || !stillNow;
      toeOffMs_ = leaving ? toeOffMs_ + ms : 0.0f;
      if (leaving && state_ != GaitState::PreSwing) state_ = GaitState::PreSwing;
      if (toeOffMs_ >= kToeOffSustainMs && footRate > config_.swingGyroDps) {
        state_ = GaitState::Swing;
        toeOffMs_ = 0.0f;
        toeOffUs_ = sample.timeUs;
        swingPeakRateDps_ = 0.0f;
        emit(GaitEventType::ToeOff, sample.timeUs, sample.frameIndex);
      }
      break;
    }

    case GaitState::Swing: {
      if (footRate > swingPeakRateDps_) swingPeakRateDps_ = footRate;
      const float sinceToeOff =
          toeOffUs_ == 0 ? 1e9f : float(sample.timeUs - toeOffUs_) / 1e6f;
      // Doc 05 §3: the candidate needs the angular speed to be decreasing toward
      // contact. Both tests below reject the push-off spike, which arrives early
      // in swing while the foot is still speeding up.
      const bool swinging = sinceToeOff >= config_.minSwingS;
      const bool slowing = config_.contactRateFallRatio >= 1.0f ||
                           footRate < swingPeakRateDps_ * config_.contactRateFallRatio;

      // A contact candidate is an impact feature; the event is timestamped at
      // the strongest one inside the window, not at the first (doc 05 §3).
      const float impact = (swinging && slowing) ? std::fabs(accelMagnitude - 1.0f) : 0.0f;
      if (impact >= config_.impactG) {
        if (contactMs_ == 0.0f) {
          contactWindowUs_ = sample.timeUs;
          bestImpact_ = 0.0f;
        }
        contactMs_ += ms;
        if (impact > bestImpact_) {
          bestImpact_ = impact;
          bestImpactUs_ = sample.timeUs;
          bestImpactFrame_ = sample.frameIndex;
        }
      } else if (contactMs_ > 0.0f && contactMs_ < kContactSustainMs &&
                 bestImpact_ < config_.contactConfirmG) {
        contactMs_ = 0.0f;  // a brief, weak blip: not a footfall
      }

      // Doc 05 §3 asks for 30 ms of sustained candidate. A heel strike can be
      // sharper than that: measured on the leg, one real contact spanned three
      // samples, and at the device's 100.147 Hz that is 29.96 ms — rejected for
      // being 0.04 ms short (TEST-030). An impact at the confirm level is taken
      // as decisive on its own.
      const bool qualified =
          contactMs_ >= kContactSustainMs || bestImpact_ >= config_.contactConfirmG;
      const bool windowClosed =
          qualified &&
          (float(sample.timeUs - contactWindowUs_) / 1000.0f >= kContactWindowMs || stillNow);
      const bool tooWeak = bestImpact_ < config_.contactConfirmG;
      const bool tooSoon =
          lastContactUs_ != 0 &&
          float(bestImpactUs_ - lastContactUs_) / 1e6f < config_.contactRefractoryS;
      if (windowClosed && (tooSoon || tooWeak)) {
        // Either the same footfall seen again — heel and forefoot are two
        // impacts — or a push-off, which is softer than a heel strike.
        contactMs_ = 0.0f;
        bestImpact_ = 0.0f;
      } else if (windowClosed) {
        state_ = GaitState::ContactTransition;
        contactMs_ = 0.0f;
        cycle_.contactSagittalDeg = sagittal;
        // Close the previous cycle and open the next at the same instant.
        closeCycle(bestImpactUs_, bestImpactFrame_);
        emit(GaitEventType::InitialContact, bestImpactUs_, bestImpactFrame_);
        lastContactUs_ = bestImpactUs_;
        const float contactSagittal = sagittal;
        startCycle(bestImpactUs_, bestImpactFrame_);
        cycle_.contactSagittalDeg = contactSagittal;
      }
      break;
    }

    case GaitState::ContactTransition:
      if (zuptActive_ || stillMs_ >= kZuptHoldMs) state_ = GaitState::FootFlatZv;
      else if (footRate > config_.swingGyroDps && movingMs_ > kToeOffSustainMs) state_ = GaitState::Stance;
      else state_ = GaitState::Stance;
      break;

    case GaitState::Fault:
      break;
  }
}

}  // namespace ead
