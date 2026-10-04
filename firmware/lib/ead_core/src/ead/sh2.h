#pragma once
// SHTP framing and the few SH-2 messages this firmware uses with the BNO086
// (DEC-017). Byte layouts follow CEVA's sh2.c (Apache 2.0), the driver the
// TEST-043 bench ran on this hardware. Portable: no Arduino, built for the host
// tests.

#include <cstddef>
#include <cstdint>

namespace ead {
namespace sh2 {

constexpr size_t kHeaderSize = 4;

// The BNO08x firmware's fixed channel assignment.
enum Channel : uint8_t {
  kChannelCommand = 0,
  kChannelExecutable = 1,
  kChannelControl = 2,
  kChannelInput = 3,
};

// Sensor report ids (SH-2 reference manual).
constexpr uint8_t kReportAccelerometer = 0x01;     // calibrated, Q8 m/s^2
constexpr uint8_t kReportGyroscope = 0x02;         // calibrated, Q9 rad/s
/// The hub's own accelerometer + gyroscope fusion, no magnetometer: unit
/// quaternion (i, j, k, real) in Q14, Z up, heading arbitrary but not jumping.
constexpr uint8_t kReportGameRotationVector = 0x08;
constexpr uint8_t kReportProductIdRequest = 0xF9;
constexpr uint8_t kReportProductIdResponse = 0xF8;
constexpr uint8_t kReportSetFeature = 0xFD;
constexpr uint8_t kReportGetFeatureResponse = 0xFC;
constexpr uint8_t kReportBaseTimestamp = 0xFB;
constexpr uint8_t kReportTimestampRebase = 0xFA;
// Executable channel, device to host: the sensor finished a reset.
constexpr uint8_t kExecutableResetComplete = 0x01;

struct Header {
  uint16_t length;  // whole packet, header included
  bool continuation;
  uint8_t channel;
  uint8_t sequence;
};

/// Decodes a 4-byte SHTP header. False when it cannot be a packet: a length
/// below the header size other than 0 (0 means nothing to send), a channel the
/// BNO08x does not use, or more than `maxLength`. An undriven MISO reads 0xFF.
bool parseHeader(const uint8_t* bytes, size_t maxLength, Header* out);

/// Host packets on the control channel; `sequence` is the host's per-channel
/// counter. Return the packet length, or 0 if `cap` is too small.
size_t encodeProductIdRequest(uint8_t sequence, uint8_t* out, size_t cap);
size_t encodeSetFeature(uint8_t sequence, uint8_t sensorId, uint32_t intervalUs, uint8_t* out,
                        size_t cap);

struct ProductId {
  uint8_t resetCause;
  uint8_t versionMajor;
  uint8_t versionMinor;
  uint16_t versionPatch;
  uint32_t partNumber;
  uint32_t buildNumber;
};

/// What a control-channel cargo held. A cargo may carry several responses.
struct ControlReplies {
  uint8_t productIds;      // product ID responses seen
  ProductId firstProductId;
  uint16_t featureSensors;  // bit n set: a Get Feature Response for sensor id n (n < 16)
  /// The report interval each Get Feature Response gave, by sensor id: the
  /// hub rounds a requested interval to one its sensors support.
  uint32_t intervalUs[16];
};

void parseControl(const uint8_t* cargo, size_t len, ControlReplies* out);

struct Sample {
  uint8_t reportId;
  uint8_t sequence;  // per report id, wraps at 256
  uint8_t accuracy;  // 0 unreliable .. 3 high
  int16_t value[4];  // x, y, z; a rotation vector adds the real part
  int64_t timeUs;
};

/// Parses an input-channel cargo (the bytes after the header). `intUs` is the
/// host time of the INT assertion that announced the packet; each sample's time
/// is that minus the base timestamp plus its own delay (100 us units). Stops at
/// a report id whose length is not known here, since nothing after it can be
/// located, and sets `*unknown`.
size_t parseInput(const uint8_t* cargo, size_t len, int64_t intUs, Sample* out, size_t cap,
                  bool* unknown);

}  // namespace sh2
}  // namespace ead
