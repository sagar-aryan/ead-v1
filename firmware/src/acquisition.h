#pragma once
// 100 Hz acquisition from the two BNO086 sensors (doc 07 §2, DEC-017). Each
// sensor reports when it has data and asserts its INT; the foot sensor's
// gyroscope clocks the frames. A frame carries the foot sample's own time
// (from the SH-2 timestamps) and an index that advances with the foot gyroscope
// sequence, so a lost report is a visible gap.

#include <cstdint>

#include <freertos/FreeRTOS.h>
#include <freertos/queue.h>

#include "ead/protocol.h"

namespace acquisition {

/// Resets and checks both sensors, raises any faults, then starts the task
/// feeding RawFrames into `frames`.
void start(QueueHandle_t frames);

/// The sensor (0 foot, 1 shank) answered its product ID in the latest check.
bool sensorAnswered(uint8_t sensor);

void latestCheck(ead::SensorCheck* foot, ead::SensorCheck* shank);

/// Asks the task to reset and check both sensors again (SERVICE_TEST). False if
/// a check is already waiting. Frames stop for about two seconds meanwhile.
bool requestCheck(uint8_t linkFlag);
/// True once, for the link that asked, when the requested check has finished.
bool takeCheckDone(uint8_t linkFlag);
bool checkPending();

}  // namespace acquisition
