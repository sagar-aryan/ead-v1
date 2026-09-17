#include "ead/protocol.h"

#include "ead/cobs.h"
#include "ead/crc32.h"

namespace ead {

bool isDurable(MsgType type) {
  return type == MsgType::RawSampleBatch || type == MsgType::EventBatch ||
         type == MsgType::StepBatch || type == MsgType::HapticBatch;
}

void writeHeader(ByteWriter& w, const Header& h) {
  w.u16(h.version);
  w.u8(h.type);
  w.u8(h.flags);
  w.u32(h.length);
  w.u32(h.sequence);
  w.u64(h.time_us);
}

size_t encodeMessage(MsgType type, uint32_t sequence, uint64_t time_us, const uint8_t* payload,
                     size_t payloadLen, uint8_t* out, size_t cap) {
  ByteWriter w(out, cap);
  writeHeader(w, Header{kProtocolVersion, uint8_t(type), 0, uint32_t(payloadLen), sequence, time_us});
  w.bytes(payload, payloadLen);
  return w.ok() ? w.size() : 0;
}

bool parseMessage(const uint8_t* msg, size_t len, Header* header, const uint8_t** payload) {
  ByteReader r(msg, len);
  header->version = r.u16();
  header->type = r.u8();
  header->flags = r.u8();
  header->length = r.u32();
  header->sequence = r.u32();
  header->time_us = r.u64();
  if (!r.ok() || header->version != kProtocolVersion) return false;
  if (header->length != len - kHeaderSize) return false;
  *payload = msg + kHeaderSize;
  return true;
}

void writeRawFrame(ByteWriter& w, const RawFrame& f) {
  w.u64(f.timestamp_us);
  w.u32(f.frame_index);
  for (int16_t v : f.foot) w.i16(v);
  for (int16_t v : f.shank) w.i16(v);
  for (int16_t v : f.q_foot) w.i16(v);
  for (int16_t v : f.q_shank) w.i16(v);
  w.u16(f.status);
}

bool readRawFrame(ByteReader& r, RawFrame* f) {
  f->timestamp_us = r.u64();
  f->frame_index = r.u32();
  for (int16_t& v : f->foot) v = r.i16();
  for (int16_t& v : f->shank) v = r.i16();
  for (int16_t& v : f->q_foot) v = r.i16();
  for (int16_t& v : f->q_shank) v = r.i16();
  f->status = r.u16();
  return r.ok();
}

size_t encodeRawBatchPayload(const RawFrame* frames, size_t count, uint8_t* out, size_t cap) {
  if (count == 0 || count > kMaxRawFramesPerBatch) return 0;
  ByteWriter w(out, cap);
  w.u8(uint8_t(count));
  w.u8(uint8_t(kRawFrameSize));
  for (size_t i = 0; i < count; i++) writeRawFrame(w, frames[i]);
  return w.ok() ? w.size() : 0;
}

size_t encodeHelloPayload(const HelloInfo& info, uint8_t* out, size_t cap) {
  ByteWriter w(out, cap);
  w.u16(info.schema);
  w.u8(info.device_state);
  w.u8(info.reset_reason);
  w.u32(info.boot_id);
  w.bytes(info.mac, sizeof info.mac);
  w.u8(info.who_foot);
  w.u8(info.who_shank);
  w.u8(info.capabilities);
  w.bytes(info.config_sha256, sizeof info.config_sha256);
  w.u32(info.oldest_seq);
  w.u32(info.last_seq);
  w.str8(info.fw_version);
  return w.ok() ? w.size() : 0;
}

bool decodeHelloRequest(const uint8_t* payload, size_t len, uint16_t* schema) {
  if (len != 2) return false;
  ByteReader r(payload, len);
  *schema = r.u16();
  return r.ok();
}

size_t encodeStatusPayload(const StatusInfo& s, uint8_t* out, size_t cap) {
  ByteWriter w(out, cap);
  w.u8(s.device_state);
  w.u8(s.link_flags);
  w.u16(s.faults);
  w.u32(s.frame_index);
  w.u32(s.frames_dropped);
  w.u32(s.shank_repeated);
  w.u32(s.i2c_errors);
  w.u32(s.imu_reinits);
  w.u32(s.oldest_seq);
  w.u32(s.last_seq);
  w.i8(s.ap_rssi_dbm);
  w.u8(s.ap_stations);
  w.u32(s.heap_free_min);
  w.u16(s.stack_free_acquisition);
  w.u16(s.stack_free_processing);
  w.u16(s.stack_free_usb);
  w.u16(s.stack_free_wifi);
  return w.ok() ? w.size() : 0;
}

size_t encodeErrorPayload(uint32_t cmdSeq, uint8_t cmdType, ErrorCode code, const char* detail,
                          uint8_t* out, size_t cap) {
  ByteWriter w(out, cap);
  w.u32(cmdSeq);
  w.u8(cmdType);
  w.u16(uint16_t(code));
  w.str8(detail);
  return w.ok() ? w.size() : 0;
}

size_t encodeConfigPayload(uint16_t format, const uint8_t sha256[32], const uint8_t* section,
                           size_t sectionLen, uint8_t* out, size_t cap) {
  if (sectionLen > 0xFFFF) return 0;
  ByteWriter w(out, cap);
  w.u16(format);
  w.bytes(sha256, 32);
  w.u16(uint16_t(sectionLen));
  w.bytes(section, sectionLen);
  return w.ok() ? w.size() : 0;
}

bool decodeBackfillRequest(const uint8_t* payload, size_t len, uint32_t* first, uint32_t* last) {
  if (len != 8) return false;
  ByteReader r(payload, len);
  *first = r.u32();
  *last = r.u32();
  return r.ok() && *first != 0 && *first <= *last;
}

size_t encodeBackfillPrefix(uint32_t cmdSeq, uint32_t firstSeq, uint32_t lastSeq, bool more,
                            uint8_t* out, size_t cap) {
  ByteWriter w(out, cap);
  w.u32(cmdSeq);
  w.u32(firstSeq);
  w.u32(lastSeq);
  w.u8(more ? 1 : 0);
  return w.ok() ? w.size() : 0;
}

size_t encodeUsbFrame(const uint8_t* msg, size_t len, uint8_t* out, size_t cap) {
  if (len > kMaxMessageSize || cap < 2) return 0;
  const uint32_t crc = crc32(msg, len);
  const uint8_t crcBytes[4] = {uint8_t(crc), uint8_t(crc >> 8), uint8_t(crc >> 16),
                               uint8_t(crc >> 24)};
  out[0] = 0;
  CobsWriter cobs(out + 1, cap - 2);
  cobs.put(msg, len);
  cobs.put(crcBytes, sizeof crcBytes);
  const size_t n = cobs.finish();
  if (n == 0) return 0;
  out[n + 1] = 0;
  return n + 2;
}

bool UsbFrameDecoder::feed(uint8_t byte) {
  if (byte != 0) {
    if (encodedLen_ < sizeof encoded_) {
      encoded_[encodedLen_++] = byte;
    } else {
      overflow_ = true;
    }
    return false;
  }
  // Delimiter: validate whatever accumulated since the previous one.
  const size_t n = encodedLen_;
  const bool overflowed = overflow_;
  encodedLen_ = 0;
  overflow_ = false;
  if (n == 0) return false;  // back-to-back delimiters
  if (overflowed) {
    rejected_++;
    return false;
  }
  const size_t d = cobsDecode(encoded_, n, decoded_, sizeof decoded_);
  if (d == kCobsError || d < kHeaderSize + 4) {
    rejected_++;
    return false;
  }
  ByteReader crcReader(decoded_ + d - 4, 4);
  if (crcReader.u32() != crc32(decoded_, d - 4)) {
    rejected_++;
    return false;
  }
  Header h;
  const uint8_t* payload;
  if (!parseMessage(decoded_, d - 4, &h, &payload)) {
    rejected_++;
    return false;
  }
  decodedLen_ = d - 4;
  return true;
}

}  // namespace ead
