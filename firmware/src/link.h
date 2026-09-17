#pragma once
// Protocol endpoint for one host link (USB or Wi-Fi), independent of the
// transport. Each instance is driven from a single task, so it needs no locks;
// shared data (ring, counters) is synchronised by its owners.
//
// Transports call onMessage() for every complete inbound message and pump
// outbound data with peek()/commit(): peek() writes the next message without
// consuming it, so a transport that cannot write yet simply retries later.

#include <cstddef>
#include <cstdint>

#include "ead/protocol.h"

class Link {
 public:
  explicit Link(uint8_t activeFlag) : activeFlag_(activeFlag) {}

  void onConnect(int64_t nowUs);
  void onDisconnect();
  void onMessage(const uint8_t* msg, size_t len, int64_t nowUs);

  size_t peek(uint8_t* out, size_t cap, int64_t nowUs);
  void commit(int64_t nowUs);

  // Streaming to a host that sent anything within the last 3 s.
  bool streaming(int64_t nowUs) const;
  bool silentFor(int64_t nowUs, int64_t us) const { return nowUs - lastRxUs_ > us; }
  uint8_t activeFlag() const { return activeFlag_; }

 private:
  enum class Source : uint8_t { None, Reply, Status, Live, Backfill };

  void queueReply(ead::MsgType type, const uint8_t* payload, size_t len, int64_t nowUs);
  void queueError(uint32_t cmdSeq, uint8_t cmdType, ead::ErrorCode code, const char* detail,
                  int64_t nowUs);
  void startBackfill(uint32_t cmdSeq, uint8_t cmdType, uint32_t first, uint32_t last,
                     int64_t nowUs);
  size_t buildBackfill(uint8_t* out, size_t cap, int64_t nowUs);

  static constexpr size_t kReplySlots = 4;
  static constexpr size_t kReplyCap = 512;
  struct Reply {
    uint16_t len;
    uint8_t data[kReplyCap];
  };

  uint8_t activeFlag_;
  Reply replies_[kReplySlots] = {};
  size_t replyHead_ = 0;
  size_t replyCount_ = 0;

  int64_t lastRxUs_ = 0;
  bool helloReceived_ = false;
  uint32_t cursor_ = 0;  // next durable sequence number to stream
  int64_t nextStatusUs_ = 0;

  bool backfillActive_ = false;
  uint32_t backfillCmd_ = 0;
  uint32_t backfillNext_ = 0;
  uint32_t backfillLast_ = 0;

  Source pending_ = Source::None;
  uint32_t pendingSeq_ = 0;
};
