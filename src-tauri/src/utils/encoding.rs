use encoding_rs::{Encoding, UTF_8, WINDOWS_1251};

pub fn detect_encoding(buffer: &[u8]) -> &'static Encoding {
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