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
#include "ead/feed.h"

namespace acquisition {

namespace {

using bno086::kFoot;
using bno086::kShank;
using ead::sh2::Sample;

constexpr int16_t kQ15One = 32767;
constexpr int16_t kQ14One = 16384;
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

// Recent samples per sensor, oldest first. A frame's time falls between two
// accelerometer samples (250 Hz against 200 Hz frames), and the shank's own
// 200 Hz samples are a fraction of a period away from it.
constexpr size_t kHistory = 6;
// A frame waits for the samples after its time at most this long, measured by
// the foot samples that arrive meanwhile: three periods, so a shank packet
// already waiting behind the foot's is read first (TEST-055).
constexpr int64_t kMaxWaitUs = 3 * kPeriodUs;
constexpr size_t kMaxPending = 4;

struct Gyro {
  int64_t us;
  int16_t v[3];
  uint8_t seq;
};

struct Rv {
  int64_t us;
  int16_t v[4];  // real, i, j, k (Q14), as the frame carries it
};

template <typename T>
struct History {
  T items[kHistory];
  size_t count;

  void push(const T& item) {
    if (count == kHistory) {
      std::copy(items + 1, items + kHistory, items);
      count--;
    }
    items[count++] = item;
  }
  bool reached(int64_t us) const { return count > 0 && items[count - 1].us >= us; }
  /// The item nearest in time to `us`; null when empty.
  const T* nearest(int64_t us) const {
    const T* best = nullptr;
    for (size_t i = 0; i < count; i++) {
      if (best == nullptr || std::llabs(items[i].us - us) < std::llabs(best->us - us)) {
        best = &items[i];
      }
    }
    return best;
  }
};

struct SensorState {
  ead::AccelPoint accel[kHistory];
  size_t accelCount;
  History<Gyro> gyro;
  History<Rv> rv;
  int64_t lastSampleUs;
  int lastShankSeq;  // shank gyroscope sequence the previous frame used, -1 if none
  int16_t last[6];
  uint32_t identical;
};

SensorState s_sensor[2] = {};
QueueHandle_t s_frames = nullptr;
QueueHandle_t s_accelOut = nullptr;
uint32_t s_frameIndex = 0;
uint8_t s_lastGyroSeq = 0;
int64_t s_lastFrameUs = 0;
bool s_sequenceKnown = false;  // false after start, a check, or a sensor reset

// Foot gyroscope samples waiting for the samples after their time, oldest
// first: each becomes one frame.
Gyro s_pending[kMaxPending];
size_t s_pendingCount = 0;

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

bool accelReached(const SensorState& s, int64_t us) {
  return s.accelCount > 0 && s.accel[s.accelCount - 1].us >= us;
}

void setRv(const SensorState& s, int64_t us, int16_t out[4], uint16_t missingBit,
           uint16_t* status) {
  const Rv* rv = s.rv.nearest(us);
  if (rv != nullptr && std::llabs(rv->us - us) <= ead::kMaxSampleAgeUs) {
    std::copy(rv->v, rv->v + 4, out);
  } else {
    out[0] = kQ14One;  // identity: real first
    *status |= missingBit;
  }
}

// Sends the oldest waiting foot sample as a frame: both accelerometers brought
// to its time, the shank's gyroscope and both rotation vectors nearest to it.
void emitFrame() {
  SensorState& foot = s_sensor[kFoot];
  SensorState& shank = s_sensor[kShank];
  const Gyro clock = s_pending[0];
  std::copy(s_pending + 1, s_pending + s_pendingCount, s_pending);
  s_pendingCount--;

  uint32_t advance = 1;
  if (s_sequenceKnown) {
    advance = uint8_t(clock.seq - s_lastGyroSeq);
    if (advance == 0) advance = 1;
  } else if (s_lastFrameUs != 0) {
    // The sequence restarted (a check or a sensor reset): keep the index
    // monotonic and let the elapsed time show how many frames are missing.
    const int64_t gap = (clock.us - s_lastFrameUs + kPeriodUs / 2) / kPeriodUs;
    advance = gap > 1 ? uint32_t(gap) : 1;
  }
  if (s_lastFrameUs != 0 && advance > 1) device::counters.framesDropped += advance - 1;
  s_frameIndex = s_lastFrameUs == 0 ? 0 : s_frameIndex + advance;
  s_lastGyroSeq = clock.seq;
  s_sequenceKnown = true;
  s_lastFrameUs = clock.us;

  ead::RawFrame f{};
  f.timestamp_us = uint64_t(clock.us);
  f.frame_index = s_frameIndex;
  if (!ead::interpolateAccel(foot.accel, foot.accelCount, clock.us, f.foot)) {
    f.status |= ead::kRawFootAccelHeld;
  }
  std::copy(clock.v, clock.v + 3, f.foot + 3);
  setRv(foot, clock.us, f.rv_foot, ead::kRawFootRvMissing, &f.status);
  const Gyro* shankGyro = shank.gyro.nearest(clock.us);
  if (shank.accelCount == 0 || shankGyro == nullptr) {
    f.status |= ead::kRawShankReadFail;  // nothing from the shank yet: values are 0
  } else {
    if (!ead::interpolateAccel(shank.accel, shank.accelCount, clock.us, f.shank)) {
      f.status |= ead::kRawShankAccelHeld;
    }
    std::copy(shankGyro->v, shankGyro->v + 3, f.shank + 3);
    if (shankGyro->seq == shank.lastShankSeq) {
      f.status |= ead::kRawShankRepeated;
      device::counters.shankRepeated++;
    }
    shank.lastShankSeq = shankGyro->seq;
    if (saturated(f.shank, EAD_ACCEL_FULL_SCALE)) f.status |= ead::kRawShankAccelSaturated;
    if (saturated(f.shank + 3, EAD_GYRO_FULL_SCALE)) f.status |= ead::kRawShankGyroSaturated;
    trackFrozen(shank, f.shank, ead::kFaultShankFrozen);
  }
  setRv(shank, clock.us, f.rv_shank, ead::kRawShankRvMissing, &f.status);
  if (saturated(f.foot, EAD_ACCEL_FULL_SCALE)) f.status |= ead::kRawFootAccelSaturated;
  if (saturated(f.foot + 3, EAD_GYRO_FULL_SCALE)) f.status |= ead::kRawFootGyroSaturated;
  trackFrozen(foot, f.foot, ead::kFaultFootFrozen);
  // Identity until the processing task fills the segment orientations: a frame
  // that reaches the host with kRawOrientationValid clear carries these.
  f.q_foot[0] = kQ15One;
  f.q_shank[0] = kQ15One;

  if (xQueueSend(s_frames, &f, 0) != pdTRUE) device::counters.framesDropped++;
  device::counters.frameIndex = f.frame_index;
}

// Sends every waiting frame whose samples have all arrived: from each sensor
// one at or after its time (a shank that never sent anything is not waited
// for). One that waited too long goes out with what there is, marked.
void tryEmit() {
  const SensorState& foot = s_sensor[kFoot];
  const SensorState& shank = s_sensor[kShank];
  while (s_pendingCount > 0) {
    const int64_t us = s_pending[0].us;
    const bool shankSilent = shank.accelCount == 0 && shank.gyro.count == 0;
    const bool ready = accelReached(foot, us) && foot.rv.reached(us) &&
                       (shankSilent || (accelReached(shank, us) && shank.gyro.reached(us) &&
                                        shank.rv.reached(us)));
    const bool overdue = s_pending[s_pendingCount - 1].us - us >= kMaxWaitUs;
    if (!ready && !overdue && s_pendingCount < kMaxPending) return;
    emitFrame();
  }
}

void consume(bno086::Sensor which, const Sample& sample) {
  SensorState& s = s_sensor[which];
  // Health uses arrival time on the device clock, not the sample's SH-2 time.
  s.lastSampleUs = esp_timer_get_time();
  switch (sample.reportId) {
    case ead::sh2::kReportGyroscope: {
      Gyro g{sample.timeUs, {sample.value[0], sample.value[1], sample.value[2]}, sample.sequence};
      if (which == kFoot) {
        if (s_pendingCount == kMaxPending) emitFrame();  // never lose a foot sample
        s_pending[s_pendingCount++] = g;
      }
      s.gyro.push(g);
      break;
    }
    case ead::sh2::kReportAccelerometer: {
      if (s.accelCount == kHistory) {
        std::copy(s.accel + 1, s.accel + kHistory, s.accel);
        s.accelCount--;
      }
      ead::AccelPoint& p = s.accel[s.accelCount++];
      p.us = sample.timeUs;
      std::copy(sample.value, sample.value + 3, p.v);
      // Every sample as measured, for the native-rate stream (§5.15). A full
      // queue drops it; its sequence number shows the gap on the host.
      ead::AccelSample out{uint64_t(sample.timeUs), uint8_t(which), sample.sequence,
                           {sample.value[0], sample.value[1], sample.value[2]}};
      xQueueSend(s_accelOut, &out, 0);
      break;
    }
    case ead::sh2::kReportGameRotationVector:
      // SH-2 reports i, j, k, real; the frame carries real first.
      s.rv.push(Rv{sample.timeUs, {sample.value[3], sample.value[0], sample.value[1],
                                   sample.value[2]}});
      break;
    default:
      break;
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
    // After the whole packet: a rotation vector and accelerometer sample can
    // follow the gyroscope sample in it.
    tryEmit();
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
    // Samples from before the reset bracket nothing.
    s.accelCount = 0;
    s.gyro.count = 0;
    s.rv.count = 0;
    s.lastShankSeq = -1;
    s.lastSampleUs = now;  // the no-data timers start over
  }
  s_pendingCount = 0;
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

void start(QueueHandle_t frames, QueueHandle_t accelSamples) {
  s_frames = frames;
  s_accelOut = accelSamples;
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
