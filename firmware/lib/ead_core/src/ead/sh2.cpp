#include "ead/sh2.h"

#include "ead/bytes.h"

namespace ead {
namespace sh2 {

namespace {

constexpr uint8_t kMaxChannel = 5;
constexpr size_t kProductIdSize = 16;
constexpr size_t kFeatureResponseSize = 17;
constexpr size_t kTimestampSize = 5;
constexpr size_t kSensorReportSize = 10;

size_t encodeControl(uint8_t sequence, const uint8_t* cargo, size_t cargoLen, uint8_t* out,
                     size_t cap) {
  ByteWriter w(out, cap);
  w.u16(uint16_t(kHeaderSize + cargoLen));
  w.u8(kChannelControl);
  w.u8(sequence);
  w.bytes(cargo, cargoLen);
  return w.ok() ? w.size() : 0;
}

}  // namespace

bool parseHeader(const uint8_t* bytes, size_t maxLength, Header* out) {
  const uint16_t raw = uint16_t(bytes[0] | (bytes[1] << 8));
  out->continuation = (raw & 0x8000u) != 0;
  out->length = uint16_t(raw & 0x7FFFu);
  out->channel = bytes[2];
  out->sequence = bytes[3];
  if (out->length == 0) return true;
  return out->length >= kHeaderSize && out->length <= maxLength && out->channel <= kMaxChannel;
}

size_t encodeProductIdRequest(uint8_t sequence, uint8_t* out, size_t cap) {
  const uint8_t cargo[2] = {kReportProductIdRequest, 0};
  return encodeControl(sequence, cargo, sizeof cargo, out, cap);
}

size_t encodeSetFeature(uint8_t sequence, uint8_t sensorId, uint32_t intervalUs, uint8_t* out,
                        size_t cap) {
  uint8_t cargo[17];
  ByteWriter w(cargo, sizeof cargo);
  w.u8(kReportSetFeature);
  w.u8(sensorId);
  w.u8(0);   // flags: no change sensitivity, no wake, not always-on
  w.u16(0);  // change sensitivity
  w.u32(intervalUs);
  w.u32(0);  // batch interval: report as soon as sampled
  w.u32(0);  // sensor-specific
  return encodeControl(sequence, cargo, sizeof cargo, out, cap);
}

void parseControl(const uint8_t* cargo, size_t len, ControlReplies* out) {
  size_t pos = 0;
  while (pos < len) {
    const uint8_t id = cargo[pos];
    if (id == kReportProductIdResponse && pos + kProductIdSize <= len) {
      if (out->productIds == 0) {
        ByteReader r(cargo + pos + 1, kProductIdSize - 1);
        ProductId& p = out->firstProductId;
        p.resetCause = r.u8();
        p.versionMajor = r.u8();
        p.versionMinor = r.u8();
        p.partNumber = r.u32();
        p.buildNumber = r.u32();
        p.versionPatch = r.u16();
      }
      out->productIds++;
      pos += kProductIdSize;
    } else if (id == kReportGetFeatureResponse && pos + kFeatureResponseSize <= len) {
      const uint8_t sensor = cargo[pos + 1];
      if (sensor < 8) out->featureSensors |= uint8_t(1u << sensor);
      pos += kFeatureResponseSize;
    } else {
      return;  // other control traffic (command responses); its length varies
    }
  }
}

size_t parseInput(const uint8_t* cargo, size_t len, int64_t intUs, Sample* out, size_t cap,
                  bool* unknown) {
  *unknown = false;
  size_t count = 0;
  int64_t referenceUs = intUs;
  size_t pos = 0;
  while (pos < len) {
    const uint8_t id = cargo[pos];
    if (id == kReportBaseTimestamp || id == kReportTimestampRebase) {
      if (pos + kTimestampSize > len) break;
      ByteReader r(cargo + pos + 1, 4);
      const int32_t ticks = int32_t(r.u32());
      // The base timestamp says how long before the INT the batch was
      // referenced; a rebase moves that reference later.
      referenceUs = id == kReportBaseTimestamp ? intUs - int64_t(ticks) * 100
                                               : referenceUs + int64_t(ticks) * 100;
      pos += kTimestampSize;
    } else if (id == kReportAccelerometer || id == kReportGyroscope) {
      if (pos + kSensorReportSize > len) break;
      const uint8_t* p = cargo + pos;
      if (count < cap) {
        Sample& s = out[count++];
        s.reportId = id;
        s.sequence = p[1];
        s.accuracy = uint8_t(p[2] & 0x03u);
        const uint16_t delay = uint16_t(((p[2] & 0xFCu) << 6) | p[3]);
        s.timeUs = referenceUs + int64_t(delay) * 100;
        ByteReader r(p + 4, 6);
        for (int16_t& v : s.value) v = r.i16();
      }
      pos += kSensorReportSize;
    } else {
      *unknown = true;
      break;
    }
  }
  return count;
}

}  // namespace sh2
}  // namespace ead
