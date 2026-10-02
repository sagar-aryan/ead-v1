#include <unity.h>

#include <vector>

#include "ead/sh2.h"

using namespace ead::sh2;

void setUp() {}
void tearDown() {}

static void test_header_accepts_a_packet_and_an_empty_read() {
  const uint8_t packet[4] = {0x1B, 0x80, kChannelInput, 7};  // 27 bytes, continuation
  Header h{};
  TEST_ASSERT_TRUE(parseHeader(packet, 512, &h));
  TEST_ASSERT_EQUAL_UINT16(27, h.length);
  TEST_ASSERT_TRUE(h.continuation);
  TEST_ASSERT_EQUAL_UINT8(kChannelInput, h.channel);
  TEST_ASSERT_EQUAL_UINT8(7, h.sequence);

  const uint8_t empty[4] = {0, 0, 0, 0};
  TEST_ASSERT_TRUE(parseHeader(empty, 512, &h));
  TEST_ASSERT_EQUAL_UINT16(0, h.length);
}

static void test_header_rejects_what_an_undriven_or_wrong_line_reads() {
  Header h{};
  const uint8_t floating[4] = {0xFF, 0xFF, 0xFF, 0xFF};  // MISO pulled up, nothing driving
  TEST_ASSERT_FALSE(parseHeader(floating, 512, &h));
  const uint8_t shortLength[4] = {3, 0, kChannelControl, 0};
  TEST_ASSERT_FALSE(parseHeader(shortLength, 512, &h));
  const uint8_t badChannel[4] = {8, 0, 9, 0};
  TEST_ASSERT_FALSE(parseHeader(badChannel, 512, &h));
  const uint8_t tooLong[4] = {0x01, 0x02, kChannelCommand, 0};  // 513
  TEST_ASSERT_FALSE(parseHeader(tooLong, 512, &h));
}

static void test_host_packets_match_the_sh2_layouts() {
  uint8_t out[32];
  const uint8_t productId[] = {6, 0, kChannelControl, 3, 0xF9, 0x00};
  TEST_ASSERT_EQUAL_size_t(sizeof productId, encodeProductIdRequest(3, out, sizeof out));
  TEST_ASSERT_EQUAL_HEX8_ARRAY(productId, out, sizeof productId);

  // Set Feature: id, flags, u16 sensitivity, u32 interval, u32 batch, u32 specific.
  const uint8_t setFeature[] = {21, 0, kChannelControl, 4, 0xFD, kReportGyroscope, 0, 0, 0,
                                0x10, 0x27, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0};
  TEST_ASSERT_EQUAL_size_t(sizeof setFeature,
                           encodeSetFeature(4, kReportGyroscope, 10000, out, sizeof out));
  TEST_ASSERT_EQUAL_HEX8_ARRAY(setFeature, out, sizeof setFeature);
  TEST_ASSERT_EQUAL_size_t(0, encodeSetFeature(4, kReportGyroscope, 10000, out, 20));
}

static void test_control_replies_give_the_product_id_and_enabled_features() {
  // The application entry TEST-043 read from both sensors: part 10004563,
  // version 3.12.6, build 62, reset cause 4; then a second entry.
  const uint8_t cargo[] = {
      0xF8, 4, 3, 12, 0x53, 0xA8, 0x98, 0x00, 62, 0, 0, 0, 6, 0, 0, 0,
      0xF8, 0, 1, 10, 0x96, 0xA4, 0x98, 0x00, 0x94, 0x01, 0, 0, 10, 0, 0, 0,
      0xFC, kReportAccelerometer, 0, 0, 0, 0x10, 0x27, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0xFC, kReportGyroscope, 0, 0, 0, 0x10, 0x27, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  };
  ControlReplies replies{};
  parseControl(cargo, sizeof cargo, &replies);
  TEST_ASSERT_EQUAL_UINT8(2, replies.productIds);
  TEST_ASSERT_EQUAL_UINT32(10004563u, replies.firstProductId.partNumber);
  TEST_ASSERT_EQUAL_UINT8(3, replies.firstProductId.versionMajor);
  TEST_ASSERT_EQUAL_UINT8(12, replies.firstProductId.versionMinor);
  TEST_ASSERT_EQUAL_UINT16(6, replies.firstProductId.versionPatch);
  TEST_ASSERT_EQUAL_UINT32(62, replies.firstProductId.buildNumber);
  TEST_ASSERT_EQUAL_UINT8(4, replies.firstProductId.resetCause);
  TEST_ASSERT_EQUAL_HEX8((1u << kReportAccelerometer) | (1u << kReportGyroscope),
                         replies.featureSensors);
}

static void test_input_reports_carry_values_and_their_own_sample_time() {
  // Base timestamp 120 ticks (12 ms before the INT); an accelerometer report
  // 3.5 ms after that reference; a gyroscope report with a 30 ms delay, whose
  // upper delay bits sit in the status byte above the accuracy.
  const uint8_t cargo[] = {
      0xFB, 120, 0, 0, 0,
      kReportAccelerometer, 9, 0x03, 35, 0x00, 0x01, 0xFF, 0xFF, 0x34, 0x12,
      kReportGyroscope, 200, (0x01 << 2) | 0x02, 0x2C, 0x10, 0x00, 0x00, 0x80, 0xF0, 0xFF,
  };
  Sample s[4];
  bool unknown = true;
  TEST_ASSERT_EQUAL_size_t(2, parseInput(cargo, sizeof cargo, 1000000, s, 4, &unknown));
  TEST_ASSERT_FALSE(unknown);

  TEST_ASSERT_EQUAL_UINT8(kReportAccelerometer, s[0].reportId);
  TEST_ASSERT_EQUAL_UINT8(9, s[0].sequence);
  TEST_ASSERT_EQUAL_UINT8(3, s[0].accuracy);
  TEST_ASSERT_EQUAL_INT64(1000000 - 12000 + 3500, s[0].timeUs);
  TEST_ASSERT_EQUAL_INT16(256, s[0].value[0]);
  TEST_ASSERT_EQUAL_INT16(-1, s[0].value[1]);
  TEST_ASSERT_EQUAL_INT16(0x1234, s[0].value[2]);

  TEST_ASSERT_EQUAL_UINT8(kReportGyroscope, s[1].reportId);
  TEST_ASSERT_EQUAL_UINT8(2, s[1].accuracy);
  TEST_ASSERT_EQUAL_INT64(1000000 - 12000 + 30000, s[1].timeUs);  // delay 0x12C = 300
  TEST_ASSERT_EQUAL_INT16(16, s[1].value[0]);
  TEST_ASSERT_EQUAL_INT16(-32768, s[1].value[1]);
  TEST_ASSERT_EQUAL_INT16(-16, s[1].value[2]);
}

static void test_a_rebase_moves_the_reference_later() {
  const uint8_t cargo[] = {
      0xFB, 100, 0, 0, 0,
      0xFA, 50, 0, 0, 0,
      kReportAccelerometer, 1, 0, 0, 0, 0, 0, 0, 0, 0,
  };
  Sample s[1];
  bool unknown = false;
  TEST_ASSERT_EQUAL_size_t(1, parseInput(cargo, sizeof cargo, 500000, s, 1, &unknown));
  TEST_ASSERT_EQUAL_INT64(500000 - 10000 + 5000, s[0].timeUs);
}

static void test_parsing_stops_at_a_report_of_unknown_length() {
  const uint8_t cargo[] = {
      0xFB, 0, 0, 0, 0,
      kReportAccelerometer, 1, 0, 0, 1, 0, 2, 0, 3, 0,
      0x05, 0, 0, 0,  // a rotation vector: not enabled, length not known here
      kReportGyroscope, 1, 0, 0, 0, 0, 0, 0, 0, 0,
  };
  Sample s[4];
  bool unknown = false;
  TEST_ASSERT_EQUAL_size_t(1, parseInput(cargo, sizeof cargo, 0, s, 4, &unknown));
  TEST_ASSERT_TRUE(unknown);
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_header_accepts_a_packet_and_an_empty_read);
  RUN_TEST(test_header_rejects_what_an_undriven_or_wrong_line_reads);
  RUN_TEST(test_host_packets_match_the_sh2_layouts);
  RUN_TEST(test_control_replies_give_the_product_id_and_enabled_features);
  RUN_TEST(test_input_reports_carry_values_and_their_own_sample_time);
  RUN_TEST(test_a_rebase_moves_the_reference_later);
  RUN_TEST(test_parsing_stops_at_a_report_of_unknown_length);
  return UNITY_END();
}
