# EAD — Error Augmentation System for Stroke Gait
## Coding-Agent Handoff — V1.0 FINALIZED IMPLEMENTATION CONTRACT

**Purpose:** This directory is the authoritative handoff for the coding agent implementing the V1 right-leg wearable + desktop research dashboard.

## Non-negotiable rule
The coding agent MUST NOT invent, substitute, reinterpret, or silently change an engineering decision covered by these documents. When code behavior is not explicitly described, use the implementation in this specification as the default. Do not introduce new sensors, driver ICs, GPIOs, power-management features, clinical thresholds, or communication mechanisms without an explicit change to this specification.

## V1 identity
- Right leg only.
- Barefoot use.
- 2 × MPU6050 IMUs: one on the right foot dorsum/midfoot, one on the right lower shank.
- 6 × ERM coin vibration motors on a circumferential lower-shank haptic band.
- ESP32-S3 is the real-time controller.
- Wi-Fi is the primary desktop link.
- Tauri 2 + React + TypeScript + Rust is the desktop application.
- No BLE dependency in V1.
- No FSRs.
- No BNO086 in V1. BNO086 is future hardware only.
- No battery ADC/percentage monitoring in V1.
- No charger/power-management logic in firmware.
- Haptics are driven directly by ESP32 PWM through six IRLML6344 MOSFET channels.
- ESP32 remains in the haptic control loop even when Wi-Fi is unavailable.

## Hardware decisions fixed in this package
- Foot MPU6050 I²C address: `0x68`.
- Shank MPU6050 I²C address: `0x69`.
- I²C bus: 400 kHz.
- Sensor sample rate: 100 Hz.
- Accelerometer range: ±4 g.
- Gyroscope range: ±500 °/s.
- MPU6050 DLPF: 42 Hz (DLPF_CFG=3).
- Sample-rate divider: 9 from the 1 kHz gyro output path.
- MPU clock source: PLL with X-axis gyro reference.
- On-board MPU breakout I²C pull-ups are used; do not add another pull-up network unless a board is verified to lack pull-ups.
- Coordinate system: X forward toward toes, Y medial/left, Z up.
- Both MPU boards MUST be mounted with the same physical axis orientation.
- Mahony orientation estimator is the V1 orientation solution. No magnetometer and no absolute yaw claim.
- Foot-only ZUPT detector using combined acceleration/gyro/statistical timing checks.
- Gait detection uses a finite-state machine with adaptive robust thresholds derived from the current/reference walking data.
- Per-cycle/per-step error score is normalized to [0,1]. Confidence is independently normalized to [0,1].
- Haptic feedback uses continuous reassessment with hysteresis/deadband, confidence gating and hard safety limits.
- Haptic PWM: 200 Hz, 8-bit, active duty 20–80% (`51–204`).
- Haptic maximum continuous ON time per motor: 5 s.
- Per-motor rolling duty limit: 50% over any 10 s window.
- Local recovery storage: ESP32 internal flash using LittleFS and append-only binary blocks; only current/latest recoverable session is retained on-device.
- WebSocket transport: ESP32 local AP, device IP `192.168.4.1`, TCP port `8080`, endpoint `/ws`, binary framed protocol.
- Segment limits are entered by the researcher; the firmware does not invent a step/error limit.
- Battery monitoring and charger control are explicitly out of scope.

## Authoritative document order
1. `01_SYSTEM_SPEC.md`
2. `02_HARDWARE_WIRING.md`
3. `03_GPIO_PIN_MAP.md`
4. `04_SENSOR_CALIBRATION_AND_ORIENTATION.md`
5. `05_GAIT_AND_ZUPT_ALGORITHM.md`
6. `06_ERROR_AND_HAPTIC_ENGINE.md`
7. `07_FIRMWARE_ARCHITECTURE.md`
8. `08_WIRELESS_PROTOCOL.md`
9. `09_STORAGE_AND_BINARY_FORMAT.md`
10. `10_DATA_SCHEMA_AND_EXPORT.md`
11. `11_DASHBOARD_SPEC.md`
12. `12_REFERENCE_PATIENT_SESSION_WORKFLOW.md`
13. `13_TEST_AND_VALIDATION_PLAN.md`
14. `14_IMPLEMENTATION_PLAN.md`
15. `15_DECISION_LOG_AND_TRACEABILITY.md`
16. `16_RESEARCH_REFERENCES.md`

`reference_images/RIGHT_LEG_IMU_PLACEMENT.png` is the required physical-placement visual and is part of the implementation contract.

## Source requirements incorporated
The project specification requires a wearable dual-IMU system, six-channel vibrotactile feedback, 100 Hz IMU data, gait speed/cadence/symmetry, stance/swing, foot-drop-like detection, knee-angle estimation in the original concept, heel-strike/toe-off logging, desktop real-time plots, PDF reports and CSV/MATLAB export. The V1 hardware limitation is two right-leg IMUs, so this handoff explicitly does **not** claim direct knee-angle measurement; true knee-angle measurement requires thigh sensing. fileciteturn9file1L69-L96

The later clinical requirements make four points core: distinguish true gait errors from sensor noise, prevent session-long drift using ZUPT, produce one quantitative per-step deviation value that drives vibration strength, calibrate to each patient's own walking instead of a generic healthy reference, and export full raw timestamped accelerometer/gyro/orientation data at native rate. fileciteturn9file0L9-L49 fileciteturn9file0L51-L59

## External research used for haptic placement
Peer-reviewed gait-haptics work supports placing vibrotactile actuators on the lower leg/shank rather than in the shoe/under the sole; a six-vibrotactor shank array has been used to cover the shank from front to back, and other gait-retraining work uses medial/lateral shank placement for directional cues. These findings informed the V1 six-motor circumferential layout. See `16_RESEARCH_REFERENCES.md`.
