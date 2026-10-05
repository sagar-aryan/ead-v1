// EAS ERM driver bring-up + dashboard firmware — XIAO ESP32-S3.
// Handoff 07 sections 4, 5 and 7: one channel at a time, then all six.
//
// Wiring: PWM1..PWM6 on the driver board -> XIAO D0..D5. XIAO GND -> board GND (or EGND).
// The board's PWR+ comes from a current-limited supply at 3.7-4.0 V, NOT from the XIAO.
// Leave the board's ESP+ pad unconnected while the XIAO is powered over USB: the XIAO's
// own charger would otherwise push current back into SWITCHED_SYSTEM+.
//
// Serial, 115200, one command per line:
//   "<ch> <percent>"  set one channel, ch 1..6, percent 0..100   (what the dashboard sends)
//   "1".."6"          ramp that channel up, hold, stop (bring-up test)
//   "a"               all six at the test duty for 3 s
//   "0"               all channels off
//   "k"               keepalive, silent: just proves the link is alive
//   "?"               print the commanded duty and the actual LEDC output of every channel
//
// Failsafe: if no command arrives for LINK_TIMEOUT_MS while any channel is driving, every channel
// is switched off. LEDC is a hardware peripheral and holds its last duty forever with no CPU
// involvement, so without this a dropped USB link leaves a motor running until the battery dies.
const int PWM[6] = {1, 2, 3, 4, 5, 6};   // GPIO1..GPIO6 = the pads marked D0..D5
const int FREQ = 20000;                  // 20 kHz: above hearing, easy for the 100R gate drive
const int BITS = 8;                      // duty 0..255
const int TEST_DUTY = 160;               // ~63 %: enough to start an ERM, not full tilt

const unsigned long LINK_TIMEOUT_MS = 3000;

int duty[6] = {0, 0, 0, 0, 0, 0};        // percent, as last commanded
unsigned long lastCommandMs = 0;
bool failsafeTripped = false;

void setDuty(int i, int percent) {
  percent = constrain(percent, 0, 100);
  duty[i] = percent;
  ledcWrite(PWM[i], (percent * 255) / 100);
}

void allOff() {
  for (int i = 0; i < 6; i++) setDuty(i, 0);
}

void report() {
  // Two numbers per channel: what was last commanded, and what the LEDC hardware is actually
  // outputting. They can disagree - the peripheral keeps driving the pin with no CPU involvement -
  // and only the second one says whether a motor is being told to run.
  Serial.print("duty");
  for (int i = 0; i < 6; i++) { Serial.print(' '); Serial.print(duty[i]); }
  Serial.print("  raw");
  for (int i = 0; i < 6; i++) { Serial.print(' '); Serial.print(ledcRead(PWM[i])); }
  Serial.println();
}

void runChannel(int i) {
  Serial.printf("channel %d (PWM%d, GPIO%d): ramp up\n", i + 1, i + 1, PWM[i]);
  for (int d = 0; d <= TEST_DUTY; d += 5) { ledcWrite(PWM[i], d); delay(20); }
  delay(1000);
  setDuty(i, 0);
  Serial.printf("channel %d: off\n", i + 1);
}

void handle(String line) {
  line.trim();
  if (line.length() == 0) return;
  lastCommandMs = millis();
  failsafeTripped = false;
  if (line == "k") return;               // keepalive: resets the timer, says nothing

  int space = line.indexOf(' ');
  if (space > 0) {                                   // "<ch> <percent>"
    int ch = line.substring(0, space).toInt();
    int percent = line.substring(space + 1).toInt();
    // No report here: one line per set command floods the port during a slider drag.
    if (ch >= 1 && ch <= 6) setDuty(ch - 1, percent);
    else Serial.println("err: channel must be 1..6");
    return;
  }
  char c = line[0];
  if (c >= '1' && c <= '6') runChannel(c - '1');
  else if (c == 'a') { for (int i = 0; i < 6; i++) setDuty(i, (TEST_DUTY * 100) / 255); delay(3000); allOff(); }
  else if (c == '0') { allOff(); report(); }
  else if (c == '?') report();
  else Serial.println("err: unknown command");
}

void setup() {
  Serial.begin(115200);
  // A full USB CDC buffer must never block loop(). With a timeout of 0, output is dropped instead
  // of stalling, so commands keep being read and the failsafe keeps running even when nothing on
  // the PC is draining the port.
  Serial.setTxTimeoutMs(0);
  for (int i = 0; i < 6; i++) {
    // ledcAttach() is Arduino-ESP32 core 3.x. On core 2.x use:
    //   ledcSetup(i, FREQ, BITS); ledcAttachPin(PWM[i], i); and ledcWrite(i, duty).
    ledcAttach(PWM[i], FREQ, BITS);
    ledcWrite(PWM[i], 0);
  }
  lastCommandMs = millis();
  Serial.println("\nEAS ERM driver ready. '<ch> <percent>', '1'-'6', 'a', '0', 'k', '?'");
  Serial.printf("failsafe: all channels off after %lu ms without a command\n", LINK_TIMEOUT_MS);
}

bool anyOutput() {
  // Checks the peripheral, not just the remembered value: the two can disagree, and the pin is
  // what actually turns a motor.
  for (int i = 0; i < 6; i++)
    if (duty[i] != 0 || ledcRead(PWM[i]) != 0) return true;
  return false;
}

void loop() {
  static String line;
  while (Serial.available()) {
    char c = Serial.read();
    if (c == '\n' || c == '\r') { handle(line); line = ""; }
    else if (line.length() < 32) line += c;
  }

  if (!failsafeTripped && millis() - lastCommandMs > LINK_TIMEOUT_MS) {
    failsafeTripped = true;              // trips once per silence, not every loop
    if (anyOutput()) {
      allOff();
      Serial.println("failsafe: link silent, all channels off");
    }
  }
}
