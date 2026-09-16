# Testing

Plan derived from `ead_agent_docs_v2/13_TEST_AND_VALIDATION_PLAN.md`.
No tests have been executed in this scaffolding task — all entries below are
PLANNED, not claimed passes.

## Replay determinism (mandatory, doc 13 §9)

### Objective
A saved raw dataset plus fixed configuration must reproduce identical gait
events, cycle features, error score, error class, and haptic decisions.

### Procedure (planned)
1. Capture or synthesize raw dual-IMU datasets covering doc 13 §3: normal
   walking, slow walking, variable cadence, dorsiflexion / plantarflexion /
   inversion-eversion deviations, sensor noise bursts, dropped samples,
   static periods, invalid values.
2. Store raw frames (ESP32 timestamps + seq) as versioned fixtures with the
   exact config (gains, thresholds, reference version, segment limits).
3. Run the firmware-equivalent algorithm (host replay harness) twice on each
   fixture; diff events/features/scores/classes/haptic commands byte-for-byte.
4. Re-run after every algorithm change as a regression gate (Phase 11).

### Expected
Bit-identical outputs across runs for identical inputs.

### Acceptance
Replay suite passes before any bench walking test counts toward release
(doc 14 Phase 12 release gate: replay determinism verified).

## TEST-001 — Hardware bring-up (planned)

### Objective
Verify electrical integration per doc 13 §1.

### Procedure
1. Enumerate MPU6050 at `0x68`/`0x69`; confirm no address collision.
2. 5-minute 100 Hz cadence run; count dropped internal frames (expect 0).
3. Confirm both INT lines fire data-ready.
4. Confirm all six PWM outputs LOW at boot; test each motor independently.
5. Verify flyback polarity physically before motor power; verify 5 s cutoff.

### Result
NOT RUN.

## TEST-002 — Sensor static/rotation checks (planned)

### Objective
Per doc 13 §2: 5-min static log ≈ 1 g accel magnitude, gyro bias stability,
axis-sign rotations, same anatomical orientation on both boards, relative
orientation sanity.

### Result
NOT RUN.

## TEST-003 — ZUPT behavior (planned)

### Objective
Per doc 13 §4: foot-flat intervals detected, swing false-ZUPTs rejected,
drift materially reduced, poor sessions flagged (never fabricated).

### Result
NOT RUN.

## TEST-004 — Haptic safety/logic (planned)

### Objective
Per doc 13 §5: no haptic on low confidence, monotonic error→PWM, ON/OFF
hysteresis ordering, correct spatial pair, temporal dual-pole cue for timing
errors, 5 s and 10 s rolling-duty enforcement.

### Result
NOT RUN.

## TEST-005 — Wireless loss + backfill (planned)

### Objective
Per doc 13 §6: 60 s Wi-Fi loss during walking → gait/haptics continue;
reconnect backfills from storage with no duplicate cycle IDs.

### Result
NOT RUN.

## TEST-006 — Storage recovery (planned)

### Objective
Per doc 13 §7: power/reset mid-write, CRC corruption, truncated block →
reboot recovers to last valid block automatically.

### Result
NOT RUN.

## TEST-007 — Dashboard + export (planned)

### Objective
Per doc 13 §8: live charts, event markers, raw-to-error traceability,
CSV/`.mat`/PDF export, segmentation, reference lock.

### Result
NOT RUN.
