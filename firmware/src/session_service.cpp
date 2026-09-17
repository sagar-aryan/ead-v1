#include "session_service.h"

#include <freertos/FreeRTOS.h>

#include "device.h"

namespace session {
namespace {

portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;

bool s_active = false;
ead::SessionKind s_kind = ead::SessionKind::Calibration;
bool s_haveReference = false;
ead::ReferenceProfile s_reference{};
ead::ReferenceBuilder s_builder;
ead::ErrorResult s_lastScore{};

/// What the confidence score needs to know about a cycle beyond its features.
///
/// Sensor quality: the cycle carries no per-frame flags, so the honest reading
/// available here is whether the device is currently faulting. Event quality:
/// a cycle that passed the temporal guards had both of its events accepted
/// inside their windows, which is what the subscore asks about.
ead::ConfidenceInputs inputsFor(const ead::GaitCycle& cycle) {
  ead::StatusInfo status;
  device::fillStatus(&status);
  const bool sensorsHealthy = status.faults == 0;
  return ead::ConfidenceInputs{sensorsHealthy ? 1.0f : 0.0f, cycle.valid ? 1.0f : 0.0f};
}

}  // namespace

bool start(ead::SessionKind kind, const ead::ReferenceProfile* reference) {
  portENTER_CRITICAL(&s_mux);
  const bool busy = s_active;
  if (!busy) {
    s_active = true;
    s_kind = kind;
    s_haveReference = reference != nullptr;
    if (reference != nullptr) s_reference = *reference;
    s_builder.reset();
    s_lastScore = ead::ErrorResult{};
  }
  portEXIT_CRITICAL(&s_mux);
  return !busy;
}

bool stop(ead::ReferenceProfile* profile, bool* wasCapture) {
  portENTER_CRITICAL(&s_mux);
  const bool capture = s_active && s_kind == ead::SessionKind::ReferenceCapture;
  s_active = false;
  portEXIT_CRITICAL(&s_mux);
  *wasCapture = capture;
  if (!capture) return false;
  // build() refuses below thirty cycles, which is the point: a reference that
  // rests on less evidence than doc 12 requires must not exist at all.
  return s_builder.build(profile);
}

const ead::ErrorResult* consume(const ead::GaitCycle& cycle) {
  portENTER_CRITICAL(&s_mux);
  const bool active = s_active;
  const ead::SessionKind kind = s_kind;
  const bool haveReference = s_haveReference;
  portEXIT_CRITICAL(&s_mux);
  if (!active) return nullptr;

  if (kind == ead::SessionKind::ReferenceCapture) {
    s_builder.add(cycle);
    return nullptr;
  }
  if (!haveReference) return nullptr;
  s_lastScore = ead::scoreCycle(cycle, s_reference, inputsFor(cycle));
  return &s_lastScore;
}

ead::SessionKind kind() {
  portENTER_CRITICAL(&s_mux);
  const ead::SessionKind value = s_kind;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

bool active() {
  portENTER_CRITICAL(&s_mux);
  const bool value = s_active;
  portEXIT_CRITICAL(&s_mux);
  return value;
}

uint16_t capturedCycles() {
  return s_builder.count();
}

}  // namespace session
