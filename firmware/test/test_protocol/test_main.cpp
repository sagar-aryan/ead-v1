#include <unity.h>

#include <algorithm>
#include <cstring>
#include <initializer_list>
#include <string>
#include <vector>

#include "../vectors.h"
#include "ead/config_section.h"
#include "ead/calibration.h"
#include "ead/gait.h"
#include "ead/protocol.h"

using ead::MsgType;

void setUp() {}
void tearDown() {}

static void assertBytes(const std::vector<uint8_t>& expected, const uint8_t* actual, size_t n) {
  TEST_ASSERT_EQUAL_size_t(expected.size(), n);
  TEST_ASSERT_EQUAL_HEX8_ARRAY(expected.data(), actual, n);
}

static std::vector<uint8_t> message(MsgType type, uint32_t seq, uint64_t time,
                                    const uint8_t* payload, size_t len) {
  std::vector<uint8_t> out(ead::kHeaderSize + len);
  const size_t n = ead::encodeMessage(type, seq, time, payload, len, out.data(), out.size());
  TEST_ASSERT_EQUAL_size_t(out.size(), n);
  return out;
}

static void test_host_hello_request() {
  const auto v = loadVector("hello_request.hex");
  ead::Header h;
  const uint8_t* payload;
  TEST_ASSERT_TRUE(ead::parseMessage(v.data(), v.size(), &h, &payload));
  TEST_ASSERT_EQUAL_UINT8(uint8_t(MsgType::Hello), h.type);
  TEST_ASSERT_EQUAL_UINT32(7, h.sequence);
  uint16_t schema = 0;
  TEST_ASSERT_TRUE(ead::decodeHelloRequest(payload, h.length, &schema));
  TEST_ASSERT_EQUAL_UINT16(ead::kSchemaVersion, schema);
  TEST_ASSERT_FALSE(ead::decodeHelloRequest(payload, 3, &schema));
}

static void test_device_hello() {
  ead::HelloInfo info{};
  info.schema = ead::kSchemaVersion;
  info.device_state = ead::kStateReady;
  info.reset_reason = 1;
  info.boot_id = 0xA1B2C3D4u;
  const uint8_t mac[6] = {0x44, 0xB1, 0x76, 0xAF, 0xFB, 0x7C};
  std::memcpy(info.mac, mac, 6);
  info.who_foot = 0x70;
  info.who_shank = 0x70;
  info.capabilities = ead::kCapPsramRing;
  const auto v = loadVector("hello_info.hex");
  // The vector's SHA-256 bytes sit at payload offset 17.
  std::memcpy(info.config_sha256, v.data() + ead::kHeaderSize + 17, 32);
  info.oldest_seq = 1;
  info.last_seq = 42;
  info.fw_version = "0.1.0+test";

  uint8_t payload[128];
  const size_t n = ead::encodeHelloPayload(info, payload, sizeof payload);
  const auto msg = message(MsgType::Hello, 42, 123456789, payload, n);
  assertBytes(v, msg.data(), msg.size());
}

static void test_device_status() {
  ead::StatusInfo s{};
  s.device_state = ead::kStateReady;
  s.link_flags = ead::kLinkUsbActive;
  s.faults = ead::kFaultShankFrozen | ead::kFaultAcquisitionStalled;
  s.frame_index = 123456;
  s.frames_dropped = 3;
  s.shank_repeated = 17;
  s.i2c_errors = 2;
  s.imu_reinits = 1;
  s.oldest_seq = 1;
  s.last_seq = 42;
  s.ap_rssi_dbm = -47;
  s.ap_stations = 1;
  s.heap_free_min = 201000;
  s.stack_free_acquisition = 1500;
  s.stack_free_processing = 2600;
  s.stack_free_usb = 3100;
  s.stack_free_wifi = 4200;
  s.calibration_state = uint8_t(ead::CalibrationState::Ready);
  s.calibration_samples = 500;
  s.calibration_reject = 0;
  s.gait_state = uint8_t(ead::GaitState::FootFlatZv);
  s.cycles_completed = 37;
  uint8_t payload[ead::kStatusPayloadSize];
  const size_t n = ead::encodeStatusPayload(s, payload, sizeof payload);
  TEST_ASSERT_EQUAL_size_t(ead::kStatusPayloadSize, n);
  const auto msg = message(MsgType::Status, 42, 987654321, payload, n);
  assertBytes(loadVector("status.hex"), msg.data(), msg.size());
  TEST_ASSERT_EQUAL_size_t(0u, ead::encodeStatusPayload(s, payload, sizeof payload - 1));
}

static ead::RawFrame frame(uint64_t ts, uint32_t index, std::initializer_list<int16_t> foot,
                           std::initializer_list<int16_t> shank,
                           std::initializer_list<int16_t> qFoot,
                           std::initializer_list<int16_t> qShank, uint16_t status) {
  ead::RawFrame f{};
  f.timestamp_us = ts;
  f.frame_index = index;
  std::copy(foot.begin(), foot.end(), f.foot);
  std::copy(shank.begin(), shank.end(), f.shank);
  std::copy(qFoot.begin(), qFoot.end(), f.q_foot);
  std::copy(qShank.begin(), qShank.end(), f.q_shank);
  f.status = status;
  return f;
}

static const ead::RawFrame kFrames[2] = {
    frame(1000000, 100, {8192, -8192, 32767, -32768, 1, -1}, {0, 16, -16, 655, -655, 32767},
          {32767, 0, 0, 0}, {32767, 0, 0, 0},
          ead::kRawFootReadFail | ead::kRawFootGyroSaturated | ead::kRawShankAccelSaturated),
    frame(1010000, 101, {-1, -2, -3, -4, -5, -6}, {1, 2, 3, 4, 5, 6}, {32767, 0, 0, 0},
          {0, -32767, 12345, -12345}, ead::kRawShankRepeated),
};

static void test_raw_batch_encode_and_decode() {
  uint8_t payload[2 + 2 * ead::kRawFrameSize];
  const size_t n = ead::encodeRawBatchPayload(kFrames, 2, payload, sizeof payload);
  TEST_ASSERT_EQUAL_size_t(sizeof payload, n);
  const auto msg = message(MsgType::RawSampleBatch, 3, 1000000, payload, n);
  const auto v = loadVector("raw_batch.hex");
  assertBytes(v, msg.data(), msg.size());

  ead::ByteReader r(v.data() + ead::kHeaderSize + 2, v.size() - ead::kHeaderSize - 2);
  for (const auto& expected : kFrames) {
    ead::RawFrame got{};
    TEST_ASSERT_TRUE(ead::readRawFrame(r, &got));
    TEST_ASSERT_EQUAL_UINT64(expected.timestamp_us, got.timestamp_us);
    TEST_ASSERT_EQUAL_UINT32(expected.frame_index, got.frame_index);
    TEST_ASSERT_EQUAL_INT16_ARRAY(expected.foot, got.foot, 6);
    TEST_ASSERT_EQUAL_INT16_ARRAY(expected.shank, got.shank, 6);
    TEST_ASSERT_EQUAL_INT16_ARRAY(expected.q_shank, got.q_shank, 4);
    TEST_ASSERT_EQUAL_UINT16(expected.status, got.status);
  }
  TEST_ASSERT_EQUAL_size_t(0u, r.remaining());
  TEST_ASSERT_EQUAL_size_t(0u, ead::encodeRawBatchPayload(kFrames, 0, payload, sizeof payload));
}

static void test_error_message() {
  uint8_t payload[64];
  const size_t n = ead::encodeErrorPayload(9, uint8_t(MsgType::SessionStart),
                                           ead::ErrorCode::NotSupported,
                                           "SESSION_START not supported", payload, sizeof payload);
  const auto msg = message(MsgType::Error, 42, 5, payload, n);
  assertBytes(loadVector("error.hex"), msg.data(), msg.size());
}

static void test_backfill_request_and_data() {
  const auto req = loadVector("backfill_request.hex");
  ead::Header h;
  const uint8_t* payload;
  TEST_ASSERT_TRUE(ead::parseMessage(req.data(), req.size(), &h, &payload));
  uint32_t first = 0, last = 0;
  TEST_ASSERT_TRUE(ead::decodeBackfillRequest(payload, h.length, &first, &last));
  TEST_ASSERT_EQUAL_UINT32(100, first);
  TEST_ASSERT_EQUAL_UINT32(250, last);

  const uint8_t zeroFirst[8] = {0, 0, 0, 0, 5, 0, 0, 0};
  TEST_ASSERT_FALSE(ead::decodeBackfillRequest(zeroFirst, 8, &first, &last));
  const uint8_t reversed[8] = {9, 0, 0, 0, 5, 0, 0, 0};
  TEST_ASSERT_FALSE(ead::decodeBackfillRequest(reversed, 8, &first, &last));

  std::vector<uint8_t> body(ead::kBackfillPrefixSize);
  TEST_ASSERT_EQUAL_size_t(ead::kBackfillPrefixSize,
                           ead::encodeBackfillPrefix(11, 3, 4, false, body.data(), body.size()));
  uint8_t p3[2 + 2 * ead::kRawFrameSize];
  size_t n3 = ead::encodeRawBatchPayload(kFrames, 2, p3, sizeof p3);
  const auto m3 = message(MsgType::RawSampleBatch, 3, 1000000, p3, n3);
  uint8_t p4[2 + ead::kRawFrameSize];
  size_t n4 = ead::encodeRawBatchPayload(kFrames + 1, 1, p4, sizeof p4);
  const auto m4 = message(MsgType::RawSampleBatch, 4, 1010000, p4, n4);
  body.insert(body.end(), m3.begin(), m3.end());
  body.insert(body.end(), m4.begin(), m4.end());
  const auto msg = message(MsgType::BackfillData, 42, 6, body.data(), body.size());
  assertBytes(loadVector("backfill_data.hex"), msg.data(), msg.size());
}

static void test_config_section_matches_contract_json() {
  uint8_t section[256];
  const size_t n = ead::encodeConfigSection(section, sizeof section);
  assertBytes(loadVector("config_section.hex"), section, n);

  const auto response = loadVector("config_response.hex");
  uint8_t payload[512];
  const size_t p = ead::encodeConfigPayload(ead::kConfigFormat, response.data() + 2, section, n,
                                            payload, sizeof payload);
  assertBytes(response, payload, p);
}

static void test_parse_rejects_bad_headers() {
  auto v = loadVector("hello_request.hex");
  ead::Header h;
  const uint8_t* payload;
  TEST_ASSERT_FALSE(ead::parseMessage(v.data(), v.size() - 1, &h, &payload));
  TEST_ASSERT_FALSE(ead::parseMessage(v.data(), 10, &h, &payload));
  v[0] = 2;  // protocol version
  TEST_ASSERT_FALSE(ead::parseMessage(v.data(), v.size(), &h, &payload));
}

static void test_usb_frame_encoding() {
  const auto hello = loadVector("hello_request.hex");
  uint8_t frame[ead::kMaxUsbFrameSize];
  size_t n = ead::encodeUsbFrame(hello.data(), hello.size(), frame, sizeof frame);
  assertBytes(loadVector("usb_frame_hello_request.hex"), frame, n);

  const auto longMsg = loadVector("long_message.hex");
  n = ead::encodeUsbFrame(longMsg.data(), longMsg.size(), frame, sizeof frame);
  assertBytes(loadVector("usb_frame_long.hex"), frame, n);
}

static void test_usb_decoder_skips_noise_and_corruption() {
  const auto helloFrame = loadVector("usb_frame_hello_request.hex");
  const auto longFrame = loadVector("usb_frame_long.hex");
  auto corrupted = helloFrame;
  corrupted[5] ^= 0x40;  // flips a message byte, so the CRC no longer matches

  std::vector<uint8_t> stream;
  const std::string rom = "ESP-ROM:esp32s3-20210327\r\nentry 0x403c98d0\r\n";
  stream.insert(stream.end(), rom.begin(), rom.end());
  stream.insert(stream.end(), helloFrame.begin(), helloFrame.end());
  stream.insert(stream.end(), corrupted.begin(), corrupted.end());
  stream.insert(stream.end(), longFrame.begin(), longFrame.end());

  auto* decoder = new ead::UsbFrameDecoder();
  std::vector<std::vector<uint8_t>> messages;
  for (uint8_t b : stream) {
    if (decoder->feed(b)) {
      messages.emplace_back(decoder->message(), decoder->message() + decoder->length());
    }
  }
  TEST_ASSERT_EQUAL_size_t(2u, messages.size());
  const auto hello = loadVector("hello_request.hex");
  assertBytes(hello, messages[0].data(), messages[0].size());
  const auto longMsg = loadVector("long_message.hex");
  assertBytes(longMsg, messages[1].data(), messages[1].size());
  // ROM text before the first delimiter, plus the corrupted frame.
  TEST_ASSERT_EQUAL_UINT32(2u, decoder->rejected());
  delete decoder;
}

static void test_session_start_round_trip() {
  const auto v = loadVector("session_start.hex");
  uint8_t payload[ead::kSessionStartPayloadSize];
  const size_t n = ead::encodeSessionStart(uint8_t(ead::SessionKind::Calibration), 5000, payload,
                                           sizeof payload);
  const auto msg = message(MsgType::SessionStart, 12, 0, payload, n);
  assertBytes(v, msg.data(), msg.size());

  uint8_t kind = 0;
  uint16_t durationMs = 0;
  TEST_ASSERT_TRUE(ead::decodeSessionStart(payload, n, &kind, &durationMs));
  TEST_ASSERT_EQUAL_UINT8(1, kind);
  TEST_ASSERT_EQUAL_UINT16(5000, durationMs);
  // A payload of the wrong length is rejected rather than read short.
  TEST_ASSERT_FALSE(ead::decodeSessionStart(payload, n - 1, &kind, &durationMs));
}

static void test_calibration_record_round_trip() {
  const auto v = loadVector("calibration_record.hex");
  const uint8_t* payload = v.data() + ead::kHeaderSize;
  const size_t len = v.size() - ead::kHeaderSize;
  TEST_ASSERT_EQUAL_size_t(ead::kCalibrationPayloadSize, len);

  uint8_t kind = 0;
  ead::CalibrationRecord record{};
  TEST_ASSERT_TRUE(ead::decodeCalibrationPayload(payload, len, &kind, &record));
  TEST_ASSERT_EQUAL_UINT8(1, kind);
  TEST_ASSERT_EQUAL_UINT16(0, record.reject);
  TEST_ASSERT_EQUAL_UINT32(500, record.samples);
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, 1.5f, record.foot.gyroBiasDps[0]);
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, -0.5f, record.foot.gyroBiasDps[1]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 33.0f, record.foot.tiltDeg);
  TEST_ASSERT_FLOAT_WITHIN(1e-6f, -0.75f, record.shank.gyroBiasDps[0]);
  TEST_ASSERT_FLOAT_WITHIN(1e-4f, 1.3f, record.shank.tiltDeg);

  uint8_t out[ead::kCalibrationPayloadSize];
  const size_t n = ead::encodeCalibrationPayload(kind, record, out, sizeof out);
  assertBytes(v, message(MsgType::SessionStop, 43, 987654321, out, n).data(),
              ead::kHeaderSize + n);
}

static void test_event_batch_matches_the_vector() {
  const ead::GaitEvent events[3] = {
      {ead::GaitEventType::InitialContact, 12'000'000, 1200},
      {ead::GaitEventType::FootFlat, 12'060'000, 1206},
      {ead::GaitEventType::ToeOff, 12'600'000, 1260},
  };
  uint8_t payload[2 + 3 * ead::kEventRecordSize];
  const size_t n = ead::encodeEventBatchPayload(events, 3, payload, sizeof payload);
  TEST_ASSERT_EQUAL_size_t(sizeof payload, n);
  assertBytes(loadVector("event_batch.hex"),
              message(MsgType::EventBatch, 51, 12'000'000, payload, n).data(),
              ead::kHeaderSize + n);
  // An oversized batch is refused rather than truncated.
  TEST_ASSERT_EQUAL_size_t(0u, ead::encodeEventBatchPayload(events, 3, payload, n - 1));
}

static void test_step_batch_matches_the_vector() {
  ead::GaitCycle cycle{};
  cycle.startFrame = 1200;
  cycle.endFrame = 1300;
  cycle.startUs = 12'000'000;
  cycle.cycleTimeS = 1.02f;
  cycle.stanceTimeS = 0.63f;
  cycle.swingTimeS = 0.39f;
  cycle.stanceRatio = 0.6176f;
  cycle.swingRatio = 0.3824f;
  cycle.cadenceStepsPerMin = 117.65f;
  cycle.peakShankRateDps = 412.5f;
  cycle.peakDorsiflexionDeg = 14.2f;
  cycle.contactSagittalDeg = -6.8f;
  cycle.peakInversionDeg = 3.1f;
  cycle.distanceM = 1.41f;
  cycle.speedMps = 1.382f;
  cycle.zuptQuality = 0.58f;
  cycle.valid = true;

  uint8_t payload[2 + ead::kCycleRecordSize];
  const size_t n = ead::encodeStepBatchPayload(&cycle, 1, payload, sizeof payload);
  TEST_ASSERT_EQUAL_size_t(sizeof payload, n);
  assertBytes(loadVector("step_batch.hex"),
              message(MsgType::StepBatch, 52, 12'000'000, payload, n).data(),
              ead::kHeaderSize + n);
}

int main() {
  UNITY_BEGIN();
  RUN_TEST(test_event_batch_matches_the_vector);
  RUN_TEST(test_step_batch_matches_the_vector);
  RUN_TEST(test_session_start_round_trip);
  RUN_TEST(test_calibration_record_round_trip);
  RUN_TEST(test_host_hello_request);
  RUN_TEST(test_device_hello);
  RUN_TEST(test_device_status);
  RUN_TEST(test_raw_batch_encode_and_decode);
  RUN_TEST(test_error_message);
  RUN_TEST(test_backfill_request_and_data);
  RUN_TEST(test_config_section_matches_contract_json);
  RUN_TEST(test_parse_rejects_bad_headers);
  RUN_TEST(test_usb_frame_encoding);
  RUN_TEST(test_usb_decoder_skips_noise_and_corruption);
  return UNITY_END();
}
