#include "link.h"

#include "acquisition.h"
#include "calibration_service.h"
#include "feedback.h"
#include "motors.h"
#include "session_service.h"

#include <algorithm>
#include <cstdio>
#include <cstring>

#include "device.h"
#include "telemetry.h"

using ead::ErrorCode;
using ead::MsgType;

namespace {

constexpr int64_t kHostTimeoutUs = 3000000;
constexpr int64_t kStatusPeriodUs = 200000;  // 5 Hz

// Why a pulse was refused, for the ERROR detail a person reads.
const char* refusalText(ead::MotorGuard::Refusal refusal) {
  using Refusal = ead::MotorGuard::Refusal;
  switch (refusal) {
    case Refusal::BadMotor: return "motor must be 1..6";
    case Refusal::Disabled:
      return "this motor's output is switched off in this firmware";
    case Refusal::BadDuty: return "duty must be 51..204 of 255 (20-80 %)";
    case Refusal::BadDuration: return "duration must be 100..5000 ms";
    case Refusal::Busy: return "another motor is pulsing: one at a time";
    case Refusal::RollingLimit: return "rolling limit: at most 5 s on per motor in any 10 s";
    case Refusal::None: break;
  }
  return "";
}

uint32_t latestSequence() {
  uint32_t oldest = 0, last = 0;
  telemetry::window(&oldest, &last);
  return last;
}

}  // namespace

void Link::onConnect(int64_t nowUs) {
  onDisconnect();
  lastRxUs_ = nowUs;
}

void Link::onDisconnect() {
  replyHead_ = 0;
  replyCount_ = 0;
  helloReceived_ = false;
  backfillActive_ = false;
  pending_ = Source::None;
}

bool Link::streaming(int64_t nowUs) const {
  return helloReceived_ && nowUs - lastRxUs_ < kHostTimeoutUs;
}

void Link::onMessage(const uint8_t* msg, size_t len, int64_t nowUs) {
  ead::Header h;
  const uint8_t* payload = nullptr;
  if (!ead::parseMessage(msg, len, &h, &payload)) {
    queueError(0, 0, ErrorCode::BadFrame, "invalid header or length", nowUs);
    return;
  }
  lastRxUs_ = nowUs;

  switch (MsgType(h.type)) {
    case MsgType::Hello: {
      uint16_t schema = 0;
      if (!ead::decodeHelloRequest(payload, h.length, &schema)) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload, "HELLO payload is u16 schema", nowUs);
        return;
      }
      helloReceived_ = true;
      backfillActive_ = false;
      // A record completed while no host was listening belongs to the previous
      // session: discard it, or this host would receive it as though it were
      // the answer to its own SESSION_START. STATUS still reports that a record
      // is held, so nothing is hidden.
      calibration::takeCompletion();
      cursor_ = latestSequence() + 1;
      nextStatusUs_ = nowUs;
      ead::HelloInfo info;
      device::fillHello(&info);
      uint8_t body[128];
      queueReply(MsgType::Hello, body, ead::encodeHelloPayload(info, body, sizeof body), nowUs);
      if (schema != ead::kSchemaVersion) {
        queueError(h.sequence, h.type, ErrorCode::SchemaMismatch, "device payload schema differs from the host's",
                   nowUs);
      }
      return;
    }
    case MsgType::Status:
      return;  // host keepalive
    case MsgType::ConfigGet: {
      uint8_t body[kReplyCap - ead::kHeaderSize];
      queueReply(MsgType::ConfigGet, body, device::encodeConfigResponse(body, sizeof body), nowUs);
      return;
    }
    case MsgType::SessionStart: {
      uint8_t kind = 0;
      uint16_t durationMs = 0;
      if (!ead::decodeSessionStart(payload, h.length, &kind, &durationMs)) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload,
                   "SESSION_START is {u8 kind, u8 reserved, u16 duration_ms}", nowUs);
        return;
      }
      const ead::SessionKind sessionKind = ead::SessionKind(kind);
      const bool carriesProfile = h.length != ead::kSessionStartPayloadSize;
      if (carriesProfile && (sessionKind == ead::SessionKind::Calibration ||
                             sessionKind == ead::SessionKind::ReferenceCapture)) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload,
                   "only a check or an evaluation carries a reference profile", nowUs);
        return;
      }
      if (sessionKind == ead::SessionKind::Calibration) {
        if (durationMs < ead::kCalibMinDurationMs || durationMs > ead::kCalibMaxDurationMs) {
          queueError(h.sequence, h.type, ErrorCode::BadPayload, "duration_ms must be 2000..30000",
                     nowUs);
          return;
        }
        if (!calibration::start(durationMs)) {
          queueError(h.sequence, h.type, ErrorCode::InvalidState, "a calibration window is running",
                     nowUs);
        }
        return;
      }
      if (sessionKind != ead::SessionKind::ReferenceCapture &&
          sessionKind != ead::SessionKind::ReferenceCheck &&
          sessionKind != ead::SessionKind::Evaluation) {
        queueError(h.sequence, h.type, ErrorCode::NotSupported, "unknown session kind", nowUs);
        return;
      }
      // A check or an evaluation is meaningless without the profile it judges
      // against, so the reference travels with the command rather than being
      // assumed to be the last one seen.
      ead::ReferenceProfile reference{};
      const bool needsReference = sessionKind != ead::SessionKind::ReferenceCapture;
      if (needsReference) {
        if (h.length != ead::kSessionStartPayloadSize + ead::kReferencePayloadSize ||
            !ead::decodeReferenceProfile(payload + ead::kSessionStartPayloadSize,
                                         ead::kReferencePayloadSize, &reference)) {
          queueError(h.sequence, h.type, ErrorCode::BadPayload,
                     "a check or evaluation needs a valid reference profile", nowUs);
          return;
        }
      }
      if (!session::start(sessionKind, needsReference ? &reference : nullptr)) {
        queueError(h.sequence, h.type, ErrorCode::InvalidState, "a session is already running",
                   nowUs);
      }
      return;
    }
    case MsgType::SessionStop: {
      if (h.length != 0) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload, "SESSION_STOP takes no payload",
                   nowUs);
        return;
      }
      calibration::cancel();
      ead::ReferenceProfile profile{};
      bool wasCapture = false;
      const bool built = session::stop(&profile, &wasCapture);
      if (wasCapture) {
        if (!built) {
          queueError(h.sequence, h.type, ErrorCode::Rejected,
                     "a reference needs at least 30 valid cycles", nowUs);
          return;
        }
        uint8_t body[ead::kReferencePayloadSize];
        const size_t n = ead::encodeReferenceProfile(profile, body, sizeof body);
        queueReply(MsgType::SessionStop, body, n, nowUs);
      }
      return;
    }
    case MsgType::ServiceTest: {
      ead::ServiceOp op{};
      bool rerun = false;
      ead::MotorPulse pulse{};
      if (!ead::decodeServiceTest(payload, h.length, &op, &rerun, &pulse)) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload,
                   "SERVICE_TEST is {1, rerun} or {2, motor, duty, 0, u16 ms}", nowUs);
        return;
      }
      if (op == ead::ServiceOp::SensorCheck && !rerun) {
        queueSensorCheck(nowUs);
        return;
      }
      // Doc 07 §7: no service test during a session; a calibration needs the
      // leg still and the sensors streaming.
      if (session::active() ||
          calibration::state() == ead::CalibrationState::Collecting) {
        queueError(h.sequence, h.type, ErrorCode::InvalidState,
                   "not during a session or a calibration", nowUs);
        return;
      }
      if (op == ead::ServiceOp::SensorCheck) {
        if (!acquisition::requestCheck(activeFlag_)) {
          queueError(h.sequence, h.type, ErrorCode::InvalidState, "a sensor check is running",
                     nowUs);
        }
        return;  // the report follows when the check finishes (peek)
      }
      const ead::MotorGuard::Refusal refusal = motors::pulse(pulse);
      if (refusal == ead::MotorGuard::Refusal::None) {
        uint8_t body[ead::kMotorPulsePayloadSize];
        queueReply(MsgType::ServiceTest, body,
                   ead::encodeMotorPulsePayload(pulse, body, sizeof body), nowUs);
        return;
      }
      const bool badRequest = refusal == ead::MotorGuard::Refusal::BadMotor ||
                              refusal == ead::MotorGuard::Refusal::BadDuty ||
                              refusal == ead::MotorGuard::Refusal::BadDuration;
      const ErrorCode code = badRequest ? ErrorCode::BadPayload
                             : refusal == ead::MotorGuard::Refusal::Busy ? ErrorCode::InvalidState
                                                                         : ErrorCode::Rejected;
      queueError(h.sequence, h.type, code, refusalText(refusal), nowUs);
      return;
    }
    case MsgType::ConfigSet: {
      ead::ConfigKey key{};
      uint8_t value = 0;
      if (!ead::decodeConfigSet(payload, h.length, &key, &value)) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload,
                   "CONFIG_SET is {u8 key = 1 haptic_feedback, u8 0 or 1}", nowUs);
        return;
      }
      feedback::setEnabled(value != 0);
      uint8_t body[ead::kConfigSetPayloadSize];
      queueReply(MsgType::ConfigSet, body, ead::encodeConfigSet(key, value, body, sizeof body),
                 nowUs);
      return;
    }
    case MsgType::BackfillRequest: {
      uint32_t first = 0, last = 0;
      if (!ead::decodeBackfillRequest(payload, h.length, &first, &last)) {
        queueError(h.sequence, h.type, ErrorCode::BadPayload,
                   "BACKFILL_REQUEST needs 1 <= first <= last", nowUs);
        return;
      }
      startBackfill(h.sequence, h.type, first, last, nowUs);
      return;
    }
    default:
      queueError(h.sequence, h.type, ErrorCode::NotSupported,
                 "message type not supported by this firmware", nowUs);
      return;
  }
}

void Link::startBackfill(uint32_t cmdSeq, uint8_t cmdType, uint32_t first, uint32_t last,
                         int64_t nowUs) {
  uint32_t oldest = 0, newest = 0;
  telemetry::window(&oldest, &newest);
  const uint32_t from = std::max(first, oldest);
  const uint32_t to = std::min(last, newest);
  if (oldest == 0 || from > to) {
    queueError(cmdSeq, cmdType, ErrorCode::BackfillUnavailable, "requested range is not stored",
               nowUs);
    return;
  }
  if (from != first || to != last) {
    char detail[64];
    std::snprintf(detail, sizeof detail, "serving %u..%u", unsigned(from), unsigned(to));
    queueError(cmdSeq, cmdType, ErrorCode::BackfillUnavailable, detail, nowUs);
  }
  backfillActive_ = true;
  backfillCmd_ = cmdSeq;
  backfillNext_ = from;
  backfillLast_ = to;
}

void Link::queueReply(MsgType type, const uint8_t* payload, size_t len, int64_t nowUs) {
  if (replyCount_ == kReplySlots || len == 0) return;  // host retries on timeout
  Reply& r = replies_[(replyHead_ + replyCount_) % kReplySlots];
  const size_t n =
      ead::encodeMessage(type, latestSequence(), uint64_t(nowUs), payload, len, r.data, kReplyCap);
  if (n == 0) return;
  r.len = uint16_t(n);
  replyCount_++;
}

void Link::queueSensorCheck(int64_t nowUs) {
  ead::SensorCheck foot{};
  ead::SensorCheck shank{};
  acquisition::latestCheck(&foot, &shank);
  uint8_t body[ead::kSensorCheckPayloadSize];
  queueReply(MsgType::ServiceTest, body,
             ead::encodeSensorCheckPayload(foot, shank, body, sizeof body), nowUs);
}

void Link::queueError(uint32_t cmdSeq, uint8_t cmdType, ErrorCode code, const char* detail,
                      int64_t nowUs) {
  uint8_t body[128];
  queueReply(MsgType::Error, body,
             ead::encodeErrorPayload(cmdSeq, cmdType, code, detail, body, sizeof body), nowUs);
}

size_t Link::peek(uint8_t* out, size_t cap, int64_t nowUs) {
  pending_ = Source::None;
  if (replyCount_ > 0) {
    const Reply& r = replies_[replyHead_];
    if (r.len > cap) return 0;
    std::memcpy(out, r.data, r.len);
    pending_ = Source::Reply;
    return r.len;
  }
  if (!streaming(nowUs)) return 0;

  if (acquisition::takeCheckDone(activeFlag_)) {
    queueSensorCheck(nowUs);  // goes out on the next peek
  }

  if (calibration::takeCompletion()) {
    ead::CalibrationRecord record;
    if (calibration::record(&record)) {
      uint8_t body[ead::kCalibrationPayloadSize];
      const size_t n = ead::encodeCalibrationPayload(uint8_t(ead::SessionKind::Calibration), record,
                                                     body, sizeof body);
      // Not durable: the record is held in RAM and can be asked for again by
      // reading STATUS, so a missed one is not a gap in the data.
      const size_t len =
          ead::encodeMessage(MsgType::SessionStop, 0, uint64_t(nowUs), body, n, out, cap);
      if (len > 0) {
        pending_ = Source::Status;
        return len;
      }
    }
  }

  if (nowUs >= nextStatusUs_) {
    ead::StatusInfo status;
    device::fillStatus(&status);
    uint8_t body[ead::kStatusPayloadSize];
    const size_t n = ead::encodeStatusPayload(status, body, sizeof body);
    const size_t len =
        ead::encodeMessage(MsgType::Status, status.last_seq, uint64_t(nowUs), body, n, out, cap);
    if (len > 0) pending_ = Source::Status;
    return len;
  }

  uint32_t oldest = 0, last = 0;
  telemetry::window(&oldest, &last);
  if (oldest != 0 && cursor_ <= last) {
    // Messages evicted before this link could send them are skipped; the host
    // sees the sequence gap.
    if (cursor_ < oldest) cursor_ = oldest;
    const size_t len = telemetry::read(cursor_, out, cap);
    if (len > 0) {
      pending_ = Source::Live;
      pendingSeq_ = cursor_;
      return len;
    }
  }

  if (backfillActive_) {
    const size_t len = buildBackfill(out, cap, nowUs);
    if (len > 0) pending_ = Source::Backfill;
    return len;
  }
  return 0;
}

size_t Link::buildBackfill(uint8_t* out, size_t cap, int64_t nowUs) {
  uint32_t oldest = 0, last = 0;
  telemetry::window(&oldest, &last);
  if (oldest != 0 && backfillNext_ < oldest) backfillNext_ = oldest;
  const uint32_t end = std::min(backfillLast_, last);
  if (oldest == 0 || backfillNext_ > end) {
    backfillActive_ = false;
    return 0;
  }

  const size_t limit = std::min(cap, ead::kMaxMessageSize);
  size_t pos = ead::kHeaderSize + ead::kBackfillPrefixSize;
  uint32_t seq = backfillNext_;
  while (seq <= end) {
    const size_t len = telemetry::lengthOf(seq);
    if (len == 0 || pos + len > limit) break;
    if (telemetry::read(seq, out + pos, limit - pos) != len) break;
    pos += len;
    seq++;
  }
  if (seq == backfillNext_) {
    backfillActive_ = false;  // first message evicted between window() and read()
    return 0;
  }

  const uint32_t chunkLast = seq - 1;
  ead::encodeBackfillPrefix(backfillCmd_, backfillNext_, chunkLast, chunkLast < end,
                            out + ead::kHeaderSize, ead::kBackfillPrefixSize);
  ead::ByteWriter w(out, ead::kHeaderSize);
  ead::writeHeader(w, ead::Header{ead::kProtocolVersion, uint8_t(MsgType::BackfillData), 0,
                                  uint32_t(pos - ead::kHeaderSize), last, uint64_t(nowUs)});
  pendingSeq_ = chunkLast;
  return pos;
}

void Link::commit(int64_t nowUs) {
  switch (pending_) {
    case Source::Reply:
      replyHead_ = (replyHead_ + 1) % kReplySlots;
      replyCount_--;
      break;
    case Source::Status:
      nextStatusUs_ = nowUs + kStatusPeriodUs;
      break;
    case Source::Live:
      cursor_ = pendingSeq_ + 1;
      break;
    case Source::Backfill:
      backfillNext_ = pendingSeq_ + 1;
      if (backfillNext_ > backfillLast_) backfillActive_ = false;
      break;
    case Source::None:
      break;
  }
  pending_ = Source::None;
}
