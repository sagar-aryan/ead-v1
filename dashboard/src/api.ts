/**
 * Typed wrapper over the Rust backend commands (`src-tauri/src/app.rs`).
 * These types mirror the Rust structs; the protocol vocabulary itself comes
 * from the backend at runtime so it is defined in one place.
 */
import { invoke } from "@tauri-apps/api/core";
import { Channel } from "@tauri-apps/api/core";

export type LinkTarget =
  | { kind: "usb"; port: string | null }
  | { kind: "wifi"; url: string };

export interface UsbPortInfo {
  port: string;
  /** The device MAC, reported as the USB serial number. */
  serial_number: string | null;
}

export interface DeviceStatus {
  device_state: number;
  link_flags: number;
  faults: number;
  frame_index: number;
  frames_dropped: number;
  shank_repeated: number;
  i2c_errors: number;
  imu_reinits: number;
  oldest_seq: number;
  last_seq: number;
  ap_rssi_dbm: number;
  ap_stations: number;
  heap_free_min: number;
  stack_free_acquisition: number;
  stack_free_processing: number;
  stack_free_usb: number;
  stack_free_wifi: number;
}

export interface Snapshot {
  link_state: "disconnected" | "connecting" | "connected";
  link_description: string | null;
  last_error: string | null;
  device_state: string;
  faults: string[];
  status: DeviceStatus | null;
  firmware: string | null;
  mac: string | null;
  haptics_fitted: boolean;
  capabilities: string[];
  boot_id: number | null;
  schema_mismatch: boolean;
  missing_messages: number;
  rejected_frames: number;
  frames_received: number;
  status_age_ms: number | null;
}

export interface MountMap extends Array<[number, number, number]> {}

export interface DeviceConfig {
  imu: {
    foot_address: number;
    shank_address: number;
    i2c_hz: number;
    sample_hz: number;
    accel_range_g: number;
    gyro_range_dps: number;
    dlpf_hz: number;
    dlpf_cfg: number;
    sample_rate_divider: number;
    accel_lsb_per_g: number;
    gyro_lsb_per_dps: number;
    foot_mount: MountMap;
    shank_mount: MountMap;
  };
  pins: {
    i2c_sda: number;
    i2c_scl: number;
    foot_imu_int: number;
    shank_imu_int: number;
    motors: number[];
  };
  calibration_static_s: number;
  mahony_kp: number;
  mahony_ki: number;
  haptics: { fitted: boolean; pwm_hz: number; motor_positions_deg: number[] };
  ap_ip: number[];
  ws_port: number;
  frames_per_batch: number;
  reference_min_cycles: number;
  reference_check_cycles: number;
}

export interface Vocabulary {
  device_states: string[];
  fault_names: string[];
  raw_status_names: string[];
  default_wifi_url: string;
}

export interface LiveTick {
  device_time_us: number;
  frame_index: number;
  frames: number;
  foot_accel_g: [number, number, number];
  foot_gyro_dps: [number, number, number];
  shank_accel_g: [number, number, number];
  shank_gyro_dps: [number, number, number];
  foot_accel_magnitude_g: number;
  shank_accel_magnitude_g: number;
  status_flags: number;
  /** False while the device configuration is unknown: values are raw counts. */
  anatomical: boolean;
}

export interface Patient {
  patient_id: string;
  name: string;
  created_at: string;
  session_count: number;
}

export interface Session {
  session_id: string;
  patient_id: string;
  patient_name: string;
  kind: string;
  started_at: string;
  stopped_at: string | null;
  firmware: string | null;
  config_sha256: string | null;
  device_mac: string | null;
  boot_id: number | null;
  first_frame_index: number | null;
  last_frame_index: number | null;
  frames_stored: number;
  frames_missing: number;
}

export const api = {
  listUsbPorts: () => invoke<UsbPortInfo[]>("list_usb_ports"),
  connect: (target: LinkTarget) => invoke<void>("connect_device", { target }),
  disconnect: () => invoke<void>("disconnect_device"),
  snapshot: () => invoke<Snapshot>("device_snapshot"),
  config: () => invoke<DeviceConfig | null>("device_config"),
  vocabulary: () => invoke<Vocabulary>("vocabulary"),
  subscribeLive: (channel: Channel<LiveTick>) => invoke<void>("subscribe_live", { channel }),
  unsubscribeLive: () => invoke<void>("unsubscribe_live"),
  createPatient: (patientId: string, name: string) =>
    invoke<Patient>("create_patient", { patientId, name }),
  patients: () => invoke<Patient[]>("patients"),
  startRecording: (patientId: string) => invoke<Session>("start_recording", { patientId }),
  stopRecording: () => invoke<Session | null>("stop_recording"),
  recordingSession: () => invoke<string | null>("recording_session"),
  sessions: () => invoke<Session[]>("sessions"),
  session: (sessionId: string) => invoke<Session>("session", { sessionId }),
};

export { Channel };
