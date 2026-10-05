# Source Notes and Engineering References

## User-provided inventory
The Robu invoice dated 13/09/2026 confirms:
- IRLML6344TRPBF, qty 10.
- 1N5819W, qty 22.
- 100 ohm 1206 resistors, qty 40.
- 100 kOhm 1206 resistors, qty 20.
- 10 uF 1206 capacitors, qty 20.
- 10 nF 1206 capacitors, qty 20.
- XIAO ESP32-S3, qty 2 (not mounted on this PCB).
- 220 uF 6.3 V polymer capacitors, qty 8.
- JST-SH wire assemblies, not used as connectors on this PCB.
- single-sided 20x30 cm, 1.5 mm copper-clad PCB.
- 1 uF 1206 capacitors, qty 10.

The same order set also documents the SmartElex MCP73833 charger and 3.7 V 2000 mAh single-cell battery, which remain external to this driver PCB.

## Manufacturer references
### Holtek HT78xx / HT7833
Holtek HT78xx Rev. 1.51, dated 04 Dec 2025:
https://www.holtek.com/webapi/116711/HT78xxv151.pdf

Relevant confirmed facts:
- HT7833 is the 3.3 V member.
- 500 mA continuous family rating.
- 3-pin SOT-89 package available.
- SOT-89 pinout: 1 GND, 2 VIN, 3 VOUT.
- thermal resistance theta_JA = 200 degC/W in the stated no-airflow/no-heatsink condition.
- power dissipation listed as 0.50 W at Ta=25 degC.
- nominal dropout specification for the 3.0-5.0 V output variants is 360 mV typical, 500 mV max, at 500 mA.
- current limiting and thermal shutdown are included.
- 1 uF input/output capacitance is used in the stated electrical-characteristic test conditions.

### Infineon IRLML6344
https://www.infineon.com/assets/row/public/documents/24/49/infineon-irlml6344-datasheet-en.pdf

Relevant confirmed facts:
- N-channel MOSFET.
- SOT-23 package.
- Pin 1 Gate, pin 2 Source, pin 3 Drain.
- 30 V VDS rating.
- Low RDS(on), including a maximum value of 37 mOhm at VGS=2.5 V in the datasheet.

### 1N5819W
The purchased part is a KEXIN 1N5819W in SOD-123, 40 V Schottky.
The exact purchased vendor's marking/polarity should be checked against the delivered part before final PCB assembly.
