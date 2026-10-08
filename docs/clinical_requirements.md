# Clinical requirements — coverage and traceability

Source: the researcher's four additions to the original quotation ("Critical Clinical
Insights", kept locally). The contract package folded these into
`ead_agent_docs_v2/SOURCE_REQUIREMENTS.md`; this file maps each one to the
specification that defines it, the code that implements it, and the test that shows
it.

**Status (2026-10-08):** all four are implemented on the BNO086 build and have run
worn, on battery over Wi-Fi, through the full reference → check → evaluation workflow
(TEST-066–068, TEST-071).

"One number per step" is, as built, one number per **right-leg gait cycle** (right
contact to right contact, about two steps), matching the right-leg scope of V1.

| # | Requirement | Specified in | Implemented | Verified |
|---|---|---|---|---|
| 1 | ZUPT drift correction so speed and distance stay trustworthy for a whole session | doc 05 §6–§8 | Yes | TEST-030: 6.39 m on a 6.00 m course; 200 Hz worn walks with 0 frames lost (TEST-058) |
| 2 | One deviation number per step, driving vibration strength, with a hard safety limit | doc 06 §1–§5 | Yes, per right-leg cycle; graded cues 60–100 % (DEC-026) | TEST-031, TEST-060, TEST-069; deliberate-error walks cued in the matching direction (TEST-071) |
| 3 | Compare each patient to their own baseline from a calibration walk, saved between sessions | doc 12 §2–§3, §6 | Device builds it, dashboard versions and locks it | TEST-031, store tests; patient 67's v6 baseline used in TEST-071 |
| 4 | Export full raw accelerometer, gyroscope and orientation at native rate, timestamped per sample | doc 09 §6, doc 10 | Yes: every 200 Hz frame with its timestamp; both accelerometers also at their native 250 Hz, each sample timestamped (DEC-021) | TEST-018, TEST-022, TEST-029, TEST-035–037 |

## 1. Drift correction (ZUPT)

**Asked for:** "every time the foot is flat on the ground for a moment, the system
resets itself… without this, I won't trust the walking-speed and distance numbers by
the end of a session."

**Specified:** foot-only zero-velocity detector (|‖a‖ − 1 g| ≤ 0.15 g, gyro ≤ 25 °/s,
held 60 ms, in stance), applied as an error-state Kalman update on the velocity
substate rather than a hard reset (doc 05 §6–§7). Speed is reported per cycle with a
quality figure, and flagged as not measured rather than estimated when ZUPT quality is
low (doc 05 §8).

**Built:** the detector, the zero-velocity windows and the distance estimate, measured
against a 6 m course: 6.39 m (TEST-030). Every foot-flat resets the velocity, so drift
cannot build up across a session; cadence comes from cycle timing and does not drift.
Strides outside walking range (over 2.5 m or 2.5 m/s) are not scored.

Thresholds are named constants, overridable at run time, so they can be adapted per
patient (doc 05 §3).

**Foundation:** a gap-free sample stream with real device timestamps. The 30-minute run
(TEST-018) kept all 180,250 frames. Integration uses the per-frame timestamps, not a
nominal period.

## 2. One error number per step

**Asked for:** "one simple number per step that tells me how far off that step was from
normal… the buzz actually gets stronger the more off the step was, with a limit so it
never gets too strong to be unsafe."

**Specified:** exactly one `error_score` in [0,1] per gait cycle, a weighted sum of
seven robust per-feature deviations (doc 06 §2–§3); an independent confidence score
(§5); duty clamped, 5 s maximum continuous on-time and 50 % duty over any 10 s
(§10–§12).

**Built:** the score, the six classes and the five confidence subscores in
`ead_core/error_engine.cpp`, tested against hand-computed cases (TEST-031). Doc 06's
four open values are chosen and justified in DEC-013.

The vibration (DEC-023, DEC-025, DEC-026): one 500 ms cue per scored cycle in an
evaluation, strength `153 + 102·score^1.5·confidence` of 255 (60–100 %), capped at the
motor rail's maximum, held to the 5 s and 50 % limits. The cue sits on the side of the
calf that names the error. An episode ends at the first step under 0.35. Cues pause on
a sensor fault and after 5 s without the laptop (DEC-027).

TEST-071 (2026-10-07): inversion, eversion, plantarflexion and fast walking were cued
in the matching direction.

## 3. Patient-specific baseline

**Asked for:** "a short calibration walk at the start of each session that sets that
patient's own baseline and saves it for next time… not a standard healthy-population
gait pattern."

**Specified:** a reference capture of at least 30 valid cycles with feedback off,
stored as a versioned, immutable profile per patient; each later session runs a
10-cycle check against it; the profile is locked during evaluation so a session can
never change the baseline (doc 12 §2–§3, §6).

**Built:** the whole path. The device collects valid cycles during a REFERENCE_CAPTURE
session and builds a profile from at least thirty; the dashboard assigns a per-patient
version, stores the profile exactly as the device sent it, and locks it once used. The
lock is enforced by the database itself (a trigger), so no code path can edit a used
reference. Profiles persist in the patient's record between sessions.

The profile is computed on the device and versioned by the dashboard (DEC-012), so the
median/MAD statistics have one implementation.

A baseline belongs to the strapping it was recorded in; a new one is recorded after
re-strapping.

## 4. Raw data export

**Asked for:** "full IMU output (accelerometer, gyroscope, orientation) at the native
sampling rate, with a synchronized timestamp on every sample… so we can later check
the device's accuracy against a reference system."

**Capture:** every frame carries a device timestamp and an index; both sensors are read
in the same frame; samples are stored exactly as the device sent them, in native
counts, with the mount maps and scale factors alongside (DEC-007). Frames lost in
transit are re-requested from the device's own buffer (about 4.5 min), and any
remaining gap is counted and shown. Evidence: 180,250 of 180,250 frames over 30
minutes (TEST-018); a dashboard session stored with zero missing (TEST-022).

**Export:** the whole doc 10 package: `raw.csv` (accelerometer, gyroscope and
rotation vector per sensor per frame, timestamped), `accel_native.csv` (both
accelerometers at their native rate, every sample timestamped), `gait.csv`,
`events.csv`, `haptics.csv`, `metadata.json`, `session.mat` and `report.pdf`, each
with the patient's name (DEC-028). Evidence: TEST-035 (the package matches the
database row for row), TEST-036 (`scipy.io.loadmat` agrees with the CSV), TEST-037
(every report section present).

The shank's gyroscope and rotation vector are aligned to the foot-clocked frames.
Timestamps are on the device clock, ready for alignment with an external
reference system in analysis.

## What the dashboard checks

- both sensors present, configured and streaming;
- acceleration magnitude at rest (about 1 g), the fastest check of scaling and mounting;
- dropped frames, bus errors, repeated samples, missing messages, corrupted link frames;
- per-frame flags including saturation on any axis;
- the device's configuration, verified against the hash it reports, recorded with every
  session;
- per-cycle distance with ZUPT quality, the error score, class and confidence, the
  reference workflow, and the export package (sections 1–4).
