#include "telemetry.h"

#include <new>

#include <esp_heap_caps.h>
#include <freertos/semphr.h>
#include <freertos/task.h>

#include "config_v1.h"
#include "device.h"
#include "ead/msg_ring.h"

namespace telemetry {

namespace {

// 4 MB holds about 12 minutes of raw batches (~5.6 kB/s).
constexpr size_t kPsramBytes = 4u * 1024u * 1024u;
constexpr size_t kPsramSlots = 16384;
constexpr size_t kInternalBytes = 64u * 1024u;
constexpr size_t kInternalSlots = 256;

SemaphoreHandle_t s_lock = nullptr;
ead::MsgRing* s_ring = nullptr;
uint8_t s_ringObject[sizeof(ead::MsgRing)];

class Guard {
 public:
  Guard() { xSemaphoreTake(s_lock, portMAX_DELAY); }
  ~Guard() { xSemaphoreGive(s_lock); }
};

void processingTask(void* arg) {
  const QueueHandle_t frames = static_cast<QueueHandle_t>(arg);
  static ead::RawFrame batch[EAD_SAMPLE_BATCH_FRAMES];
  static uint8_t payload[2 + EAD_SAMPLE_BATCH_FRAMES * ead::kRawFrameSize];
  size_t count = 0;
  for (;;) {
    xQueueReceive(frames, &batch[count], portMAX_DELAY);
    if (++count < EAD_SAMPLE_BATCH_FRAMES) continue;
    const size_t len = ead::encodeRawBatchPayload(batch, count, payload, sizeof payload);
    {
      Guard guard;
      s_ring->append(ead::MsgType::RawSampleBatch, batch[0].timestamp_us, payload, len);
    }
    count = 0;
  }
}

}  // namespace

bool begin() {
  s_lock = xSemaphoreCreateMutex();
  auto* storage = static_cast<uint8_t*>(heap_caps_malloc(kPsramBytes, MALLOC_CAP_SPIRAM));
  auto* slots = static_cast<ead::MsgRing::Slot*>(
      heap_caps_malloc(kPsramSlots * sizeof(ead::MsgRing::Slot), MALLOC_CAP_SPIRAM));
  if (storage != nullptr && slots != nullptr) {
    s_ring = new (s_ringObject) ead::MsgRing(storage, kPsramBytes, slots, kPsramSlots);
    return true;
  }
  heap_caps_free(storage);
  heap_caps_free(slots);
  storage = static_cast<uint8_t*>(heap_caps_malloc(kInternalBytes, MALLOC_CAP_INTERNAL));
  slots = static_cast<ead::MsgRing::Slot*>(
      heap_caps_malloc(kInternalSlots * sizeof(ead::MsgRing::Slot), MALLOC_CAP_INTERNAL));
  s_ring = new (s_ringObject) ead::MsgRing(storage, kInternalBytes, slots, kInternalSlots);
  return false;
}

void startProcessing(QueueHandle_t frames) {
  TaskHandle_t handle = nullptr;
  xTaskCreatePinnedToCore(processingTask, "processing", 6144, frames, 20, &handle, 1);
  device::registerTask(device::TaskRole::Processing, handle);
}

size_t read(uint32_t seq, uint8_t* out, size_t cap) {
  Guard guard;
  return s_ring->read(seq, out, cap);
}

size_t lengthOf(uint32_t seq) {
  Guard guard;
  return s_ring->lengthOf(seq);
}

void window(uint32_t* oldest, uint32_t* last) {
  Guard guard;
  *oldest = s_ring->oldest();
  *last = s_ring->last();
}

}  // namespace telemetry
