/**
 * EAD V1 dashboard shared types.
 * Field names mirror doc 10 (10_DATA_SCHEMA_AND_EXPORT.md) CSV/JSON columns
 * so the UI, export package, and firmware contract stay aligned.
 */

// ---------------------------------------------------------------------------
// Identifiers (doc 10 §1)
// ---------------------------------------------------------------------------
export type DeviceState =
  | "disconnected"
  | "booting"
  | "calibrating"
  | "reference"
  | "ready"
  | "running"
  | "paused"
  | "fault"
  | "recovered";

export type SensorId = "foot" | "shank";

export type EventType =
  | "INITIAL_CONTACT"
  | "TOE_OFF"
  | "FOOT_FLAT"
  | "ZUPT_APPLIED"
  | "CYCLE_START"
  | "CYCLE_END"
  | "ERROR_ACTIVE"
  | "ERROR_RESOLVED"
  | "FAULT"
  | "SERVICE_TEST";

// ---------------------------------------------------------------------------
// raw.csv (doc 10 §2): one row per IMU sample. Foot + shank rows of a frame
// share the same synchronized timestamp_us.
// ---------------------------------------------------------------------------
export interface RawSample {
  timestamp_us: number;
  frame_index: number;
  sensor: SensorId;
  ax: number;
  ay: number;
  az: number;
  gx: number;
  gy: number;
  gz: number;
  qw: number;
  qx: number;
  qy: number;
  qz: number;
  status_flags: number;
}

// ---------------------------------------------------------------------------
// gait.csv (doc 10 §3): one row per gait cycle.
// ---------------------------------------------------------------------------
export interface Cycle {
  session_id: string;
  segment_id: number;
  cycle_id: number;
  start_us: number;
  end_us: number;
  cycle_time_s: number;
  stance_s: number;
  swing_s: number;
  cadence_spm: number;
  cycle_distance_m: number;
  speed_mps: number;
  unilateral_symmetry_proxy: number;
  error_score: number;
  confidence: number;
  primary_error_class: string;
  zupt_quality: number;
}

// ---------------------------------------------------------------------------
// events.csv (doc 10 §4)
// ---------------------------------------------------------------------------
export interface GaitEvent {
  session_id: string;
  segment_id: number;
  cycle_id: number;
  timestamp_us: number;
  event_type: EventType;
  quality: number;
}

// ---------------------------------------------------------------------------
// haptics.csv (doc 10 §5)
// ---------------------------------------------------------------------------
export interface HapticRecord {
  session_id: string;
  segment_id: number;
  cycle_id: number;
  timestamp_us: number;
  motor_ids: number[];
  error_class: string;
  pwm: number;
  duration_ms: number;
  error_score: number;
  confidence: number;
  reason: string;
}

// ---------------------------------------------------------------------------
// metadata.json (doc 10 §6)
// ---------------------------------------------------------------------------
export interface SessionMetadata {
  firmware_version: string;
  protocol_version: number;
  hardware_id: string;
  sensor_addresses: { foot: string; shank: string };
  gpio_map: Record<string, number>;
  sensor_config: {
    sample_rate_hz: number;
    accel_range_g: number;
    gyro_range_dps: number;
    dlpf_hz: number;
  };
  calibration: {
    params: Record<string, number>;
    quality: number;
  };
  coordinate_convention: string;
  reference_profile_id: string;
  reference_profile_version: number;
  segmentation_thresholds: {
    max_valid_cycles_per_segment: number;
    max_errors_per_segment: number;
  };
  haptic_safety: {
    pwm_min: number;
    pwm_max: number;
    max_on_time_s: number;
    rolling_duty_limit_pct: number;
  };
  storage_recovery_state: string;
  session_start_time: string;
  session_stop_time: string;
}

// ---------------------------------------------------------------------------
// Session / Segment hierarchy (docs 10 §1, 11 §SESSIONS, 12 §5)
// ---------------------------------------------------------------------------
export interface Segment {
  segment_id: number;
  session_id: string;
  valid_cycle_count: number;
  error_count: number;
  closed: boolean;
}

export interface Session {
  session_id: string; // YYYYMMDD-HHMMSS-<4hex>
  patient_name: string;
  patient_id: string;
  reference_profile_id: string;
  reference_profile_version: number;
  segments: Segment[];
  metadata: SessionMetadata;
}

// ---------------------------------------------------------------------------
// Reference profile (docs 11 §REFERENCE PROFILES, 12 §§2-3,6)
// Immutable once used for an evaluation session.
// ---------------------------------------------------------------------------
export interface ReferenceProfile {
  reference_profile_id: string;
  version: number;
  patient_id: string;
  valid_cycle_count: number;
  feature_medians: Record<string, number>;
  feature_mads: Record<string, number>;
  locked: boolean;
  created_at: string;
}

// ---------------------------------------------------------------------------
// Live telemetry snapshot + WS link status (docs 08, 11 §LIVE)
// ---------------------------------------------------------------------------
export type WsStatus = "disconnected" | "connecting" | "connected" | "error";

export interface LiveSnapshot {
  patient_name: string;
  patient_id: string;
  reference_profile_id: string;
  session_id: string;
  segment_id: number;
  elapsed_s: number;
  cycle_id: number;
  cadence_spm: number;
  speed_mps: number;
  stance_s: number;
  swing_s: number;
  unilateral_symmetry_proxy: number;
  foot_quat: [number, number, number, number];
  shank_quat: [number, number, number, number];
  error_score: number;
  confidence: number;
  error_class: string;
  zupt_state: string;
  zupt_quality: number;
  sensor_status: string;
  motor_state: string;
}
