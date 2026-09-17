#pragma once
// Durable telemetry: the processing task turns acquired frames into
// RAW_SAMPLE_BATCH messages and appends them to a mutex-protected message ring
// (PSRAM when available) that every link reads from by sequence number.

#include <cstddef>
#include <cstdint>

#include <freertos/FreeRTOS.h>
#include <freertos/queue.h>

#include "ead/protocol.h"

namespace telemetry {

// Allocates the ring. Returns true when it lives in PSRAM; otherwise a small
// internal-RAM ring is used and backfill reaches back only a few seconds.
bool begin();

void startProcessing(QueueHandle_t frames);

size_t read(uint32_t seq, uint8_t* out, size_t cap);
size_t lengthOf(uint32_t seq);
// Oldest and newest stored sequence numbers (0, 0 when empty).
void window(uint32_t* oldest, uint32_t* last);

}  // namespace telemetry
