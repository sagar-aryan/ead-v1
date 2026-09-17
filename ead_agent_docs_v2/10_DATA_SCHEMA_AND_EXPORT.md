# 10 — Data Schema and Export

## 1. Required identifiers
- `patient_name` — researcher-entered.
- `patient_id` — researcher-entered unique study identifier.
- `reference_profile_id`.
- `session_id` generated as `YYYYMMDD-HHMMSS-<4hex>`.
- `segment_id` sequential within session.

## 2. Raw data export
`raw.csv` columns:
```text
timestamp_us, frame_index, sensor, ax, ay, az, gx, gy, gz, qw, qx, qy, qz, status_flags
```
The CSV contains one row per IMU sample, with the same synchronized timestamp for foot and shank rows belonging to a frame.

## 3. Gait/cycle export
`gait.csv`:
```text
session_id,segment_id,cycle_id,start_us,end_us,cycle_time_s,stance_s,swing_s,cadence_spm,cycle_distance_m,speed_mps,unilateral_symmetry_proxy,error_score,confidence,primary_error_class,zupt_quality
```

## 4. Event export
`events.csv`:
```text
session_id,segment_id,cycle_id,timestamp_us,event_type,quality
```
Event types:
- `INITIAL_CONTACT`
- `TOE_OFF`
- `FOOT_FLAT`
- `ZUPT_APPLIED`
- `CYCLE_START`
- `CYCLE_END`
- `ERROR_ACTIVE`
- `ERROR_RESOLVED`
- `FAULT`
- `SERVICE_TEST`

## 5. Haptic export
`haptics.csv`:
```text
session_id,segment_id,cycle_id,timestamp_us,motor_ids,error_class,pwm,duration_ms,error_score,confidence,reason
```

## 6. Metadata
`metadata.json` must include:
- firmware version;
- protocol version;
- hardware identifier;
- sensor addresses;
- fixed GPIO map;
- sensor configuration/ranges/DLPF/sample rate;
- calibration parameters and quality;
- coordinate convention;
- reference profile ID/version;
- researcher-entered segmentation thresholds;
- haptic safety configuration;
- storage recovery state;
- session start/stop time.

## 7. MATLAB export
Create a **Level-5 `.mat` file** named `session.mat` with these top-level variables:
- `raw`
- `gait`
- `events`
- `haptics`
- `metadata`
- `reference`

Use numeric arrays/double values for processed measures and cell/string fields only where required. The raw integer values and timestamps must not be lost during conversion.

## 8. PDF clinical/research report
Generate a PDF containing:
- patient/session metadata;
- reference profile version;
- session mean speed;
- cadence;
- stance/swing;
- unilateral symmetry proxy;
- error-score distribution;
- error-class counts;
- haptic response summary;
- ZUPT quality;
- seven trend plots;
- event timeline;
- data quality/fault summary.

Clearly label the document as a research/engineering report, not a validated clinical diagnostic report.

## 9. Traceability
Every cycle row must carry `cycle_id`, and every event/haptic row must reference that cycle where applicable. Plot error markers must be linked to the cycle ID and raw timestamp range.
