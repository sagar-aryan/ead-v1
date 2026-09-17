#pragma once
// EAD-V1 wire protocol: doc 08 frame header, doc 09 §6 raw frame, and the
// payload layouts defined in docs/protocol.md (schema 1).

#include <cstddef>
#include <cstdint>

#include "ead/bytes.h"

namespace ead {

constexpr uint16_t kProtocolVersion = 1;  // doc 08 header field
constexpr uint16_t kSchemaVersion = 2;    // payload layouts, docs/protocol.md
constexpr size_t kHeaderSize = 20;
constexpr size_t kRawFrameSize = 54;
constexpr size_t kMaxRawFramesPerBatch = 10;
// lwIP reports a socket writable only when more than 2880 bytes of its 5760-byte
// send buffer are free, so messages below that size never block a Wi-Fi send.
constexpr size_t kMaxMessageSize = 2800;

enum class MsgType : uint8_t {
  Hello = 0x01,
  ConfigGet = 0x02,
  ConfigSet = 0x03,
  SessionStart = 0x04,
  SessionStop = 0x05,
  Pause = 0x06,
  Resume = 0x07,
  RawSampleBatch = 0x08,
  EventBatch = 0x09,
  StepBatch = 0x0A,
  HapticBatch = 0x0B,
  Status = 0x0C,
  Ack = 0x0D,
  Error = 0x0E,
  RecoveryInfo = 0x0F,
  BackfillRequest = 0x10,
  BackfillData = 0x11,
  ServiceTest = 0x12,
};

// Durable messages carry their own sequence number and can be backfilled.
bool isDurable(MsgType type);

struct Header {
  uint16_t version;
  uint8_t type;
  uint8_t flags;
  uint32_t length;
  uint32_t sequence;
  uint64_t time_us;
};

void writeHeader(ByteWriter& w, const Header& h);
// Encodes header + payload. Returns the total length, or 0 if it does not fit.
size_t encodeMessage(MsgType type, uint32_t sequence, uint64_t time_us, const uint8_t* payload,
                     size_t payloadLen, uint8_t* out, size_t cap);
// Validates the protocol version and that the header length matches `len`.
bool parseMessage(const uint8_t* msg, size_t len, Header* header, const uint8_t** payload);

// ---- RAW_SAMPLE_BATCH ------------------------------------------------------

enum RawStatus : uint16_t {
  kRawFootReadFail = 1u << 0,
  kRawShankReadFail = 1u << 1,
  kRawShankRepeated = 1u << 2,
  kRawFootAccelSaturated = 1u << 3,
  kRawFootGyroSaturated = 1u << 4,
  kRawShankAccelSaturated = 1u << 5,
  kRawShankGyroSaturated = 1u << 6,
  kRawFootRepeated = 1u << 7,
};

// One synchronized frame (doc 09 §6). Accel/gyro are chip-frame ADC counts in
// the order ax, ay, az, gx, gy, gz (DEC-007). Quaternions are Q15 (w, x, y, z).
struct RawFrame {
  uint64_t timestamp_us;
  uint32_t frame_index;
  int16_t foot[6];
  int16_t shank[6];
  int16_t q_foot[4];
  int16_t q_shank[4];
  uint16_t status;
};

void writeRawFrame(ByteWriter& w, const RawFrame& f);
bool readRawFrame(ByteReader& r, RawFrame* f);
size_t encodeRawBatchPayload(const RawFrame* frames, size_t count, uint8_t* out, size_t cap);

// ---- HELLO -----------------------------------------------------------------

enum Capability : uint8_t {
  kCapHapticsFitted = 1u << 0,
  kCapFlashStorage = 1u << 1,
  kCapPsramRing = 1u << 2,
};

struct HelloInfo {
  uint16_t schema;
  uint8_t device_state;
  uint8_t reset_reason;
  uint32_t boot_id;
  uint8_t mac[6];
  uint8_t who_foot;
  uint8_t who_shank;
  uint8_t capabilities;
  uint8_t config_sha256[32];
  uint32_t oldest_seq;
  uint32_t last_seq;
  const char* fw_version;
};

size_t encodeHelloPayload(const HelloInfo& info, uint8_t* out, size_t cap);
// Host HELLO: u16 schema.
bool decodeHelloRequest(const uint8_t* payload, size_t len, uint16_t* schema);

// ---- STATUS ----------------------------------------------------------------

enum DeviceState : uint8_t {
  kStateBoot = 0,
  kStateSelfTest = 1,
  kStateCalibrating = 2,
  kStateReferenceCapture = 3,
  kStateReady = 4,
  kStateRunning = 5,
  kStatePaused = 6,
  kStateFault = 7,
  kStateRecovery = 8,
};

enum Fault : uint16_t {
  kFaultFootAbsent = 1u << 0,
  kFaultShankAbsent = 1u << 1,
  kFaultFootConfig = 1u << 2,
  kFaultShankConfig = 1u << 3,
  kFaultFootNoDataReady = 1u << 4,
  kFaultShankNoDataReady = 1u << 5,
  kFaultNoPsram = 1u << 6,
  kFaultFootFrozen = 1u << 7,
  kFaultShankFrozen = 1u << 8,
  kFaultAcquisitionStalled = 1u << 9,
};

enum LinkFlag : uint8_t {
  kLinkUsbActive = 1u << 0,
  kLinkWifiActive = 1u << 1,
};

struct StatusInfo {
  uint8_t device_state;
  uint8_t link_flags;
  uint16_t faults;
  uint32_t frame_index;
  uint32_t frames_dropped;
  uint32_t shank_repeated;
  uint32_t i2c_errors;
  uint32_t imu_reinits;
  uint32_t oldest_seq;
  uint32_t last_seq;
  int8_t ap_rssi_dbm;
  uint8_t ap_stations;
  uint32_t heap_free_min;
  uint16_t stack_free_acquisition;
  uint16_t stack_free_processing;
  uint16_t stack_free_usb;
  uint16_t stack_free_wifi;
  uint8_t calibration_state;
  uint32_t calibration_samples;
  uint16_t calibration_reject;
};

constexpr size_t kStatusPayloadSize = 53;

/// STATUS calibration_state (docs/protocol.md §5.3).
enum class CalibrationState : uint8_t { None = 0, Collecting = 1, Ready = 2, Rejected = 3 };

/// Session kinds. Schema 2 accepts CALIBRATION only.
enum class SessionKind : uint8_t { Calibration = 1 };

constexpr size_t kSessionStartPayloadSize = 4;
constexpr size_t kCalibrationPayloadSize = 128;
constexpr uint16_t kCalibMinDurationMs = 2000;
constexpr uint16_t kCalibMaxDurationMs = 30000;

/// SESSION_START, host → device.
bool decodeSessionStart(const uint8_t* payload, size_t len, uint8_t* kind, uint16_t* durationMs);
size_t encodeSessionStart(uint8_t kind, uint16_t durationMs, uint8_t* out, size_t cap);

struct CalibrationRecord;  // ead/calibration.h

/// SESSION_STOP, device → host: the calibration record.
size_t encodeCalibrationPayload(uint8_t kind, const CalibrationRecord& record, uint8_t* out,
                                size_t cap);
bool decodeCalibrationPayload(const uint8_t* payload, size_t len, uint8_t* kind,
                              CalibrationRecord* record);
size_t encodeStatusPayload(const StatusInfo& s, uint8_t* out, size_t cap);

// ---- ERROR -----------------------------------------------------------------

enum class ErrorCode : uint16_t {
  BadFrame = 1,
  SchemaMismatch = 2,
  NotSupported = 3,
  InvalidState = 4,
  BadPayload = 5,
  BackfillUnavailable = 6,
};

size_t encodeErrorPayload(uint32_t cmdSeq, uint8_t cmdType, ErrorCode code, const char* detail,
                          uint8_t* out, size_t cap);

// ---- CONFIG_GET response ---------------------------------------------------

size_t encodeConfigPayload(uint16_t format, const uint8_t sha256[32], const uint8_t* section,
                           size_t sectionLen, uint8_t* out, size_t cap);

// ---- BACKFILL --------------------------------------------------------------

bool decodeBackfillRequest(const uint8_t* payload, size_t len, uint32_t* first, uint32_t* last);
constexpr size_t kBackfillPrefixSize = 13;
// Payload prefix; the complete original messages follow it.
size_t encodeBackfillPrefix(uint32_t cmdSeq, uint32_t firstSeq, uint32_t lastSeq, bool more,
                            uint8_t* out, size_t cap);

// ---- USB framing: 00 | COBS(message || CRC32-LE) | 00 ----------------------

constexpr size_t kMaxUsbFrameSize = kMaxMessageSize + 4 + (kMaxMessageSize + 4) / 254 + 1 + 2;

size_t encodeUsbFrame(const uint8_t* msg, size_t len, uint8_t* out, size_t cap);

// Reassembles messages from the USB byte stream. Bytes between delimiters that
// fail COBS, CRC or header validation are counted and discarded, which also
// skips ROM boot text and anything else that is not a frame.
class UsbFrameDecoder {
 public:
  // Returns true when `byte` completed a valid message, available via
  // message()/length() until the next call.
  bool feed(uint8_t byte);
  const uint8_t* message() const { return decoded_; }
  size_t length() const { return decodedLen_; }
  uint32_t rejected() const { return rejected_; }

 private:
  uint8_t encoded_[kMaxUsbFrameSize];
  uint8_t decoded_[kMaxMessageSize + 4];
  size_t encodedLen_ = 0;
  size_t decodedLen_ = 0;
  bool overflow_ = false;
  uint32_t rejected_ = 0;
};

}  // namespace ead
