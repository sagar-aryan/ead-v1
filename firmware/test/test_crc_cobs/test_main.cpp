#include <unity.h>

#include <cstring>
#include <vector>

#include "ead/cobs.h"
#include "ead/crc32.h"

void setUp() {}
void tearDown() {}

static void test_crc32_check_value() {
  const char* check = "123456789";
  TEST_ASSERT_EQUAL_HEX32(0xCBF43926u, ead::crc32(reinterpret_cast<const uint8_t*>(check), 9));
  TEST_ASSERT_EQUAL_HEX32(0u, ead::crc32(nullptr, 0));
}

static void expectEncode(std::vector<uint8_t> in, std::vector<uint8_t> expected) {
  std::vector<uint8_t> out(ead::cobsMaxEncodedSize(in.size()));
  const size_t n = ead::cobsEncode(in.data(), in.size(), out.data(), out.size());
  TEST_ASSERT_EQUAL_size_t(expected.size(), n);
  TEST_ASSERT_EQUAL_HEX8_ARRAY(expected.data(), out.data(), n);
}

static void expectDecode(std::vector<uint8_t> encoded, std::vector<uint8_t> expected) {
  std::vector<uint8_t> out(encoded.size());
  const size_t n = ead::cobsDecode(encoded.data(), encoded.size(), out.data(), out.size());
  TEST_ASSERT_EQUAL_size_t(expected.size(), n);
  if (n > 0) TEST_ASSERT_EQUAL_HEX8_ARRAY(expected.data(), out.data(), n);
}

// Examples from the COBS article (Cheshire & Baker 1999, as tabulated on Wikipedia).
static void test_cobs_reference_examples() {
  expectEncode({0x00}, {0x01, 0x01});
  expectEncode({0x00, 0x00}, {0x01, 0x01, 0x01});
  expectEncode({0x00, 0x11, 0x00}, {0x01, 0x02, 0x11, 0x01});
  expectEncode({0x11, 0x22, 0x00, 0x33}, {0x03, 0x11, 0x22, 0x02, 0x33});
  expectEncode({0x11, 0x22, 0x33, 0x44}, {0x05, 0x11, 0x22, 0x33, 0x44});
  expectEncode({0x11, 0x00, 0x00, 0x00}, {0x02, 0x11, 0x01, 0x01, 0x01});
  expectDecode({0x01, 0x01}, {0x00});
  expectDecode({0x03, 0x11, 0x22, 0x02, 0x33}, {0x11, 0x22, 0x00, 0x33});
  expectDecode({0x02, 0x11, 0x01, 0x01, 0x01}, {0x11, 0x00, 0x00, 0x00});
}

static void test_cobs_254_byte_block_boundaries() {
  std::vector<uint8_t> run254;
  for (int i = 1; i <= 254; i++) run254.push_back(uint8_t(i));

  // Reference encoder output: full block, then an empty trailing block.
  std::vector<uint8_t> withTrailer = {0xFF};
  withTrailer.insert(withTrailer.end(), run254.begin(), run254.end());
  withTrailer.push_back(0x01);
  expectEncode(run254, withTrailer);

  // Decoders must also accept the shorter form without the trailing block.
  std::vector<uint8_t> withoutTrailer(withTrailer.begin(), withTrailer.end() - 1);
  expectDecode(withTrailer, run254);
  expectDecode(withoutTrailer, run254);

  // 01..FF (255 bytes) -> FF 01..FE 02 FF
  std::vector<uint8_t> run255 = run254;
  run255.push_back(0xFF);
  std::vector<uint8_t> expected = {0xFF};
  expected.insert(expected.end(), run254.begin(), run254.end());
  expected.push_back(0x02);
  expected.push_back(0xFF);
  expectEncode(run255, expected);
}

static void test_cobs_rejects_malformed_input() {
  uint8_t out[16];
  const uint8_t embeddedZero[] = {0x03, 0x11, 0x00};
  TEST_ASSERT_EQUAL_size_t(ead::kCobsError, ead::cobsDecode(embeddedZero, 3, out, sizeof out));
  const uint8_t truncated[] = {0x05, 0x11, 0x22};
  TEST_ASSERT_EQUAL_size_t(ead::kCobsError, ead::cobsDecode(truncated, 3, out, sizeof out));
  const uint8_t tooLong[] = {0x05, 0x11, 0x22, 0x33, 0x44};
  TEST_ASSERT_EQUAL_size_t(ead::kCobsError, ead::cobsDecode(tooLong, 5, out, 3));
  const uint8_t in[] = {0x11, 0x22, 0x33};
  TEST_ASSERT_EQUAL_size_t(0u, ead::cobsEncode(in, 3, out, 3));
}

static void test_cobs_round_trip_deterministic_data() {
  uint32_t lcg = 12345;
  for (size_t size = 0; size <= 1200; size += 7) {
    std::vector<uint8_t> in(size);
    for (auto& b : in) {
      lcg = lcg * 1664525u + 1013904223u;
      const uint8_t v = uint8_t(lcg >> 24);
      b = (v < 40) ? 0 : v;  // ~15% zeros
    }
    std::vector<uint8_t> enc(ead::cobsMaxEncodedSize(size));
    const size_t n = ead::cobsEncode(in.data(), size, enc.data(), enc.size());
    TEST_ASSERT_TRUE(n > 0);
    TEST_ASSERT_NULL(std::memchr(enc.data(), 0, n));
    std::vector<uint8_t> dec(size + 1);
    TEST_ASSERT_EQUAL_size_t(size, ead::cobsDecode(enc.data(), n, dec.data(), dec.size()));
    if (size > 0) TEST_ASSERT_EQUAL_HEX8_ARRAY(in.data(), dec.data(), size);
  }
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_crc32_check_value);
  RUN_TEST(test_cobs_reference_examples);
  RUN_TEST(test_cobs_254_byte_block_boundaries);
  RUN_TEST(test_cobs_rejects_malformed_input);
  RUN_TEST(test_cobs_round_trip_deterministic_data);
  return UNITY_END();
}
