#include "bno086.h"

#include <cstring>

#include <Arduino.h>
#include <SPI.h>
#include <esp_timer.h>

#include "config_v1.h"

namespace bno086 {

namespace {

using ead::sh2::Header;

constexpr uint8_t kCs[2] = {EAD_PIN_FOOT_CS, EAD_PIN_SHANK_CS};
constexpr uint8_t kInt[2] = {EAD_PIN_FOOT_INT, EAD_PIN_SHANK_INT};
// The reset advertisement is the largest packet a sensor sends (a few hundred bytes).
constexpr size_t kMaxPacket = 1024;
// The sensor must be serviced within about 10 ms of asserting INT (wiring rule 7).
constexpr uint32_t kWakeWaitUs = 10000;
constexpr uint32_t kBootWaitUs = 500000;

uint8_t s_packet[kMaxPacket];
uint8_t s_controlSequence[2] = {0, 0};
const SPISettings kSettings(EAD_SPI_HZ, MSBFIRST, SPI_MODE3);

void transfer(Sensor s, uint8_t* bytes, size_t n) {
  SPI.beginTransaction(kSettings);
  digitalWrite(kCs[s], LOW);
  SPI.transfer(bytes, n);  // in place: what goes out is overwritten by what comes back
  digitalWrite(kCs[s], HIGH);
  SPI.endTransaction();
}

bool waitInt(Sensor s, uint32_t timeoutUs) {
  const int64_t start = esp_timer_get_time();
  while (digitalRead(kInt[s]) != LOW) {
    if (esp_timer_get_time() - start > int64_t(timeoutUs)) return false;
  }
  return true;
}

// Reads one packet into s_packet: its length, 0 if the sensor had nothing, -1
// if what came back was not a packet. As the TEST-043 bench driver does: the
// header first, then, once INT is asserted again, the whole packet with 0x00
// on MOSI.
int readPacket(Sensor s) {
  uint8_t head[ead::sh2::kHeaderSize] = {};
  transfer(s, head, sizeof head);
  Header h{};
  if (!ead::sh2::parseHeader(head, kMaxPacket, &h)) return -1;
  if (h.length == 0) return 0;
  if (!waitInt(s, kWakeWaitUs)) return -1;
  std::memset(s_packet, 0, h.length);
  transfer(s, s_packet, h.length);
  Header full{};
  if (!ead::sh2::parseHeader(s_packet, kMaxPacket, &full) || full.length == 0) return -1;
  return full.length < h.length ? full.length : h.length;
}

// Host writes use the WAKE handshake: WAKE low, the sensor asserts INT when it
// can receive, then the packet goes out under CS (CEVA BNO08X datasheet, SPI).
// WAKE is shared, so the other sensor wakes too; it gives up after its timeout.
// Whatever the sensor sends during the write is discarded, as the bench driver
// does; writes only happen while it has nothing else to say.
bool writePacket(Sensor s, const uint8_t* packet, size_t n) {
  digitalWrite(EAD_PIN_SENSOR_WAKE, LOW);
  const bool ready = waitInt(s, kWakeWaitUs);
  if (ready) {
    std::memcpy(s_packet, packet, n);
    transfer(s, s_packet, n);
  }
  digitalWrite(EAD_PIN_SENSOR_WAKE, HIGH);
  return ready;
}

struct Drain {
  bool anyValid;
  bool released;  // INT went high and stayed high for the quiet time
  ead::sh2::ControlReplies control;
};

// Reads until INT stays released for `quietUs`, until `wantFeatures` (or a
// product ID, when `wantProductId`) has been seen, or until `maxUs`. INT that
// stays asserted while reads through this CS bring nothing back means CS and
// INT do not reach the same sensor.
Drain drain(Sensor s, uint32_t quietUs, uint32_t maxUs, bool wantProductId = false,
            uint8_t wantFeatures = 0) {
  Drain d{};
  int fruitless = 0;
  const int64_t start = esp_timer_get_time();
  while (esp_timer_get_time() - start < int64_t(maxUs)) {
    if (wantProductId && d.control.productIds > 0) break;
    if (wantFeatures != 0 && (d.control.featureSensors & wantFeatures) == wantFeatures) break;
    if (!waitInt(s, quietUs)) {
      d.released = true;
      break;
    }
    const int n = readPacket(s);
    if (n <= 0) {
      if (++fruitless >= 20) break;
      delayMicroseconds(500);
      continue;
    }
    fruitless = 0;
    d.anyValid = true;
    if (s_packet[2] == ead::sh2::kChannelControl) {
      ead::sh2::parseControl(s_packet + ead::sh2::kHeaderSize,
                             size_t(n) - ead::sh2::kHeaderSize, &d.control);
    }
  }
  return d;
}

bool sendControl(Sensor s, size_t (*encode)(uint8_t, uint8_t*, size_t)) {
  uint8_t packet[32];
  const size_t n = encode(s_controlSequence[s]++, packet, sizeof packet);
  return n > 0 && writePacket(s, packet, n);
}

bool sendSetFeature(Sensor s, uint8_t reportId) {
  uint8_t packet[32];
  const size_t n = ead::sh2::encodeSetFeature(s_controlSequence[s]++, reportId,
                                              EAD_REPORT_INTERVAL_US, packet, sizeof packet);
  return n > 0 && writePacket(s, packet, n);
}

}  // namespace

void beginBus() {
  for (uint8_t cs : kCs) {
    pinMode(cs, OUTPUT);
    digitalWrite(cs, HIGH);
  }
  // WAKE (= PS0) must be high whenever RST is released, or a sensor latches a
  // UART mode instead of SPI (wiring rule 2).
  pinMode(EAD_PIN_SENSOR_WAKE, OUTPUT);
  digitalWrite(EAD_PIN_SENSOR_WAKE, HIGH);
  pinMode(EAD_PIN_SENSOR_RST, OUTPUT);
  digitalWrite(EAD_PIN_SENSOR_RST, HIGH);
  for (uint8_t pin : kInt) pinMode(pin, INPUT_PULLUP);
  SPI.begin(EAD_PIN_SPI_SCK, EAD_PIN_SPI_MISO, EAD_PIN_SPI_MOSI, -1);
}

bool intAsserted(Sensor s) { return digitalRead(kInt[s]) == LOW; }

void resetAndCheck(ead::SensorCheck* foot, ead::SensorCheck* shank) {
  ead::SensorCheck* checks[2] = {foot, shank};
  for (ead::SensorCheck* c : checks) {
    *c = ead::SensorCheck{};
    c->bootMs = 0xFFFF;
    c->wakeUs = 0xFFFF;
  }

  digitalWrite(EAD_PIN_SENSOR_WAKE, HIGH);
  digitalWrite(EAD_PIN_SENSOR_RST, LOW);
  delay(20);
  for (uint8_t s = 0; s < 2; s++) {
    if (digitalRead(kInt[s]) == HIGH) checks[s]->flags |= ead::kCheckIntHighInReset;
  }

  // RST is shared: both sensors boot together. Each announces itself on INT.
  digitalWrite(EAD_PIN_SENSOR_RST, HIGH);
  const int64_t released = esp_timer_get_time();
  bool booted[2] = {false, false};
  while (!(booted[0] && booted[1]) && esp_timer_get_time() - released < kBootWaitUs) {
    for (uint8_t s = 0; s < 2; s++) {
      if (!booted[s] && digitalRead(kInt[s]) == LOW) {
        booted[s] = true;
        checks[s]->bootMs = uint16_t((esp_timer_get_time() - released) / 1000);
        checks[s]->flags |= ead::kCheckBooted;
      }
    }
  }

  // The reset messages (advertisement, reset complete) come back through each
  // CS; reading them all must release that sensor's INT.
  for (uint8_t s = 0; s < 2; s++) {
    if (!booted[s]) continue;
    const Drain d = drain(Sensor(s), 50000, 1500000);
    if (d.anyValid) checks[s]->flags |= ead::kCheckReadValid;
    if (d.released) checks[s]->flags |= ead::kCheckIntReleased;
  }

  // WAKE: each idle sensor must assert INT. One whose INT is already asserted
  // cannot show it, so it is not credited.
  bool answered[2];
  for (uint8_t s = 0; s < 2; s++) answered[s] = digitalRead(kInt[s]) == LOW;
  digitalWrite(EAD_PIN_SENSOR_WAKE, LOW);
  const int64_t woken = esp_timer_get_time();
  while (!(answered[0] && answered[1]) && esp_timer_get_time() - woken < kWakeWaitUs) {
    for (uint8_t s = 0; s < 2; s++) {
      if (!answered[s] && digitalRead(kInt[s]) == LOW) {
        answered[s] = true;
        checks[s]->wakeUs = uint16_t(esp_timer_get_time() - woken);
        checks[s]->flags |= ead::kCheckWake;
      }
    }
  }
  digitalWrite(EAD_PIN_SENSOR_WAKE, HIGH);
  for (uint8_t s = 0; s < 2; s++) drain(Sensor(s), 20000, 200000);

  // A product ID request proves MOSI; then the two reports.
  for (uint8_t s = 0; s < 2; s++) {
    ead::SensorCheck& c = *checks[s];
    if ((c.flags & ead::kCheckReadValid) == 0) continue;
    if (sendControl(Sensor(s), ead::sh2::encodeProductIdRequest)) {
      const Drain d = drain(Sensor(s), 300000, 300000, true);
      if (d.control.productIds > 0) {
        const ead::sh2::ProductId& id = d.control.firstProductId;
        c.flags |= ead::kCheckProductId;
        c.resetCause = id.resetCause;
        c.versionMajor = id.versionMajor;
        c.versionMinor = id.versionMinor;
        c.versionPatch = id.versionPatch;
        c.partNumber = id.partNumber;
        c.buildNumber = id.buildNumber;
      }
    }
    // One at a time: a write discards whatever the sensor sends meanwhile, which
    // could be the previous command's confirmation.
    bool reports = true;
    for (uint8_t report : {ead::sh2::kReportAccelerometer, ead::sh2::kReportGyroscope}) {
      const uint8_t bit = uint8_t(1u << report);
      reports = reports && sendSetFeature(Sensor(s), report) &&
                (drain(Sensor(s), 300000, 300000, false, bit).control.featureSensors & bit) != 0;
    }
    if (reports) c.flags |= ead::kCheckReports;
  }
}

ReadResult read(Sensor s, int64_t intUs, ead::sh2::Sample* out, size_t cap) {
  ReadResult r{};
  const int n = readPacket(s);
  if (n < 0) {
    r.bad = true;
    return r;
  }
  if (n <= int(ead::sh2::kHeaderSize)) return r;
  const uint8_t* cargo = s_packet + ead::sh2::kHeaderSize;
  const size_t len = size_t(n) - ead::sh2::kHeaderSize;
  switch (s_packet[2]) {
    case ead::sh2::kChannelInput: {
      bool unknown = false;
      r.samples = ead::sh2::parseInput(cargo, len, intUs, out, cap, &unknown);
      r.bad = unknown;
      break;
    }
    case ead::sh2::kChannelExecutable:
      r.resetSeen = cargo[0] == ead::sh2::kExecutableResetComplete;
      break;
    default:
      break;  // advertisement, control responses: nothing the stream needs
  }
  return r;
}

bool enableReports(Sensor s) {
  return sendSetFeature(s, ead::sh2::kReportAccelerometer) &&
         sendSetFeature(s, ead::sh2::kReportGyroscope);
}

}  // namespace bno086
