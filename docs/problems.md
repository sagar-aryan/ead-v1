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

## PROB-011 — Accelerometer range is marginal for heel strike

**Status:** Open, measured

### Symptoms
A 6 m walk clipped the accelerometer on 2 frames and the gyroscope on 1, out of
4,010 (TEST-030). Peak |a| at heel strike reached 5.3 g against a ±4 g range.

### Evidence
`foot_accel_saturated` and `foot_gyro_saturated` flags in the recording, and the
acceleration magnitudes around each contact: 2.3–5.3 g.

### Consequence
Clipping loses part of the impact peak. It does not affect contact detection —
a clipped impact is still far above the confirm threshold — but it under-reads
the acceleration that distance integration uses, at the one moment per stride
where acceleration is largest.

### Options
1. ±8 g accelerometer and ±1000 °/s gyroscope: halves the resolution everywhere
   else, where the signal is small.
2. Keep ±4 g and accept that a heel strike clips.
3. Measure a faster walk and a stair descent before deciding; both are harsher
   than the level walk that produced this.

### Decision
Deferred until there are recordings of faster walking (option 3). At three
clipped frames in forty seconds, this is not what limits distance accuracy today
— the orientation error during swing is (PROB-012).

## PROB-012 — Orientation error during swing limits distance accuracy

**Status:** Workaround in place, root cause understood

### Symptoms
Integrating acceleration over a swing gives a foot velocity of 6–10 m/s at the
instant the foot is back on the ground and demonstrably at rest (TEST-030).

### Investigation
Measured in the recording: during the walk the world-frame acceleration averaged
−2.65 m/s² vertically, where it should average zero, and its horizontal RMS was
8.2 m/s². Both point at the orientation estimate rather than at the sensor: a
tilt error of θ leaks 9.81·sin θ of gravity into the horizontal axes, which
integrates into exactly this kind of runaway velocity.

### Root cause
A 6-DoF estimator has only gravity to level itself against, and during swing the
accelerometer measures gravity plus the foot's own acceleration. Correcting
toward that tilts the estimate toward the direction of travel, which is worst
exactly when the foot is moving fastest.

### Workaround
Two measures, both measured against the 6 m course:

1. The gravity correction is gated off while |a| is more than 0.25 g from 1 g
   (`kAccelGateG`), so the estimator integrates the gyroscope through swing and
   re-levels during stance. Chosen from a sweep: no gate −23 %, 0.15 g +8.7 %,
   0.25 g +6.5 %, 0.40 g −9 %.
2. The velocity that has accumulated by the end of a swing is treated as drift
   and removed as a linear ramp over that swing, which has a closed form: the
   displacement correction is the end velocity times half the segment duration
   (`GaitEngine::removeSegmentDrift`). Without it the same recording measured
   0.94 m instead of 6.39 m.

### Remaining error
+6.5 % on a 6 m course. The proper fix is an error-state Kalman filter that
corrects tilt as well as velocity at each zero-velocity update, which is what a
foot-mounted inertial navigator normally does; doc 05 §7 asks only for the
velocity substate in V1.

### Lessons
The failure was invisible in every synthetic test, because a synthetic signal is
generated from a known orientation and so has no tilt error at all. It took one
walk down a corridor of known length.

## PROB-013 — Reference capture ran for three minutes against old firmware and collected nothing

**Status:** Resolved in the dashboard; the device needs reflashing

### Symptoms
First real reference capture, over Wi-Fi. The valid-cycle counter never moved.
Switching to another tab and back reset the References page to "Start capture
walk", while the capture kept recording in the background; starting again gave
"another session is already recording".

### Investigation
The open session in the store held 17,850 frames with no gaps, all flagged
orientation-valid, and 26 gait events including 3 initial contacts — but 0
cycles. The frames showed about 20 s of walking and then a still device.

`sessions.firmware` was `0.1.0+a7c6d0d`: firmware built before `adc059d`, the
schema-4 commit. The device had never been reflashed after M5. The Device page
said so ("The device speaks a different protocol schema than this build").

### Root cause
Two, both mine.
1. **The device was not reflashed after the protocol changed.** Schema-3
   firmware sends 72-byte cycle records; the schema-4 dashboard expects 132 and
   rejects every one, so no cycle could ever reach the store. The old firmware
   also does not know the REFERENCE_CAPTURE session kind.
2. **The session gate ignored a known schema mismatch.** `schema_mismatch` was
   detected and displayed, but `session_blockers` did not check it, so the
   capture was allowed to start.

Separately, the References page kept "a capture is running" in component state,
which React discards when the view unmounts on a tab switch.

### Resolution
- `session_blockers` refuses a capture, check or evaluation on a schema mismatch.
- The References page reads the open session from the backend when it mounts and
  resumes showing it: patient, counter, and the Stop button.
- Reflash the device (`pio run -t upload` over USB).

### Lessons
A protocol schema bump is not finished until the device on the desk runs it.
Every schema change should end with a flash and a HELLO showing the new schema.
And a condition the UI already warns about should be a gate, not only a warning.

## PROB-014 — The device refused every reference check and evaluation

**Status:** Resolved in firmware `d6c9d84`+1; the device must be reflashed

### Symptoms
First real reference check, against a profile captured minutes earlier. The
"Scored cycles" counter stayed at 0 while the user walked.

### Investigation
The check session in the store held 3,870 frames and 14 cycles, 13 valid — so
steps were detected and delivered — but every cycle had error score 0 and
confidence 0: not scored. The capture just before it had worked.

The difference between the two commands is their length. A capture's
SESSION_START is 4 bytes; a check or an evaluation carries the 64-byte reference
profile after them, 68 in all. `ead::decodeSessionStart` returned false for any
length other than 4, so the device answered every check and every evaluation
with ERROR BadPayload and never entered the session. It kept detecting steps,
and without a session nothing was scored.

### Root cause
The decoder was written for schema 2, when SESSION_START was always 4 bytes, and
not widened when schema 4 appended the profile. The golden vector for exactly
this message (`reference_profile.hex`) existed, but its firmware test skipped
the 4-byte head and decoded only the profile — so the one part that was wrong
was the one part not tested.

A second fault made it silent: the device's ERROR reached the dashboard's
`last_error`, but the References page never showed it, and the dashboard started
the recording (and locked the reference) without waiting to hear whether the
device accepted.

### Resolution
- `decodeSessionStart` accepts 4 bytes, or 4 + 64; `link.cpp` still refuses a
  profile on a calibration or a capture.
- New test `test_session_start_with_reference_decodes` decodes the whole golden
  message. Verified that it **fails on the old decoder** and passes on the new.
- The dashboard waits 800 ms after starting a capture, check or evaluation. A
  refusal fails the Start button with the device's reason, stops the recording,
  and leaves the reference unlocked.

### Verification
61/61 firmware native tests, 56 Rust tests (all), zero clippy warnings. On hardware:
pending the reflash.

### Lessons
A golden vector only protects the bytes a test actually decodes. And a command
the device can refuse must not be reported as started until the device has had
the chance to say so.

### Side effect already in the store
Versions 1 and 2 were marked locked when these checks started, although the
device never scored against either. Locking only prevents a profile's numbers
from changing, so it does not stop either being used.

## PROB-015 — Peak inversion depends on sensor heading drift, not only on the ankle

**Status:** Partly resolved. Heading drift: fixed. A second shift after the
device was re-worn and recalibrated: root cause unknown.

### Symptoms
The first working reference check (against version 2) gave a median error score
of 0.45, dominated by peak inversion (median deviation 0.88), and classified
nearly every stride as EVERSION_DEVIATION — for the same person walking the same
way minutes after capturing the reference. Cycle distance showed a deviation of
0.00.

### Investigation
Every frame stores the device's own foot and shank quaternions, so the ankle
angles were recomputed from the stored data for all eight sessions of the day.

1. **Heading drift.** The heading difference between the foot and shank
   filters (the transverse part of `inverse(q_shank)·q_foot`) was about 4° after
   calibration and −44° to −60° ten minutes later, with no recalibration in
   between. A foot cannot turn 50° on its shank: this is two 6-DoF filters
   drifting in heading independently, with nothing to correct them. Session
   averages of peak inversion tracked it: +6° drift → 17°, −50° → 24–25°,
   +15–20° → 7–10°.
2. **Removing heading before the decomposition** (turning the foot about world
   vertical onto the shank's heading) brought the five sessions that shared one
   calibration from 18–28° to 16–20° per-session median. Sagittal was unchanged.
3. **The two sessions after the device was re-worn and recalibrated** still read
   5.5° and −1.3° after that correction, against 16–20° before. Their
   frontal~sagittal slope rose to 0.23–0.39.
4. **Tested and rejected:** a hinge-axis correction — the dominant axis of
   relative angular velocity, taken as the ankle's flexion axis and rotated onto
   Y. It did not close the gap (3–11° against 17–22°), and the estimated axis
   moved by 20° between sessions with no change in mounting, so it is not a
   stable measurement of anything.
5. Cycle distance: zero-velocity quality was 0 in 16 of 20 check strides. The
   engine dropped distance only at exactly 0, so strides at 0.02–0.04 — with
   reported strides of 15–48 m — were scored at full deviation.

### Hypotheses for the unexplained shift (3)
- The sensors sat differently after the device was taken off to be flashed.
- A different standing posture during the second calibration (shank tilt 8.4°
  against 4.4°), setting a different neutral.
- A real difference in how the walk was done.
None has been tested. Root cause: **unknown**.

### Resolution so far
- `ead::relativeOrientation` (and the dashboard's `orientation::relative`) turn
  the foot onto the shank's heading before composing. Tests on both sides with
  40° of drift and 12° of dorsiflexion: frontal stays 0 (the old code read
  7.68°), and a real 10° inversion survives.
- The engine drops cycle distance below ZUPT quality 0.15, the threshold doc 05
  §8 already sets for distance; the host's symmetry proxy follows (DEC-013
  updated).
- The References check shows "not measured" for a feature no cycle measured,
  instead of a median of the zeros the device sends for a dropped feature.

### Consequence until (3) is explained
A reference is only trustworthy within the wearing and calibration it was
captured in. Taking the device off, or recalibrating, may shift peak inversion by
15° or more — enough to classify normal walking as eversion.

## PROB-016 — About half the strides split in two at the foot slap

**Status:** Resolved (firmware `kMinStanceS = 0.25`); flash pending at time of writing

### Symptoms
Reference version 3 (50 cycles) had a median cycle time of 0.86 s with a spread
of 0.49 s, and a stance ratio of 0.30 ± 0.26. The walk itself had ~1.35 s
strides.

### Investigation
Cycle times in the capture came in pairs summing to one stride — 0.80 + 0.54,
0.83 + 0.57, 0.86 + 0.50 — with the first of each pair at a stance ratio of
0.07–0.10: about 60 ms of "stance" after a real heel strike. The same pattern
existed in the v1 capture on older firmware (1.06 + 0.68, 0.93 + 0.61), less
often. Event detection does not read the ankle angles, so PROB-015's change was
not the cause.

Replayed through the device's own code (stored frames converted to an .eadlog)
the split reproduced exactly.

### Root cause
After a heel strike the foot rotates flat — the foot slap — as fast as it
lifts off. Nothing stopped that being taken as toe-off (foot rate > 90 °/s for
40 ms). The engine then sat in "swing" through stance, and once `kMinSwingS` had
passed, the real push-off impact qualified as the next contact.

### Resolution
A floor on stance: toe-off cannot be declared until `minStanceS` after the last
contact. Swept by replay on the v3 capture:

| minStanceS | cycles | median cycle | strides < 1.0 s | stance median |
|---|---|---|---|---|
| 0 | 50 | 0.86 s | 28 | 0.30 |
| 0.15 | 40 | 1.35 s | 9 | 0.48 |
| 0.25 | 38 | 1.37 s | 6 | 0.50 |
| 0.35 | 38 | 1.36 s | 6 | 0.48 |

0.25 s: where the benefit levels off, with margin for fast walking (stance
~0.35–0.4 s). Regression on the 6 m ground-truth course: unchanged, 6 cycles,
6.39 m. v1 capture: 8 split strides → 1. v3 check: 18 of 24 → 0.

New test `test_the_foot_slapping_flat_is_not_a_toe_off`: two heel strikes with
a 150 ms slap and a push-off spike between them. Verified to fail with the floor
at 0 (a third contact) and pass at 0.25.

### Lessons
The 6 m course was one walk by one person at one speed; a harder heel strike
exposed a failure it never showed. Every recording with ground truth should go
into `recordings/` so the next threshold change is checked against all of them.
Reference versions 1–3 were captured with this fault and should not be used.

## PROB-017 — The board went completely silent on USB: it was sitting in the ROM bootloader

**Status:** Resolved

### Symptoms

While bringing up the BNO086 bench firmware, nothing at all arrived on USB: not the
bench firmware's own lines, not a bare `Serial.println` sketch, and not the product
firmware answering `eadprobe hello`. `/dev/ttyACM0` opened normally and held open.

### Environment

XIAO ESP32-S3, USB Serial/JTAG (303a:1001), Linux host, 2026-09-23.

### Investigation

1. Flashed a three-line `Serial.println` sketch: silent. Consistent with PROB-006
   (Arduino's HWCDC is unreliable on this chip), so that told us little.
2. Rewrote the bench output on the low-level `usb_serial_jtag_ll` path that
   `firmware/src/link_usb.cpp` already uses: still silent.
3. Reflashed the product firmware and ran `eadprobe --usb hello`: also silent. A
   host-side or chip-state cause, then, not the sketch.
4. No ModemManager, no other process holding the port, port node freshly created.
5. `esptool --before no_reset --after no_reset flash_id` **succeeded**, which only
   works when the ROM loader is running.

### Root Cause

The chip was sitting in ROM download mode, so no application was running at all. It
was put there earlier in the session by hand-toggling DTR/RTS on the open port to
try to reset the board: on the ESP32-S3's USB Serial/JTAG peripheral those two lines
drive a reset/strapping state machine, and the sequence used selected download mode.
Repeated `--after hard_reset` did not get it out.

### Resolution

`esptool --before no_reset --after watchdog_reset` left download mode and started the
application. The USB device then re-enumerated as **/dev/ttyACM1**, and reads of the
old `/dev/ttyACM0` name would have stayed silent anyway.

### Verification

A tick sketch on the low-level writer printed continuously; the bench firmware then
ran end to end (TEST-039).

### Lessons

- Do not hand-toggle DTR/RTS on this chip to reset it. Use
  `esptool --after hard_reset`, or `watchdog_reset` to recover, or unplug it.
- Total USB silence means "check whether the application is running at all" before
  suspecting the code: `esptool --before no_reset flash_id` succeeding is the tell.
- Re-enumerate the port after every reset; the device node number changes.
- Confirmed again, independently of PROB-006: an Arduino `Serial.println` sketch on
  this board produces no bytes at all. The direct USB Serial/JTAG writer works.

## PROB-018 — After a device reboot the dashboard kept the old calibration, so sessions ran with none

**Status:** Resolved; verified on hardware 2026-10-02 (TEST-042)

### Symptoms

On 2026-09-18 two evaluations for patient 67 recorded frames but no events and no
cycles. Not noticed at the time; found on 2026-10-02 while writing the handoff, by
querying the dashboard database (read-only).

| Session | Kind | Firmware | boot_id | Frames | Frames with `orientation_valid` | Cycles | Stored calibration |
|---|---|---|---|---:|---:|---:|---|
| `20260918-121312-32bb` | reference_check | `22d4aa5` | 2910462277 | 2440 | 2440 | 25 | record R |
| `20260918-122259-6762` | evaluation | `22d4aa5` | 2910462277 | 0 | — | 0 | R |
| `20260918-122444-de5c` | evaluation | `6b36044` | 121483439 | 2640 | **0** | 0 | **R** |
| `20260918-122519-06f1` | evaluation | `6b36044` | 121483439 | 210 | **0** | 0 | **R** |
| `20260918-122947-05a4` | reference_check | `6b36044` | 121483439 | 690 | 690 | 0 | a new record |

"R" is the same calibration JSON, byte for byte (foot gyro bias 3.0320017…).

### Environment

Dashboard at the 2026-09-18 commits; firmware `22d4aa5`, then `6b36044`; USB link.

### Expected Behavior

Calibration lives in device RAM only and is lost on any reset or reflash. A session
started on a new boot without recalibrating must be refused by the session gate
("the device has not been calibrated since it started").

### Actual Behavior

The two evaluations on boot 121483439 started, and each stored record R, which the
device had produced on the previous boot (2910462277). The device, with no
calibration, never produced orientation (0 frames flagged `orientation_valid`),
so it detected no gait and scored no cycles.

### Investigation

1. **Observed (database):** the firmware string changed from `22d4aa5` to
   `6b36044` and the boot_id changed between `122259-6762` and `122444-de5c`. A
   firmware change means the device was reflashed. The user does not recall the
   sequence (asked 2026-10-02).
2. **Confirmed (code):** `device.rs` `Tracker::on_hello` cleared the config on a
   changed boot_id but never `State::calibration`, which is set only from a
   SESSION_STOP calibration record. `Device::snapshot()` exposes it whenever
   connected, and `app.rs` `session_blockers` passes when it is usable.
3. **Confirmed (code):** `Tracker` is created anew on every link session
   (`session()`), so its own `boot_id` is `None` after every reconnect, not only
   after a reboot.
4. **Confirmed (test):** `a_device_reboot_discards_the_calibration` failed on the
   unmodified code: after HELLO with a new boot_id the snapshot still held the
   record.
5. The firmware's other paths were checked and agree with the host: cancelling a
   still window restores the previous state; a rejected recalibration replaces the
   record, and the host stores and blocks on it (`calibration_service.cpp`).

### Attempts

#### Attempt 1 — clear the calibration in the existing boot_id branch (rejected)

This was the fix first proposed in the handoff. Because the tracker's `boot_id`
starts empty on every reconnect (item 3), that branch also runs after a USB replug
or a Wi-Fi drop of the same boot. Tried with both tests in place:
`a_device_reboot_discards_the_calibration` passed and
`a_reconnect_to_the_same_boot_keeps_the_calibration` **failed**. It would have
blocked sessions after any cable wiggle until the patient recalibrated. Reverted.

#### Attempt 2 — compare with the last HELLO the host kept (applied)

`State::hello` survives reconnects. `on_hello` now clears `State::calibration`
when the new HELLO's boot_id differs from the previous HELLO's. Both tests pass.

### Root Cause

**Confirmed in code:** the host kept a calibration record across device boots. That
this caused the 2026-09-18 zero-cycle evaluations is a hypothesis: it fits
everything in the database (the reflash, the carried-over record, no orientation on
the new boot, valid orientation as soon as the device was recalibrated), but it was
not reproduced on hardware.

`20260918-122259-6762` is a different case: same boot as its calibration, 0 frames
recorded at all. Root cause: Unknown. It is not explained by this bug.

### Resolution

`dashboard/src-tauri/src/device.rs` `Tracker::on_hello`: clear `State::calibration`
when the boot_id differs from the last HELLO's.

### Verification

- `cargo test`: `device::tests::a_device_reboot_discards_the_calibration` and
  `a_reconnect_to_the_same_boot_keeps_the_calibration` (TEST-042).
- Hardware, later the same day once the product firmware supported DEC-016: the ignored
  test `a_device_reset_ends_the_session_and_its_calibration` calibrates, replugs (the
  calibration stays), resets the board with esptool (the calibration is gone, so the
  gate blocks). PASS (TEST-042).

### Lessons

- Any host copy of device RAM state must be tied to the boot it came from.
- A tracker that is rebuilt per connection cannot tell a reboot from a reconnect;
  compare with state that outlives the connection.
- Limitation: between the link connecting and the device's HELLO arriving (a few
  milliseconds), the snapshot still shows the previous record. The device's HELLO
  is the first thing that tells the host the boot changed.
- Related, not addressed: `State::session_kind` (the session this host started)
  also survives a device reboot, although the device's session ended with it.

## PROB-019 — A device reboot during a recording is not handled by the dashboard

**Status:** Resolved 2026-10-02 (option 1, the user's default); verified on hardware

### Symptoms

None observed. Found on 2026-10-02 while fixing PROB-018, by following what else the
host keeps across a device reboot.

### Expected Behavior

A recording session holds frames from one device boot, or says clearly where a boot
changed. Nothing is silently dropped.

### Actual Behavior (from the code)

If the device reboots (reset, brown-out, reflash) while the dashboard is recording:

1. **Confirmed (code):** the store session stays open. Nothing in `store/` or
   `app.rs` looks at the boot_id after the session starts.
2. **Confirmed (code):** the device restarts `frame_index` at 0 each boot, and
   `raw_frames` is keyed by `(session_id, frame_index)` and written with
   `INSERT OR IGNORE` (`store/mod.rs`, so a backfilled frame is not stored twice).
   New-boot frames whose index was already stored are therefore dropped without an
   error; later ones are stored in the same session, mixed with the old boot's.
   Events are keyed and written the same way. Cycles are keyed by `start_frame` but
   written with `INSERT OR REPLACE`, so a new-boot cycle that collides **overwrites**
   the old boot's cycle.
3. **Confirmed (code):** `State::session_kind` and `session_valid_cycles` survive
   the reboot, so the References view keeps showing a capture in progress and its
   cycle count, although the device's session ended with the reboot. Stopping it
   then reaches a device with no session.
4. Hypothesis: a scored session continues recording raw frames with no cycles,
   much like PROB-018's evaluations.

### Root Cause

The host treats a session as continuous across device boots; the device does not.

### Resolution

The contract does not say what the host should do (doc 09's reboot rules are about
on-device storage, M7). Options put to the user: (1) end the host session at the reboot
and say so; (2) keep one session and store the boot with each frame (a schema change
every view and export would have to handle). The user did not object to option 1.

Built: on a HELLO whose boot_id differs from the last one this host saw, `device.rs`
clears the host's session kind and calls `Sink::device_restarted`; the app's sink ends
the store's recording with segments closed as `device_restarted`
(`Store::end_session_at_restart`). The state bar shows "<session> ended: device
restarted" until another session starts. A replug of the same boot, or the first HELLO
of a run, ends nothing.

### Verification

- Rust: `only_a_device_reboot_ends_the_session` (device), `a_device_reset_ends_the_recording`
  (store).
- Hardware (TEST-042): a recording survived a replug, then a board reset ended it, with
  the notice.

## PROB-020 — GPIO42 (motor 3) is held low, unlike the other five motor pins

**Status:** Resolved 2026-10-02 — wiring measured correct; motor 3 enabled

### Symptoms

TEST-044: GPIO42 (MTMS back pad, motor 3 in DEC-016) reads 0 with the internal
pull-up on. GPIO1, 2, 4, 5 and 6, wired to identical channels, read 1. On the bare
chip (TEST-041) GPIO42 read 1/0, so the pull-up works; something outside holds it low.

### Expected Behavior

The same reading as the other motor pins: 100 Ω to the gate, 100 kΩ gate to GND.

### Hypotheses (none tested)

1. A wrong pulldown value on channel 3 (for example 100 Ω fitted as R9).
2. The MTMS wire on GND or EGND instead of PWM3.
3. The MTMS wire on the M3- pad (MOSFET drain) or a rail, with rail A unpowered at
   that boot (its 220 µF would hold the pin low for far longer than 300 µs).
4. A copper or solder bridge to the GND pour (the driver PCB is bare copper).

### Why it matters

Every firmware for this build drives the motor pins LOW first. Under hypotheses 1,
2 and 4 that is harmless. Under hypothesis 3, with rail A powered, it would sink
motor current through GPIO42. Driving it HIGH (any motor-3 test) would be wrong
under 2, 3 and 4. So nothing is flashed until it is measured.

### Next step

Board unpowered: resistance from the MTMS back pad to GND, compared with D0 to GND
(expected about 100 kΩ for both), and where the MTMS wire lands on the driver PCB.

### Measurement (user, 2026-10-02)

MTMS to GND, board unpowered: about 100 kΩ, steady. D0 and the continuity checks were
not reported.

### Root cause

No wiring fault. The reading rules out hypotheses 1, 2 and 4 (each reads far below
100 kΩ) and 3 (a drain or rail would read low and climb as its 220 µF charged). It is
the gate network every motor pin sees: 100 Ω + 100 kΩ to GND.

The original reading is explained by the method, not established by a test: the
internal pull-up (about 45 kΩ) against the 100 kΩ pulldown leaves about 2.3 V on the
pin, inside the band the ESP32-S3 does not guarantee to read as high or low (0.25–0.75
of 3.3 V), so one pin reading 0 where five read 1 is within specification.

GPIO42 is not wired to any other motor pin: TEST-044 read it differently from all five,
and pins on one node read alike. With 100 kΩ to GND it must be a PWM node, and the other
five are taken, so it is PWM3.

### Resolution

`EAD_MOTOR_ENABLED_MASK` = 0x3F: GPIO42 is driven LOW at boot with the others and motor 3
accepts service pulses. The bench firmware no longer skips it. After the flash: motor
service test available, sensor check 14/14 PASS, 15 s at 100.143 Hz, no faults. Whether
motor 3 turns is for the user to feel (TEST-049).

### Lessons

A pull-up/pull-down readback cannot tell a 100 kΩ pulldown from nothing, nor reliably
read the 2.3 V it makes; a resistance measurement on the unpowered board can.

## PROB-021 — USB 5 V and the switched battery meet at the XIAO's 5V pin

**Status:** Closed by the user (2026-10-02): "the power wiring is correct, I have
implemented a way". The provision was not described and has not been verified.
The hazard analysis below stays as the record.

### Symptoms

None observed. Found 2026-10-02 from the user's description of the power wiring.

### Environment

The DEC-016 build. The user wired the master switch output (from the MCP73833
module's LOAD+) to the ERM driver's PWR+ and to the XIAO's 5V pin. The design
(`wiring_reference.md` §9, the PCB's `ESP+` pad) used the XIAO's BAT+ pad and left
5V unconnected.

### Evidence

- **Confirmed (vendor doc):** Seeed's XIAO ESP32-S3 wiki: "5V — This is 5v out from
  the USB port. You can also use this as a voltage input but you must have some sort
  of diode … between your external power source and this pin, with anode to
  battery, cathode to 5V pin." So nothing on the XIAO stops USB 5 V reaching the pin.
- **Confirmed (datasheet):** the HT7833 accepts up to 8 V (absolute maximum 8.5 V)
  (Holtek HT78xx Rev 1.51), so 5 V on PWR+ does not harm the regulators. At 5 V in,
  each regulator dissipates 1.7 V × its load: about 0.2 W for one motor at 120 mA,
  about 0.46 W for three at 90 mA, close to its 0.50 W rating.

### Consequences (from the wiring, not measured)

- **USB plugged in, switch OFF (how TEST-043 ran):** USB 5 V powers the ERM driver,
  so both motor rails are live from the laptop's USB port. Motors still need a gate
  driven high to run.
- **USB plugged in, switch ON:** USB 5 V is connected straight to the charger
  module's LOAD+ output. What that does to the module and the cell depends on the
  module (whether LOAD+ is the cell itself) and on whether the cell has a protection
  circuit; both unknown. Treated as a hazard.
- **USB unplugged, switch ON:** works: the battery feeds the XIAO through its 5V pin
  and regulator. The 3.3 V rail will sag earlier as the cell discharges than it would
  on BAT+ (not measured).

### Workaround

Never turn the master switch ON while USB is plugged into the XIAO.

### Options (the user's decision)

1. Move the XIAO feed from the 5V pin to the BAT+ pad, as designed. The XIAO's own
   power path then handles USB and battery; with USB plugged in it charges the cell
   at about 50 mA through BAT+ when the switch is ON.
2. Keep the 5V pin and add a Schottky diode (anode at the switch output, cathode at
   the 5V pin), as Seeed instructs. Costs about 0.3 V of the battery range.

Root cause: a deviation from the designed power wiring.

## PROB-022 — Still on the desk, the gyroscopes show episodes of a few °/s

**Status:** Open — observed, cause unknown

### Symptoms
A 2 s calibration of the BNO086 build, device untouched on the desk, was rejected as
"the sensor moved": foot gyro sd 3.82 °/s against the 2 °/s limit (hardware test run,
2026-10-02). The same test passed on its own and in a later run.

### Investigation
- Eight 2 s calibrations from 2.8 s after a board reset: all accepted, foot sd 0.0–0.97,
  shank 0.0–0.23 °/s.
- 40 s still recording after a reset, largest |gyro| per 2 s: foot 3.5 °/s at 12–17 s
  and 2.4–3.1 °/s at 26–34 s, near 0 between; shank 0.6–1.2 °/s in the same intervals
  and 0 between. A 20 s recording at another time: foot ≤ 1.2, shank ≤ 0.9 °/s.
- The episodes are simultaneous in both sensors.

### Hypotheses (none tested)
1. Real motion of the desk (the two sensors lie on it together).
2. The BNO086's own dynamic gyroscope calibration adjusting.

### Workaround
Calibrate again when a window is rejected as "moved" (the hardware test retries up to
three times).

Root cause: Unknown. A test that tells them apart: the device on a heavy, isolated
surface, or one sensor held in a vice and the other loose.

## PROB-023 — A step into a stop with no impact left the gait engine in swing, without zero-velocity updates

**Status:** Resolved (flashed 0.1.0+77ae282, 2026-10-02)

### Symptoms
In the first 10 m walks on the BNO086 build (TEST-052), cycles of 16–29 m: the
cycle spanning a stop (stand, turn, stand) and the first cycle after it. Recorded
totals of 13–33 m per recording for 20 m walked.

### Environment
Firmware 0.1.0+9c91b7d, measured mount maps (TEST-051), Wi-Fi, battery. Replayed
on the host with `tools/replay` from the dashboard database
(`tools/session2eadlog.py`), which reproduces the device's cycles to 0.01 m.

### Investigation
`eadreplay --trace` on `20261002-153957-590d`: toe-off at 22.46 s, then the last
step into the stop landed with no impact above the contact threshold. The engine
stayed in `Swing` until the next contact at 39.70 s, through 15 s of standing
with |a| 1.021 g and a foot rate below 2 °/s. The zero-velocity update is only
allowed outside `Swing` (`stanceContext`), so the velocity integrated unchecked:
ZUPT quality 0.02 for that 18 s cycle, 29.23 m. At the next foot-flat,
`removeSegmentDrift` removed the drift over the whole 17.6 s moving segment from
a displacement reset at the contact 0.3 s before: 16.47 m for a 1.59 s cycle.

### Root cause
Confirmed: `Swing` had no exit except a detected contact. A soft footfall
(the last step into a stop, a first step from standing) left the engine in swing
for as long as the foot stood still. The device's own first cycles (20.4 m in
`20261002-154232-3056`) came from the same state carried over from before the
recording.

### Resolution
`gait.cpp`: in `Swing`, a foot still for as long as a zero-velocity window
needs (`kZuptHoldMs + kZuptEntryHysteresisMs`, 80 ms) goes to `Stance`. No
contact is claimed; the open cycle runs on to the next contact, so the cycle
over a stop stays long and invalid.

### Verification
- Unit test `test_a_step_into_a_stop_without_an_impact_still_gets_zero_velocity`:
  fails on the old engine, passes on the new.
- Replay of the five walks (TEST-052): the stop cycle in `153957-590d` goes from
  29.23 m (ZUPT quality 0.02) to 1.07 m (0.79); the next from 16.47 m to 0.84 m.
  `walk6m-2026-09-18` is unchanged: 7 contacts, 6 valid cycles, 6.39 m.
- Two contacts are no longer reported: the first landing after a turn in
  `154336-5c28` (47.79 s) and in `154509-130f` (35.75 s). Both were real
  landings, seen only because the engine was still in swing from before the
  stop. The first landing after standing still was already missed on every
  other leg; it is PROB-024.

### Lessons
Every state needs an exit that does not depend on the event the state waits for.

## PROB-024 — BNO086 walks: soft landings missed, slow-walk strides read short

**Status:** Open

### Symptoms
From the five 10 m walks (TEST-052), replayed with the PROB-023 fix:
1. **Soft landings are not contacts.** The first one to three landings from
   standing and the closing step into a stop. Slow walk, leg 1: 11 toe-offs
   (the user counted 11 right-foot landings), 8 contacts. Normal pace: 8 of 9.
   Fast: 8 of 8 counted.
2. **Slow strides read about 30 % short.** Median 0.65–0.73 m per cycle against
   0.91–1.00 m from the course and the count, with ZUPT quality ≥ 0.15 on 12 of
   15 cycles. Normal pace reads 1.04–1.25 m (expected 1.11–1.25), fast 1.24–1.30 m
   (expected 1.25–1.43).
3. **Low ZUPT quality at normal and fast pace.** Below the error engine's 0.15
   (`kDistanceMinZuptQuality`) on 33 of 47 normal-pace cycles and 13 of 14 fast
   ones. Those cycles' distances run from 0.7 to 6.3 m.
4. **A slow stride split in two** once (`154509-130f`, contacts at 40.56 s and
   41.24 s after a 0.21 s swing).

### Root cause
Unknown for all four. Hypotheses, none tested:
- 1: the contact threshold (`impactG`) was tuned on the MPU6500 build.
- 2: the linear drift ramp over a moving segment is applied at the foot-flat
  after the contact, to a displacement reset at that contact, so a segment's
  swing and its correction land in different cycles.
- 3: the 25 °/s foot-rate limit for a zero-velocity window is too tight for a
  board on the instep at these paces.

### Attempts
All on the five walks (`tools/replay/walks.py`), with the 6 m MPU6500 walk as a
regression check (6 valid cycles, 6.39 m before and after every kept change).

#### Attempt 1: lower the contact confirm level (rejected)
`--confirm` 1.0: 90 of 92 landings, but 4 slow strides split in two (cycles under
0.9 s at a 1.8 s cadence). 0.9 and below: the 6 m walk gains 2 false cycles. Slow
landings peak at 0.9–1.0 g above gravity, and so does push-off: the level cannot
separate them.

#### Attempt 2: a swing that ends in stillness ended in a footfall (kept)
In `Swing`, the strongest impact once the swing could end (`kMinSwingS`) is kept;
when the foot then stands still (PROB-023's exit) and that impact reached the
candidate level `impactG`, it is claimed as the contact, subject to the same
refractory period (DEC-019). Contacts 86 → 92 of 92 counted; no new split
strides; the 6 m walk unchanged. The new contacts are the soft second landing of
the slow walk (8.68 s), the closing steps into each stop, and one step in two of
the turns.

#### Attempt 3: lower the swing-start rate to catch the first step (rejected)
The first step from standing in `walk10m-normal3` swings at 86 °/s, under
`kSwingGyroDps` (90). `--swing-rate` 80: 5 split strides at fast pace; 70: 17;
60 and 50: worse. Kept at 90.

#### Attempt 4: the stillness test on the event path (kept)
Traced in `walk10m-normal3` at 19.39–19.60 s: the foot turns at 11–24 °/s for
210 ms, but |a| alternates 0.82 / 1.24 / 0.82 g between consecutive samples, and
each sample outside 1 ± 0.15 g restarts the 80 ms count. The MPU6500 build had a
42 Hz low-pass in the sensor (PROB-003); the BNO086 build has none, and doc 04 §7's
filtering layers (6 Hz gait path, 20 Hz event path) were never implemented. The
stillness test now reads |a| through a 2nd-order Butterworth low-pass at 20 Hz
(`LowPass2`, DEC-019). Both uncorrected cycles in `walk10m-normal3` leg 1 are
corrected (13.00 → 9.57 m); uncorrected valid cycles 8 → 5 across the walks.

#### Attempt 5: a higher gyro limit for stillness (applied 2026-10-04 by the user's decision, DEC-020; TEST-056)
The five remaining uncorrected stances dip under 25 °/s for only 50–90 ms at a
time (minimum 14–18 °/s). `--zupt-gyro` 30: uncorrected cycles 5 → 2,
`walk10m-normal2` 24.2 → 18.3 m, `walk10m-fast` 24.2 → 17.4 m, one slow contact
lost, 6 m walk 6.34 m. 35: 1 uncorrected. 25 °/s is the contract's (doc 05 §6);
changing it is the user's decision.

#### Investigation: why slow strides read short
Instrumented `removeSegmentDrift` in a scratch build (not committed). At normal
pace each swing ends at about −2 m/s (it should end at 0), and the linear drift
correction adds about +1.2 m per stride: most of a normal stride comes from the
correction. At slow pace the end velocity is ±0.3 m/s, the correction is under
0.25 m, and the raw integration is what reads 30 % short. The Mahony filter
already ignores the accelerometer when |a| is more than 0.25 g from 1 g; the
MPU6500 build reached 6–10 m/s at the end of a swing (TEST-030). Cause of the
slow-walk shortfall: Unknown.

### Current state (after attempts 2 and 4)
1. Contacts: 92 of 92 counted; per leg, one first step from standing is still
   missed when its swing stays under 90 °/s (`walk10m-normal3` leg 2,
   `walk10m-slow` leg 1).
2. Slow strides: median 0.69 m, legs 6.24 and 6.16 m; unchanged. Open.
3. Five valid cycles without a zero-velocity update, each followed by one of
   2.5–6.3 m (`walk10m-normal2` leg 2, `walk10m-fast` leg 1). Open; attempt 5 is
   the user's decision.
4. The slow split stride (40.55 s): unchanged. Open.
5. Legs without an uncorrected cycle read 8.0–9.6 m at normal and fast pace, for
   10 m less the first step from standing.

### Barefoot walks at 200 Hz (2026-10-04, TEST-058)
Seven video-synced walks on schema 6, firmware 0.1.0+523b49d:
1. Slow legs read 9.4–10.3 m per 10 m: symptom 2 is not reproduced. Not explained:
   the rate, the orientation source and the footwear all changed together.
2. New: a second contact about 1.0 s after a swing, with no swing of its own, splits a
   stride (40 such contacts in 272). In fast 1's return leg contacts come every
   0.5–0.6 s and one cycle reads 5.04 m; the video shows a normal walk with the
   sensors in place, so the detector, not the recording, is at fault.
3. Root cause of 2: Unknown. Not yet examined trace by trace.

### Shank-swing detection (2026-10-04, DEC-022, TEST-059)
Symptoms 1 and 4 and the split strides of TEST-058 are resolved on the fixtures:
contacts are one per shank swing, timed at the foot's impact. Counting error over
176 landings: 21 → 3. Not yet flashed or tried on the leg.

Still open: symptom 2 on the 2026-10-02 slow walk (6.19 and 7.02 m per 10 m leg);
the 2026-10-04 barefoot slow walks read 9.4–9.6 m. That recording differs in rate
(100 Hz), orientation (Mahony), footwear and day. Root cause: Unknown.

### Next
Flash and walk with the shank detector; then the distance work (DEC-020 6c).

## PROB-025 — Dashboard: event and step batches bypassed gap detection; backfilled ones were dropped

**Status:** Resolved (2026-10-04)

### Symptoms
None observed by the user. Found by the agent that ported the dashboard to schema 6, and
confirmed by reading `dashboard/src-tauri/src/device.rs`.

### Root cause
Confirmed by reading the code: `Tracker::handle` matched EVENT_BATCH and STEP_BATCH before
its catch-all for durable messages, so their sequence numbers never reached `on_durable`.
Each looked like a gap and was requested again; the backfilled copy then reached
`on_durable`, which decoded only raw batches, and was dropped. A real gap containing a
step batch lost its cycles.

### Resolution
Both are decoded in `on_durable`, after the sequence bookkeeping, like raw and
accelerometer batches. Test `gait_batches_are_durable_and_reach_the_sink`: a live event
(51) and step batch (52) after a raw batch (3) leave exactly 4–50 missing, and a step batch
arriving by backfill reaches the sink.

## PROB-026 — Feedback kept running on a dead or frozen sensor; OFF could miss a motor

**Status:** Resolved in code (2026-10-05); not exercised on hardware

### Symptoms
Found by an external code audit on 2026-10-05 (finding I10), confirmed by reading the
code at `982a350`; no failure was observed on the device.
1. A sensor that stopped sending left its last rotation vector in the acquisition
   history. `History::nearest` had no age limit, so every later frame carried it as
   current, orientation stayed valid and nothing called `feedback::fault`.
2. Device-level faults (no data from a sensor, frozen data, acquisition stalled) only
   lowered the sensor-quality subscore (weight 0.30). An episode continues down to
   confidence 0.50, so it could carry on. Doc 06 §12: any sensor fault or stale sample
   turns every motor off.
3. `motors::run` wrote the PWM outputs before recording which were active. A
   `stopAll` from the link task in between read the old set, missed the new outputs,
   and they ran to the end of the cue (250 ms).
4. `session::stop` built the reference after releasing its lock, while the processing
   task, which had read the session as active, could still be adding a cycle to the
   builder; `consume` returned a pointer to a score a new `start` could reset.

### Root cause
1–2: the fault path covered frame-level failures only (read failure, frame gap, no
orientation). 3–4: shared state between the link and processing tasks protected for
the flags but not for the objects they guard.

### Resolution
- `ead::feedbackFault` (`lib/ead_core/src/ead/feed.{h,cpp}`) is the one rule:
  read failure, missing rotation vector, no orientation, a gap, or any device fault.
  Saturation and held or repeated samples are not faults: a heel strike can saturate.
  `gait::consume` stops feedback on such a frame and runs no cue for a cycle that
  closes on one. A rotation vector more than `kMaxSampleAgeUs` (50 ms, ten frame
  periods) from its frame is reported missing.
- `motors.cpp`: one FreeRTOS mutex around every output change (cue, OFF, stop timer).
  A stop-timer callback that waited behind a newer cue leaves it alone (end time
  check with a 1 ms margin); a cue whose stop timer fails to start is turned off and
  refused.
- `session_service.cpp`: one mutex around start, stop and consume; the score is
  returned by copy. The unused `capturedCycles` was removed.

### Verification
Native test `test_a_sensor_fault_of_any_kind_stops_feedback` (test_feed). The motor
and session changes are concurrency fixes in device code with no host test; the
firmware builds. Not exercised on hardware: unplugging a sensor during a cue needs
the battery and Wi-Fi.

### Lessons
A safety rule written in one place (doc 06 §12) has to be one function in the code,
not a list of conditions repeated at each call site.

## PROB-027 — A second calibration was accepted and ignored

**Status:** Resolved in code (2026-10-05); not verified on hardware

### Symptoms
Audit finding I03, confirmed in the code: after the first accepted calibration, a
new one was reported to the dashboard as accepted, but orientation and the gait
engine kept the first record's alignment and gyroscope bias until a reboot.

### Root cause
`processingTask` called `orientation::adopt()` only while orientation was invalid,
which it never is again after the first adoption. `gait::consume` cached the record
behind `s_haveCalibration`, which `gait::reset()` did not clear.

### Resolution
`calibration::consume` returns true on the frame that completes a window; the
processing task adopts every completion. `gait::reset` clears `s_haveCalibration`.
`orientation::valid`, now unused, was removed.

### Verification
Firmware builds; device code with no host test. To verify on the board: calibrate,
tilt the foot sensor about 20° and hold it, calibrate again, and check that the
foot's sagittal angle reads near 0 again.

## PROB-028 — The initial-contact angle was read after the foot was flat

**Status:** Resolved (2026-10-05)

### Symptoms
Audit finding I04: a contact at −20° followed by a flat foot reported 0°.

### Root cause
The detector keeps the strongest impact's time and frame and decides the contact up
to `kContactSearchS` (0.15 s) after the swing rate crosses zero, but passed the angle
of the deciding sample to `claimContact`. Present since `a4ab310` (the old engine's
confirm window had the same pattern) and kept by DEC-022.

### Resolution
`bestImpactSagittalDeg_` is stored with the impact and passed to `claimContact`. The
field's meaning is unchanged: the angle of the landing that closes the cycle.

### Verification
`test_the_contact_angle_is_the_one_at_the_impact` (test_gait) fails on the previous
engine ("Expected -20 Was 0") and passes.

### Lessons
A backdated event must carry every value measured at its time. Remaining: the other
per-cycle quantities still run to the deciding sample (docs/implementation.md, "Gait
event detection", Limitations).

## PROB-029 — A failed database commit threw the recording's data away

**Status:** Resolved (2026-10-05)

### Symptoms
Audit finding I01, confirmed in the code: when a commit failed, `commit` printed the
error to stderr and cleared every pending batch. `Store::flush` returned nothing and
ignored its own 10 s timeout, so a stop or an export went ahead as if all was saved.
The audit reproduced it with an SQLite trigger refusing one frame insert. No such
failure has been seen on the user's machine.

### Root cause
The writer's error path, from its first version (`317b67f`): failure was logged, not
handled.

### Resolution
`dashboard/src-tauri/src/store/mod.rs`:
- `commit` keeps the batches when the transaction fails (it rolled back, so a retry
  writes each row once) and retries at the next interval.
- Frames and accelerometer samples held for retry are capped at
  `MAX_RETAINED_ROWS` (five minutes of recording, about 30 MB); past it the oldest
  batches of the longer stream are dropped and counted (`Pending::trim`).
- `WriteHealth`, shared with the writer: the current failure, and rows lost since
  the recording started. `Store::write_problem` words it; the state bar shows it
  beside the recording (`write_problem` command).
- `flush` returns an error for a failing commit, lost rows, a stopped writer or the
  timeout, and no longer holds the writer lock while it waits. `close_recording`
  closes the session and then returns that error, so stopping reports it.

### Verification
`a_failed_commit_is_retried_and_reported_not_dropped`: the audit's trigger; flush
reports "disk refused", nothing is stored; the trigger is dropped and the next flush
stores all 10 frames and clears the warning.
`retention_drops_the_oldest_batches_of_the_longer_stream`. The first cannot compile
against the old `flush`, which returned nothing; the behaviour it checks is the one
the audit reproduced.

### Limitations
Lost rows are reported while the app runs, not stored with the session: a later
reader sees them only as gaps in `frame_index`. Data still held for retry when the
app closes is lost with the process (stderr says so).
