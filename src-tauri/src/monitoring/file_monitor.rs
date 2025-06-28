use crate::{
    models::log_entry::LogEntry,
    utils::{encoding::detect_encoding, log_parser::extract_log_level},
};
use chrono::Local;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter};

pub const SLEEP_DURATION: Duration = Duration::from_millis(200);
pub const BATCH_SIZE: usize = 568;

pub struct FileMonitorState {
    pub is_running: bool,
    pub current_file: Option<String>,
    pub current_offset: u64,
    pub initial_hash: Option<[u8; 32]>,
    pub last_modified: Option<SystemTime>,
}

pub struct MonitoringState {
    pub state: Arc<Mutex<FileMonitorState>>,
}
pub fn run_monitoring_loop(
    file_path: String,
    state: Arc<Mutex<FileMonitorState>>,
    app_handle: AppHandle,
    mut offset: u64,
) {
    let path = Path::new(&file_path);

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            emit_error(&app_handle, format!("Failed to open file: {}", e));
            return;
        }
    };

    let mut last_size = get_file_size(path);
    let mut first_cycle = true;

    loop {
        {
            let monitor = state.lock().unwrap();
            if !monitor.is_running || monitor.current_file.as_ref() != Some(&file_path) {
                println!("[MONITOR] Monitoring stopped for: {}", file_path);
                break;
            }
        }

        let metadata = match path.metadata() {
            Ok(m) => m,
            Err(e) => {
                emit_error(&app_handle, format!("Failed to get metadata: {}", e));
                thread::sleep(SLEEP_DURATION);
                continue;
            }
        };

        let current_size = metadata.len();

        if current_size < last_size {
            println!("[MONITOR] File truncated. Resetting offset.");
            if file.seek(SeekFrom::Start(0)).is_ok() {
                offset = 0;
                let mut monitor = state.lock().unwrap();
                monitor.current_offset = 0;
                emit_event(&app_handle, "file_cleared", ());
            }
        }

        last_size = current_size;

        if offset < current_size {
            if !first_cycle {
                if let Err(e) =
                    process_new_data(&mut file, offset, &state, &app_handle, &mut offset)
                {
                    emit_error(&app_handle, e);
                }
            } else {
                first_cycle = false;
            }
        }

        thread::sleep(SLEEP_DURATION);
    }

    println!("[MONITOR] Monitoring thread exited.");
}

fn process_new_data(
    file: &mut File,
    offset: u64,
    state: &Arc<Mutex<FileMonitorState>>,
    app_handle: &AppHandle,
    new_offset: &mut u64,
) -> Result<(), String> {
    let mut buffer = Vec::new();

    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Seek failed: {}", e))?;
    let bytes_read = file
        .read_to_end(&mut buffer)
        .map_err(|e| format!("Read failed: {}", e))?;

    if bytes_read == 0 {
        return Ok(()); 
    }

    *new_offset += bytes_read as u64;

    {
        let mut monitor = state.lock().unwrap();
        monitor.current_offset = *new_offset;
    }

    let encoding = detect_encoding(&buffer);
    let (cow, _enc, had_errors) = encoding.decode(&buffer);
    if had_errors {
        log::warn!("Encoding issues detected");
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let content = cow.into_owned();
    let mut batch = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (level, message) = extract_log_level(trimmed);
        batch.push(LogEntry {
            timestamp: now.clone(),
            level,
            message: message.to_string(),
        });

        if batch.len() >= BATCH_SIZE {
            emit_event(app_handle, "new_logs_batch", batch.clone());
            batch.clear();
        }
    }

    if !batch.is_empty() {
        emit_event(app_handle, "new_logs_batch", batch);
    }

    Ok(())
}

fn emit_error(app_handle: &AppHandle, message: impl ToString) {
    let _ = app_handle.emit("monitoring_error", message.to_string());
}

fn emit_event<T: serde::Serialize + Clone>(app_handle: &AppHandle, event: &str, payload: T) {
    let _ = app_handle.emit(event, payload);
}

pub fn get_file_size(path: &Path) -> u64 {
    path.metadata().map(|m| m.len()).unwrap_or(0)
}