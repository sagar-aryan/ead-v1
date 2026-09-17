#pragma once
// Consistent Overhead Byte Stuffing: removes every 0x00 from a buffer so 0x00
// can delimit frames on the USB byte stream (docs/protocol.md).

#include <cstddef>
#include <cstdint>

namespace ead {

constexpr size_t cobsMaxEncodedSize(size_t n) { return n + n / 254 + 1; }

// Streaming encoder, so a message and its trailing CRC can be encoded without
// first copying them into one buffer.
class CobsWriter {
 public:
  CobsWriter(uint8_t* out, size_t cap);
  void put(uint8_t byte);
  void put(const uint8_t* bytes, size_t n);
  // Returns the encoded length, or 0 if the output buffer overflowed.
  size_t finish();

 private:
  void startBlock();

  uint8_t* out_;
  size_t cap_;
  size_t pos_ = 0;
  size_t codeIndex_ = 0;
  uint8_t code_ = 1;
  bool ok_ = true;
};

// Returns the encoded length, or 0 if `cap` is too small.
size_t cobsEncode(const uint8_t* in, size_t n, uint8_t* out, size_t cap);

// Returns the decoded length, or kCobsError on malformed input (a zero byte or
// a block that runs past the end) or when `cap` is too small.
constexpr size_t kCobsError = static_cast<size_t>(-1);
size_t cobsDecode(const uint8_t* in, size_t n, uint8_t* out, size_t cap);

}  // namespace ead
