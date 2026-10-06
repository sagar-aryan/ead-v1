# EAD V1

A right-leg, barefoot wearable for gait research after stroke, built on error
augmentation. Two IMUs on the foot and shank detect every gait cycle; the device
scores each cycle against the wearer's own recorded baseline and, when a step is off,
cues it through six vibration motors on a calf band. A desktop dashboard records the
raw data, manages baselines and sessions, and exports CSV, MATLAB and PDF.

> Research and engineering prototype. Not a medical device and not a validated clinical
> tool.

![Worn assembly: foot and shank IMUs, controller, and the six-motor calf band](docs/images/worn_assembly.png)

*Generated illustration of the worn layout and motor positions. Known errors: in
panel A the controller should be on the outer side of the leg (panel B is right) and
the foot is drawn as a left foot; the motor cable has 8 wires (two separate positive
rails), not 7. Details: [`docs/hardware.md`](docs/hardware.md), worn assembly.*

## How it works

```text
foot + shank BNO086 (SPI, 200 Hz)
  → XIAO ESP32-S3: orientation, gait events, zero-velocity updates, per-cycle score
  → vibration cue on the device (no laptop in the loop)
  → Wi-Fi access point or USB → dashboard (Tauri): live view, SQLite store, exports
```

- **Gait:** each right-leg cycle is found from the shank's swing; heel strike, toe-off,
  stance, swing, cadence, stride length and speed are computed on the device, with
  foot-flat zero-velocity updates to bound integration drift.
- **Score:** seven features (swing dorsiflexion, contact angle, inversion/eversion,
  cycle time, stance ratio, stride distance, shank swing rate) are compared with the
  baseline's medians and spreads and combined into one score from 0 to 1.
- **Cue:** a score of 0.35 or more starts a 0.5 s buzz, 60–100 % strength by how far off
  the step was. Where it buzzes names the error, on the side to move away from: the
  back of the calf for too little toe lift or a toe-first landing, the inner side for
  rolling onto the outer edge (inversion), the outer side for rolling in (eversion),
  front and back in turn for timing. The buzz stops at the first good step. On-time limits (5 s
  continuous, 50 % over any 10 s) are enforced on the device; sensor faults, a switched
  off feedback toggle, or 5 s without the laptop hold cues back.
- **Data:** every raw sample is kept. A 6 MB buffer on the device holds about 4.5 min,
  so a Wi-Fi dropout is filled in after reconnecting.

## Hardware

| Part | Detail |
|---|---|
| Controller | Seeed XIAO ESP32-S3 (plain, not Sense) |
| IMUs | 2 × 7Semi BNO086 breakout on SPI: foot (dorsum) and shank (anterior shin) |
| Haptics | 6 ERM coin motors around the calf, driven by the PCB in `hardware/erm-driver-pcb/` (tight variant): low-side MOSFETs, two 3.3 V regulators |
| Power | 1S Li-ion cell, charger module, master switch |

Every pin and connection, with its evidence: [`docs/wiring_reference.md`](docs/wiring_reference.md).
All 15 usable XIAO GPIOs are in use.

## Repository

| Folder | Contents |
|---|---|
| `firmware/` | Device firmware (PlatformIO, Arduino-ESP32); portable, host-tested core in `lib/ead_core` |
| `dashboard/` | Desktop app: Rust backend (device link, SQLite, exports) and React frontend |
| `hardware/erm-driver-pcb/` | KiCad project of the motor driver board, with its own docs |
| `protocol/vectors/` | Golden wire-protocol messages and their generator |
| `tools/` | `eadprobe.py` (independent device client), export checkers, walk replay |
| `recordings/` | Walk recordings with counted steps, used to test gait detection |
| `docs/` | Engineering record: decisions, problems, tests, progress, protocol, handoff |
| `ead_agent_docs_v2/` | The original V1 specification (read-only; deviations are recorded in `docs/decisions.md`) |

## Getting started

**Prerequisites:** PlatformIO, Node.js 20+, a stable Rust toolchain, Python 3 with
pyserial (numpy and scipy for the export checker), and on Linux WebKitGTK 4.1.
`scripts/setup-linux.sh check` reports what is missing (macOS and Windows scripts
exist but have not been run).

**Firmware:**

```sh
pio run -d firmware -t upload        # build and flash over USB
python3 tools/eadprobe.py hello      # identity, protocol schema, firmware version
python3 tools/eadprobe.py check      # per-wire sensor check
```

The first build creates `firmware/include/ead_secrets.h` with a random Wi-Fi
passphrase. It is ignored by git; never commit it.

**Dashboard:**

```sh
cd dashboard && npm install
npx tauri dev                        # development
npx tauri build --no-bundle          # release build; see dashboard/README.md for `ead`
```

**Connecting:** over USB, or join the device's access point `EAD-V1-xxxx` (passphrase in
`ead_secrets.h`) and connect the dashboard to `ws://192.168.4.1:8080/ws`.

## Using it

1. **Calibrate:** stand still for 5 s with the sensors strapped on.
2. **Baseline:** walk normally on a straight path until at least 30 valid cycles
   (about 60 steps) are collected. Baselines are versioned per patient and locked.
3. **Check:** a short scored walk against the baseline.
4. **Evaluate:** walk with feedback on; the session's limits (cycles, errors) are set
   first.
5. **Export:** raw samples, cycles, events and cues as CSV, plus `metadata.json`,
   `session.mat` and a PDF report.

Calibrate again after every restart, and record a new baseline whenever the sensors
are re-strapped: the baseline is only valid for the strapping it was recorded in.

## Testing

```sh
pio test -d firmware -e native                     # firmware core
(cd dashboard/src-tauri && cargo test)             # backend
(cd dashboard && npm test && npm run build)        # frontend
python3 tools/eadprobe.py vectors                  # protocol, against the golden vectors
```

On hardware, `eadprobe stats` and `cargo test hardware -- --ignored --test-threads=1`.
Every test run, with its measured results, is in [`docs/testing.md`](docs/testing.md).

## Safety

- **Never turn the battery switch on while USB is plugged in.** Motor tests run on
  battery over Wi-Fi with USB unplugged.
- Motors are held off from power-up; GPIO39 must never drive a motor
  ([`docs/wiring_reference.md`](docs/wiring_reference.md) §10).

## Status

Working end to end on one healthy wearer, worn on battery over Wi-Fi. Open items: the
confidence measure cannot yet block a cue; nothing detects that a baseline no longer
matches the strapping; slow-walk distance reads short; event timing and thresholds are
unvalidated beyond one wearer. Current state and next steps:
[`docs/handoff.md`](docs/handoff.md).

## Documentation

Start with [`docs/handoff.md`](docs/handoff.md); [`docs/README.md`](docs/README.md)
indexes the rest, including the wire protocol ([`docs/protocol.md`](docs/protocol.md))
and the decision log ([`docs/decisions.md`](docs/decisions.md)).

## License

MIT ([`LICENSE`](LICENSE)): firmware, dashboard, tools, hardware design files and
documentation. Exceptions keep their own licences: the IBM Plex fonts in
`dashboard/src-tauri/assets/fonts/` (SIL Open Font License 1.1, `OFL.txt` beside them)
and the agent skills in `.claude/skills/` (Apache 2.0 and MIT, as each states). The
software comes with no warranty, and nothing here is cleared for clinical use.
