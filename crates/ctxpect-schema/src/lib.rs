//! Canonical JSON and content digests for Contexpect.
//!
//! Every digest that appears in a Receipt, a content manifest or an acceptance
//! fixture is derived through [`digest_value`]. Where this crate's canonicalisation
//! differs from the acceptance generator's is documented in the generated section
//! below, from the table in [`boundary`].
//!
//! Status: the product runtime is not implemented. This crate carries the
//! canonicalisation contract only.
#![doc = include_str!("../BOUNDARY.md")]

pub mod boundary;

pub mod hmac;
pub mod json;
pub mod sha256;

pub use hmac::{hmac_sha256, hmac_sha256_hex};
pub use json::{
    array, canonical_json, object, opt_string, parse, string, strip_time_fields, ParseError, Value,
};
pub use sha256::{sha256_hex, sha256_text, Hasher};

/// SHA-256 of a value's canonical JSON form.
///
/// Equivalent to the generator's `_digest_obj`.
#[must_use]
pub fn digest_value(value: &Value) -> String {
    sha256_text(&canonical_json(value))
}

/// Parse `text` and digest it, so a document on disk and a document in memory
/// produce the same content digest regardless of formatting.
pub fn digest_json_text(text: &str) -> Result<String, ParseError> {
    Ok(digest_value(&parse(text)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_ignores_formatting_and_key_order() {
        let a = digest_json_text(r#"{"b":1,"a":[1,2]}"#).expect("a");
        let b = digest_json_text("{\n  \"a\" : [1, 2],\n  \"b\": 1\n}").expect("b");
        assert_eq!(a, b);
    }

    #[test]
    fn digest_of_empty_object_is_stable() {
        // Python: sha256(json.dumps({}, separators=(",",":")).encode()).hexdigest()
        assert_eq!(
            digest_value(&parse("{}").expect("parse")),
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }
}
