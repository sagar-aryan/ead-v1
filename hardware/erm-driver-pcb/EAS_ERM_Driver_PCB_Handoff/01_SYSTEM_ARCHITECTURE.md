# System Architecture

## 1. Top-level power system

```text
                          USB 5 V
                            |
                            v
                     +--------------+
                     |   MCP73833   |
                     |    CHARGER   |
                     +----+----+----+
                          |    |
                        BAT  LOAD+
                          |    |
                          v    +----> external MASTER SWITCH ----+
                         LiPo                                      |
                                                                  v
                                                     SWITCHED_SYSTEM+
                                                                  |
                                      +---------------------------+------------------+
                                      |                                              |
                                      v                                              v
                                XIAO ESP32-S3                               ERM DRIVER PCB
                                switched battery                             +-----------+
                                      |                                      | HT7833 A  |
                                      | PWM1..PWM6                           | HT7833 B  |
                                      +------------------------------------->| 6 channels|
                                                                             +-----------+
```

`GND` is common across charger load/system, ESP32, and ERM driver.

## 2. Why the switch is external
The master switch must sit between `MCP73833 LOAD+` and the system. The driver PCB has no switch footprint.

- Switch OFF: ESP32 and both motor regulator domains are unpowered.
- Switch ON: ESP32 and ERM driver receive switched battery/load power.
- USB charging with switch OFF: charger remains connected to the battery, while the wearable electronics remain off.

The charger BAT wiring is external to the driver PCB.

## 3. ERM power domains
The two HT7833 outputs must remain electrically separate.

```text
SWITCHED_SYSTEM+ -> U1 HT7833 -> HAPTIC_3V3_A -> M1/M2/M3 positive
SWITCHED_SYSTEM+ -> U2 HT7833 -> HAPTIC_3V3_B -> M4/M5/M6 positive
```

Do **not** connect `HAPTIC_3V3_A` and `HAPTIC_3V3_B` together.

All six MOSFET sources connect to `GND`.

## 4. Control architecture
Each motor has one dedicated PWM-controlled MOSFET. PWM is applied to the MOSFET gate, so the motor is switched on its negative/low side.

```text
ESP32 PWMx -> 100R -> MOSFET gate
                     |
                   100k
                     |
                    GND

HAPTIC_3V3_X -> ERM+ -> ERM- / MOSFET drain -> MOSFET source -> GND
```

The ESP32 never drives motor current directly.

## 5. Motor flyback and EMI architecture
Each motor gets an individual diode directly across the motor terminals:
- diode cathode to motor positive/regulator output;
- diode anode to motor negative/MOSFET drain.

Each motor also gets a 10 nF ceramic suppression capacitor directly across the motor terminals in the V1 populated design. The PCB must minimize the physical loop formed by motor, MOSFET, diode, and local supply/ground.

## 6. Design intent
The PCB is intentionally simple. The ESP32 firmware determines haptic intensity and timing; the PCB provides reliable power regulation and six independent low-side current switches.
