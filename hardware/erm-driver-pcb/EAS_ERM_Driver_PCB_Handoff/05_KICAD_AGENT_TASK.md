# READY-TO-PASTE KICAD AGENT TASK

Design the **EAS V1 6-channel ERM haptic driver PCB** described in this handoff. Treat this document as an implementation contract, not as a starting point for changing the electrical architecture.

## Objective
Create a complete KiCad project for a compact, single-sided, toner-transferable, SMD-only ERM driver PCB. The finished PCB will drive six 3.0 V nominal ERM vibration motors from a single-cell 3.7 V LiPo system using two HT7833 3.3 V SOT-89 regulators and six independent IRLML6344 low-side MOSFET channels.

## Hard requirements
1. One copper layer only.
2. No vias.
3. No drilled holes.
4. No through-hole parts.
5. No connectors.
6. No onboard ON/OFF switch; the master switch is external and located between the charger LOAD+ output and this board's `SWITCHED_SYSTEM+` input.
7. No charger circuit on this PCB.
8. No battery monitoring.
9. No ESP32 module on this PCB.
10. No BNO086 or sensor circuitry.
11. No FSR circuitry.
12. No motor-driver IC.
13. Use direct SMD solder pads for all external wires.
14. Minimize total board area while respecting power, thermal, EMI, clearance, and hand-solder requirements.
15. 0 ohm/insulated crossover jumper is allowed only if unavoidable; prefer zero jumpers.
16. No test points.

## System power interface
The board receives switched charger/load-side power, not the raw LiPo BAT terminals.

External system:
`MCP73833 LOAD+ -> external master switch -> board SWITCHED_SYSTEM+`
`MCP73833 LOAD-/GND -> board GND`

The battery remains permanently wired to the MCP73833 BAT+ / BAT- terminals outside this PCB.

The board therefore has two power input solder pads:
- `SWITCHED_SYSTEM+`
- `GND`

Do not label these BAT+ / BAT-.

## ESP32 interface
The board exposes eight SMD solder pads to the separate XIAO ESP32-S3:
1. `ESP_SW_BAT+` (connected internally to `SWITCHED_SYSTEM+`)
2. `GND`
3. `PWM1`
4. `PWM2`
5. `PWM3`
6. `PWM4`
7. `PWM5`
8. `PWM6`

The ESP32 is powered from the switched raw single-cell battery/load voltage through `ESP_SW_BAT+`. Do not use or generate a 3.3 V ESP32 supply from the haptic rails.

## Regulator topology
Use exactly two HT7833 3.3 V SOT-89 LDOs:

`U1`: VIN=`SWITCHED_SYSTEM+`, GND=`GND`, VOUT=`HAPTIC_3V3_A`.
`U2`: VIN=`SWITCHED_SYSTEM+`, GND=`GND`, VOUT=`HAPTIC_3V3_B`.

**Never connect `HAPTIC_3V3_A` and `HAPTIC_3V3_B` together.**

Populate for each regulator:
- 1 uF ceramic from VIN to GND adjacent to pins.
- 10 uF ceramic from VIN to GND nearby.
- 1 uF ceramic from VOUT to GND adjacent to pins.
- 220 uF polymer from VOUT to GND nearby.

Use the manufacturer's SOT-89 pinout: pin 1 = GND, pin 2 = VIN, pin 3 = VOUT.

## Motor topology
Six identical channels.

For each channel n:
- ESP32 `PWMn` -> 100 ohm -> MOSFET gate.
- Gate -> 100 kOhm -> GND.
- IRLML6344 source -> GND.
- IRLML6344 drain -> motor negative `Mn-`.
- Motor positive `Mn+` -> the assigned 3.3 V rail.
- Flyback diode 1N5819W cathode -> `Mn+`.
- Flyback diode anode -> `Mn-` / MOSFET drain.
- 10 nF ceramic directly across `Mn+` and `Mn-`.

Assign:
- M1, M2, M3 to `HAPTIC_3V3_A`.
- M4, M5, M6 to `HAPTIC_3V3_B`.

Physical motor ordering may be optimized freely; electrical net names must remain exact.

## Component footprints
Use or create verified footprints:
- HT7833: SOT-89-3.
- IRLML6344: SOT-23.
- 1N5819W: SOD-123.
- Resistors: 1206 metric.
- Capacitors: 1206 metric.
- 220 uF polymer: verify exact purchased component land pattern from its part marking/datasheet.
- External wire connections: project-local **SMD solder-pad-only** footprints with no hole.

## PCB pad groups
### Power input
`SWITCHED_SYSTEM+`, `GND`

### ESP32
`ESP_SW_BAT+`, `GND`, `PWM1`, `PWM2`, `PWM3`, `PWM4`, `PWM5`, `PWM6`

### Motors
`MOTOR_A+`, `M1-`, `M2-`, `M3-`, `MOTOR_B+`, `M4-`, `M5-`, `M6-`

`MOTOR_A+` is the exposed common positive pad for M1-M3 rail A. `MOTOR_B+` is the exposed common positive pad for M4-M6 rail B. Do not combine A+ and B+.

## Layout requirements
- Use one copper layer only.
- No vias.
- Main switched-power and ground trunks >= 1.5 mm where space permits.
- Regulator input/output trunks >= 1.0 mm.
- Individual motor branches >= 0.8 mm.
- PWM/gate traces about 0.30-0.50 mm.
- Keep flyback diode and 10 nF capacitor close to each motor switching area.
- Keep regulator input/output capacitors at the regulator pins.
- Give each SOT-89 a generous copper heat-spreading area.
- Keep motor high-current returns away from the narrow ESP32 interface return route.
- Minimize long parallel routing between PWM and motor traces.
- Keep motor switching loop area minimal.
- Optimize board dimensions after routing rather than before routing.

## Manufacturer constraints that must not be ignored
The Holtek HT78xx datasheet specifies the 3-pin SOT-89 pinout as GND=1, VIN=2, VOUT=3, and specifies the family at 500 mA continuous output with current limiting and thermal shutdown. It specifies 1 uF input/output capacitance in its electrical-characteristic test conditions and SOT-89 thermal resistance of 200 degC/W with 0.50 W power dissipation at Ta=25 degC. Use these values and the manufacturer's layout guidance as the basis for placement and thermal copper.

The Infineon IRLML6344 is an N-channel SOT-23 MOSFET with pin 1 Gate, pin 2 Source, pin 3 Drain. Its 30 V rating and low RDS(on) provide ample electrical margin for the low-voltage motor channels.

The 1N5819W is a 40 V Schottky diode in SOD-123; verify the exact purchased vendor marking/pin orientation before final plot.

## Required verification
Before declaring the PCB complete:
1. Run KiCad ERC; resolve all genuine electrical errors.
2. Run KiCad DRC; resolve all rule violations.
3. Verify there is exactly one copper layer containing all routed copper.
4. Verify no vias and no holes exist.
5. Verify both HT7833 outputs are electrically isolated.
6. Verify all six MOSFET sources reach GND.
7. Verify each diode polarity.
8. Verify each motor suppression capacitor is across its correct motor pair.
9. Verify all six PWM inputs go through 100 ohm resistors to the correct gates.
10. Verify every gate has a 100 kOhm pulldown to GND.
11. Verify `ESP_SW_BAT+` is the switched battery/load rail and not a regulated 3.3 V net.
12. Verify no raw charger BAT net enters the PCB.
13. Verify motor rail current paths and ground returns are suitably wide.
14. Verify the purchased 220 uF polymer footprints and polarity.
15. Produce a human-readable connection summary and BOM.

## Deliverables
Return:
- `.kicad_pro`
- `.kicad_sch`
- `.kicad_pcb`
- project-local footprints/libraries if created
- BOM
- ERC result
- DRC result
- Gerber/plot output
- PDF schematic
- PDF or image plot of the PCB top copper and assembly view
- short README explaining the final pad map and power rails

Do not leave major design choices to the user. Resolve placement and routing decisions yourself using the requirements above.
