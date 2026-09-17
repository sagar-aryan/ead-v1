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

**Status:** Resolved in firmware (2026-09-17); on-body standing check pending

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
Shank map `X = −chipZ, Y = +chipX, Z = −chipY`, enforced as a proper rotation at
compile time.

### Verification
- Build-time: the committed map passes both static assertions. A copy of the
  header with the previous map fails with "shank mount map must be a proper
  rotation" (g++ 13, `-std=gnu++17`, 2026-09-17).
- Host check: chip (1, 2, 3) maps to anatomical (−3, 1, −2), and chip X = −32768
  maps without overflow.
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
