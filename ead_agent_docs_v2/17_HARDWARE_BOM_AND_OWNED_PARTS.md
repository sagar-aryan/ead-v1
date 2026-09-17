# 17 — V1 Hardware BOM / Owned Parts

## Required active hardware
| Item | V1 quantity | Notes |
|---|---:|---|
| Seeed XIAO ESP32-S3 | 1 | Main controller; 2nd board remains spare |
| MPU6050 breakout | 2 | Foot + shank |
| ERM coin vibration motor | 6 | One per haptic channel; user has 12, 6 remain spare |
| IRLML6344 SOT-23 N-MOSFET | 6 | One per motor; user has 10 |
| 1N5819W SOD-123 Schottky | 6 | One per motor; user has 22 |
| 100 Ω 1206 resistor | 6 | One gate resistor/channel; user has 40 |
| 100 kΩ 1206 resistor | 6 | One gate pulldown/channel; user has 20 |
| 100 nF 1206 capacitor | 2 minimum | One at each MPU; more may be used for local decoupling |
| 10 nF 1206 capacitor | 0 default | DNP motor suppression footprint; optional after EMI test |
| 10 µF 1206 capacitor | 2+ | General local rail decoupling/spares |
| 1 µF 1206 capacitor | 4 minimum | HT7833 input/output, 2 per regulator |
| 220 µF polymer | 2 | One per haptic rail; user has 8 |
| HT7833 | 2 | One regulator per 3-motor group |
| 1S 3.7 V ~2000 mAh battery | 1 | Physical power source; firmware has no battery monitoring |
| MCP73833 charger module | 1 | Hardware-side charging; firmware does not control it |

## Wiring material
- Thin flexible wire from existing JST-SH harnesses may be cut free and soldered directly to the PCB.
- Main battery conductors should use heavier flexible wire than motor/signal conductors.
- No motor connectors or XIAO headers are part of the final PCB design.

## Parts intentionally NOT used in V1
- BNO055.
- BNO086.
- DRV2605L.
- TCA9548A.
- FSRs.
- AMS1117-3.3.
- USB boost/charger module.
- Battery fuel gauge.
- Battery ADC.
- Switch-status input.

The user's latest Robu order confirms IRLML6344, 1N5819W, 100 Ω/100 kΩ 1206 resistors, capacitors, two XIAO ESP32-S3 boards, 220 µF polymer capacitors, 1 µF capacitors and the copper-clad PCB. fileciteturn8file0L36-L65 fileciteturn8file7L502-L525
The Robocraze cart contains two XIAO boards, 12 coin motors and three BNO086s; only the two XIAOs/6 motors required for V1 are active in this specification, with BNO086 reserved for future hardware. fileciteturn8file4L338-L346
