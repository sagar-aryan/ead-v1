#include "telemetry.h"

#include "calibration_service.h"
#include "gait_service.h"
#include "orientation.h"

#include <new>

#include <esp_heap_caps.h>
#include <freertos/semphr.h>
#include <freertos/task.h>

#include "config_v1.h"
#include "device.h"
#include "ead/msg_ring.h"

namespace telemetry {

namespace {

// Schema 6 streams about 23 kB/s (200 Hz frames and the native accelerometer
// of both sensors, DEC-021): 6 MB holds about 4.5 minutes of it.
constexpr size_t kPsramBytes = 6u * 1024u * 1024u;
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

QueueHandle_t s_frames = nullptr;
QueueHandle_t s_accel = nullptr;

// Every accelerometer sample as measured (§5.15), sent alongside the frames.
ead::AccelSample s_accelBatch[ead::kMaxAccelPerBatch];
size_t s_accelCount = 0;

void flushAccel() {
  if (s_accelCount == 0) return;
  static uint8_t payload[2 + ead::kMaxAccelPerBatch * ead::kAccelRecordSize];
  const size_t len = ead::encodeAccelBatchPayload(s_accelBatch, s_accelCount, payload,
                                                  sizeof payload);
  {
    Guard guard;
    s_ring->append(ead::MsgType::RawAccelBatch, s_accelBatch[0].timestamp_us, payload, len);
  }
  s_accelCount = 0;
}

void drainAccel() {
  while (xQueueReceive(s_accel, &s_accelBatch[s_accelCount], 0) == pdTRUE) {
    if (++s_accelCount == ead::kMaxAccelPerBatch) flushAccel();
  }
}

void processingTask(void*) {
  static ead::RawFrame batch[EAD_SAMPLE_BATCH_FRAMES];
  static uint8_t payload[2 + EAD_SAMPLE_BATCH_FRAMES * ead::kRawFrameSize];
  size_t count = 0;
  for (;;) {
    xQueueReceive(s_frames, &batch[count], portMAX_DELAY);
    drainAccel();
    // Every window that completes is adopted, the second as much as the first: a
    // recalibration after re-strapping was reported accepted and then ignored,
    // because this waited for the orientation to be invalid (audit I03). A
    // rejected one leaves no orientation, as the dashboard says.
    if (calibration::consume(batch[count])) orientation::adopt();
    orientation::process(&batch[count]);
    gait::consume(batch[count]);
    if (++count < EAD_SAMPLE_BATCH_FRAMES) continue;
    const size_t len = ead::encodeRawBatchPayload(batch, count, payload, sizeof payload);
    {
      Guard guard;
      s_ring->append(ead::MsgType::RawSampleBatch, batch[0].timestamp_us, payload, len);
    }
    flushAccel();
    // After the frames, so an event always follows the frame it refers to.
    gait::publish();
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

void startProcessing(QueueHandle_t frames, QueueHandle_t accelSamples) {
  s_frames = frames;
  s_accel = accelSamples;
  TaskHandle_t handle = nullptr;
  xTaskCreatePinnedToCore(processingTask, "processing", 6144, nullptr, 20, &handle, 1);
  device::registerTask(device::TaskRole::Processing, handle);
}

void append(ead::MsgType type, uint64_t timeUs, const uint8_t* payload, size_t len) {
  if (len == 0 || s_ring == nullptr) return;
  Guard guard;
  s_ring->append(type, timeUs, payload, len);
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
