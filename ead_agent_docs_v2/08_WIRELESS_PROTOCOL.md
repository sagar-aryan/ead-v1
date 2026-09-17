# 08 — Wi-Fi and WebSocket Protocol

## 1. Network
- ESP32-S3 runs a local Wi-Fi access point.
- Device IPv4: `192.168.4.1`.
- WebSocket TCP port: `8080`.
- Endpoint: `ws://192.168.4.1:8080/ws`.
- BLE is not required for V1.

## 2. Frame format
All frames are binary little-endian:
```text
u16 protocol_version
u8  message_type
u8  flags
u32 payload_length
u32 sequence
u64 device_time_us
payload[payload_length]
```
Protocol version V1 = `1`.

## 3. Message types
- `0x01 HELLO`
- `0x02 CONFIG_GET`
- `0x03 CONFIG_SET`
- `0x04 SESSION_START`
- `0x05 SESSION_STOP`
- `0x06 PAUSE`
- `0x07 RESUME`
- `0x08 RAW_SAMPLE_BATCH`
- `0x09 EVENT_BATCH`
- `0x0A STEP_BATCH`
- `0x0B HAPTIC_BATCH`
- `0x0C STATUS`
- `0x0D ACK`
- `0x0E ERROR`
- `0x0F RECOVERY_INFO`
- `0x10 BACKFILL_REQUEST`
- `0x11 BACKFILL_DATA`
- `0x12 SERVICE_TEST`

## 4. Telemetry batching
The ESP32 sends raw sample telemetry in batches of **10 synchronized 100 Hz frames** (nominally 100 ms worth of data).
The dashboard may render at 20 Hz or lower, but must preserve all stored raw samples.

## 5. Reconnection
When Wi-Fi is lost:
- gait processing continues;
- haptics continue;
- current data continues to LittleFS;
- the desktop shows `DISCONNECTED`;
- when reconnected, the desktop receives current state and requests missing sequence ranges from the device when available.

## 6. Time
ESP32 monotonic time is canonical for device data. Laptop time is not substituted into raw records.

## 7. Command permissions
Allowed remote commands:
- get device status;
- select/create reference profile;
- configure researcher parameters;
- start/stop/pause session;
- request exports/backfill;
- enter service-test mode when no patient run is active.

Not allowed remotely during `RUNNING`:
- direct arbitrary PWM;
- direct arbitrary motor activation;
- modification of safety limits;
- changing sensor addresses/configuration;
- changing coordinate convention;
- disabling sensor/fault interlocks.
