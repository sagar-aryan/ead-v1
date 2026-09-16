// EAD-V1 wire codec implementation. See codec.h for format refs.
#include "codec.h"

namespace ead {

static void putU16le(uint8_t* p, uint16_t v) {
  p[0] = (uint8_t)(v & 0xFF);
  p[1] = (uint8_t)((v >> 8) & 0xFF);
}
static void putU32le(uint8_t* p, uint32_t v) {
  p[0] = (uint8_t)(v & 0xFF);
  p[1] = (uint8_t)((v >> 8) & 0xFF);
  p[2] = (uint8_t)((v >> 16) & 0xFF);
  p[3] = (uint8_t)((v >> 24) & 0xFF);
}
static void putU64le(uint8_t* p, uint64_t v) {
  for (int i = 0; i < 8; i++) p[i] = (uint8_t)((v >> (8 * i)) & 0xFF);
}
static uint16_t getU16le(const uint8_t* p) {
  return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}
static uint32_t getU32le(const uint8_t* p) {
  return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
         ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}
static uint64_t getU64le(const uint8_t* p) {
  uint64_t v = 0;
  for (int i = 7; i >= 0; i--) v = (v << 8) | p[i];
  return v;
}

uint32_t crc32(const uint8_t* data, size_t len) {
  uint32_t crc = 0xFFFFFFFFu;
  for (size_t i = 0; i < len; i++) {
    crc ^= data[i];
    for (int k = 0; k < 8; k++) {
      crc = (crc & 1u) ? (crc >> 1) ^ 0xEDB88320u : (crc >> 1);
    }
  }
  return crc ^ 0xFFFFFFFFu;
}

size_t wsHeaderEncode(const WsHeader& h, uint8_t* out, size_t outLen) {
  if (outLen < 20 || out == nullptr) return 0;
  putU16le(out + 0, h.ver);
  out[2] = h.type;
  out[3] = h.flags;
  putU32le(out + 4, h.len);
  putU32le(out + 8, h.seq);
  putU64le(out + 12, h.time_us);
  return 20;
}

bool wsHeaderDecode(const uint8_t* buf, size_t bufLen, WsHeader& h) {
  if (bufLen < 20 || buf == nullptr) return false;
  h.ver     = getU16le(buf + 0);
  h.type    = buf[2];
  h.flags   = buf[3];
  h.len     = getU32le(buf + 4);
  h.seq     = getU32le(buf + 8);
  h.time_us = getU64le(buf + 12);
  return true;
}

size_t ead1HeaderEncode(const Ead1Header& h, uint8_t* out, size_t outLen) {
  // Layout (32 bytes total):
  //   magic [0,4) | type [4] | flags [5] | header_len [6,8) |
  //   payload_len [8,12) | seq [12,16) | start_time_us [16,24) |
  //   payload_crc32 [24,28) | header_crc32 [28,32).
  if (outLen < 32 || out == nullptr) return 0;
  putU32le(out + 0, h.magic);
  out[4] = h.block_type;
  out[5] = h.block_flags;
  putU16le(out + 6, 32);
  putU32le(out + 8, h.payload_len);
  putU32le(out + 12, h.seq);
  putU64le(out + 16, h.start_time_us);
  putU32le(out + 24, h.payload_crc32);
  // header_crc32 covers the whole 32-byte header with its own field zeroed;
  // the caller's header_crc32 input is ignored — canonical CRC always wins.
  (void)h.header_crc32;
  putU32le(out + 28, 0);
  putU32le(out + 28, crc32(out, 32));
  return 32;
}

bool ead1HeaderDecode(const uint8_t* buf, size_t bufLen, Ead1Header& h) {
  if (bufLen < 32 || buf == nullptr) return false;
  h.magic         = getU32le(buf + 0);
  h.block_type    = buf[4];
  h.block_flags   = buf[5];
  h.header_len    = getU16le(buf + 6);
  h.payload_len   = getU32le(buf + 8);
  h.seq           = getU32le(buf + 12);
  h.start_time_us = getU64le(buf + 16);
  h.payload_crc32 = getU32le(buf + 24);
  h.header_crc32  = getU32le(buf + 28);
  if (h.magic != kEad1Magic) return false;
  if (h.header_len != 32) return false;
  uint8_t tmp[32];
  for (int i = 0; i < 32; i++) tmp[i] = buf[i];
  tmp[28] = tmp[29] = tmp[30] = tmp[31] = 0;  // zero header-crc field
  return crc32(tmp, 32) == h.header_crc32;
}

}  // namespace ead
