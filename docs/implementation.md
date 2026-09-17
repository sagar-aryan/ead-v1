# Implementation

What exists in the code today. Target design: `docs/architecture.md`.
Milestone plan: `docs/handoff.md`.

## Status by milestone

| Milestone | Scope | Status |
|---|---|---|
| M0 | Baseline fixes and cleanup | Complete except on-body gravity check (TEST-014) |
| M1 | Data-ready acquisition, protocol, Wi-Fi + USB links, backfill ring | Complete |
| M2 | Dashboard foundation: backend, shell, LIVE, RAW, recording, SESSIONS | Complete |
| M3 | Calibration, Mahony orientation, mounting check, datasets | Not started |
| M4 | Gait events + ZUPT, EVENTS/CYCLES/TRENDS | Not started |
| M5 | Reference, error engine, session workflow | Not started |
| M6 | CSV, `.mat`, PDF exports | Not started |
| M7 | On-device flash storage and recovery | Not planned in detail (needs a DEC) |

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
