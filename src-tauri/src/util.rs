//! Small shared helpers: monotonic-ish wall-clock and content hashing.

use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Current wall-clock time in unix milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Hex-encoded SHA-256 of arbitrary bytes. Used as the dedup / suppress key.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Hex-encoded SHA-256 of a string.
pub fn hash_text(text: &str) -> String {
    hash_bytes(text.as_bytes())
}
