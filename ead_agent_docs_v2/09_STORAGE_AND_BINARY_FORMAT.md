# 09 — Internal Flash Storage and Binary Format

## 1. Storage technology
Use ESP32 internal flash through **LittleFS**.
Do not write one flash record per sample. Use RAM buffering and append block writes.

## 2. Retention policy
- Device stores only the latest/current recoverable session.
- Desktop is the long-term research archive.
- On successful export/hand-off, the desktop may explicitly request cleanup of the completed on-device session.
- Never automatically erase an incomplete session after reboot until recovery is acknowledged or storage is intentionally reset.

## 3. File layout
```text
/ealdat/current.ead
/ealdat/manifest.bin
/ealdat/calibration.bin
```

## 4. Binary block envelope
```text
u32 magic = 0x45414431   // ASCII-like "EAD1"
u8  block_type
u8  block_flags
u16 header_length
u32 payload_length
u32 sequence
u64 start_time_us
u32 payload_crc32
u32 header_crc32
payload[payload_length]
```

## 5. Block types
- `0x01 SESSION_HEADER`
- `0x02 RAW_SAMPLES`
- `0x03 EVENTS`
- `0x04 CYCLES_STEPS`
- `0x05 HAPTICS`
- `0x06 STATUS_FAULTS`
- `0x07 REFERENCE_PROFILE`
- `0x08 SESSION_FOOTER`

## 6. Raw synchronized frame
To conserve flash while preserving timestamped raw data, timestamp each synchronized frame rather than duplicating the timestamp for every sensor:
```text
u64 timestamp_us
u32 frame_index
u16 foot_ax_raw
u16 foot_ay_raw
u16 foot_az_raw
u16 foot_gx_raw
u16 foot_gy_raw
u16 foot_gz_raw
u16 shank_ax_raw
u16 shank_ay_raw
u16 shank_az_raw
u16 shank_gx_raw
u16 shank_gy_raw
u16 shank_gz_raw
i16 qf_w
 i16 qf_x
 i16 qf_y
 i16 qf_z
i16 qs_w
 i16 qs_x
 i16 qs_y
 i16 qs_z
u16 status_flags
```
Raw accel/gyro are stored in native ADC counts; scaling parameters are stored in session metadata. Quaternions are normalized signed Q15 values (`[-32767,+32767]` maps to `[-1,+1]`).

## 7. Block sizing
Target payload block size: **4096 bytes**. Flush when the block reaches the target or at a 500 ms safety interval, whichever occurs first.

## 8. Crash recovery
A block is considered committed only if:
- header CRC is valid;
- payload length is inside file bounds;
- payload CRC is valid.

On reboot:
1. scan the file;
2. stop at the first invalid/incomplete block;
3. truncate the tail to the last valid block;
4. reconstruct session state from valid records;
5. mark session as `RECOVERED_INCOMPLETE`.

## 9. Sequence numbers
Every block has monotonically increasing sequence numbers. Duplicate block numbers are ignored during reconstruction.

## 10. Capacity behavior
Before a new session begins, estimate required storage from current raw format and available flash. If free space falls below 15%, automatically stop accepting new data blocks and enter a storage fault; do not silently delete research data.
