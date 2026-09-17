# Hardware (as built)

Contract values come from `ead_agent_docs_v2/02_HARDWARE_WIRING.md`,
`03_GPIO_PIN_MAP.md` and `17_HARDWARE_BOM_AND_OWNED_PARTS.md`. This file records
what the physical unit actually is, how it deviates, and what has been verified.
Every row states its evidence; "unverified" means nobody has measured it yet.

## Controller

| Item | Value | Evidence |
|---|---|---|
| Board | Seeed XIAO ESP32-S3 | PlatformIO env `seeed_xiao_esp32s3` |
| Flash / PSRAM | 8 MB / 8 MB (OPI) | Board definition (`-DBOARD_HAS_PSRAM`, `qio_opi`); PSRAM presence on this unit unverified until the M1 self-test |
| USB | Native USB Serial/JTAG, VID:PID `303a:1001`, serial number = Wi-Fi MAC `44:B1:76:AF:FB:7C` | `lsusb` / `udevadm`, 2026-09-17 |
| Partition table | `default_8MB.csv`: 2 × 3.2 MB app slots, 1.5 MB data (`spiffs`), 64 KB coredump | Arduino-ESP32 2.0.17 package |
| Wi-Fi antenna | Seeed recommends fitting the supplied U.FL antenna for usable Wi-Fi range | Seeed wiki (accessed 2026-09-17); fitted on this unit: unverified |
| Power | Battery-powered and wearable (user, 2026-09-17). Firmware has no battery or charger logic (spec) | User statement |

## IMUs

Both breakouts were sold as MPU6050 but carry **MPU6500** silicon.

| Sensor | I²C address | WHO_AM_I | Evidence |
|---|---|---|---|
| Foot (dorsum) | `0x68` (AD0 → GND) | `0x70` | Boot log 2026-09-17 |
| Shank (anterior shin) | `0x69` (AD0 → 3V3) | `0x70` | Boot log 2026-09-17 |

Register configuration written at boot and verified by readback on both sensors
(boot log + `c` diagnostics, 2026-09-17):

| Register | Address | Value | Meaning |
|---|---|---|---|
| PWR_MGMT_1 | 0x6B | 0x01 | Awake, PLL clock source |
| SMPLRT_DIV | 0x19 | 0x09 | 1 kHz / 10 = 100 Hz output |
| CONFIG | 0x1A | 0x03 | Gyro DLPF ≈ 41–42 Hz |
| ACCEL_CONFIG2 | 0x1D | 0x03 | Accel DLPF ≈ 41 Hz (MPU6500 only; see PROB-003) |
| GYRO_CONFIG | 0x1B | 0x08 | ±500 °/s, 65.5 LSB/(°/s) |
| ACCEL_CONFIG | 0x1C | 0x08 | ±4 g, 8192 LSB/g |
| INT_PIN_CFG | 0x37 | 0x00 | Active-high, push-pull, 50 µs pulse |
| INT_ENABLE | 0x38 | 0x01 | Data-ready interrupt |

Observed at rest on a desk (not worn), 78 samples over 7.8 s, 2026-09-17:
foot |a| = 1.024 g, shank |a| = 1.014 g, accel σ ≤ 0.002 g, gyro offsets up to
3.1 °/s (foot X) and 2.0 °/s (shank Y). The offsets are ordinary MEMS gyro bias;
startup calibration (milestone M3) removes them.

## Pin map (doc 03, unchanged)

| Function | XIAO pin | ESP32-S3 GPIO | Status |
|---|---|---:|---|
| I²C SDA | D4 | 5 | Verified: both IMUs enumerate |
| I²C SCL | D5 | 6 | Verified: both IMUs enumerate |
| Foot IMU INT | D8 | 7 | Wiring unverified; interrupt-count test is the first M1 step |
| Shank IMU INT | D9 | 8 | Wiring unverified; interrupt-count test is the first M1 step |
| Motor 1 PWM | D0 | 1 | Held LOW; driver not fitted |
| Motor 2 PWM | D1 | 2 | Held LOW; driver not fitted |
| Motor 3 PWM | D3 | 4 | Held LOW; driver not fitted |
| Motor 4 PWM | D10 | 9 | Held LOW; driver not fitted |
| Motor 5 PWM | D6 | 43 | Held LOW; driver not fitted; see PROB-004 |
| Motor 6 PWM | D7 | 44 | Held LOW; driver not fitted; see PROB-004 |

## Sensor mounting and axis maps

The anatomical frame (doc 04 §1) is X forward toward the toes, Y medial (left for
the right leg), Z up. It is right-handed, and so is the MPU chip frame, so each
sensor's chip → anatomical map must be a proper rotation (determinant +1).
`firmware/include/config_v1.h` stores the maps as signed-permutation matrices and
rejects anything else at compile time.

| Sensor | Physical mounting | Map (anat = M · chip) |
|---|---|---|
| Foot | Flat on the dorsum, chip X toward the toes, chip Z up | Identity |
| Shank | On the anterior shin; chip +Z toward the bone (posterior); chip +Y down the leg | `X = −chipZ`, `Y = +chipX`, `Z = −chipY` |

Derivation of the shank map: the user confirmed chip +Z points toward the bone
(2026-09-17). An earlier still capture put gravity on chip −Y, so chip +Y points
down the leg. That fixes `X = −chipZ` and `Z = −chipY`; right-handedness then forces
`Y = Z × X = +chipX`.

**Placement image discrepancy.** `reference_images/RIGHT_LEG_IMU_PLACEMENT.png`
draws the shank board with its axes labelled as if it matched the foot board. A
board lying flat on a vertical shin cannot have chip Z pointing up, so the image
cannot describe any real shank mounting. The user confirmed the image is wrong
about the shank board and right about the anatomical convention. The contract
file is left unmodified; this table is the as-built record (DEC-009).

**Verification status.** Compile-time proper-rotation checks pass, including a
negative test with the previous map. On-body check pending: standing still must
read anatomical a ≈ (0, 0, +1) g on both sensors.

## Haptics

ERM driver channels (IRLML6344 + flyback diode) are **not fitted** (user,
2026-09-17). Firmware contains no PWM or haptic code (DEC-006); the six motor GPIOs
are driven LOW as the first action in `setup()`.

## Known hardware risks

- **PROB-004 (unverified):** GPIO43 is UART0 TX, and the ESP32-S3 ROM prints its
  boot log on it before firmware runs. Once drivers are fitted, the M5 gate may see
  that serial waveform at every reset. Measure with a logic probe before fitting
  drivers.
- **Fixed sensor ranges (doc 00):** ±4 g and ±500 °/s may clip foot impacts and fast
  swing. The M3 recordings measure how often samples saturate before any change is
  proposed.
