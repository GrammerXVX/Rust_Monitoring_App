use encoding_rs::{Encoding, UTF_8, WINDOWS_1251};

pub fn detect_encoding(buffer: &[u8]) -> &'static Encoding {
    if buffer.starts_with(&[0xFF, 0xFE]) || buffer.starts_with(&[0xFE, 0xFF]) {
        return UTF_8; 
    }
    if let Ok(_) = std::str::from_utf8(buffer) {
        return UTF_8;
    }

    WINDOWS_1251 
}