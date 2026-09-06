//! HMAC-SHA256 for local-continuity Receipt signatures.
//!
//! This is not an organizational identity. It only proves that a store that
//! holds the continuity key produced the bytes. It must never be labeled as
//! a team/org signature.

use crate::sha256::Hasher;

const BLOCK: usize = 64;

/// HMAC-SHA256 (`key`, `message`) → 32-byte MAC.
#[must_use]
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let mut hasher = Hasher::new();
        hasher.update(key);
        key_block[..32].copy_from_slice(&hasher.finish_bytes());
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let inner = {
        let mut hasher = Hasher::new();
        hasher.update(&ipad);
        hasher.update(message);
        hasher.finish_bytes()
    };
    let mut hasher = Hasher::new();
    hasher.update(&opad);
    hasher.update(&inner);
    hasher.finish_bytes()
}

/// Hex-encode 32 bytes, lowercase.
#[must_use]
pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    let mac = hmac_sha256(key, message);
    let mut out = String::with_capacity(64);
    for byte in mac {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4231_case_1() {
        let mac = hmac_sha256_hex(&[0x0b; 20], b"Hi There");
        assert_eq!(
            mac,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }
}
