// EAD-V1 bench test — ONE BNO086 over SPI on a Seeed XIAO ESP32-S3.
//
// Not product firmware. Purpose, in order:
//   1. check the wiring of every GPIO going to the sensor,
//   2. read the sensor's identity so a relabelled / wrong part shows up,
//   3. stream orientation so tools/bno_view.py can show how it is tilted.
//
// Wiring: docs/hardware.md, "BNO086 bench test (one sensor)".
// Output: one line per event on USB CDC.
//   CHK,<name>,<PASS|FAIL|INFO>,<detail>
//   ID,<entry>,part=<n>,ver=<a.b.c>,build=<n>,reset_cause=<n>
//   D,<t_ms>,<qi>,<qj>,<qk>,<qr>,<acc_deg>,<ax>,<ay>,<az>,<gx>,<gy>,<gz>
//   # <free text>

#include <Arduino.h>
#include <SPI.h>
#include <hal/usb_serial_jtag_ll.h>
#include <SparkFun_BNO08x_Arduino_Library.h>

namespace {

constexpr uint8_t kPinCs = D3;    // GPIO4
constexpr uint8_t kPinInt = D2;   // GPIO3
constexpr uint8_t kPinRst = D1;   // GPIO2
constexpr uint8_t kPinWake = D0;  // GPIO1, the sensor's PS0/WAKE net

constexpr uint32_t kSpiHz = 3000000;  // BNO08x maximum (CEVA datasheet §6.2)
constexpr uint16_t kReportMs = 10;    // 100 Hz, the rate the product uses

BNO08x imu;

// The checks run once, at boot. A serial monitor is usually opened after that,
// so every line is kept and re-sent when the host asks (any byte, see loop()).
String transcript;
bool stopped = false;

// Arduino's HWCDC Serial prints nothing on this board (docs/problems.md
// PROB-006, and measured again here: a bare Serial.println sketch produced no
// bytes). Like firmware/src/link_usb.cpp, this writes the USB Serial/JTAG IN
// endpoint directly: one packet at a time, and never appending to a packet the
// host has not collected yet, which duplicated bytes.
constexpr uint32_t kPacket = 64;

bool inEndpointEmpty() { return USB_SERIAL_JTAG.int_raw.serial_in_empty_int_raw; }

void usbWrite(const char* data, size_t len) {
  static bool inFlight = false;
  size_t pos = 0;
  while (pos < len) {
    if (inFlight) {
      uint32_t waited = 0;
      while (!inEndpointEmpty() && waited < 200) {
        delay(1);
        waited++;
      }
      if (!inEndpointEmpty()) return;  // no host reading; drop the rest
      inFlight = false;
    }
    const size_t want = len - pos < kPacket ? len - pos : kPacket;
    const uint32_t accepted =
        usb_serial_jtag_ll_write_txfifo(reinterpret_cast<const uint8_t*>(data + pos), want);
    if (accepted == 0) return;
    pos += accepted;
    usb_serial_jtag_ll_clr_intsts_mask(USB_SERIAL_JTAG_INTR_SERIAL_IN_EMPTY);
    usb_serial_jtag_ll_txfifo_flush();
    inFlight = true;
  }
}

void usbPrint(const String& text) { usbWrite(text.c_str(), text.length()); }

bool usbAsked() {
  if (!usb_serial_jtag_ll_rxfifo_data_available()) return false;
  uint8_t drain[kPacket];
  while (usb_serial_jtag_ll_rxfifo_data_available()) usb_serial_jtag_ll_read_rxfifo(drain, kPacket);
  return true;
}

void emit(const String& line) {
  transcript += line;
  transcript += '\n';
  usbPrint(line + "\n");
}

void chk(const char* name, bool pass, const String& detail) {
  emit(String("CHK,") + name + "," + (pass ? "PASS" : "FAIL") + "," + detail);
}

void info(const char* name, const String& detail) {
  emit(String("CHK,") + name + ",INFO," + detail);
}

// Reads a line back with the ESP32's own pull-up and then its pull-down.
// 1/0 means nothing outside is holding the line: the wire goes only to the
// sensor's high-impedance pin. 1/1 means an external pull-up (the breakout
// has them on PS0/WAKE and RST). 0/0 means the line is held low, which on a
// bench is almost always a short to GND or a swapped wire.
String readBack(uint8_t pin) {
  pinMode(pin, INPUT_PULLUP);
  delayMicroseconds(200);
  const int high = digitalRead(pin);
  pinMode(pin, INPUT_PULLDOWN);
  delayMicroseconds(200);
  const int low = digitalRead(pin);
  pinMode(pin, INPUT);
  return String(high) + "/" + String(low);
}

// Waits for INT to reach `level`, returns the time it took, or -1 on timeout.
int32_t waitInt(int level, uint32_t timeoutMs) {
  const uint32_t start = millis();
  while (millis() - start < timeoutMs) {
    if (digitalRead(kPinInt) == level) return int32_t(millis() - start);
    delay(1);
  }
  return -1;
}

// Every line except power, before any SPI traffic. The sensor is held in
// reset so it drives nothing and cannot answer.
bool checkWiring() {
  pinMode(kPinWake, OUTPUT);
  digitalWrite(kPinWake, HIGH);  // PS0 must be HIGH at reset or the part
                                 // comes up in a UART mode, not SPI.
  pinMode(kPinRst, OUTPUT);
  digitalWrite(kPinRst, LOW);
  delay(20);

  pinMode(kPinInt, INPUT_PULLUP);
  const bool intIdle = digitalRead(kPinInt) == HIGH;
  chk("int_idle_in_reset", intIdle,
      intIdle ? "INT high while RST low" : "INT stuck low: short to GND, or INT on the wrong pin");

  // 1/1 = an external pull-up. On this breakout SCK, MISO and MOSI are the
  // same chip pins as SCL, SDA and SA0, which carry the board's I2C pull-ups,
  // so 1/1 is normal there. 0/0 on any line means it is held low: a short to
  // GND, or a wire on the wrong pin.
  info("readback_miso", readBack(MISO) + " (1/0 floating, 1/1 pulled up, 0/0 held low = fault)");
  info("readback_cs", readBack(kPinCs) + " (same reading)");
  info("readback_sck", readBack(SCK) + " (same reading)");
  info("readback_mosi", readBack(MOSI) + " (same reading)");

  pinMode(kPinCs, OUTPUT);
  digitalWrite(kPinCs, HIGH);

  // RST and INT together: releasing reset must make the sensor boot and
  // announce itself by pulling INT low.
  pinMode(kPinInt, INPUT_PULLUP);
  digitalWrite(kPinRst, HIGH);
  const int32_t t = waitInt(LOW, 500);
  if (t >= 0) {
    chk("rst_int", true, "INT asserted " + String(t) + " ms after reset released");
    return true;
  }
  chk("rst_int", false,
      "INT never went low after reset: check 3V3, GND, RST, INT, and that PS0/PS1 are unsoldered");
  return false;
}

void printIds() {
  for (uint8_t i = 0; i < imu.prodIds.numEntries; i++) {
    const sh2_ProductId_t& e = imu.prodIds.entry[i];
    char line[128];
    snprintf(line, sizeof(line), "ID,%u,part=%lu,ver=%u.%u.%u,build=%lu,reset_cause=%u", i,
             (unsigned long)e.swPartNumber, e.swVersionMajor, e.swVersionMinor,
             e.swVersionPatch, (unsigned long)e.swBuildNumber, e.resetCause);
    emit(line);
  }
  info("id_entries", String(imu.prodIds.numEntries) +
                         " entries (a BNO08x answers the SH-2 product ID request; a part that"
                         " cannot is not running CEVA/Hillcrest firmware)");
}

// The host's wake line: pulling PS0/WAKE low asks a sleeping sensor for a
// transfer, and the sensor answers by asserting INT. Run with no reports
// enabled, so INT is only low because of the request.
void checkWake() {
  // Wait for the sensor to go quiet first.
  const uint32_t start = millis();
  while (millis() - start < 1000) {
    if (digitalRead(kPinInt) == HIGH) break;
    imu.getSensorEvent();  // drain whatever is pending
  }
  if (digitalRead(kPinInt) == LOW) {
    info("wake", "skipped: INT still busy");
    return;
  }
  digitalWrite(kPinWake, LOW);
  const int32_t t = waitInt(LOW, 100);
  digitalWrite(kPinWake, HIGH);
  if (t >= 0) {
    chk("wake", true, "INT answered the wake request in " + String(t) + " ms");
  } else {
    info("wake", "no INT within 100 ms — WAKE may be unwired; not conclusive on its own");
  }
  delay(10);
}

// Which SH-2 features the part admits to having. Genuine CEVA/Hillcrest
// firmware answers all of these; a relabelled or different part will not.
void checkFeatures() {
  struct Feature { const char* name; sh2_SensorId_t id; };
  static const Feature kFeatures[] = {
      {"arvr_stabilized_rv", SH2_ARVR_STABILIZED_RV},
      {"gyro_integrated_rv", SH2_GYRO_INTEGRATED_RV},
      {"stability_classifier", SH2_STABILITY_CLASSIFIER},
      {"magnetometer", SH2_MAGNETIC_FIELD_CALIBRATED},
      {"tap_detector", SH2_TAP_DETECTOR},
  };
  String have;
  for (const Feature& f : kFeatures) {
    const bool ok = imu.enableReport(f.id, 100000);
    imu.enableReport(f.id, 0);  // interval 0 turns the report back off
    have += String(f.name) + "=" + (ok ? "1" : "0") + " ";
    delay(20);
  }
  info("features", have);
}

// A talking sensor is not yet a working one: held still, gravity must read
// ~9.81 m/s^2 and the gyro ~0.
void checkStill() {
  float ax = 0, ay = 0, az = 0, gx = 0, gy = 0, gz = 0;
  uint32_t n = 0, nGyro = 0;
  const uint32_t start = millis();
  while (millis() - start < 2000) {
    if (!imu.getSensorEvent()) continue;
    if (imu.getSensorEventID() == SH2_ACCELEROMETER) {
      ax += imu.getAccelX(); ay += imu.getAccelY(); az += imu.getAccelZ();
      n++;
    } else if (imu.getSensorEventID() == SH2_GYROSCOPE_CALIBRATED) {
      gx += imu.getGyroX(); gy += imu.getGyroY(); gz += imu.getGyroZ();
      nGyro++;
    }
  }
  if (n == 0) {
    chk("still", false, "no accelerometer reports arrived");
    return;
  }
  ax /= n; ay /= n; az /= n;
  if (nGyro > 0) { gx /= nGyro; gy /= nGyro; gz /= nGyro; }
  const float g = sqrtf(ax * ax + ay * ay + az * az);
  const float w = sqrtf(gx * gx + gy * gy + gz * gz);
  chk("still", g > 9.0f && g < 10.6f,
      "|a|=" + String(g, 3) + " m/s^2 over " + String(n) + " accel reports, |w|=" +
          String(w, 4) + " rad/s over " + String(nGyro) +
          " gyro reports (hold it still; |a| should be ~9.81)");
}

}  // namespace

void setup() {
  // No Serial.begin: the USB endpoint is written directly (see usbWrite).
  // Checks printed before a monitor attaches are kept in `transcript` and
  // re-sent whenever the host sends a byte.
  delay(1500);  // let the host enumerate the USB device
  emit("# EAD bench: one BNO086 over SPI");
  char pins[128];
  snprintf(pins, sizeof(pins), "# pins cs=%u int=%u rst=%u wake=%u sck=%u miso=%u mosi=%u", kPinCs,
           kPinInt, kPinRst, kPinWake, SCK, MISO, MOSI);
  emit(pins);

  if (!checkWiring()) {
    // The SH-2 driver blocks waiting for INT, so do not call it with a dead
    // INT line: the sketch would hang with nothing on the serial port.
    emit("# stopped: the sensor never answered on INT, so SPI was not attempted");
    stopped = true;
    return;
  }

  SPI.begin(SCK, MISO, MOSI, -1);
  if (!imu.beginSPI(kPinCs, kPinInt, kPinRst, kSpiHz)) {
    chk("spi_init", false,
        "no answer over SPI: check SCK/MISO/MOSI/CS, and that the PS0 and PS1 jumpers are open");
    emit("# stopped: fix the wiring and reset the board");
    stopped = true;
    return;
  }
  chk("spi_init", true, "SH-2 opened at " + String(kSpiHz / 1000000) + " MHz, SPI mode 3");
  printIds();
  checkWake();
  checkFeatures();

  const bool rv = imu.enableRotationVector(kReportMs);
  const bool acc = imu.enableAccelerometer(kReportMs);
  const bool gyr = imu.enableGyro(kReportMs);
  chk("reports", rv && acc && gyr,
      String("rotation_vector=") + rv + " accel=" + acc + " gyro=" + gyr);
  checkStill();
  emit("# streaming: D,t_ms,qi,qj,qk,qr,acc_deg,ax,ay,az,gx,gy,gz");
}

void loop() {
  if (usbAsked()) usbPrint(transcript);
  if (stopped) {
    delay(100);
    return;
  }

  static float qi = 0, qj = 0, qk = 0, qr = 1, accDeg = 0;
  static float ax = 0, ay = 0, az = 0, gx = 0, gy = 0, gz = 0;

  if (imu.wasReset()) {
    info("reset", "sensor reset itself; re-enabling reports");
    imu.enableRotationVector(kReportMs);
    imu.enableAccelerometer(kReportMs);
    imu.enableGyro(kReportMs);
  }
  if (!imu.getSensorEvent()) return;

  switch (imu.getSensorEventID()) {
    case SH2_ROTATION_VECTOR: {
      qi = imu.getQuatI(); qj = imu.getQuatJ(); qk = imu.getQuatK();
      qr = imu.getQuatReal();
      accDeg = imu.getQuatRadianAccuracy() * 57.29578f;
      // One line per orientation report; accel and gyro carry their latest value.
      char line[160];
      const int n = snprintf(line, sizeof(line),
                             "D,%lu,%.4f,%.4f,%.4f,%.4f,%.1f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f\n",
                             (unsigned long)millis(), qi, qj, qk, qr, accDeg, ax, ay, az, gx, gy,
                             gz);
      usbWrite(line, n);
      break;
    }
    case SH2_ACCELEROMETER:
      ax = imu.getAccelX(); ay = imu.getAccelY(); az = imu.getAccelZ();
      break;
    case SH2_GYROSCOPE_CALIBRATED:
      gx = imu.getGyroX(); gy = imu.getGyroY(); gz = imu.getGyroZ();
      break;
    default:
      break;
  }
}
