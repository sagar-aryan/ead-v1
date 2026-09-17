#include "ead/cobs.h"

namespace ead {

CobsWriter::CobsWriter(uint8_t* out, size_t cap) : out_(out), cap_(cap) { startBlock(); }

void CobsWriter::startBlock() {
  if (pos_ >= cap_) {
    ok_ = false;
    return;
  }
  codeIndex_ = pos_++;
  code_ = 1;
}

void CobsWriter::put(uint8_t byte) {
  if (!ok_) return;
  if (byte == 0) {
    out_[codeIndex_] = code_;
    startBlock();
    return;
  }
  if (pos_ >= cap_) {
    ok_ = false;
    return;
  }
  out_[pos_++] = byte;
  if (++code_ == 0xFF) {
    // A full block of 254 data bytes carries no implied zero.
    out_[codeIndex_] = code_;
    startBlock();
  }
}

void CobsWriter::put(const uint8_t* bytes, size_t n) {
  for (size_t i = 0; i < n; i++) put(bytes[i]);
}

size_t CobsWriter::finish() {
  if (!ok_) return 0;
  out_[codeIndex_] = code_;
  return pos_;
}

size_t cobsEncode(const uint8_t* in, size_t n, uint8_t* out, size_t cap) {
  CobsWriter w(out, cap);
  w.put(in, n);
  return w.finish();
}

size_t cobsDecode(const uint8_t* in, size_t n, uint8_t* out, size_t cap) {
  size_t i = 0;
  size_t o = 0;
  while (i < n) {
    const uint8_t code = in[i++];
    if (code == 0) return kCobsError;
    for (uint8_t k = 1; k < code; k++) {
      if (i >= n || in[i] == 0 || o >= cap) return kCobsError;
      out[o++] = in[i++];
    }
    if (code != 0xFF && i < n) {
      if (o >= cap) return kCobsError;
      out[o++] = 0;
    }
  }
  return o;
}

}  // namespace ead
