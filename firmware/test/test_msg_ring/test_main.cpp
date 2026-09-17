#include <unity.h>

#include <cstring>
#include <deque>
#include <vector>

#include "ead/msg_ring.h"

using ead::MsgRing;
using ead::MsgType;

void setUp() {}
void tearDown() {}

namespace {

std::vector<uint8_t> payloadFor(uint32_t seq, size_t len) {
  std::vector<uint8_t> p(len);
  for (size_t i = 0; i < len; i++) p[i] = uint8_t(seq * 31 + i);
  return p;
}

void assertStored(const MsgRing& ring, uint32_t seq, const std::vector<uint8_t>& payload) {
  std::vector<uint8_t> out(ead::kHeaderSize + payload.size());
  TEST_ASSERT_EQUAL_size_t(out.size(), ring.read(seq, out.data(), out.size()));
  ead::Header h;
  const uint8_t* body;
  TEST_ASSERT_TRUE(ead::parseMessage(out.data(), out.size(), &h, &body));
  TEST_ASSERT_EQUAL_UINT32(seq, h.sequence);
  if (!payload.empty()) TEST_ASSERT_EQUAL_HEX8_ARRAY(payload.data(), body, payload.size());
}

}  // namespace

static void test_sequences_start_at_one_and_read_back() {
  uint8_t storage[1024];
  MsgRing::Slot slots[16];
  MsgRing ring(storage, sizeof storage, slots, 16);
  TEST_ASSERT_EQUAL_UINT32(0, ring.oldest());
  TEST_ASSERT_EQUAL_UINT32(0, ring.last());
  for (uint32_t i = 1; i <= 5; i++) {
    const auto p = payloadFor(i, 30 + i);
    TEST_ASSERT_EQUAL_UINT32(i, ring.append(MsgType::RawSampleBatch, i * 10, p.data(), p.size()));
  }
  TEST_ASSERT_EQUAL_UINT32(1, ring.oldest());
  TEST_ASSERT_EQUAL_UINT32(5, ring.last());
  for (uint32_t i = 1; i <= 5; i++) assertStored(ring, i, payloadFor(i, 30 + i));
  uint8_t out[64];
  TEST_ASSERT_EQUAL_size_t(0u, ring.read(6, out, sizeof out));
  TEST_ASSERT_EQUAL_size_t(0u, ring.read(0, out, sizeof out));
  TEST_ASSERT_EQUAL_size_t(0u, ring.read(5, out, 10));  // buffer too small
}

static void test_evicts_oldest_when_bytes_run_out() {
  uint8_t storage[200];
  MsgRing::Slot slots[64];
  MsgRing ring(storage, sizeof storage, slots, 64);
  // Each message is 20 + 40 = 60 bytes, so at most 3 fit.
  for (uint32_t i = 1; i <= 7; i++) {
    const auto p = payloadFor(i, 40);
    ring.append(MsgType::RawSampleBatch, 0, p.data(), p.size());
    TEST_ASSERT_TRUE(ring.usedBytes() <= sizeof storage);
  }
  TEST_ASSERT_EQUAL_UINT32(5, ring.oldest());
  TEST_ASSERT_EQUAL_UINT32(7, ring.last());
  uint8_t out[64];
  TEST_ASSERT_EQUAL_size_t(0u, ring.read(4, out, sizeof out));
  for (uint32_t i = 5; i <= 7; i++) assertStored(ring, i, payloadFor(i, 40));
}

static void test_evicts_oldest_when_slots_run_out() {
  uint8_t storage[4096];
  MsgRing::Slot slots[4];
  MsgRing ring(storage, sizeof storage, slots, 4);
  for (uint32_t i = 1; i <= 10; i++) {
    const auto p = payloadFor(i, 8);
    ring.append(MsgType::EventBatch, 0, p.data(), p.size());
  }
  TEST_ASSERT_EQUAL_UINT32(7, ring.oldest());
  TEST_ASSERT_EQUAL_UINT32(10, ring.last());
  for (uint32_t i = 7; i <= 10; i++) assertStored(ring, i, payloadFor(i, 8));
}

static void test_rejects_message_larger_than_storage() {
  uint8_t storage[64];
  MsgRing::Slot slots[4];
  MsgRing ring(storage, sizeof storage, slots, 4);
  const auto p = payloadFor(1, 45);  // 65 bytes with header
  TEST_ASSERT_EQUAL_UINT32(0, ring.append(MsgType::StepBatch, 0, p.data(), p.size()));
  TEST_ASSERT_EQUAL_UINT32(0, ring.last());
}

// Random sizes against a reference model, with enough appends to wrap the
// byte storage many times at odd offsets.
static void test_matches_reference_model_across_wraps() {
  const size_t storageCap = 1537;
  std::vector<uint8_t> storage(storageCap);
  std::vector<MsgRing::Slot> slots(40);
  MsgRing ring(storage.data(), storageCap, slots.data(), slots.size());

  struct Stored {
    uint32_t seq;
    std::vector<uint8_t> payload;
  };
  std::deque<Stored> model;
  size_t modelBytes = 0;
  uint32_t lcg = 777;
  for (uint32_t i = 1; i <= 5000; i++) {
    lcg = lcg * 1103515245u + 12345u;
    const size_t len = (lcg >> 16) % 300;
    const auto p = payloadFor(i, len);
    TEST_ASSERT_EQUAL_UINT32(i, ring.append(MsgType::RawSampleBatch, i, p.data(), p.size()));
    model.push_back({i, p});
    modelBytes += ead::kHeaderSize + len;
    while (modelBytes > storageCap || model.size() > slots.size()) {
      modelBytes -= ead::kHeaderSize + model.front().payload.size();
      model.pop_front();
    }
    TEST_ASSERT_EQUAL_UINT32(model.front().seq, ring.oldest());
    TEST_ASSERT_EQUAL_size_t(modelBytes, ring.usedBytes());
    if (i % 97 == 0) {
      for (const auto& s : model) assertStored(ring, s.seq, s.payload);
    }
  }
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_sequences_start_at_one_and_read_back);
  RUN_TEST(test_evicts_oldest_when_bytes_run_out);
  RUN_TEST(test_evicts_oldest_when_slots_run_out);
  RUN_TEST(test_rejects_message_larger_than_storage);
  RUN_TEST(test_matches_reference_model_across_wraps);
  return UNITY_END();
}
