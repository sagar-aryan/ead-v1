# Architecture

## Overview
The EAS V1 ERM driver is a passive power-and-switching board. It receives switched single-cell LiPo power, makes two independent 3.3 V motor rails, and switches six ERM motors on their negative side from six ESP32 PWM signals. It has no firmware, no connectors and no active control. The system-level architecture is fixed by the handoff (`01_SYSTEM_ARCHITECTURE.md`, `08_DECISION_LOG.md`).

```text
LiPo ── MCP73833 BAT (permanent)        MCP73833 LOAD+ ── external master switch ──┐
                                                                                    │ PWR+ pad
                         ┌──────────────────── THIS BOARD ──────────────────────────┴────────┐
                         │  SWITCHED_SYSTEM+ ──┬── U1 HT7833 ── HAPTIC_3V3_A ── ch1-3 (A+ pad) │
                         │                     ├── U2 HT7833 ── HAPTIC_3V3_B ── ch4-6 (B+ pad) │
                         │                     └── ESP+ pad ──► XIAO ESP32-S3 battery input     │
                         │  PWM1..6 pads ◄── XIAO GPIO      GND pads (PWR, ESP) = one GND net    │
                         └────────────────────────────────────────────────────────────────────┘
```

## Components
- **Power input:** pads `PWR+` (SWITCHED_SYSTEM+) and `GND`. The master switch is external, so this board is dead whenever the switch is off, even while charging.
- **Regulator domains:** U1 feeds rail A (motors 1–3) and U2 feeds rail B (motors 4–6).
  - Each has 1 µF + 10 µF on the input, and 1 µF + 220 µF polymer on the output.
  - The rails are never joined.
- **Channel ×6 (identical cells):**
  - `PWMn → 100 Ω → IRLML6344 gate`, with a 100 kΩ pulldown so the gate stays off when the ESP32 is unpowered or its pin floats.
  - Source to GND; drain to the motor negative `Mn-`.
  - 1N5819W flyback diode (cathode to the rail) and 10 nF across the motor.
  - 100 nF from rail to GND at the cell (DEC-001).
- **Wire pads:** 18 SMD pads replace connectors (see `README.md`).

## Current paths
- **Motor current (DC and low frequency):** PWR+ → SW trunk → HT7833 → rail bus → motor (off-board) → Mn- pad → MOSFET → GND tooth → GND bus → GND pad.
- **Switching current (high frequency):** at each PWM edge the motor current commutates between the MOSFET and the flyback diode. The local 100 nF supplies that edge current within the cell (loop about 12 mm², estimated). Without it the loop would close through the regulator output capacitors, a loop about as large as the whole board. This is the reason for DEC-001.
- **ESP32 supply:** PWR+ → ESP+ pad, and EGND → GND. Both sit at the power corner, the star point, so motor return current does not flow between the ESP32 ground and the supply return.

## Physical topology
One copper layer forces a planar layout. Two constraints drive the floorplan:
1. **Planarity:** each channel connects to the outside world (its PWM and M- pads), to GND and to its rail. With three channels sharing a rail that forms K3,3, so the PWM and M- pads must be enclosed by copper rather than sit on the board edge.
2. **Chirality:** parts can only be rotated, not mirrored, on a top-only board. The fixed clockwise pin order is VIN → VOUT → GND for the SOT-89, and PWM → M- → rail → GND for the channel cell. Both regulators must therefore sit at the left end of their channel group.

The result (DEC-013):
```text
top edge  ══ SWITCHED_SYSTEM+ ═══════════════════════════╗        (B+ pad on free top edge)
          [U1]══ rail A bus ══ ch1 ch2 ch3   [C2 C4 U2]══ rail B bus ══ ch4 ch5 ch6
          SW strip                GND teeth down each cell's right side
bottom    PWR+ ESP+ GND EGND ══ GND bus (PWM pads just above it) ══════════════════════
```
Each cell hangs D, the 10 nF and the 100 nF from the rail bus. The MOSFET sits below with its drain pointing left to the M- pad; the gate network and PWM pad are at the bottom.

### Compact variant (DEC-016)
```text
top edge   [U1 blk]══ rail A bus ══ ch1 ch2 ch3 ─┐
SW strip   PWM1-3 pads ══ GND bus A ═════════════╡ GND link (right edge)
(left      [U2 blk]══ rail B bus ══ ch4 ch5 ch6  │
 edge)     PWR+ ESP+ GND EGND ══ GND bus B ══════╛
```
Both rows reuse the same cell and regulator geometry. The SW strip up the left edge feeds both HT7833 tabs directly. The only new copper is the right-edge link that joins the two GND buses beyond the end of the rail B bus.

The **mini variant** (DEC-017) has the same two-row topology, re-derived for the standard lands and the relaxed rules.

## Interfaces
| Interface | Pads | Electrical |
|---|---|---|
| Power in | PWR+, GND | 1 S LiPo range, about 3.0–4.2 V switched; about 1 A peak total (6 motors starting at 120 mA plus the ESP32) |
| ESP32 | ESP+, EGND, PWM1–6 | ESP+ is the raw switched battery rail, not 3.3 V; PWM is 3.3 V logic into 100 Ω / 100 kΩ |
| Motors | A+, M1–M3-, B+, M4–M6- | 3.3 V at up to 3 × 120 mA per rail |

## Dependencies
- **KiCad 10.0.6:** `kicad-cli` and the `pcbnew` Python module, used by the build, verification and exports. The files are KiCad 10 format.
- **Python 3.12** standard library, plus **Pillow** (preview cropping only).
- **poppler `pdftoppm`:** PNG previews.

## Constraints
- **From the handoff:** one copper layer, no vias, no holes, no through-hole parts, no connectors, no test points, no onboard switch. No part substitutions (handoff 00, 05).
- **From the user:**
  - 0.4 mm etch rules with hand-solder pads; no copper text.
  - The 100 nF capacitors used as local decoupling.
  - Jumpers allowed only if necessary (none were needed).
