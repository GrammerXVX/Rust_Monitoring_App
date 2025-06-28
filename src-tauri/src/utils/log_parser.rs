use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

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
pub fn normalize_log_level(level: &str) -> String {
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
pub fn extract_log_level(line: &str) -> (String, String) {
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
pub fn count_lines(file_path: &str) -> std::io::Result<usize> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut count = 0;
    let mut buf = Vec::new();

    while reader.read_until(b'\n', &mut buf)? > 0 {
        count += 1;
        buf.clear();
    }
    Ok(count)
}

pub fn is_system_process(pid: u32, name: &str) -> bool {
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