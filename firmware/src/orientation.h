#pragma once
// Runs a Mahony estimator per sensor and fills each frame's quaternions.
//
// Orientation needs calibration: the estimator's input is the bias-removed
// angular rate, and it starts from the alignment the calibration measured so it
// does not spend the first seconds of a recording converging. Without a record
// the quaternions stay identity and the frame's orientation-valid bit stays
// clear, rather than shipping a number that looks like an answer.
//
// Called from the processing task, on the frame path only: the frame's stored
// counts are never modified (doc 04 §7).

#include <cstdint>

#include "ead/protocol.h"

namespace orientation {

/// Re-reads the calibration record and restarts both estimators from it.
/// Called when a calibration window completes.
void adopt();

/// Estimates orientation for one frame and writes it into the frame.
void process(ead::RawFrame* frame);

/// True while both estimators are running on a calibrated input.
bool valid();

}  // namespace orientation
