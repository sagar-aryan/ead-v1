#pragma once
// Runs the static calibration window on the device.
//
// The samples are the frames already being acquired, so calibration costs no
// extra I2C traffic and does not interrupt streaming: the host keeps receiving
// RAW_SAMPLE_BATCH throughout. The record lives in RAM only (no flash storage
// until M7) and is reported in STATUS and in SESSION_STOP.
//
// Frames arrive on the processing task and the record is read by the link
// tasks, so the shared state is guarded by a critical section rather than a
// mutex: the sections are a few loads and stores.

#include <cstdint>

#include "ead/calibration.h"
#include "ead/protocol.h"

namespace calibration {

/// Begins a window. Returns false if one is already running.
bool start(uint16_t durationMs);
/// Cancels a running window, discarding the partial record.
void cancel();

/// Feeds one acquired frame. Called from the processing task.
void consume(const ead::RawFrame& frame);

ead::CalibrationState state();
uint32_t samples();
uint16_t reject();

/// Copies the last record; false when no window has completed since boot.
bool record(ead::CalibrationRecord* out);

/// True exactly once per completed window, so the link can emit SESSION_STOP.
bool takeCompletion();

}  // namespace calibration
