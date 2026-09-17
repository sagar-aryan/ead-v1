#pragma once
// Durable telemetry ring: complete encoded messages addressed by sequence
// number, so every link can stream from its own cursor and a reconnecting host
// can backfill a gap (docs/protocol.md). The oldest messages are evicted when
// either the byte storage or the slot table is full.
// Not thread-safe: the device serialises access with a mutex.

#include <cstddef>
#include <cstdint>

#include "ead/protocol.h"

namespace ead {

class MsgRing {
 public:
  struct Slot {
    uint32_t seq;
    uint32_t offset;
    uint32_t length;
  };

  MsgRing(uint8_t* storage, size_t storageCap, Slot* slots, size_t slotCap);

  // Writes header (with the next sequence number) + payload. Returns the
  // sequence number, or 0 if the message is larger than the whole ring.
  uint32_t append(MsgType type, uint64_t time_us, const uint8_t* payload, size_t payloadLen);

  // Copies message `seq` into `out`. Returns its length, or 0 when `seq` is not
  // stored (evicted or not yet written) or `cap` is too small.
  size_t read(uint32_t seq, uint8_t* out, size_t cap) const;
  // Length of message `seq`, or 0 when it is not stored.
  size_t lengthOf(uint32_t seq) const;

  uint32_t oldest() const { return count_ ? slots_[head_].seq : 0; }
  uint32_t last() const { return nextSeq_ - 1; }
  size_t usedBytes() const { return usedBytes_; }

 private:
  void evictOldest();
  void copyIn(size_t offset, const uint8_t* src, size_t n);
  void copyOut(size_t offset, uint8_t* dst, size_t n) const;
  const Slot* find(uint32_t seq) const;

  uint8_t* storage_;
  size_t storageCap_;
  Slot* slots_;
  size_t slotCap_;
  size_t head_ = 0;
  size_t count_ = 0;
  size_t writePos_ = 0;
  size_t usedBytes_ = 0;
  uint32_t nextSeq_ = 1;
};

}  // namespace ead
