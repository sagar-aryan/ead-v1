# EAS V1 — Six-Channel ERM Driver PCB

A single-sided, SMD-only, drill-free driver board for six 3 V ERM vibration motors in the EAS wearable. Two HT7833 LDOs make two isolated 3.3 V motor rails; six IRLML6344 low-side switches take PWM from a separate XIAO ESP32-S3. The specification is in `../EAS_ERM_Driver_PCB_Handoff/`; this folder records how it was implemented.

**In the EAD V1 repository since 2026-10-05** (`hardware/erm-driver-pcb/`), copied from
`~/Documents/ead pcb/` without the purchase invoices and KiCad's per-user files. The
**tight variant** (`EAS_ERM_Driver_tight/`, 36.07 × 27.60 mm) is the board fitted on
the device (user, 2026-10-05). The device as a whole is described in `../../../docs/`
(`hardware.md`, `wiring_reference.md`); the notes below are this board's own record,
as written.

## Status (2026-09-22)
- **Four layout variants, all verified in software:** ERC 0, DRC 0 violations, 0 unconnected, 0 schematic-parity issues, and every `check.py` handoff check passes.
- **None is fabricated or tested on hardware yet.** See `testing.md`.

| Variant | Folder | Board | Layout | 0 Ω jumpers |
|---|---|---|---|---|
| Single row (original) | `EAS_ERM_Driver/` | 82.75 × 19.00 mm (1572 mm²) | one row of 6 channels (DEC-013) | **none** |
| Compact | `EAS_ERM_Driver_compact/` | **44.70 × 32.60 mm** (1457 mm²) | two rows of 3 channels (DEC-016) | **none** |
| Mini | `EAS_ERM_Driver_mini/` | **38.91 × 29.28 mm** (1139 mm²) | two rows of 3, relaxed rules (DEC-017) | **none** |
| Tight | `EAS_ERM_Driver_tight/` | **36.07 × 27.60 mm** (996 mm²) | mini + 0.3 mm gaps and tight courtyards (DEC-018) | **none** |

All variants share the same circuit, parts and schematic content. Single row and compact also share footprints and rules and differ only in layout. The mini uses the standard (smaller) lands, a 0.4 mm gap around wire pads, a 0.5 mm edge margin and 1.0 mm power trunks (user-approved). The tight variant additionally uses 0.3 mm copper gaps and tight part courtyards. Choose one to fabricate.

| Common item | Value |
|---|---|
| Copper | one layer (F.Cu); no vias, no holes, no jumpers |
| Parts | 46 SMD parts + 18 SMD wire pads (no connectors) |
| Etch rules | 0.4 mm minimum track and gap; 0.5 mm pour clearance. Single row / compact: 0.6 mm around wire pads, 1.0 mm copper-to-edge, 1.5 mm power trunks, hand-solder lands. Mini: 0.4 mm, 0.5 mm, 1.0 mm, standard lands |

## Power rails
| Net | Source | Feeds | Copper |
|---|---|---|---|
| `SWITCHED_SYSTEM+` | wire pad `PWR+`, from the external master switch on the MCP73833 LOAD+ side (never raw BAT) | U1 and U2 VIN (SOT-89 tab), pad `ESP+` | 1.5 mm strip up the left edge (single row: also along the top edge to U2; mini: 1.0 mm) |
| `HAPTIC_3V3_A` | U1 HT7833 | motors 1–3 (pad `A+`), C5 1 µF, C7 220 µF, C15–C17 100 nF | 1.0 mm bus |
| `HAPTIC_3V3_B` | U2 HT7833 | motors 4–6 (pad `B+`), C6 1 µF, C8 220 µF, C18–C20 100 nF | 1.0 mm bus |
| `GND` | wire pad `GND` | everything | 1.5 mm bus under each channel row (mini: 1.0 mm), a 1.0 mm tooth per channel (mini: 0.8 mm), and pour (compact and mini: the two row buses are joined down the right edge) |

The two 3.3 V rails are never connected on the board. `check.py` confirms this, and DRC would flag any short.

## Wire pad map
Labels are as printed on the assembly drawings. Positions are pad centres in mm from the top-left corner of the outline.

| Drawing label | Ref | Net | Connect to | Pad (mm) |
|---|---|---|---|---|
| `PWR+` | J1 | SWITCHED_SYSTEM+ | master switch output (charger LOAD+ side) | 3.0 × 2.5 |
| `GND` | J2 | GND | charger LOAD− / system ground | 3.0 × 2.5 |
| `ESP+` | J3 | SWITCHED_SYSTEM+ | XIAO **battery** input (`ESP_SW_BAT+`). **Not** the XIAO 3V3 pin | 2.0 × 2.0 |
| `EGND` | J4 | GND | XIAO GND | 2.0 × 2.0 |
| `PWM1`…`PWM6` | J5–J10 | PWM1–6 | XIAO GPIOs | 2.0 × 2.0 |
| `A+` | J11 | HAPTIC_3V3_A | + leads of motors 1–3 (up to 3 wires) | 2.5 × 7.0 |
| `M1-`…`M3-` | J12–J14 | M1-…M3- | − lead of motors 1–3 | 2.5 × 2.5 |
| `B+` | J15 | HAPTIC_3V3_B | + leads of motors 4–6 (up to 3 wires) | 2.5 × 7.0 |
| `M4-`…`M6-` | J16–J18 | M4-…M6- | − lead of motors 4–6 | 2.5 × 2.5 |

Pad centres (mm from the top-left corner of the outline; each variant's full list is in its `output/check.txt`):

| Pads | Single row | Compact | Mini | Tight |
|---|---|---|---|---|
| J1 `PWR+`, J2 `GND` | (2.5, 16.8), (6.2, 16.8) | (2.5, 30.4), (6.2, 30.4) | (2.0, 27.5), (7.8, 27.5) | (2.0, 25.9), (7.6, 25.9) |
| J3 `ESP+`, J4 `EGND` | (2.0, 14.2), (9.2, 17.0) | (2.0, 27.9), (9.2, 30.6) | (4.9, 26.4), (10.7, 27.8) | (4.8, 24.8), (10.4, 26.1) |
| `PWM1`–`PWM3` | y 14.9; x 16.7, 26.1, 35.6 | y 13.0; x 16.7, 26.1, 35.6 | y 12.0; x 14.8, 22.9, 31.1 | y 11.3; x 14.1, 21.5, 28.9 |
| `PWM4`–`PWM6` | y 14.9; x 55.0, 64.4, 73.9 | y 28.5; x 16.7, 26.1, 35.6 | y 26.4; x 14.8, 22.9, 31.1 | y 24.8; x 14.1, 21.5, 28.9 |
| `A+` | (13.4, 6.7) vertical | (13.4, 4.8) vertical | (10.2, 4.0) vertical | (9.8, 4.0) vertical |
| `M1-`–`M3-` | y 9.9; x 16.8, 26.2, 35.6 | y 8.1; x 16.8, 26.2, 35.6 | y 7.2; x 15.6, 23.7, 31.8 | y 6.9; x 14.5, 21.9, 29.4 |
| `B+` | (59.7, 2.2) horizontal | (13.4, 20.3) vertical | (10.2, 18.3) vertical | (9.8, 17.4) vertical |
| `M4-`–`M6-` | y 9.9; x 55.0, 64.5, 74.0 | y 23.6; x 16.8, 26.2, 35.6 | y 21.6; x 15.6, 23.7, 31.8 | y 20.4; x 14.5, 21.9, 29.4 |

- **Board edge:** only the four power/ESP-power pads sit at the edge. The PWM and M- pads are enclosed by a GND or rail trace, so their wires pass over that trace (PROB-002, DEC-003). Use insulated wire and glue it down for strain relief.
- **Channel mapping:** PWMn drives Mn- (n = 1…6). The motor sits between A+ (or B+) and its Mn- pad.

## Rebuild and verify
```bash
cd EAS_ERM_Driver && bash scripts/export.sh            # single-row variant
cd EAS_ERM_Driver_compact && bash scripts/export.sh    # compact variant
cd EAS_ERM_Driver_mini && bash scripts/export.sh       # mini variant
cd EAS_ERM_Driver_tight && bash scripts/export.sh      # tight variant
```
Each run rebuilds from its own `scripts/build.py`, then runs ERC, DRC+parity, `check.py` and all exports, and stops on any failure.
- **Requirements:** KiCad 10.0.6 (`kicad-cli` and the `pcbnew` Python module), Python 3, poppler `pdftoppm`, Pillow.
- **To inspect:** open `EAS_ERM_Driver*.kicad_pro` in KiCad 10; the files are not readable by KiCad ≤ 9.

## Outputs (`<variant folder>/output/`, file names prefixed with the project name)
| File | Use |
|---|---|
| `*_TONER_F.Cu_MIRRORED_print_1to1.pdf` | **print this for toner transfer**, at 100 % scale (one copy) |
| `*_TONER_PANEL_6up_MIRRORED_print_1to1.pdf` | tight and mini variants: six copies on one A4 sheet, 30 mm apart, with a printed ruler to confirm the print scale (DEC-020). Any other grid: `scripts/panel.py <cols> <rows> [gap mm]` (it is not produced by `export.sh`, which clears `output/`) |
| `*_PANEL.kicad_pcb` | throwaway board the panel PDF is plotted from; regenerate it with `scripts/panel.py`, never edit it |
| `*_F.Cu_top_view_fitcheck_1to1.pdf` | unmirrored top view; lay the real parts on it before etching |
| `*_placement_3to1.pdf` | **place parts from this one**: part outlines, reference + value, pad labels and legend, without pad numbers |
| `*_assembly_1to1.pdf`, `*_assembly_3to1.pdf` | the same drawing with the pads and their pin numbers sketched; use it to identify pin 1 on U1/U2 and Q1–Q6. The pad digits overlap some labels, which is why `*_placement_3to1.pdf` exists |
| `*_schematic.pdf` | schematic |
| `*_bom.csv` | BOM |
| `gerbers/` | F.Cu and Edge.Cuts Gerbers (no drill file: zero holes) |
| `erc.rpt`, `drc.rpt`, `check.txt` | verification reports; `check.txt` contains the full connection summary and pad map |
| `preview_*.png` | quick-look images |

## Documentation index
- `architecture.md`: system and board structure.
- `implementation.md`: how the build scripts generate the design.
- `hardware.md`: BOM, pinouts, footprints, thermal, fabrication and assembly.
- `decisions.md`: DEC-001…DEC-020.
- `problems.md`: PROB-001…PROB-006.
- `testing.md`: what was verified and what is still pending.
- `progress.md`: chronological log.
- `handoff.md`: how to continue.
