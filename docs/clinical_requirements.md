# Clinical requirements — coverage and traceability

Source: `Critical Clinical Insights.docx` (repository root, not tracked by git),
the researcher's four additions to the original quotation. The contract package
folded these into `ead_agent_docs_v2/SOURCE_REQUIREMENTS.md`; this file maps each
one to the specification that defines it, the code that implements it, and the
test that proves it.

**Status summary (2026-09-17):** the data foundation every requirement depends on
is built and verified. The clinical analysis itself — drift correction, the error
score, the patient baseline — is specified but **not yet implemented**. Nothing
in the document is missing from the plan; three of the four are scheduled work.

| # | Requirement | Specified in | Implemented | Verified |
|---|---|---|---|---|
| 1 | ZUPT drift correction so speed and distance stay trustworthy for a whole session | doc 05 §6–§8 | No — milestone M4 | — |
| 2 | One deviation number per step, driving vibration strength, with a hard safety limit | doc 06 §1–§5, §10–§12 | No — M5; vibration itself deferred (DEC-006) | — |
| 3 | Compare each patient to their own baseline from a short calibration walk, saved between sessions | doc 12 §2–§3, §6 | No — M5 | — |
| 4 | Export full raw accelerometer, gyroscope and orientation at native rate, timestamped per sample | doc 09 §6, doc 10 §2 | Capture: yes. Export files: no — M6 | TEST-018, TEST-022 |

## 1. Drift correction (ZUPT)

**Asked for:** "every time the foot is flat on the ground for a moment, the
system resets itself… without this, I won't trust the walking-speed and distance
numbers by the end of a session."

**Specified:** foot-only zero-velocity detector (|‖a‖ − 1 g| ≤ 0.15 g, gyro
≤ 25 °/s, held 60 ms, in stance), applied as an error-state Kalman update on the
velocity substate rather than a hard reset (doc 05 §6–§7). Speed is reported per
cycle with a quality figure, and flagged low-confidence rather than fabricated
when ZUPT quality is poor (doc 05 §8).

**Built so far:** nothing of the detector. What exists is the requirement it
depends on: a sample stream with no gaps and real device timestamps. Drift
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

**Built so far:** nothing. This needs gait events (M4) before it can exist.

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

**Built so far:** the storage side only — patients and sessions exist in the
database, and every session records the firmware version and configuration hash
that produced it. Reference profiles themselves are M5.

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

**Not yet built:** the export files themselves — `raw.csv`, `gait.csv`,
`events.csv`, `haptics.csv`, `metadata.json`, MATLAB `session.mat` and the PDF
report (doc 10). That is milestone M6. Today the data can only be read out of the
SQLite store directly.

**Gap to close in M3:** the orientation quaternions in each frame are still
identity, because orientation is not estimated yet. The export is not complete
against this requirement until they are real.

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
