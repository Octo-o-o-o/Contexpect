//! File-backed native projection for project instruction files.
//!
//! Cursor user-instructions cells remain export-only. This crate is the
//! contexpect-native authority for in-repo AGENTS.md-style files.
//!
//! # Transaction shape
//!
//! `preview` computes what an apply would do and freezes the target's
//! current digest. The preview is bound to the **project** it was computed
//! in (`project_digest`) and carries a consumption `state`: `previewed`
//! until an apply lands, then `applied`, then `rolled-back`. A transaction id
//! is unique per preview (project, intent, target, both digests and a
//! per-process nonce), so a later preview never overwrites an earlier
//! transaction's record or backup, and a preview from one project cannot
//! name another project's transaction directory. `apply` accepts only that
//! frozen preview: it re-hashes the target and refuses when the bytes moved
//! (`projection.concurrent_hash`), refuses a preview from another project
//! (`projection.preview_scope`) and refuses a consumed one
//! (`projection.tx_consumed`).
//! The transaction record `tx.json` carries the project digest and is
//! written **before** the target, in state `pending`, then moved to
//! `committed` after the target landed through an exclusively created
//! temp-file rename. `rollback` checks the record's project, re-hashes the
//! target against the recorded `after_digest`, refuses a concurrent edit
//! (`projection.rollback_conflict`), and removes a file the apply created
//! instead of leaving an empty one.
//!
//! # What is never a target
//!
//! A target that exists but is not a regular file — a symlink (dangling or
//! not), FIFO, socket or directory — is refused (`projection.not_a_file`)
//! rather than read (a FIFO would block forever) or written through. A
//! symlink to a regular file inside the project is resolved by containment
//! and the *real* file is the target. Paths under `.git/` or `.ctxpect/`,
//! and any path inside the store, are control paths (`projection.control_path`):
//! an `apply` authorization must not be able to mint further authorizations
//! or edit git hooks.
//!
//! A secret gate runs on both the desired bytes and the bytes that would
//! land in the backup: a secret-bearing executor path is refused, not
//! archived.

use ctxpect_doctor::secret_literal;
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

fn io_err(err: std::io::Error) -> ProjectionError {
    ProjectionError::new("projection.io", err.to_string())
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

    /// Structural validation without touching the filesystem.
    pub fn validate(&self) -> Result<(), ProjectionError> {
        if self.intent_id.is_empty() {
            return Err(ProjectionError::new("projection.intent_id", "intent_id is required"));
        }
        if self.authority != "contexpect-native" && self.authority != "project-file" {
            return Err(ProjectionError::new(
                "projection.authority",
                format!(
                    "authority `{}` is not the unique writer for this cell",
                    self.authority
                ),
            ));
        }
        check_rel_path(&self.target_rel)
    }
}

/// Lexical checks a target path must pass before any filesystem access:
/// relative, no `..` or empty segment, and not a control path.
fn check_rel_path(rel: &str) -> Result<(), ProjectionError> {
    if rel.is_empty()
        || rel.starts_with('/')
        || rel.contains('\\')
        || rel.split('/').any(|part| part.is_empty() || part == "..")
    {
        return Err(ProjectionError::new(
            "projection.path",
            "target must be a relative path inside the project",
        ));
    }
    if rel.split('/').any(|part| part == ".git" || part == ".ctxpect") {
        return Err(ProjectionError::new(
            "projection.control_path",
            "targets under `.git/` or `.ctxpect/` are control paths; an apply authorization does not extend to them",
        ));
    }
    Ok(())
}

/// Whether `tx` has the shape every transaction id this crate mints has.
/// Callers must check this before joining a caller-supplied id onto a path.
#[must_use]
pub fn valid_tx_id(tx: &str) -> bool {
    tx.len() == 19
        && tx.starts_with("tx_")
        && tx[3..].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Refuse a target that lies inside the store, wherever the store was
/// placed. The store holds policy, exceptions and transaction records; a
/// target there would let an `apply` authorization rewrite its own gate.
pub fn target_outside_store(root: &Root, rel: &str, store_root: &Path) -> Result<(), ProjectionError> {
    check_rel_path(rel)?;
    let store = fs::canonicalize(store_root).unwrap_or_else(|_| store_root.to_path_buf());
    let target = ctxpect_fs::lexical_normalize(&root.path().join(rel));
    if target == store || target.starts_with(&store) {
        return Err(ProjectionError::new(
            "projection.control_path",
            "target lies inside the store; the store is a control path",
        ));
    }
    Ok(())
}

/// Read the target if it is a regular file; `None` if nothing is there.
/// Anything else (symlink, FIFO, socket, directory) is refused before it
/// is read, so a FIFO cannot block the process and a link is never
/// followed here.
fn probe_target(target: &Path) -> Result<Option<Vec<u8>>, ProjectionError> {
    match fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_file() => fs::read(target).map(Some).map_err(io_err),
        Ok(meta) => Err(ProjectionError::new(
            "projection.not_a_file",
            format!(
                "target exists but is {}; only a regular file can be projected",
                if meta.file_type().is_symlink() {
                    "a symlink"
                } else if meta.is_dir() {
                    "a directory"
                } else {
                    "not a regular file"
                }
            ),
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io_err(err)),
    }
}

fn nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}:{}:{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Preview consumption states.
pub const PREVIEW_PREVIEWED: &str = "previewed";
pub const PREVIEW_APPLIED: &str = "applied";
pub const PREVIEW_ROLLED_BACK: &str = "rolled-back";

/// The frozen plan an apply is bound to.
#[derive(Debug, Clone)]
pub struct Preview {
    pub tx_id: String,
    /// The project scope digest this preview was computed in.
    pub project_digest: String,
    /// One of [`PREVIEW_PREVIEWED`], [`PREVIEW_APPLIED`], [`PREVIEW_ROLLED_BACK`].
    pub state: String,
    pub intent_id: String,
    pub authority: String,
    pub target_rel: String,
    /// Digest of the target at preview time. Empty-string digest for an
    /// absent target is distinguished by `existed_before`.
    pub current_digest: String,
    pub existed_before: bool,
    pub desired_digest: String,
    pub desired: String,
    pub loss: String,
    pub native_diff: String,
}

impl Preview {
    /// The user-facing preview. The desired bytes are represented by their
    /// digest; the record form ([`Preview::to_record`]) carries them.
    pub fn to_value(&self) -> Value {
        object([
            ("tx_id", string(&self.tx_id)),
            ("project_digest", string(&self.project_digest)),
            ("state", string(&self.state)),
            ("intent_id", string(&self.intent_id)),
            ("authority", string(&self.authority)),
            ("target_rel", string(&self.target_rel)),
            ("current_digest", string(&self.current_digest)),
            ("existed_before", Value::Bool(self.existed_before)),
            ("desired_digest", string(&self.desired_digest)),
            ("loss", string(&self.loss)),
            ("native_diff", string(&self.native_diff)),
            ("is_apply", Value::Bool(false)),
        ])
    }

    /// The persisted form `apply --tx` reads back. It is the only input an
    /// apply accepts, so it has to carry the desired bytes.
    pub fn to_record(&self) -> Value {
        match self.to_value() {
            Value::Object(mut map) => {
                map.insert("schema".into(), string("ctxpect-preview-v1"));
                map.insert("desired".into(), string(&self.desired));
                Value::Object(map)
            }
            other => other,
        }
    }

    pub fn from_record(record: &Value) -> Result<Preview, ProjectionError> {
        let field = |name: &str| -> Result<String, ProjectionError> {
            record
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| {
                    ProjectionError::new(
                        "projection.preview_malformed",
                        format!("persisted preview lacks `{name}`"),
                    )
                })
        };
        if record.get("schema").and_then(Value::as_str) != Some("ctxpect-preview-v1") {
            return Err(ProjectionError::new(
                "projection.preview_malformed",
                "persisted preview is not ctxpect-preview-v1",
            ));
        }
        let desired = field("desired")?;
        let desired_digest = field("desired_digest")?;
        if sha256_hex(desired.as_bytes()) != desired_digest {
            return Err(ProjectionError::new(
                "projection.preview_malformed",
                "persisted preview desired bytes do not match desired_digest",
            ));
        }
        Ok(Preview {
            tx_id: field("tx_id")?,
            project_digest: field("project_digest")?,
            state: field("state")?,
            intent_id: field("intent_id")?,
            authority: field("authority")?,
            target_rel: field("target_rel")?,
            current_digest: field("current_digest")?,
            existed_before: record
                .get("existed_before")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    ProjectionError::new(
                        "projection.preview_malformed",
                        "persisted preview lacks `existed_before`",
                    )
                })?,
            desired_digest,
            desired,
            loss: field("loss")?,
            native_diff: field("native_diff")?,
        })
    }

    /// The same preview in another consumption state.
    #[must_use]
    pub fn with_state(&self, state: &str) -> Preview {
        Preview {
            state: state.to_string(),
            ..self.clone()
        }
    }

    /// Refuse a preview that was computed for another project, or that an
    /// apply already consumed. Called by every apply entry before policy.
    pub fn check_applicable(&self, project_digest: &str) -> Result<(), ProjectionError> {
        if self.project_digest != project_digest {
            return Err(ProjectionError::new(
                "projection.preview_scope",
                "this preview was computed for another project; run `intent preview` in this project",
            ));
        }
        if self.state != PREVIEW_PREVIEWED {
            return Err(ProjectionError::new(
                "projection.tx_consumed",
                format!(
                    "this transaction is `{}`; a preview is consumed by its apply and must be recomputed after a rollback",
                    self.state
                ),
            ));
        }
        Ok(())
    }
}

/// Refuse a payload that carries a credential shape.
fn secret_gate(label: &str, bytes: &[u8]) -> Result<(), ProjectionError> {
    let text = String::from_utf8_lossy(bytes);
    if let Some(class) = secret_literal(&text) {
        return Err(ProjectionError::new(
            "projection.contains_secrets",
            format!(
                "{label} carries a `{}` secret shape; a secret-bearing executor path is refused",
                class.as_str()
            ),
        ));
    }
    Ok(())
}

/// Compute the frozen plan for `intent` inside `root`. `project_digest`
/// identifies the project scope (the same digest authorization is bound to);
/// it becomes part of the transaction id and of the persisted record.
pub fn preview(root: &Root, intent: &Intent, project_digest: &str) -> Result<Preview, ProjectionError> {
    intent.validate()?;
    secret_gate("desired content", intent.desired.as_bytes())?;
    let target = contained_target(root, &intent.target_rel)?;
    let probed = probe_target(&target)?;
    let existed = probed.is_some();
    let current = probed.unwrap_or_default();
    // The bytes that would be archived in the backup are gated too.
    secret_gate("current target content (would enter the backup)", &current)?;
    let current_text = String::from_utf8_lossy(&current);
    let diff = native_diff(&current_text, &intent.desired);
    let desired_digest = sha256_hex(intent.desired.as_bytes());
    let current_digest = sha256_hex(&current);
    // Unique per preview: two previews of the same intent never share a
    // transaction directory, so neither can overwrite the other's backup.
    let tx_id = format!(
        "tx_{}",
        &sha256_hex(
            format!(
                "{project_digest}|{}|{}|{current_digest}|{desired_digest}|{}",
                intent.intent_id,
                intent.target_rel,
                nonce()
            )
            .as_bytes()
        )[..16]
    );
    Ok(Preview {
        tx_id,
        project_digest: project_digest.to_string(),
        state: PREVIEW_PREVIEWED.to_string(),
        intent_id: intent.intent_id.clone(),
        authority: intent.authority.clone(),
        target_rel: intent.target_rel.clone(),
        current_digest,
        existed_before: existed,
        desired_digest,
        desired: intent.desired.clone(),
        loss: if intent.target_rel.contains("cursor") && intent.target_rel.contains("user") {
            "export-only: Cursor user instructions have no required-write executor in this cell"
                .to_string()
        } else {
            "none".to_string()
        },
        native_diff: diff,
    })
}

/// Transaction record states. `pending` means the record exists and the
/// target may or may not have been written; `committed` means it has.
pub const TX_PENDING: &str = "pending";
pub const TX_COMMITTED: &str = "committed";
pub const TX_ROLLED_BACK: &str = "rolled-back";

pub const TX_SCHEMA: &str = "ctxpect-projection-tx-v1";

/// The backup file's name inside the transaction directory. Recorded as a
/// relative name: a transaction record is publishable and carries no
/// absolute path.
const BACKUP_NAME: &str = "before";

fn tx_meta(preview: &Preview, backup: bool, state: &str) -> Value {
    object([
        ("schema", string(TX_SCHEMA)),
        ("tx_id", string(&preview.tx_id)),
        ("project_digest", string(&preview.project_digest)),
        ("intent_id", string(&preview.intent_id)),
        ("target_rel", string(&preview.target_rel)),
        ("before_digest", string(&preview.current_digest)),
        ("after_digest", string(&preview.desired_digest)),
        ("existed_before", Value::Bool(preview.existed_before)),
        (
            "backup",
            if backup { string(BACKUP_NAME) } else { Value::Null },
        ),
        ("state", string(state)),
        ("is_apply", Value::Bool(true)),
    ])
}

/// Write bytes through an exclusively created sibling temp file and rename
/// it into place (see [`ctxpect_fs::write_atomic`]).
pub fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), ProjectionError> {
    ctxpect_fs::write_atomic(target, bytes).map_err(io_err)
}

/// A transaction directory must be a real directory whose parent (the
/// store's `apply/`) is one too: a pre-placed symlink at either name would
/// redirect the backup and record out of the store.
fn ensure_tx_dir(backup_dir: &Path) -> Result<(), ProjectionError> {
    fs::create_dir_all(backup_dir).map_err(io_err)?;
    if let Some(parent) = backup_dir.parent() {
        ctxpect_fs::real_dir(parent).map_err(io_err)?;
    }
    ctxpect_fs::real_dir(backup_dir).map_err(io_err)
}

/// Apply a frozen preview. See the crate docs for the ordering guarantees.
pub fn apply(
    root: &Root,
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
    if sha256_hex(preview.desired.as_bytes()) != preview.desired_digest {
        return Err(ProjectionError::new(
            "projection.preview_malformed",
            "preview desired bytes do not match desired_digest",
        ));
    }
    secret_gate("desired content", preview.desired.as_bytes())?;
    let target = contained_target(root, &preview.target_rel)?;
    let probed = probe_target(&target)?;
    let exists_now = probed.is_some();
    let current = probed.unwrap_or_default();
    if exists_now != preview.existed_before || sha256_hex(&current) != preview.current_digest {
        return Err(ProjectionError::new(
            "projection.concurrent_hash",
            "target hash changed since preview; refuse to overwrite concurrent edits",
        ));
    }
    secret_gate("current target content (would enter the backup)", &current)?;

    // 1. Backup and the pending transaction record land before the target
    //    is touched, so an interruption leaves a record, never a half state.
    ensure_tx_dir(backup_dir)?;
    let backup_path = backup_dir.join(BACKUP_NAME);
    let backup = preview.existed_before;
    if backup {
        write_atomic(&backup_path, &current)?;
    }
    let tx_path = backup_dir.join("tx.json");
    write_atomic(
        &tx_path,
        canonical_json(&tx_meta(preview, backup, TX_PENDING)).as_bytes(),
    )?;

    // 2. The target is written through a temp file and renamed into place.
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    write_atomic(&target, preview.desired.as_bytes())?;

    // 3. Only now is the transaction committed.
    let meta = tx_meta(preview, backup, TX_COMMITTED);
    write_atomic(&tx_path, canonical_json(&meta).as_bytes())?;
    Ok(meta)
}

/// Judge every transaction record still in state `pending` under
/// `apply_root` (the store's `apply/` directory) against the target's current
/// bytes. `committed-recovered`: the target holds the applied bytes;
/// `aborted`: it still holds the pre-apply bytes (or is absent for a file the
/// apply would have created); `in-doubt`: neither, or the target cannot be
/// examined; `unreadable`: the record itself cannot be parsed (a torn write)
/// — it is listed, not skipped, and counts as in doubt. Only this crate's
/// records for `project_digest` are judged: an assets transaction has its
/// own executor, and another project's record says nothing about this
/// project's files. Read-only — no record and no target is rewritten here;
/// `rollback --id` closes a record.
pub fn pending_transactions(
    root: &Root,
    apply_root: &Path,
    project_digest: &str,
) -> Result<Vec<Value>, ProjectionError> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(apply_root) else {
        return Ok(out);
    };
    let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    for dir in dirs {
        if !dir.join("tx.json").exists() {
            continue;
        }
        let meta = match read_tx(&dir) {
            Ok(meta) => meta,
            Err(err) => {
                out.push(object([
                    ("tx_id", string(dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())),
                    ("target_rel", Value::Null),
                    ("recorded_state", Value::Null),
                    ("judgement", string("unreadable")),
                    ("reason_code", string(err.code)),
                    ("target_digest", Value::Null),
                    ("before_digest", Value::Null),
                    ("after_digest", Value::Null),
                    ("target_rewritten", Value::Bool(false)),
                ]));
                continue;
            }
        };
        if meta.get("schema").and_then(Value::as_str) != Some(TX_SCHEMA)
            || meta.get("project_digest").and_then(Value::as_str) != Some(project_digest)
        {
            continue;
        }
        if meta.get("state").and_then(Value::as_str) != Some(TX_PENDING) {
            continue;
        }
        let tx_id = meta.get("tx_id").and_then(Value::as_str).unwrap_or("").to_string();
        let target_rel = meta.get("target_rel").and_then(Value::as_str).unwrap_or("").to_string();
        let before = meta.get("before_digest").and_then(Value::as_str).unwrap_or("");
        let after = meta.get("after_digest").and_then(Value::as_str).unwrap_or("");
        let existed_before = meta.get("existed_before").and_then(Value::as_bool).unwrap_or(true);
        let judgement = match contained_target(root, &target_rel).and_then(|target| probe_target(&target)) {
            Err(err) => ("in-doubt", string(err.code)),
            Ok(probed) => {
                let exists = probed.is_some();
                let digest = probed.map(|b| sha256_hex(&b)).unwrap_or_default();
                let state = if exists && digest == after {
                    "committed-recovered"
                } else if (existed_before && exists && digest == before) || (!existed_before && !exists) {
                    "aborted"
                } else {
                    "in-doubt"
                };
                (state, string(&digest))
            }
        };
        out.push(object([
            ("tx_id", string(&tx_id)),
            ("target_rel", string(&target_rel)),
            ("recorded_state", string(TX_PENDING)),
            ("judgement", string(judgement.0)),
            ("target_digest", judgement.1),
            ("before_digest", string(before)),
            ("after_digest", string(after)),
            ("target_rewritten", Value::Bool(false)),
        ]));
    }
    Ok(out)
}

/// Read a transaction record, if the backup directory holds one.
pub fn read_tx(backup_dir: &Path) -> Result<Value, ProjectionError> {
    let meta_text = fs::read_to_string(backup_dir.join("tx.json"))
        .map_err(|_| ProjectionError::new("projection.no_tx", "no apply transaction to roll back"))?;
    ctxpect_schema::parse(&meta_text)
        .map_err(|err| ProjectionError::new("projection.parse", err.to_string()))
}

/// Undo a transaction. `project_digest` must be the project the record was
/// written for: a record from another project that happens to name a file
/// with the same bytes is refused (`projection.rollback_scope`).
pub fn rollback(
    root: &Root,
    backup_dir: &Path,
    managed_rel: &str,
    project_digest: &str,
) -> Result<Value, ProjectionError> {
    let meta = read_tx(backup_dir)?;
    if meta.get("schema").and_then(Value::as_str) != Some(TX_SCHEMA) {
        return Err(ProjectionError::new(
            "projection.rollback_scope",
            "this record is not a projection transaction",
        ));
    }
    if meta.get("project_digest").and_then(Value::as_str) != Some(project_digest) {
        return Err(ProjectionError::new(
            "projection.rollback_scope",
            "this transaction was applied in another project; rollback refuses to write here",
        ));
    }
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
    let state = meta.get("state").and_then(Value::as_str).unwrap_or(TX_COMMITTED);
    if state == TX_ROLLED_BACK {
        return Err(ProjectionError::new(
            "projection.already_rolled_back",
            "this transaction was already rolled back",
        ));
    }
    let before_digest = meta.get("before_digest").and_then(Value::as_str).unwrap_or("");
    let after_digest = meta.get("after_digest").and_then(Value::as_str).unwrap_or("");
    // Records written before `existed_before` existed are read as "existed":
    // restoring bytes is the conservative reading of an old record.
    let existed_before = meta
        .get("existed_before")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let target = contained_target(root, managed_rel)?;
    let probed = probe_target(&target)?;
    let exists_now = probed.is_some();
    let now_digest = probed.map(|b| sha256_hex(&b)).unwrap_or_default();
    let at_after = exists_now && now_digest == after_digest;
    let at_before = if existed_before {
        exists_now && now_digest == before_digest
    } else {
        !exists_now
    };

    let action = if at_after {
        // The apply's bytes are still what is on disk: undo them.
        if existed_before {
            let before = fs::read(backup_dir.join(BACKUP_NAME)).map_err(io_err)?;
            if sha256_hex(&before) != before_digest {
                return Err(ProjectionError::new(
                    "projection.backup_corrupt",
                    "backup bytes do not match the recorded before_digest",
                ));
            }
            write_atomic(&target, &before)?;
            "restored-previous-bytes"
        } else {
            fs::remove_file(&target).map_err(io_err)?;
            "removed-created-file"
        }
    } else if at_before && state == TX_PENDING {
        // The apply never reached the target; the record is closed.
        "target-untouched"
    } else {
        // Neither the applied bytes nor the pre-apply bytes: someone edited
        // the target after the apply. Their edit is left alone.
        return Err(ProjectionError::new(
            "projection.rollback_conflict",
            "target was modified after apply; rollback refuses to discard the concurrent edit",
        ));
    };

    let mut closed = match meta {
        Value::Object(map) => map,
        _ => Default::default(),
    };
    closed.insert("state".into(), string(TX_ROLLED_BACK));
    write_atomic(
        &backup_dir.join("tx.json"),
        canonical_json(&Value::Object(closed)).as_bytes(),
    )?;
    Ok(object([
        ("rolled_back", Value::Bool(true)),
        ("target_rel", string(managed_rel)),
        ("action", string(action)),
        ("unmanaged_untouched", Value::Bool(true)),
    ]))
}

/// Resolve a project-relative path that may not exist yet.
///
/// `Root::contain` canonicalises, so it refuses a target whose file or parent
/// directories have not been created. Containment is still enforced: the
/// deepest existing ancestor is canonicalised and checked, and the remaining
/// segments are plain names because `Intent::validate` already refused `..`
/// and absolute paths.
fn contained_target(root: &Root, rel: &str) -> Result<PathBuf, ProjectionError> {
    check_rel_path(rel)?;
    let joined = root.path().join(rel);
    if let Ok(path) = root.contain(&joined) {
        return Ok(path);
    }
    let mut tail = Vec::new();
    let mut cursor = joined.as_path();
    while !cursor.exists() {
        let name = cursor.file_name().ok_or_else(|| {
            ProjectionError::new("projection.escapes", "target has no resolvable base")
        })?;
        tail.push(name.to_owned());
        cursor = cursor.parent().ok_or_else(|| {
            ProjectionError::new("projection.escapes", "target has no resolvable base")
        })?;
    }
    let mut out = root
        .contain(cursor)
        .map_err(|_| ProjectionError::new("projection.escapes", "target escapes project"))?;
    for part in tail.iter().rev() {
        out.push(part);
    }
    Ok(out)
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
        (fs::canonicalize(path).unwrap(), root)
    }

    fn intent(target: &str, desired: &str) -> Intent {
        Intent {
            intent_id: "i1".into(),
            authority: "contexpect-native".into(),
            target_rel: target.into(),
            desired: desired.into(),
        }
    }

    #[test]
    fn concurrent_hash_and_unrelated_rollback_are_refused() {
        let (path, root) = scratch("apply");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        fs::write(path.join("AGENTS.md"), "changed\n").unwrap();
        let backup = path.join("backup");
        let err = apply(&root, &previewed, &backup, true).expect_err("hash");
        assert_eq!(err.code, "projection.concurrent_hash");

        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        apply(&root, &previewed, &backup, true).unwrap();
        let err = rollback(&root, &backup, "OTHER.md", "p").expect_err("scope");
        assert_eq!(err.code, "projection.rollback_scope");
        rollback(&root, &backup, "AGENTS.md", "p").unwrap();
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "one\n");
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn a_persisted_preview_round_trips_and_binds_the_apply() {
        let (path, root) = scratch("record");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        let record = previewed.to_record();
        let restored = Preview::from_record(&record).unwrap();
        assert_eq!(restored.desired, "two\n");
        assert_eq!(restored.current_digest, previewed.current_digest);

        // Tampering with the desired bytes in the record is caught.
        let mut tampered = match record {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        tampered.insert("desired".into(), string("PWNED\n"));
        let err = Preview::from_record(&Value::Object(tampered)).expect_err("tamper");
        assert_eq!(err.code, "projection.preview_malformed");
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn rollback_refuses_a_concurrent_edit_and_removes_a_created_file() {
        let (path, root) = scratch("rollback");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        let backup = path.join("b1");
        apply(&root, &previewed, &backup, true).unwrap();
        // The user edits after the apply; rollback must not discard it.
        fs::write(path.join("AGENTS.md"), "user edit\n").unwrap();
        let err = rollback(&root, &backup, "AGENTS.md", "p").expect_err("conflict");
        assert_eq!(err.code, "projection.rollback_conflict");
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "user edit\n");

        // A file the apply created is removed, not emptied.
        let previewed = preview(&root, &intent("NEW.md", "fresh\n"), "p").unwrap();
        assert!(!previewed.existed_before);
        let backup = path.join("b2");
        let meta = apply(&root, &previewed, &backup, true).unwrap();
        assert_eq!(meta.get("existed_before").and_then(Value::as_bool), Some(false));
        assert_eq!(meta.get("state").and_then(Value::as_str), Some(TX_COMMITTED));
        assert!(!backup.join(BACKUP_NAME).exists(), "nothing to back up for a new file");
        let undone = rollback(&root, &backup, "NEW.md", "p").unwrap();
        assert_eq!(undone.get("action").and_then(Value::as_str), Some("removed-created-file"));
        assert!(!path.join("NEW.md").exists());
        let closed = read_tx(&backup).unwrap();
        assert_eq!(closed.get("state").and_then(Value::as_str), Some(TX_ROLLED_BACK));
        let err = rollback(&root, &backup, "NEW.md", "p").expect_err("twice");
        assert_eq!(err.code, "projection.already_rolled_back");
        let _ = fs::remove_dir_all(path);
    }

    #[cfg(unix)]
    #[test]
    fn the_transaction_record_lands_before_the_target() {
        use std::os::unix::fs::PermissionsExt;
        let (path, root) = scratch("pending");
        fs::create_dir_all(path.join("locked")).unwrap();
        fs::write(path.join("locked/AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("locked/AGENTS.md", "two\n"), "p").unwrap();
        let backup = path.join("b");
        // A read-only directory makes the target write fail after the
        // record was written.
        fs::set_permissions(path.join("locked"), fs::Permissions::from_mode(0o555)).unwrap();
        let err = apply(&root, &previewed, &backup, true).expect_err("io");
        fs::set_permissions(path.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(err.code, "projection.io");
        let tx = read_tx(&backup).expect("record written first");
        assert_eq!(tx.get("state").and_then(Value::as_str), Some(TX_PENDING));
        assert_eq!(fs::read_to_string(path.join("locked/AGENTS.md")).unwrap(), "one\n");
        // Rolling back a pending transaction whose target was never touched
        // closes the record without writing.
        let undone = rollback(&root, &backup, "locked/AGENTS.md", "p").unwrap();
        assert_eq!(undone.get("action").and_then(Value::as_str), Some("target-untouched"));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn secret_bearing_content_is_refused_before_any_backup_exists() {
        let (path, root) = scratch("secret");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let err = preview(
            &root,
            &intent("AGENTS.md", "token=ghp_fixture_not_a_real_secret_00\n"),
            "p",
        )
        .expect_err("desired secret");
        assert_eq!(err.code, "projection.contains_secrets");

        // A secret already in the target would be archived by the backup.
        fs::write(path.join("AGENTS.md"), "sk-abcdefghijklmnopqrstuvwxyz0123\n").unwrap();
        let err = preview(&root, &intent("AGENTS.md", "clean\n"), "p").expect_err("backup secret");
        assert_eq!(err.code, "projection.contains_secrets");
        assert!(!path.join("backup").exists());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn a_preview_is_bound_to_its_project_and_consumed_by_its_apply() {
        let (path, root) = scratch("scope");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let for_a = preview(&root, &intent("AGENTS.md", "two\n"), "project-a").unwrap();
        let for_b = preview(&root, &intent("AGENTS.md", "two\n"), "project-b").unwrap();
        assert_ne!(for_a.tx_id, for_b.tx_id, "the transaction id carries the project");
        let err = for_a.check_applicable("project-b").expect_err("scope");
        assert_eq!(err.code, "projection.preview_scope");
        for_a.check_applicable("project-a").unwrap();
        let err = for_a.with_state(PREVIEW_APPLIED).check_applicable("project-a").expect_err("used");
        assert_eq!(err.code, "projection.tx_consumed");
        let err = for_a.with_state(PREVIEW_ROLLED_BACK).check_applicable("project-a").expect_err("used");
        assert_eq!(err.code, "projection.tx_consumed");
        let record = for_a.to_record();
        assert_eq!(record.get("state").and_then(Value::as_str), Some(PREVIEW_PREVIEWED));
        assert_eq!(Preview::from_record(&record).unwrap().project_digest, "project-a");
        let _ = fs::remove_dir_all(path);
    }

    #[cfg(unix)]
    #[test]
    fn pending_transactions_are_judged_without_rewriting_anything() {
        use std::os::unix::fs::PermissionsExt;
        let (path, root) = scratch("judge");
        let apply_root = path.join("store-apply");
        fs::create_dir_all(path.join("locked")).unwrap();
        fs::write(path.join("locked/AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("locked/AGENTS.md", "two\n"), "p").unwrap();
        fs::set_permissions(path.join("locked"), fs::Permissions::from_mode(0o555)).unwrap();
        let _ = apply(&root, &previewed, &apply_root.join(&previewed.tx_id), true).expect_err("io");
        fs::set_permissions(path.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();

        // Target untouched: aborted.
        let judged = pending_transactions(&root, &apply_root, "p").unwrap();
        assert_eq!(judged.len(), 1);
        assert_eq!(judged[0].get("judgement").and_then(Value::as_str), Some("aborted"));
        // The applied bytes landed after all: committed-recovered.
        fs::write(path.join("locked/AGENTS.md"), "two\n").unwrap();
        let judged = pending_transactions(&root, &apply_root, "p").unwrap();
        assert_eq!(judged[0].get("judgement").and_then(Value::as_str), Some("committed-recovered"));
        // Neither: in-doubt, and the target is left exactly as found.
        fs::write(path.join("locked/AGENTS.md"), "someone else\n").unwrap();
        let judged = pending_transactions(&root, &apply_root, "p").unwrap();
        assert_eq!(judged[0].get("judgement").and_then(Value::as_str), Some("in-doubt"));
        assert_eq!(judged[0].get("target_rewritten"), Some(&Value::Bool(false)));
        assert_eq!(fs::read_to_string(path.join("locked/AGENTS.md")).unwrap(), "someone else\n");
        let tx = read_tx(&apply_root.join(&previewed.tx_id)).unwrap();
        assert_eq!(tx.get("state").and_then(Value::as_str), Some(TX_PENDING), "record not rewritten");
        let _ = fs::remove_dir_all(path);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_fifos_and_pre_placed_directories_are_refused_not_followed() {
        use std::os::unix::fs::symlink;
        let (path, root) = scratch("links");
        let outside = std::env::temp_dir().join(format!("cx-proj-victim-{}", std::process::id()));
        fs::write(&outside, "victim\n").unwrap();

        // A dangling symlink is not a file to project into.
        symlink(path.join("nowhere"), path.join("DANGLING.md")).unwrap();
        let err = preview(&root, &intent("DANGLING.md", "x\n"), "p").expect_err("dangling");
        assert_eq!(err.code, "projection.not_a_file");

        // A symlink pointing outside the project escapes.
        symlink(&outside, path.join("OUT.md")).unwrap();
        let err = preview(&root, &intent("OUT.md", "x\n"), "p").expect_err("outside");
        assert_eq!(err.code, "projection.escapes");
        assert_eq!(fs::read_to_string(&outside).unwrap(), "victim\n");

        // A non-regular file (here a socket; a FIFO takes the same branch) is
        // refused before anything reads it — a read would block forever.
        let sock = path.join("SOCK.md");
        let _listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
        let err = preview(&root, &intent("SOCK.md", "x\n"), "p").expect_err("socket");
        assert_eq!(err.code, "projection.not_a_file");

        // A pre-placed symlink at the transaction directory cannot redirect
        // the backup and record out of the store.
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let previewed = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        let elsewhere = std::env::temp_dir().join(format!("cx-proj-elsewhere-{}", std::process::id()));
        let _ = fs::remove_dir_all(&elsewhere);
        fs::create_dir_all(&elsewhere).unwrap();
        let apply_root = path.join("store/apply");
        fs::create_dir_all(&apply_root).unwrap();
        symlink(&elsewhere, apply_root.join(&previewed.tx_id)).unwrap();
        let err = apply(&root, &previewed, &apply_root.join(&previewed.tx_id), true).expect_err("linked dir");
        assert_eq!(err.code, "projection.io");
        assert!(!elsewhere.join("before").exists() && !elsewhere.join("tx.json").exists());
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "one\n");

        // A pre-placed symlink where the backup file would land is refused too.
        let tx_dir = path.join("store/apply-b");
        fs::create_dir_all(&tx_dir).unwrap();
        symlink(&outside, tx_dir.join("before")).unwrap();
        let err = apply(&root, &previewed, &tx_dir, true).expect_err("linked backup");
        assert_eq!(err.code, "projection.io");
        assert_eq!(fs::read_to_string(&outside).unwrap(), "victim\n");
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "one\n");

        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&elsewhere);
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn every_preview_owns_its_transaction_so_no_backup_is_overwritten() {
        let (path, root) = scratch("owntx");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let first = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        let again = preview(&root, &intent("AGENTS.md", "two\n"), "p").unwrap();
        assert_ne!(first.tx_id, again.tx_id, "a preview never reuses a transaction id");
        assert!(valid_tx_id(&first.tx_id) && valid_tx_id(&again.tx_id));
        assert!(!valid_tx_id("../../evil") && !valid_tx_id("tx_ZZZZZZZZZZZZZZZZ"));

        let apply_root = path.join("store/apply");
        apply(&root, &first, &apply_root.join(&first.tx_id), true).unwrap();
        let second = preview(&root, &intent("AGENTS.md", "three\n"), "p").unwrap();
        apply(&root, &second, &apply_root.join(&second.tx_id), true).unwrap();
        // The first transaction's backup still holds the original bytes.
        assert_eq!(fs::read_to_string(apply_root.join(&first.tx_id).join(BACKUP_NAME)).unwrap(), "one\n");
        // The first can no longer be rolled back over the second's edit ...
        let err = rollback(&root, &apply_root.join(&first.tx_id), "AGENTS.md", "p").expect_err("conflict");
        assert_eq!(err.code, "projection.rollback_conflict");
        // ... but unwinding in order reaches the original bytes.
        rollback(&root, &apply_root.join(&second.tx_id), "AGENTS.md", "p").unwrap();
        rollback(&root, &apply_root.join(&first.tx_id), "AGENTS.md", "p").unwrap();
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "one\n");

        // A record from another project is refused even when the bytes match.
        let other = preview(&root, &intent("AGENTS.md", "two\n"), "q").unwrap();
        apply(&root, &other, &apply_root.join(&other.tx_id), true).unwrap();
        let err = rollback(&root, &apply_root.join(&other.tx_id), "AGENTS.md", "p").expect_err("project");
        assert_eq!(err.code, "projection.rollback_scope");
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "two\n");
        let tx = read_tx(&apply_root.join(&other.tx_id)).unwrap();
        assert_eq!(tx.get("backup").and_then(Value::as_str), Some(BACKUP_NAME), "no absolute path in the record");
        assert_eq!(tx.get("project_digest").and_then(Value::as_str), Some("q"));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn control_paths_are_never_targets() {
        let (path, root) = scratch("control");
        for rel in [".git/config", ".ctxpect/store/exceptions/x.json", "a/.git/hooks/pre-commit", "sub/../.git/x", "a//b"] {
            let err = preview(&root, &intent(rel, "x\n"), "p").expect_err(rel);
            assert!(
                err.code == "projection.control_path" || err.code == "projection.path",
                "{rel}: {}",
                err.code
            );
        }
        // Wherever the store was placed, a target inside it is a control path.
        let store = path.join("elsewhere/store");
        fs::create_dir_all(&store).unwrap();
        let err = target_outside_store(&root, "elsewhere/store/policies/active.json", &store).expect_err("store");
        assert_eq!(err.code, "projection.control_path");
        target_outside_store(&root, "elsewhere/notes.md", &store).unwrap();
        // A file name that merely contains `..` is a plain name.
        fs::write(path.join("notes..md"), "n\n").unwrap();
        preview(&root, &intent("notes..md", "x\n"), "p").unwrap();
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn a_secret_smuggled_into_a_persisted_preview_is_refused_at_apply() {
        let (path, root) = scratch("smuggle");
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let mut previewed = preview(&root, &intent("AGENTS.md", "clean\n"), "p").unwrap();
        // The record on disk is edited to carry a secret with a matching digest.
        previewed.desired = "token=ghp_fixture_not_a_real_secret_00\n".to_string();
        previewed.desired_digest = sha256_hex(previewed.desired.as_bytes());
        let restored = Preview::from_record(&previewed.to_record()).unwrap();
        let backup = path.join("store/apply").join(&restored.tx_id);
        let err = apply(&root, &restored, &backup, true).expect_err("secret at apply");
        assert_eq!(err.code, "projection.contains_secrets");
        assert!(!backup.exists(), "refused before any backup exists");
        assert_eq!(fs::read_to_string(path.join("AGENTS.md")).unwrap(), "one\n");
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn a_torn_record_is_listed_as_unreadable_and_other_projects_are_skipped() {
        let (path, root) = scratch("torn");
        let apply_root = path.join("store/apply");
        fs::create_dir_all(apply_root.join("tx_0000000000000000")).unwrap();
        fs::write(apply_root.join("tx_0000000000000000/tx.json"), "{\"schema\":\"ctxpect-projection-tx-v1\",\"state\":\"pen").unwrap();
        fs::write(path.join("AGENTS.md"), "one\n").unwrap();
        let other = preview(&root, &intent("AGENTS.md", "two\n"), "q").unwrap();
        let dir = apply_root.join(&other.tx_id);
        fs::create_dir_all(&dir).unwrap();
        write_atomic(&dir.join("tx.json"), canonical_json(&tx_meta(&other, true, TX_PENDING)).as_bytes()).unwrap();
        let judged = pending_transactions(&root, &apply_root, "p").unwrap();
        assert_eq!(judged.len(), 1, "{judged:?}");
        assert_eq!(judged[0].get("judgement").and_then(Value::as_str), Some("unreadable"));
        assert_eq!(judged[0].get("tx_id").and_then(Value::as_str), Some("tx_0000000000000000"));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn apply_without_policy_is_denied_and_shell_is_refused() {
        let (path, root) = scratch("policy");
        let previewed = preview(&root, &intent("AGENTS.md", "x\n"), "p").unwrap();
        let err = apply(&root, &previewed, &path.join("b"), false).expect_err("policy");
        assert_eq!(err.code, "projection.policy_denied");
        let err = launch_argv("codex", &["exec", "$(rm -rf /)"]).expect_err("inj");
        assert_eq!(err.code, "launch.injection");
        let _ = fs::remove_dir_all(path);
    }
}
