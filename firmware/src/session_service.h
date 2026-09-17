#pragma once
// Session kinds and the reference workflow (doc 12 §2–§3).
//
// The clinical requirement this serves is that every patient is compared with
// their own walking, captured in a short calibration walk and saved for next
// time — not with a healthy-population pattern. So a reference is built from at
// least thirty of that patient's own valid cycles, and once it has been used it
// is never modified: a bad session cannot teach the device bad gait.
//
// The device builds and applies the profile (DEC-012); the dashboard versions,
// stores and locks it, and hands it back at the start of a check or evaluation.

#include <cstdint>

#include "ead/error_engine.h"
#include "ead/gait.h"
#include "ead/protocol.h"
#include "ead/reference.h"

namespace session {

/// Begins a session. `reference` is required for a check or an evaluation and
/// ignored otherwise. Returns false when one is already running.
bool start(ead::SessionKind kind, const ead::ReferenceProfile* reference);

/// Ends the session. For a capture, fills `profile` and returns true only when
/// at least thirty valid cycles were collected.
bool stop(ead::ReferenceProfile* profile, bool* wasCapture);

/// Feeds a completed cycle. Returns the score when a reference is loaded.
const ead::ErrorResult* consume(const ead::GaitCycle& cycle);

ead::SessionKind kind();
bool active();
/// Valid cycles collected in a reference capture so far.
uint16_t capturedCycles();

}  // namespace session
