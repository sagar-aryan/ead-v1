# EAD V1 — Complete connection reference

Every electrical connection in the device: the XIAO ESP32-S3, both BNO086 IMUs,
the six ERM motor channels, the haptic power rails, and the power input.

Generated from the contract (`ead_agent_docs_v2/02_HARDWARE_WIRING.md`,
`03_GPIO_PIN_MAP.md`, `17_HARDWARE_BOM_AND_OWNED_PARTS.md`), the decision log
(`docs/decisions.md`) and the measured bench results (`docs/testing.md`
TEST-039, TEST-040).

## 1. How to read the status column

| Status | Meaning |
|---|---|
| **BUILT** | Wired and working on the current device |
| **BENCH** | Wired and measured on the bench, one sensor only |
| **PROPOSED** | Decided on paper, nothing soldered, no firmware yet |
| **NOT FITTED** | Parts owned, nothing built; firmware holds these pins LOW |

The target build in sections 3–9 is **PROPOSED** as a whole. Only the bench
sensor of section 10 has been powered and measured.

## 2. What connects to what

```text
                 USB-C (programming, bench power)
                        |
  BNO086 #1  --SPI+3 ---+                  +-- 3V3 logic --- both BNO086
  (foot)                |                  |
                   XIAO ESP32-S3 ----------+
  BNO086 #2  --SPI+3 ---+                  |
  (shank)               |              6 x PWM
                        |                  |
                   battery 1S         6 x IRLML6344 low-side switch
                   3.7 V 2000 mAh          |
                        |            +-----+-----+
                   MCP73833 charger   HT7833 A   HT7833 B
                        |            M1 M2 M3    M4 M5 M6
                        +----------- (haptic rails, kept separate)
```

Counts: 1 controller, 2 IMUs, 6 motors, 6 MOSFETs, 6 flyback diodes,
6 gate resistors, 6 gate pulldowns, 2 regulators, 1 battery, 1 charger.

## 3. XIAO ESP32-S3 — every pin, target build

All 15 usable GPIOs are consumed: 6 motors, 9 for the two IMUs. There is no
spare GPIO in this build.

| XIAO pin | GPIO | Direction | Connects to | Status |
|---|---:|---|---|---|
| D0 | 1 | Output | Motor 1 gate drive, via 100 Ω | PROPOSED |
| D1 | 2 | Output | Motor 2 gate drive, via 100 Ω | PROPOSED |
| D2 | 3 | Output | Motor 3 gate drive, via 100 Ω — see §11 caution | PROPOSED |
| D3 | 4 | Output | Motor 4 gate drive, via 100 Ω | PROPOSED |
| D4 | 5 | Output | Motor 5 gate drive, via 100 Ω | PROPOSED |
| D5 | 6 | Output | Motor 6 gate drive, via 100 Ω | PROPOSED |
| D6 | 43 | Output | Chip select, **foot** BNO086 (active low) | PROPOSED |
| D7 | 44 | Output | Chip select, **shank** BNO086 (active low) | PROPOSED |
| D8 | 7 | Output | SPI clock, SCK — shared by both sensors | PROPOSED |
| D9 | 8 | Input | SPI data in, MISO — shared by both sensors | PROPOSED |
| D10 | 9 | Output | SPI data out, MOSI — shared by both sensors | PROPOSED |
| Back pad MTCK | 39 | Input | Data-ready interrupt, **foot** BNO086 | PROPOSED |
| Back pad MTDO | 40 | Input | Data-ready interrupt, **shank** BNO086 | PROPOSED |
| Back pad MTDI (D12) | 41 | Output | Reset, RST — shared by both sensors (active low) | PROPOSED |
| Back pad MTMS (D11) | 42 | Output | Wake / PS0 — shared by both sensors (active low pulse) | PROPOSED |
| 3V3 | — | Power out | Logic supply to both BNO086 boards | PROPOSED |
| GND | — | Power | Common ground: sensors, gate pulldowns, haptic ground | PROPOSED |
| 5V | — | Power | Unused in V1 |  |
| BAT+ / BAT− (underside) | — | Power in | 1S battery, via the charger — see §9 | PROPOSED |
| USB-C | — | Data + power | Programming and the bench link | BUILT |

Pins deliberately not used: GPIO0 (BOOT strapping), GPIO19/20 (USB), GPIO26–37
(SPI flash / PSRAM), GPIO45/46 (strapping), GPIO21 (on-board LED, not brought out).

Using GPIO39–42 means the JTAG header on those pads is no longer available.
Debugging stays on USB, which is how the firmware already works.

## 4. BNO086 — every pad on each board

Both boards are wired identically except for chip select and interrupt. Nine
conductors per sensor.

| BNO086 pad | Foot sensor goes to | Shank sensor goes to | Note |
|---|---|---|---|
| 3V3 | XIAO 3V3 | XIAO 3V3 | **3.3 V only, never 5 V** |
| GND | XIAO GND | XIAO GND | Common ground |
| SCK (shares SCL) | D8 / GPIO7 | D8 / GPIO7 | Shared clock |
| MISO (shares SDA) | D9 / GPIO8 | D9 / GPIO8 | Shared data in |
| MOSI (shares SA0) | D10 / GPIO9 | D10 / GPIO9 | Shared data out |
| CS | D6 / GPIO43 | D7 / GPIO44 | One per sensor; idles high |
| INT | GPIO39 pad | GPIO40 pad | One per sensor; sensor pulls it low when it has data |
| RST | GPIO41 pad | GPIO41 pad | Shared; both sensors reset together |
| PS0 / WAKE | GPIO42 pad | GPIO42 pad | Shared; one net on this board, see below |
| PS1 | leave open | leave open | Pulled high on the board |
| BOOT | leave open | leave open | Firmware update only |
| Qwiic connectors ×2 | leave open | leave open | I²C only; unusable in SPI mode |

Three things about this board, confirmed by measurement on the bench (TEST-039):

- **PS0 and WAKE are the same net.** The board has pull-ups on PS0, PS1 and
  WAKE. With the PS0/PS1 solder jumpers opened — which has been done — both read
  high at reset, so the part comes up in **SPI mode**, and its Qwiic/I²C
  connectors stop working.
- **Start-up order matters.** Drive WAKE high, then pulse RST low, then wait for
  the first INT. Never pulse RST while WAKE is low: PS0 would be sampled low and
  the part would come up in a UART mode instead of SPI.
- **SCK, MISO and MOSI sit on pads that carry the board's I²C pull-ups**, so
  those three lines read high with the sensor deselected. That is normal here,
  not a fault.

SPI settings: **mode 3** (clock idles high, data sampled on the rising edge),
MSB first, **3 MHz maximum**. Measured working at 3 MHz, 99.9 Hz per sensor.

## 5. Shared SPI bus — net list

| Net | XIAO | Foot BNO086 | Shank BNO086 |
|---|---|---|---|
| SCK | D8 / GPIO7 | SCK | SCK |
| MISO | D9 / GPIO8 | MISO | MISO |
| MOSI | D10 / GPIO9 | MOSI | MOSI |
| CS_FOOT | D6 / GPIO43 | CS | — |
| CS_SHANK | D7 / GPIO44 | — | CS |
| INT_FOOT | GPIO39 | INT | — |
| INT_SHANK | GPIO40 | — | INT |
| RST | GPIO41 | RST | RST |
| WAKE | GPIO42 | PS0/WAKE | PS0/WAKE |
| 3V3 | 3V3 | 3V3 | 3V3 |
| GND | GND | GND | GND |

Keep the three shared lines as short as the build allows, and run a ground
conductor alongside the foot sensor's cable, which is the long one.

## 6. One ERM motor channel — repeated six times

From `ead_agent_docs_v2/02_HARDWARE_WIRING.md` §4, unchanged.

```text
  3V3_HAPTIC  ------+-------------------+
                    |                   |
                    |              cathode (stripe)
                  ERM +              1N5819W
                    |               anode
                  ERM -                 |
                    +-------------------+----- Drain   IRLML6344
                                                 Source ----- GND_HAPTIC
                                                 Gate
  XIAO PWM pin ---- 100 R ------------------------+
                                                  |
                                                100 k
                                                  |
                                                 GND
```

Per channel: one ERM coin motor, one IRLML6344 N-channel MOSFET, one 1N5819W
Schottky flyback diode, one 100 Ω gate resistor, one 100 kΩ gate pulldown.

- The diode's **stripe goes to the positive rail**, its other end to the drain.
  Backwards, it shorts the rail.
- The 100 kΩ pulldown holds the motor off while the ESP32 boots, before the
  firmware drives the pin.
- A 10 nF suppression capacitor footprint across each motor is allowed but
  **not populated** by default. Fit it only if EMI testing shows it helps.

### All six channels

| Motor | Position on the band | Gate driven by | Rail |
|---|---|---|---|
| M1 | Anterior, 0° | D0 / GPIO1 | HT7833 A |
| M2 | Anterolateral, 60° | D1 / GPIO2 | HT7833 A |
| M3 | Posterolateral, 120° | D2 / GPIO3 | HT7833 A |
| M4 | Posterior, 180° | D3 / GPIO4 | HT7833 B |
| M5 | Posteromedial, 240° | D4 / GPIO5 | HT7833 B |
| M6 | Anteromedial, 300° | D5 / GPIO6 | HT7833 B |

Angles are measured from the front of the shin, clockwise seen from above, on
the right leg (`ead_agent_docs_v2/06_ERROR_AND_HAPTIC_ENGINE.md` §7).

PWM, when the firmware is written: 200 Hz, 8-bit, duty between 51 and 204
(20 %–80 %). All six pins are driven LOW as the first action at boot and stay
LOW until the device reaches READY with healthy sensors.

## 7. Haptic power rails

Two regulators, three motors each, outputs kept apart.

| From | To | Component | Note |
|---|---|---|---|
| Battery + | HT7833 A input | — | 1 µF input capacitor |
| Battery + | HT7833 B input | — | 1 µF input capacitor |
| HT7833 A output | 3V3_HAPTIC_A → M1, M2, M3 | 220 µF polymer + 1 µF | — |
| HT7833 B output | 3V3_HAPTIC_B → M4, M5, M6 | 220 µF polymer + 1 µF | — |
| Both MOSFET source groups | GND_HAPTIC | — | Joins the common ground plane |

**The two regulator outputs must never be tied together.** The firmware neither
controls nor monitors them; they are always on with the battery connected.

Motors are never powered from the XIAO's own 3V3 pin. That pin feeds the two
sensors only.

## 8. Logic power and decoupling

| Item | Value | Where |
|---|---|---|
| Sensor decoupling | 100 nF | As close as possible to each BNO086's 3V3 pin |
| Rail decoupling | 10 µF | On the logic 3V3 rail |
| Controller decoupling | — | The XIAO module carries its own |

Keep the sensor signal and power region physically away from the motor current
paths; do not route motor current under a sensor footprint; use a solid ground
plane and wide haptic supply and ground buses.

## 9. Power input

The contract puts the charger, battery protection, fuse and power management
outside the software scope, so no wiring for them is specified. The owned parts
are a 1S 3.7 V ~2000 mAh cell and an MCP73833 charger module; the cell reaches
the XIAO through its BAT pads. There is no battery sense wire, no fuel gauge, no
ADC and no power switch input — the firmware has no way to read the battery, by
decision, so none should be added to the harness expecting software to use it.

## 10. What is actually wired today

**Current device — BUILT.** Two MPU6500 breakouts on I²C, which the BNO086 build
replaces:

| Function | XIAO pin | GPIO |
|---|---|---:|
| I²C SDA, both IMUs | D4 | 5 |
| I²C SCL, both IMUs | D5 | 6 |
| Foot IMU INT (address 0x68) | D8 | 7 |
| Shank IMU INT (address 0x69) | D9 | 8 |
| Six motor outputs, held LOW | D0, D1, D3, D10, D6, D7 | 1, 2, 4, 9, 43, 44 |

Foot AD0 to GND gives 0x68; shank AD0 to 3V3 gives 0x69. The breakouts carry
their own I²C pull-ups. **No ERM drivers are fitted** — nothing is connected to
the six motor pins.

**Bench — BENCH.** One BNO086 on edge pins only, so nothing needs soldering to
the back pads. This is a test layout, not the product one:

| BNO086 | XIAO pin | GPIO |
|---|---|---:|
| 3V3, GND | 3V3, GND | — |
| SCK | D8 | 7 |
| MISO | D9 | 8 |
| MOSI | D10 | 9 |
| CS | D3 | 4 |
| INT | D2 | 3 |
| RST | D1 | 2 |
| WAKE | D0 | 1 |

## 11. Cautions, each with its reason

1. **GPIO3 (D2) is a strapping pin.** `03_GPIO_PIN_MAP.md` §2 says not to use it.
   The target build puts Motor 3 there because the SPI bus needs D6–D10. Its
   100 kΩ gate pulldown holds it low through boot, which is a defined state, but
   this is a deviation from the contract and needs a recorded decision before the
   board is made. If it has to be avoided, the shared WAKE line is the only
   other candidate to move, because every remaining pin is spoken for.
2. **GPIO43/44 carry the ROM boot log.** GPIO43 is UART0 TX and idles high before
   firmware runs (PROB-004). In the target build these two pins drive chip
   select, which idles high anyway — so the risk the contract's motor map carried
   disappears. Do not put motors back on them.
3. **Never pulse RST while WAKE is low.** PS0 is sampled at reset; low selects a
   UART mode and the sensors go silent on SPI.
4. **3.3 V only at every sensor pin.** The BNO086 is not 5 V tolerant.
5. **Keep the two haptic rails separate**, and keep motor current off the sensor
   region.
6. **Check the flyback diode orientation on all six channels** before applying
   power: stripe to the positive rail.
7. **The shank IMU and the motor band clamp the same bone.** Vibration will reach
   the sensor; measure it once the drivers are fitted (DEC-015).

## 12. Conductor count per harness run

| Run | Conductors | Contents |
|---|---:|---|
| Controller → shank BNO086 | 9 | 3V3, GND, SCK, MISO, MOSI, CS, INT, RST, WAKE |
| Controller → foot BNO086 | 9 | same, with its own CS and INT |
| Controller → motor band | 7 | 6 switched returns + 1 shared positive |
| Battery → board | 2 | heavier gauge than signal wire |

No connectors are used in V1: harness conductors are soldered directly to the
board.

## 13. Before first power-up

1. Measure 3V3 to GND for a short, with power off.
2. Confirm each BNO086 reads about 3.3 V on PS0, PS1 and WAKE.
3. Confirm all six gates read near 0 V through their pulldowns.
4. Check all six diode stripes face the positive rail.
5. Confirm the two HT7833 outputs are not connected to each other.
6. Power up with no motors fitted, run the bench check, then fit the motors.

---

*This document is the source for `EAD_V1_wiring_reference.pdf`. Rebuild it with:*

```bash
pandoc docs/wiring_reference.md -t html5 --standalone --css=style.css \
  --metadata title="EAD V1 wiring reference" -o wiring.html
libreoffice --headless --convert-to pdf wiring.html
```
