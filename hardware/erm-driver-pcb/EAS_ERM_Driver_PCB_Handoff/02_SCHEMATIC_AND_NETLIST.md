# Schematic and Netlist Specification

## 1. Naming convention
Use these exact net names where practical:

### Power
- `SWITCHED_SYSTEM+`
- `GND`
- `HAPTIC_3V3_A`
- `HAPTIC_3V3_B`

### ESP32
- `ESP_SW_BAT+`
- `PWM1` ... `PWM6`

### Motors
- `M1+`, `M1-`
- `M2+`, `M2-`
- `M3+`, `M3-`
- `M4+`, `M4-`
- `M5+`, `M5-`
- `M6+`, `M6-`

## 2. Suggested reference designators
- `U1` = HT7833, 3.3 V, SOT-89, rail A
- `U2` = HT7833, 3.3 V, SOT-89, rail B
- `Q1`...`Q6` = IRLML6344, SOT-23
- `D1`...`D6` = 1N5819W, SOD-123
- `R1`...`R6` = 100 ohm gate resistors, 1206
- `R7`...`R12` = 100 kOhm gate pulldowns, 1206
- `C1`,`C2` = 1 uF regulator input capacitors, one per HT7833
- `C3`,`C4` = 10 uF regulator input bulk capacitors, one per HT7833
- `C5`,`C6` = 1 uF regulator output capacitors, one per HT7833
- `C7`,`C8` = 220 uF polymer output bulk capacitors, one per HT7833
- `C9`...`C14` = 10 nF motor suppression capacitors, one per ERM

External wire pads should use clear functional names rather than connector numbering.

## 3. Regulator channel A
`U1` HT7833 SOT-89:
- Pin 1 `GND` -> ground.
- Pin 2 `VIN` -> `SWITCHED_SYSTEM+`.
- Pin 3 `VOUT` -> `HAPTIC_3V3_A`.

Input capacitors:
- `C1` 1 uF from `SWITCHED_SYSTEM+` to `GND`, physically adjacent to U1 VIN/GND.
- `C3` 10 uF from `SWITCHED_SYSTEM+` to `GND`, adjacent to U1 input region.

Output capacitors:
- `C5` 1 uF from `HAPTIC_3V3_A` to `GND`, physically adjacent to U1 VOUT/GND.
- `C7` 220 uF polymer from `HAPTIC_3V3_A` to `GND`, close to U1 output/load region.

Motor positives:
- `M1+`, `M2+`, `M3+` all connect to `HAPTIC_3V3_A`.

## 4. Regulator channel B
`U2` HT7833 SOT-89:
- Pin 1 `GND` -> ground.
- Pin 2 `VIN` -> `SWITCHED_SYSTEM+`.
- Pin 3 `VOUT` -> `HAPTIC_3V3_B`.

Input capacitors:
- `C2` 1 uF from `SWITCHED_SYSTEM+` to `GND`.
- `C4` 10 uF from `SWITCHED_SYSTEM+` to `GND`.

Output capacitors:
- `C6` 1 uF from `HAPTIC_3V3_B` to `GND`.
- `C8` 220 uF polymer from `HAPTIC_3V3_B` to `GND`.

Motor positives:
- `M4+`, `M5+`, `M6+` all connect to `HAPTIC_3V3_B`.

## 5. Motor channel template
For each channel `n = 1...6`:

- `PWMn` -> `Rn_gate` 100 ohm -> `Qn.G`
- `Qn.G` -> `Rpulldown` 100 kOhm -> `GND`
- `Qn.S` -> `GND`
- `Qn.D` -> `Mn-`
- `Mn+` -> appropriate `HAPTIC_3V3_A` or `HAPTIC_3V3_B`
- `Dn` cathode -> `Mn+`
- `Dn` anode -> `Mn-`
- `Cn` 10 nF directly from `Mn+` to `Mn-`

Channel rail assignment:
- M1/M2/M3 -> rail A
- M4/M5/M6 -> rail B

The physical placement of M1-M6 may be rearranged for minimum board area, but electrical net names must not change.

## 6. ESP32 interface
Provide eight large SMD solder pads:
- `ESP_SW_BAT+` -> `SWITCHED_SYSTEM+`
- `GND` -> `GND`
- `PWM1` -> R1
- `PWM2` -> R2
- `PWM3` -> R3
- `PWM4` -> R4
- `PWM5` -> R5
- `PWM6` -> R6

The ESP32 supply is the switched raw single-cell rail. Do not connect the driver PCB to the XIAO 3.3 V pin.

## 7. Power input pads
Provide two SMD solder pads:
- `SWITCHED_SYSTEM+`
- `GND`

Never label these `BAT+`/`BAT-` because the external master switch is located before this PCB and raw charger BAT terminals are not the intended PCB input.
