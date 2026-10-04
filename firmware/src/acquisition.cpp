#include "acquisition.h"

#include <algorithm>
#include <atomic>
#include <cstdlib>

#include <driver/gpio.h>
#include <esp_timer.h>
#include <freertos/task.h>

#include "bno086.h"
#include "config_v1.h"
#include "device.h"

namespace acquisition {

namespace {

using bno086::kFoot;
using bno086::kShank;
using ead::sh2::Sample;

constexpr int16_t kQ15One = 32767;
// A live MEMS sensor always shows noise; 50 identical samples (0.5 s) means the
// data stopped updating.
constexpr uint32_t kFrozenFrames = 50;
constexpr TickType_t kWaitTicks = pdMS_TO_TICKS(10);
constexpr int64_t kStallUs = 30000;
constexpr int64_t kNoDataUs = 250000;
constexpr int64_t kPeriodUs = 1000000 / EAD_SAMPLE_HZ;
// Packets read per INT pass before checking the other sensor again.
constexpr int kReadsPerPass = 4;
constexpr uint32_t kCheckBit = 1u << 2;

TaskHandle_t s_task = nullptr;
portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;
int64_t s_intUs[2] = {0, 0};

ead::SensorCheck s_check[2] = {};
std::atomic<uint8_t> s_checkRequester{0};  // link flag of a pending request, 0 if none
std::atomic<uint8_t> s_checkDoneFor{0};

struct SensorState {
  int16_t accel[3];
  int16_t gyro[3];
  int64_t gyroUs;
  int64_t lastSampleUs;
  uint8_t gyroSeq;
  bool haveAccel;
  bool haveGyro;
  bool newAccel;
  bool newGyro;
  int16_t last[6];
  uint32_t identical;
};

SensorState s_sensor[2] = {};
QueueHandle_t s_frames = nullptr;
uint32_t s_frameIndex = 0;
uint8_t s_lastGyroSeq = 0;
int64_t s_lastFrameUs = 0;
bool s_sequenceKnown = false;  // false after start, a check, or a sensor reset

void IRAM_ATTR intAsserted(void* arg) {
  const uint32_t sensor = reinterpret_cast<uintptr_t>(arg);
  const int64_t now = esp_timer_get_time();
  portENTER_CRITICAL_ISR(&s_mux);
  s_intUs[sensor] = now;
  portEXIT_CRITICAL_ISR(&s_mux);
  BaseType_t woken = pdFALSE;
  if (s_task != nullptr) xTaskNotifyFromISR(s_task, 1u << sensor, eSetBits, &woken);
  if (woken == pdTRUE) portYIELD_FROM_ISR();
}

bool saturated(const int16_t* v, int16_t fullScale) {
  for (int i = 0; i < 3; i++) {
    if (std::abs(int32_t(v[i])) >= fullScale) return true;
  }
  return false;
}

void trackFrozen(SensorState& s, const int16_t* sample, uint16_t frozenFault) {
  bool same = true;
  for (int i = 0; i < 6; i++) same &= (sample[i] == s.last[i]);
  std::copy(sample, sample + 6, s.last);
  s.identical = same ? s.identical + 1 : 0;
  if (s.identical >= kFrozenFrames) {
    device::raiseFault(frozenFault);
  } else {
    device::clearFault(frozenFault);
  }
}

void fill(const SensorState& s, int16_t out[6]) {
  std::copy(s.accel, s.accel + 3, out);
  std::copy(s.gyro, s.gyro + 3, out + 3);
}

// One frame from the foot sample waiting to be sent, with the shank's latest.
void emitFrame() {
  SensorState& foot = s_sensor[kFoot];
  SensorState& shank = s_sensor[kShank];

  uint32_t advance = 1;
  if (s_sequenceKnown) {
    advance = uint8_t(foot.gyroSeq - s_lastGyroSeq);
    if (advance == 0) advance = 1;
  } else if (s_lastFrameUs != 0) {
    // The sequence restarted (a check or a sensor reset): keep the index
    // monotonic and let the elapsed time show how many frames are missing.
    const int64_t gap = (foot.gyroUs - s_lastFrameUs + kPeriodUs / 2) / kPeriodUs;
    advance = gap > 1 ? uint32_t(gap) : 1;
  }
  if (s_lastFrameUs != 0 && advance > 1) device::counters.framesDropped += advance - 1;
  s_frameIndex = s_lastFrameUs == 0 ? 0 : s_frameIndex + advance;
  s_lastGyroSeq = foot.gyroSeq;
  s_sequenceKnown = true;
  s_lastFrameUs = foot.gyroUs;

  ead::RawFrame f{};
  f.timestamp_us = uint64_t(foot.gyroUs);
  f.frame_index = s_frameIndex;
  fill(foot, f.foot);
  if (!foot.newAccel) f.status |= ead::kRawFootRepeated;
  if (!shank.haveAccel || !shank.haveGyro) {
    f.status |= ead::kRawShankReadFail;  // nothing from the shank yet: values are 0
  } else {
    fill(shank, f.shank);
    if (!shank.newGyro) {
      f.status |= ead::kRawShankRepeated;
      device::counters.shankRepeated++;
    }
    if (saturated(f.shank, EAD_ACCEL_FULL_SCALE)) f.status |= ead::kRawShankAccelSaturated;
    if (saturated(f.shank + 3, EAD_GYRO_FULL_SCALE)) f.status |= ead::kRawShankGyroSaturated;
    trackFrozen(shank, f.shank, ead::kFaultShankFrozen);
  }
  if (saturated(f.foot, EAD_ACCEL_FULL_SCALE)) f.status |= ead::kRawFootAccelSaturated;
  if (saturated(f.foot + 3, EAD_GYRO_FULL_SCALE)) f.status |= ead::kRawFootGyroSaturated;
  trackFrozen(foot, f.foot, ead::kFaultFootFrozen);
  // Identity until the processing task estimates orientation: a frame that
  // reaches the host with kRawOrientationValid clear carries these.
  f.q_foot[0] = kQ15One;
  f.q_shank[0] = kQ15One;

  foot.newAccel = foot.newGyro = false;
  shank.newAccel = shank.newGyro = false;
  if (xQueueSend(s_frames, &f, 0) != pdTRUE) device::counters.framesDropped++;
  device::counters.frameIndex = f.frame_index;
}

void consume(bno086::Sensor which, const Sample& sample) {
  SensorState& s = s_sensor[which];
  // Health uses arrival time on the device clock, not the sample's SH-2 time.
  s.lastSampleUs = esp_timer_get_time();
  // The gyroscope clocks the frames: it runs at the requested 100 Hz, while the
  // accelerometer runs at the nearest rate it has, 125 Hz (measured 2026-10-02),
  // so each frame carries the latest accelerometer sample, at most 8 ms old.
  if (sample.reportId == ead::sh2::kReportGyroscope) {
    // A second foot gyroscope sample before its frame went out: send the first.
    if (which == kFoot && s.newGyro && s.haveAccel) emitFrame();
    std::copy(sample.value, sample.value + 3, s.gyro);
    s.gyroUs = sample.timeUs;
    s.gyroSeq = sample.sequence;
    s.haveGyro = s.newGyro = true;
  } else if (sample.reportId == ead::sh2::kReportAccelerometer) {
    std::copy(sample.value, sample.value + 3, s.accel);
    s.haveAccel = s.newAccel = true;
  }
}

void service(bno086::Sensor which) {
  for (int i = 0; i < kReadsPerPass && bno086::intAsserted(which); i++) {
    portENTER_CRITICAL(&s_mux);
    const int64_t intUs = s_intUs[which];
    portEXIT_CRITICAL(&s_mux);
    Sample samples[8];
    const bno086::ReadResult r = bno086::read(which, intUs, samples, 8);
    if (r.bad) device::counters.busErrors++;
    if (r.resetSeen) {
      // The sensor reset itself (a brown-out, for example) and its reports are
      // off; turn them back on, as the MPU6500 build reconfigured after one.
      device::counters.imuReinits++;
      s_sequenceKnown = false;
      bno086::enableReports(which, EAD_REPORT_INTERVAL_US);
    }
    for (size_t k = 0; k < r.samples; k++) consume(which, samples[k]);
    SensorState& foot = s_sensor[kFoot];
    if (which == kFoot && foot.newGyro && foot.haveAccel) emitFrame();
  }
}

void applyCheck() {
  const uint16_t absent[2] = {ead::kFaultFootAbsent, ead::kFaultShankAbsent};
  const uint16_t config[2] = {ead::kFaultFootConfig, ead::kFaultShankConfig};
  for (uint8_t s = 0; s < 2; s++) {
    const uint16_t flags = s_check[s].flags;
    device::clearFault(absent[s] | config[s]);
    if ((flags & (ead::kCheckBooted | ead::kCheckReadValid)) !=
        (ead::kCheckBooted | ead::kCheckReadValid)) {
      device::raiseFault(absent[s]);
    } else if ((flags & (ead::kCheckProductId | ead::kCheckReports)) !=
               (ead::kCheckProductId | ead::kCheckReports)) {
      device::raiseFault(config[s]);
    }
  }
}

void runCheck() {
  ead::SensorCheck foot{};
  ead::SensorCheck shank{};
  bno086::resetAndCheck(&foot, &shank);
  portENTER_CRITICAL(&s_mux);
  s_check[kFoot] = foot;
  s_check[kShank] = shank;
  portEXIT_CRITICAL(&s_mux);
  applyCheck();
  const int64_t now = esp_timer_get_time();
  for (SensorState& s : s_sensor) {
    s.newAccel = s.newGyro = false;
    s.lastSampleUs = now;  // the no-data timers start over
  }
  s_sequenceKnown = false;
}

void watchHealth(int64_t now) {
  const uint16_t noData[2] = {ead::kFaultFootNoDataReady, ead::kFaultShankNoDataReady};
  for (uint8_t s = 0; s < 2; s++) {
    if (now - s_sensor[s].lastSampleUs > kNoDataUs) {
      device::raiseFault(noData[s]);
    } else {
      device::clearFault(noData[s]);
    }
  }
  if (now - s_sensor[kFoot].lastSampleUs > kStallUs) {
    device::raiseFault(ead::kFaultAcquisitionStalled);
  } else {
    device::clearFault(ead::kFaultAcquisitionStalled);
  }
}

void acquisitionTask(void*) {
  for (;;) {
    uint32_t bits = 0;
    xTaskNotifyWait(0, UINT32_MAX, &bits, kWaitTicks);
    const uint8_t requester = s_checkRequester.load();
    if (requester != 0) {
      runCheck();
      s_checkRequester = 0;
      s_checkDoneFor = requester;
      continue;
    }
    // Keep going while either INT stays asserted: a sensor with more queued
    // does not make a new edge.
    for (int pass = 0; pass < 4; pass++) {
      if (!bno086::intAsserted(kFoot) && !bno086::intAsserted(kShank)) break;
      service(kFoot);
      service(kShank);
    }
    watchHealth(esp_timer_get_time());
  }
}

}  // namespace

void start(QueueHandle_t frames) {
  s_frames = frames;
  bno086::beginBus();
  runCheck();

  // IRAM service: INT edges keep being timestamped while flash is busy.
  gpio_install_isr_service(ESP_INTR_FLAG_IRAM);
  const uint8_t pins[2] = {EAD_PIN_FOOT_INT, EAD_PIN_SHANK_INT};
  for (uintptr_t s = 0; s < 2; s++) {
    const gpio_num_t pin = gpio_num_t(pins[s]);
    gpio_set_intr_type(pin, GPIO_INTR_NEGEDGE);
    gpio_isr_handler_add(pin, intAsserted, reinterpret_cast<void*>(s));
  }

  xTaskCreatePinnedToCore(acquisitionTask, "acquisition", 4096, nullptr, 22, &s_task, 1);
  device::registerTask(device::TaskRole::Acquisition, s_task);
}

bool sensorAnswered(uint8_t sensor) {
  portENTER_CRITICAL(&s_mux);
  const bool answered = (s_check[sensor].flags & ead::kCheckProductId) != 0;
  portEXIT_CRITICAL(&s_mux);
  return answered;
}

void latestCheck(ead::SensorCheck* foot, ead::SensorCheck* shank) {
  portENTER_CRITICAL(&s_mux);
  *foot = s_check[kFoot];
  *shank = s_check[kShank];
  portEXIT_CRITICAL(&s_mux);
}

bool requestCheck(uint8_t linkFlag) {
  uint8_t none = 0;
  if (!s_checkRequester.compare_exchange_strong(none, linkFlag)) return false;
  s_checkDoneFor = 0;
  if (s_task != nullptr) xTaskNotify(s_task, kCheckBit, eSetBits);
  return true;
}

bool takeCheckDone(uint8_t linkFlag) {
  uint8_t expected = linkFlag;
  return s_checkDoneFor.compare_exchange_strong(expected, 0);
}

bool checkPending() { return s_checkRequester.load() != 0; }

}  // namespace acquisition
