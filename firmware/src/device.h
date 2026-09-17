#pragma once
// Device-wide state shared between tasks: identity captured at boot, fault
// flags, acquisition counters, and the builders for HELLO, STATUS and
// CONFIG_GET payloads.

#include <atomic>
#include <cstddef>
#include <cstdint>

#include <freertos/FreeRTOS.h>
#include <freertos/task.h>

#include "ead/protocol.h"

namespace device {

struct Counters {
  std::atomic<uint32_t> frameIndex{0};
  std::atomic<uint32_t> framesDropped{0};
  std::atomic<uint32_t> shankRepeated{0};
  std::atomic<uint32_t> i2cErrors{0};
  std::atomic<uint32_t> imuReinits{0};
};

extern Counters counters;

void raiseFault(uint16_t faults);
void clearFault(uint16_t faults);
void setLinkActive(uint8_t linkFlag, bool active);
void setWifiStackFree(uint16_t bytes);

enum class TaskRole : uint8_t { Acquisition, Processing, Usb };
void registerTask(TaskRole role, TaskHandle_t handle);

// Called once after self-test, before any link starts.
void captureIdentity(uint8_t whoFoot, uint8_t whoShank, bool psramRing);
void markBooted();

void fillHello(ead::HelloInfo* info);
void fillStatus(ead::StatusInfo* status);
size_t encodeConfigResponse(uint8_t* out, size_t cap);
const uint8_t* mac();

}  // namespace device
