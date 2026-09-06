//! Encrypted sync is refused until age/SOPS is pinned. Folder transport still
//! carries redacted desired-state bundles with secret *refs* only.

use ctxpect_schema::{array, canonical_json, object, parse, sha256_text, string, Value};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncError {
    pub code: &'static str,
    pub message: String,
}

impl SyncError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub fn bundle(
    bundle_id: &str,
    desired_state: Value,
    secret_refs: &[&str],
    receipts: Vec<Value>,
    vault_required: bool,
) -> Result<Value, SyncError> {
    if vault_required {
        return Err(SyncError::new(
            "sync.encryption_unavailable",
            "age/SOPS version_pin is evidence-backed-unavailable; refusing vault-required sync",
        ));
    }
    for item in receipts.iter() {
        if canonical_json(item).contains("\"secret_value\"") {
            return Err(SyncError::new(
                "sync.secret_in_bundle",
                "secret values must never enter a sync bundle",
            ));
        }
    }
    for refer in secret_refs {
        if refer.contains('=') || refer.contains("sk-") {
            return Err(SyncError::new(
                "sync.secret_in_bundle",
                "secret refs must be names, not values",
            ));
        }
    }
    Ok(object([
        ("schema", string("ctxpect-sync-bundle-v1")),
        ("bundle_id", string(bundle_id)),
        ("desired_state", desired_state),
        (
            "secret_refs",
            array(secret_refs.iter().map(|item| string(*item))),
        ),
        ("receipts", array(receipts)),
        ("encryption", string("unavailable")),
        ("last_write_wins", Value::Bool(false)),
    ]))
}

pub fn preview_apply(dest: &Path, incoming: &Value) -> Result<Value, SyncError> {
    let id = incoming
        .get("bundle_id")
        .and_then(Value::as_str)
        .ok_or_else(|| SyncError::new("sync.missing_id", "bundle_id required"))?;
    let applied = dest.join("applied.jsonl");
    if applied.exists() {
        let text = fs::read_to_string(&applied)
            .map_err(|err| SyncError::new("sync.io", err.to_string()))?;
        if text.lines().any(|line| line.contains(id)) {
            return Err(SyncError::new(
                "sync.replay",
                "bundle_id already applied; replay refused",
            ));
        }
    }
    let current = dest.join("current.json");
    if current.exists() {
        let text = fs::read_to_string(&current)
            .map_err(|err| SyncError::new("sync.io", err.to_string()))?;
        let existing = parse(&text).map_err(|err| SyncError::new("sync.parse", err.to_string()))?;
        let existing_id = existing.get("bundle_id").and_then(Value::as_str);
        if existing_id.is_some() && existing_id != Some(id) {
            return Ok(object([
                ("conflict", Value::Bool(true)),
                ("reason_code", string("sync.conflict")),
                ("last_write_wins", Value::Bool(false)),
                ("transport", string("not-started")),
                ("semantic", string("indeterminate")),
            ]));
        }
    }
    Ok(object([
        ("conflict", Value::Bool(false)),
        ("bundle_id", string(id)),
        ("transport", string("ready")),
        ("semantic", string("not-verified")),
        (
            "transport_success_is_verified",
            Value::Bool(false),
        ),
    ]))
}

pub fn apply_folder(dest: &Path, incoming: &Value) -> Result<Value, SyncError> {
    let preview = preview_apply(dest, incoming)?;
    if preview.get("conflict").and_then(Value::as_bool) == Some(true) {
        return Ok(preview);
    }
    fs::create_dir_all(dest).map_err(|err| SyncError::new("sync.io", err.to_string()))?;
    let id = incoming
        .get("bundle_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    fs::write(dest.join("current.json"), canonical_json(incoming))
        .map_err(|err| SyncError::new("sync.io", err.to_string()))?;
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest.join("applied.jsonl"))
        .map_err(|err| SyncError::new("sync.io", err.to_string()))?;
    use std::io::Write;
    writeln!(log, "{id}").map_err(|err| SyncError::new("sync.io", err.to_string()))?;
    Ok(object([
        ("transport", string("success")),
        ("semantic", string("structural-only")),
        (
            "transport_success_is_verified",
            Value::Bool(false),
        ),
        ("reconciliation", string("indeterminate")),
        ("bundle_id", string(id)),
        (
            "digest",
            string(sha256_text(&canonical_json(incoming))),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    #[test]
    fn secrets_replay_and_transport_are_honest() {
        let err = bundle("b1", object::<String>([]), &["TOKEN=sekrit"], vec![], false)
            .expect_err("secret");
        assert_eq!(err.code, "sync.secret_in_bundle");
        let err = bundle("b1", object::<String>([]), &["env:TOKEN"], vec![], true)
            .expect_err("vault");
        assert_eq!(err.code, "sync.encryption_unavailable");

        let dir = std::env::temp_dir().join(format!(
            "cx-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                % 1_000_000
        ));
        fs::create_dir_all(&dir).unwrap();
        let bundle_a = parse(r#"{"bundle_id":"b1","secret_refs":["env:TOKEN"]}"#).unwrap();
        let result = apply_folder(&dir, &bundle_a).unwrap();
        assert_eq!(
            result
                .get("transport_success_is_verified")
                .and_then(Value::as_bool),
            Some(false)
        );
        let err = apply_folder(&dir, &bundle_a).expect_err("replay");
        assert_eq!(err.code, "sync.replay");
        let bundle_b = parse(r#"{"bundle_id":"b2"}"#).unwrap();
        let conflict = apply_folder(&dir, &bundle_b).unwrap();
        assert_eq!(conflict.get("conflict").and_then(Value::as_bool), Some(true));
        let _ = fs::remove_dir_all(dir);
    }
}
