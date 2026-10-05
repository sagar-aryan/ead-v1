# Implementation

## Build pipeline

### Objective
Produce the complete KiCad 10 project (schematic, PCB, libraries, rules) reproducibly from one description of the circuit, then verify and export it.

### Design
```text
scripts/build.py ──► EAS.kicad_sym, EAS.pretty/, lib tables
                 ──► EAS_ERM_Driver.kicad_sch   (S-expression text)
                 ──► EAS_ERM_Driver.kicad_pcb   (pcbnew Python API)
                 ──► EAS_ERM_Driver.kicad_pro / .kicad_dru (rules, net classes)
scripts/export.sh ─► build.py → ERC → DRC(+parity, refill zones, save) → check.py → PDFs/Gerbers/BOM/PNGs
scripts/check.py ──► handoff verification items 3–15 + connection summary (exit 1 on failure)
```
The two outputs come from the same `PARTS` list, and `kicad-cli pcb drc --schematic-parity` checks them against each other independently. A mismatch in references, values, footprints or pad nets would fail the export.

### Important files
- `EAS_ERM_Driver/scripts/build.py`: the single source of truth. Edit this, not the KiCad files.
- `EAS_ERM_Driver/scripts/check.py`: the verification asserts.
- `EAS_ERM_Driver/scripts/export.sh`: the full pipeline.
- `EAS_ERM_Driver/EAS.pretty/`: generated project footprints (SOT-89 variant, wire pads).
- `EAS_ERM_Driver/EAS.kicad_sym`: the generated `HT7833` symbol.

### Important functions (`build.py`)
- `add(ref, sym, value, fp, pins, desc, ds)`: registers one part with its pin→net map. The netlist lives here: regulators, then `for n in 1..6` channels, then `WIREPADS`.
- `write_libs()`:
  - Writes the `HT7833` symbol, a copy of KiCad `HT75xx-1-SOT89` with the identical 1 GND / 2 VIN / 3 VOUT pinout, renamed and re-described.
  - Writes `SOT-89-3_Handsoldering_0.4mm` (DEC-010) and the four `WirePad_*` footprints.
- `write_schematic()`:
  - Embeds the library symbols and places every part in functional groups on A3.
  - Draws a 2.54 mm wire stub and a global label on every pin (DEC-014), and adds PWR_FLAGs on SWITCHED_SYSTEM+ and GND.
  - UUIDs are deterministic (`uuid5`), so re-running produces identical files.
- `Board.place/pc/trk/zone`: thin wrappers over `pcbnew`. `pc()` returns the real pad centre after rotation, and every track is routed from these points, never from hand-computed pad offsets.
- `channel(bd, n, xq)`: places and routes one channel cell; see the geometry below.
- `build_pcb()`: power corner and U1 block, channels 1–3, U2 block, channels 4–6, buses, zones, pour keepouts, fab labels, outline.

## Layout geometry
All values are in mm, board-local, with the origin at the top-left of the outline (the outline sits at (100, 80) on the drawing sheet).

| Constant | Value | Meaning |
|---|---|---|
| `Y_SW` | 1.75 | SWITCHED_SYSTEM+ line (1.5 wide) |
| `Y_RAIL` | 3.40 | rail buses (1.0 wide) |
| `Y_CAP`, `Y_DIO` | 5.625, 5.50 | vertical 1206 / SOD-123 hanging from the rail bus (top pad overlaps the bus) |
| `Y_Q` | 9.95 | SOT-23 centre, orientation 180 (drain left, source top-right, gate bottom-right) |
| `Y_RG`, `Y_RPD` | 12.92, 14.24 | gate resistor horizontal, pulldown vertical under the gate |
| `Y_PWM` | 14.9 | PWM pad; the GND bus sits 0.6 mm below it at `Y_GB` = 17.25 |
| `P` | 9.45 | channel pitch; the clearance limit is 9.40 |

**Cell template (`channel()`), relative to the MOSFET centre `xq`:**
- **Row hanging from the rail bus:** Cs (10 nF) at −3.89, D at −1.5, Cd (100 nF) at +0.89.
- **Motor-minus node:** the M- pad at −3.65 connects to the drain and to Cs's lower pad; D's anode drops straight to the drain.
- **GND tooth** at +3.4: Cd's lower pad → source → tooth → GND bus.
- **Gate network:** Rg at −2.17 (horizontal) and Rpd at +1.5 (vertical). The PWM pad sits under Rg's left pad.

**Buses** are drawn as polylines with a vertex at every tap x-coordinate, so each stub ends on a segment endpoint and connectivity never relies on T-junctions.

**Zones:**
- GND pour over the whole board (priority 0, thermal spokes).
- Solid SWITCHED_SYSTEM+ heat spreaders at U1 and U2 (priority 1, no thermal relief).
- Copper-pour keepout rule areas over every other part's courtyard (DEC-011).
- Zones are filled by `kicad-cli pcb drc --refill-zones --save-board`, so they are filled with the project's own rule settings.

## Compact variant (`EAS_ERM_Driver_compact/scripts/build.py`)
A copy of the single-row script with only the PCB layout section changed (DEC-016); the netlist model, library and schematic code are identical.
- **Row offsets:** `DY_A`/`DY_B` shift the single-row template (row A −1.9 mm, row B +13.6 mm), so the verified geometry is reused unchanged.
- **`reg_block(bd, dy, …, corner)`:** the single-row U1 block as a function. It is used for both regulators; `corner=True` adds the power pads.
- **`channel(bd, n, xq, dy)`:** the same cell as before, plus the row offset.
- **`XLINK`:** the right-edge GND link joining the two row GND buses.

## Mini variant (`EAS_ERM_Driver_mini/scripts/build.py`)
A copy of the compact script with standard footprints and the relaxed rules (DEC-017).
- **Cell and block geometry:** re-derived for the smaller lands, keeping the same topology as the compact.
- **Cell** (offsets from the MOSFET centre `xq` and the row's rail-bus centre `yr`):
  - Q at `yr + 6.24`.
  - D, Cs and Cd hanging from the bus at `xq − 0.9375`, `xq − 3.3275` and `xq + 1.4525`.
  - Rg horizontal at `xq − 2.7` and Rpd vertical at `xq + 0.8`.
  - 0.8 mm GND tooth at `xq + 2.525`; GND bus at `yr + 12.94`.
  - Pitch 8.1 mm, limited by the tooth-to-next-gate-resistor gap of 0.45 mm.
- **`reg_block(bd, yr, …)`:**
  - U at x 3.5 (tab on the 1.0 mm SW strip), Cout at x 7.6, the + wire pad at x 10.15.
  - The 220 µF sits under them at x 9.4, with the input caps under the tab.
  - The power corner J1/J3/J2/J4 is along the bottom-left.

## Tight variant (`EAS_ERM_Driver_tight/scripts/build.py`)
A copy of the mini script with 0.3 mm clearances and `tight_fp()` (DEC-018).
- **`tight_fp()`:** copies a KiCad footprint and replaces its F.CrtYd graphics with one rectangle equal to the body and pad extents + 0.1 mm.
- **Geometry:** cell pitch 7.45 mm, row height 13.15 mm; the outline is rounded up (PROB-006).

## Edge cases and limitations
- **Re-running `build.py` overwrites GUI edits** to the schematic and PCB. To take over by hand, stop using `build.py` and keep only `check.py` and `export.sh` minus the build step.
- **Nets named on the handoff's `M1+`…`M6+` don't exist:** those points are the rail nets `HAPTIC_3V3_A/B`.
- **The schematic is label-based** (every pin carries a labelled stub), not a wired drawing. It is correct (ERC clean, parity clean) but less pretty than a hand-drawn schematic.
- **pcbnew warning:** it prints a harmless `PROPERTY_ENUM ... No enum choices defined` assert on stderr; `export.sh` filters it out.

## Verification
See `testing.md`: TEST-001 to TEST-006 were executed; TEST-007 and TEST-008 are pending hardware.
