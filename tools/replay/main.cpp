// Replays a recorded .eadlog through the device's own pipeline on the host.
//
// The point is to tune and debug gait detection against real signals without
// standing on one leg next to the bench: the same ead_core code that runs on the
// device runs here, so a threshold changed here is a threshold changed there.
//
// The recording holds raw frames only, so calibration is recomputed from the
// still period at the start — the same accumulator the device uses — and
// orientation is re-estimated rather than taken from the frame's quaternions.
// That makes the replay a check on the whole chain, not just on gait.
//
// Build:
//   g++ -std=gnu++17 -O2 -I firmware/lib/ead_core/src -I firmware/include \
//       -o /tmp/eadreplay tools/replay/main.cpp \
//       firmware/lib/ead_core/src/ead/{calibration,mahony,gait}.cpp
// Run:
//   /tmp/eadreplay recording.eadlog [--still-seconds 3] [--trace]

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "config_v1.h"
#include "ead/calibration.h"
#include "ead/protocol.h"
#include "ead/gait.h"
#include "ead/mahony.h"

namespace {

struct Frame {
  uint64_t timeUs;
  uint32_t index;
  int16_t foot[6];
  int16_t shank[6];
  uint16_t status;
};

/// Reads the probe's .eadlog container: "EADLOG1\n" then (u32 length, u64 host
/// time, message) records. Only RAW_SAMPLE_BATCH messages carry frames.
std::vector<Frame> readLog(const char* path) {
  std::vector<Frame> frames;
  FILE* f = std::fopen(path, "rb");
  if (f == nullptr) {
    std::fprintf(stderr, "cannot open %s\n", path);
    return frames;
  }
  char magic[8] = {};
  if (std::fread(magic, 1, 8, f) != 8 || std::memcmp(magic, "EADLOG1\n", 8) != 0) {
    std::fprintf(stderr, "not an .eadlog file\n");
    std::fclose(f);
    return frames;
  }
  std::vector<uint8_t> message;
  for (;;) {
    uint32_t length = 0;
    uint64_t hostUs = 0;
    if (std::fread(&length, 4, 1, f) != 1) break;
    if (std::fread(&hostUs, 8, 1, f) != 1) break;
    message.resize(length);
    if (std::fread(message.data(), 1, length, f) != length) break;
    if (length < ead::kHeaderSize + 2) continue;
    if (message[2] != uint8_t(ead::MsgType::RawSampleBatch)) continue;

    const uint8_t* payload = message.data() + ead::kHeaderSize;
    const uint8_t count = payload[0];
    const uint8_t size = payload[1];
    for (uint8_t i = 0; i < count; ++i) {
      const uint8_t* r = payload + 2 + i * size;
      Frame frame{};
      std::memcpy(&frame.timeUs, r, 8);
      std::memcpy(&frame.index, r + 8, 4);
      std::memcpy(frame.foot, r + 12, 12);
      std::memcpy(frame.shank, r + 24, 12);
      std::memcpy(&frame.status, r + 52, 2);
      frames.push_back(frame);
    }
  }
  std::fclose(f);
  return frames;
}

void mapAxes(const EadMountMap& mount, const int16_t chip[3], float scale, float out[3]) {
  for (int axis = 0; axis < 3; ++axis) {
    float sum = 0.0f;
    for (int source = 0; source < 3; ++source) {
      if (mount.m[axis][source] == 0) continue;
      sum += float(mount.m[axis][source]) * float(chip[source]);
    }
    out[axis] = sum / scale;
  }
}

void accelOf(const EadMountMap& mount, const int16_t raw[6], float out[3]) {
  mapAxes(mount, raw, EAD_ACCEL_LSB_PER_G, out);
}

void gyroOf(const EadMountMap& mount, const int16_t raw[6], const float bias[3], float out[3]) {
  float chip[3];
  for (int i = 0; i < 3; ++i) chip[i] = float(raw[3 + i]) / EAD_GYRO_LSB_PER_DPS - bias[i];
  for (int axis = 0; axis < 3; ++axis) {
    float sum = 0.0f;
    for (int source = 0; source < 3; ++source) {
      if (mount.m[axis][source] == 0) continue;
      sum += float(mount.m[axis][source]) * chip[source];
    }
    out[axis] = sum;
  }
}

const char* kEventNames[] = {"", "initial_contact", "toe_off", "foot_flat", "zupt_start",
                            "zupt_end"};

}  // namespace

int main(int argc, char** argv) {
  if (argc < 2) {
    std::fprintf(stderr, "usage: %s recording.eadlog [--still-seconds N] [--trace]\n", argv[0]);
    return 2;
  }
  float stillSeconds = 3.0f;
  bool trace = false;
  bool events = false;
  ead::GaitConfig config;
  for (int i = 2; i < argc; ++i) {
    const std::string flag = argv[i];
    const bool hasValue = i + 1 < argc;
    if (flag == "--still-seconds" && hasValue) stillSeconds = std::stof(argv[++i]);
    else if (flag == "--trace") trace = true;
    else if (flag == "--events") events = true;
    else if (flag == "--impact" && hasValue) config.impactG = std::stof(argv[++i]);
    else if (flag == "--confirm" && hasValue) config.contactConfirmG = std::stof(argv[++i]);
    else if (flag == "--swing-rate" && hasValue) config.swingGyroDps = std::stof(argv[++i]);
    else if (flag == "--min-swing" && hasValue) config.minSwingS = std::stof(argv[++i]);
    else if (flag == "--rate-fall" && hasValue) config.contactRateFallRatio = std::stof(argv[++i]);
    else if (flag == "--refractory" && hasValue) config.contactRefractoryS = std::stof(argv[++i]);
    else if (flag == "--zupt-accel" && hasValue) config.zuptAccelToleranceG = std::stof(argv[++i]);
    else if (flag == "--zupt-gyro" && hasValue) config.zuptGyroDps = std::stof(argv[++i]);
  }

  const std::vector<Frame> frames = readLog(argv[1]);
  if (frames.empty()) return 1;
  std::printf("%zu frames over %.1f s\n", frames.size(),
              float(frames.back().timeUs - frames.front().timeUs) / 1e6f);

  // Calibration from the still period at the start of the recording.
  const size_t stillFrames = size_t(stillSeconds * EAD_SAMPLE_HZ);
  ead::CalibrationAccumulator footCalibration;
  ead::CalibrationAccumulator shankCalibration;
  footCalibration.reset();
  shankCalibration.reset();
  const float noBias[3] = {0.0f, 0.0f, 0.0f};
  for (size_t i = 0; i < stillFrames && i < frames.size(); ++i) {
    float accel[3];
    float gyro[3];
    accelOf(kEadFootMount, frames[i].foot, accel);
    gyroOf(kEadFootMount, frames[i].foot, noBias, gyro);
    footCalibration.add(accel, gyro);
    accelOf(kEadShankMount, frames[i].shank, accel);
    gyroOf(kEadShankMount, frames[i].shank, noBias, gyro);
    shankCalibration.add(accel, gyro);
  }
  ead::CalibrationRecord calibration{};
  calibration.samples = uint32_t(stillFrames);
  calibration.reject = uint16_t(footCalibration.finish(&calibration.foot) |
                                shankCalibration.finish(&calibration.shank));
  std::printf("calibration from the first %.1f s: reject 0x%04X, foot tilt %.1f deg, "
              "shank tilt %.1f deg\n",
              stillSeconds, calibration.reject, calibration.foot.tiltDeg,
              calibration.shank.tiltDeg);
  if (calibration.reject != 0) {
    std::printf("  (the recording does not start still enough to calibrate from)\n");
  }

  // Orientation and gait over the whole recording.
  ead::Mahony foot;
  ead::Mahony shank;
  const float identity[4] = {1.0f, 0.0f, 0.0f, 0.0f};
  foot.reset(identity);
  shank.reset(identity);
  ead::GaitEngine engine;
  engine.reset();
  engine.configure(config);

  uint64_t previousUs = 0;
  size_t cycleCount = 0;
  std::printf("\n%6s %7s %7s %8s %7s %7s %6s %7s  valid\n", "cycle", "time", "stance", "cadence",
              "dist", "speed", "zupt", "dorsi");
  float distanceTotal = 0.0f;
  int validCount = 0;
  for (const Frame& frame : frames) {
    if (previousUs == 0) {
      previousUs = frame.timeUs;
      continue;
    }
    const float dt = float(frame.timeUs - previousUs) / 1e6f;
    previousUs = frame.timeUs;
    if (dt <= 0.0f || dt > 0.1f) continue;

    float accel[3];
    float gyro[3];
    ead::GaitSample sample{};
    sample.timeUs = frame.timeUs;
    sample.frameIndex = frame.index;

    accelOf(kEadFootMount, frame.foot, accel);
    gyroOf(kEadFootMount, frame.foot, calibration.foot.gyroBiasDps, gyro);
    ead::rotateByQuaternion(calibration.foot.alignment, accel, sample.footAccelG);
    ead::rotateByQuaternion(calibration.foot.alignment, gyro, sample.footGyroDps);
    foot.update(sample.footGyroDps, sample.footAccelG, dt, EAD_MAHONY_KP, EAD_MAHONY_KI);

    float shankAccel[3];
    float shankGyro[3];
    accelOf(kEadShankMount, frame.shank, accel);
    gyroOf(kEadShankMount, frame.shank, calibration.shank.gyroBiasDps, gyro);
    ead::rotateByQuaternion(calibration.shank.alignment, accel, shankAccel);
    ead::rotateByQuaternion(calibration.shank.alignment, gyro, shankGyro);
    shank.update(shankGyro, shankAccel, dt, EAD_MAHONY_KP, EAD_MAHONY_KI);
    for (int i = 0; i < 3; ++i) sample.shankGyroDps[i] = shankGyro[i];

    for (int i = 0; i < 4; ++i) sample.footQuaternion[i] = foot.quaternion()[i];
    ead::relativeOrientation(shank.quaternion(), foot.quaternion(), sample.relativeQuaternion);

    engine.update(sample);

    if (trace) {
      const float rate = std::sqrt(sample.footGyroDps[0] * sample.footGyroDps[0] +
                                   sample.footGyroDps[1] * sample.footGyroDps[1] +
                                   sample.footGyroDps[2] * sample.footGyroDps[2]);
      const float magnitude = std::sqrt(sample.footAccelG[0] * sample.footAccelG[0] +
                                        sample.footAccelG[1] * sample.footAccelG[1] +
                                        sample.footAccelG[2] * sample.footAccelG[2]);
      std::printf("TRACE %.3f %6.3f %8.1f %d %d\n",
                  float(frame.timeUs - frames.front().timeUs) / 1e6f, magnitude, rate,
                  int(engine.state()), engine.inZupt() ? 1 : 0);
    }

    ead::GaitEvent event;
    while (engine.takeEvent(&event)) {
      if (events && event.type == ead::GaitEventType::InitialContact) {
        std::printf("IC %.2f\n", float(event.timeUs - frames.front().timeUs) / 1e6f);
      }
      if (trace) {
        std::printf("EVENT %.3f %s\n", float(event.timeUs - frames.front().timeUs) / 1e6f,
                    kEventNames[int(event.type)]);
      }
    }
    ead::GaitCycle cycle;
    while (engine.takeCycle(&cycle)) {
      ++cycleCount;
      std::printf("%6zu %7.2f %7.2f %8.1f %7.2f %7.2f %6.2f %7.1f  %s\n", cycleCount,
                  cycle.cycleTimeS, cycle.stanceRatio, cycle.cadenceStepsPerMin, cycle.distanceM,
                  cycle.speedMps, cycle.zuptQuality, cycle.peakDorsiflexionDeg,
                  cycle.valid ? "yes" : "no");
      if (cycle.valid) {
        distanceTotal += cycle.distanceM;
        ++validCount;
      }
    }
  }
  std::printf("\n%d valid cycles, total distance %.2f m\n", validCount, distanceTotal);
  return 0;
}
