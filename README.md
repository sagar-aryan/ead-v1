# EAD V1

A right-leg, barefoot, wearable error-augmentation device for gait research after
stroke. Two BNO086 IMUs (foot and shank) on a Seeed XIAO ESP32-S3 detect each gait
cycle, score it against the wearer's own recorded baseline, and cue errors through six
vibration motors on a calf band. A Tauri desktop dashboard records the raw data,
manages baselines and sessions, and exports CSV, MATLAB and PDF.

Research and engineering prototype. Not a medical device and not a validated
clinical tool.

## Where to start

`docs/handoff.md`: the current state, how to build, run and verify, and what is
next. `docs/README.md` indexes the rest of the engineering record.

| Folder | What it holds |
|---|---|
| `firmware/` | Device firmware (PlatformIO); host-testable core in `lib/ead_core` |
| `dashboard/` | Tauri 2 dashboard: Rust backend, React frontend |
| `hardware/erm-driver-pcb/` | KiCad project of the motor driver board (tight variant fitted) |
| `protocol/vectors/` | Golden wire-protocol vectors and their generator |
| `tools/` | `eadprobe.py` (independent device client), export checkers, replay |
| `recordings/` | Walk recordings with step-count ground truth |
| `docs/` | Decisions, problems, tests, progress, protocol, hardware |
| `ead_agent_docs_v2/` | The original V1 specification (read-only; deviations are in `docs/decisions.md`) |

## Safety

Never turn the battery switch ON while USB is plugged into the XIAO. Motor tests run
on battery over Wi-Fi with USB unplugged.
