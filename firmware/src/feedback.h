#pragma once
// Error-driven haptic feedback (doc 06, DEC-023): the dashboard's master switch,
// the haptic engine, the motors and the HAPTIC_BATCH log.
//
// Cues run only in an EVALUATION session with the switch on. The switch is off
// at every boot, so a device that restarts never resumes buzzing on its own.

#include <cstdint>

#include "ead/error_engine.h"
#include "ead/gait.h"

namespace feedback {

/// The master switch, from the link tasks. Off stops the motors at once; the
/// episode's log record follows on the processing task.
void setEnabled(bool on);
bool enabled();
/// A feedback episode is running.
bool episodeActive();

// The rest runs on the processing task.

/// A completed cycle and its score, or null when the session did not score it.
void onCycle(const ead::GaitCycle& cycle, const ead::ErrorResult* score);
/// A frame that cannot be trusted (read failure, gap, no orientation): any
/// episode ends and the motors stop (doc 06 §12).
void fault(uint64_t timeUs);
/// Each frame: ends an episode the switch or the end of the session closed.
void poll(uint64_t timeUs);
/// Appends the queued records as HAPTIC_BATCH.
void publish();

}  // namespace feedback
