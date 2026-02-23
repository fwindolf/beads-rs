//! SHA256 + base36 ID generation.

use chrono::{DateTime, Utc};
use num_bigint::BigUint;
use num_traits::Zero;
use sha2::{Digest, Sha256};

/// Base36 alphabet (0-9, a-z).
const BASE36_ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Converts a byte slice to a base36 string of the specified length.
///
/// Matches the algorithm used for bd hash IDs.
pub fn encode_base36(data: &[u8], length: usize) -> String {
    let mut num = BigUint::from_bytes_be(data);
    let base = BigUint::from(36u32);
    let zero = BigUint::zero();

    // Build the string in reverse.
    let mut chars: Vec<u8> = Vec::with_capacity(length);
    while num > zero {
        let rem = &num % &base;
        num /= &base;
        // rem is guaranteed to be < 36, so fits in a u8 index.
        let idx = rem.to_u32_digits();
        let i = if idx.is_empty() { 0 } else { idx[0] as usize };
        chars.push(BASE36_ALPHABET[i]);
    }

    // Reverse to get most-significant digit first.
    chars.reverse();

    let mut s = String::from_utf8(chars).expect("base36 chars are valid UTF-8");

    // Pad with zeros if needed.
    if s.len() < length {
        let padding = "0".repeat(length - s.len());
        s = padding + &s;
    }

    // Truncate to exact length (keep least significant digits).
    if s.len() > length {
        s = s[s.len() - length..].to_owned();
    }

    s
}

/// Creates a hash-based ID for an issue.
///
/// Uses base36 encoding (0-9, a-z) for better information density than hex.
/// The `length` parameter is expected to be 3-8; other values fall back to
/// a 3-char byte width.
pub fn generate_hash_id(
    prefix: &str,
    title: &str,
    description: &str,
    creator: &str,
    timestamp: DateTime<Utc>,
    length: usize,
    nonce: i32,
) -> String {
    // Combine inputs into a stable content string.
    let content = format!(
        "{}|{}|{}|{}|{}",
        title,
        description,
        creator,
        timestamp.timestamp_nanos_opt().unwrap_or(0),
        nonce
    );

    let hash = Sha256::digest(content.as_bytes());

    // Determine how many bytes to use based on desired output length.
    let num_bytes = match length {
        3 => 2, // 2 bytes = 16 bits ~ 3.09 base36 chars
        4 => 3, // 3 bytes = 24 bits ~ 4.63 base36 chars
        5 => 4, // 4 bytes = 32 bits ~ 6.18 base36 chars
        6 => 4, // 4 bytes = 32 bits ~ 6.18 base36 chars
        7 => 5, // 5 bytes = 40 bits ~ 7.73 base36 chars
        8 => 5, // 5 bytes = 40 bits ~ 7.73 base36 chars
        _ => 3, // default to 3 chars
    };

    let short_hash = encode_base36(&hash[..num_bytes], length);
    format!("{}-{}", prefix, short_hash)
}

/// Computes the collision probability using the birthday paradox approximation.
///
/// P(collision) ~ 1 - e^(-n^2 / 2N)
/// where n = number of items, N = total possible values.
fn collision_probability(num_issues: usize, id_length: usize) -> f64 {
    let total: f64 = 36.0_f64.powi(id_length as i32);
    let exponent = -(num_issues as f64).powi(2) / (2.0 * total);
    1.0 - exponent.exp()
}

/// Determines the optimal ID length for the current database size.
///
/// Tries lengths from `min_length` to `max_length`, returning the first
/// that keeps the collision probability at or below `max_collision_prob`.
pub fn compute_adaptive_length(
    num_issues: usize,
    min_length: usize,
    max_length: usize,
    max_collision_prob: f64,
) -> usize {
    for length in min_length..=max_length {
        let prob = collision_probability(num_issues, length);
        if prob <= max_collision_prob {
            return length;
        }
    }
    max_length
}

/// Default adaptive ID configuration constants.
pub mod adaptive_defaults {
    /// Default collision probability threshold (25%).
    pub const MAX_COLLISION_PROB: f64 = 0.25;
    /// Default minimum hash length.
    pub const MIN_LENGTH: usize = 3;
    /// Default maximum hash length.
    pub const MAX_LENGTH: usize = 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_base36_basic() {
        // 0 bytes -> all zeros
        let result = encode_base36(&[], 4);
        assert_eq!(result, "0000");
    }

    #[test]
    fn encode_base36_length() {
        let data = [0xFF, 0xFF];
        let result = encode_base36(&data, 4);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn encode_base36_truncates() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let result = encode_base36(&data, 3);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn generate_hash_id_format() {
        let ts = chrono::Utc::now();
        let id = generate_hash_id("bd", "Test Title", "desc", "alice", ts, 6, 0);
        assert!(id.starts_with("bd-"));
        // prefix "bd-" + 6 chars = 9 total
        assert_eq!(id.len(), 9);
    }

    #[test]
    fn generate_hash_id_deterministic() {
        let ts = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let id1 = generate_hash_id("bd", "Title", "Desc", "alice", ts, 6, 0);
        let id2 = generate_hash_id("bd", "Title", "Desc", "alice", ts, 6, 0);
        assert_eq!(id1, id2);
    }

    #[test]
    fn generate_hash_id_nonce_changes_output() {
        let ts = chrono::Utc::now();
        let id1 = generate_hash_id("bd", "Title", "Desc", "alice", ts, 6, 0);
        let id2 = generate_hash_id("bd", "Title", "Desc", "alice", ts, 6, 1);
        assert_ne!(id1, id2);
    }

    #[test]
    fn adaptive_length_small_repo() {
        let len = compute_adaptive_length(10, 3, 8, 0.25);
        assert_eq!(len, 3); // 10 issues easily fits in 3 chars
    }

    #[test]
    fn adaptive_length_large_repo() {
        let len = compute_adaptive_length(100_000, 3, 8, 0.25);
        assert!(len >= 6); // 100K issues needs longer IDs
    }

    #[test]
    fn adaptive_length_capped_at_max() {
        let len = compute_adaptive_length(10_000_000, 3, 8, 0.01);
        assert_eq!(len, 8);
    }
}
