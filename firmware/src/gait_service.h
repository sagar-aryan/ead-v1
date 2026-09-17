#pragma once
// Runs the gait engine on the frame path and publishes what it finds.
//
// Gait needs orientation, which needs calibration, so this produces nothing
// until the device has been calibrated — the same rule as orientation, for the
// same reason: a cadence derived from an uncalibrated estimate would look like a
// measurement.
//
// Events and cycles are appended to the durable ring as EVENT_BATCH and
// STEP_BATCH, so a host that misses them can ask for them again by sequence.

#include <cstdint>

#include "ead/protocol.h"

namespace gait {

/// Feeds one frame. Called from the processing task, after orientation.
void consume(const ead::RawFrame& frame);

/// Publishes anything the engine produced. Called on the processing task, after
/// a batch of frames has been consumed, so a batch is written at most once.
void publish();

uint8_t state();
uint32_t cyclesCompleted();

/// Restarts the engine, e.g. when a new calibration is adopted.
void reset();

}  // namespace gait
