// EAD V1 desktop dashboard — Tauri 2 shell.
// Device link, storage and export modules arrive in milestone M2 onward
// (docs/handoff.md).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
