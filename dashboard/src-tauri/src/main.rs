// EAD V1 dashboard Rust backend — Tauri 2 skeleton.
// Binary WebSocket frames follow doc 08 §2 (little-endian):
//   u16 protocol_version | u8 message_type | u8 flags | u32 payload_length
//   | u32 sequence | u64 device_time_us | payload[payload_length]
// Message types (doc 08 §3): 0x01 HELLO … 0x12 SERVICE_TEST.
// Telemetry arrives in 10-frame 100 Hz batches; the backend preserves every
// raw sample and serves ~20 Hz downsampled views to the UI (doc 11 §3).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
enum DeviceState {
    Disconnected,
    Booting,
    Calibrating,
    Reference,
    Ready,
    Running,
    Paused,
    Fault,
    Recovered,
}

#[derive(Debug, Serialize, Clone)]
struct DeviceStatus {
    state: DeviceState,
    ws_url: String,
    last_sequence: u32,
}

/// Placeholder: returns cached device status.
/// Later: return live state parsed from 0x0C STATUS frames.
#[tauri::command]
fn get_status() -> DeviceStatus {
    DeviceStatus {
        state: DeviceState::Disconnected,
        ws_url: "ws://192.168.4.1:8080/ws".to_string(),
        last_sequence: 0,
    }
}

/// Placeholder: asks the device (0x10 BACKFILL_REQUEST) for missing
/// sequence ranges after a reconnect (doc 08 §5), then 0x11 BACKFILL_DATA
/// fills the gaps. Later: implement over the WS task.
#[tauri::command]
fn backfill_request(start_sequence: u32, end_sequence: u32) -> String {
    format!(
        "backfill_request placeholder: sequences {}..={} (not yet sent to device)",
        start_sequence, end_sequence
    )
}

// --- WS binary codec hook (commented; implemented in a later task) ---
// async fn ws_task(url: &str) {
//     // 1. connect to ws://192.168.4.1:8080/ws (ESP32 AP, doc 08 §1)
//     // 2. read exactly 16-byte header (2+1+1+4+4+8), little-endian:
//     //      protocol_version: u16 (expect 1)
//     //      message_type:     u8  (0x08 RAW batch, 0x09 EVENT, 0x0A STEP,
//     //                            0x0B HAPTIC, 0x0C STATUS, …)
//     //      flags:            u8
//     //      payload_length:   u32
//     //      sequence:          u32  // gap detection -> backfill_request
//     //      device_time_us:    u64  // canonical time (doc 08 §6)
//     // 3. read payload_length bytes; dispatch by message_type:
//     //      - RAW_SAMPLE_BATCH: append all 10 frames to the raw store
//     //      - STATUS: update DeviceState, emit to frontend
//     //      - BACKFILL_DATA: merge missing ranges, verify CRCs
//     // 4. on disconnect: keep gait/haptics running on device (doc 08 §5),
//     //    show DISCONNECTED, resume + backfill on reconnect.
// }

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_status, backfill_request])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
