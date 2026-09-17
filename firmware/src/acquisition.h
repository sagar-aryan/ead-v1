#pragma once
// 100 Hz acquisition clocked by the foot IMU's data-ready interrupt (doc 07 §2).
// Every frame gets the ESP32 monotonic timestamp of that interrupt and a frame
// index equal to the interrupt count, so a missed interrupt is a visible gap.

#include <cstdint>

#include <freertos/FreeRTOS.h>
#include <freertos/queue.h>

namespace acquisition {

struct BootReport {
  uint8_t whoFoot;
  uint8_t whoShank;
};

// Configures both IMUs, verifies both data-ready lines, raises any self-test
// faults, then starts the acquisition task feeding RawFrames into `frames`.
BootReport start(QueueHandle_t frames);

}  // namespace acquisition
