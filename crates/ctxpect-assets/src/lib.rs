//! Asset supply chain: vet, then copy, then record.
//!
//! This is **not** a package manager (R09). It resolves nothing, downloads
//! nothing, and understands no version ranges. It does one job: copy a file
//! that a version-controlled registry already declares — by origin, license
//! and content digest — into a declared location inside the project, as a
//! reversible transaction, and record what landed.
//!
//! The supply-chain rule from F-11 is the reason the vetting exists: an asset
//! with no license, or of unknown origin, is not copied. A file whose bytes
//! disagree with the registered digest is not copied either — that is what
//! stops a look-alike from taking a trusted name.

use ctxpect_fs::Root;
use ctxpect_schema::{array, object, sha256_hex, string, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const REGISTRY_SCHEMA: &str = "ctxpect-assets-v1";
pub const LOCK_SCHEMA: &str = "ctxpect-asset-lock-v1";
pub const SBOM_SCHEMA: &str = "ctxpect-sbom-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetError {
    pub code: &'static str,
    pub message: String,
}

impl AssetError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// A registered asset. Every field is required before anything is copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredAsset {
    pub asset_id: String,
    /// Where the bytes came from, as a reviewable identifier. This slice
    /// accepts project-relative sources only; nothing is fetched.
    pub origin: String,
    pub license: String,
    /// SHA-256 the registry expects. A mismatch refuses the copy.
    pub digest: String,
    /// Project-relative destination.
    pub target_rel: String,
}

fn field<'a>(record: &'a Value, name: &str) -> Option<&'a str> {
    record.get(name).and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// Read one asset out of the registry, refusing an incomplete declaration.
///
/// The refusals are separate codes because they are separate problems: an
/// unlicensed asset is a policy question, an unknown origin is a provenance
/// question, and a missing target is a malformed record.
pub fn registered(registry: Option<&Value>, asset_id: &str) -> Result<RegisteredAsset, AssetError> {
    let Some(registry) = registry else {
        return Err(AssetError::new(
            "assets.registry_absent",
            "no asset registry is present; copying is fail-closed",
        ));
    };
    let schema = registry.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != REGISTRY_SCHEMA {
        return Err(AssetError::new(
            "assets.registry_schema",
            format!("asset registry schema must be `{REGISTRY_SCHEMA}`, found `{schema}`"),
        ));
    }
    let items = registry
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AssetError::new(
                "assets.registry_malformed",
                "asset registry must carry an `assets` array",
            )
        })?;
    // An asset that is not registered cannot be copied. This is what stops a
    // look-alike name from being installed.
    let record = items
        .iter()
        .find(|item| field(item, "asset_id") == Some(asset_id))
        .ok_or_else(|| {
            AssetError::new(
                "assets.not_registered",
                format!("`{asset_id}` is not in the asset registry"),
            )
        })?;

    let origin = field(record, "origin").ok_or_else(|| {
        AssetError::new(
            "assets.origin_unknown",
            format!("`{asset_id}` declares no origin; an asset of unknown provenance is not copied"),
        )
    })?;
    let license = field(record, "license").ok_or_else(|| {
        AssetError::new(
            "assets.license_unknown",
            format!("`{asset_id}` declares no license; an unlicensed asset is not copied"),
        )
    })?;
    let digest = field(record, "digest").ok_or_else(|| {
        AssetError::new(
            "assets.digest_absent",
            format!("`{asset_id}` declares no content digest; there is nothing to verify against"),
        )
    })?;
    let target_rel = field(record, "target_rel").ok_or_else(|| {
        AssetError::new(
            "assets.target_absent",
            format!("`{asset_id}` declares no target_rel"),
        )
    })?;

    // The origin names a file inside the project. Nothing is fetched, so an
    // origin that tries to escape the project is refused outright.
    if !origin.starts_with("project:") {
        return Err(AssetError::new(
            "assets.origin_unsupported",
            format!("origin `{origin}` is not supported; this slice copies only `project:<rel>` sources and fetches nothing"),
        ));
    }
    for path in [origin.trim_start_matches("project:"), target_rel] {
        if path.is_empty() || path.starts_with('/') || path.contains("..") {
            return Err(AssetError::new(
                "assets.path_escapes",
                format!("`{path}` must be a relative path inside the project"),
            ));
        }
    }

    Ok(RegisteredAsset {
        asset_id: asset_id.to_string(),
        origin: origin.to_string(),
        license: license.to_string(),
        digest: digest.to_string(),
        target_rel: target_rel.to_string(),
    })
}

/// Resolve a project-relative path that may not exist yet.
///
/// `Root::contain` canonicalises, so it refuses a target whose file — or
/// whose parent directories — have not been created. Containment is still
/// enforced: the nearest existing ancestor is canonicalised and checked, and
/// the remaining segments are known to be plain names because the registry
/// already refused `..` and absolute paths.
fn contained_new_path(root: &Root, rel: &str) -> Result<PathBuf, AssetError> {
    let joined = root.path().join(rel);
    if let Ok(path) = root.contain(&joined) {
        return Ok(path);
    }
    let mut tail = Vec::new();
    let mut cursor = joined.as_path();
    while !cursor.exists() {
        let name = cursor.file_name().ok_or_else(|| {
            AssetError::new("assets.path_escapes", format!("`{rel}` has no resolvable base"))
        })?;
        tail.push(name.to_owned());
        cursor = cursor.parent().ok_or_else(|| {
            AssetError::new("assets.path_escapes", format!("`{rel}` has no resolvable base"))
        })?;
    }
    // The deepest existing ancestor is the containment anchor; a symlinked
    // ancestor pointing outside the project fails here.
    let mut out = root.contain(cursor).map_err(|_| {
        AssetError::new("assets.path_escapes", format!("`{rel}` escapes the project"))
    })?;
    for part in tail.iter().rev() {
        out.push(part);
    }
    Ok(out)
}

/// What a copy would do, decided before anything is written.
#[derive(Debug, Clone)]
pub struct CopyPlan {
    pub tx_id: String,
    pub asset: RegisteredAsset,
    pub source_rel: String,
    /// Digest of the bytes actually on disk at the source.
    pub actual_digest: String,
    /// Digest of the target before the copy; empty when the target is absent.
    pub target_before: String,
    pub overwrites: bool,
}

impl CopyPlan {
    #[must_use]
    pub fn to_value(&self) -> Value {
        object([
            ("tx_id", string(&self.tx_id)),
            ("asset_id", string(&self.asset.asset_id)),
            ("origin", string(&self.asset.origin)),
            ("license", string(&self.asset.license)),
            ("target_rel", string(&self.asset.target_rel)),
            ("source_rel", string(&self.source_rel)),
            ("digest", string(&self.actual_digest)),
            ("target_before_digest", string(&self.target_before)),
            ("overwrites", Value::Bool(self.overwrites)),
            (
                "loss",
                string(if self.overwrites {
                    "existing target content is replaced; the previous bytes are kept in the transaction backup"
                } else {
                    "none"
                }),
            ),
            ("authority", string("contexpect-assets-copy-executor")),
        ])
    }
}

/// Vet the asset and describe the copy. Nothing is written.
///
/// Vetting happens here, before any write, so a refusal costs nothing and
/// leaves nothing behind.
pub fn preview(root: &Root, asset: &RegisteredAsset) -> Result<CopyPlan, AssetError> {
    let source_rel = asset.origin.trim_start_matches("project:").to_string();
    let source = root
        .contain(&source_rel)
        .map_err(|_| AssetError::new("assets.source_escapes", "source path escapes the project"))?;
    let bytes = fs::read(&source).map_err(|err| {
        AssetError::new(
            "assets.source_missing",
            format!("cannot read `{source_rel}`: {err}"),
        )
    })?;
    let actual = sha256_hex(&bytes);
    // Registered digest versus the bytes actually present. A file that does
    // not hash to what the registry recorded is not the registered asset,
    // whatever its name says.
    if actual != asset.digest {
        return Err(AssetError::new(
            "assets.digest_mismatch",
            format!(
                "`{}` hashes to {actual}, but the registry recorded {}; refusing to copy",
                asset.asset_id, asset.digest
            ),
        ));
    }

    let target = contained_new_path(root, &asset.target_rel)?;
    let target_before = probe_target(&target)?
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_default();

    static NEXT_PREVIEW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos()).unwrap_or(0);
    let serial = NEXT_PREVIEW.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(CopyPlan {
        tx_id: format!("tx_{}", &sha256_hex(format!(
            "{}|{}|{nonce}|{}|{serial}", root.path().display(), asset.asset_id, std::process::id()
        ).as_bytes())[..16]),
        source_rel,
        actual_digest: actual,
        overwrites: !target_before.is_empty(),
        target_before,
        asset: asset.clone(),
    })
}

/// Perform the copy as a reversible transaction.
///
/// `policy_allows` is the caller's authorization decision; this executor does
/// not make it and refuses without it. The target is re-hashed first, so an
/// edit made between preview and apply aborts rather than being overwritten.
pub fn apply(
    root: &Root,
    plan: &CopyPlan,
    backup_dir: &Path,
    policy_allows: bool,
) -> Result<Value, AssetError> {
    if !policy_allows {
        return Err(AssetError::new(
            "assets.policy_denied",
            "copy requires a prior policy/Approval pass",
        ));
    }
    let source = root
        .contain(&plan.source_rel)
        .map_err(|_| AssetError::new("assets.source_escapes", "source path escapes the project"))?;
    let bytes = fs::read(&source)
        .map_err(|err| AssetError::new("assets.io", format!("cannot read source: {err}")))?;
    if sha256_hex(&bytes) != plan.actual_digest {
        return Err(AssetError::new(
            "assets.source_changed",
            "source changed since preview; refusing to copy different bytes than were vetted",
        ));
    }

    // A secret-bearing executor path is refused, not archived: neither the
    // bytes being copied nor the bytes a backup would keep may carry a
    // credential shape (the same gate projection uses).
    secret_gate("asset content", &bytes)?;
    let target = contained_new_path(root, &plan.asset.target_rel)?;
    let current = probe_target(&target)?;
    if let Some(current) = &current {
        secret_gate("current target content (would enter the backup)", current)?;
    }
    let now_before = current
        .as_deref()
        .map(sha256_hex)
        .unwrap_or_default();
    if now_before != plan.target_before {
        return Err(AssetError::new(
            "assets.concurrent_hash",
            "target changed since preview; refuse to overwrite concurrent edits",
        ));
    }

    if let Some(parent) = backup_dir.parent() {
        fs::create_dir_all(parent).map_err(|err| AssetError::new("assets.io", err.to_string()))?;
        ctxpect_fs::real_dir(parent).map_err(|err| AssetError::new("assets.io", err.to_string()))?;
    }
    // One transaction owns one backup directory. Never replace a previous
    // attempt's before-image, including an interrupted attempt.
    fs::create_dir(backup_dir).map_err(|err| AssetError::new(
        if err.kind() == std::io::ErrorKind::AlreadyExists { "assets.tx_exists" } else { "assets.io" },
        "copy transaction directory is unavailable; use a new preview or recover the existing transaction",
    ))?;
    ctxpect_fs::real_dir(backup_dir).map_err(|err| AssetError::new("assets.io", err.to_string()))?;
    // Only an overwrite has previous bytes to keep. Whether the copy created
    // the file is recorded in `tx.json`, so rollback removes it rather than
    // restoring an empty one. The backup and the pending record land before
    // the target is touched: an interruption leaves a record, not a half
    // state.
    if plan.overwrites {
        let previous = current.unwrap_or_default();
        write_atomic(&backup_dir.join("before"), &previous)?;
    }
    let tx_path = backup_dir.join("tx.json");
    write_atomic(
        &tx_path,
        ctxpect_schema::canonical_json(&tx_meta(root, plan, TX_PENDING)).as_bytes(),
    )?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| AssetError::new("assets.io", err.to_string()))?;
    }
    write_atomic(&target, &bytes)?;

    let meta = tx_meta(root, plan, TX_COMMITTED);
    write_atomic(&tx_path, ctxpect_schema::canonical_json(&meta).as_bytes())?;
    Ok(meta)
}

/// Transaction record states.
pub const TX_PENDING: &str = "pending";
pub const TX_COMMITTED: &str = "committed";
pub const TX_ROLLED_BACK: &str = "rolled-back";

fn tx_meta(root: &Root, plan: &CopyPlan, state: &str) -> Value {
    object([
        ("schema", string("ctxpect-assets-tx-v1")),
        ("project_digest", string(sha256_hex(format!("project:{}", root.path().display()).as_bytes()))),
        ("tx_id", string(&plan.tx_id)),
        ("asset_id", string(&plan.asset.asset_id)),
        ("target_rel", string(&plan.asset.target_rel)),
        ("before_digest", string(&plan.target_before)),
        ("after_digest", string(&plan.actual_digest)),
        ("created_target", Value::Bool(!plan.overwrites)),
        ("state", string(state)),
        ("authority", string("contexpect-assets-copy-executor")),
    ])
}

/// Refuse a payload that carries a credential shape (`assets.contains_secrets`).
fn secret_gate(label: &str, bytes: &[u8]) -> Result<(), AssetError> {
    let text = String::from_utf8_lossy(bytes);
    if let Some(class) = ctxpect_doctor::secret_literal(&text) {
        return Err(AssetError::new(
            "assets.contains_secrets",
            format!("{label} carries a `{}` secret shape; a secret-bearing executor path is refused", class.as_str()),
        ));
    }
    Ok(())
}

/// Write through an exclusively created sibling temp file and rename into
/// place (see [`ctxpect_fs::write_atomic`]): a pre-placed link at the temp
/// name or at the target is refused, never written through.
fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), AssetError> {
    ctxpect_fs::write_atomic(target, bytes).map_err(|err| AssetError::new("assets.io", err.to_string()))
}

/// Read the target if it is a regular file; `None` if nothing is there.
/// A symlink, FIFO, socket or directory is refused (`assets.not_a_file`)
/// before it is read, so nothing blocks on it or writes through it.
fn probe_target(target: &Path) -> Result<Option<Vec<u8>>, AssetError> {
    match fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_file() => fs::read(target)
            .map(Some)
            .map_err(|err| AssetError::new("assets.io", format!("cannot read target: {err}"))),
        Ok(_) => Err(AssetError::new(
            "assets.not_a_file",
            "target exists but is not a regular file; only a regular file can be a copy target",
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AssetError::new("assets.io", err.to_string())),
    }
}

/// Undo a copy: restore the previous bytes, or remove a file the copy created.
///
/// The target is re-hashed against the recorded `after_digest` first. A
/// target someone edited after the copy is left alone
/// (`assets.rollback_conflict`): rollback undoes the copy, not the user.
pub fn rollback(root: &Root, backup_dir: &Path) -> Result<Value, AssetError> {
    let meta_text = fs::read_to_string(backup_dir.join("tx.json"))
        .map_err(|_| AssetError::new("assets.no_tx", "no copy transaction to roll back"))?;
    let meta = ctxpect_schema::parse(&meta_text)
        .map_err(|err| AssetError::new("assets.parse", err.to_string()))?;
    let scope = sha256_hex(format!("project:{}", root.path().display()).as_bytes());
    if meta.get("project_digest").and_then(Value::as_str) != Some(scope.as_str()) {
        return Err(AssetError::new("assets.project_mismatch", "copy transaction has no matching project identity; refusing rollback"));
    }
    let target_rel = meta
        .get("target_rel")
        .and_then(Value::as_str)
        .ok_or_else(|| AssetError::new("assets.parse", "transaction has no target_rel"))?
        .to_string();
    let target_rel = target_rel.as_str();
    let state = meta
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or(TX_COMMITTED)
        .to_string();
    let state = state.as_str();
    if state == TX_ROLLED_BACK {
        return Err(AssetError::new(
            "assets.already_rolled_back",
            "this copy transaction was already rolled back",
        ));
    }
    let created = meta.get("created_target").and_then(Value::as_bool) == Some(true);
    let before_digest = meta
        .get("before_digest")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let after_digest = meta
        .get("after_digest")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let target = contained_new_path(root, target_rel)?;
    let probed = probe_target(&target)?;
    let exists_now = probed.is_some();
    let now_digest = probed.as_deref().map(sha256_hex).unwrap_or_default();
    let at_after = exists_now && now_digest == after_digest;
    let at_before = if created {
        !exists_now
    } else {
        exists_now && now_digest == before_digest
    };

    let (action, restored) = if at_after {
        if created {
            fs::remove_file(&target)
                .map_err(|err| AssetError::new("assets.io", err.to_string()))?;
            ("removed-created-file", None)
        } else {
            let previous = fs::read(backup_dir.join("before"))
                .map_err(|err| AssetError::new("assets.io", err.to_string()))?;
            if sha256_hex(&previous) != before_digest {
                return Err(AssetError::new(
                    "assets.backup_corrupt",
                    "backup bytes do not match the recorded before_digest",
                ));
            }
            write_atomic(&target, &previous)?;
            ("restored-previous-bytes", Some(sha256_hex(&previous)))
        }
    } else if at_before && state == TX_PENDING {
        ("target-untouched", None)
    } else {
        return Err(AssetError::new(
            "assets.rollback_conflict",
            "target was modified after the copy; rollback refuses to discard the concurrent edit",
        ));
    };

    let mut closed = match meta {
        Value::Object(map) => map,
        _ => Default::default(),
    };
    closed.insert("state".into(), string(TX_ROLLED_BACK));
    write_atomic(
        &backup_dir.join("tx.json"),
        ctxpect_schema::canonical_json(&Value::Object(closed)).as_bytes(),
    )?;
    let mut out = vec![
        ("rolled_back", Value::Bool(true)),
        ("target_rel", string(target_rel)),
        ("action", string(action)),
    ];
    if let Some(digest) = restored {
        out.push(("restored_digest", string(digest)));
    }
    Ok(object(out))
}

/// One line of the lock file: what landed, from where, under which license.
#[must_use]
pub fn lock_entry(plan: &CopyPlan) -> Value {
    object([
        ("tx_id", string(&plan.tx_id)),
        ("asset_id", string(&plan.asset.asset_id)),
        ("origin", string(&plan.asset.origin)),
        ("license", string(&plan.asset.license)),
        ("digest", string(&plan.actual_digest)),
        ("target_rel", string(&plan.asset.target_rel)),
    ])
}

#[must_use]
pub fn lock_document(entries: Vec<Value>) -> Value {
    object([
        ("schema", string(LOCK_SCHEMA)),
        ("assets", array(entries)),
    ])
}

/// A bill of materials for what this project actually installed.
///
/// It is deliberately *not* claimed to be SPDX or CycloneDX: emitting either
/// format means conforming to its specification, and a document that merely
/// looks like one would be worse than an honest own-schema listing.
#[must_use]
pub fn sbom(lock: &Value) -> Value {
    let entries = lock.get("assets").and_then(Value::as_array).unwrap_or(&[]);
    let mut components = Vec::new();
    let mut unlicensed = 0i64;
    for item in entries {
        let license = item.get("license").and_then(Value::as_str).unwrap_or("");
        if license.is_empty() {
            unlicensed += 1;
        }
        components.push(object([
            ("name", item.get("asset_id").cloned().unwrap_or(Value::Null)),
            ("origin", item.get("origin").cloned().unwrap_or(Value::Null)),
            ("license", string(license)),
            (
                "hash",
                object([
                    ("alg", string("sha256")),
                    ("value", item.get("digest").cloned().unwrap_or(Value::Null)),
                ]),
            ),
            (
                "installed_at",
                item.get("target_rel").cloned().unwrap_or(Value::Null),
            ),
        ]));
    }
    object([
        ("schema", string(SBOM_SCHEMA)),
        ("components", array(components)),
        ("component_count", Value::Int(entries.len() as i64)),
        ("unlicensed_components", Value::Int(unlicensed)),
        // Scope, stated so nobody reads this as a whole-dependency SBOM.
        (
            "scope",
            string("assets copied by this executor; not the Cargo or npm dependency graph"),
        ),
        (
            "spdx_or_cyclonedx",
            string("not-emitted"),
        ),
        (
            "spdx_or_cyclonedx_reason",
            string("emitting either format requires conforming to its specification; an approximate document carrying its name would misrepresent conformance"),
        ),
    ])
}

/// Where a copy transaction keeps its backup.
#[must_use]
pub fn backup_dir(store_root: &Path, tx_id: &str) -> PathBuf {
    store_root.join(format!("apply/{tx_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    struct Scratch {
        path: PathBuf,
    }

    impl Scratch {
        fn new(label: &str) -> Scratch {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "cx-assets-{label}-{}-{}",
                std::process::id(),
                nanos % 1_000_000
            ));
            fs::create_dir_all(&path).expect("mkdir");
            Scratch {
                path: fs::canonicalize(&path).expect("canon"),
            }
        }

        fn write(&self, rel: &str, contents: &str) {
            let target = self.path.join(rel);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("parent");
            }
            fs::write(target, contents).expect("write");
        }

        fn read(&self, rel: &str) -> Option<String> {
            fs::read_to_string(self.path.join(rel)).ok()
        }

        fn root(&self) -> Root {
            Root::new(&self.path).expect("root")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    const BODY: &str = "skill body\n";

    fn registry_json(extra: &str) -> Value {
        let digest = sha256_hex(BODY.as_bytes());
        parse(&format!(
            r#"{{"schema":"ctxpect-assets-v1","assets":[{{"asset_id":"skill-a","origin":"project:vendor/skill-a.md","license":"MIT","digest":"{digest}","target_rel":".ctxpect/skills/skill-a.md"{extra}}}]}}"#
        ))
        .unwrap()
    }

    #[test]
    fn an_unregistered_or_incomplete_asset_is_not_copied() {
        let registry = registry_json("");
        let err = registered(Some(&registry), "look-alike").expect_err("unregistered");
        assert_eq!(err.code, "assets.not_registered");

        let err = registered(None, "skill-a").expect_err("absent");
        assert_eq!(err.code, "assets.registry_absent");

        // Each missing declaration is its own refusal.
        for (field, code) in [
            ("license", "assets.license_unknown"),
            ("origin", "assets.origin_unknown"),
            ("digest", "assets.digest_absent"),
            ("target_rel", "assets.target_absent"),
        ] {
            let mut doc = match registry_json("") {
                Value::Object(map) => map,
                _ => unreachable!(),
            };
            if let Some(Value::Array(items)) = doc.get_mut("assets")
                && let Some(Value::Object(first)) = items.first_mut()
            {
                first.remove(field);
            }
            let err = registered(Some(&Value::Object(doc)), "skill-a")
                .unwrap_err();
            assert_eq!(err.code, code, "missing {field}");
        }
    }

    #[test]
    fn nothing_is_fetched_and_no_path_escapes_the_project() {
        let remote = parse(
            r#"{"schema":"ctxpect-assets-v1","assets":[{"asset_id":"a","origin":"https://example.invalid/a","license":"MIT","digest":"00","target_rel":"x"}]}"#,
        )
        .unwrap();
        let err = registered(Some(&remote), "a").expect_err("remote");
        assert_eq!(err.code, "assets.origin_unsupported");

        let escaping = parse(
            r#"{"schema":"ctxpect-assets-v1","assets":[{"asset_id":"a","origin":"project:../../etc/passwd","license":"MIT","digest":"00","target_rel":"x"}]}"#,
        )
        .unwrap();
        let err = registered(Some(&escaping), "a").expect_err("escape");
        assert_eq!(err.code, "assets.path_escapes");

        let escaping_target = parse(
            r#"{"schema":"ctxpect-assets-v1","assets":[{"asset_id":"a","origin":"project:ok.md","license":"MIT","digest":"00","target_rel":"/etc/x"}]}"#,
        )
        .unwrap();
        let err = registered(Some(&escaping_target), "a").expect_err("abs target");
        assert_eq!(err.code, "assets.path_escapes");
    }

    #[test]
    fn copy_backups_are_unique_and_cannot_be_reused_or_rolled_into_another_project() {
        let scratch = Scratch::new("tx-identity");
        scratch.write("vendor/skill-a.md", BODY);
        scratch.write(".ctxpect/skills/skill-a.md", "original");
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();
        let next = preview(&root, &asset).unwrap();
        assert_ne!(plan.tx_id, next.tx_id);
        let backup = backup_dir(&scratch.path.join("store"), &plan.tx_id);
        apply(&root, &plan, &backup, true).unwrap();
        let before = fs::read(backup.join("before")).unwrap();
        let record = fs::read(backup.join("tx.json")).unwrap();
        let other = Scratch::new("other-project");
        other.write(".ctxpect/skills/skill-a.md", BODY);
        assert_eq!(rollback(&other.root(), &backup).unwrap_err().code, "assets.project_mismatch");
        assert_eq!(other.read(".ctxpect/skills/skill-a.md").as_deref(), Some(BODY));
        scratch.write(".ctxpect/skills/skill-a.md", "original");
        assert_eq!(apply(&root, &plan, &backup, true).unwrap_err().code, "assets.tx_exists");
        assert_eq!(fs::read(backup.join("before")).unwrap(), before);
        assert_eq!(fs::read(backup.join("tx.json")).unwrap(), record);
    }

    #[test]
    fn bytes_that_disagree_with_the_registry_are_refused() {
        let scratch = Scratch::new("digest");
        // A look-alike: right name and license, different content.
        scratch.write("vendor/skill-a.md", "malicious body\n");
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let err = preview(&scratch.root(), &asset).expect_err("digest");
        assert_eq!(err.code, "assets.digest_mismatch");
        // Nothing was written.
        assert!(scratch.read(".ctxpect/skills/skill-a.md").is_none());
    }

    #[test]
    fn a_secret_bearing_target_is_never_archived_into_a_backup() {
        let scratch = Scratch::new("secret");
        scratch.write("vendor/skill-a.md", BODY);
        // The file the copy would overwrite carries a credential shape.
        scratch.write(".ctxpect/skills/skill-a.md", "token=ghp_fixture_not_a_real_secret_00\n");
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();
        assert!(plan.overwrites);
        let backup = scratch.path.join("store/apply/tx-secret");
        let err = apply(&root, &plan, &backup, true).expect_err("secret");
        assert_eq!(err.code, "assets.contains_secrets");
        assert!(!backup.exists(), "refused before any backup exists");
        assert_eq!(scratch.read(".ctxpect/skills/skill-a.md").as_deref(), Some("token=ghp_fixture_not_a_real_secret_00\n"));
    }

    #[test]
    fn a_vetted_copy_is_reversible_and_records_what_landed() {
        let scratch = Scratch::new("copy");
        scratch.write("vendor/skill-a.md", BODY);
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();
        assert!(!plan.overwrites);
        assert_eq!(
            plan.to_value().get("loss").and_then(Value::as_str),
            Some("none")
        );
        // Preview writes nothing.
        assert!(scratch.read(".ctxpect/skills/skill-a.md").is_none());

        let backup = scratch.path.join("store/apply/tx1");
        // Without authorization the executor refuses.
        let err = apply(&root, &plan, &backup, false).expect_err("policy");
        assert_eq!(err.code, "assets.policy_denied");
        assert!(scratch.read(".ctxpect/skills/skill-a.md").is_none());

        let meta = apply(&root, &plan, &backup, true).unwrap();
        assert_eq!(
            meta.get("created_target").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(scratch.read(".ctxpect/skills/skill-a.md").as_deref(), Some(BODY));

        // The lock records provenance, not just the fact of a copy.
        let lock = lock_document(vec![lock_entry(&plan)]);
        let bom = sbom(&lock);
        assert_eq!(bom.get("component_count").and_then(Value::as_i64), Some(1));
        assert_eq!(
            bom.get("unlicensed_components").and_then(Value::as_i64),
            Some(0)
        );
        let components = bom.get("components").and_then(Value::as_array).unwrap();
        assert_eq!(
            components[0].get("license").and_then(Value::as_str),
            Some("MIT")
        );
        assert_eq!(
            components[0].get("origin").and_then(Value::as_str),
            Some("project:vendor/skill-a.md")
        );
        // The document does not claim a conformance it does not have.
        assert_eq!(
            bom.get("spdx_or_cyclonedx").and_then(Value::as_str),
            Some("not-emitted")
        );

        // Rollback removes a file the copy created.
        let undone = rollback(&root, &backup).unwrap();
        assert_eq!(
            undone.get("action").and_then(Value::as_str),
            Some("removed-created-file")
        );
        assert!(scratch.read(".ctxpect/skills/skill-a.md").is_none());
    }

    #[test]
    fn overwriting_keeps_the_previous_bytes_and_restores_them() {
        let scratch = Scratch::new("overwrite");
        scratch.write("vendor/skill-a.md", BODY);
        scratch.write(".ctxpect/skills/skill-a.md", "previous\n");
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();
        assert!(plan.overwrites);
        assert!(
            plan.to_value()
                .get("loss")
                .and_then(Value::as_str)
                .is_some_and(|loss| loss.contains("replaced")),
            "an overwrite must state the loss"
        );

        let backup = scratch.path.join("store/apply/tx2");
        apply(&root, &plan, &backup, true).unwrap();
        assert_eq!(scratch.read(".ctxpect/skills/skill-a.md").as_deref(), Some(BODY));

        let undone = rollback(&root, &backup).unwrap();
        assert_eq!(
            undone.get("action").and_then(Value::as_str),
            Some("restored-previous-bytes")
        );
        assert_eq!(
            scratch.read(".ctxpect/skills/skill-a.md").as_deref(),
            Some("previous\n")
        );
    }

    #[test]
    fn a_target_edited_between_preview_and_apply_aborts_the_copy() {
        let scratch = Scratch::new("concurrent");
        scratch.write("vendor/skill-a.md", BODY);
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();

        // Someone writes the target after the preview was taken.
        scratch.write(".ctxpect/skills/skill-a.md", "someone else\n");
        let backup = scratch.path.join("store/apply/tx3");
        let err = apply(&root, &plan, &backup, true).expect_err("concurrent");
        assert_eq!(err.code, "assets.concurrent_hash");
        assert_eq!(
            scratch.read(".ctxpect/skills/skill-a.md").as_deref(),
            Some("someone else\n"),
            "the concurrent edit must survive"
        );
    }

    #[test]
    fn a_target_edited_after_the_copy_is_not_rolled_over() {
        let scratch = Scratch::new("rbconflict");
        scratch.write("vendor/skill-a.md", BODY);
        scratch.write(".ctxpect/skills/skill-a.md", "previous\n");
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();
        let backup = scratch.path.join("store/apply/tx5");
        let meta = apply(&root, &plan, &backup, true).unwrap();
        assert_eq!(meta.get("state").and_then(Value::as_str), Some(TX_COMMITTED));

        scratch.write(".ctxpect/skills/skill-a.md", "edited after copy\n");
        let err = rollback(&root, &backup).expect_err("conflict");
        assert_eq!(err.code, "assets.rollback_conflict");
        assert_eq!(
            scratch.read(".ctxpect/skills/skill-a.md").as_deref(),
            Some("edited after copy\n"),
            "the later edit must survive"
        );

        // Put the copy's bytes back; now the rollback proceeds and closes.
        scratch.write(".ctxpect/skills/skill-a.md", BODY);
        rollback(&root, &backup).unwrap();
        assert_eq!(scratch.read(".ctxpect/skills/skill-a.md").as_deref(), Some("previous\n"));
        let err = rollback(&root, &backup).expect_err("twice");
        assert_eq!(err.code, "assets.already_rolled_back");
    }

    #[test]
    fn a_source_changed_after_vetting_is_not_copied() {
        let scratch = Scratch::new("source");
        scratch.write("vendor/skill-a.md", BODY);
        let root = scratch.root();
        let asset = registered(Some(&registry_json("")), "skill-a").unwrap();
        let plan = preview(&root, &asset).unwrap();

        // The vetted bytes are replaced after the plan was made.
        scratch.write("vendor/skill-a.md", "swapped after vetting\n");
        let backup = scratch.path.join("store/apply/tx4");
        let err = apply(&root, &plan, &backup, true).expect_err("source changed");
        assert_eq!(err.code, "assets.source_changed");
        assert!(scratch.read(".ctxpect/skills/skill-a.md").is_none());
    }

    #[test]
    fn the_sbom_counts_unlicensed_components_rather_than_hiding_them() {
        // A lock written by an older run could carry an entry with no
        // license; the SBOM must surface that rather than print a clean list.
        let lock = parse(
            r#"{"schema":"ctxpect-asset-lock-v1","assets":[{"asset_id":"a","origin":"project:x","license":"","digest":"00","target_rel":"y"}]}"#,
        )
        .unwrap();
        let bom = sbom(&lock);
        assert_eq!(
            bom.get("unlicensed_components").and_then(Value::as_i64),
            Some(1)
        );
    }
}
