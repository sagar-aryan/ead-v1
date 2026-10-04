# Implementation

What exists in the code today. Target design: `docs/architecture.md`.
Milestone plan: `docs/handoff.md`.

## Status by milestone

| Milestone | Scope | Status |
|---|---|---|
| M0 | Baseline fixes and cleanup | Complete except on-body gravity check (TEST-014) |
| M1 | Data-ready acquisition, protocol, Wi-Fi + USB links, backfill ring | Complete |
| M2 | Dashboard foundation: backend, shell, LIVE, RAW, recording, SESSIONS | Complete |
| M3 | Calibration, Mahony orientation, mounting check, datasets | Complete; verified on the leg (TEST-027–029) |
| M4 | Gait events + ZUPT, EVENTS/CYCLES/TRENDS | Complete; 6.39 m on a 6.00 m course (TEST-030), 5.83 m since contacts come from the shank (DEC-022, TEST-059). Trends live inside CYCLES, not as doc 11's seven panels |
| M5 | Reference, error engine, session workflow | Complete in code (TEST-031, TEST-032). Used with patient 67 on 2026-09-18; those references carry the PROB-016 fault |
| M6 | CSV, `.mat`, PDF exports | Complete; checked against synthetic sessions only (TEST-035–037) |
| M7 | On-device flash storage and recovery | Not planned in detail (needs a DEC) |

| BNO086 build | DEC-016 pins, own SH-2 driver, sensor check, motor service test, `ead --check` | Complete on hardware (TEST-045–050), except an accepted motor pulse felt by a person; mount maps measured on the leg (TEST-051); step detection rebuilt on its walks (DEC-022) |

M0–M6 were built and verified on the MPU6500/I²C build. Since schema 5 (2026-10-02)
the product firmware runs on the DEC-016 build instead; the processing chain is
unchanged except for the measured mount maps (TEST-051) and, since DEC-022, step
detection from the shank's swing, set on BNO086 walks.

## Firmware: acquisition and links (M1)

### Objective
Turn the bring-up sketch into a real data path: timestamped 100 Hz acquisition,
a binary protocol over two transports, and recovery of anything lost in transit.

### Design
- **Clock.** The foot IMU's data-ready interrupt is the timebase (doc 07 §2). The
  handler timestamps with the monotonic microsecond clock, counts the interrupt
  (that count is `frame_index`, so a missed one is a visible gap) and wakes the
  acquisition task. The interrupt service is installed with `ESP_INTR_FLAG_IRAM`
  so it keeps firing while flash is busy.
- **Guard delay.** Reads start 1–2 ms after the edge, never on it (PROB-007).
- **Tasks.** Acquisition (core 1, priority 22) → frame queue → processing
  (core 1, priority 20) → message ring → USB and Wi-Fi link tasks (core 0,
  priority 5). Links never block acquisition.
- **Durability.** Every RAW_SAMPLE_BATCH is appended to a 4 MB PSRAM ring
  (~12 minutes) keyed by sequence number. Each link streams from its own cursor,
  and a host can request any stored range (BACKFILL_REQUEST).
- **Transports.** Wi-Fi: ESP-IDF `esp_http_server` WebSocket, sending only when
  the socket reports writable (DEC-010). USB: the protocol messages COBS-framed,
  written directly to the USB Serial/JTAG endpoint one 64-byte packet at a time,
  with Arduino `Serial` unused and core logging compiled out (PROB-006).

### Important files
- `firmware/lib/ead_core/` — portable codec, COBS, CRC-32, message ring, config
  section. Builds for the device and for host tests (`pio test -e native`).
- `firmware/src/acquisition.cpp` — interrupt, self-test, frame assembly, sensor watches.
- `firmware/src/imu.cpp` — register driver, readback verification, bus recovery.
- `firmware/src/link.cpp` — per-link protocol endpoint (replies, status, streaming, backfill).
- `firmware/src/link_usb.cpp`, `src/link_wifi.cpp` — the two transports.
- `firmware/src/telemetry.cpp`, `src/device.cpp` — ring and shared device state.
- `docs/protocol.md` — the wire specification all three implementations follow.

### Edge cases
- Both IMUs run on independent clocks (0.14 % apart, TEST-015), so the shank
  occasionally repeats a sample; the frame is flagged, not hidden.
- An IMU that browns out is detected within a second (`PWR_MGMT_1` readback) and
  reconfigured; the count is reported.
- A reset during an I²C transaction is cleared by pulsing the bus before the
  driver starts (TEST-020).
- A host that stops reading USB never stalls the device: a half-written packet is
  abandoned after 3 s and the fragment fails CRC at the next host.

### Limitations
No calibration, orientation, gait analysis or session control yet: quaternions in
the raw frame are identity, and SESSION_* commands answer NotSupported. Those
arrive with M3–M5 as protocol schema 2.

### Verification
TEST-015 to TEST-021.

## Dashboard backend (M2)

### Objective
Receive, verify and store the device's data, and serve the UI.

### Design
- `protocol/` mirrors the wire format independently of the firmware; both are
  checked against `protocol/vectors/`.
- `link/` has one transport per link behind a common event channel. USB runs on a
  blocking thread and opens the port without touching DTR/RTS (PROB-005).
- `device.rs` performs the handshake, sends keepalives, fetches and hash-verifies
  the device configuration, tracks durable sequence numbers and requests backfill
  for anything missing.
- `store/` writes raw frames through a single writer thread, batched into
  transactions every 250 ms, with `INSERT OR IGNORE` so a backfilled frame cannot
  duplicate a live one. WAL mode keeps UI queries off the writer's path.
- `live.rs` aggregates the 100 Hz stream into 20 Hz updates for the UI (doc 11 §3).

### Important files
- `dashboard/src-tauri/src/{protocol,link,device,store,live,app}.rs`
- `dashboard/src/` — React UI (see below)

### Raw view
Stored frames are read back through one decimated query: frames are grouped into
buckets and each bucket reports its minimum and maximum, so a single-frame impact
survives decimation where sampling every Nth frame would drop it. The conversion
from ADC counts to anatomical physical units happens in Rust, using the
configuration stored with that session, so the view holds no knowledge of mount
maps and a recording made under different settings still reads correctly.

Summary tables were considered and deliberately not built: measurement showed a
full-session query over an hour takes 309 ms and every zoomed view under 90 ms
(TEST-024).

All selected signals come back from one request (`raw_window` takes a list of
signal groups). They read the same rows, so the store aggregates every group's
columns in a single `GROUP BY` pass and returns one shared time base. That halved
the whole-session load for four signals (542 ms as four queries, 291 ms as one,
TEST-026) and guarantees the stacked charts have identical bucket boundaries.

### Raw view navigation
Focus plus context. An overview strip draws the whole session (first selected
signal, 700 points, loaded once per session/signal) with the visible window
marked; below it every selected signal is stacked with a cursor synced across
charts (uPlot `cursor.sync`). Each chart prints its own time axis rather than
sharing one under the bottom chart: a reader looking at the third signal down
should not have to track a tick label across the whole stack. The reader moves by dragging
a range on any chart, ◀/▶ or arrow keys (pan half a window), In/Out or +/−
(halve/double the width, centred), or Whole session.

The window arithmetic lives in `dashboard/src/timeline.ts` and is tested
(`npm test`, Node's built-in runner executing TypeScript directly, no framework;
`@types/node` added as a types-only dev dependency so `tsc` checks the tests).
Windows move by frame index, never derived from time, because the device runs at
100.145 Hz rather than 100 Hz (TEST-018). A window covering the whole session is
represented as `null` so "zoomed" and "whole session" cannot disagree.

### Limitations
No export and no gait views (M4–M6).

### Verification
23 Rust tests (`cargo test`), plus a hardware test run with the device attached
(`cargo test -- --ignored`).

## Dashboard UI (M2)

### Design
An instrument panel rather than a web dashboard: a state bar that reads across a
bench, flat panels separated by hairlines, measurements in tabular mono numerals.
Foot and shank keep one colour identity everywhere (categorical slots 1 and 2,
validated for colour-vision deficiency). Type is IBM Plex Sans and Mono, bundled
locally so the app works offline.

Only three views exist — Live, Sessions, Device — because those are the
measurements that exist. A value that is not available says so rather than
showing a zero.

### Important files
- `dashboard/src/styles.css` — design tokens and the panel/state-bar structure
- `dashboard/src/useDevice.ts` — device state; live samples land in ring buffers
  outside React, so the 20 Hz stream never re-renders the tree
- `dashboard/src/components/Strip.tsx` — uPlot signal strip fed from a ring buffer
- `dashboard/src/views/{Live,Sessions,Device}.tsx`

## Sensor mount maps

### Objective
Express both IMUs in the doc 04 anatomical frame.

### Design
`anat = M · chip`, one signed-permutation matrix per sensor, applied identically
to accel and gyro. Both frames are right-handed, so `M` must be a proper rotation
(DEC-009, PROB-002).

### Implementation
`firmware/include/config_v1.h`:
- `EadMountMap`, `kEadFootMount` (identity), `kEadShankMount`.
- `eadMountDet()` and `eadMountIsSignedPermutation()` are `constexpr` and
  enforced with `static_assert`, so an invalid map is a build error.
- `eadMountApply()` writes `int32_t` so negating a raw −32768 cannot overflow.

### Verification
TEST-009 (negative and positive compile tests). On-body check TEST-014 pending.

## IMU initialisation (bring-up firmware)

### Objective
Configure both sensors to the doc 00 values and prove the configuration took effect.

### Implementation
`firmware/src/main.cpp`, `mpuInit()`:
- Accepts WHO_AM_I 0x68 (MPU6050) and 0x70 (MPU6500).
- Writes PWR_MGMT_1, SMPLRT_DIV, CONFIG, GYRO_CONFIG, ACCEL_CONFIG, INT_PIN_CFG,
  INT_ENABLE, and ACCEL_CONFIG2 when the part is an MPU6500 (PROB-003).
- Reads every register back; any mismatch fails init and prints the register,
  value read and value expected.

### Boot order
`setup()` drives the six motor GPIOs LOW before starting USB serial, then I²C,
scan and init. There is no PWM or haptic code (DEC-006).

### Edge cases
ACCEL_CONFIG2 is compared through a 0x0F mask; the upper bits are reserved.

### Verification
TEST-008, TEST-010, TEST-011, and again on every boot since: a configuration
mismatch raises a fault bit that the dashboard displays.

## Dashboard shell

### Implementation
- `tauri.conf.json`: strict CSP (`default-src 'self'`, IPC allowed); bundle icons
  generated from `dashboard/app-icon.svg` with `tauri icon`, keeping only desktop
  sizes.
- `capabilities/default.json` grants `core:default` to the `main` window.

### Verification
TEST-012, TEST-013, and the application running against hardware (TEST-023).

## Mounting check (M3)

### Objective
Prove each IMU is mounted the way the firmware's mount map assumes, without
asking the operator to read numbers off a screen. This is how PROB-002 (the
shank map) gets settled with evidence.

### Design
Three guided moves, judged against the anatomical frame (X forward, Y medial,
Z up, right leg):

| Step | Expected |
|---|---|
| Stand still, 3 s | mean a ≈ (0, 0, +1) g on both sensors |
| Raise the toes, heel down, 4 s | foot peak angular rate on Y, negative |
| Seated knee extension, 4 s | shank peak angular rate on Y, negative |

Both moves rotate the segment about the medial axis in the negative sense, so
both expect a negative Y rate. A move that is too gentle (< 30 °/s) or not
clearly about one axis (leading axis less than 1.5× the next) fails rather than
being guessed at, and every verdict shows the measured vector so a failure says
what the mounting actually is.

### Implementation
`dashboard/src/mounting.ts` holds the verdicts and thresholds and is tested
(`dashboard/src/mounting.test.ts`, 11 tests, including gravity on the wrong axis
and a reversed sign). `dashboard/src/views/MountingCheck.tsx` collects samples
and renders; it appears in the Device view and needs no firmware support.

Accelerations come from the live tick state (5 Hz, enough for a mean). Angular
rates come from the 20 Hz rings in `useDevice`, because the tick state is
throttled to 5 Hz for the readouts and would miss the peak of a short movement.

### Limitations
Judged on the single strongest sample rather than an integrated angle, so it
confirms axis and sign, not range of motion.

### Verification
11 unit tests. Not yet run on a worn device (TEST-027).

## Static calibration (M3)

### Objective
Remove the two constant errors a strapped-on IMU has: the gyroscope's resting
offset, which integrates into drift, and the mounting tilt, which would otherwise
appear as a permanently flexed joint.

### Design
Five seconds of stillness (2–30 s accepted). Per sensor the device computes the
mean angular rate (the bias, kept in the chip frame because it belongs to the
part, not to the anatomy), the mean acceleration direction (gravity, in the
anatomical frame), and the quaternion taking that direction to +Z. It also
reports the mean |a| and the largest per-axis standard deviation, which are what
make a bad window visible: a record is rejected when the sensor moved, when |a|
is not 1 g, when gravity is not upward, or when too few frames were collected.

### Implementation
- `firmware/lib/ead_core/src/ead/calibration.{h,cpp}` — pure, float32, no
  Arduino headers, so device and host replay agree.
- `firmware/src/calibration_service.{h,cpp}` — window state, fed from the
  processing task with the frames already being acquired.
- Protocol schema 2: SESSION_START (kind, duration) and a 128-byte record in
  SESSION_STOP, with calibration state and progress in STATUS.
- `dashboard/src/views/Calibration.tsx` — start, progress, and the numbers.

### Edge cases
- A completion that finishes while no host is listening is discarded when the
  next host says HELLO, so a record cannot be mistaken for the answer to a later
  request.
- A host that stops sending keepalives stops the stream, and with it the record;
  the 1 Hz keepalive is part of the protocol, not an optimisation.

### Limitations
The record lives in RAM and is lost on reset (no flash storage until M7). Nothing
consumes it yet — orientation is the next step.

### Verification
Seven native unit tests, golden-vector tests in firmware and Rust, and TEST-028
on hardware.

## Gait storage and the Cycles view (M4)

### Objective
Keep what the device measured per cycle, and show it in the form a reader needs:
a table to check individual cycles and a trend to see the session.

### Design
Schema 3 adds two tables. `cycles` holds one row per completed gait cycle and
`events` one row per gait event, both keyed by session and frame so a backfilled
batch replaces rather than duplicates. They are derived values, kept separate
from `raw_frames`: a corrected detector produces different cycles from the same
recording, and the recording is the thing that must not change.

The Cycles view reports distance and speed only for cycles whose zero-velocity
quality reaches 0.15, and shows the rest as low-confidence rather than correcting
them (doc 05 §8). Cycles rejected by the temporal guards are greyed rather than
hidden, because a missed or doubled contact is exactly what a reader needs to
see. Spread is reported as median and MAD, the robust pair the spec uses.

### Important files
- `dashboard/src-tauri/src/store/schema.rs` — tables and the 2 → 3 migration
- `dashboard/src-tauri/src/store/mod.rs` — `record_gait`, `cycles`, `events`
- `dashboard/src/views/Cycles.tsx`

### Verification
Three store tests: a round trip including a repeated (backfilled) batch, the
rule that gait is only stored while recording, and a migration from schema 2
that keeps existing frames.

## Gait event detection (DEC-022)

### Objective
One initial contact and one toe-off per right-leg stride, timed well enough for
cycle features and for the zero-velocity integrator's cycle boundaries.

### Design
The shank's forward swing rate, `-shankGyroDps[1]` in the right shank's anatomical
frame, through the 20 Hz event-path filter. A stride is one swing of at least
`kMidSwingDps` (100 °/s). Toe-off is the deepest backward rate in the run below zero
that led into the swing; the contact is the strongest foot impact from the
swing's descent (below half its peak) to `kContactSearchS` (0.15 s) after the rate
crosses zero.

### Implementation
`GaitEngine::update` in `firmware/lib/ead_core/src/ead/gait.cpp`: the stance states
track the toe-off candidate and start `Swing` at the threshold, emitting `ToeOff`
with the candidate's time; `Swing` tracks the peak, the descent, the zero crossing
and the best impact, and calls `claimContact` with the impact's time. ZUPT,
integration and cycle features are unchanged.

### Important Files
- `firmware/lib/ead_core/src/ead/gait.{h,cpp}`
- `firmware/test/test_gait/test_main.cpp`
- `tools/replay/main.cpp` (`--mid-swing`, `--contact-search`), `tools/replay/walks.py`

### Edge Cases
- A swing that ends in stillness before the search closes: the contact is the best
  impact so far, then `Stance` (PROB-023).
- A swing with no downward zero crossing within `kMaxSwingS` (1 s): dropped.
- The first stride from standing has no contact before it, so it opens no cycle.
- Events are emitted up to 0.15 s after the time they carry.

### Limitations
Right leg only (the sign). Thresholds from one healthy wearer (TEST-059). A swing
below 100 °/s, such as a very short first step or a shuffle, is not a stride.

### Verification
TEST-059: 3 counting errors over 176 counted landings (21 before); 13 gait tests,
of which 3 fail on the previous engine.

## Host-side segmentation (M5)

### Objective
Doc 12 §5: the researcher enters `max_valid_cycles_per_segment` and
`max_errors_per_segment`, and a segment closes when either is reached first. No
default may be invented for either.

### Design
The rule is applied to the stored cycle stream rather than on the device
(DEC-014). Limits live on the session row; `segments` holds one row per segment
with its counts and why it closed; each cycle carries the index of the segment
that was open when it arrived — so the cycle that trips a limit belongs to the
segment it closed, which is the reading that makes the counts add up.

An **error**, for the purpose of the limit, is a scored cycle whose primary class
is not NONE and whose confidence reaches 0.50. Doc 12 leaves the word undefined;
0.50 is the level below which doc 06 §7 says the classification should not even
be shown, and ending a segment on a finding the system will not display would be
worse than not counting it.

### Implementation
`roll_segment` in `dashboard/src-tauri/src/store/mod.rs`, called inside the
writer's transaction for each cycle. It reads the session's limits, finds the
open segment, updates its counts, closes it when a limit is met and opens the
next. It reads the row back each time rather than caching a counter: cycles
arrive at roughly 1 Hz, so four small statements cost nothing, and a backfilled
batch cannot double-count against a stale in-memory total.

A session with no limits — every recording, capture and check — short-circuits to
segment 0 without touching the table.

### Edge cases
- Nothing open (the session was stopped between the cycle arriving and the
  commit): the cycle is attributed to the last segment rather than reopening one.
- Stopping a session closes the open segment with `session_stopped`.
- An invalid cycle counts toward neither limit.

### Important files
- `dashboard/src-tauri/src/store/mod.rs` — `roll_segment`, `counts_as_error`
- `dashboard/src-tauri/src/store/schema.rs` — schema 5
- `dashboard/src/views/Sessions.tsx` — the doc 12 §4 gate

### Verification
TEST-032.

## Unilateral cycle symmetry proxy (M6)

### Objective
Doc 05 §10: `proxy = 1 - normalized_difference(current, previous)` between
consecutive valid right-leg cycles, "using the same robust feature normalization
used by the error engine". Required by `gait.csv` and by the doc 11 LIVE panel.

### Design
The error engine's normalization is `d = clamp(z / 3)` where `z` is a difference
in units of the reference's spread for that feature, weighted by doc 06 §3. The
proxy is the same, with the previous cycle in place of the reference's median:

```
proxy = 1 − Σ w_f · clamp(|x_f(i) − x_f(i−1)| / spread_f / 3) / Σ w_f
```

It therefore **needs a reference profile**. A session with none has no spreads to
normalize by, and the proxy is null — not zero, not one. Cycle distance is
dropped from both sides when either cycle had no zero-velocity window, exactly as
the error engine drops it.

Computed on the host in `Store::cycles`, for the same reason as segmentation: it
is a pure function of the stored cycle stream, so it reproduces exactly on replay
and costs no protocol change.

### Limitations
This is cycle repeatability, never a left-versus-right symmetry claim — only the
right leg is instrumented. Doc 05 §10 insists on the name
`unilateral_cycle_symmetry_proxy`, and it is spelled that way in the CSV, the
`.mat` and the report.

### Important files
- `dashboard/src-tauri/src/store/mod.rs` — `symmetry_proxy`, `fill_symmetry_proxy`

### Verification
TEST-035 checks that the first valid cycle's cell is empty and the second is not.
Never yet computed from a real reference profile.

## Export package (M6)

### Objective
Doc 10: `raw.csv`, `gait.csv`, `events.csv`, `haptics.csv`, `metadata.json`,
`session.mat` and the PDF report, all derived from the store so a session
recorded last month exports the same bytes today.

### Design
Three rules run through all of it.

1. **Not measured is empty, never zero.** A CSV cell is blank and a `.mat` value
   is NaN wherever the value does not exist: an unscored cycle's error columns, a
   symmetry proxy with no previous cycle, a distance with no zero-velocity
   window. A column of zeroes would read as perfect agreement.
2. **Did not happen is stated, not omitted.** `haptics.csv` exists with its
   header alone; `session.mat` has a `haptics` variable carrying the reason it is
   empty; the report draws the haptic-response panel doc 10 §8 asks for, empty
   and labelled. A missing file or panel looks like an oversight.
3. **The raw integers survive.** Doc 10 §7 requires it of the `.mat`, so
   `raw.csv` carries the same stored ADC counts rather than converting to
   physical units, and `metadata.json` carries the scale factors and both mount
   maps so the conversion is one multiplication away.

`events.csv` is where doc 10 asks for more than the device produces: ten event
types against the device's five. Cycle bounds come from the cycles, ERROR_ACTIVE
and ERROR_RESOLVED from the class transitions of displayable cycles, FAULT from
the recorded status changes. SERVICE_TEST never appears, and `metadata.json` says
why. ZUPT_END is written although doc 10 §4 does not list it: the device reports
windows, not instants.

The `.mat` writer is hand-rolled (DEC-003). The rules that are easy to get wrong
are listed at the top of `mat.rs`; the one that actually bit was the char-array
type code, `miUTF16` = 17, not 18.

The PDF is composed on a fixed grid with krilla — every element at a measured
coordinate on A4, no layout engine — with the IBM Plex faces the UI uses embedded
as TTF subsets.

### Important files
- `dashboard/src-tauri/src/export/mod.rs` — orchestration and `metadata.json`
- `dashboard/src-tauri/src/export/csv.rs`
- `dashboard/src-tauri/src/export/mat.rs`
- `dashboard/src-tauri/src/export/pdf.rs`
- `dashboard/src/views/Export.tsx`
- `tools/check_mat.py`, `tools/check_pdf.py`

### Limitations
The PDF's paragraph wrapping estimates line width from a mean advance of 0.52 em
rather than shaping each candidate line. The report's prose is all lowercase
Latin and nothing is set flush right, so this is invisible — but a long
unbroken token, such as a very long patient identifier, will overrun.

### Verification
TEST-035, TEST-036, TEST-037. Every one of them ran against synthetic cycles;
none has seen a real session.

## BNO086 acquisition, sensor check and motor service test (schema 5)

### Objective
Run the product firmware on the DEC-016 build (DEC-017), let a researcher check every
sensor wire and each motor from the dashboard (`ead --check`, DEC-018).

### Design
- **Driver** (`src/bno086.cpp`): both sensors on one SPI bus; reads exactly as the
  TEST-043 bench driver did, writes with the WAKE handshake. The SH-2 byte work is
  portable (`ead/sh2.cpp`) and tested on the host.
- **Check** (`bno086::resetAndCheck`): RST held low (INT must idle high), released (each
  INT must assert: 3V3, GND, RST, INT), drained through each CS (valid packets: SCK,
  MISO, CS; INT released: CS and INT on the same board), WAKE pulled (each idle INT must
  answer), product ID request (MOSI), then the two reports, each command confirmed before
  the next. Results go to faults and to SERVICE_TEST.
- **Frames** (`src/acquisition.cpp`): INT edges timestamped in IRAM; packets read until
  both INTs release; each sample timed from the SH-2 base timestamp and its delay; one
  frame per foot gyroscope sample; the index follows the gyroscope sequence and stays
  monotonic across a check or a sensor reset (the elapsed time sets the jump). Since
  schema 6 (DEC-021, TEST-055): 200 Hz; up to four frames wait for the samples after
  their time; each accelerometer is interpolated to the frame time (`ead::interpolateAccel`)
  and the shank's gyroscope and both rotation vectors are the samples nearest it; a frame
  is forced out after 15 ms, marked held. Every accelerometer sample also goes to the
  processing task, which sends them as RAW_ACCEL_BATCH alongside each raw batch.
- **Orientation** (`src/orientation.cpp`, `ead/feed.cpp`, schema 6): the segment
  orientation is the chip's game rotation vector followed by the inverse of mount map then
  alignment (`segmentOrientation`); no estimator runs on the ESP32. Mahony stays in
  `ead_core` for the replay of older recordings.
- **Motors** (`src/motors.cpp`, `ead/motor_guard.cpp`): LEDC 200 Hz, 8 bit on the enabled
  pins; `MotorGuard` applies the contract's limits; an `esp_timer` ends each pulse.
- **Dashboard**: `SERVICE_TEST` codec (`protocol/mod.rs`), requests and replies counted in
  `device.rs`, commands in `app.rs` (refused while recording, doc 11 §6), log in store
  schema 7 (`service_tests`), `views/Check.tsx`, `--check` read in `main.rs`.

### Important Files
- `firmware/src/bno086.{h,cpp}`, `firmware/src/acquisition.{h,cpp}`, `firmware/src/motors.{h,cpp}`
- `firmware/lib/ead_core/src/ead/sh2.{h,cpp}`, `firmware/lib/ead_core/src/ead/motor_guard.{h,cpp}`
- `firmware/src/link.cpp` (SERVICE_TEST), `firmware/include/config_v1.h`
- `dashboard/src-tauri/src/{protocol/mod.rs,protocol/config.rs,device.rs,app.rs,store/}`
- `dashboard/src/views/Check.tsx`, `tools/eadprobe.py` (`check`, `pulse`), `tools/replay/main.cpp`

### Edge cases
- A sensor that resets itself sends "reset complete": its reports are re-enabled and
  counted in `imu_reinits`.
- WAKE is shared: a sensor whose INT was already asserted is not credited with WAKE.
- A refused pulse says why in words (ERROR detail), shown in the Motors panel.
- A motor whose wiring is in doubt is switched off in `EAD_MOTOR_ENABLED_MASK`: never
  driven, not even LOW. Motor 3 was, until PROB-020 was measured; all six are on now.

### Limitations
- Mount maps were measured on the leg (TEST-051); step detection was set on BNO086
  walks of one wearer (DEC-022).
- The device cannot sense a motor turning: "felt" is the operator's answer.
- Shank repeats 1.9 % (phase crossings) and frame-period sd 200 µs (TEST-046).
- SPI stays at 1 MHz until a soak test on the harness.

### Verification
TEST-045 (host), TEST-046 (acquisition), TEST-047 (check), TEST-048 (backend on device),
TEST-049 (`ead --check`), TEST-050 (replay, migration).

## Schema 6 on the dashboard (DEC-021)

### Objective
Read, store and export the 200 Hz frames with rotation vectors and the native
accelerometer stream.

### Implementation
- `protocol/mod.rs`: 70-byte frames (`rv_foot`, `rv_shank`; a 54-byte frame is
  refused); RAW_ACCEL_BATCH (0x13) as `AccelSample`; status names to bit 11.
- `device.rs`: accelerometer batches, and since PROB-025 event and step batches,
  go through `on_durable`, so they count in gap detection and backfill.
- Store schema 8 (`MIGRATE_7_TO_8`): `raw_frames` gains `rfw, rfx, rfy, rfz, rsw,
  rsx, rsy, rsz` (rotation vectors, Q14, NULL before schema 6); new table
  `raw_accel(session_id, sensor, timestamp_us, sequence, ax, ay, az)`, key
  `(session_id, timestamp_us, sensor, sequence)`, INSERT OR IGNORE like frames,
  stored only while a session records.
- Export: `raw.csv` gains `rv_real, rv_i, rv_j, rv_k` (one row per sensor, empty for
  older frames); new `accel_native.csv` (`timestamp_us, sensor, sequence, ax, ay, az,
  ax_g, ay_g, az_g`, chip frame); `session.mat` gains `raw.rotation_vector` (NaN for older
  frames) and an `accel` struct; `metadata.json` notes both.
- 100 Hz assumptions removed: the calibration view's expected frame count uses the
  configured rate; the hardware test expects 200 Hz. Live decimation was already time
  based.

### Verification
cargo test 69 passed (then 70 with PROB-025's test), clippy clean, npm test 28, build;
`tools/check_mat.py` on an exported sample: 0 failures. Not yet run against the device.

## Session names (store schema 9)

### Objective
Let the user name a recording ("normal pace 1") instead of telling sessions apart by
their generated id (user request, 2026-10-04).

### Implementation
- `sessions.label TEXT`, NULL until given (`MIGRATE_8_TO_9`, also in `CREATE_SCHEMA`).
- `Store::set_session_label` (trimmed; blank clears it; unknown session refused).
- Commands: `start_recording(patient_id, label)` names the new session;
  `rename_session(session_id, label)` names or renames any session, any kind.
- UI: an optional name field beside Start recording; a Name column with Rename in the
  sessions table; every session picker shows `name · id` (`sessionTitle` in `api.ts`).
- Export: `metadata.json` `session.label`; the PDF's Session row shows the name with
  the id.

### Verification
Store test `a_session_can_be_named_and_renamed`; the v2 and v7 upgrade tests also undo
schema 9 and pass; cargo test, clippy, npm test and build. The UI was not looked at
on screen (no screenshot tool in this Wayland session).

## Haptic feedback (DEC-023, protocol schema 7, store schema 10)

### Objective
Doc 06's spatial vibrotactile cue on the shank during walking: one cue per scored
step, after it lands, with a master switch on the dashboard.

### Design
Device-side, closed loop. Each cycle an EVALUATION session scores goes to the haptic
engine, which returns a cue (ON or UPDATE: one or two motors, duties, 250 ms) or an
episode end (OFF with a reason). The motor guard accepts or refuses the cue against
the contract's limits. The switch, the episode bit and every record travel to the
dashboard (CONFIG_SET, STATUS `haptics`, HAPTIC_BATCH).

### Implementation
- `lib/ead_core/src/ead/haptics.{h,cpp}`: `HapticEngine::onCycle`, `stop`; the class
  cue table (doc 06 §8), two-nearest-motor placement (§9), intensity (§10).
- `lib/ead_core/src/ead/motor_guard.{h,cpp}`: `requestCue` (two motors together, each
  held to every limit; history 256 entries).
- `src/feedback.{h,cpp}`: the switch (atomic, off at boot), the engine on the
  processing task, the log, `publish()`. `src/motors.cpp`: `cue()`, `stopAll()`.
- `src/gait_service.cpp`: calls `feedback::onCycle` after scoring, `fault` on a read
  failure, a frame gap or no orientation, `poll` every frame, `publish` per batch.
- `src/link.cpp`: CONFIG_SET. `src/device.cpp`: STATUS `haptics`, capability bit 0.
- Dashboard: `protocol::{haptic_switch_request, parse_haptic_batch}`,
  `Device::set_haptic_feedback`, the `haptics` table and `Store::{record_haptics,
  haptics}`, `haptics.csv` rows, the MAT `haptics` struct, the PDF panel, the
  `set_haptic_feedback` command and the switch in `StateBar.tsx`.
- `tools/eadprobe.py`: `haptics on|off`, `score --evaluate --haptics`.

### Edge Cases
- A score with no class (distance or shank dynamics only) has no direction: no
  episode starts, a running one ends (`no_direction`).
- A refused cue is logged with duty 0 and reason `refused`; the episode goes on.
- Switching off stops the motors from the link task at once; the OFF record follows on
  the next frame.

### Limitations
Not flashed or felt. The 250 ms cue, TIMING's alternation and dropping a second motor
under 51 are choices doc 06 leaves open. Right leg only.

### Verification
TEST-060.

## Dashboard: doc 11 gaps closed (2026-10-04)

### Objective
The doc 11 items that needed no new device data: LIVE cycle metrics, the seven TRENDS
panels, CYCLES → RAW, the haptic column and lane.

### Implementation
- LIVE (`views/Live.tsx` `LastCycle`): the latest cycle the device sent, kept in the
  device tracker (`State::last_cycle`, a backfilled older batch never replaces it):
  cycle time, cadence, speed and stride (marked "low ZUPT" under 0.15), stance and swing,
  ZUPT quality, error score, confidence and class (only when scored), vibration state.
- TRENDS (`views/Cycles.tsx`): `PRIMARY_TRENDS` is doc 11's seven in its order
  (cadence, symmetry proxy, error score, stance ratio, peak dorsiflexion, cue duty,
  ZUPT quality); the rest under "Other measurements". Error and haptic panels only in
  a scored session; a missing value is a gap, never zero.
- CYCLES: a Vibration column (motors and duty of the cycle's cue, "refused" for duty 0);
  clicking a row opens RAW on that cycle with a quarter-cycle either side
  (`RawFocus`, held in `App.tsx`, cleared by plain navigation).
- EVENTS: a Vibration lane, cues as bars of their length, episode ends as ticks.
- Backend: `haptics` command; `Snapshot::last_cycle`.

### Limitations
Not seen on screen (no screenshot tool in this environment). LIVE shows no symmetry
proxy (it needs the session's reference spreads; it is in CYCLES and TRENDS). RAW has
no cycle or error filter beyond the click-through; no separate HAPTICS page (table in
CYCLES, timeline in EVENTS, file in the export); PAUSE/RESUME remains. The "roll the
sole inward" mounting step is not built: with Y and Z checked and every mount map a
proper rotation, the foot's X axis is already determined.

### Verification
`npm test` (28), `npm run build`, `cargo test` (the device test checks `last_cycle`),
clippy.
