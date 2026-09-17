#include "ead/msg_ring.h"

#include <cstring>

namespace ead {

MsgRing::MsgRing(uint8_t* storage, size_t storageCap, Slot* slots, size_t slotCap)
    : storage_(storage), storageCap_(storageCap), slots_(slots), slotCap_(slotCap) {}

uint32_t MsgRing::append(MsgType type, uint64_t time_us, const uint8_t* payload,
                         size_t payloadLen) {
  const size_t total = kHeaderSize + payloadLen;
  if (total > storageCap_ || slotCap_ == 0) return 0;
  while (count_ > 0 && (usedBytes_ + total > storageCap_ || count_ == slotCap_)) evictOldest();

  const uint32_t seq = nextSeq_++;
  uint8_t header[kHeaderSize];
  ByteWriter w(header, sizeof header);
  writeHeader(w, Header{kProtocolVersion, uint8_t(type), 0, uint32_t(payloadLen), seq, time_us});
  copyIn(writePos_, header, kHeaderSize);
  copyIn((writePos_ + kHeaderSize) % storageCap_, payload, payloadLen);

  slots_[(head_ + count_) % slotCap_] = Slot{seq, uint32_t(writePos_), uint32_t(total)};
  count_++;
  writePos_ = (writePos_ + total) % storageCap_;
  usedBytes_ += total;
  return seq;
}

size_t MsgRing::read(uint32_t seq, uint8_t* out, size_t cap) const {
  const Slot* s = find(seq);
  if (s == nullptr || s->length > cap) return 0;
  copyOut(s->offset, out, s->length);
  return s->length;
}

size_t MsgRing::lengthOf(uint32_t seq) const {
  const Slot* s = find(seq);
  return s ? s->length : 0;
}

const MsgRing::Slot* MsgRing::find(uint32_t seq) const {
  if (count_ == 0) return nullptr;
  const uint32_t first = slots_[head_].seq;
  // Sequence numbers in the ring are contiguous, so the slot index is direct.
  if (seq < first || seq - first >= count_) return nullptr;
  return &slots_[(head_ + (seq - first)) % slotCap_];
}

void MsgRing::evictOldest() {
  usedBytes_ -= slots_[head_].length;
  head_ = (head_ + 1) % slotCap_;
  count_--;
}

void MsgRing::copyIn(size_t offset, const uint8_t* src, size_t n) {
  if (n == 0) return;
  const size_t first = n < storageCap_ - offset ? n : storageCap_ - offset;
  std::memcpy(storage_ + offset, src, first);
  std::memcpy(storage_, src + first, n - first);
}

void MsgRing::copyOut(size_t offset, uint8_t* dst, size_t n) const {
  const size_t first = n < storageCap_ - offset ? n : storageCap_ - offset;
  std::memcpy(dst, storage_ + offset, first);
  std::memcpy(dst + first, storage_, n - first);
}

}  // namespace ead
