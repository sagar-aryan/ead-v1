# 07 — Firmware Architecture

## 1. Runtime components
```text
hardware/
  mpu6050
  pwm_haptics
  gpio
  timer

sensor/
  acquisition
  calibration
  filtering
  orientation

gait/
  state_machine
  events
  zupt
  metrics

reference/
  profile
  builder
  loader

error/
  score
  confidence
  classifer
  hysteresis

storage/
  littlefs
  block_writer
  recovery
  session_manager

protocol/
  websocket
  codec
  commands

safety/
  sensor_watchdog
  haptic_limits

app_state/
  device_state
```

## 2. Timing model
Use a deterministic 100 Hz acquisition timer as the timing backbone.
- Sensor interrupt indicates data ready.
- Acquisition task reads both sensors in a synchronized frame as closely as practical.
- Every frame receives one master ESP32 monotonic timestamp and one frame sequence number.
- Long operations are queued outside the acquisition path.

## 3. Processing priority
Highest priority:
1. sensor acquisition;
2. safety/fault checks;
3. gait/orientation update;
4. haptic control;
5. storage block flushing;
6. Wi-Fi/network telemetry.

Wi-Fi or storage must never block acquisition or haptic control.

## 4. Sensor abstraction
Define a common `ImuSample` interface:
```text
sensor_id
imu_timestamp_us
ax ay az
 gx gy gz
quaternion
calibration_state
health_flags
```
The current concrete implementation is MPU6050. A future BNO086 backend can produce the same interface without changing gait/error algorithms.

## 5. No battery logic
Do not add battery ADC code, percentage estimators, charger-state parsing, fuel-gauge code, or switch-status handling.

## 6. Device states
```text
BOOT
SELF_TEST
CALIBRATING
REFERENCE_CAPTURE
READY
RUNNING
PAUSED
FAULT
RECOVERY
```
Motor outputs are OFF in all states except `RUNNING` when haptic conditions are valid.

## 7. Motor service/test mode
The desktop app may request a service test only when the device is not in a patient evaluation session.
- one motor at a time by default;
- all-six test sequence allowed only in service mode;
- hard safety limits still apply;
- test events are marked as `SERVICE_TEST` and excluded from gait/haptic outcome statistics.
