#include "ead/haptics.h"

#include <cmath>

#include "ead/reference.h"

namespace ead {
namespace {

/// Doc 06 §8: where each class's cue sits, degrees clockwise from anterior. The
/// cue is on the side the wearer should move away from. Negative: no direction
/// of its own (none, timing, overall).
constexpr float kClassCueDeg[7] = {
    -1.0f,   // None
    180.0f,  // InsufficientDorsiflexion: posterior, M4
    180.0f,  // ExcessPlantarflexion: posterior, M4
    270.0f,  // InversionDeviation: medial, between M5 and M6
    90.0f,   // EversionDeviation: lateral, between M2 and M3
    -1.0f,   // TimingDeviation: M1 and M4 in turn
    -1.0f,   // OverallDeviation: the sum of the active classes' cues
};

/// The feature whose deviation measures each directional class.
constexpr GaitFeature kClassFeature[7] = {
    GaitFeature::SwingDorsiflexion,   GaitFeature::SwingDorsiflexion,
    GaitFeature::ContactPlantarflexion, GaitFeature::Inversion,
    GaitFeature::Inversion,           GaitFeature::CycleTime,
    GaitFeature::CycleTime,
};

constexpr float kDegToRad = 0.017453292f;

float circularDistance(float a, float b) {
  float d = std::fmod(std::fabs(a - b), 360.0f);
  return d > 180.0f ? 360.0f - d : d;
}

}  // namespace

bool HapticEngine::direction(const ErrorResult& score, float* degrees) const {
  const uint8_t primary = uint8_t(score.primaryClass);
  if (primary < 7 && kClassCueDeg[primary] >= 0.0f) {
    *degrees = kClassCueDeg[primary];
    return true;
  }
  if (score.primaryClass != ErrorClass::OverallDeviation) return false;
  // Doc 06 §8: the vector sum of the active classes' cues, each as long as its
  // feature's deviation.
  float x = 0.0f;
  float y = 0.0f;
  for (uint8_t c = 1; c < 7; ++c) {
    if ((score.activeClasses & errorClassBit(ErrorClass(c))) == 0 || kClassCueDeg[c] < 0.0f) {
      continue;
    }
    const float d = score.deviations[size_t(kClassFeature[c])];
    x += d * std::cos(kClassCueDeg[c] * kDegToRad);
    y += d * std::sin(kClassCueDeg[c] * kDegToRad);
  }
  if (std::sqrt(x * x + y * y) < 1e-3f) return false;  // only timing, or cues cancelling
  float angle = std::atan2(y, x) / kDegToRad;
  if (angle < 0.0f) angle += 360.0f;
  *degrees = angle;
  return true;
}

void HapticEngine::place(float degrees, uint8_t duty, HapticCue* cue) const {
  // Doc 06 §9: the two nearest motors, weighted linearly by closeness.
  int first = 0;
  int second = 1;
  float distance[6];
  for (int i = 0; i < 6; ++i) distance[i] = circularDistance(degrees, config_.motorDeg[i]);
  if (distance[second] < distance[first]) {
    first = 1;
    second = 0;
  }
  for (int i = 2; i < 6; ++i) {
    if (distance[i] < distance[first]) {
      second = first;
      first = i;
    } else if (distance[i] < distance[second]) {
      second = i;
    }
  }
  const float span = distance[first] + distance[second];
  const float nearWeight = span > 0.0f ? distance[second] / span : 1.0f;
  const float farWeight = 1.0f - nearWeight;
  cue->motor[0] = uint8_t(first + 1);
  cue->duty[0] = duty;
  // The nearer motor carries the intensity; the other a share of it, dropped
  // when that share is below the weakest duty a motor may run at (§10).
  const float share = std::round(float(duty) * farWeight / nearWeight);
  if (share >= float(config_.minDuty)) {
    cue->motor[1] = uint8_t(second + 1);
    cue->duty[1] = uint8_t(share);
  }
}

uint8_t HapticEngine::intensity(float score, float confidence) const {
  // Doc 06 §10.
  const float p = std::pow(score, config_.intensityExponent);
  float pwm = float(config_.minDuty) + float(config_.maxDuty - config_.minDuty) * p * confidence;
  pwm = std::round(pwm);
  if (pwm < float(config_.minDuty)) pwm = float(config_.minDuty);
  if (pwm > float(config_.maxDuty)) pwm = float(config_.maxDuty);
  return uint8_t(pwm);
}

bool HapticEngine::onCycle(const GaitCycle& cycle, const ErrorResult& score, HapticCue* cue) {
  *cue = HapticCue{};
  cue->errorClass = score.primaryClass;
  cue->score = score.score;
  cue->confidence = score.confidence;
  cue->cycleStartFrame = cycle.startFrame;
  cue->timeUs = cycle.endUs;

  float degrees = 0.0f;
  const bool directional = direction(score, &degrees);
  const bool hasClass = score.primaryClass != ErrorClass::None;

  HapticReason end = HapticReason::None;
  if (!cycle.valid) {
    end = HapticReason::InvalidStep;
  } else if (score.confidence < config_.keepConfidence) {
    end = HapticReason::LowConfidence;
  } else if (score.score < config_.offScore) {
    end = HapticReason::BelowThreshold;
  } else if (!hasClass) {
    end = HapticReason::NoDirection;
  }
  if (end != HapticReason::None) {
    if (!stop(end, cycle.endUs, cue)) return false;
    cue->score = score.score;
    cue->confidence = score.confidence;
    cue->cycleStartFrame = cycle.startFrame;
    return true;
  }
  if (!active_) {
    // Doc 06 §5, §11: a new episode needs the on threshold and full confidence.
    if (score.score < config_.onScore || score.confidence < config_.startConfidence) return false;
    active_ = true;
    cue->event = HapticEvent::On;
  } else {
    cue->event = HapticEvent::Update;
  }
  class_ = score.primaryClass;

  const uint8_t duty = intensity(score.score, score.confidence);
  if (directional) {
    place(degrees, duty, cue);
  } else {
    cue->motor[0] = timingFront_ ? 1 : 4;
    cue->duty[0] = duty;
    timingFront_ = !timingFront_;
  }
  cue->durationMs = config_.cueMs;
  return true;
}

bool HapticEngine::stop(HapticReason reason, uint64_t timeUs, HapticCue* cue) {
  if (!active_) return false;
  active_ = false;
  *cue = HapticCue{};
  cue->event = HapticEvent::Off;
  cue->reason = reason;
  cue->errorClass = class_;
  cue->timeUs = timeUs;
  return true;
}

}  // namespace ead
