#pragma once
// Bounds-checked little-endian serialisation for protocol payloads.
// A writer or reader that runs out of space latches ok() == false and ignores
// further operations, so callers check once after a sequence of fields.

#include <cstddef>
#include <cstdint>
#include <cstring>

namespace ead {

class ByteWriter {
 public:
  ByteWriter(uint8_t* buf, size_t cap) : buf_(buf), cap_(cap) {}

  void u8(uint8_t v) { put(&v, 1); }
  void u16(uint16_t v) {
    const uint8_t b[2] = {uint8_t(v), uint8_t(v >> 8)};
    put(b, 2);
  }
  void u32(uint32_t v) {
    const uint8_t b[4] = {uint8_t(v), uint8_t(v >> 8), uint8_t(v >> 16), uint8_t(v >> 24)};
    put(b, 4);
  }
  void u64(uint64_t v) {
    u32(uint32_t(v));
    u32(uint32_t(v >> 32));
  }
  void i8(int8_t v) { u8(uint8_t(v)); }
  void i16(int16_t v) { u16(uint16_t(v)); }
  void f32(float v) {
    uint32_t bits;
    std::memcpy(&bits, &v, sizeof bits);
    u32(bits);
  }
  void bytes(const uint8_t* p, size_t n) { put(p, n); }
  // u8 length prefix + UTF-8 bytes, no terminator.
  void str8(const char* s) {
    const size_t n = std::strlen(s);
    if (n > 255) {
      ok_ = false;
      return;
    }
    u8(uint8_t(n));
    put(reinterpret_cast<const uint8_t*>(s), n);
  }

  size_t size() const { return len_; }
  bool ok() const { return ok_; }

 private:
  void put(const uint8_t* p, size_t n) {
    if (!ok_ || n > cap_ - len_) {
      ok_ = false;
      return;
    }
    std::memcpy(buf_ + len_, p, n);
    len_ += n;
  }

  uint8_t* buf_;
  size_t cap_;
  size_t len_ = 0;
  bool ok_ = true;
};

class ByteReader {
 public:
  ByteReader(const uint8_t* buf, size_t len) : buf_(buf), len_(len) {}

  uint8_t u8() {
    uint8_t b[1] = {0};
    get(b, 1);
    return b[0];
  }
  uint16_t u16() {
    uint8_t b[2] = {0};
    get(b, 2);
    return uint16_t(b[0] | (b[1] << 8));
  }
  uint32_t u32() {
    uint8_t b[4] = {0};
    get(b, 4);
    return uint32_t(b[0]) | (uint32_t(b[1]) << 8) | (uint32_t(b[2]) << 16) |
           (uint32_t(b[3]) << 24);
  }
  uint64_t u64() {
    const uint64_t lo = u32();
    const uint64_t hi = u32();
    return lo | (hi << 32);
  }
  int8_t i8() { return int8_t(u8()); }
  int16_t i16() { return int16_t(u16()); }
  float f32() {
    const uint32_t bits = u32();
    float v;
    std::memcpy(&v, &bits, sizeof v);
    return v;
  }
  void bytes(uint8_t* out, size_t n) { get(out, n); }

  size_t remaining() const { return len_ - pos_; }
  bool ok() const { return ok_; }

 private:
  void get(uint8_t* out, size_t n) {
    if (!ok_ || n > len_ - pos_) {
      ok_ = false;
      return;
    }
    std::memcpy(out, buf_ + pos_, n);
    pos_ += n;
  }

  const uint8_t* buf_;
  size_t len_;
  size_t pos_ = 0;
  bool ok_ = true;
};

}  // namespace ead
