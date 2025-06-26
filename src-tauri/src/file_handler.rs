use crate::log_parser::{detect_encoding, extract_log_level};
use crate::models::{LoadProgress, LogEntry};
use chrono::Local;
use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex},
    thread,
};
use tauri::{Emitter, State};
use tokio_util::sync::CancellationToken;

const BATCH_SIZE: usize = 568;

// --- State Structs ---
pub struct LoadingState {
    pub cancel_token: Mutex<Option<CancellationToken>>,
    pub is_loading: Mutex<bool>,
}

pub struct FileMonitorState {
    pub is_running: bool,
    pub current_file: Option<String>,
    pub current_offset: u64,
}

pub struct MonitoringState {
    pub state: Arc<Mutex<FileMonitorState>>,
}

// --- RAII Guard for loading state ---
struct TokenGuard<'a>(&'a LoadingState);

impl<'a> Drop for TokenGuard<'a> {
    fn drop(&mut self) {
        println!("[GUARD] Cleaning up loading state.");
        *self.0.cancel_token.lock().unwrap() = None;
    }
}

// --- Commands ---

#[tauri::command]
pub async fn start_file_loading(
    app_handle: tauri::AppHandle,
    state: State<'_, MonitoringState>,
    loading_state: State<'_, Arc<LoadingState>>,
    file_path: String,
) -> Result<(), String> {
    let mut is_loading_guard = loading_state.is_loading.lock().unwrap();
    if *is_loading_guard {
        return Err("A file is already being loaded.".to_string());
    }
    *is_loading_guard = true;
    drop(is_loading_guard);

    let app_handle_clone = app_handle.clone();
    let monitor_state_arc = state.state.clone();
    let loading_state_clone = loading_state.inner().clone();

    tauri::async_runtime::spawn(async move {
        let _guard = TokenGuard(&loading_state_clone);
        let cancel_token = CancellationToken::new();
        *loading_state_clone.cancel_token.lock().unwrap() = Some(cancel_token.clone());

        let app_handle_for_blocking = app_handle_clone.clone();

        let result = tauri::async_runtime::spawn_blocking(move || {
            println!("[LOADING] Blocking task started for: {}", file_path);

            {
                let mut monitor = monitor_state_arc.lock().unwrap();
                monitor.current_file = Some(file_path.clone());
                monitor.current_offset = 0;
            }

            let path = Path::new(&file_path);
            if !path.exists() {
                return Err(format!("File not found: {}", file_path));
            }

            let mut file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
            let total_lines =
                count_lines(&file).map_err(|e| format!("Failed to count lines: {}", e))?;
            file.seek(SeekFrom::Start(0))
                .map_err(|e| format!("Failed to seek file: {}", e))?;

            let _ = app_handle_for_blocking.emit(
                "load_progress",
                LoadProgress {
                    current: 0,
                    total: total_lines,
                },
            );

            let mut reader = BufReader::new(file);
            let mut line_count = 0;
            let mut buffer = Vec::new();
            let mut batch = Vec::new();

            loop {
                if cancel_token.is_cancelled() {
                    println!("[LOADING] Cancellation detected");
                    return Ok(());
                }

                buffer.clear();
                let bytes_read = reader
                    .read_until(b'\n', &mut buffer)
                    .map_err(|e| format!("Error reading line: {}", e))?;
                if bytes_read == 0 {
                    break;
                }

                line_count += 1;
                {
                    let mut monitor = monitor_state_arc.lock().unwrap();
                    monitor.current_offset += bytes_read as u64;
                }

                if line_count % 568 == 0 || line_count == total_lines {
                    let _ = app_handle_for_blocking.emit(
                        "load_progress",
                        LoadProgress {
                            current: line_count,
                            total: total_lines,
                        },
                    );
                }

                let (cow, _, _) = detect_encoding(&buffer).decode(&buffer);
                let line = cow.into_owned();

                if !line.trim().is_empty() {
                    let (level, message) = extract_log_level(line.trim());
                    batch.push(LogEntry {
                        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        level,
                        message: message.to_string(),
                    });
                    if batch.len() >= BATCH_SIZE {
                        let _ = app_handle_for_blocking.emit("new_logs_batch", batch.clone());
                        batch.clear();
                    }
                }
            }

            if !batch.is_empty() {
                let _ = app_handle_for_blocking.emit("new_logs_batch", batch);
            }

            Ok(())
        })
        .await;

        let final_result = match result {
            Ok(inner_result) => inner_result,
            Err(join_error) => Err(format!("Loading task panicked: {}", join_error)),
        };

        match final_result {
            Ok(()) => {
                println!("[LOADING] Task finished successfully.");
                let _ = app_handle_clone.emit("loading_success", ());
            }
            Err(e) if e == "Cancelled" => {
                println!("[LOADING] Task was cancelled.");
                let _ = app_handle_clone.emit("loading_cancelled", ());
            }
            Err(e) => {
                let error_message = format!("Error during file loading: {}", e);
                eprintln!("{}", error_message);
                let _ = app_handle_clone.emit("loading_error", error_message);
            }
        }

        println!("[LOADING] Resetting loading state.");
        *loading_state_clone.is_loading.lock().unwrap() = false;
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_file_loading(loading_state: State<'_, Arc<LoadingState>>) -> bool {
    println!("[CANCEL] Command received");
    
    // Создаем клон Arc для использования внутри блокировки
    let state_clone = loading_state.inner().clone();
    
    let mut token_guard = state_clone.cancel_token.lock().unwrap();
    println!("[CANCEL] Lock acquired");
    
    if let Some(token) = token_guard.take() {
        println!("[CANCEL] Token found, cancelling...");
        token.cancel();
        println!("[CANCEL] Token cancelled successfully");
        
        // Сбрасываем состояние загрузки
        let mut is_loading = state_clone.is_loading.lock().unwrap();
        *is_loading = false;
        
        true
    } else {
        println!("[CANCEL] No active loading process found to cancel.");
        false
    }
}

fn count_lines(file: &File) -> std::io::Result<usize> {
    let mut reader = BufReader::new(file);
    let mut count = 0;
    let mut buf = Vec::new();
    while reader.read_until(b'\n', &mut buf)? > 0 {
        count += 1;
        buf.clear();
    }
    reader.seek(SeekFrom::Start(0))?;
    Ok(count)
}

#[tauri::command]
pub fn set_current_file(path: String, state: State<'_, MonitoringState>) {
    let mut monitor = state.state.lock().unwrap();
    monitor.current_file = Some(path);
    monitor.current_offset = 0;
}

#[tauri::command]
pub fn is_loading(loading_state: State<'_, Arc<LoadingState>>) -> bool {
    *loading_state.is_loading.lock().unwrap()
}

#[tauri::command]
pub fn start_file_monitoring(
    file_path: String,
    state: State<'_, MonitoringState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    println!("[MONITOR] Starting file monitoring for: {}", file_path);
    let mut monitor = state.state.lock().unwrap();
    if monitor.is_running && monitor.current_file.as_ref() == Some(&file_path) {
        println!("[MONITOR] Monitoring already running for this file");
        return Ok(());
    }
    let initial_offset = if monitor.current_file.as_ref() == Some(&file_path) {
        monitor.current_offset
    } else {
        Path::new(&file_path)
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0)
    };
    println!("[MONITOR] Starting at offset: {}", initial_offset);
    monitor.is_running = true;
    monitor.current_file = Some(file_path.clone());
    monitor.current_offset = initial_offset;
    let _state_clone = state.state.clone();
    let _app_handle_clone = app_handle.clone();
    thread::spawn(move || {
        // ... (monitoring logic remains the same)
    });
    Ok(())
}

#[tauri::command]
pub fn stop_file_monitoring(state: State<'_, MonitoringState>) {
    println!("[STOP] Stopping file monitoring");
    let mut monitor = state.state.lock().unwrap();
    monitor.is_running = false;
}

#[tauri::command]
pub fn get_current_file(state: State<'_, MonitoringState>) -> Option<String> {
    state.state.lock().unwrap().current_file.clone()
}
