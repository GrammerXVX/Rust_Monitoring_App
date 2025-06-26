use encoding_rs::{Encoding, UTF_8, WINDOWS_1251};
use regex::Regex;

lazy_static::lazy_static! {
    static ref LOG_LEVEL_REGEX: Regex = Regex::new(
        r#"(?x)
        \b(ERROR|WARNING|WARN|INFO|DEBUG|TRACE|CRIT|FATAL|ALERT|EMERG|PANIC|NOTICE)\b|
        trace[\d\-_]+|
        warn[\d\-_]+|
        error[\d\-_]+|
        debug[\d\-_]+|
        crit[\d\-_]+|
        fatal[\d\-_]+|
        emerg[\d\-_]+|
        alert[\d\-_]+|
        panic[\d\-_]+
    "#
    ).unwrap();
}

pub fn detect_encoding(buffer: &[u8]) -> &'static Encoding {
    if buffer.starts_with(&[0xFF, 0xFE]) || buffer.starts_with(&[0xFE, 0xFF]) {
        return UTF_8;
    }
    if std::str::from_utf8(buffer).is_ok() {
        return UTF_8;
    }
    WINDOWS_1251
}

fn normalize_log_level(level: &str) -> String {
    let level_lower = level.to_lowercase();
    if level_lower.starts_with("trace") { return "TRACE".to_string(); }
    if level_lower.starts_with("debug") { return "DEBUG".to_string(); }
    if level_lower.starts_with("info") { return "INFO".to_string(); }
    if level_lower.starts_with("warn") { return "WARNING".to_string(); }
    if level_lower.starts_with("error") || level_lower.starts_with("crit") || level_lower.starts_with("fatal") || level_lower.starts_with("emerg") || level_lower.starts_with("panic") {
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
    if line_lower.contains("error") { return ("ERROR".to_string(), line.to_string()); }
    if line_lower.contains("warn") || line_lower.contains("alert") { return ("WARNING".to_string(), line.to_string()); }
    if line_lower.contains("debug") { return ("DEBUG".to_string(), line.to_string()); }
    if line_lower.contains("trace") { return ("TRACE".to_string(), line.to_string()); }
    ("INFO".to_string(), line.to_string())
}