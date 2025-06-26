#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::Local;
use encoding_rs::{Encoding, UTF_8, WINDOWS_1251};
use nvml_wrapper::{self as nvml};
use regex::Regex;
use serde::Serialize;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};
use sysinfo::{CpuExt, CpuRefreshKind, PidExt, ProcessExt, RefreshKind, System, SystemExt};
use systemstat::{Platform, System as Stats};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

struct TokenGuard<'a>(&'a LoadingState);
const BATCH_SIZE: usize = 568;
impl<'a> Drop for TokenGuard<'a> {
    fn drop(&mut self) {
        let mut token = self.0.cancel_token.lock().unwrap();
        if token.is_some() {
            println!("[GUARD] Cleaning up token");
            *token = None;
        }
    }
}
struct LoadingState {
    cancel_token: Mutex<Option<CancellationToken>>,
    is_loading: Mutex<bool>, // Добавляем флаг загрузки
}
struct SystemMonitorState {
    sys: Mutex<System>,
    stats: Stats,
    nvml: Option<Arc<nvml::Nvml>>,
}
struct FileMonitorState {
    is_running: bool,
    current_file: Option<String>,
    current_offset: u64,
    initial_hash: Option<[u8; 32]>,
    last_modified: Option<SystemTime>, // Добавляем сохранение позиции
}
struct MonitoringState {
    state: Arc<Mutex<FileMonitorState>>,
}
#[derive(Serialize, Clone)]
pub struct SystemInfo {
    cpu_usage: f32,
    total_memory: u64,
    used_memory: u64,
    cpu_temp: Option<f32>,
    cpu_name: Option<String>,
    gpu_name: Option<String>,
    gpu_temp: Option<u32>,
    gpu_usage: Option<u32>,
    processes: Vec<LocalProcessInfo>,
    selected_process: Option<ProcessDetail>,
}
#[derive(Serialize, Clone)]
pub struct ProcessDetail {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory: u64,
    status: String,
    exe_path: Option<String>,
    command_line: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LocalProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory: u64,
}
#[derive(Clone, Serialize)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
}

lazy_static::lazy_static! {
    static ref LOG_LEVEL_REGEX: Regex = Regex::new(
        r#"(?x)
        \b(ERROR|WARNING|WARN|INFO|DEBUG|TRACE|CRIT|FATAL|ALERT|EMERG|PANIC|NOTICE)\b|
        trace[\d\-_]+|  # Для обработки trace1-8, trace-2 и т.д.
        warn[\d\-_]+|   # Для обработки warn123
        error[\d\-_]+|  # Для обработки error404
        debug[\d\-_]+|  # Для обработки debug5
        crit[\d\-_]+|   # Для обработки crit1
        fatal[\d\-_]+|  # Для обработки fatal2
        emerg[\d\-_]+|  # Для обработки emerg3
        alert[\d\-_]+|  # Для обработки alert4
        panic[\d\-_]+   # Для обработки panic5
    "#
    ).unwrap();
}
fn detect_encoding(buffer: &[u8]) -> &'static Encoding {
    // Простая эвристика для определения кодировки
    if buffer.starts_with(&[0xFF, 0xFE]) || buffer.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 BOM
        return UTF_8; // Будем использовать UTF-8 как fallback
    }

    // Попробуем декодировать как UTF-8
    if let Ok(_) = std::str::from_utf8(buffer) {
        return UTF_8;
    }

    // Попробуем другие распространенные кодировки
    WINDOWS_1251 // Для русской локали
}
fn hash_file_start(file_path: &str, max_bytes: usize) -> std::io::Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(file_path)?;
    let mut buffer = vec![0u8; max_bytes];
    let n = file.read(&mut buffer)?;
    let mut hasher = Sha256::new();
    hasher.update(&buffer[..n]);
    Ok(hasher.finalize().into())
}
#[tauri::command]
fn start_file_loading(
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
fn cancel_file_loading(loading_state: State<'_, Arc<LoadingState>>) {
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
fn count_lines(file_path: &str) -> std::io::Result<usize> {
    let file = File::open(file_path)?; // Открываем новый файловый дескриптор
    let mut reader = BufReader::new(file);
    let mut count = 0;
    let mut buf = Vec::new();

    while reader.read_until(b'\n', &mut buf)? > 0 {
        count += 1;
        buf.clear();
    }

    // Нет необходимости возвращать курсор в начало,
    // так как этот reader будет удален после завершения функции.
    Ok(count)
}

#[derive(Serialize, Clone)]
struct LoadProgress {
    current: usize,
    total: usize,
}

fn is_system_process(pid: u32, name: &str) -> bool {
    if cfg!(target_os = "windows") {
        pid < 100
            || name == "System"
            || name == "Registry"
            || name.contains("svchost")
            || name.contains("dllhost")
    } else if cfg!(target_os = "linux") {
        pid < 1000
            || name == "systemd"
            || name == "kthreadd"
            || name == "ksoftirqd"
            || name.contains("rcu")
            || name == "migration"
    } else {
        false
    }
}
#[tauri::command]
async fn get_system_info(
    state: State<'_, SystemMonitorState>,
    selected_pid: Option<u32>,
) -> Result<SystemInfo, String> {
    {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_all();
    }

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let (cpu_usage, total_memory, used_memory, cpu_name, mut processes, selected_process_detail) = {
        let mut sys = state.sys.lock().unwrap();
        sys.refresh_cpu();

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let cpu_name = sys.global_cpu_info().brand().to_string();

        let mut processes = Vec::new();
        let mut selected_process_detail = None;

        for (pid, process) in sys.processes() {
            let pid_value = pid.as_u32();
            let name = process.name().to_string();

            if is_system_process(pid_value, &name) {
                continue;
            }

            let process_info = LocalProcessInfo {
                pid: pid_value,
                name: name.clone(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            };

            processes.push(process_info);

            if Some(pid_value) == selected_pid {
                selected_process_detail = Some(ProcessDetail {
                    pid: pid_value,
                    name,
                    cpu_usage: process.cpu_usage() / (sys.cpus().len() as f32),
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                    exe_path: Some(process.exe().to_string_lossy().to_string()),
                    command_line: Some(process.cmd().join(" ")),
                });
            }
        }

        processes.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        (
            cpu_usage,
            total_memory,
            used_memory,
            cpu_name,
            processes,
            selected_process_detail,
        )
    };

    let cpu_temp = state.stats.cpu_temp().ok();

    let (gpu_name, gpu_temp, gpu_usage) = match &state.nvml {
        Some(nvml) => match nvml.device_by_index(0) {
            Ok(device) => {
                let name = device.name().ok();
                let temp = device
                    .temperature(nvml::enum_wrappers::device::TemperatureSensor::Gpu)
                    .ok();
                let usage = device.utilization_rates().map(|u| u.gpu).ok();
                (name, temp, usage)
            }
            Err(_) => (None, None, None),
        },
        None => (None, None, None),
    };

    Ok(SystemInfo {
        cpu_usage,
        total_memory,
        used_memory,
        cpu_temp,
        cpu_name: Some(cpu_name),
        gpu_name,
        gpu_temp,
        gpu_usage,
        processes,
        selected_process: selected_process_detail,
    })
}
fn normalize_log_level(level: &str) -> String {
    let level_lower = level.to_lowercase();

    if level_lower.starts_with("trace") {
        return "TRACE".to_string();
    }
    if level_lower.starts_with("debug") {
        return "DEBUG".to_string();
    }
    if level_lower.starts_with("info") {
        return "INFO".to_string();
    }
    if level_lower.starts_with("warn") {
        return "WARNING".to_string();
    }
    if level_lower.starts_with("error")
        || level_lower.starts_with("crit")
        || level_lower.starts_with("fatal")
        || level_lower.starts_with("emerg")
        || level_lower.starts_with("panic")
    {
        return "ERROR".to_string();
    }
    if level_lower.starts_with("alert") || level_lower.starts_with("notice") {
        return "WARNING".to_string();
    }

    match level_lower.as_str() {
        "error" | "crit" | "critical" | "fatal" | "emerg" | "panic" => "ERROR".to_string(),
        "alert" | "notice" => "WARNING".to_string(),
        "info" | "information" => "INFO".to_string(),
        "debug" => "DEBUG".to_string(),
        "trace" => "TRACE".to_string(),
        _ => level.to_uppercase(),
    }
}

fn extract_log_level(line: &str) -> (String, String) {
    if let Some(caps) = LOG_LEVEL_REGEX.captures(line) {
        if let Some(level_match) = caps.iter().find_map(|m| m) {
            let level = level_match.as_str().to_string();
            let normalized_level = normalize_log_level(&level);
            return (normalized_level, line.to_string());
        }
    }

    let line_lower = line.to_lowercase();
    if line_lower.contains("error")
        || line_lower.contains("crit")
        || line_lower.contains("fatal")
        || line_lower.contains("emerg")
        || line_lower.contains("panic")
    {
        return ("ERROR".to_string(), line.to_string());
    }
    if line_lower.contains("warn") || line_lower.contains("alert") {
        return ("WARNING".to_string(), line.to_string());
    }
    if line_lower.contains("debug") {
        return ("DEBUG".to_string(), line.to_string());
    }
    if line_lower.contains("trace") {
        return ("TRACE".to_string(), line.to_string());
    }
    if line_lower.contains("info") {
        return ("INFO".to_string(), line.to_string());
    }
    ("INFO".to_string(), line.to_string())
}
#[tauri::command]
fn set_current_file(path: String, state: State<'_, MonitoringState>) {
    let mut monitor = state.state.lock().unwrap();
    monitor.current_file = Some(path);
    monitor.current_offset = 0;
    monitor.initial_hash = None;        // <--- добавь сброс!
    monitor.last_modified = None; 
}
#[tauri::command]
fn is_loading(loading_state: State<'_, Arc<LoadingState>>) -> bool {
    let flag = loading_state.is_loading.lock().unwrap();
    *flag
}

const SLEEP_DURATION: Duration = Duration::from_millis(200);
type SharedState = Arc<Mutex<MonitoringState>>;

/// Запускает мониторинг указанного файла логов.
#[tauri::command]
fn start_file_monitoring(
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

fn run_monitoring_loop(
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
        // Проверка условий на продолжение
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

        // Обрезка файла
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

        // Новые данные
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
        return Ok(()); // ничего не прочитали
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

fn get_file_size(path: &Path) -> u64 {
    path.metadata().map(|m| m.len()).unwrap_or(0)
}

#[tauri::command]
fn stop_file_monitoring(state: State<'_, MonitoringState>) {
    println!("[STOP] Stopping file monitoring");
    let mut monitor = state.state.lock().unwrap();
    monitor.is_running = false;
    // Не сбрасываем current_file и current_offset!
}
#[tauri::command]
fn get_current_file(state: State<'_, MonitoringState>) -> Option<String> {
    let monitor = state.state.lock().unwrap();
    monitor.current_file.clone()
}
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
        .manage(MonitoringState {
            state: Arc::new(Mutex::new(FileMonitorState {
                is_running: false,
                current_file: None,
                current_offset: 0,
                last_modified: None,
                initial_hash: None,
            })),
        })
        .manage(Arc::new(LoadingState {
            cancel_token: Mutex::new(None),
            is_loading: Mutex::new(false),
        }))
        .invoke_handler(tauri::generate_handler![
            set_current_file,
            start_file_monitoring,
            stop_file_monitoring,
            start_file_loading,
            get_system_info,
            get_current_file,
            is_loading,
            cancel_file_loading
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
