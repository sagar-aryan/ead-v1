# Engineering Decisions

Handoff decisions locked in `EAS_ERM_Driver_PCB_Handoff/08_DECISION_LOG.md` are not repeated here. This file records decisions made during the KiCad implementation.

## DEC-001 — Use the invoice's 100 nF capacitors as per-channel local decoupling

**Date:** 2026-09-22  **Status:** Accepted (user decision)

### Context
Invoice #3698208 contains only 17× 100 nF 1206 MLCC, which the handoff BOM does not use. Single-layer routing forces each channel's rail and GND to arrive from opposite sides of the board (see DEC-003). Without a local capacitor, the MOSFET/diode switching current loop closes through the regulator output capacitors, which are far away.

### Options Considered
1. One 100 nF from rail to GND at each channel (6 parts).
2. Option 1 plus 100 nF at each HT7833 VIN/VOUT (10 parts).
3. Swap the motor suppression capacitors C9–C14 from 10 nF to 100 nF.
4. Leave the 100 nF capacitors off this board.

### Decision
Option 1: C15–C20 = 100 nF, each from the channel's rail to GND, placed next to that channel's diode and MOSFET.

### Reason
It shrinks each channel's high-di/dt loop (diode → rail → capacitor → GND → MOSFET source) to a local loop. Option 3 would raise the charge pulled through the MOSFET at each PWM turn-on about 10× (C·V = 330 nC vs 33 nC).

### Trade-offs
6 extra parts and a little area. The handoff circuit is otherwise unchanged; C9–C14 stay 10 nF.

## DEC-002 — 220 µF polymer footprint: `Capacitor_SMD:CP_Elec_5x5.9`

**Date:** 2026-09-22  **Status:** Accepted

### Context
The handoff describes the part as "220 µF 6.3 V polymer, D5×L6.1" and requires the land pattern to be verified. The user confirmed it is an SMD V-chip (can on a plastic base with flat terminals).

### Decision
Use KiCad `CP_Elec_5x5.9`. Every `CP_Elec_5x5.x` footprint has identical pads (3.0×1.6 mm at ±2.2 mm); only the height/3D model differs, and 5.9 mm is the closest listed height to 6.1 mm.

### Consequences
A paper fit check (lay the part on the unmirrored 1:1 print) comes before etching.

## DEC-003 — PWM and M- wire pads are enclosed by copper, not on the board edge

**Date:** 2026-09-22  **Status:** Accepted (user decision)

### Context
Planarity analysis: with one copper layer, no jumpers and no traces under component bodies, the channels of a rail group cannot have their PWM or M- pads on the outer board edge (K3,3).

### Options Considered
1. Pads set just inside a GND/rail trace (wires soldered on top, passing over that trace).
2. Edge pads plus 0 Ω jumpers.
3. Edge pads plus traces routed between the pads of 1206 parts.

### Decision
Option 1.

### Trade-offs
Wires cross over a bare copper trace to leave the board (insulated wire; tie down or glue for strain relief). No jumpers, and no bridging risk from traces under parts on an unmasked board.

## DEC-004 — 0 Ω jumpers: target zero, allowed if necessary

**Date:** 2026-09-22  **Status:** Accepted (user instruction)

A 0 Ω 1206 jumper (or wire link) is permitted only if routing cannot close without one, or it clearly reduces board area or a switching loop. Each jumper must be recorded here. Neither invoice includes 0 Ω parts.

## DEC-005 — Etch rules 0.4 mm; hand-solder footprint variants

**Date:** 2026-09-22  **Status:** Accepted (user decision)

- Minimum track width 0.4 mm and minimum clearance 0.4 mm (laser-toner transfer).
- Footprints:
  - Resistors: `R_1206_3216Metric_Pad1.30x1.75mm_HandSolder`
  - Capacitors: `C_1206_3216Metric_Pad1.33x1.80mm_HandSolder`
  - Regulators: `SOT-89-3_Handsoldering` (superseded for U1/U2 by the local ≥ 0.4 mm variant, DEC-010)
  - MOSFETs: `SOT-23_Handsoldering`
  - Diodes: `D_SOD-123`. KiCad has no hand-solder SOD-123 variant, and inventing one would need verification against the vendor drawing, so the stock footprint is kept.
- Handoff trace-width minimums still apply: SW/GND trunks ≥ 1.5 mm, rails ≥ 1.0 mm, motor branches ≥ 0.8 mm, PWM/gate 0.4–0.5 mm.

## DEC-006 — No copper labels; printed assembly drawing instead

**Date:** 2026-09-22  **Status:** Accepted (user decision)

No text in copper. Pad functions and reference designators appear on the F.Fab layer and are plotted as an assembly drawing (1:1 and 3:1).

## DEC-007 — Motor pads: the handoff's 8-pad map, long A+/B+ pads

**Date:** 2026-09-22  **Status:** Accepted (user confirmed 6 motors and the handoff pad map)

Motor pads: `A+`, `M1-`, `M2-`, `M3-`, `B+`, `M4-`, `M5-`, `M6-`. The A+ and B+ pads are 2.5 × 7.0 mm so they can take up to three wires; the other M- pads are 2.5 × 2.5 mm.

## DEC-008 — Generate schematic and PCB from one Python data model

**Date:** 2026-09-22  **Status:** Accepted

### Context
The agent cannot operate the KiCad GUI. KiCad 10 provides `kicad-cli` (ERC, DRC with schematic parity, exports) and the `pcbnew` Python module. There is no Python API for schematics.

### Decision
`EAS_ERM_Driver/scripts/build.py` holds the part list and pin→net map once.
- It writes the `.kicad_sch` as S-expressions (library symbols copied from the installed KiCad libraries).
- It builds the `.kicad_pcb` through `pcbnew` from the same data (footprints, nets, symbol paths, placement, tracks, zones).
- `kicad-cli pcb drc --schematic-parity` then independently checks that the two agree.

### Consequences
The layout is reproducible and parametric: column pitch and row offsets are constants. Hand edits in the KiCad GUI are overwritten if `build.py` is re-run, so to keep them, stop using `build.py` for layout after editing.

## DEC-009 — HT7833 pinout: Holtek 1 GND, 2 VIN (tab), 3 VOUT

**Date:** 2026-09-22  **Status:** Accepted (user confirmed genuine Holtek parts)

Confirmed against Holtek HT78xx Rev. 1.51. The SOT-89 tab is pin 2 (VIN). The regulator's heat-spreading copper is therefore `SWITCHED_SYSTEM+` copper, connected solidly (no thermal relief).

## DEC-010 — Project-local SOT-89 land with ≥ 0.4 mm pad gaps

**Date:** 2026-09-22  **Status:** Accepted

### Context
KiCad's stock SOT-89 lands, `SOT-89-3` and `SOT-89-3_Handsoldering`, leave only **0.254 mm** between pin 1 (GND) or pin 3 (VOUT) and the inner corner of the wide tab pad (pin 2, VIN). That breaks the 0.4 mm etch rule (DEC-005). On a toner-transfer board such a gap can bridge under the regulator body, shorting VIN to GND or VOUT where it cannot be inspected after soldering.

### Options Considered
1. Keep the stock land and add a DRC exception for it.
2. Trim the wide part of the tab pad so every gap is ≥ 0.4 mm.

### Decision
Option 2. `EAS.pretty/SOT-89-3_Handsoldering_0.4mm` is generated by `build.py` from the stock hand-solder land:
- The wide tab strip now starts at x = −0.85 mm instead of −1.125 mm.
- A 0.9 mm neck joins it to the pin-2 lead.
- Pin positions, pin-1/3 pads and the tab end at x = +3.0 are unchanged.

### Trade-offs
0.275 mm less tab land under the body, next to the pin row. The tab still extends 1.75 mm beyond the body.

### Verification
- DRC is clean at 0.4 mm.
- The pin pitch (1.50 mm) and package outline (A 4.40–4.70, E 3.94–4.40 mm) were checked against Holtek HT78xx Rev 1.51, page 9.
- Fit against the physical part: checked by the paper fit check.

## DEC-011 — No GND pour under part bodies

**Date:** 2026-09-22  **Status:** Accepted

### Context
The first fill put 0.3–0.8 mm pour slivers between the pads of every 1206/SOD-123 part. It also produced 7 "connection too narrow" warnings (0.31–0.38 mm necks).

### Decision
`build.py` adds a copper-pour keepout rule area over the courtyard of every part except U1/U2 (whose tab copper is the heat spreader) and the wire pads. Every GND pad is connected by an explicit track, so no connection depends on the pour. Zone settings:
- clearance 0.5 mm
- minimum width 0.5 mm
- thermal spokes 0.5 mm
- islands removed

### Trade-offs
Less GND copper area (321 mm² pour remains). In exchange there is no hidden copper under parts on an unmasked board, and no narrow slivers to lift during etching.

## DEC-012 — 0.6 mm clearance around wire pads

**Date:** 2026-09-22  **Status:** Accepted

Wire pads carry a local 0.6 mm clearance instead of the global 0.4 mm. A hand-soldered wire needs more margin to neighbouring copper than a component lead, and the board has no solder mask. It costs about 0.2 mm per pocket.

## DEC-013 — Floorplan: one row of six channel cells ("comb")

**Date:** 2026-09-22  **Status:** Accepted

### Context
The user asked for the smallest area with no shape limit. Constraints:
- Single-layer planarity.
- The SOT-89 is placed on the top side only, so its pin order around the part is fixed (clockwise VIN → VOUT → GND).
- The same holds for each channel cell (clockwise PWM → M- → rail → GND).

### Decision
One row: `[U1 block][ch1 ch2 ch3][U2 block][ch4 ch5 ch6]`.
- **Top edge:** SWITCHED_SYSTEM+ line (1.5 mm) to both regulators.
- **Buses:** 3.3 V rail bus (1.0 mm) under the SW line; GND bus (1.5 mm) along the bottom edge.
- **Cells:** each hangs D, the 10 nF and the 100 nF from the rail bus. Its GND "tooth" (1.0 mm) runs on its right side to the bus.
- **Wire pads:** M- pad on the left of each cell; PWM pad at the bottom, enclosed by the GND bus.
- **Regulator orientation:** U1 has its tab facing the SW strip on the left edge; U2 has its tab up to the SW line.
- **Power corner:** bottom-left, holding SWITCHED_SYSTEM+, GND, ESP_SW_BAT+ and ESP GND.
- **MOTOR_B+:** sits on the free top edge above channels 4–6. MOTOR_A+ is a vertical pad beside U1's output.

### Reason
- Both regulators must sit at the left end of their group, so the VOUT rail flows away from the SW input without crossing GND.
- The SW line must reach U2 across the left group, so it runs along the rail side (top).
- A two-row version needs a second GND bus and a second SW run for roughly the same area, so it was not built.

### Result
- 82.75 × 19.0 mm (1572 mm²) with 0 jumpers.
- Channel pitch 9.45 mm, which is 0.05 mm above the clearance-limited minimum (tooth to next M- pad at 0.6 mm).

## DEC-014 — Schematic uses global labels on every pin

**Date:** 2026-09-22  **Status:** Accepted

- Local labels on the root sheet produce net names prefixed with "/" (e.g. `/PWM1`). Global labels give the exact handoff names (`PWM1`, `SWITCHED_SYSTEM+`), which the PCB nets must match for the schematic-parity DRC.
- `build.py` draws a 2.54 mm stub plus a global label at each pin rather than routing schematic wires. It is less pretty but trivially correct to generate.
- `M1+`…`M6+` are not separate nets; they are `HAPTIC_3V3_A/B`, because a net carries one name.

## DEC-015 — Assembly drawing content

**Date:** 2026-09-22  **Status:** Accepted

- **Part labels:** F.Fab only. Each part body shows its reference and a short value (`C15`/`100n`); all 1206 capacitors look identical in hand.
- **Wire pads:** short labels `PWR+`, `GND`, `ESP+`, `EGND`, `PWM1`…`PWM6`, `A+`, `B+`, `M1-`…`M6-`, plus a legend below the outline.
- **Value field:** C7/C8 carry Value `220uF`; the "6.3 V polymer, SMD V-chip D5" text is in Description, which the BOM exports.

## DEC-016 — Compact two-row variant (both sides < 50 mm), original kept unchanged

**Date:** 2026-09-22  **Status:** Accepted (user request)

### Context
The user asked to shorten the 82.75 × 19.0 mm strip, accepting more width, with both sides under 50 mm. The existing design must stay untouched: copy it, then edit the copy.

### Options Considered
1. **Two stacked rows of three channels, same orientation, each with its regulator at the left end.**
2. **A shared middle GND bus with row B rotated 180°**, so all PWM pads sit along the middle. Rejected: the rotated regulator block needs a SWITCHED_SYSTEM+ line across the board and a crossing-free path round the shared bus, for about the same size.
3. **Three rows of two channels.** Rejected: about 38 × 49 mm, a larger area.

### Decision
Option 1, in `EAS_ERM_Driver_compact/` (project `EAS_ERM_Driver_compact`). It is a full copy of `EAS_ERM_Driver/` with only the PCB layout section of `build.py` changed.
- **What changed (layout only):**
  - Row A = U1 block + ch1–3 on rail A; row B = U2 block + ch4–6 on rail B.
  - Every cell and regulator block is the verified single-row geometry, shifted.
  - Both HT7833 tabs face the SWITCHED_SYSTEM+ strip on the left edge, so the old top-edge SW line is gone.
  - GND bus A (between the rows) joins GND bus B down the right edge with a 1.5 mm link beside the end of the rail B bus.
  - MOTOR_B+ becomes a vertical 2.5 × 7 pad next to U2, mirroring MOTOR_A+.
- **What is unchanged:** netlist, parts, footprints, rules, schematic content.

### Result
- 44.70 × 32.60 mm (1457 mm²), **0 jumpers**, 0 vias, 0 holes.
- The single-row original: 82.75 × 19.00 mm (1572 mm²), also 0 jumpers.

### Trade-offs
- PWM1–3 and M1–3- are now in the middle of the board (row A), and PWM4–6 and M4–6- in row B. Wires reach the row A pads over bare copper, as before.
- Row A's motor return current flows along GND bus A, down the right-edge link and along GND bus B to the power pad. That is about 95 mm of 1.5 mm copper, roughly 30 mΩ by calculation. At 0.36 A (three motors starting) the drop is about 11 mV. Estimated, not measured.

## DEC-017 — Mini variant: four rule relaxations approved by the user

**Date:** 2026-09-22  **Status:** Accepted (user request and user-approved relaxations)

### Context
The user asked whether the compact board (44.70 × 32.60 mm) could be made smaller, without changing it: copy it, then edit the copy. My analysis found that under the existing rules the compact layout is close to its limit.
- The six channel cells are about 95 % packed (parts plus required clearances fill the 9.45 × 15.1 mm cell).
- Each regulator block is set by the 8 × 5.9 mm courtyard of the 220 µF next to the two input caps.
- The best rearrangement I found was worth about 0–3 %.
- A 0 Ω jumper would only remove the right-edge GND link, which costs 0.5 mm.

### Options offered (estimates given before building)
1. Wire-pad clearance 0.6 → 0.4 mm (the user's etch rule; undoes DEC-012).
2. Copper-to-edge 1.0 → 0.5 mm (handoff 04 §10 "prefers" 1 mm).
3. SWITCHED_SYSTEM+ strip and GND buses 1.5 → 1.0 mm (handoff 04 §2 recommends 1.5 mm).
4. Standard KiCad SOT-23 / SOT-89 / 1206 lands instead of hand-solder ones (undoes DEC-005's footprint choice).

### Decision
The user approved all four. `EAS_ERM_Driver_mini/` is a copy of `EAS_ERM_Driver_compact/`. Its `build.py` was changed as follows:
- **Footprints:** `R_1206_3216Metric`, `C_1206_3216Metric`, `SOT-23`, and `EAS:SOT-89-3_0.4mm`. The last is derived from the standard `SOT-89-3`, which has the same 0.25 mm tab-corner gap, so it gets the DEC-010 fix: the wide tab starts at x = −0.85, with a 0.9 mm neck.
- **Rules:** `WIRE_CLR` 0.4, `EDGE` 0.5, power trunks 1.0. `check.py` now requires trunks ≥ 1.0 mm.
- **Cell re-derived for the smaller lands:** pitch 8.1 mm (was 9.45), row height 13.94 mm (was 15.1). The GND tooth is 0.8 mm, the handoff's motor-branch minimum.
- **Regulator block re-derived:** about 13.4 mm wide (was ~15.3). The power corner is J1 (PWR+), J3 (ESP+), J2 (GND), J4 (EGND) along the bottom-left; ESP+ sits between PWR+ and GND and is fed from PWR+.

### Result
- 38.91 × 29.28 mm (1139 mm², 22 % smaller than the compact, 28 % smaller than the single row), **0 jumpers**.
- ERC, DRC and parity clean, and `check.py` passes, all on the first build (TEST-010).

### Trade-offs
- **Harder hand soldering:** shorter pads.
- **Cutting:** only 0.5 mm of copper margin, so cut carefully along the line.
- **Narrower power copper:** 1.0 mm still carries over 2 A, against about 1 A needed. This is a calculation, not a measurement.
- **Less regulator heat-spreading copper:** 20.1 mm² per regulator (compact 28.3 mm²), plus the 1.0 mm SW strip. Regulator temperature is still to be measured in bring-up.

## DEC-018 — Tight variant: 0.3 mm gaps and tight courtyards

**Date:** 2026-09-22  **Status:** Accepted (user choice)

### Context
The user asked whether the mini (38.91 × 29.28 mm) could go smaller. I offered four options:
- Stay with the mini.
- Tighter packing with 0.3 mm gaps (estimated about 37 × 27 mm).
- 0805 R/C, which means buying new parts (about 35 × 24 mm).
- Both (about 33 × 22 mm).

The user chose tighter packing with 0.3 mm gaps.

### Decision
`EAS_ERM_Driver_tight/` is a copy of the mini. Its `build.py` was changed as follows:
- **Clearance 0.3 mm** everywhere: the global rule, net classes and wire pads. Track widths are unchanged (signals 0.5, motor 0.8, rails 1.0, trunks 1.0), as is the 0.5 mm edge margin.
- **Tight courtyards instead of disabling the courtyard check.** Project copies of each footprint (`EAS:R_1206_tight`, `C_1206_tight`, `SOT-23_tight`, `D_SOD-123_tight`, `CP_Elec_5x5.9_tight`, `SOT-89-3_0.4mm_tight`) get one courtyard rectangle equal to the body and pad extents + 0.1 mm; KiCad's own add about 0.25–0.3 mm. KiCad's courtyard-overlap DRC stays active, so parts can never touch or overlap; they just sit closer.
- **Pads unchanged:** the SOT-89 keeps its ≥ 0.4 mm pad-gap fix.
- **Cell re-derived:** pitch 7.45 mm (was 8.1), row height 13.15 mm (was 13.94).
- **Regulator block re-derived:** about 12.8 mm wide (was ~13.4).

### Result
- 36.07 × 27.60 mm (996 mm²): 12.6 % smaller than the mini, 37 % smaller than the single row.
- **0 jumpers.** ERC, DRC and parity clean; `check.py` passes (TEST-011).

### Trade-offs
- **Etching:** 0.3 mm gaps are below the 0.4 mm the user's toner process was set for, so the transfer must be crisp. Inspect every gap before soldering.
- **Soldering:** parts sit about 0.2 mm apart body to body, which is harder to solder and rework.
- **Copper:** GND pour 144 mm² (mini 193); regulator heat-spreader copper 19.9 mm² each (mini 20.1).

## DEC-019 — Fabricate the tight variant, accepting 0.3 mm gaps on 0.5 mm-rated paper

**Date:** 2026-09-23  **Status:** Accepted (user decision)

### Context
The user will etch this themselves with Robocraze A4 PCB toner transfer paper (pack of 10), on a laser printer of unknown dpi (most home lasers are 600 dpi). The product listing claims "almost lossless transfer for upto 0.5mm tracks" and gives no gap figure.

Measured against that claim:
- **Traces:** the narrowest trace in every variant is 0.5 mm, exactly at the claim.
- **Gaps:** single row / compact / mini use 0.4 mm; the tight variant uses 0.3 mm, below it.
- **Pour-to-copper gap:** 0.5 mm in every variant.

### Options offered
1. Test strip first (trace/gap pairs 0.25–0.6 mm plus the real pad patterns).
2. A 0.5 mm-gap variant, estimated about 39.2 × 29.6 mm, only ~0.3 mm per side larger than the mini.
3. Keep the tight variant (36.07 × 27.60 mm) and accept the risk.
4. 0805 parts for a smaller board (needs a new purchase).

### Decision
Option 3: fabricate the tight variant as built. No further variants.

### Consequences
- The 0.3 mm gaps are below the paper's stated capability, so **the risk is a short between neighbouring tracks** where the copper between them does not clear.
- Mitigation is inspection, not design: see the pre-etch and post-etch checks in `hardware.md`.
- If the etch shows bridges that cannot be cleaned up, the fallback is the mini (0.4 mm gaps, 38.91 × 29.28 mm), which is already built and verified.

### Parts check
All parts are in stock with spares, and no 0 Ω part is needed: HT7833 2/10, IRLML6344 6/10, 1N5819W 6/22, 100 Ω 6/40, 100 kΩ 6/20, 1 µF 4/10, 10 µF 2/20, 10 nF 6/20, 100 nF 6/17, 220 µF 2/8.

## DEC-020 — Six-up toner sheet with a printed ruler

**Date:** 2026-09-23

**Status:** Accepted

### Context
The tight variant is what will be etched. A toner transfer usually takes more than one attempt, and the transfer is worthless if the printer silently rescales the page: at 96 % scale the 0.3 mm gaps and every land shrink with it, and nothing on a single-board print reveals it.

### Options Considered
1. Print the existing single-board toner PDF several times.
2. Tile copies on one sheet with a printed dimension reference.
3. Rely on measuring the board outline alone to detect rescaling.

### Decision
Option 2: `EAS_ERM_Driver_tight/scripts/panel.py` generates a 3 × 2 sheet from the verified board, 30 mm apart, with a 100 mm horizontal ruler (10 mm ticks) and a 50 mm vertical ruler outside the copies.

### Reason
- Six copies from one sheet of transfer paper; a smeared transfer costs one copy, not a sheet.
- A 100 mm reference resolves a 1 % scale error as 1 mm, which a ruler shows plainly. The board outline is only 36 mm, where the same error is 0.36 mm and easy to miss.
- Two rulers, because a printer can scale the axes differently.

### Trade-offs
- The sheet must be cut twice: the ruler strip off first, then the copies apart. The 30 mm pitch is not kerf allowance: it leaves about 15 mm of paper around each cut-out copy, which the user folds around the copper clad to hold the transfer in place under the iron. It was 8 mm until the user asked for the fold-over margin, and the gap is now an argument (`panel.py 3 2 30`).
- `panel.py` is a separate script, not part of `export.sh`: it reads the verified board and writes only to `output/`, so it can never alter a verified design.

### Consequences
- The panel PDF is a derived artifact. After any change to the tight board, re-run `panel.py`; it re-measures its own output and exits non-zero if the scale check fails.
- The same script works for the other variants by changing `PROJ`, and takes the grid as arguments (`panel.py 3 2`).
