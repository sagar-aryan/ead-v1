# EAD V1 — Wiring reference

Every electrical connection in the device: the XIAO ESP32-S3, both BNO086
breakouts, the six ERM motor channels, the two haptic power rails, the
harnesses and the power input. Pin assignment fixed by DEC-016.

## 1. How to trust this document

Every connection carries the evidence behind it. Nothing here is an estimate.

| Mark | Meaning |
|---|---|
| **[DS]** | Read from the manufacturer's datasheet (sources in §16) |
| **[MEAS]** | Measured on this project's hardware |
| **[USER]** | Confirmed by the builder looking at the actual part |
| **[CONTRACT]** | Fixed by `ead_agent_docs_v2/` |
| **[DEC]** | A recorded project decision (DEC-nnn) |

Build status (2026-10-07): **the DEC-016 build is the device.** Two BNO086 on SPI,
and the ERM driver PCB, tight variant (`hardware/erm-driver-pcb/`), with everything
fitted (user, 2026-10-02 and 2026-10-05; §15).

## 2. Parts

| Part | Qty | Exact type | Role |
|---|---:|---|---|
| Controller | 1 | Seeed XIAO ESP32-S3 (**plain, not Sense**) | Everything |
| IMU | 2 | 7Semi BNO086 breakout (ES-12143), PS0/PS1 jumpers opened | Foot, shank |
| Motor | 6 | ERM coin vibration motor — rated 3 V, 90 mA (120 mA at start) | Feedback |
| MOSFET | 6 | Infineon IRLML6344, SOT-23 | Low-side switch per motor |
| Diode | 6 | 1N5819W, SOD-123 Schottky | Flyback per motor |
| Resistor | 6 | 100 Ω, 1206 | Gate series |
| Resistor | 6 | 100 kΩ, 1206 | Gate pulldown |
| Regulator | 2 | HT7833, 3.3 V 500 mA LDO — Holtek, SOT-89 | Haptic rails A and B |
| Capacitor | 2 | 220 µF polymer | Bulk, one per haptic rail |
| Capacitor | 4 | 1 µF, 1206 | HT7833 input + output, each regulator |
| Capacitor | 2+ | 100 nF, 1206 | At each sensor's 3V3 pad |
| Capacitor | 1+ | 10 µF, 1206 | Logic 3V3 rail |
| Capacitor | 6 | 10 nF, 1206 | Across each motor (tight BOM C9–C14; fitted, §15) |
| Capacitor | 6 | 100 nF, 1206 | Local rail decoupling per channel (tight BOM C15–C20) |
| Resistor | 2 | 33 Ω, 1206 | SPI series termination, §5 (recommended, not in the contract) |
| Cell | 1 | 1S Li-ion/LiPo, 3.7 V nominal, ~2000 mAh | Power |
| Charger | 1 | MCP73833 module — SmartElex module (`docs/hardware.md`) | Charging |

## 3. XIAO ESP32-S3 — every pin

All 15 usable GPIOs are used. There is no spare. **[DEC-016]** The motors were
rewired on 2026-10-07 to a new order over the same six pins **[USER]** **[DEC-029]**.

| XIAO pin | GPIO | Dir | Connects to | Evidence |
|---|---:|---|---|---|
| D0 | 1 | Out | Motor 6 gate network (§6) | [DEC] no internal pull at reset [DS] |
| D1 | 2 | Out | Motor 5 gate network | [DEC] no internal pull [DS] |
| D2 | 3 | Out | **WAKE** pad on both BNO086 boards | [DEC] strapping pin, see §14 |
| D3 | 4 | Out | Motor 1 gate network | [DEC] no internal pull [DS] |
| D4 | 5 | Out | Motor 4 gate network | [DEC] no internal pull [DS] |
| D5 | 6 | Out | Motor 2 gate network | [DEC] no internal pull [DS] |
| D6 | 43 | Out | **CS** pad, foot BNO086 | [DEC] internal pull-up keeps it deselected through boot [DS] |
| D7 | 44 | Out | **CS** pad, shank BNO086 | [DEC] same [DS] |
| D8 | 7 | Out | **SCK** pad on both boards, via 33 Ω | [DEC] |
| D9 | 8 | In | **MISO** pad on both boards | [DEC] |
| D10 | 9 | Out | **MOSI** pad on both boards, via 33 Ω | [DEC] |
| Back pad MTCK | 39 | In | **INT** pad, foot BNO086 | [DEC] weak pull-up after reset — never a motor [DS] |
| Back pad MTDO | 40 | In | **INT** pad, shank BNO086 | [DEC] |
| Back pad MTDI | 41 | Out | **RST** pad on both boards | [DEC] |
| Back pad MTMS | 42 | Out | Motor 3 gate network | [DEC] no internal pull, no power-up glitch [DS] |
| 3V3 | — | Power out | **3V3** pad on both boards; 700 mA available | [DS] Seeed |
| GND | — | Power | Common ground | — |
| 5V | — | — | **Leave unconnected.** Dead when running from battery | [DS] Seeed |
| BAT+ / BAT− pads (underside) | — | Power in | Battery, §9. **BAT− is the pad nearer the USB port**; confirm against the silkscreen before soldering | [DS] Seeed |
| USB-C | — | — | Programming and the bench link | [MEAS] |

Never connected: GPIO0, 45, 46 (strapping), GPIO19/20 (USB), GPIO26–37 (flash
and octal PSRAM). On the **Sense** variant the four back pads belong to the
camera — this map is for the plain XIAO ESP32-S3 only.

## 4. BNO086 breakout — every pad on each board

Pad names exactly as printed on the 7Semi board. **[DS]** 7Semi ES-12143 §3.

| Pad | Foot board | Shank board | On-board state | Evidence |
|---|---|---|---|---|
| 3V3 | XIAO 3V3 | XIAO 3V3 | Supply, **3.3 V only** (chip max 3.63 V) | [DS] CEVA §6.1 |
| GND | GND | GND | — | — |
| MISO | D9 / GPIO8 | D9 / GPIO8 | Same chip pin as SDA, so it carries the I²C pull-up | [MEAS] TEST-039 |
| SCK | D8 / GPIO7 via 33 Ω | D8 / GPIO7 via 33 Ω | Same chip pin as SCL, pulled up | [MEAS] TEST-039 |
| MOSI | D10 / GPIO9 via 33 Ω | D10 / GPIO9 via 33 Ω | Same chip pin as SA0, pulled up | [MEAS] TEST-039 |
| CS | D6 / GPIO43 | D7 / GPIO44 | No pull-up on the board | [MEAS] TEST-039 |
| WAKE | D2 / GPIO3 | D2 / GPIO3 | Pulled up; **same net as PS0** | [USER] |
| RST | GPIO41 pad | GPIO41 pad | **10 kΩ pull-up on each board** | [USER] |
| INT | GPIO39 pad | GPIO40 pad | Active-low output | [DS] CEVA fig. 1-6 |
| BOOT | open | open | Pulled high (the bench part booted its application firmware, which needs BOOT high at reset) | [MEAS] TEST-040 |
| PS1 | open | open | Pulled up; jumper opened | [USER] [MEAS] |
| PS0 | open | open | Same net as WAKE — drive it through the WAKE pad only | [USER] |
| SCL | open | open | Same net as SCK | [DS] CEVA fig. 1-6 |
| SDA | open | open | Same net as MISO | [DS] CEVA fig. 1-6 |
| Qwiic ×2 | open | open | I²C only; dead in SPI mode | [MEAS] |

Nine pads wired per board: 3V3, GND, MISO, SCK, MOSI, CS, WAKE, RST, INT.

Consequences worth knowing:

- **RST is pulled up twice** — two 10 kΩ in parallel is 5 kΩ. GPIO41 pulls that
  low with 0.66 mA, nothing. The line no longer floats during boot, so the
  open item from the previous revision is closed. **[USER]**
- **SPI mode is latched by the boards themselves.** PS0/WAKE and PS1 are pulled
  high on each board, and CEVA requires exactly that: "Both pins must be high
  from before reset until after the first assertion of H_INTN to select the SPI
  interface." **[DS]** CEVA §1.2.4. The firmware therefore keeps WAKE high from
  reset until the first INT, and never pulses RST while WAKE is low.
- **No firmware update of the BNO086 from the XIAO.** CEVA recommends wiring BOOT
  to a GPIO for that; there is no spare GPIO. A sensor firmware update would need
  temporary wiring.

## 5. SPI bus

| Net | XIAO | Foot BNO086 | Shank BNO086 |
|---|---|---|---|
| SCK | D8 / GPIO7 → 33 Ω | SCK | SCK |
| MOSI | D10 / GPIO9 → 33 Ω | MOSI | MOSI |
| MISO | D9 / GPIO8 | MISO | MISO |
| CS_FOOT | D6 / GPIO43 | CS | — |
| CS_SHANK | D7 / GPIO44 | — | CS |
| INT_FOOT | GPIO39 | INT | — |
| INT_SHANK | GPIO40 | — | INT |
| RST | GPIO41 | RST | RST |
| WAKE | D2 / GPIO3 | WAKE | WAKE |
| 3V3 | 3V3 | 3V3 | 3V3 |
| GND | GND | GND | GND |

Settings: **SPI mode 3**, MSB first, **3 MHz maximum** **[DS]**. Measured working
at 3 MHz on short bench wires **[MEAS]**.

The foot sensor sits at the end of a long cable (§11), which the bench test
did not have. Three measures make that link robust:

1. **33 Ω in series on SCK and MOSI, at the XIAO end.** Damps ringing on the
   long run. With the line's few tens of pF this adds about a nanosecond of
   delay — irrelevant at 3 MHz. Recommended engineering practice, not in the
   contract.
2. **A ground conductor on each side of SCK** in the foot cable (§11).
3. **Run the bus at 1 MHz — and no slower.** Two sensors streaming three
   reports each at 100 Hz need well under 20 kB/s; 1 MHz gives 125 kB/s, over
   six times that. The floor comes from latency, not bandwidth: CEVA asks for
   each interrupt to be serviced "typically within 1/10 of the fastest sensor
   period" **[DS]** CEVA §1.2.4.1 — 1 ms at 100 Hz. If both sensors interrupt
   together, the second waits for the first transfer, about half a millisecond
   at 1 MHz. Move to 3 MHz only after a soak test
   on the real harness shows zero errors. This is a firmware setting.

The two boards' I²C pull-ups on SCK, MISO and MOSI end up in parallel. Every
one of those lines is driven push-pull in SPI mode, so the pull-ups only add a
little static current.

## 6. One motor channel — repeated six times

**As built (2026-10-02):** the six channels and both haptic rails (§7) are on a
separate ERM driver PCB, designed in `hardware/erm-driver-pcb/` (KiCad, with its own
docs). Same circuit as below, plus a 10 nF across each motor (C9–C14, fitted), a
100 nF rail-to-GND per channel, and 10 µF at each regulator input **[PCB project
BOM]**. Its PWM1–PWM6 pads take the motor GPIOs of §3; its `ESP+` pad feeds the
XIAO's BAT+ from the switched battery.

**[CONTRACT]** `02_HARDWARE_WIRING.md` §4. Pin numbers **[DS]** Infineon.

```text
  3V3_HAPTIC_A or _B ---+------------------+
                        |                  |
                        |           cathode (band)
                      ERM +            1N5819W
                        |               anode
                      ERM -                |
                        +------------------+------ pin 3  DRAIN
                                                   IRLML6344
  XIAO GPIO --- 100 R ---------------------+------ pin 1  GATE
                                           |       pin 2  SOURCE ---- GND
                                         100 k
                                           |
                                          GND
```

| Part | Pin | Connects to |
|---|---|---|
| IRLML6344 (SOT-23) | 1, gate | 100 Ω from the GPIO, and 100 kΩ to GND |
| | 2, source | GND |
| | 3, drain | ERM − and the diode's anode |
| 1N5819W (SOD-123) | cathode — the **band** end | The channel's haptic rail (ERM +) |
| | anode | The MOSFET drain (ERM −) |
| ERM motor | + | The channel's haptic rail, A or B |
| | − | The MOSFET drain |

Why it is safe:

- Gate threshold is 0.5–1.1 V; on-resistance at most 37 mΩ at 2.5 V gate
  **[DS]**. The ESP32's 3.3 V drive turns it fully on.
- The 100 kΩ pulldown holds the gate at 0 V from power-up until firmware drives
  the pin. That only works because no motor GPIO has an internal pull-up (§14).
- Diode backwards = a dead short across the rail. **Check every band.**
- The diode sits on the board, across the channel's two wires — not at the motor.
- The 10 nF footprint across each motor stays empty unless EMI testing shows a
  need **[CONTRACT]**.

### All six channels

| Motor | On the band | XIAO pin | GPIO | Rail | Rail wire in the band harness |
|---|---|---|---:|---|---|
| M1 | Anterior, 0° | D3 | 4 | A | +A |
| M2 | Anterolateral, 60° | D5 | 6 | A | +A |
| M3 | Posterolateral, 120° | back pad MTMS | 42 | A | +A |
| M4 | Posterior, 180° | D4 | 5 | B | +B |
| M5 | Posteromedial, 240° | D1 | 2 | B | +B |
| M6 | Anteromedial, 300° | D0 | 1 | B | +B |

XIAO pins as rewired by the user on 2026-10-07 (DEC-029), with M5 and M6 then swapped
at the user's request (M5 D1, M6 D0); until then M1–M6 were on D0, D1, MTMS, D3, D4, D5. The Rail column is the design's (M1–M3 on rail A). Each motor's
position is confirmed by a service pulse felt on the band.

Angles from the front of the shin, clockwise seen from above, right leg
**[CONTRACT]** `06_ERROR_AND_HAPTIC_ENGINE.md` §7. Rail A feeds the front-lateral
half of the band, rail B the back-medial half.

## 7. Haptic power rails

**[CONTRACT]** `02_HARDWARE_WIRING.md` §5.

| From | To | Capacitors |
|---|---|---|
| Switched battery + (§9) | HT7833 A — VIN | 1 µF VIN to GND |
| Switched battery + | HT7833 B — VIN | 1 µF VIN to GND |
| HT7833 A — VOUT | Rail A → M1, M2, M3 positive | 1 µF + 220 µF VOUT to GND |
| HT7833 B — VOUT | Rail B → M4, M5, M6 positive | 1 µF + 220 µF VOUT to GND |
| HT7833 A and B — GND | Common ground | — |

- **The two VOUT pins must never touch.** **[CONTRACT]**
- Motors are never powered from the XIAO's 3V3 pin.
- **HT7833 pin numbers depend on who made the part.** At least two different
  manufacturers sell a 3.3 V, 500 mA LDO marked HT7833: Holtek, and Shenzhen
  Airupton. The Airupton datasheet gives **[DS]**:

  | Package | GND | VIN | VOUT | EN |
  |---|---:|---:|---:|---:|
  | SOT-89 | 1 | 2 | 3 | — |
  | SOT-23-3 | 1 | 3 | 2 | — |
  | SOT-23-5 | 2 | 1 | 5 | 3, tie to VIN |

  Holtek's own HT78xx datasheet link is dead at the time of writing, so its
  table could not be read. The fitted part's pin order is in the update below;
  a regulator fitted backwards would put battery voltage on the motor rail.
- **Update 2026-10-02:** the ERM driver PCB uses the Holtek HT7833, SOT-89, with
  pad 1 GND, pad 2 VIN (and the tab), pad 3 VOUT, read by that project from
  Holtek HT78xx Rev 1.51 (`holtek.com/webapi/116711/HT78xxv151.pdf`) **[DS, PCB
  project]**: the same order as the Airupton SOT-89 row above.

Thermal check, worst case: full cell 4.2 V, three motors running. Dissipation
is (4.2 − 3.3) V × I. At 300 mA that is 0.27 W; a SOT-89 at 200 °C/W **[DS]**
Holtek AN0553 runs 54 °C above ambient. Acceptable for a pulsed load (motors rated
90 mA, 120 mA at start).

Operating window: dropout is 220 mV at 200 mA **[DS]** Airupton, so the rails hold
3.3 V down to a cell voltage of about 3.5 V.

## 8. Logic power

| Item | Value | Where |
|---|---|---|
| Sensor decoupling | 100 nF | At each BNO086's 3V3 pad |
| Logic rail | 10 µF | On the 3V3 line near the XIAO |
| Sensor current | 10.7 mA each at 100 Hz rotation vector (3.18 mA VDDIO + 7.50 mA VDD) | [DS] CEVA fig. 6-18 |
| Available | 700 mA from the XIAO 3V3 pin | [DS] Seeed |

Two sensors take about 22 mA of the 700 mA available. The XIAO itself draws
about 100 mA with Wi-Fi active **[DS]** Seeed.

Layout rule **[CONTRACT]**: keep the sensor region away from motor current; no
motor current under a sensor footprint; solid ground plane; wide haptic supply
and ground buses.

## 9. Power input and charging

The contract leaves this outside its scope. These are the facts that fix it:

- The XIAO has its own charger on the BAT pads, at **50 mA** fast charge
  **[DS]** Seeed spec table. That takes about 40 hours to fill a 2000 mAh cell,
  so the MCP73833 module is the practical charger.
- With two chargers on one cell, the XIAO's would also charge whenever USB-C is
  plugged into the XIAO.
- A charger cannot detect "full" while the device is drawing current from the
  same cell.

Recommended wiring, which gives one charger and a clean full-charge cut-off:

```text
                       +------------------ MCP73833 module BAT+
                       |
  Cell + (protected) --+---- SWITCH ---+---- XIAO BAT+
                                       +---- HT7833 A VIN
                                       +---- HT7833 B VIN

  Cell - --------------+------------------ MCP73833 module BAT-
                       +------------------ XIAO BAT- , HT7833 GND x2,
                                           all six MOSFET sources (common GND)
```

- **Charge with the switch OFF.** The MCP73833 then sees only the cell and
  terminates properly; the XIAO's own charger is disconnected.
- With the switch ON and USB-C in the XIAO (bench use), the XIAO's 50 mA charger
  can also reach the cell. Harmless, but avoid charging that way.
- The cell must have its own protection circuit. A body-worn lithium cell
  without one is not acceptable.
- Battery wires: heavier gauge than signal wire **[CONTRACT]**.

As built: an external master switch, the SmartElex MCP73833 module, and a cell with
its own protection board (user).

## 10. Firmware rules the wiring depends on

1. Drive all six motor GPIOs LOW as the first action in `setup()`.
2. Drive WAKE high, then pulse RST low, then wait for the first INT before
   touching the sensors. Never pulse RST while WAKE is low.
3. Enable the internal pull-up on both INT inputs, so a broken INT wire reads
   "no data", not noise.
4. Hold both CS lines high whenever the bus is idle.
5. Start the SPI bus at 1 MHz (§5).
6. Service each INT within about 1 ms — CEVA's guidance is "typically within
   1/10 of the fastest sensor period" — and never later than 10 ms, when the
   BNO086 times out and retries **[DS]** CEVA §1.2.4.1. Late service costs the
   sensor processing time and degrades its outputs.

## 11. Harnesses — conductor by conductor

No connectors in V1; conductors soldered to the board **[CONTRACT]**.

**Shank sensor — 9 conductors** (short run on the same strap):
3V3, GND, SCK, MOSI, MISO, CS_SHANK, INT_SHANK, RST, WAKE.

**Foot sensor — 10 conductors**, in this order in a flat cable, or with SCK
twisted against a ground in a round one:

| # | Signal |
|---:|---|
| 1 | GND |
| 2 | SCK |
| 3 | GND |
| 4 | MOSI |
| 5 | MISO |
| 6 | CS_FOOT |
| 7 | INT_FOOT |
| 8 | RST |
| 9 | WAKE |
| 10 | 3V3 |

**Motor band — 8 conductors:**

| # | Signal |
|---:|---|
| 1 | Rail A + — to M1, M2, M3 |
| 2 | Rail B + — to M4, M5, M6 |
| 3–8 | M1 − … M6 −, one return per motor, to its MOSFET drain |

The contract keeps the two regulator outputs apart, so each half of the band has its
own positive wire.

**Battery — 2 conductors**, heavier gauge than signal wire.

## 12. Cautions

1. **Never put a motor on GPIO39.** It comes out of reset with a 45 kΩ weak
   pull-up unless an eFuse is burnt; against the 100 kΩ gate pulldown that is
   2.3 V on the gate, above the 1.1 V maximum threshold. The motor would run
   from power-up until firmware intervened **[DS]**.
2. **3.3 V only at every sensor pad.** Absolute maximum 3.63 V **[DS]**.
3. **Diode band to the rail**, on all six channels.
4. **HT7833 pinout varies by manufacturer.** Confirm before soldering (§7).
5. **Keep rail A and rail B apart.**
6. **Never pulse RST while WAKE is low.**
7. **Plain XIAO ESP32-S3 only** — on the Sense variant the back pads are taken.
8. **The shank IMU and the motor band clamp the same bone.** Measure vibration
   pickup once the drivers are fitted (DEC-015).

## 13. Before first power-up

1. Power off: measure 3V3 to GND, rail A to GND, rail B to GND, battery + to
   GND. None may read as a short.
2. Confirm rail A and rail B are not connected to each other.
3. Confirm every diode's band faces its rail.
4. Confirm each HT7833's VIN, VOUT and GND against the seller's pinout.
5. Power on with **no motors fitted**: rails A and B read 3.3 V; each BNO086
   reads ~3.3 V on PS0, PS1 and WAKE; all six gates read ~0 V.
6. Done on 2026-10-01 (TEST-041): `firmware/bench/padstate` confirmed the pad
   states of §14 and the JTAG eFuses on this chip. Repeat it on any other XIAO
   that is used.
7. Run the bench sensor check on both sensors, then fit the motors one at a time.

## 14. Why each awkward pin carries what it carries

| Pin | Constraint | Carries | Why that is safe |
|---|---|---|---|
| GPIO39 | Weak pull-up after reset unless EFUSE_DIS_PAD_JTAG is burnt **[DS]** | Foot INT, an input | A pull-up suits an input that idles high |
| GPIO3 | Strapping pin; 60 µs low glitch at power-up **[DS]** | WAKE | Only selects the JTAG source if an eFuse is burnt; the boards' own pull-ups set WAKE's boot level |
| GPIO43/44 | UART0; GPIO43 carries the ROM boot log | Chip selects | Internal pull-ups keep them high; with no clock on SCK the boot-log wiggle transfers nothing |
| GPIO40, 41, 42 | Nothing — no pull, not strapping, no power-up glitch **[DS]** | Shank INT, RST, Motor 3 | Clean pins |
| GPIO1, 2, 4, 5, 6 | 60 µs **low** glitch at power-up **[DS]** | Motors 1, 2, 4, 5, 6 | Low is "off"; none has an internal pull |

### Conflict check

Every rule a pin can break, checked against the final map.

| Check | Result | Evidence |
|---|---|---|
| Every GPIO used exactly once | **Pass** — 1, 2, 3, 4, 5, 6, 7, 8, 9, 39, 40, 41, 42, 43, 44 | — |
| No motor pin has an internal pull-up | **Pass** — GPIO1, 2: input-enable only; 4, 5, 6: nothing; 42: input-enable only after reset | [DS] Table 2-1 |
| No motor pin glitches high at power-up | **Pass** — GPIO1–6 glitch low only; GPIO42 has no glitch | [DS] Table 2-2 |
| GPIO39's pull-up lands on an input | **Pass** — foot INT | [DS] Table 2-1 note 7 |
| Back pads not connected to the JTAG controller | **Pass, measured** — eFuses read 0 on the chip (TEST-041); with them at factory default, JTAG goes to the USB Serial/JTAG controller, so the foot sensor's INT on MTCK cannot clock a JTAG state machine and MTDO is never driven against the shank INT | [DS] Table 3-5 |
| GPIO3's strap role inert | **Pass, measured** — "Ignored" with the eFuses at factory default, which TEST-041 confirmed | [DS] Table 3-5, [MEAS] |
| Boot-mode straps GPIO0, 45, 46 untouched | **Pass** | [DS] Table 3-1 |
| USB pins GPIO19/20 untouched | **Pass** | [DS] |
| Flash and octal-PSRAM pins GPIO26–37 untouched | **Pass** | ESP-IDF GPIO docs |
| Chip selects high through boot | **Pass** — GPIO43/44 have internal pull-ups | [DS] Table 2-1 |
| Boot log on GPIO43 reaching the foot CS | **No effect** — no clock on SCK; the shank CS stays high, so MISO is never contested | [DS] |
| Two sensors sharing SCK, MISO and MOSI | **Supported by the manufacturer** — "Multiple slave devices can exist on a SPI interface by the use of a chip select signal"; MISO is driven only after CS falls (CS-to-MISO 31 ns). Proven on these boards only by bring-up step 3, both sensors together | [DS] CEVA §1.2.4.2, §6.5.2 |
| One sensor on MISO at a time | **Firmware rule** — never assert both CS together | §10 |
| Sensors come up in SPI mode | **Pass** — both boards hold PS0/WAKE and PS1 high | [DS] CEVA §1.2.4, [USER] |
| RST held high through boot | **Pass** — 10 kΩ on each board | [USER] |
| PWM channels | **Pass** — 6 used of 8 | [DS] |
| 3V3 budget | **Pass** — about 22 mA of 700 mA | [DS] CEVA, Seeed |
| UART0 never started | **Firmware rule** — starting it would reclaim GPIO43/44 from the chip selects | — |
| Board variant | **Plain XIAO ESP32-S3 only** — on the Sense the back pads drive the camera | [DS] Seeed |

**Measured on the chip (TEST-041, 2026-10-01):** all three JTAG eFuses read 0,
the factory value, so the three rows that depend on them are confirmed. Every
motor pin has neither pull-up nor pull-down; both chip-select pins have their
pull-ups and sit high. GPIO39's pull-up is already off when user code runs, but
the pad still reads high with nothing attached: the pull-up was on from reset,
the window in which a motor there would have switched on.

## 15. What is wired today

**As built, power (user, 2026-10-02):** the switched LOAD+ goes to the ERM
driver's PWR+ and to the XIAO's **5V pin**, not to BAT+ as §9 and the PCB design
specify. Seeed: the 5V pin is USB VBUS and needs an external diode when used as an
input **[vendor doc]**. **Never turn the master
switch ON while USB is plugged into the XIAO.**

**Current device (user, 2026-10-02): the DEC-016 build of §3, with the ERM driver
PCB of §6, everything fitted.** Motors rewired to a new pin order on 2026-10-07
(user; §6, DEC-029).

**Previous device, no longer exists.** Two MPU6500s on I²C.

| Function | XIAO pin | GPIO |
|---|---|---:|
| I²C SDA | D4 | 5 |
| I²C SCL | D5 | 6 |
| Foot INT (address 0x68) | D8 | 7 |
| Shank INT (address 0x69) | D9 | 8 |
| Motor outputs, held LOW, nothing connected | D0, D1, D3, D10, D6, D7 | 1, 2, 4, 9, 43, 44 |

**Bench — one BNO086** (edge pins only; not the product layout):
3V3→3V3, GND→GND, SCK→D8, MISO→D9, MOSI→D10, CS→D3, INT→D2, RST→D1, WAKE→D0.

## 16. Sources

| Source | Used for |
|---|---|
| Espressif, ESP32-S3 Series Datasheet v2.2 — Tables 2-1, 2-2, 3-1, 5-4 | Pad states at and after reset, glitches, strapping, pull resistors, drive current |
| ESP-IDF GPIO documentation, ESP32-S3 | Pin restrictions |
| Seeed Studio, XIAO ESP32-S3 wiki | Back pads, BAT pad polarity, 50 mA charge current, 700 mA 3V3 output, 5V pin on battery |
| CEVA, BNO08X Datasheet 1000-3927 v1.17 | Pin functions, SPI-mode strapping, power, absolute maxima, INT timeout |
| 7Semi, BNO086 breakout manual ES-12143 v1.0 | Pad names |
| Infineon, IRLML6344 datasheet v01_01 | Pinout, gate threshold, on-resistance |
| Shenzhen Airupton, HT78XX datasheet V1.0 | HT7833 pinout by package, dropout |
| Holtek, AN0553 application note | SOT-89 thermal resistance, capacitor use |
| `ead_agent_docs_v2/02, 03, 06, 17` | Motor channel, rails, band angles, parts |
| TEST-039, TEST-040 | Measured board behaviour |

---

*This document is the source for `EAD_V1_wiring_reference.pdf`. Rebuild it with:*

```bash
pandoc docs/wiring_reference.md -t html5 --standalone --css=style.css \
  --metadata title="EAD V1 wiring reference" -o wiring.html
libreoffice --headless --convert-to pdf wiring.html
```
