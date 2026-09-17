# Clinical requirements — coverage and traceability

Source: `Critical Clinical Insights.docx` (repository root, not tracked by git),
the researcher's four additions to the original quotation. The contract package
folded these into `ead_agent_docs_v2/SOURCE_REQUIREMENTS.md`; this file maps each
one to the specification that defines it, the code that implements it, and the
test that proves it.

**Status summary (2026-09-18):** all four are implemented. Two are verified on
hardware against measured ground truth; the other two have only been exercised
against synthetic data, because nobody has yet walked the thirty cycles a real
reference profile needs. Until that walk happens, requirements 2 and 3 are code
that has never seen a patient.

| # | Requirement | Specified in | Implemented | Verified |
|---|---|---|---|---|
| 1 | ZUPT drift correction so speed and distance stay trustworthy for a whole session | doc 05 §6–§8 | Yes | TEST-030: 6.39 m measured on a 6.00 m course, ZUPT quality 0.22–0.29 per cycle |
| 2 | One deviation number per step, driving vibration strength, with a hard safety limit | doc 06 §1–§5 | Yes, except the vibration itself (DEC-006, no drivers fitted) | TEST-031, hand-computed cases only |
| 3 | Compare each patient to their own baseline from a short calibration walk, saved between sessions | doc 12 §2–§3, §6 | Device builds it, dashboard versions and locks it | TEST-031 and four store tests; never yet captured from a person |
| 4 | Export full raw accelerometer, gyroscope and orientation at native rate, timestamped per sample | doc 09 §6, doc 10 | Yes: capture with real orientation, and the whole doc 10 package | TEST-018, TEST-022, TEST-029, TEST-035, TEST-036, TEST-037 |

## 1. Drift correction (ZUPT)

**Asked for:** "every time the foot is flat on the ground for a moment, the
system resets itself… without this, I won't trust the walking-speed and distance
numbers by the end of a session."

**Specified:** foot-only zero-velocity detector (|‖a‖ − 1 g| ≤ 0.15 g, gyro
≤ 25 °/s, held 60 ms, in stance), applied as an error-state Kalman update on the
velocity substate rather than a hard reset (doc 05 §6–§7). Speed is reported per
cycle with a quality figure, and flagged low-confidence rather than fabricated
when ZUPT quality is poor (doc 05 §8).

**Built (2026-09-18):** the detector, the zero-velocity windows and the distance
estimate, measured against a 6 m course: 6.39 m, +6.5 % (TEST-030). Cycles whose
ZUPT quality is inadequate report distance as low-confidence rather than
corrected, which is what this requirement asks for. Two orientation errors that
made distance untrustworthy were found and fixed by that measurement (PROB-012).

**Also true, and worth stating:** the thresholds were fitted to one recording of
one person's walking. They are named constants, overridable at run time, and doc
05 §3 wants them adaptive per patient.

**What it depends on, verified earlier:** a sample stream with no gaps and real device timestamps. Drift
correction integrates acceleration over time, so a dropped or mistimed sample
becomes a permanent position error. The 30-minute run (TEST-018) lost none of
180,250 frames and held a 9985.5 µs period with 0.5 µs standard deviation.

**Note for implementation:** the sample rate is 100.145 Hz, not 100 Hz — it comes
from the sensor's own oscillator. Integration must use the per-frame timestamps;
assuming 10.00 ms would accumulate about 0.15 % of drift by itself.

## 2. One error number per step

**Asked for:** "one simple number per step that tells me how far off that step
was from normal… the buzz actually gets stronger the more off the step was, with
a limit so it never gets too strong to be unsafe."

**Specified:** exactly one `error_score` in [0,1] per gait cycle, a weighted sum
of seven robust per-feature deviations (doc 06 §2–§3); an independent confidence
score gating feedback (§5); intensity `51 + 153·error^1.5·confidence` clamped to
20–80 % duty, 5 s maximum continuous on-time and 50 % duty over any 10 s (§10–§12).

**Built (2026-09-18):** the score, the six classes and the five confidence
subscores, in `ead_core/error_engine.cpp`, tested against hand-computed cases
(TEST-031). Confidence is reported beside the score and never folded into it: a
cycle can deviate a great deal and be worth little.

Doc 06 leaves four values undefined; they are chosen and justified in DEC-013
rather than invented silently.

**Important:** the vibration itself cannot be delivered on the current hardware —
no ERM driver channels are fitted, which is why there is no haptic code (DEC-006,
your decision). The dashboard states "haptics not fitted" rather than implying
feedback is happening. The error score is still worth building without it: it is
the research measurement, and it drives the feedback once drivers are added.

## 3. Patient-specific baseline

**Asked for:** "a short calibration walk at the start of each session that sets
that patient's own baseline and saves it for next time… not a standard
healthy-population gait pattern."

**Specified:** a reference capture of at least 30 valid cycles with feedback off,
stored as a versioned, immutable profile per patient; each later session runs a
10-cycle check against it; the profile is locked during evaluation so a bad
session can never teach the device bad gait (doc 12 §2–§3, §6).

**Built (2026-09-18):** the whole path. The device collects valid cycles during a
REFERENCE_CAPTURE session and refuses to produce a profile from fewer than
thirty; the dashboard assigns a per-patient version, stores the profile exactly
as the device sent it, and locks it the moment it is used to judge a session.
The lock is enforced by the database itself — a trigger refuses the update — so a
reference cannot be edited after an evaluation has been made against it, by any
code path.

Profiles persist between sessions in the patient's record, which is the "saves it
for next time" half of the requirement.

**Decision already taken:** the profile is computed on the device and versioned
by the dashboard (DEC-012), so there is one implementation of the median/MAD
statistics rather than two that could disagree.

## 4. Raw data export

**Asked for:** "full IMU output (accelerometer, gyroscope, orientation) at the
native sampling rate, with a synchronized timestamp on every sample… so we can
later check the device's accuracy against a reference system."

This is the one requirement whose hard half is done.

**Capture — built and verified.** Every frame carries a device timestamp and an
index; both sensors are read in the same frame; samples are stored exactly as the
device sent them, in native ADC counts, with the mount maps and scale factors
kept alongside (DEC-007). Nothing is overwritten with filtered values. Frames
lost in transit are re-requested from the device's own buffer, and any that
remain missing are counted and shown rather than quietly skipped.

Evidence: 180,250 of 180,250 frames over 30 minutes with zero missing and zero
corrupted (TEST-018); the dashboard storing a session from the device with zero
missing (TEST-022).

**Export — built (2026-09-18).** The whole doc 10 package: `raw.csv` with one row
per IMU sample and the frame's timestamp on both, `gait.csv`, `events.csv`,
`haptics.csv`, `metadata.json`, `session.mat` and `report.pdf`. The EXPORT view
reports the row counts it wrote rather than announcing success.

Raw values are exported as the stored ADC counts, not physical units (DEC-007),
with the scale factors and both mount maps in `metadata.json`. That is what the
requirement asks for — the device's own output, checkable against a reference
system — and it is what keeps doc 10 §7's rule, that the raw integers survive
into the `.mat`, true of the CSV as well.

Evidence: TEST-035 (the package matches the database, row for row), TEST-036
(`scipy.io.loadmat` reads `session.mat` and every value agrees with the CSV
beside it), TEST-037 (`pdftotext` finds every section doc 10 §8 names). Both
checkers found real defects on their first run, recorded in those entries.

**Closed since this section was first written:** the orientation quaternions were
identity until M3. They are now the device's own Mahony estimate, and
`raw.csv`/`session.mat` carry them per frame as Q15 integers.

## What the dashboard checks today

Live view and recording cover sensor health and data integrity, not clinical
measurement:

- both sensors present, configured, and not frozen;
- acceleration magnitude, which must read about 1 g at rest — the fastest check
  that scaling and mounting are right;
- dropped frames, I²C errors, repeated shank samples, missing messages,
  corrupted link frames;
- per-frame flags including saturation on any axis;
- the device's own configuration, verified against the hash the device reports,
  recorded with every session.

It does **not** yet check anything in the clinical document: no drift correction,
no per-step error, no patient baseline, no export. Those are M4, M5 and M6.
