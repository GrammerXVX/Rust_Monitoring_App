#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod monitoring;
mod state;
mod utils;

use state::{system::SystemMonitorState};
use tauri::{generate_context};
use std::sync::{Arc, Mutex};
use nvml_wrapper;
use sysinfo::{CpuRefreshKind, RefreshKind, System, SystemExt};
use systemstat::{Platform, System as Stats};

use crate::monitoring::file_monitor::FileMonitorState;

fn main() {
    println!("[INIT] Starting application");
    
    let sys_monitor = SystemMonitorState {
        sys: Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        )),
        stats: Stats::new(),
        nvml: nvml_wrapper::Nvml::init().ok().map(Arc::new),
    };
    
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(sys_monitor)
        .manage(monitoring::file_monitor::MonitoringState {
            state: Arc::new(Mutex::new(FileMonitorState {
                is_running: false,
                current_file: None,
                current_offset: 0,
                last_modified: None,
                initial_hash: None,
            })),
        })
        .manage(Arc::new(state::logs::LoadingState {
            cancel_token: Mutex::new(None),
            is_loading: Mutex::new(false),
        }))
        .invoke_handler(tauri::generate_handler![
            commands::logs::set_current_file,
            commands::logs::start_file_monitoring,
            commands::logs::stop_file_monitoring,
            commands::logs::start_file_loading,
            commands::system::get_system_info,
            commands::logs::get_current_file,
            commands::logs::is_loading,
            commands::logs::cancel_file_loading
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}