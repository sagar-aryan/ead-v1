# 18 — Coding-Agent Rules

## Do not assume
The agent MUST NOT:
- choose different GPIOs;
- add a battery ADC;
- add switch monitoring;
- add BLE as a dependency;
- add BNO086 support to V1 code paths unless behind the future sensor abstraction and disabled;
- introduce FSR inputs;
- use DRV2605L/TCA9548A;
- place motor control in the desktop app;
- change the coordinate system;
- rename the unilateral symmetry proxy into bilateral symmetry;
- claim knee-angle measurement;
- change haptic safety limits;
- silently change I²C addresses;
- overwrite raw data with filtered data;
- write one flash transaction per sample;
- adapt the reference profile automatically during an evaluation.

## Fixed implementation values
All exact values in `03_GPIO_PIN_MAP.md`, `04_SENSOR_CALIBRATION_AND_ORIENTATION.md`, `05_GAIT_AND_ZUPT_ALGORITHM.md`, `06_ERROR_AND_HAPTIC_ENGINE.md`, `08_WIRELESS_PROTOCOL.md`, and `09_STORAGE_AND_BINARY_FORMAT.md` are implementation requirements.

## Researcher-configurable values
Only the values explicitly labelled researcher-configurable may be entered/changed by the researcher:
- segment cycle/step limit;
- segment error limit;
- reference profile selection/recapture;
- non-safety analysis threshold parameters exposed by the firmware.

## Safety hierarchy
Sensor invalidity -> no haptic. Low confidence -> no new haptic. Device startup/reset -> haptics OFF. Network loss -> local control continues. Storage failure -> no silent data deletion.

## Code quality
- Strong typing for protocol/storage structures.
- Unit tests for every mathematical function with known inputs.
- No global mutable state for gait/error calculations where avoidable.
- Deterministic replay path.
- Version all persisted formats.
- Log firmware version and configuration hash with each session.
