use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;

pub fn hash_file_start(file_path: &str, max_bytes: usize) -> std::io::Result<[u8; 32]> {

    let mut file = File::open(file_path)?;
    let mut buffer = vec![0u8; max_bytes];
    let n = file.read(&mut buffer)?;
    let mut hasher = Sha256::new();
    hasher.update(&buffer[..n]);
    Ok(hasher.finalize().into())
}