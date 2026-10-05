# Components, Footprints, and Assembly

## Purchased/available design parts
The user's Robu invoice confirms the following stock relevant to this board: 10× IRLML6344, 22× 1N5819W, 40× 100 ohm 1206 resistors, 20× 100 kOhm 1206 resistors, 20× 10 uF 1206 capacitors, 20× 10 nF 1206 capacitors, 8× 220 uF 6.3 V polymer capacitors, 10× 1 uF 50 V 1206 capacitors, plus the single-sided 20×30 cm 1.5 mm copper-clad board. The same invoice also confirms the XIAO ESP32-S3 stock, but the module is separate from this PCB. 

## Components to populate on this PCB
| Ref | Value / Part | Qty | Footprint target | Notes |
|---|---|---:|---|---|
| U1,U2 | HT7833 3.3 V SOT-89 | 2 | `Package_TO_SOT_SMD:SOT-89-3` or verified manufacturer-compatible custom footprint | Pin 1 GND, 2 VIN, 3 VOUT |
| Q1-Q6 | IRLML6344 | 6 | `Package_TO_SOT_SMD:SOT-23` | G=1, S=2, D=3 |
| D1-D6 | 1N5819W | 6 | `Diode_SMD:D_SOD-123` | Cathode to motor+, anode to drain |
| R1-R6 | 100 ohm | 6 | `Resistor_SMD:R_1206_3216Metric` | Gate series |
| R7-R12 | 100 kOhm | 6 | `Resistor_SMD:R_1206_3216Metric` | Gate pulldown |
| C1,C2 | 1 uF | 2 | `Capacitor_SMD:C_1206_3216Metric` | Mandatory regulator input |
| C3,C4 | 10 uF | 2 | `Capacitor_SMD:C_1206_3216Metric` | Input bulk |
| C5,C6 | 1 uF | 2 | `Capacitor_SMD:C_1206_3216Metric` | Mandatory regulator output |
| C7,C8 | 220 uF polymer | 2 | Exact verified pad/land pattern for purchased D5xL6.1 part | Output bulk |
| C9-C14 | 10 nF | 6 | `Capacitor_SMD:C_1206_3216Metric` | EMI suppression directly across motor |

## External solder pads
Create project-local SMD solder-pad footprints. They are not connectors.

Recommended starting sizes:
- Power input pads: >= 3.0 mm x 2.5 mm copper.
- Motor pads: >= 2.5 mm x 2.5 mm copper.
- ESP32 interface pads: >= 2.0 mm x 2.0 mm copper.

The agent may increase pad size when needed for mechanical strength or hand soldering, but should not shrink below these targets without a documented reason.

No holes are permitted in these pads.

## Component orientation
Orient SOT-89, SOT-23, SOD-123, 1206 capacitors and resistors so that the board remains easy to hand solder and inspection is straightforward. Exact orientation is free for the optimizer.

## Marking
Silkscreen is optional for toner-transfer fabrication but, if a silkscreen-equivalent layer is plotted, use very short labels:
- `PWR`, `GND`
- `PWM1` ... `PWM6`
- `A+`, `B+`
- `M1-` ... `M6-`
- `U1`, `U2`, `Q1` ... `Q6`

On an etch-only board, prefer copper legends or simple etched labels if desired. Avoid wasting area on long text.
