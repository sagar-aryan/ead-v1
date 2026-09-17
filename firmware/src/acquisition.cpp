#include "acquisition.h"

#include <algorithm>

#include <driver/gpio.h>
#include <esp_timer.h>
#include <freertos/task.h>

#include "config_v1.h"
#include "device.h"
#include "ead/protocol.h"
#include "imu.h"

namespace acquisition {

namespace {

constexpr int16_t kQ15One = 32767;
// A live MEMS sensor always shows noise; 50 identical samples (0.5 s) means the
// data registers stopped updating.
constexpr uint32_t kFrozenFrames = 50;
constexpr uint32_t kPowerCheckFrames = 100;
constexpr TickType_t kStallTimeout = pdMS_TO_TICKS(30);
constexpr TickType_t kDataReadyGuardTicks = 2;

TaskHandle_t s_task = nullptr;
portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;
uint32_t s_footCount = 0;
uint32_t s_shankCount = 0;
int64_t s_footTimeUs = 0;

void IRAM_ATTR footDataReady(void*) {
  const int64_t now = esp_timer_get_time();
  portENTER_CRITICAL_ISR(&s_mux);
  s_footCount++;
  s_footTimeUs = now;
  portEXIT_CRITICAL_ISR(&s_mux);
  BaseType_t woken = pdFALSE;
  if (s_task != nullptr) vTaskNotifyGiveFromISR(s_task, &woken);
  if (woken == pdTRUE) portYIELD_FROM_ISR();
}

void IRAM_ATTR shankDataReady(void*) {
  portENTER_CRITICAL_ISR(&s_mux);
  s_shankCount++;
  portEXIT_CRITICAL_ISR(&s_mux);
}

struct SensorWatch {
  uint8_t addr;
  uint16_t absentFault;
  uint16_t configFault;
  uint16_t frozenFault;
  int16_t last[6];
  uint32_t identical;
};

bool saturated(const int16_t* v) {
  for (int i = 0; i < 3; i++) {
    if (v[i] == INT16_MAX || v[i] == INT16_MIN) return true;
  }
  return false;
}

void applyConfigResult(uint16_t absentFault, uint16_t configFault, const imu::ConfigResult& r) {
  device::clearFault(absentFault | configFault);
  if (!r.present) {
    device::raiseFault(absentFault);
  } else if (!r.configOk) {
    device::raiseFault(configFault);
  }
}

void trackFrozen(SensorWatch& s, const int16_t* sample) {
  bool same = true;
  for (int i = 0; i < 6; i++) same &= (sample[i] == s.last[i]);
  std::copy(sample, sample + 6, s.last);
  s.identical = same ? s.identical + 1 : 0;
  if (s.identical >= kFrozenFrames) {
    device::raiseFault(s.frozenFault);
  } else {
    device::clearFault(s.frozenFault);
  }
}

// Reads one sensor into `out`, updating counters, status bits and watches.
void readSensor(SensorWatch& s, int16_t out[6], uint16_t* status, uint16_t readFail,
                uint16_t repeated, uint16_t accelSat, uint16_t gyroSat, bool countRepeats) {
  bool fresh = false;
  if (!imu::readSample(s.addr, out, &fresh)) {
    *status |= readFail;
    device::counters.i2cErrors++;
    return;
  }
  if (!fresh) {
    *status |= repeated;
    if (countRepeats) device::counters.shankRepeated++;
  }
  if (saturated(out)) *status |= accelSat;
  if (saturated(out + 3)) *status |= gyroSat;
  trackFrozen(s, out);
}

void checkPower(SensorWatch& s) {
  if (imu::powerOk(s.addr)) return;
  device::counters.imuReinits++;
  applyConfigResult(s.absentFault, s.configFault, imu::configure(s.addr));
}

void acquisitionTask(void* arg) {
  const QueueHandle_t frames = static_cast<QueueHandle_t>(arg);
  SensorWatch foot{EAD_FOOT_MPU_ADDR, ead::kFaultFootAbsent, ead::kFaultFootConfig,
                   ead::kFaultFootFrozen, {}, 0};
  SensorWatch shank{EAD_SHANK_MPU_ADDR, ead::kFaultShankAbsent, ead::kFaultShankConfig,
                    ead::kFaultShankFrozen, {}, 0};
  uint32_t sincePowerCheck = 0;
  portENTER_CRITICAL(&s_mux);
  uint32_t lastCount = s_footCount;
  portEXIT_CRITICAL(&s_mux);

  for (;;) {
    if (ulTaskNotifyTake(pdTRUE, kStallTimeout) == 0) {
      device::raiseFault(ead::kFaultAcquisitionStalled);
      continue;
    }
    device::clearFault(ead::kFaultAcquisitionStalled);

    portENTER_CRITICAL(&s_mux);
    const uint32_t count = s_footCount;
    const int64_t timeUs = s_footTimeUs;
    portEXIT_CRITICAL(&s_mux);
    if (count - lastCount > 1) device::counters.framesDropped += count - lastCount - 1;
    lastCount = count;

    ead::RawFrame f{};
    f.timestamp_us = uint64_t(timeUs);
    f.frame_index = count - 1;
    // Guard after the data-ready edge. Reading immediately fails ~0.6% of
    // transactions on whichever sensor is read first (docs/problems.md
    // PROB-007). vTaskDelay(2) guarantees at least one full 1 ms tick and
    // yields core 1 to the processing task, unlike a busy-wait.
    vTaskDelay(kDataReadyGuardTicks);
    readSensor(foot, f.foot, &f.status, ead::kRawFootReadFail, ead::kRawFootRepeated,
               ead::kRawFootAccelSaturated, ead::kRawFootGyroSaturated, false);
    readSensor(shank, f.shank, &f.status, ead::kRawShankReadFail, ead::kRawShankRepeated,
               ead::kRawShankAccelSaturated, ead::kRawShankGyroSaturated, true);
    // Identity until the processing task estimates orientation: a frame that
    // reaches the host with kRawOrientationValid clear carries these.
    f.q_foot[0] = kQ15One;
    f.q_shank[0] = kQ15One;

    if (xQueueSend(frames, &f, 0) != pdTRUE) device::counters.framesDropped++;
    device::counters.frameIndex = f.frame_index;

    if (++sincePowerCheck >= kPowerCheckFrames) {
      sincePowerCheck = 0;
      checkPower(foot);
      checkPower(shank);
    }
  }
}

void installDataReady(gpio_num_t pin, gpio_isr_t handler) {
  gpio_set_direction(pin, GPIO_MODE_INPUT);
  gpio_set_intr_type(pin, GPIO_INTR_POSEDGE);
  gpio_isr_handler_add(pin, handler, nullptr);
}

}  // namespace

static imu::ConfigResult configureWithRetry(uint8_t addr) {
  constexpr int kAttempts = 3;
  imu::ConfigResult result{};
  for (int attempt = 0; attempt < kAttempts; attempt++) {
    result = imu::configure(addr);
    if (result.present && result.configOk) break;
    vTaskDelay(pdMS_TO_TICKS(20));
  }
  return result;
}

BootReport start(QueueHandle_t frames) {
  const imu::ConfigResult footCfg = configureWithRetry(EAD_FOOT_MPU_ADDR);
  const imu::ConfigResult shankCfg = configureWithRetry(EAD_SHANK_MPU_ADDR);
  applyConfigResult(ead::kFaultFootAbsent, ead::kFaultFootConfig, footCfg);
  applyConfigResult(ead::kFaultShankAbsent, ead::kFaultShankConfig, shankCfg);

  // IRAM service: data-ready interrupts keep firing while flash is busy.
  gpio_install_isr_service(ESP_INTR_FLAG_IRAM);
  installDataReady(gpio_num_t(EAD_PIN_FOOT_IMU_INT), footDataReady);
  installDataReady(gpio_num_t(EAD_PIN_SHANK_IMU_INT), shankDataReady);

  // Self-test: both lines must pulse at ~100 Hz; 250 ms should give ~25 each.
  vTaskDelay(pdMS_TO_TICKS(250));
  portENTER_CRITICAL(&s_mux);
  const uint32_t footPulses = s_footCount;
  const uint32_t shankPulses = s_shankCount;
  s_footCount = 0;
  s_shankCount = 0;
  portEXIT_CRITICAL(&s_mux);
  if (footPulses < 20) device::raiseFault(ead::kFaultFootNoDataReady);
  if (shankPulses < 20) device::raiseFault(ead::kFaultShankNoDataReady);

  xTaskCreatePinnedToCore(acquisitionTask, "acquisition", 4096, frames, 22, &s_task, 1);
  device::registerTask(device::TaskRole::Acquisition, s_task);
  return BootReport{footCfg.who, shankCfg.who};
}

}  // namespace acquisition
