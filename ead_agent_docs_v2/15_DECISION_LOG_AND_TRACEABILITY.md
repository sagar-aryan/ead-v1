# 15 — Decision Log and Traceability

## 1. Final decisions
| Topic | Final V1 decision |
|---|---|
| Limb | Right leg only |
| Footwear | Barefoot |
| IMUs | 2 × MPU6050 |
| Foot IMU | Dorsum/midfoot |
| Shank IMU | Anterior/anteromedial lower shank, 10–15 cm below knee |
| Addresses | Foot 0x68, Shank 0x69 |
| I²C | 400 kHz, onboard breakout pull-ups |
| Sample rate | 100 Hz |
| Accel range | ±4 g |
| Gyro range | ±500 °/s |
| DLPF | 42 Hz |
| Orientation | Mahony 6-DoF |
| Magnetometer | None |
| ZUPT | Foot-only, IMU-only, error-state update |
| FSR | Not used |
| Haptics | 6 × ERM, direct PWM/MOSFET |
| Motor driver | IRLML6344 ×6 |
| Flyback | 1N5819W ×6 |
| Gate resistor | 100 Ω ×6 |
| Gate pulldown | 100 kΩ ×6 |
| Motor band | Circumferential lower shank, 6 positions |
| PWM | 200 Hz, 8-bit |
| PWM limits | 20–80% |
| Haptic max continuous | 5 s |
| Rolling duty limit | 50% / 10 s |
| Error score | 0–1 weighted robust deviation |
| Confidence | 0–1 quality score |
| Error classes | 6 defined V1 classes |
| Reference | Patient-specific, versioned, locked during evaluation |
| Bilateral symmetry | Not directly measurable; unilateral cycle proxy used |
| Knee angle | Not directly measured in V1 |
| Wireless | Wi-Fi AP + WebSocket |
| BLE | Not required |
| Desktop | Tauri 2 + React + TypeScript + Rust |
| Local storage | ESP32 internal flash + LittleFS |
| Raw storage | Binary blocks, current/latest recoverable session |
| Battery ADC | None |
| Switch status | None |
| Charger/power control in firmware | None |
| Segmentation | Researcher enters cycle/error limits; whichever comes first |
| Direct motor control from dashboard | Forbidden during patient RUNNING |

## 2. Deliberately excluded hardware/software
- BNO086 for V1.
- BNO055.
- DRV2605L.
- TCA9548A.
- FSRs.
- battery fuel gauge.
- battery ADC.
- switch input monitoring.
- BLE dependency.
- ESP32-served dashboard UI.
- direct laptop-to-motor control.

## 3. Requirement traceability
### Original project
The original project calls for a wearable dual-IMU, 6-channel vibrotactile array, 100 Hz data, wireless streaming, desktop Windows/Linux dashboard, gait speed/cadence/symmetry, stance/swing, foot-drop-like detection, knee-angle estimation, heel-strike/toe-off, real-time plots, PDF reports, and CSV/MATLAB export. fileciteturn9file1L69-L96

### Later clinical requirements
The clinical additions require: noise-vs-error discrimination, drift correction with ZUPT, a single per-step/cycle error number driving haptic intensity, patient-specific baseline rather than generic healthy gait, and full raw timestamped accel/gyro/orientation export for validation. fileciteturn9file0L9-L49 fileciteturn9file0L51-L59

## 4. Historical decisions superseded
Any earlier discussion using BNO055/BNO086 as the active V1 sensor, FSRs, BLE as the primary link, DRV2605L/TCA9548A, or a laptop-controlled haptic loop is superseded by this package.

## 5. Coding-agent behavior rule
When an implementation choice is not obvious from code context, first look in this directory. Do not choose a different value merely because a library's default is different. Treat every numeric value in this handoff as intentional unless explicitly tagged as researcher-configurable.
