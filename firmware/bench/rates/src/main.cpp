// See platformio.ini. Output: one line per sensor per requested interval.
#include <Arduino.h>
#include <esp_timer.h>
#include <hal/usb_serial_jtag_ll.h>

#include "bno086.h"
#include "config_v1.h"

namespace {

// The USB Serial/JTAG IN endpoint written directly, as the BNO086 bench does
// (PROB-006: Arduino's HWCDC Serial prints nothing on this board).
void usbWrite(const char* data, size_t len) {
  size_t pos = 0;
  while (pos < len) {
    uint32_t waited = 0;
    while (!USB_SERIAL_JTAG.int_raw.serial_in_empty_int_raw && waited < 200) {
      delay(1);
      waited++;
    }
    const size_t want = len - pos < 64 ? len - pos : 64;
    const uint32_t accepted =
        usb_serial_jtag_ll_write_txfifo(reinterpret_cast<const uint8_t*>(data + pos), want);
    if (accepted == 0) return;
    pos += accepted;
    usb_serial_jtag_ll_clr_intsts_mask(USB_SERIAL_JTAG_INTR_SERIAL_IN_EMPTY);
    usb_serial_jtag_ll_txfifo_flush();
  }
}

String transcript;

void emit(const String& line) {
  transcript += line + "\n";
  usbWrite(line.c_str(), line.length());
  usbWrite("\n", 1);
}

constexpr uint8_t kIds[] = {ead::sh2::kReportAccelerometer, ead::sh2::kReportGyroscope,
                            ead::sh2::kReportGameRotationVector};
const char* const kNames[] = {"accel", "gyro", "game_rv"};

struct Tally {
  uint32_t count[3];
  uint32_t gaps[3];  // reports missing by sequence number
  int last[3];
  uint32_t bad;
};

void service(bno086::Sensor s, Tally* t) {
  while (bno086::intAsserted(s)) {
    ead::sh2::Sample samples[16];
    const bno086::ReadResult r = bno086::read(s, esp_timer_get_time(), samples, 16);
    if (r.bad) t->bad++;
    for (size_t i = 0; i < r.samples; ++i) {
      for (int k = 0; k < 3; ++k) {
        if (samples[i].reportId != kIds[k]) continue;
        if (t->last[k] >= 0) t->gaps[k] += uint8_t(samples[i].sequence - t->last[k] - 1);
        t->last[k] = samples[i].sequence;
        t->count[k]++;
      }
    }
  }
}

void survey(uint32_t intervalUs) {
  for (bno086::Sensor s : {bno086::kFoot, bno086::kShank}) bno086::enableReports(s, intervalUs);
  Tally tally[2]{};
  // Settle: drain at the new rate, and collect the hub's confirmations.
  const uint32_t settle = millis();
  while (millis() - settle < 500) {
    service(bno086::kFoot, &tally[0]);
    service(bno086::kShank, &tally[1]);
  }
  for (Tally& t : tally) t = Tally{{0, 0, 0}, {0, 0, 0}, {-1, -1, -1}, 0};
  const uint32_t start = millis();
  while (millis() - start < 3000) {
    service(bno086::kFoot, &tally[0]);
    service(bno086::kShank, &tally[1]);
  }
  for (int s = 0; s < 2; ++s) {
    String line = "requested " + String(intervalUs) + " us " + (s == 0 ? "foot " : "shank") + ":";
    for (int k = 0; k < 3; ++k) {
      line += String(" ") + kNames[k] + " " + String(tally[s].count[k] / 3.0f, 1) + " Hz (hub " +
              String(bno086::reportIntervalUs(bno086::Sensor(s), kIds[k])) + " us, gaps " +
              String(tally[s].gaps[k]) + ")";
    }
    line += " bad " + String(tally[s].bad);
    emit(line);
  }
}

}  // namespace

void setup() {
  for (uint8_t pin : {EAD_MOTOR_M1_GPIO, EAD_MOTOR_M2_GPIO, EAD_MOTOR_M3_GPIO, EAD_MOTOR_M4_GPIO,
                      EAD_MOTOR_M5_GPIO, EAD_MOTOR_M6_GPIO}) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, LOW);
  }
  delay(1500);  // let the host enumerate
  bno086::beginBus();
  ead::SensorCheck foot{}, shank{};
  bno086::resetAndCheck(&foot, &shank);
  emit("# rates survey: foot check flags 0x" + String(foot.flags, HEX) + ", shank 0x" +
       String(shank.flags, HEX) + ", SPI " + String(EAD_SPI_HZ) + " Hz");
  for (uint32_t us : {10000u, 5000u, 4000u, 2500u, 2000u}) survey(us);
  emit("# done");
}

void loop() {
  if (usb_serial_jtag_ll_rxfifo_data_available()) {
    uint8_t drain[64];
    while (usb_serial_jtag_ll_rxfifo_data_available()) usb_serial_jtag_ll_read_rxfifo(drain, 64);
    usbWrite(transcript.c_str(), transcript.length());
  }
  delay(20);
}
