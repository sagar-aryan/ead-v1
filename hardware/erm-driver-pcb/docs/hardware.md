# Hardware

## Bill of materials (populated)
Generated BOM: `EAS_ERM_Driver/output/EAS_ERM_Driver_bom.csv` (identical in the compact variant; the mini has the same parts and values with the standard lands `R_1206_3216Metric`, `C_1206_3216Metric`, `SOT-23`, `EAS:SOT-89-3_0.4mm`, per DEC-017). Stock figures come from handoff `06_BOM.md` / `09_SOURCE_NOTES.md` (Robu order of 13/09/2026) and invoice #3698208 (16/09/2026).

| Ref | Part | Qty | Footprint | Stock |
|---|---|---:|---|---:|
| U1, U2 | Holtek HT7833, 3.3 V 500 mA LDO, SOT-89 | 2 | `EAS:SOT-89-3_Handsoldering_0.4mm` (DEC-010) | ~10 (user) |
| Q1–Q6 | Infineon IRLML6344, N-MOSFET | 6 | `SOT-23_Handsoldering` | 10 |
| D1–D6 | 1N5819W Schottky 40 V (KEXIN) | 6 | `D_SOD-123` | 22 |
| R1–R6 | 100 Ω 1206, gate series | 6 | `R_1206_…_HandSolder` | 40 |
| R7–R12 | 100 kΩ 1206, gate pulldown | 6 | `R_1206_…_HandSolder` | 20 |
| C1, C2, C5, C6 | 1 µF 1206 50 V, regulator in/out | 4 | `C_1206_…_HandSolder` | 10 |
| C3, C4 | 10 µF 1206, regulator input bulk | 2 | `C_1206_…_HandSolder` | 20 |
| C7, C8 | 220 µF 6.3 V polymer, SMD V-chip D5 | 2 | `CP_Elec_5x5.9` (DEC-002) | 8 |
| C9–C14 | 10 nF 1206, across each motor | 6 | `C_1206_…_HandSolder` | 20 |
| C15–C20 | **100 nF 1206**, rail-to-GND per channel (DEC-001) | 6 | `C_1206_…_HandSolder` | 17 (invoice #3698208) |
| J1–J18 | SMD wire solder pads (copper only, no part) | 18 | `EAS:WirePad_*` | — |

No 0 Ω jumper is used.

## Pinouts used (manufacturer pin → footprint pad)
| Part | Pad 1 | Pad 2 | Pad 3 | Source |
|---|---|---|---|---|
| HT7833 SOT-89 | GND | VIN (also the tab) | VOUT | Holtek HT78xx Rev 1.51, pin description (confirmed) |
| IRLML6344 SOT-23 | Gate | Source | Drain | Infineon datasheet (handoff 09) |
| 1N5819W SOD-123 | Cathode (band) | Anode | — | KiCad `D_SOD-123`: pad 1 = K. **Check the delivered part's band before soldering.** |
| 220 µF polymer | + | − | — | KiCad `CP_Elec`: pad 1 = + (marked "+" on the assembly drawing) |

## Wire pads
See `README.md` for the pad map.
- **Sizes** meet or exceed handoff 03: power 3.0 × 2.5, motor 2.5 × 2.5, ESP 2.0 × 2.0 mm. A+ and B+ are 2.5 × 7.0 mm for up to three wires.
- **Construction:** every pad is SMD (F.Cu + F.Mask opening), has no hole, and carries a 0.6 mm local clearance (DEC-012).

## Electrical
| Net / class | Track width | Notes |
|---|---|---|
| SWITCHED_SYSTEM+ trunk | 1.5 mm | left-edge strip and top-edge line |
| GND bus | 1.5 mm | bottom edge; per-channel teeth 1.0 mm |
| HAPTIC_3V3_A / _B | 1.0 mm | rail buses and every rail stub |
| M1-…M6- | 0.8 mm | |
| PWM / GATE | 0.5 mm | |

- **Clearances:**
  - 0.4 mm minimum (`.kicad_pro`).
  - Pour to other nets 0.5 mm.
  - Wire pads 0.6 mm.
  - Copper to board edge 1.0 mm (single row and compact); 0.5 mm in the mini and tight variants (DEC-017).
- **DRC custom rules (`EAS_ERM_Driver.kicad_dru`):**
  - No vias, no holes, nothing on B.Cu.
  - Power, rail and motor tracks ≥ 0.8 mm.
  - Signal tracks 0.4–0.5 mm.
- **Switching loop per channel:** the local 100 nF sits between the rail bus directly above the diode and the channel's GND tooth. The high-di/dt loop (D → rail → C15..C20 → GND tooth → Q source → Q drain → D) is therefore about 2.4 × 5 mm ≈ 12 mm². This is an estimate from the layout coordinates and has not been measured.

## Thermal (regulators)
- **Load (handoff 04):** 3 motors × 90 mA = 270 mA per regulator, which dissipates 0.243 W at 4.2 V in; 360 mA start current (transient) dissipates 0.324 W.
- **Datasheet rise:** with θJA = 200 °C/W (Holtek, "no ambient airflow, no heat sink"), the junction rise would be 49 °C and 65 °C (transient).
- **Heat-spreading copper:** the tab (VIN) is solidly connected to SWITCHED_SYSTEM+ copper.
  - Single row: a zone of 38.5 mm² at U1 and 20.9 mm² at U2, plus the 1.5 mm SW line joined to both tabs.
  - Compact: 28.3 mm² and 28.4 mm², plus the 1.5 mm left-edge SW strip joined to both tabs.
  - Mini: 20.1 mm² each; tight: 19.9 mm² each. Both have the 1.0 mm left-edge SW strip. These have the least copper, so check the temperature first on them.
- This copper is there to lower θJA; the three-motor rail test (handoff 07 §6) records the regulator temperature.

## Fabrication (toner transfer)
**Paper:** Robocraze A4 PCB toner transfer paper, laser printers only, iron or laminator. Its listing claims "almost lossless transfer for upto 0.5mm tracks" and states no gap figure. Every variant's narrowest **trace** is 0.5 mm, at that claim. The **gaps** are 0.4 mm (single row, compact, mini) and 0.3 mm (tight, DEC-019). The tight variant is therefore being etched below the paper's stated capability, deliberately.

**Extra checks when etching the tight variant (DEC-019):**
1. Print with toner saving OFF and density at maximum; the darkest possible toner is what survives the transfer.
2. Before ironing, inspect the printout under a loupe or phone macro: every gap must be a clean white line, with no toner bridging.
3. After transfer and before etching, inspect again and repair any bridged gap by scraping the toner with a scalpel.
4. After etching, inspect every gap, especially around the SOT-23 pins, the SOT-89 pins and between the rail buses and the GND pour. Clear any remaining copper with a scalpel.
5. Before soldering, measure resistance: SWITCHED_SYSTEM+ to GND, each rail to GND, and HAPTIC_3V3_A to HAPTIC_3V3_B. All must be open circuits.
6. If bridges cannot be cleaned up, fall back to the mini variant (0.4 mm gaps), which is built and verified.

1. Print at **100 % scale** on a laser printer, with "fit to page" / "scale to fit" disabled. Both PDFs are already mirrored for face-down transfer.
   - Tight variant: `EAS_ERM_Driver_tight/output/EAS_ERM_Driver_tight_TONER_PANEL_6up_MIRRORED_print_1to1.pdf` — six copies per A4 sheet, 30 mm apart, with a printed ruler (DEC-020), so one smeared transfer costs a copy rather than the sheet. The 30 mm pitch leaves about 15 mm of paper on each side of a cut-out copy, to fold around the copper clad while ironing. Other grids: `python3 scripts/panel.py <cols> <rows> [gap mm]`, which verifies its own output at 1:1.
   - Mini variant (the 0.4 mm-gap fallback): the same sheet at `EAS_ERM_Driver_mini/output/EAS_ERM_Driver_mini_TONER_PANEL_6up_MIRRORED_print_1to1.pdf`.
   - Any variant, one copy: `output/*_TONER_F.Cu_MIRRORED_print_1to1.pdf`.
2. **Check the print scale before ironing.** On either panel sheet, measure the horizontal ruler from tick 0 to tick 100: it must read exactly 100.0 mm centre to centre, and the vertical ruler 50.0 mm. If either is off, the printer rescaled the page; fix the print dialog and reprint. Then measure a board outline: **36.07 × 27.60 mm** (tight), **82.75 × 19.0 mm** (single row), **44.70 × 32.60 mm** (compact) or **38.91 × 29.28 mm** (mini). The PDFs were measured at those sizes in software (TEST-005, TEST-009, TEST-010, TEST-012).
   Cut the ruler strip off the sheet before ironing, then cut the copies apart through the middle of the 30 mm gaps so each keeps its paper border.
3. Before ironing, lay the real parts on the unmirrored fit-check print (TEST-007). Check especially the 220 µF V-chip base, the SOT-89 and the SOD-123 band orientation.
4. Transfer, etch, and clean the toner off. No drilling is required.
5. Cut along the outline line (hacksaw or shear). Copper stays 1 mm inside the line on the single row and compact, and 0.5 mm on the mini and tight, so on those two cut on the line or slightly outside it, and never more than 0.3 mm inside.
6. Inspect under magnification for bridges. Look first at the SOT-23 pins, the SOT-89 pins and every gap between the rail buses and the GND pour. Then measure resistance: SWITCHED_SYSTEM+ to GND, each rail to GND, and HAPTIC_3V3_A to HAPTIC_3V3_B (handoff 07 §1–2).

## Assembly
Place parts from `output/*_placement_3to1.pdf` (3× enlarged). Every part body shows its reference and value, e.g. `C15`/`100n`, because the 1206 capacitors are unmarked. Keep `output/*_assembly_3to1.pdf` beside it when a pin number is needed (U1, U2, Q1–Q6): it is the same drawing with the pads and pin numbers sketched, at the cost of the digits overlapping some labels.
1. **1206 parts first (R1–R12, C1–C6, C9–C20).** Keep the values separate: 1 µF (C1, C2, C5, C6), 10 µF (C3, C4), 10 nF (C9–C14), 100 nF (C15–C20).
2. **D1–D6.** Cathode band toward the top rail bus (pad 1 at the top).
3. **Q1–Q6.** Drain (single pin) faces left, toward the M- pad.
4. **U1, U2.** Pre-heat: the tab joins a solid VIN copper area with no thermal relief. U1's tab faces the left edge; U2's tab faces the top edge (single row) or the left edge (compact).
5. **C7, C8 (220 µF).** "+" pad as marked, on the right: toward A+ (C7), and toward the rail B riser (single row) or B+ (compact) for C8.
6. **Tight variant:** parts sit about 0.2 mm apart and gaps are 0.3 mm. Inspect the etched board for bridges before soldering, solder one part at a time, and re-check neighbours after each joint.
7. **Mini and tight variants:** the standard lands are shorter. Use a fine tip and thin solder, and check each joint for bridges under magnification.
8. **Wires last.** Tin each pad, solder the wire, then glue or tie the wire down. Insulate any wire that crosses bare copper.

## Bench test wiring (XIAO ESP32-S3)
The handoff leaves the GPIO numbers to firmware (`08_DECISION_LOG.md`), so this mapping is a
recommendation, not a spec. It matches `firmware/erm_channel_test/erm_channel_test.ino`.

| Board pad | XIAO pad | GPIO | Note |
|---|---|---|---|
| PWM1 | D0 | 1 | |
| PWM2 | D1 | 2 | |
| PWM3 | D2 | 3 | strapping pin (JTAG source select); safe here, the only DC load is the 100 k gate pulldown |
| PWM4 | D3 | 4 | |
| PWM5 | D4 | 5 | |
| PWM6 | D5 | 6 | |
| GND or EGND | GND | — | **mandatory**: the logic reference |
| ESP+ | BAT+ | — | final battery build only; leave open while the XIAO is on USB |
| PWR+ | — | — | current-limited supply at 3.7–4.0 V, or the switched battery |

D6/D7 (GPIO43/44) are the UART pins and are avoided. 20 kHz PWM, 8-bit duty.

**Firmware and dashboard.** `firmware/erm_channel_test/erm_channel_test.ino` takes one command per
line at 115200: `<ch> <percent>` to set a channel, `1`-`6` to ramp one for the bring-up test, `a`
for all six, `0` for all off, `k` as a silent keepalive, `?` to print both the commanded duty and
`ledcRead()` of every pin (they can disagree; the second is what drives a motor).

**Failsafe.** The firmware switches every channel off after 3 s without a command, provided anything
is driving. LEDC holds its last duty with no CPU involvement, so without this a dropped USB link
leaves a motor running until the battery is flat. The dashboard sends `k` once a second, so an open
window keeps motors alive and a closed or crashed one stops them. **A dashboard without the
keepalive will see motors stop after 3 s** - run the current `erm_dashboard.py`. `firmware/dashboard/erm_dashboard.py` is a
tkinter window with one slider per motor that drives the same protocol
(`python3 erm_dashboard.py [port] [--offline] [--selftest]`). The XIAO re-enumerates on every
reset and usually returns on a different `/dev/ttyACM` number, so the no-argument form with the
port dropdown is the practical one. While the link is down the status line turns red and says
NOT CONNECTED: a channel holds its last duty until a new command actually reaches the board.

**Backfeed warning:** with ESP+ connected and the XIAO's USB plugged in, the XIAO's own charger
can push current into `SWITCHED_SYSTEM+` (`README.md`, observation, unverified). During bench
testing, power the board from the supply and the XIAO from USB, joined only at ground.

## Known hardware limitations
- **Bare copper:** no solder mask. Consider conformal coating or Kapton tape after bring-up.
- **Operating window:** the HT7833 regulates down to a cell voltage of about 3.3 V + dropout (360 mV typ at 500 mA per datasheet; less at 270 mA) (handoff 07 §8).
- **Outside this board (observation, unverified):** the XIAO ESP32-S3 has its own charger on its BAT pads. With the XIAO's USB plugged in and the master switch ON, it may push charge current into SWITCHED_SYSTEM+. Check against the XIAO schematic; this PCB is unaffected.
