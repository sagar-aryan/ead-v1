# EAS V1 — 6-Channel ERM Driver PCB Handoff

## Purpose
This package is the complete handoff for a KiCad agent to design the **standalone six-channel ERM haptic motor driver PCB** for the Error Augmentation System (EAS) wearable.

The agent's job is to produce a manufacturable **single-sided, toner-transfer-friendly, hand-solderable PCB** with no unresolved electrical architecture decisions.

## Scope
This PCB contains:
- Two HT7833 3.3 V SOT-89 linear regulators.
- Six independent low-side ERM motor channels.
- Six IRLML6344 N-channel MOSFETs.
- Six 1N5819W flyback Schottky diodes.
- One 100 ohm gate resistor and one 100 kOhm gate pulldown per channel.
- Local regulator capacitors and rail bulk capacitors.
- Direct solder pads for power, ESP32 interface, and six motors.

This PCB does **not** contain:
- ESP32-S3 module.
- BNO086 IMUs.
- MCP73833 charger.
- Battery protection/BMS.
- Battery ADC/fuel gauge.
- Master ON/OFF switch.
- FSRs.
- Motor-driver ICs such as DRV2605.
- Through-hole connectors.
- Test points.

## Critical system-level topology
The external master switch is **not on this PCB**.

```text
MCP73833 LOAD+ -> EXTERNAL MASTER SWITCH -> THIS PCB SWITCHED_SYSTEM+
MCP73833 LOAD- ---------------------------------> THIS PCB GND

LiPo -> MCP73833 BAT+/BAT- permanently

THIS PCB SWITCHED_SYSTEM+ -> HT7833-A -> M1/M2/M3 supply
                         -> HT7833-B -> M4/M5/M6 supply
                         -> ESP32 switched battery input
```

The switch must disconnect the entire wearable electronics system while allowing the charger to remain connected to the battery. The driver PCB itself therefore accepts the **switched load-side power**, not the raw battery/charger BAT connection.

## Manufacturing target
- Single copper layer only.
- No plated or unplated drilled vias.
- No through-hole components.
- No drilled mounting holes.
- SMD-only components and solder pads.
- 0 ohm resistor or insulated wire crossover is permitted only when needed; zero-jumper routing is preferred.
- Board should be the smallest practical rectangle after routing and thermal/clearance constraints are satisfied.
- Suitable for toner transfer, etching, drilling-free assembly, hand soldering, and 1206/SOT-23/SOD-123 parts.

## External interfaces
### Power input
- `SWITCHED_SYSTEM+`
- `GND`

### ESP32 interface
Eight solder pads:
1. `ESP_SW_BAT+`
2. `GND`
3. `PWM1`
4. `PWM2`
5. `PWM3`
6. `PWM4`
7. `PWM5`
8. `PWM6`

`ESP_SW_BAT+` is the switched raw single-cell battery/load voltage, not 3.3 V. The XIAO ESP32-S3 is powered separately from its battery input; the driver PCB only distributes the switched supply and PWM control.

### Motor interface
Eight solder pads:
- `MOTOR_A+` → positive terminals of three channels.
- `M1-`
- `M2-`
- `M3-`
- `MOTOR_B+` → positive terminals of the other three channels.
- `M4-`
- `M5-`
- `M6-`

The physical assignment of which three channels belong to rail A versus B may be arranged for minimum PCB area, but the net names must remain unambiguous.

## Required KiCad deliverables
The agent must produce:
- Complete KiCad schematic.
- Complete KiCad PCB layout.
- Project-local/custom footprints for all direct-wire solder pads.
- ERC report or documented ERC review.
- DRC report or documented DRC review.
- Gerber files for the single copper layer, solder mask/silkscreen only if the chosen fabrication process uses them, and board outline.
- Drill files only if the final design unexpectedly requires them; target is **zero holes**.
- BOM.
- Pick-and-place is optional and not required for hand assembly.
- PDF schematic and PCB plot for human inspection.

## Non-negotiable rule
The agent must not invent alternate regulators, alternate MOSFETs, alternate diode types, connectors, a two-layer PCB, or an onboard power switch. If a component cannot be routed/manufactured as specified, the agent must flag the exact conflict instead of silently changing the architecture.
