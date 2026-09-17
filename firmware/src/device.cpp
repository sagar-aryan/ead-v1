#include "device.h"

#include "calibration_service.h"
#include "gait_service.h"

#include <algorithm>

#include <esp_heap_caps.h>
#include <esp_random.h>
#include <esp_system.h>
#include <esp_wifi.h>
#include <mbedtls/sha256.h>

#include "config_v1.h"
#include "ead/config_section.h"
#include "fw_version.h"
#include "telemetry.h"

namespace device {

Counters counters;

namespace {

std::atomic<uint16_t> s_faults{0};
std::atomic<uint8_t> s_linkFlags{0};
std::atomic<uint16_t> s_wifiStackFree{0};
std::atomic<bool> s_booted{false};

uint32_t s_bootId = 0;
uint8_t s_mac[6] = {};
uint8_t s_resetReason = 0;
uint8_t s_whoFoot = 0;
uint8_t s_whoShank = 0;
uint8_t s_capabilities = 0;
uint8_t s_configSection[256] = {};
size_t s_configLen = 0;
uint8_t s_configSha256[32] = {};

TaskHandle_t s_tasks[3] = {};

uint8_t currentState(uint16_t faults) {
  if (!s_booted) return ead::kStateSelfTest;
  if (faults) return ead::kStateFault;
  if (calibration::state() == ead::CalibrationState::Collecting) return ead::kStateCalibrating;
  return ead::kStateReady;
}

uint16_t stackFree(TaskRole role) {
  const TaskHandle_t handle = s_tasks[static_cast<size_t>(role)];
  // ESP-IDF reports the high-water mark in bytes.
  return handle ? uint16_t(uxTaskGetStackHighWaterMark(handle)) : 0;
}

}  // namespace

void raiseFault(uint16_t faults) { s_faults.fetch_or(faults); }
void clearFault(uint16_t faults) { s_faults.fetch_and(uint16_t(~faults)); }

void setLinkActive(uint8_t linkFlag, bool active) {
  if (active) {
    s_linkFlags.fetch_or(linkFlag);
  } else {
    s_linkFlags.fetch_and(uint8_t(~linkFlag));
  }
}

void setWifiStackFree(uint16_t bytes) { s_wifiStackFree = bytes; }

void registerTask(TaskRole role, TaskHandle_t handle) {
  s_tasks[static_cast<size_t>(role)] = handle;
}

void captureIdentity(uint8_t whoFoot, uint8_t whoShank, bool psramRing) {
  s_bootId = esp_random();
  esp_efuse_mac_get_default(s_mac);
  s_resetReason = uint8_t(esp_reset_reason());
  s_whoFoot = whoFoot;
  s_whoShank = whoShank;
  s_capabilities = (EAD_HAPTICS_FITTED ? ead::kCapHapticsFitted : 0) |
                   (psramRing ? ead::kCapPsramRing : 0);
  s_configLen = ead::encodeConfigSection(s_configSection, sizeof s_configSection);
  mbedtls_sha256_ret(s_configSection, s_configLen, s_configSha256, 0);
}

void markBooted() { s_booted = true; }

const uint8_t* mac() { return s_mac; }

void fillHello(ead::HelloInfo* info) {
  info->schema = ead::kSchemaVersion;
  info->device_state = currentState(s_faults.load());
  info->reset_reason = s_resetReason;
  info->boot_id = s_bootId;
  std::copy(s_mac, s_mac + 6, info->mac);
  info->who_foot = s_whoFoot;
  info->who_shank = s_whoShank;
  info->capabilities = s_capabilities;
  std::copy(s_configSha256, s_configSha256 + 32, info->config_sha256);
  telemetry::window(&info->oldest_seq, &info->last_seq);
  info->fw_version = EAD_FW_VERSION;
}

void fillStatus(ead::StatusInfo* s) {
  const uint16_t faults = s_faults.load();
  s->device_state = currentState(faults);
  s->link_flags = s_linkFlags.load();
  s->faults = faults;
  s->frame_index = counters.frameIndex.load();
  s->frames_dropped = counters.framesDropped.load();
  s->shank_repeated = counters.shankRepeated.load();
  s->i2c_errors = counters.i2cErrors.load();
  s->imu_reinits = counters.imuReinits.load();
  telemetry::window(&s->oldest_seq, &s->last_seq);

  s->ap_stations = 0;
  s->ap_rssi_dbm = 0;
  wifi_sta_list_t stations;
  if (esp_wifi_ap_get_sta_list(&stations) == ESP_OK) {
    s->ap_stations = uint8_t(stations.num);
    for (int i = 0; i < stations.num; i++) {
      if (i == 0 || stations.sta[i].rssi > s->ap_rssi_dbm) s->ap_rssi_dbm = stations.sta[i].rssi;
    }
  }

  s->heap_free_min = uint32_t(heap_caps_get_minimum_free_size(MALLOC_CAP_INTERNAL));
  s->stack_free_acquisition = stackFree(TaskRole::Acquisition);
  s->gait_state = gait::state();
  s->cycles_completed = gait::cyclesCompleted();
  s->calibration_state = uint8_t(calibration::state());
  s->calibration_samples = calibration::samples();
  s->calibration_reject = calibration::reject();
  s->stack_free_processing = stackFree(TaskRole::Processing);
  s->stack_free_usb = stackFree(TaskRole::Usb);
  s->stack_free_wifi = s_wifiStackFree.load();
}

size_t encodeConfigResponse(uint8_t* out, size_t cap) {
  return ead::encodeConfigPayload(ead::kConfigFormat, s_configSha256, s_configSection, s_configLen,
                                  out, cap);
}

}  // namespace device
