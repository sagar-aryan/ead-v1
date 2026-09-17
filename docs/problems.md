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

**Status:** Investigating (candidate map in firmware, needs locked confirmation)

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

### Attempts
#### Attempt 1
Candidate map committed in `config_v1.h` (EAD_SHANK_MAP_*); viewer unaffected
(firmware outputs anatomical).

### Root Cause
Root cause: Unknown (pending controlled standing-still capture).

### Resolution
Pending: 5 s standing-still capture, then toe-lift/tilt checks in viewer.

### Verification
Not yet verified.

### Lessons
One unknown-pose capture cannot fix a 3-axis map; always capture in a
declared known pose.

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
