//! File-backed ledger for Receipts, snapshots, sessions, and settings.
//!
//! ADR 0001 freezes SQLite+FTS5 as the target engine. This crate implements the
//! same DTO/ledger contracts with a JSON document store and a simple inverted
//! index so the Cargo workspace stays third-party-free and the nine required
//! gates remain offline. The engine choice is a recorded deviation; the
//! Receipt/Claim contracts are not.

pub mod settings;

pub use settings::{default_settings, settings_schema, validate_settings, SETTINGS_SCHEMA};

use ctxpect_fs::Root;
use ctxpect_receipt::{tombstone, ContinuityKey};
use ctxpect_schema::{
    array, canonical_json, hmac_sha256_hex, object, parse, sha256_hex, string, strip_time_fields,
    Value,
};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// On-disk schema of this ledger layout.
pub const STORE_SCHEMA: &str = "ctxpect-store-v1";

/// A ledger rooted at an authorized directory.
pub struct Store {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    pub code: &'static str,
    pub message: String,
}

impl StoreError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn io(message: impl Into<String>) -> Self {
        Self::new("store.io", message)
    }
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::io(error.to_string())
    }
}

impl Store {
    /// Open or create a store inside an already-contained directory.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        fs::create_dir_all(path)?;
        let store = Self {
            root: path.to_path_buf(),
        };
        store.ensure_layout()?;
        Ok(store)
    }

    /// Open a store only if `path` stays inside `container`.
    pub fn open_contained(container: &Root, path: &Path) -> Result<Self, StoreError> {
        let contained = container
            .contain(path)
            .map_err(|refusal| StoreError::new("store.escapes_root", format!("{refusal:?}")))?;
        Self::open(&contained)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn ensure_layout(&self) -> Result<(), StoreError> {
        for dir in [
            "receipts",
            "snapshots",
            "tombstones",
            "sessions",
            "intents",
            "policies",
            "exceptions",
            "experiments",
            "advisor",
            "apply",
            "audit",
            "keys",
            "insights",
            "standards",
            "sync",
            "notifications",
        ] {
            fs::create_dir_all(self.root.join(dir))?;
        }
        let meta_path = self.root.join("meta.json");
        if !meta_path.exists() {
            let meta = object([
                ("schema", string(STORE_SCHEMA)),
                ("engine", string("json-document-ledger")),
                ("sqlite_fts5", string("deferred-target")),
                ("created_at", string(now_rfc3339())),
            ]);
            atomic_write(&meta_path, &canonical_json(&meta))?;
        }
        let settings_path = self.root.join("settings.json");
        if !settings_path.exists() {
            atomic_write(&settings_path, &canonical_json(&default_settings()))?;
        }
        let index_path = self.root.join("index.json");
        if !index_path.exists() {
            atomic_write(&index_path, &canonical_json(&array([])))?;
        }
        Ok(())
    }

    pub fn continuity_key(&self) -> Result<ContinuityKey, StoreError> {
        let path = self.root.join("keys/continuity.key");
        if path.exists() {
            let secret = fs::read(&path)?;
            return Ok(ContinuityKey::from_secret(secret));
        }
        let mut secret = vec![0u8; 32];
        fill_random(&mut secret);
        atomic_write_bytes(&path, &secret)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }
        Ok(ContinuityKey::from_secret(secret))
    }

    pub fn settings(&self) -> Result<Value, StoreError> {
        read_json(&self.root.join("settings.json"))
    }

    /// Replace settings, whole and validated.
    ///
    /// Validation lives here rather than in a caller so the CLI, the API and
    /// any future client cannot disagree about what the store accepts.
    pub fn put_settings(&self, value: Value) -> Result<(), StoreError> {
        validate_settings(&value)?;
        atomic_write(&self.root.join("settings.json"), &canonical_json(&value))?;
        self.audit("settings.put", "settings", None)?;
        Ok(())
    }

    pub fn put_snapshot(&self, id: &str, snapshot: &Value) -> Result<(), StoreError> {
        validate_id(id)?;
        atomic_write(
            &self.root.join(format!("snapshots/{id}.json")),
            &canonical_json(snapshot),
        )
    }

    pub fn get_snapshot(&self, id: &str) -> Result<Value, StoreError> {
        validate_id(id)?;
        read_json(&self.root.join(format!("snapshots/{id}.json")))
    }

    pub fn put_receipt(&self, receipt: &Value) -> Result<String, StoreError> {
        let id = receipt
            .get("receipt_id")
            .and_then(Value::as_str)
            .ok_or_else(|| StoreError::new("store.missing_id", "receipt_id required"))?
            .to_string();
        validate_id(&id)?;
        if self.root.join(format!("tombstones/{id}.json")).exists() {
            return Err(StoreError::new(
                "store.tombstoned",
                "a tombstoned receipt cannot be recreated with the same id",
            ));
        }
        let path = self.root.join(format!("receipts/{id}.json"));
        if path.exists() {
            // First write wins. A later persist of the same snapshot id may
            // differ only in created_at / MAC; overwriting would break immutability.
            return Ok(id);
        }
        atomic_write(&path, &canonical_json(receipt))?;
        self.index_insert(receipt)?;
        self.audit("receipt.put", &id, None)?;
        Ok(id)
    }

    pub fn get_receipt(&self, id: &str) -> Result<Value, StoreError> {
        validate_id(id)?;
        let tomb = self.root.join(format!("tombstones/{id}.json"));
        if tomb.exists() {
            return read_json(&tomb);
        }
        read_json(&self.root.join(format!("receipts/{id}.json")))
    }

    pub fn delete_receipt(&self, id: &str, reason: &str) -> Result<Value, StoreError> {
        validate_id(id)?;
        let path = self.root.join(format!("receipts/{id}.json"));
        if !path.exists() {
            return Err(StoreError::new("store.missing", format!("receipt {id}")));
        }
        let receipt = read_json(&path)?;
        let stone = tombstone(&receipt, &now_rfc3339(), reason)
            .map_err(|err| StoreError::new(err.code, err.message))?;
        atomic_write(
            &self.root.join(format!("tombstones/{id}.json")),
            &canonical_json(&stone),
        )?;
        fs::remove_file(&path)?;
        self.drop_derived(id)?;
        self.index_remove(id)?;
        self.audit("receipt.delete", id, Some(reason))?;
        Ok(stone)
    }

    pub fn list_receipts(&self) -> Result<Value, StoreError> {
        read_json(&self.root.join("index.json"))
    }

    /// Write a named document. Like [`Store::put_receipt`], the write lands in
    /// the audit chain: a store mutation that leaves no trace is not
    /// reconstructable afterwards.
    pub fn put_named(&self, folder: &str, id: &str, value: &Value) -> Result<(), StoreError> {
        validate_id(id)?;
        validate_id(folder)?;
        fs::create_dir_all(self.root.join(folder))?;
        let existed = self.root.join(format!("{folder}/{id}.json")).exists();
        atomic_write(
            &self.root.join(format!("{folder}/{id}.json")),
            &canonical_json(value),
        )?;
        self.audit(
            if existed { "named.replace" } else { "named.put" },
            &format!("{folder}/{id}"),
            None,
        )?;
        Ok(())
    }

    pub fn get_named(&self, folder: &str, id: &str) -> Result<Value, StoreError> {
        validate_id(id)?;
        validate_id(folder)?;
        read_json(&self.root.join(format!("{folder}/{id}.json")))
    }

    /// Delete a named document, returning whether one was actually there.
    ///
    /// The caller needs that distinction: reporting a deletion that removed
    /// nothing reads as a destructive action that never happened. The
    /// deletion is audited either way.
    pub fn delete_named(&self, folder: &str, id: &str) -> Result<bool, StoreError> {
        validate_id(id)?;
        validate_id(folder)?;
        let path = self.root.join(format!("{folder}/{id}.json"));
        let existed = path.exists();
        if existed {
            fs::remove_file(path)?;
        }
        self.audit(
            "named.delete",
            &format!("{folder}/{id}"),
            Some(if existed { "removed" } else { "absent" }),
        )?;
        Ok(existed)
    }

    pub fn list_named(&self, folder: &str) -> Result<Vec<String>, StoreError> {
        validate_id(folder)?;
        let dir = self.root.join(folder);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(id) = name.strip_suffix(".json") {
                ids.push(id.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    pub fn search(&self, query: &str) -> Result<Vec<String>, StoreError> {
        let index = self.list_receipts()?;
        let q = query.to_ascii_lowercase();
        let mut hits = Vec::new();
        if let Some(items) = index.as_array() {
            for item in items {
                let blob = canonical_json(item).to_ascii_lowercase();
                if blob.contains(&q)
                    && let Some(id) = item.get("receipt_id").and_then(Value::as_str)
                {
                    hits.push(id.to_string());
                }
            }
        }
        Ok(hits)
    }

    fn drop_derived(&self, receipt_id: &str) -> Result<(), StoreError> {
        for folder in ["insights", "advisor", "experiments"] {
            let dir = self.root.join(folder);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let text = fs::read_to_string(&path)?;
                if text.contains(receipt_id) {
                    fs::remove_file(&path)?;
                }
            }
        }
        Ok(())
    }

    fn index_insert(&self, receipt: &Value) -> Result<(), StoreError> {
        let mut items = match read_json(&self.root.join("index.json"))? {
            Value::Array(items) => items,
            _ => Vec::new(),
        };
        let id = receipt
            .get("receipt_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        items.retain(|item| item.get("receipt_id").and_then(Value::as_str) != Some(id));
        items.push(object([
            (
                "receipt_id",
                string(id),
            ),
            (
                "receipt_kind",
                string(
                    receipt
                        .get("receipt_kind")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ),
            ),
            (
                "created_at",
                string(
                    receipt
                        .get("created_at")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ),
            ),
            (
                "harness",
                string(
                    receipt
                        .pointer(&["coordinate", "harness"])
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ),
            ),
            (
                "digest",
                string(
                    receipt
                        .pointer(&["manifest", "digest"])
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ),
            ),
            ("tombstone", Value::Bool(false)),
        ]));
        atomic_write(
            &self.root.join("index.json"),
            &canonical_json(&Value::Array(items)),
        )
    }

    fn index_remove(&self, id: &str) -> Result<(), StoreError> {
        let mut items = match read_json(&self.root.join("index.json"))? {
            Value::Array(items) => items,
            _ => Vec::new(),
        };
        for item in &mut items {
            if item.get("receipt_id").and_then(Value::as_str) == Some(id)
                && let Value::Object(map) = item
            {
                map.insert("tombstone".to_string(), Value::Bool(true));
            }
        }
        atomic_write(
            &self.root.join("index.json"),
            &canonical_json(&Value::Array(items)),
        )
    }

    /// Append an audit entry, linked to the one before it.
    ///
    /// Each entry carries its sequence number, the previous entry's `mac`,
    /// and its own `mac` over both. Deleting, editing or reordering entries
    /// therefore breaks the chain and is detectable.
    ///
    /// The MAC uses the store's continuity key, so the chain cannot be
    /// re-forged without it — but that key lives in this store, so a writer
    /// with full local access could rewrite the whole chain. The guarantee is
    /// the same one Receipt signatures make: it detects tampering that did
    /// not come from a holder of this store's key, and it is never presented
    /// as an organization-level attestation.
    pub fn audit(&self, action: &str, target: &str, detail: Option<&str>) -> Result<(), StoreError> {
        let previous = self.audit_tail()?;
        let seq = previous.as_ref().map_or(0, |(seq, _)| seq + 1);
        let prev_mac = previous.map_or_else(String::new, |(_, mac)| mac);
        let body = object([
            ("at", string(now_rfc3339())),
            ("action", string(action)),
            ("target", string(target)),
            (
                "detail",
                match detail {
                    Some(text) => string(text),
                    None => Value::Null,
                },
            ),
            ("seq", Value::Int(seq)),
            ("prev", string(&prev_mac)),
        ]);
        let key = self.continuity_key()?;
        let mac = hmac_sha256_hex(&key.secret, canonical_json(&body).as_bytes());
        let entry = match body {
            Value::Object(mut map) => {
                map.insert("mac".to_string(), string(&mac));
                Value::Object(map)
            }
            other => other,
        };
        let path = self.root.join("audit/events.jsonl");
        let mut file = fs::OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{}", canonical_json(&entry))?;
        Ok(())
    }

    /// Sequence number and MAC of the last entry, if any.
    fn audit_tail(&self) -> Result<Option<(i64, String)>, StoreError> {
        let path = self.root.join("audit/events.jsonl");
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path)?;
        let Some(line) = text.lines().rfind(|line| !line.trim().is_empty()) else {
            return Ok(None);
        };
        let entry =
            parse(line).map_err(|err| StoreError::new("store.audit_parse", err.to_string()))?;
        match (
            entry.get("seq").and_then(Value::as_i64),
            entry.get("mac").and_then(Value::as_str),
        ) {
            (Some(seq), Some(mac)) => Ok(Some((seq, mac.to_string()))),
            // A tail written before the chain existed has no seq or mac. The
            // new entry starts a fresh chain rather than silently claiming
            // continuity over records it cannot verify.
            _ => Ok(None),
        }
    }

    /// Read the audit log and verify its linkage.
    ///
    /// Entries written before the chain existed carry no `mac`; they are
    /// reported as `unverifiable-legacy` rather than as either valid or
    /// tampered, because neither claim would be true.
    pub fn audit_chain(&self) -> Result<Value, StoreError> {
        let path = self.root.join("audit/events.jsonl");
        if !path.exists() {
            return Ok(object([
                ("schema", string("ctxpect-audit-chain-v1")),
                ("entries", array(Vec::new())),
                ("count", Value::Int(0)),
                ("verified", Value::Bool(true)),
                ("legacy_entries", Value::Int(0)),
                ("reason_code", string("audit.empty")),
            ]));
        }
        let text = fs::read_to_string(path)?;
        let key = self.continuity_key()?;
        let mut entries = Vec::new();
        let mut legacy = 0i64;
        let mut expected_prev = String::new();
        let mut expected_seq = 0i64;
        let mut broken: Option<&'static str> = None;

        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let entry =
                parse(line).map_err(|err| StoreError::new("store.audit_parse", err.to_string()))?;
            let Some(mac) = entry.get("mac").and_then(Value::as_str) else {
                legacy += 1;
                entries.push(entry);
                // A legacy run resets the expectation: the chain restarts at
                // the first linked entry after it.
                expected_prev = String::new();
                expected_seq = 0;
                continue;
            };
            let Value::Object(map) = &entry else {
                broken.get_or_insert("audit.entry_malformed");
                entries.push(entry);
                continue;
            };
            let mut body = map.clone();
            body.remove("mac");
            let recomputed = hmac_sha256_hex(&key.secret, canonical_json(&Value::Object(body)).as_bytes());
            if recomputed != mac {
                broken.get_or_insert("audit.mac_mismatch");
            } else if entry.get("prev").and_then(Value::as_str).unwrap_or("") != expected_prev {
                broken.get_or_insert("audit.link_broken");
            } else if entry.get("seq").and_then(Value::as_i64) != Some(expected_seq) {
                broken.get_or_insert("audit.sequence_broken");
            }
            expected_prev = mac.to_string();
            expected_seq += 1;
            entries.push(entry);
        }

        let count = entries.len() as i64;
        Ok(object([
            ("schema", string("ctxpect-audit-chain-v1")),
            ("count", Value::Int(count)),
            ("verified", Value::Bool(broken.is_none())),
            ("legacy_entries", Value::Int(legacy)),
            (
                "reason_code",
                string(broken.unwrap_or(if legacy > 0 {
                    "audit.unverifiable_legacy_present"
                } else {
                    "ok"
                })),
            ),
            // A local MAC is not an organization attestation, and this
            // document does not let itself be read as one.
            ("org_identity", Value::Bool(false)),
            ("entries", array(entries)),
        ]))
    }
}

fn validate_id(id: &str) -> Result<(), StoreError> {
    if id.is_empty()
        || id.len() > 80
        || id.contains("..")
        || id.contains('/')
        || id.contains('\\')
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(StoreError::new("store.bad_id", "id must be a safe token"));
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, StoreError> {
    let text = fs::read_to_string(path).map_err(|_| {
        StoreError::new("store.missing", format!("{}", path.display()))
    })?;
    parse(&text).map_err(|err| StoreError::new("store.parse", err.to_string()))
}

fn atomic_write(path: &Path, text: &str) -> Result<(), StoreError> {
    atomic_write_bytes(path, text.as_bytes())
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn fill_random(buf: &mut [u8]) {
    if let Ok(mut file) = fs::File::open("/dev/urandom") {
        use std::io::Read;
        if file.read_exact(buf).is_ok() {
            return;
        }
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let hex = sha256_hex(format!("{nanos}").as_bytes());
    let raw = hex.as_bytes();
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = raw[i % raw.len()];
    }
}

#[must_use]
pub fn now_rfc3339() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{}Z", now.as_secs(), now.subsec_millis())
}

/// Digest of a document with time fields stripped — used by tests for stability.
#[must_use]
pub fn stable_digest(value: &Value) -> String {
    ctxpect_schema::digest_value(&strip_time_fields(value))
}

#[cfg(test)]
mod audit_chain_tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cx-audit-{label}-{}-{}",
            std::process::id(),
            nanos % 1_000_000
        ));
        fs::create_dir_all(&path).expect("mkdir");
        fs::canonicalize(&path).expect("canon")
    }

    #[test]
    fn the_chain_verifies_and_detects_every_kind_of_tampering() {
        let root = scratch("chain");
        let store = Store::open(&root).expect("open");
        for n in 0..4 {
            store.audit("test.action", &format!("target-{n}"), None).expect("audit");
        }

        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("count").and_then(Value::as_i64), Some(4));
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(true));
        assert_eq!(chain.get("reason_code").and_then(Value::as_str), Some("ok"));
        // A local MAC is not an organization attestation.
        assert_eq!(chain.get("org_identity").and_then(Value::as_bool), Some(false));

        let path = root.join("audit/events.jsonl");
        let original = fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = original.lines().collect();

        // Editing an entry breaks its MAC.
        let edited = original.replace("target-1", "target-X");
        fs::write(&path, &edited).expect("write");
        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(false));
        assert_eq!(
            chain.get("reason_code").and_then(Value::as_str),
            Some("audit.mac_mismatch")
        );

        // Deleting an entry breaks the link.
        let without_second = format!("{}\n{}\n{}\n", lines[0], lines[2], lines[3]);
        fs::write(&path, &without_second).expect("write");
        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(false));

        // Reordering breaks it too.
        let reordered = format!("{}\n{}\n{}\n{}\n", lines[0], lines[2], lines[1], lines[3]);
        fs::write(&path, &reordered).expect("write");
        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(false));

        // Restoring the original restores the verdict.
        fs::write(&path, &original).expect("write");
        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(true));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn entries_written_before_the_chain_are_reported_as_unverifiable() {
        let root = scratch("legacy");
        let store = Store::open(&root).expect("open");
        // An entry in the old shape: no seq, no prev, no mac.
        let path = root.join("audit/events.jsonl");
        fs::create_dir_all(root.join("audit")).expect("mkdir");
        fs::write(
            &path,
            "{\"action\":\"old.action\",\"at\":\"1\",\"detail\":null,\"target\":\"t\"}\n",
        )
        .expect("write");

        store.audit("new.action", "t2", None).expect("audit");
        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("count").and_then(Value::as_i64), Some(2));
        assert_eq!(chain.get("legacy_entries").and_then(Value::as_i64), Some(1));
        // Neither "valid" nor "tampered" would be true of the legacy entry,
        // so the chain says exactly that.
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(true));
        assert_eq!(
            chain.get("reason_code").and_then(Value::as_str),
            Some("audit.unverifiable_legacy_present")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_log_is_a_verified_empty_chain() {
        let root = scratch("empty");
        let store = Store::open(&root).expect("open");
        let chain = store.audit_chain().expect("chain");
        assert_eq!(chain.get("count").and_then(Value::as_i64), Some(0));
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(true));
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_receipt::{migrate_dev_inspect_v0, ContinuityKey};
    use ctxpect_schema::parse;

    fn scratch() -> (tempdir::Guard, Store) {
        let dir = tempdir::new("store");
        let store = Store::open(&dir.path).expect("open");
        (dir, store)
    }

    mod tempdir {
        use super::*;
        use std::sync::atomic::{AtomicU64, Ordering};

        static SEQ: AtomicU64 = AtomicU64::new(0);

        pub struct Guard {
            pub path: PathBuf,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
        pub fn new(label: &str) -> Guard {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let seq = SEQ.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cx-store-{label}-{}-{nanos}-{seq}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("mkdir");
            Guard { path }
        }
    }

    fn sample_receipt(store: &Store) -> Value {
        let snap = parse(
            r#"{"schema":"dev-inspect-v0","receipt_kind":"development-snapshot","snapshot_digest":"x","scope":{"harness":"codex","version":"0.147.0","surface":"cli","os_lane":"macos-27-arm64","cwd":"<project>/"},"results":[],"unknown":[],"findings":[],"policy_result":{"verdict":"pass"}}"#,
        )
        .unwrap();
        let key = store.continuity_key().unwrap();
        migrate_dev_inspect_v0(&snap, "one-shot", Some(&key), "t", "r_sample1").unwrap()
    }

    #[test]
    fn receipts_are_immutable_and_tombstone_invalidates_derived() {
        let (_dir, store) = scratch();
        let receipt = sample_receipt(&store);
        store.put_receipt(&receipt).unwrap();
        store
            .put_named(
                "insights",
                "i1",
                &object([("receipt_id", string("r_sample1")), ("note", string("x"))]),
            )
            .unwrap();
        let mut changed = receipt.clone();
        if let Value::Object(map) = &mut changed {
            map.insert("receipt_kind".to_string(), string("ci"));
        }
        store.put_receipt(&changed).expect("first-write-wins");
        let stored = store.get_receipt("r_sample1").unwrap();
        assert_eq!(
            stored.get("receipt_kind").and_then(Value::as_str),
            Some("one-shot")
        );
        store.delete_receipt("r_sample1", "user-request").unwrap();
        assert!(store.get_named("insights", "i1").is_err());
        let stone = store.get_receipt("r_sample1").unwrap();
        assert!(stone.get("tombstone").is_some());
        assert_eq!(
            stone
                .pointer(&["tombstone", "external_copies"])
                .and_then(Value::as_str),
            Some("not-recallable")
        );
    }

    #[test]
    fn continuity_key_round_trip() {
        let (_dir, store) = scratch();
        let a = store.continuity_key().unwrap();
        let b = store.continuity_key().unwrap();
        assert_eq!(a.key_id, b.key_id);
        assert_eq!(a.secret, b.secret);
        let _ = ContinuityKey::from_secret(a.secret.clone());
    }
}
