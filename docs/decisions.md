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

**Status:** Accepted

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

**Status:** Accepted

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
