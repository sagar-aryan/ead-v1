// EAD V1 desktop dashboard (Tauri 2). Architecture: docs/architecture.md.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(test)]
mod hardware_tests;
mod device;
mod link;
mod live;
mod orientation;
mod protocol;
mod store;

use std::sync::Arc;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|handle| {
            // Research data lives beside the app's other user data.
            let directory = handle.path().app_data_dir()?;
            let store = store::Store::open(directory.join("ead.sqlite3"))?;
            let state = app::App::new(store);
            tauri::async_runtime::spawn(live::run(
                state.live.clone(),
                state.shutdown_signal(),
            ));
            handle.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<Arc<app::App>>() {
                    state.shutdown();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            app::list_usb_ports,
            app::connect_device,
            app::disconnect_device,
            app::device_snapshot,
            app::device_config,
            app::vocabulary,
            app::subscribe_live,
            app::unsubscribe_live,
            app::create_patient,
            app::patients,
            app::start_recording,
            app::stop_recording,
            app::recording_session,
            app::sessions,
            app::session,
            app::raw_window,
            app::cycles,
            app::start_reference_capture,
            app::stop_device_session,
            app::save_reference,
            app::start_scored_session,
            app::references,
            app::events,
            app::start_calibration,
            app::cancel_calibration,
            app::session_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
