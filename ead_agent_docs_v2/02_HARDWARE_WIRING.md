# 02 — Hardware Wiring

## 1. Scope
This file is the authoritative firmware-facing electrical interface. It deliberately excludes charger, battery protection, fuse/PTC, and power-management implementation because those are outside the software agent's responsibility.

## 2. MPU6050 shared I²C bus
Both MPU6050 boards share the same I²C lines.

```text
XIAO GPIO5 / D4 (SDA) ----+---- Foot MPU6050 SDA
                           +---- Shank MPU6050 SDA

XIAO GPIO6 / D5 (SCL) ----+---- Foot MPU6050 SCL
                           +---- Shank MPU6050 SCL
```

Power:
```text
XIAO 3V3 ---- Foot MPU VCC
           └- Shank MPU VCC
XIAO GND ---- Foot MPU GND
           └- Shank MPU GND
```

Addresses:
- Foot AD0 -> GND -> `0x68`.
- Shank AD0 -> 3V3 -> `0x69`.

The user has confirmed the MPU6050 breakout boards already have I²C pull-ups. The firmware assumes the existing pull-ups and does not require another external pull-up network.

## 3. MPU interrupts
- Foot MPU `INT` -> XIAO GPIO7 / D8.
- Shank MPU `INT` -> XIAO GPIO8 / D9.
- Configure both for data-ready interrupt.
- The acquisition task timestamps samples from the ESP32 monotonic clock; do not timestamp them on laptop receipt.

## 4. Motor channel
Every ERM is low-side switched by one IRLML6344.

```text
3V3_HAPTIC --- ERM +
                 ERM - --- Drain(Qx)
                             Source(Qx) --- GND_HAPTIC
ESP32 PWM --- 100R --- Gate(Qx)
Gate(Qx) --- 100k --- GND
```

Flyback diode:
```text
1N5819W across ERM
Cathode/stripe -> ERM + / 3V3_HAPTIC
Anode -> ERM - / MOSFET drain
```

Optional 10 nF motor suppression capacitor footprint is allowed but **DNP by default**; populate only if EMI/noise testing shows benefit and it does not destabilize the motor rail.

## 5. Haptic rail division
- HT7833 rail A -> motors M1, M2, M3.
- HT7833 rail B -> motors M4, M5, M6.
- Outputs of the two HT7833 regulators MUST NOT be tied together.
- One 220 µF bulk capacitor on each haptic rail.
- 1 µF input and output decoupling per HT7833.

The firmware does not control or monitor the regulators.

## 6. Logic decoupling
- Place at least 100 nF close to each MPU6050 supply.
- Use the XIAO board's own required local decoupling as supplied by the module.

## 7. Physical wiring rule
Keep the IMU signal/power region physically separated from high-current motor paths. Do not route motor current traces underneath the MPU6050 footprint if avoidable. Use a solid/common ground plane on the PCB and wide haptic supply/ground buses.

## 8. No connectors in V1
The PCB does not require dedicated JST/headers for the motors or XIAO. The user will solder stripped pre-crimped wire harness conductors directly to the board.
