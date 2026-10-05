# Assembly, Bring-Up, and Validation Procedure

## 1. Visual inspection before power
Confirm:
- No copper bridges between the two 3.3 V rails.
- No solder bridges at MOSFET SOT-23 pins.
- All SOD-123 diode orientations match the schematic.
- HT7833 package orientation/pinout is correct.
- 220 uF polymer polarity is correct.
- No accidental short between `SWITCHED_SYSTEM+` and GND.
- Motor pads are clearly labeled.

## 2. Unpowered resistance checks
With no battery/USB attached:
- Measure `SWITCHED_SYSTEM+` to `GND`; verify there is no hard short.
- Measure `HAPTIC_3V3_A` to GND.
- Measure `HAPTIC_3V3_B` to GND.
- Verify `HAPTIC_3V3_A` to `HAPTIC_3V3_B` is not a direct short.
- Verify each gate is not shorted to source/drain.

## 3. Regulator-only bring-up
Disconnect all six motors initially.

Use a current-limited bench supply set to a single-cell-compatible voltage, initially around 3.7-4.0 V.

Power the board and verify:
- `SWITCHED_SYSTEM+` reaches both HT7833 inputs.
- `HAPTIC_3V3_A` is approximately 3.3 V while input is sufficiently above dropout and load is light.
- `HAPTIC_3V3_B` is approximately 3.3 V under the same condition.
- The regulators do not heat abnormally with no motor load.

## 4. One-motor test
Connect only one motor to one channel.

Drive the channel from the ESP32 at low duty cycle first.

Verify:
- motor is OFF at PWM=0;
- motor starts cleanly;
- motor stops when PWM is removed;
- MOSFET remains cool;
- no ESP32 reset occurs.

## 5. Channel-by-channel test
Repeat the one-motor test for all six channels.

Confirm channel mapping:
- PWM1 -> M1
- PWM2 -> M2
- PWM3 -> M3
- PWM4 -> M4
- PWM5 -> M5
- PWM6 -> M6

## 6. Three-motor rail test
Run M1-M3 simultaneously, then M4-M6 simultaneously.

Observe:
- regulator temperature;
- rail voltage;
- unexpected noise/resets;
- diode/capacitor temperature;
- PCB hot spots.

## 7. Six-motor test
Run all six motors with a controlled duty cycle. Test both steady vibration and the intended haptic PWM pattern.

If an oscilloscope is available, probe:
- `HAPTIC_3V3_A` near U1 output capacitor;
- `HAPTIC_3V3_B` near U2 output capacitor;
- one MOSFET drain switching node;
- ESP32 3.3 V rail separately;
- ground bounce between ESP32 ground and the driver board's high-current ground region.

The exact acceptable transient limits should be established during prototype validation rather than invented in the PCB documentation.

## 8. Battery discharge test
Because HT7833 is a linear 3.3 V regulator, regulation will eventually be lost as a single-cell LiPo approaches the 3.3 V output level. This is expected behavior for this V1.

Characterize motor vibration strength as battery voltage decreases and document the practical operating window. Do not treat low-battery rail droop as a PCB failure unless it occurs unexpectedly above the expected dropout region.

## 9. Charging test
Test with the master switch OFF:
- connect USB to MCP73833;
- verify battery charges normally;
- verify driver PCB and ESP32 remain OFF.

Then, only after the above works, test switch ON while charging. Treat simultaneous charge + load as a secondary condition and verify charger/load behavior empirically.
