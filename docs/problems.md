# Problems

---

## PROB-001 — Accel read ~2g at rest on both sensors

**Status:** Resolved

### Symptoms
Both IMUs streamed accel magnitude ~2.0g while still (foot ~2.04g, shank ~1.99g).

### Environment
XIAO ESP32-S3 + 2x breakout boards sold as MPU6050, PlatformIO/Arduino, 2026-09-17.

### Expected Behavior
|a| ≈ 1.0g at rest (±4g range, 8192 LSB/g).

### Actual Behavior
|a| ≈ 2.0g on both sensors; gyro looked plausible (small error masked by 2x).

### Investigation
Sent `c` (new DIAG command): `WHO=0x70` on both, all config registers 0x00.

### Hypotheses
1. Wrong scale divisor — confirmed contributor.
2. Sensor variant — confirmed: 0x70 = MPU6500 silicon, not MPU6050 (0x68).

### Attempts
#### Attempt 1
Read back registers via DIAG — showed init had never applied (early-return on WHO check).

### Root Cause
Confirmed: `mpuInit()` rejected WHO_AM_I 0x70, returned before writing any
config, leaving power-on defaults (±2g/16384). Driver divided by 8192 → 2x.

### Resolution
Accept 0x68 (MPU6050) and 0x70 (MPU6500, register-compatible, same
sensitivities). After fix: CFG readback SMPL=0x09/CFG=0x03/GYRO=0x08/
ACCEL=0x08, foot |a|=1.03, shank |a|=1.00.

### Verification
60-sample means over 6 s post-flash, device worn. Commit d052a9c.

### Lessons
Never gate init on a single WHO value when clones exist; always read back
config registers; DIAG-on-demand (`c`) beats reboot-timing captures.

---

## PROB-002 — Shank anatomical mapping differs from placement image

**Status:** Resolved (2026-09-17), confirmed on the leg

### Symptoms
With image-identity mapping, shank gravity did not sit on +Z.

### Environment
Same hardware; shank on anterior shin, user-verified chip +Z = posterior.

### Expected Behavior
Still/vertical shin → shank ANAT a ≈ (0, 0, +1).

### Actual Behavior
Raw still captures put gravity on chip −Y (pose 1) and chip +X (pose 2);
leg pose changed between captures so second axis is not yet isolated.

### Investigation
Pose 1 chip ≈ (0.08,−0.96,−0.24); pose 2 chip ≈ (0.98,0.06,0.16) (÷2 scale
corrected). Combined with user statement chip+Z = posterior (−anatX).

### Hypotheses
Candidate map: anatX=−chipZ, anatY=−chipX, anatZ=−chipY (det −1 reflection,
expected: anatomical frame is left-handed). Pose-2 lateral gravity
(−0.98 anatY) is consistent with a non-vertical shin (e.g. crossed leg),
not necessarily a wrong map.

Note added 2026-09-17: the "left-handed" expectation above is wrong. X forward,
Y left, Z up is a right-handed frame (X × Y = Z), so a det −1 map cannot describe
any rigid mounting.

### Attempts
#### Attempt 1
Candidate map committed in `config_v1.h` (EAD_SHANK_MAP_*); viewer unaffected
(firmware outputs anatomical).

Failed. The map is a reflection: gyro rates about the anatomical axes would have
the wrong handedness relative to accel-derived tilt, which breaks Mahony fusion
and every gyro-based gait feature. Nothing had consumed it yet, so no recorded
data is affected (raw data is chip-frame; DEC-007).

#### Attempt 2 (2026-09-17)
The user re-confirmed chip +Z points toward the bone (posterior). Pose 1 gives
chip +Y down the leg. Those two facts fix anatX = −chipZ and anatZ = −chipY, and
right-handedness forces anatY = anatZ × anatX = +chipX. The map is now a matrix
in `config_v1.h` with compile-time checks that it is a signed permutation with
determinant +1 (DEC-009).

### Root Cause
Confirmed: the anatomical frame was assumed to be left-handed, so Y was given
the sign of a reflection. The one axis no measurement constrained (Y) was
therefore derived with the wrong sign.

### Resolution
First attempt: `X = −chipZ, Y = +chipX, Z = −chipY`, derived from a bench capture
and the user's statement that chip +Z faces the bone. Wrong on the leg.

Measured with the mounting check (TEST-027): standing still put gravity on
anatomical Y (+0.99 g) and a seated knee extension turned about anatomical Z
(+43 °/s), so anatomical Y and Z were interchanged. The correction
(`Z_true = Y_measured`, `Y_true = −Z_measured`) gives

  `X = −chipZ, Y = +chipY, Z = +chipX`

which is still a proper rotation and is what is flashed. The assumption that
failed was that chip +Y points down the leg; chip +X does.

### Verification
- Build-time: the committed map passes both static assertions. A copy of the
  header with the previous map fails with "shank mount map must be a proper
  rotation" (g++ 13, `-std=gnu++17`, 2026-09-17).
- Host check: chip (1, 2, 3) maps to anatomical (−3, 1, −2), and chip X = −32768
  maps without overflow.
- On-body: the mounting check (TEST-027) found the error and gave the numbers the
  correction was derived from. The device reports the new map
  (`eadprobe config`: `shank_mount [0,0,-1, 0,1,0, 1,0,0]`), and the check was
  re-run on the leg with all three steps passing.

### Lessons
The map was derived twice from statements about how the board faces and was wrong
both times; it took three seconds of measurement on the leg to settle. Physical
orientation is cheap to measure and expensive to reason about — measure it.
- On-body: pending. Standing still must read anatomical a ≈ (0, 0, +1) g on the
  shank. The M3 mounting check then verifies gyro signs with a toe raise and a
  seated knee extension.

### Lessons
One unknown-pose capture cannot fix a 3-axis map; always capture in a
declared known pose. Derive an unmeasured axis from the cross product of the
measured ones, and let the compiler reject any map that is not a proper rotation.

---

## PROB-003 — Accelerometer low-pass filter never configured on MPU6500 silicon

**Status:** Resolved (2026-09-17)

### Symptoms
None observed directly. Found by review of the init sequence against the MPU6500
register map, after PROB-001 showed both boards are MPU6500.

### Environment
Both IMUs report WHO_AM_I 0x70; Arduino-ESP32 2.0.17.

### Expected Behavior
Doc 00 / CONFIG_V1.json: 42 Hz DLPF with a 100 Hz output rate for accel and gyro.

### Actual Behavior
`mpuInit()` wrote only CONFIG (0x1A). On the MPU6050 that register filters both
sensors; on the MPU6500 it filters only the gyro. The accel filter is
ACCEL_CONFIG2 (0x1D), whose reset value 0x00 leaves the accel at its widest
bandwidth (≈460 Hz per the MPU-6500 register map) while the output is decimated
to 100 Hz, so vibration and impact content above 50 Hz can alias into the samples.

### Investigation
The Linux driver fix "iio: imu: inv_mpu6050: add accel lpf setting for chip >=
MPU6500" writes the same DLPF value to ACCEL_CONFIG_2 for exactly this reason.

### Root Cause
Confirmed: 0x1D was never written, because the init sequence was written for the
MPU6050 register semantics.

### Resolution
When WHO_AM_I is 0x70, write ACCEL_CONFIG2 = 0x03 (≈41 Hz). Every configuration
register is now read back after init; a mismatch fails init and is printed.

### Verification
Flashed 2026-09-17. The boot log reports "Foot OK  Shank OK" (readback passed),
and `c` diagnostics show `ACCEL2=0x03` on both sensors.

### Lessons
Board labels are not silicon; when a clone substitutes a newer part, re-check
every register whose meaning changed, not just WHO_AM_I.

---

## PROB-004 — GPIO43 (motor M5) may toggle during boot before firmware runs

**Status:** Open, unverified (no drivers fitted, so no current effect)

### Symptoms
None yet; identified by review.

### Environment
XIAO ESP32-S3; M5 PWM is on GPIO43, M6 on GPIO44 (doc 03).

### Expected Behavior
Doc 03 §4 and doc 13 §1.5: all six motor outputs stay LOW at boot.

### Actual Behavior
Not measured.

### Hypotheses
GPIO43/44 are UART0 TX/RX at reset. The ESP32-S3 ROM prints its boot log on
UART0 TX before any firmware runs, and UART TX idles HIGH. With a driver fitted,
the M5 gate (100 kΩ pull-down) could see that waveform and pulse the motor at
every reset. Firmware cannot act before the ROM stage.

### Attempts
None yet.

### Root Cause
Root cause: Unknown (hypothesis only).

### Resolution
Before fitting drivers: probe GPIO43/44 through a reset. If activity is
confirmed, options are disabling ROM UART printing (eFuse/strapping), or a
hardware change. Either needs a DEC because doc 03 fixes the pin map.

### Verification
Not yet verified.

### Lessons
Boot-state guarantees must cover the ROM stage, not only `setup()`.

---

## PROB-005 — Opening the USB port reset the device

**Status:** Resolved (2026-09-17)

### Symptoms
Every `eadprobe` connection produced a new `boot_id`, so sequence numbers
restarted and no backfill could span a reconnect.

### Environment
XIAO ESP32-S3 native USB Serial/JTAG (`303a:1001`), Linux 7.0, pyserial 3.5.

### Expected Behavior
Opening and closing the port is passive; the device keeps running (doc 13 §6
requires telemetry to survive a host disconnect).

### Actual Behavior
`boot_id` changed on every open. Device uptime and `frame_index` restarted.

### Investigation
The probe set `dtr = False` and `rts = False` after `Serial()` construction but
before `open()`; pyserial applies them in sequence after opening. Linux asserts
both lines on open, so the sequence passed through DTR=0 with RTS=1. The
ESP32-S3 USB Serial/JTAG interprets that combination as a reset request, the
same mechanism `esptool` uses to enter the bootloader.

### Root Cause
Confirmed: a transient DTR=0/RTS=1 state while clearing both control lines.

### Resolution
The probe leaves both lines asserted (`dtr = rts = True` before `open()`), which
is the state Linux already sets. Recorded as a host rule in `docs/protocol.md` §2.2.

### Verification
`eadprobe.py reopen --cycles 20`: one distinct `boot_id` across 20 open/close
cycles, `last_seq` rising monotonically (TEST-016).

### Lessons
On this chip the serial control lines are a reset interface. Any host tool must
open the port without changing them.

---

## PROB-006 — USB protocol frames intermittently corrupted

**Status:** Resolved (2026-09-17)

### Symptoms
Roughly 0.5–1% of USB frames failed CRC. A 6-minute run rejected 240 frames, and
whole backfill chunks were lost, so requested ranges arrived incomplete
(8–12 missing messages per full-window request).

### Environment
Arduino-ESP32 2.0.17, ESP32-S3 native USB Serial/JTAG, streaming ~5.6 kB/s of
raw batches plus backfill bursts at ~55 kB/s.

### Expected Behavior
Every framed message arrives intact; COBS and CRC exist to detect line noise,
not to mask routine loss.

### Actual Behavior
Corrupted frames fell into two groups: byte runs missing from the middle of a
frame, and frames carrying 80 extra bytes.

### Investigation
1. Captured raw port bytes during two identical backfill requests and byte-diffed
   each corrupted frame against an intact copy of the same chunk. The damage was
   neither a truncation nor a prefix: bytes were missing from the middle, and the
   oversized frames contained an inserted run.
2. Printed the inserted bytes. They were Arduino core log text:
   `[ 86010][E][Wire.cpp:499] requestFrom(): i2cWriteReadNonStop returned Error -1`.
3. Slowing the host reader changed the rate slightly but never eliminated it,
   ruling out host-side overrun as the main cause.

### Hypotheses
1. Host `cdc_acm` overrun — rejected: a slow reader did not make it worse in
   proportion.
2. Arduino `HWCDC` write path losing data — supported by upstream reports
   (arduino-esp32 issues #9378, #11959, both open) of missing chunks on S3.
3. Core log output sharing the USB endpoint — confirmed by the captured text.

### Attempts
#### Attempt 1
Silenced ESP-IDF logging with `esp_log_level_set("*", ESP_LOG_NONE)` at boot.
Insufficient: that controls the IDF logger, not the Arduino `log_e()` macros,
which are compiled in by `CORE_DEBUG_LEVEL` and write through `ets_printf` to
the same USB FIFO.

#### Attempt 2
Replaced Arduino `Serial` with direct writes to the USB Serial/JTAG FIFO, one
64-byte packet at a time, waiting for the endpoint-empty status before the next
packet. This removed the `HWCDC` ring buffer, its interrupt, and its
disconnect-time buffer flush from the path. Corruption rate dropped but did not
reach zero, because core logging still wrote into the same FIFO.

#### Attempt 3
Added `-DCORE_DEBUG_LEVEL=0`, compiling out every Arduino `log_*` call.

### Root Cause
Confirmed: two writers shared the USB IN endpoint. Arduino core error logging
(triggered here by the I²C failures of PROB-007) interleaved text into the byte
stream mid-frame. The `HWCDC` write path contributed additional loss under
sustained load.

### Resolution
- `-DCORE_DEBUG_LEVEL=0` in `platformio.ini`: no core log output exists to
  interleave. `esp_log_level_set("*", ESP_LOG_NONE)` covers the IDF logger.
- `src/link_usb.cpp` owns the endpoint: no Arduino `Serial`, one packet in
  flight, next packet only after the peripheral reports the endpoint empty.
- Firmware must never print to USB. Diagnostics travel as protocol messages.

### Verification
90-second run: 0 rejected frames, 0 missing messages (TEST-017). Confirmed again
over 30 minutes (TEST-018).

### Lessons
A binary protocol needs exclusive ownership of its transport. On this core that
means disabling both loggers and bypassing `HWCDC`, whose data loss is a known
open upstream issue.

---

## PROB-007 — I²C reads fail when started on the data-ready edge

**Status:** Resolved (2026-09-17)

### Symptoms
About 0.6% of frames carried a read-failure flag (237 of 38,460 frames over
6 minutes); `i2c_errors` rose steadily. Affected frames carry zeroed sensor
values.

### Environment
Both MPU6500 IMUs on one 400 kHz bus; acquisition clocked by the foot data-ready
interrupt, reading foot then shank immediately on each interrupt.

### Expected Behavior
Every scheduled read succeeds; a failure means a real bus fault.

### Actual Behavior
`Wire` returned `i2cWriteReadNonStop ... Error -1` at irregular intervals
(median 112 frames apart, minimum 2, maximum 2334).

### Investigation
1. Failures showed no periodicity against `frame_index` (87 distinct values of
   `frame_index % 100`), ruling out a fixed beat against the 100 Hz cycle or the
   1 Hz power check.
2. Swapping the read order moved the failures: with foot first, only the foot
   failed (237 fails, 0 shank); with shank first, only the shank failed
   (20 fails, 0 foot). The failure follows the *position* in the frame, not the
   device.
3. Adding a delay between the interrupt and the first read eliminated them.

### Hypotheses
1. Bus contention between the two IMUs — rejected: they are read sequentially,
   and the second read never failed.
2. A faulty sensor or connection — rejected: either sensor fails when read first.
3. Reading while the sensor updates its output registers at the data-ready edge
   disturbs the transaction — supported by the order-swap result and the fix.

### Attempts
#### Attempt 1
Swapped read order (diagnostic, not a fix): moved the failures to the other
sensor, which identified the timing relationship.

#### Attempt 2
Guard delay of at least 1 ms after the data-ready notification, before the first
read. `vTaskDelay(2)` guarantees one full 1 ms tick and yields core 1 to the
processing task, unlike a busy-wait.

### Root Cause
Confirmed by experiment: starting an I²C transaction immediately on the
data-ready edge fails intermittently. The precise mechanism inside the sensor is
unverified; the timing dependence and the order-swap result establish the cause.

### Resolution
A guard delay of 1–2 ms after each data-ready notification. The frame timestamp
still comes from the interrupt, so timing accuracy is unaffected, and the read
completes well inside the 10 ms sample period.

### Verification
90 seconds: 0 read failures, `i2c_errors` 0 (TEST-017), previously ~30 in the
same period. Confirmed over 30 minutes (TEST-018).

### Lessons
When a failure follows the position in a sequence rather than the device, it is a
timing problem. Swapping the order is a cheap way to tell the two apart.

---

## PROB-000 — Template (do not treat as a real problem)

**Status:** Open / Investigating / Resolved / Workaround

### Symptoms
What was observed?

### Environment
Relevant hardware, software, versions, configuration, etc.

### Expected Behavior
What should have happened?

### Actual Behavior
What actually happened?

### Investigation
What was checked?

### Hypotheses
Possible causes considered.

### Attempts

#### Attempt 1
What was changed and what happened.

#### Attempt 2
What was changed and what happened.

### Root Cause
Root cause: Unknown

### Resolution
What fixed the issue.

### Verification
How the fix was confirmed.

### Lessons
What should be remembered to avoid the problem again.

## PROB-009 — Foot accelerometer reads 2.5 % high

**Status:** Open, measured, not yet corrected

### Symptoms
With the device still, the foot sensor reports |a| = 1.0249 g while the shank
sensor reports 0.998 g. Both use the same configured range (±4 g, 8192 LSB/g).

### Environment
Both MPU6500 (WHO 0x70), firmware schema 2, bench, 2026-09-18 (TEST-028).

### Investigation
Three calibration windows of different lengths agree to within 0.0002 g, so it
is a constant scale error rather than motion or noise. The MPU6500 datasheet
allows ±3 % initial sensitivity tolerance, so 2.5 % is within specification for
the part; the two boards simply differ.

### Root cause
Per-part accelerometer sensitivity tolerance. Not established beyond that: the
possibility of a supply-voltage or temperature contribution has not been ruled
out.

### Resolution
None yet, deliberately. Nothing downstream depends on absolute acceleration
magnitude today: orientation uses the direction of gravity, and the gait work
uses thresholds that will be set from recorded walks. If a measurement ever
depends on the magnitude, the calibration record is the right place to carry a
per-sensor scale factor, since it already carries `accel_magnitude_g`.

### Lessons
The calibration's `accel_magnitude_g` field earns its place: it is what made a
2.5 % scale error visible, and it would equally catch a wrong range setting.

## PROB-010 — Mounting tilt appeared as a permanent ankle angle

**Status:** Resolved (2026-09-18)

### Symptoms
With the device worn and the foot flat on the floor, the sagittal ankle angle
read about −34° instead of 0°, and the frontal angle a constant +20°. The angles
still followed the foot correctly: a toe raise moved the sagittal angle about
+28° in the right direction.

### Environment
Firmware with Mahony orientation, foot board strapped over the instep at 40.7°
from upright, shank board 1.6° (TEST-029, 2026-09-18).

### Investigation
The offsets were constant and matched the mounting tilts the calibration had
measured, which pointed at the alignment rather than at the estimator: a drifting
or broken estimator does not hold a fixed offset while tracking movement
correctly. The static check had already shown each sensor's estimate agreeing
with its own measured gravity to 0.09°, so each estimator was right about its
own board — the boards were simply not the segments.

### Root cause
The calibration's alignment rotation was used only as the estimator's initial
attitude, never applied to the measurements. The estimator therefore described
the orientation of the board, and the relative orientation of two boards
mounted at different angles is not the ankle angle. Doc 04 §4 says the alignment
transform corrects the data; §5 says the estimator's input is calibrated
accelerometer and gyroscope.

### Resolution
`firmware/src/orientation.cpp` now rotates both the acceleration and the angular
rate by the sensor's alignment quaternion before the Mahony update, and starts
both estimators from identity.

### Verification
Standing still after the fix, over 502 frames: sagittal mean −0.11° (spread
0.19°), frontal −0.02° (0.11°), transverse −0.19° (0.43°). Before the fix the
same pose read −34° and +20°.

### Lessons
The bug was invisible to unit tests, which fed the estimator ideal data, and
invisible to the static gravity check, which compared each sensor against
itself. It took a physical pose with a known answer — a flat foot is 0° — to
show it. Every estimator needs at least one test whose expected value comes from
the world rather than from the code.
