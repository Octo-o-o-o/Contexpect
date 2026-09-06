//! File-backed native projection for project instruction files.
//!
//! Cursor user-instructions cells remain export-only. This crate is the
//! contexpect-native authority for in-repo AGENTS.md-style files.

use ctxpect_fs::Root;
use ctxpect_schema::{canonical_json, object, sha256_hex, string, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionError {
    pub code: &'static str,
    pub message: String,
}

impl ProjectionError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Intent {
    pub intent_id: String,
    pub authority: String,
    pub target_rel: String,
    pub desired: String,
}

impl Intent {
    pub fn to_value(&self) -> Value {
        object([
            ("schema", string("canonical-intent-v1")),
            ("intent_id", string(&self.intent_id)),
            ("authority", string(&self.authority)),
            ("target_rel", string(&self.target_rel)),
            (
                "desired_digest",
                string(sha256_hex(self.desired.as_bytes())),
            ),
            ("byte_equality_is_semantic", Value::Bool(false)),
        ])
    }
}

#[derive(Debug, Clone)]
pub struct Preview {
    pub tx_id: String,
    pub authority: String,
    pub target_rel: String,
    pub current_digest: String,
    pub desired_digest: String,
    pub loss: String,
    pub native_diff: String,
}

impl Preview {
    pub fn to_value(&self) -> Value {
        object([
            ("tx_id", string(&self.tx_id)),
            ("authority", string(&self.authority)),
            ("target_rel", string(&self.target_rel)),
            ("current_digest", string(&self.current_digest)),
            ("desired_digest", string(&self.desired_digest)),
            ("loss", string(&self.loss)),
            ("native_diff", string(&self.native_diff)),
            ("is_apply", Value::Bool(false)),
        ])
    }
}

pub fn preview(root: &Root, intent: &Intent) -> Result<Preview, ProjectionError> {
    if intent.authority != "contexpect-native" && intent.authority != "project-file" {
        return Err(ProjectionError::new(
            "projection.authority",
            format!(
                "authority `{}` is not the unique writer for this cell",
                intent.authority
            ),
        ));
    }
    if intent.target_rel.contains("..") || intent.target_rel.starts_with('/') {
        return Err(ProjectionError::new(
            "projection.path",
            "target must be a relative path inside the project",
        ));
    }
    let target = contained_target(root, &intent.target_rel)?;
    let current = if target.exists() {
        fs::read(&target).map_err(|err| ProjectionError::new("projection.io", err.to_string()))?
    } else {
        Vec::new()
    };
    let current_text = String::from_utf8_lossy(&current);
    let diff = native_diff(&current_text, &intent.desired);
    let tx_id = format!(
        "tx_{}",
        &sha256_hex(format!("{}|{}", intent.intent_id, intent.target_rel).as_bytes())[..16]
    );
    Ok(Preview {
        tx_id,
        authority: intent.authority.clone(),
        target_rel: intent.target_rel.clone(),
        current_digest: sha256_hex(&current),
        desired_digest: sha256_hex(intent.desired.as_bytes()),
        loss: if intent.target_rel.contains("cursor") && intent.target_rel.contains("user") {
            "export-only: Cursor user instructions have no required-write executor in this cell"
                .to_string()
        } else {
            "none".to_string()
        },
        native_diff: diff,
    })
}

pub fn apply(
    root: &Root,
    intent: &Intent,
    preview: &Preview,
    backup_dir: &Path,
    policy_allows: bool,
) -> Result<Value, ProjectionError> {
    if !policy_allows {
        return Err(ProjectionError::new(
            "projection.policy_denied",
            "apply requires a prior policy/Approval pass",
        ));
    }
    if preview.loss.starts_with("export-only") {
        return Err(ProjectionError::new(
            "projection.export_only",
            "required-write cells cannot degrade to export-only; this cell is export-only and apply is refused",
        ));
    }
    let target = contained_target(root, &intent.target_rel)?;
    let current = if target.exists() {
        fs::read(&target).map_err(|err| ProjectionError::new("projection.io", err.to_string()))?
    } else {
        Vec::new()
    };
    let now_digest = sha256_hex(&current);
    if now_digest != preview.current_digest {
        return Err(ProjectionError::new(
            "projection.concurrent_hash",
            "target hash changed since preview; refuse to overwrite concurrent edits",
        ));
    }
    fs::create_dir_all(backup_dir)
        .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    let backup = backup_dir.join("before");
    fs::write(&backup, &current)
        .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    }
    fs::write(&target, intent.desired.as_bytes())
        .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    let meta = object([
        ("tx_id", string(&preview.tx_id)),
        ("target_rel", string(&intent.target_rel)),
        ("before_digest", string(&preview.current_digest)),
        ("after_digest", string(&preview.desired_digest)),
        ("backup", string(backup.to_string_lossy())),
        ("is_apply", Value::Bool(true)),
    ]);
    fs::write(backup_dir.join("tx.json"), canonical_json(&meta))
        .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    Ok(meta)
}

pub fn rollback(root: &Root, backup_dir: &Path, managed_rel: &str) -> Result<Value, ProjectionError> {
    let meta_text = fs::read_to_string(backup_dir.join("tx.json"))
        .map_err(|_| ProjectionError::new("projection.no_tx", "no apply transaction to roll back"))?;
    let meta = ctxpect_schema::parse(&meta_text)
        .map_err(|err| ProjectionError::new("projection.parse", err.to_string()))?;
    let recorded = meta
        .get("target_rel")
        .and_then(Value::as_str)
        .unwrap_or("");
    if recorded != managed_rel {
        return Err(ProjectionError::new(
            "projection.rollback_scope",
            "rollback refuses to touch a path that was not in the transaction",
        ));
    }
    let target = contained_target(root, managed_rel)?;
    let before = fs::read(backup_dir.join("before"))
        .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    fs::write(&target, before)
        .map_err(|err| ProjectionError::new("projection.io", err.to_string()))?;
    Ok(object([
        ("rolled_back", Value::Bool(true)),
        ("target_rel", string(managed_rel)),
        ("unmanaged_untouched", Value::Bool(true)),
    ]))
}

fn contained_target(root: &Root, rel: &str) -> Result<PathBuf, ProjectionError> {
    let joined = root.path().join(rel);
    match root.contain(&joined) {
        Ok(path) => Ok(path),
        Err(_) => {
            let parent = joined.parent().unwrap_or(root.path());
            let parent = root.contain(parent).map_err(|_| {
                ProjectionError::new("projection.escapes", "target escapes project")
            })?;
            Ok(parent.join(joined.file_name().unwrap_or_default()))
        }
    }
}

fn native_diff(before: &str, after: &str) -> String {
    if before == after {
        return "(no change)".to_string();
    }
    format!("- {} bytes\n+ {} bytes", before.len(), after.len())
}

/// Launch argv must be a structured array. Shell concatenation is refused.
pub fn launch_argv(program: &str, args: &[&str]) -> Result<Vec<String>, ProjectionError> {
    let mut out = vec![program.to_string()];
    for arg in args {
        if arg.as_bytes().iter().any(|b| matches!(*b, b';' | b'|' | b'`' | b'\n' | 0))
            || arg.contains("$(")
            || arg.contains("${")
        {
            return Err(ProjectionError::new(
                "launch.injection",
                "refusing shell metacharacters in structured argv",
            ));
        }
        out.push((*arg).to_string());
    }
    Ok(out)
}

pub fn backup_dir(store_root: &Path, tx_id: &str) -> PathBuf {
    store_root.join("apply").join(tx_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_fs::Root;

    fn scratch(label: &str) -> (PathBuf, Root) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cx-proj-{label}-{}-{}",
            std::process::id(),
            nanos % 1_000_000
        ));
        fs::create_dir_all(&path).unwrap();
        let root = Root::new(&path).unwrap();
        (path, root)
    }

    #[test]
    fn concurrent_hash_and_unrelated_rollback_are_refused() {
        let (path, root) = scratch("apply");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let intent = Intent {
            intent_id: "i1".into(),
            authority: "contexpect-native".into(),
            target_rel: "AGENTS.md".into(),
            desired: "two\n".into(),
        };
        let previewed = preview(&root, &intent).unwrap();
        fs::write(path.join("AGENTS.md"), "changed\n").unwrap();
        let backup = path.join("backup");
        let err = apply(&root, &intent, &previewed, &backup, true).expect_err("hash");
        assert_eq!(err.code, "projection.concurrent_hash");

        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent).unwrap();
        apply(&root, &intent, &previewed, &backup, true).unwrap();
        let err = rollback(&root, &backup, "OTHER.md").expect_err("scope");
        assert_eq!(err.code, "projection.rollback_scope");
        rollback(&root, &backup, "AGENTS.md").unwrap();
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "one\n");
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn apply_without_policy_is_denied_and_shell_is_refused() {
        let (path, root) = scratch("policy");
        let intent = Intent {
            intent_id: "i2".into(),
            authority: "contexpect-native".into(),
            target_rel: "AGENTS.md".into(),
            desired: "x\n".into(),
        };
        let previewed = preview(&root, &intent).unwrap();
        let err = apply(&root, &intent, &previewed, &path.join("b"), false).expect_err("policy");
        assert_eq!(err.code, "projection.policy_denied");
        let err = launch_argv("codex", &["exec", "$(rm -rf /)"]).expect_err("inj");
        assert_eq!(err.code, "launch.injection");
        let _ = fs::remove_dir_all(path);
    }
}
