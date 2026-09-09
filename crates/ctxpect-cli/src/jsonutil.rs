//! Build `ctxpect-schema::Value` trees without a third-party JSON crate.

use ctxpect_schema::{Value, digest_value};

/// The one time-key stripper: `ctxpect_schema::strip_time_fields`. A second
/// list here once drifted from it (seven keys against ten), so the Receipt
/// fallback digest and the CLI's snapshot digest disagreed on documents
/// carrying `deleted_at` / `applied_at` / `imported_at`.
pub use ctxpect_schema::strip_time_fields;

#[must_use]
pub fn obj(pairs: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

#[must_use]
pub fn arr(items: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(items.into_iter().collect())
}

#[must_use]
pub fn s(text: impl Into<String>) -> Value {
    Value::Str(text.into())
}

#[must_use]
pub fn opt_s(text: Option<&str>) -> Value {
    match text {
        Some(text) => s(text),
        None => Value::Null,
    }
}

/// Insert `snapshot_digest` of the time-stripped body without that field.
#[must_use]
pub fn with_snapshot_digest(value: Value) -> Value {
    let stripped = strip_time_fields(&value);
    let body = match stripped {
        Value::Object(mut map) => {
            map.remove("snapshot_digest");
            Value::Object(map)
        }
        other => other,
    };
    let digest = digest_value(&body);
    match value {
        Value::Object(mut map) => {
            map.insert("snapshot_digest".to_string(), s(digest));
            Value::Object(map)
        }
        other => other,
    }
}
