# Final V1 BOM — 6-Channel ERM Driver PCB

| Ref | Part | Qty populated | User stock | Notes |
|---|---|---:|---:|---|
| U1,U2 | Holtek HT7833, 3.3 V, SOT-89 | 2 | ~10 available | Two independent rails |
| Q1-Q6 | Infineon IRLML6344, SOT-23 | 6 | 10 | Low-side motor switches |
| D1-D6 | 1N5819W, SOD-123 Schottky | 6 | 22 | Flyback, one per motor |
| R1-R6 | 100 ohm, 1206 | 6 | 40 | Gate series resistors |
| R7-R12 | 100 kOhm, 1206 | 6 | 20 | Gate pulldowns |
| C1,C2 | 1 uF, 1206, X7R | 2 | 10 | HT7833 input minimum |
| C3,C4 | 10 uF, 1206, X7R | 2 | 20 | Input bulk |
| C5,C6 | 1 uF, 1206, X7R | 2 | 10 | HT7833 output minimum |
| C7,C8 | 220 uF, 6.3 V polymer | 2 | 8 | Output bulk, polarity required |
| C9-C14 | 10 nF, 1206, X7R | 6 | 20 | Motor EMI suppression |
| — | SMD solder pads | 18? | Custom | 2 power + 8 ESP32 + 8 motor = 18 total pad sites |

## External connections
### Power input
2 pads:
- `SWITCHED_SYSTEM+`
- `GND`

### ESP32 interface
8 pads:
- `ESP_SW_BAT+`
- `GND`
- `PWM1` ... `PWM6`

### Motor interface
8 pads:
- `MOTOR_A+`
- `M1-`
- `M2-`
- `M3-`
- `MOTOR_B+`
- `M4-`
- `M5-`
- `M6-`

There are **18 external wire pad sites** because the ESP32 ground and power input are intentionally separate physical interfaces. The internal nets are common as specified.

## Not populated / not on board
- Charger components.
- Battery protection components.
- Switch.
- ESP32.
- IMUs.
- FSRs.
- 0 ohm jumper, unless routing proves one is absolutely necessary.
