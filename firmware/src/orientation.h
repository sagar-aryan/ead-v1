#pragma once
// Fills each frame's segment orientations from the BNO086 game rotation
// vectors it carries (DEC-021): the chip's own fusion, carried into anatomical
// axes by the mount map and through the calibration's alignment.
//
// Orientation needs calibration, for the alignment. Without a record the
// segment quaternions stay identity and the frame's orientation-valid bit stays
// clear, rather than shipping a number that looks like an answer.
//
// Called from the processing task, on the frame path only: the frame's stored
// counts and rotation vectors are never modified (doc 04 §7).

#include <cstdint>

#include "ead/protocol.h"

namespace orientation {

/// Re-reads the calibration record. Called when a calibration window completes.
void adopt();

/// Writes the frame's segment orientations from its rotation vectors.
void process(ead::RawFrame* frame);

}  // namespace orientation
