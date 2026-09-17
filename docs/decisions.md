# Engineering Decisions

## DEC-001 — PlatformIO + Arduino on Linux for firmware builds

**Date:** 2026-09-16

**Status:** Accepted

### Context
V1 firmware targets Seeed XIAO ESP32-S3 and must be reproducibly built,
flashed, and CI-checked on the team's Linux workstations. Doc 15 fixes the
hardware; the build system choice was left to the implementation.

### Options Considered
1. Arduino IDE (GUI sketch flow).
2. PlatformIO + Arduino framework on Linux (selected).
3. ESP-IDF native (no Arduino layer).

### Decision
Use PlatformIO with the Arduino framework on Linux (`espressif32` platform,
`seeed_xiao_esp32s3` board), with `platformio.ini` as the build definition.

### Reason
Reproducible declarative config (board, framework, libs, filesystem,
build flags) checked into git; CLI builds suitable for agents/CI; Arduino
ecosystem gives MPU6050/WebSocket libraries without ESP-IDF rewrite cost;
Linux matches the Tauri/Rust desktop toolchain on the same host.

### Trade-offs
Gained: versioned deps, `pio run/test` automation, LittleFS + USB-CDC flags
in one file. Sacrificed: fine-grained ESP-IDF control and minimal binary
size; accepted because V1 schedule favors library reuse.

### Consequences
All firmware builds go through `firmware/platformio.ini`. Do not add a
parallel Arduino-IDE sketch flow. Future ESP-IDF migration would require a
new DEC entry.

## DEC-002 — USB-C CDC for firmware debug console

**Date:** 2026-09-16

**Status:** Superseded by DEC-005 (2026-09-17). The USB port keeps its role for
flashing, but from milestone M1 it carries the binary protocol instead of a
text console. The board definition already sets both USB flags, so they were
removed from `platformio.ini`.

### Context
Developers need a log/flash/debug channel on the XIAO ESP32-S3 without extra
UART hardware during bring-up (doc 13 §1).

### Options Considered
1. Native USB-C CDC serial (selected).
2. External USB-UART bridge on GPIO TX/RX.
3. Wireless-only logging (WebSocket).

### Decision
Enable `ARDUINO_USB_MODE=1` + `ARDUINO_USB_CDC_ON_BOOT=1`, 115200 monitor,
and use the XIAO USB-C port as the debug console.

### Reason
Zero extra hardware; works before Wi-Fi is up; compatible with `pio device
monitor`; wireless logging is unavailable during network-outage tests.

### Trade-offs
Gained: simplest bring-up path. Sacrificed: USB stack occupies native USB;
accepted (V1 has no TinyUSB composite-device requirement).

### Consequences
Firmware must keep CDC init early in BOOT/SELF_TEST; do not repurpose USB
pins for motors/sensors. Log output must not block the 100 Hz loop.

## DEC-003 — Rust-native MATLAB Level-5 `.mat` export in dashboard backend

**Date:** 2026-09-16

**Status:** Accepted

### Context
Doc 14 Phase 10 requires raw CSV, gait/event/haptic CSVs, metadata JSON,
MATLAB Level-5 `.mat`, and PDF report. The Tauri backend is Rust; the export
must be traceable to raw timestamps without requiring MATLAB installed.

### Options Considered
1. Rust-native Level-5 `.mat` writer in Tauri backend (selected).
2. Call out to Python/scipy (`savemat`) at export time.
3. Ship CSV-only and document MATLAB `csvread` workflow.

### Decision
Implement the `.mat` exporter natively in Rust in the dashboard backend,
writing MATLAB Level-5 format directly from the stored raw + derived tables.

### Reason
No MATLAB/Python runtime dependency on researcher machines; single Tauri
binary stays self-contained; deterministic output simplifies replay/export
tests (doc 13 §8); raw-to-`.mat` traceability stays inside the Rust store.

### Trade-offs
Gained: zero-dependency export, testable. Sacrificed: must implement/verify
Level-5 framing ourselves (risk of reader incompatibility); mitigated by
validating output against MATLAB/Octave/scipy readers in Phase 11.

### Consequences
`.mat` schema (variable names, dtypes, time bases) must be versioned and
documented; export tests must open the file in an independent reader.

## DEC-004 — Session segment limits are researcher-entered in dashboard UI

**Date:** 2026-09-16

**Status:** Accepted; the enforcement half superseded by DEC-014 on 2026-09-18.
The limits are still researcher-entered and still refuse a default, but they are
counted on the host rather than sent to the device.

### Context
Doc 15 fixes segmentation: "Researcher enters cycle/error limits; whichever
comes first." Firmware must not invent a step/error limit.

### Options Considered
1. Researcher enters cycle + error limits in dashboard UI (selected).
2. Firmware-internal default limits.
3. Fixed compile-time constants shared by firmware/dashboard.

### Decision
Segment limits are required UI inputs in the dashboard session-setup flow and
are sent to the device as session configuration; firmware enforces the
received limits only.

### Reason
Enforces the doc 15 contract (no invented limits); keeps protocol explicit
and auditable; allows per-patient/per-session values and reference locking.

### Trade-offs
Gained: spec compliance, flexibility. Sacrificed: session cannot start
without UI input; accepted because V1 has no standalone-device workflow.

### Consequences
Dashboard must validate/gate session start on limits; protocol must carry
limits + reference version; firmware defaults to no-RUN without them.

## DEC-005 — USB CDC carries the doc-08 binary protocol as a bench fallback

**Date:** 2026-09-17

**Status:** Accepted (user-selected)

### Context
Doc 08 defines one link: the device's Wi-Fi access point. The development laptop
has only Wi-Fi, so joining the device AP drops its internet connection and the
agent session with it. Hardware-in-the-loop testing needs a link that leaves the
laptop online.

### Options Considered
1. Wi-Fi AP only, exactly as doc 08.
2. Wi-Fi AP primary, plus the identical binary messages over USB CDC (selected).
3. Device joins an existing network (station mode).

### Decision
Wi-Fi AP + WebSocket stays the primary link. The same doc-08 messages also run
over the USB port, framed as `00 | COBS(message ‖ CRC32-LE) | 00`. The dashboard
connects over one link at a time.

### Reason
One message codec and one test suite serve both links. USB tests can run without
losing connectivity, and the Wi-Fi path stays fully spec-compliant.

### Trade-offs
Gained: bench testing on real hardware while online. Sacrificed: the human-readable
serial console (diagnostics move into the protocol and dashboard), plus a framing
layer to maintain. Deviates from doc 15 "Wireless: Wi-Fi AP + WebSocket" as the
only link.

### Consequences
USB writes must never block acquisition when the host is absent or not reading.
The dashboard needs a link selector. Supersedes DEC-002.

## DEC-006 — No haptic code until ERM drivers are fitted

**Date:** 2026-09-17

**Status:** Accepted (user-selected)

### Context
Docs 06 and 14 (Phase 6) define a haptic engine, but no MOSFET driver channels
are fitted, and the user asked for no code targeting them.

### Options Considered
1. No haptic code; firmware and dashboard report "not fitted" (selected).
2. Compute haptic decisions and log them without actuating.
3. Remove haptic areas from the dashboard.

### Decision
Option 1. The six motor GPIOs are driven LOW as the first action at boot (doc 03
§4). No PWM, motor map or haptic controller exists. From M1, HELLO reports
`haptics_fitted = 0`, the HAPTICS view and "Haptic response" trend show a
not-fitted state, and `haptics.csv` exports a header row only.

### Reason
User direction. Haptic safety logic that cannot be exercised on hardware would
ship unverified.

### Trade-offs
The doc 13 §5 haptic tests and doc 14 Phase 6 stay open. The error engine still
computes scores and classes for analysis.

### Consequences
The doc 14 release gate "haptics are safe by construction" is not met. Fitting
drivers needs its own milestone, including the doc 13 §5 tests and the PROB-004
boot-pin check.

## DEC-007 — Raw samples are chip-frame native ADC counts

**Date:** 2026-09-17

**Status:** Accepted

### Context
Doc 04 §7 calls stored raw data "unmodified calibrated measurements"; doc 09 §6
says native ADC counts with scaling in metadata. Doc 09 also declares the accel
and gyro fields `u16` although the sensor values are signed.

### Options Considered
1. Chip-frame native counts; mount maps, scales and calibration in metadata (selected).
2. Counts remapped to the anatomical frame.
3. Calibrated physical units (float).

### Decision
Option 1, following doc 09. The fields hold the int16 two's-complement bit
pattern and are interpreted as signed everywhere, including exports.

### Reason
It is the only lossless record. Every processed value can be recomputed, and a
corrected mount map or calibration never invalidates earlier recordings
(PROB-002 is exactly that case).

### Trade-offs
Consumers must apply mount map, scale and calibration; `raw.csv` is not directly
in physical units.

### Consequences
`metadata.json` carries mount maps, LSB scales and the calibration record; the
RAW view converts for display.

## DEC-008 — Workflow modes are SESSION_START kinds, not new message types

**Date:** 2026-09-17

**Status:** Accepted

### Context
Doc 08 §3 lists 18 message types. None starts calibration, reference capture or
a reference check, yet docs 04, 11 §4 and 12 need the dashboard to control all three.

### Options Considered
1. New message types beyond 0x12.
2. A `kind` field in SESSION_START (selected).
3. Mode flags through CONFIG_SET.

### Decision
SESSION_START kinds: CALIBRATION, REFERENCE_CAPTURE, REFERENCE_CHECK, EVALUATION.
A reference check runs in device state REFERENCE_CAPTURE, so it is haptic-free by
construction; RUNNING means an evaluation only. CALIBRATION does not create a
doc-10 session. The device's SESSION_STOP carries results (calibration record,
built reference).

### Reason
Stays inside the doc 08 message set and matches the doc 07 §6 states and the doc
12 workflow.

### Trade-offs
SESSION_START validation depends on the kind.

### Consequences
Byte layouts go in `docs/protocol.md` (M1).

## DEC-009 — Anatomical frame is defined per sensor by a mount map

**Date:** 2026-09-17

**Status:** Accepted

### Context
Docs 00 and 04 require both boards to share one physical axis orientation. A
board flat on a vertical shin cannot share the foot board's orientation
(`docs/hardware.md`). PROB-002 came from a map that was not a rotation.

### Options Considered
1. Require identical physical orientation (not realizable on the shank).
2. Per-sensor chip → anatomical mount map, plus doc 04 §4 gravity alignment for
   small residual tilt (selected).

### Decision
Each sensor has a signed-permutation mount map in `config_v1.h`, applied
identically to accel and gyro, with compile-time checks that it is a proper
rotation (determinant +1).

### Reason
Physically realizable, keeps doc 04's anatomical convention for every algorithm,
and makes a PROB-002-style reflection a build error.

### Consequences
Remounting a board requires updating its map and `hardware.md`. The dashboard
mounting check (M3) verifies axis signs on the body.

## DEC-010 — ESP-IDF `esp_http_server` serves the device WebSocket

**Date:** 2026-09-17

**Status:** Accepted

### Context
links2004 WebSockets 2.7.3 writes through Arduino `WiFiClient::write`, which
retries up to 10 times with 1 s `select()` timeouts
(`WIFI_CLIENT_MAX_WRITE_RETRY`, Arduino-ESP32 2.0.17). A half-open Wi-Fi client
can therefore stall the sender for ~10 s. The library also allocates per frame.

### Options Considered
1. links2004 WebSockets in a dedicated task.
2. ESPAsyncWebServer (additional dependency).
3. ESP-IDF `esp_http_server` WebSocket support, already compiled into the
   Arduino SDK (`CONFIG_HTTPD_WS_SUPPORT=1`) (selected).

### Decision
Option 3. Both external `lib_deps` are removed; Adafruit MPU6050 was never used.

### Reason
No added dependency, proper `/ws` routing, and socket-level control so sends can
be gated on writability.

### Trade-offs
Lower-level API; client bookkeeping is ours.

### Consequences
Implemented in M1. The link task must never block acquisition or processing.

## DEC-011 — Dashboard stack

**Date:** 2026-09-17

**Status:** Accepted

### Context
Doc 11 fixes Tauri 2 + React + TypeScript + Rust. Storage, charting, orientation
rendering and PDF generation were open; DEC-003 already fixes a Rust-native `.mat`.

### Options Considered
- Storage: SQLite vs flat binary session files.
- Charts: uPlot vs Recharts/ECharts.
- Orientation view: three.js vs Canvas2D.
- PDF: krilla vs typst as a library vs webview print.

### Decision
SQLite through rusqlite (bundled) with one writer thread and precomputed min/max
summaries; tokio-tungstenite and serialport for the two links; uPlot for all
time series; Canvas2D for the orientation view; krilla for the PDF with charts
drawn directly; plain CSS with design tokens and bundled IBM Plex fonts. No UI
kit, router or state library.

### Reason
SQLite gives range queries for the RAW view and cycle-to-raw traceability. uPlot
renders long streaming series cheaply. Two rotated boxes do not justify WebGL,
which is fragile under WebKitGTK. krilla is maintained (it backs typst) and
works offline; typst as a library pulls hundreds of crates; webview printing has
no programmatic PDF path on Linux.

### Trade-offs
Hand-built PDF layout and a small chart wrapper to maintain.

### Consequences
Each dependency is added only in the milestone that first uses it.

## DEC-012 — Reference profiles are built on the device

**Date:** 2026-09-17

**Status:** Accepted

### Context
Doc 07 §1 places `reference/builder` in firmware; doc 12 needs versioned, locked
references archived by the desktop.

### Options Considered
1. Device builds the profile from in-RAM cycle features when a reference capture
   stops (selected).
2. Dashboard builds it from stored cycles.

### Decision
The device returns the profile (median and MAD per feature plus derived event
thresholds) in SESSION_STOP. The dashboard assigns ID and version, stores it,
locks it once used, and sends it back in SESSION_START for checks and evaluations.

### Reason
Matches doc 07, keeps a single median/MAD implementation (no Rust/C++ parity to
maintain), is unaffected by telemetry gaps, and also yields the doc 05 §3 event
thresholds.

### Consequences
The reference blob format is versioned in `docs/protocol.md`. The dashboard never
recomputes reference statistics.

## DEC-013 — Values doc 06 leaves open

**Date:** 2026-09-18

**Status:** Accepted, provisional

### Context
Doc 06 specifies the error engine's structure — features, weights, the
`d = clamp(z/3)` normalization, the class list, the confidence weights and the
feedback gates — but leaves four values undefined. The engine cannot be written
without them, and inventing them silently would make an arbitrary number look
like a specified one.

### Decisions

| Value | Chosen | Reason |
|---|---|---|
| Spread floors per feature | 0.5°, 0.5°, 0.5°, 0.010 s, 0.005, 0.020 m, 5 °/s | Doc 06 §2 requires "a minimum floor so zero-variance features do not divide by zero" but gives none. These are roughly the smallest difference in each feature worth calling a deviation, and they are below the cycle-to-cycle variation measured on the 6 m walk. |
| Reference stability subscore | `cycles / 60`, clamped | Doc 06 §5 names the subscore without defining it. Thirty cycles is the documented minimum for a reference, so a reference at the minimum scores 0.5 and one with twice that scores 1. |
| "No single class dominates" | Another class within 10 % of the leader's weighted contribution | Doc 06 §4 uses the phrase without a threshold. Ten percent is narrow enough that a clear leader stays primary and wide enough that a near-tie reads as OVERALL_DEVIATION. |
| Unmeasurable feature | Cycle distance is dropped when ZUPT quality is 0 | Doc 06 §3 says missing features leave the denominator and reduce confidence, without saying which can be missing. Distance is the only feature that depends on a zero-velocity window. |

### Trade-offs
Every one of these is a guess constrained by measurement rather than a
specification. They are single named constants — `kFeatureSpreadFloor`, the
stability divisor, the 0.9 factor — so each can be changed in one place, and the
tests state the behaviour each produces.

### Consequences
Before any of this is used for a claim about a patient, these four values need
review against captures from more than one person. `docs/testing.md` records that
the error engine has been tested against hand-computed vectors only, never
against a real reference capture, because nobody has yet walked thirty cycles.

## DEC-014 — Segments are counted on the host, not the device

**Date:** 2026-09-18

**Status:** Accepted. Supersedes the enforcement half of DEC-004.

### Context
Doc 12 §5 has the researcher enter `max_valid_cycles_per_segment` and
`max_errors_per_segment`, and the segment closes on whichever is reached first.
DEC-004 decided in M0 that the limits would travel in SESSION_START and the
firmware would enforce them. Implementing that in M5 meant a fifth protocol
schema: two fields in SESSION_START, a segment index in every STEP_BATCH record
(132 → 134 bytes), regenerated golden vectors, and matching changes in the
firmware, the Rust decoder and `eadprobe`.

Before doing that, the question was asked: what on the device behaves
differently at a segment boundary? In V1, nothing. Haptics are not fitted
(DEC-006), so no feedback is gated per segment. There is no on-device flash
storage (M7 deferred), so no segment footer is written. The device state machine
does not change state at a boundary. The segment is a bookkeeping unit for the
researcher and for export.

### Options Considered
1. Firmware enforces the limits, as DEC-004 planned; protocol schema 5.
2. The host counts cycles as they arrive and rolls the segment in the store.
3. Both: the device counts and the host re-derives, checking agreement.

### Decision
Option 2. The limits are stored on the session row; `roll_segment` in the store
writer counts each cycle into the open segment and closes it when either limit
is reached, writing the segment index onto the cycle.

### Reason
The segment is a property of the session, which the host owns, and a rule
applied to a stored cycle stream is more reproducible than one applied to a live
one: re-running it over the same `cycles` rows gives the same segmentation, on
any machine, with no device present. Option 1 costs a protocol schema for a
counter. Option 3 costs the same and adds a disagreement to handle.

### What counts as an error
Doc 12 §5 says `max_errors_per_segment` without defining an error. Here it is a
scored cycle whose primary class is not NONE **and** whose confidence reaches
`kConfidenceForDisplay` (0.50). Below that confidence doc 06 §7 says the
classification should not be shown at all, so counting it toward a limit that
ends a segment would act on a finding the system will not even display.

### Trade-offs
Gained: no protocol change, reproducible segmentation, the rule in one function
with one test. Sacrificed: a device running without a host cannot segment — but
V1 has no standalone workflow, and a device with no host also has nowhere to
store the result.

### Consequences
This must move to the device if the device ever has to change behaviour at a
segment boundary: haptics gated per segment, or on-device storage writing a
segment footer (doc 09 §10). Both are the point at which the protocol change
earns its cost. `roll_segment` and `counts_as_error` in
`dashboard/src-tauri/src/store/mod.rs` are the two places to look.
