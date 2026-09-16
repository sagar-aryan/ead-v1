#pragma once
// EAD-V1 wire codec: WebSocket binary frame header (doc 08) +
// LittleFS EAD1 block envelope (doc 09), little-endian, CRC32.
// Header layout (doc 08 sec 2):
//   u16 protocol_version | u8 message_type | u8 flags |
//   u32 payload_length | u32 sequence | u64 device_time_us | payload[]
// Block envelope (doc 09 sec 4):
//   u32 magic=0x45414431 | u8 block_type | u8 block_flags |
//   u16 header_length | u32 payload_length | u32 sequence |
//   u64 start_time_us | u32 payload_crc32 | u32 header_crc32 | payload[]

#include <stdint.h>
#include <stddef.h>

namespace ead {

// ---- Message types (doc 08 sec 3) ----
enum class WsMsg : uint8_t {
  HELLO           = 0x01,
  CONFIG_GET      = 0x02,
  CONFIG_SET      = 0x03,
  SESSION_START   = 0x04,
  SESSION_STOP    = 0x05,
  PAUSE           = 0x06,
  RESUME          = 0x07,
  RAW_SAMPLE_BATCH= 0x08,
  EVENT_BATCH     = 0x09,
  STEP_BATCH      = 0x0A,
  HAPTIC_BATCH    = 0x0B,
  STATUS          = 0x0C,
  ACK             = 0x0D,
  ERROR           = 0x0E,
  RECOVERY_INFO   = 0x0F,
  BACKFILL_REQUEST= 0x10,
  BACKFILL_DATA   = 0x11,
  SERVICE_TEST    = 0x12,
};

// ---- Block types (doc 09 sec 5) ----
enum class BlockType : uint8_t {
  SESSION_HEADER    = 0x01,
  RAW_SAMPLES       = 0x02,
  EVENTS            = 0x03,
  CYCLES_STEPS      = 0x04,
  HAPTICS           = 0x05,
  STATUS_FAULTS     = 0x06,
  REFERENCE_PROFILE = 0x07,
  SESSION_FOOTER    = 0x08,
};

static const uint16_t kProtoVersion = 1;
static const uint32_t kEad1Magic = 0x45414431u;  // "EAD1"

#pragma pack(push, 1)
struct WsHeader {
  uint16_t ver;        // = 1
  uint8_t  type;       // WsMsg
  uint8_t  flags;
  uint32_t len;        // payload_length
  uint32_t seq;
  uint64_t time_us;    // device monotonic time
};
struct Ead1Header {
  uint32_t magic;      // = 0x45414431
  uint8_t  block_type; // BlockType
  uint8_t  block_flags;
  uint16_t header_len; // sizeof(Ead1Header)
  uint32_t payload_len;
  uint32_t seq;        // monotonic block sequence
  uint64_t start_time_us;
  uint32_t payload_crc32;
  uint32_t header_crc32;  // CRC over header with this field zeroed
};
#pragma pack(pop)

// IEEE CRC32 (polynomial 0xEDB88320), init 0xFFFFFFFF, xor-out 0xFFFFFFFF.
uint32_t crc32(const uint8_t* data, size_t len);

// Serialize header into 20-byte LE buffer; returns bytes written (0 on error).
size_t wsHeaderEncode(const WsHeader& h, uint8_t* out, size_t outLen);
// Parse header from LE buffer; returns true iff outLen >= 20.
bool wsHeaderDecode(const uint8_t* buf, size_t bufLen, WsHeader& h);

// Serialize block header into 32-byte LE buffer; header_crc32 is computed
// over the header with its own field zeroed (doc 09 sec 8 commit rule).
size_t ead1HeaderEncode(const Ead1Header& h, uint8_t* out, size_t outLen);
// Parse + validate: magic, header_len, header CRC. Payload CRC must be
// checked separately against payload bytes (doc 09 sec 8).
bool ead1HeaderDecode(const uint8_t* buf, size_t bufLen, Ead1Header& h);

}  // namespace ead
