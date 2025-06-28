use crate::{
    models::log_entry::LogEntry,
    monitoring::file_monitor::{get_file_size, run_monitoring_loop, MonitoringState},
    state::logs::*,
    utils::{encoding::detect_encoding, hashing::hash_file_start, log_parser::{count_lines, extract_log_level}},
};
use chrono::Local;
use std::{
    fs::File,
    io::{BufRead, BufReader,Seek, SeekFrom},
    path::Path,
    sync::{Arc},
    thread,
    time::SystemTime,
};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

const BATCH_SIZE: usize = 568;
#[tauri::command]
pub fn start_file_loading(
    app_handle: tauri::AppHandle,
    state: State<'_, MonitoringState>,
    loading_state: State<'_, Arc<LoadingState>>,
    reload_all: bool,
    file_path: String,
) -> Result<(), String> {
    // Ставим флаг что начали загрузку
    {
        let mut fl = loading_state.is_loading.lock().unwrap();
        *fl = true;
    }

    // Подготавливаем общие данные
    let start_offset = {
        let mut mon = state.state.lock().unwrap();

        if reload_all || mon.current_file.as_deref() != Some(&file_path) {
            // сброс оффсета если указан reload_all или новый файл
            mon.current_file = Some(file_path.clone());
            mon.current_offset = 0;
            0
        } else {
            mon.current_offset
        }
    };
    log::info!(
        "start_file_loading: reload_all = {}, start_offset = {}",
        reload_all,
        start_offset
    );
    let cancel_token = CancellationToken::new();
    {
        let mut tok = loading_state.cancel_token.lock().unwrap();
        *tok = Some(cancel_token.clone());
    }
    let app = app_handle.clone();
    let mon_state = state.state.clone();
    let loading_st = loading_state.inner().clone();

    std::thread::spawn(move || {
        // Открываем и сразу ставим оффсет
        let file = match File::open(&file_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit("loading_error", e.to_string());
                let mut fl = loading_st.is_loading.lock().unwrap();
                *fl = false;
                return;
            }
        };
        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(e) => {
                let _ = app.emit(
                    "loading_error",
                    format!("Failed to get file metadata: {}", e),
                );
                let mut fl = loading_st.is_loading.lock().unwrap();
                *fl = false;
                return;
            }
        };
        let file_size = metadata.len();
        let file_modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let current_hash = match hash_file_start(&file_path, 1024) {
            Ok(h) => h,
            Err(e) => {
                let _ = app.emit("loading_error", format!("Failed to hash file: {}", e));
                let mut fl = loading_st.is_loading.lock().unwrap();
                *fl = false;
                return;
            }
        };

        let latest_hash = match hash_file_start(&file_path, 1024) {
            Ok(h) => h,
            Err(_) => current_hash,
        };

        let mut mon = mon_state.lock().unwrap();
        let hash_changed = mon.initial_hash.map_or(true, |h| h != latest_hash);
        let was_truncated = mon.current_offset > file_size;

        let force_reload = was_truncated || hash_changed;

        if force_reload {
            log::info!(
                "Reload triggered: truncated = {}, hash_changed = {}",
                was_truncated,
                hash_changed
            );
            mon.current_offset = 0;
            mon.initial_hash = Some(latest_hash);
            mon.last_modified = Some(file_modified);
            drop(mon); // Разлочим до чтения

            let _ = app.emit("file_truncated", ());
        }

        // ⚠️ Сообщаем фронту, что нужно очистить старые строки
        if force_reload {
            let _ = app.emit("file_truncated", ());
        }
        {
            let mut mon = mon_state.lock().unwrap();
            mon.last_modified = Some(file_modified);
        }
        {
            let mut mon = mon_state.lock().unwrap();
            if mon.current_offset > file_size {
                log::info!(
                    "Detected file truncation: offset {} > filesize {}, resetting to 0",
                    mon.current_offset,
                    file_size
                );
                mon.current_offset = 0;
            }
        }
        let mut reader = BufReader::new(file);
        if let Err(e) = reader.seek(SeekFrom::Start(start_offset)) {
            let _ = app.emit("loading_error", e.to_string());
            let mut fl = loading_st.is_loading.lock().unwrap();
            *fl = false;
            return;
        }
        if file_size == 0 {
            let _ = app.emit("loading_error", "File is empty or unreadable");
            let mut fl = loading_st.is_loading.lock().unwrap();
            *fl = false;
            return;
        }

        // Определяем total_lines только на первой загрузке
        let total = if start_offset == 0 {
            count_lines(&file_path).unwrap_or(0)
        } else {
            0
        };
        let _ = app.emit("load_progress", LoadProgress { current: 0, total });

        let mut buf = Vec::new();
        let mut count = 0;
        let mut batch = Vec::new();

        loop {
            // проверяем отмену
            if cancel_token.is_cancelled() {
                let _ = app.emit("loading_cancelled", ());
                let mut fl = loading_st.is_loading.lock().unwrap();
                *fl = false;
                return;
            }

            buf.clear();
            if !reload_all {
                let latest_hash = match hash_file_start(&file_path, 1024) {
                    Ok(h) => h,
                    Err(_) => current_hash, // если ошибка, не сбрасываем
                };

                let mut mon = mon_state.lock().unwrap();
                if mon.initial_hash != Some(latest_hash) {
                    log::info!("Detected inline change, forcing reload");
                    mon.initial_hash = Some(latest_hash);
                    mon.current_offset = 0;
                    let _ = app.emit("file_truncated", ());
                    drop(mon);
                    // Перезапуск всей загрузки — просто возвращаемся
                    let mut fl = loading_st.is_loading.lock().unwrap();
                    *fl = false;
                    return;
                }
            }
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    count += 1;
                    // обновляем offset
                    {
                        let mut mon = mon_state.lock().unwrap();
                        mon.current_offset += n as u64;
                    }
                    // прогресс
                    if count % BATCH_SIZE == 0 || count == total {
                        let _ = app.emit(
                            "load_progress",
                            LoadProgress {
                                current: count,
                                total,
                            },
                        );
                    }
                    // парсим
                    let (cow, _, err) = detect_encoding(&buf).decode(&buf);
                    if err {
                        log::warn!("Invalid chars");
                    }
                    let line = cow.into_owned();
                    if !line.trim().is_empty() {
                        let (lvl, msg) = extract_log_level(line.trim());
                        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        batch.push(LogEntry {
                            timestamp: now,
                            level: lvl,
                            message: msg.to_string(),
                        });
                    }
                    if batch.len() >= BATCH_SIZE {
                        let _ = app.emit("new_logs_batch", batch.clone());
                        batch.clear();
                    }
                }
                Err(e) => {
                    let _ = app.emit("loading_error", e.to_string());
                    let mut fl = loading_st.is_loading.lock().unwrap();
                    *fl = false;
                    return;
                }
            }
        }
        // финальный batch
        if !batch.is_empty() {
            let _ = app.emit("new_logs_batch", batch);
        }
        if count == 0 && start_offset > 0 {
            log::info!("No new lines detected since last load");
            let _ = app.emit("loading_already_loaded", ());
        }
        // успешно завершили
        let _ = app.emit("loading_success", ());
        let mut fl = loading_st.is_loading.lock().unwrap();
        *fl = false;
    });

    // сразу возвращаемся во фронт, загрузка идёт в фоне
    Ok(())
}
#[tauri::command]
pub fn cancel_file_loading(loading_state: State<'_, Arc<LoadingState>>) {
    println!("[CANCEL] Command received");

    let mut guard = loading_state.cancel_token.lock().unwrap();
    println!("[CANCEL] Lock acquired");

    if let Some(token) = guard.take() {
        println!("[CANCEL] Token found, cancelling...");
        token.cancel();
        println!("[CANCEL] Token cancelled successfully");
    } else {
        println!("[CANCEL] No token found. Trying to cancel anyway");
        // Попробуем создать новый токен и отменить его
        // Это нужно для обработки крайних случаев
        let new_token = CancellationToken::new();
        new_token.cancel();
        *guard = Some(new_token);
        println!("[CANCEL] Fallback token created and cancelled");
    }
    let mut is_loading = loading_state.is_loading.lock().unwrap();
    *is_loading = false;
}
#[tauri::command]
pub fn set_current_file(path: String, state: State<'_, MonitoringState>) {
    let mut monitor = state.state.lock().unwrap();
    monitor.current_file = Some(path);
    monitor.current_offset = 0;
    monitor.initial_hash = None;        // <--- добавь сброс!
    monitor.last_modified = None; 
}
#[tauri::command]
pub fn is_loading(loading_state: State<'_, Arc<LoadingState>>) -> bool {
    let flag = loading_state.is_loading.lock().unwrap();
    *flag
}
#[tauri::command]
pub fn start_file_monitoring(
    file_path: String,
    state: State<'_, MonitoringState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    println!("[MONITOR] Request to monitor: {}", file_path);

    let mut monitor = state.state.lock().unwrap();

    if monitor.is_running && monitor.current_file.as_ref() == Some(&file_path) {
        println!("[MONITOR] Already monitoring this file.");
        return Ok(());
    }

    let initial_offset = if monitor.current_file.as_ref() == Some(&file_path) {
        monitor.current_offset
    } else {
        get_file_size(Path::new(&file_path))
    };

    println!("[MONITOR] Initial offset: {}", initial_offset);

    monitor.is_running = true;
    monitor.current_file = Some(file_path.clone());
    monitor.current_offset = initial_offset;

    let state_clone = state.state.clone();
    let app_handle_clone = app_handle.clone();

    thread::spawn(move || {
        run_monitoring_loop(file_path, state_clone, app_handle_clone, initial_offset);
    });

    Ok(())
}
#[tauri::command]
pub fn stop_file_monitoring(state: State<'_, MonitoringState>) {
    println!("[STOP] Stopping file monitoring");
    let mut monitor = state.state.lock().unwrap();
    monitor.is_running = false;
    // Не сбрасываем current_file и current_offset!
}
#[tauri::command]
pub fn get_current_file(state: State<'_, MonitoringState>) -> Option<String> {
    let monitor = state.state.lock().unwrap();
    monitor.current_file.clone()
}