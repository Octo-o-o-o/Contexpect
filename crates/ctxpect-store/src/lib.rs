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
    array, canonical_json, hmac_sha256_hex, object, parse, string, strip_time_fields, Value,
};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// On-disk schema of this ledger layout.
pub const STORE_SCHEMA: &str = "ctxpect-store-v1";
/// Schema of a write-ahead journal record (`journal/<op-id>.json`).
pub const JOURNAL_SCHEMA: &str = "ctxpect-store-journal-v1";
/// Schema of the `store status` document.
pub const STORE_STATUS_SCHEMA: &str = "ctxpect-store-status-v1";
/// The advisory lock file, relative to the store root.
const LOCK_FILE: &str = "lock";
/// Schema of the audit chain-tail anchor (`audit/head.json`).
pub const AUDIT_HEAD_SCHEMA: &str = "ctxpect-audit-head-v1";

fn audit_head_input(seq: i64, mac: &str) -> String {
    format!("{AUDIT_HEAD_SCHEMA}\n{seq}\n{mac}")
}

/// Held while one process mutates the store. See [`Store::lock_mutation`].
///
/// Dropping the guard releases the OS advisory lock. A reentrant acquisition
/// (the same process already holds the lock) yields a guard that owns
/// nothing and releases nothing on drop.
#[must_use = "the lock is released when this guard is dropped"]
#[derive(Debug)]
pub struct MutationLock {
    key: Option<PathBuf>,
    file: Option<fs::File>,
}

impl Drop for MutationLock {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            if let Ok(mut held) = HELD_LOCKS.lock() {
                held.remove(&key);
            }
            if let Some(file) = self.file.take() {
                let _ = file.unlock();
            }
        }
    }
}

/// Lock keys this process currently holds, so a mutation nested inside
/// another (a post-Receipt inside an apply) is reentrant instead of busy.
static HELD_LOCKS: std::sync::Mutex<std::collections::BTreeSet<PathBuf>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

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
        // Finish or close out any Receipt write that did not reach its end —
        // but only while holding the mutation lock: a record that belongs to
        // a write another live process is in the middle of is not a crash
        // leftover, and replaying it would race that writer. When the lock
        // is busy the records stay where they are and `status` reports them.
        match store.lock_mutation() {
            Ok(_guard) => {
                store.recover_journal()?;
            }
            Err(err) if err.code == "store.busy" => {}
            Err(err) => return Err(err),
        }
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
            "journal",
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
            // An empty index is only minted for a store that holds nothing.
            // Receipts without an index are a damaged ledger, reported as
            // `store.index_missing` and rebuilt by `store repair`, never read
            // as an empty one.
            let has_documents = ["receipts", "tombstones"].iter().any(|folder| {
                fs::read_dir(self.root.join(folder))
                    .map(|mut dir| dir.next().is_some())
                    .unwrap_or(false)
            });
            if !has_documents {
                atomic_write(&index_path, &canonical_json(&array([])))?;
            }
        }
        Ok(())
    }

    pub fn continuity_key(&self) -> Result<ContinuityKey, StoreError> {
        let path = self.root.join("keys/continuity.key");
        if path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::symlink_metadata(&path)?.permissions().mode();
                if mode & 0o077 != 0 {
                    return Err(StoreError::new(
                        "store.key_permissions",
                        format!(
                            "keys/continuity.key is readable by others (mode {:o}); refusing to use a key that may have leaked",
                            mode & 0o777
                        ),
                    ));
                }
            }
            let secret = fs::read(&path)?;
            return Ok(ContinuityKey::from_secret(secret));
        }
        let _lock = self.lock_mutation()?;
        if path.exists() {
            // Minted by a concurrent opener while this one waited.
            let secret = fs::read(&path)?;
            return Ok(ContinuityKey::from_secret(secret));
        }
        let mut secret = vec![0u8; 32];
        fill_random(&mut secret)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // The key is born 0600: there is no window in which the file exists
        // with the process umask's wider mode.
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path)?;
        file.write_all(&secret)?;
        file.sync_all()?;
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
        let _lock = self.lock_mutation()?;
        atomic_write(&self.root.join("settings.json"), &canonical_json(&value))?;
        self.audit("settings.put", "settings", None)?;
        Ok(())
    }

    pub fn put_snapshot(&self, id: &str, snapshot: &Value) -> Result<(), StoreError> {
        validate_id(id)?;
        let _lock = self.lock_mutation()?;
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
        let _lock = self.lock_mutation()?;
        if self.root.join(format!("tombstones/{id}.json")).exists() {
            return Err(StoreError::new(
                "store.tombstoned",
                "a tombstoned receipt cannot be recreated with the same id",
            ));
        }
        let path = self.root.join(format!("receipts/{id}.json"));
        if path.exists() {
            // First write wins: a later persist of the same snapshot id may
            // differ only in created_at / MAC, and overwriting would break
            // immutability. The index and audit entries are still completed,
            // so a put that failed after the Receipt landed can be retried.
            let stored = read_json(&path)?;
            self.complete_receipt_put(&id, &stored)?;
            return Ok(id);
        }
        // Receipt, index and audit are three files. The journal record lands
        // first and is removed last, so a crash between any two of the writes
        // leaves a record that [`Store::recover_journal`] can act on.
        let journal = self.journal_path("receipt.put", &id);
        self.journal_write(&journal, "receipt.put", &id, "begin")?;
        atomic_write(&path, &canonical_json(receipt))?;
        self.journal_write(&journal, "receipt.put", &id, "receipt-written")?;
        self.index_insert(receipt)?;
        self.journal_write(&journal, "receipt.put", &id, "indexed")?;
        self.audit("receipt.put", &id, None)?;
        fs::remove_file(&journal)?;
        Ok(id)
    }

    /// Bring the index and audit chain in line with a Receipt file that
    /// exists. Idempotent: a present index row or audit entry is left alone.
    fn complete_receipt_put(&self, id: &str, receipt: &Value) -> Result<(), StoreError> {
        if !self.index_has(id)? {
            self.index_insert(receipt)?;
        }
        if !self.audit_has("receipt.put", id)? {
            self.audit("receipt.put", id, None)?;
        }
        Ok(())
    }

    /// Finish a delete whose tombstone already landed: the Receipt file,
    /// derived documents, index row and audit entry are removed / marked.
    fn complete_receipt_delete(&self, id: &str, reason: &str) -> Result<(), StoreError> {
        let path = self.root.join(format!("receipts/{id}.json"));
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.drop_derived(id)?;
        self.index_remove(id)?;
        if !self.audit_has("receipt.delete", id)? {
            self.audit("receipt.delete", id, Some(reason))?;
        }
        Ok(())
    }

    fn journal_path(&self, op: &str, id: &str) -> PathBuf {
        self.root.join(format!("journal/{}-{id}.json", op.replace('.', "-")))
    }

    fn journal_write(&self, path: &Path, op: &str, id: &str, stage: &str) -> Result<(), StoreError> {
        let record = object([
            ("schema", string(JOURNAL_SCHEMA)),
            ("op", string(op)),
            ("receipt_id", string(id)),
            ("stage", string(stage)),
            ("at", string(now_rfc3339())),
        ]);
        atomic_write(path, &canonical_json(&record))
    }

    /// Journal records whose operation did not reach its end.
    fn pending_journal(&self) -> Result<Vec<(PathBuf, Value)>, StoreError> {
        let dir = self.root.join("journal");
        let mut out = Vec::new();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let record = read_json(&path).unwrap_or_else(|err| {
                object([
                    ("schema", string(JOURNAL_SCHEMA)),
                    ("op", string("unreadable")),
                    ("receipt_id", string("")),
                    ("stage", string("unknown")),
                    ("parse_error", string(err.message)),
                ])
            });
            out.push((path, record));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Replay unfinished journal records. Each lands in exactly one of three
    /// states: `committed` (the operation's remaining writes were completed),
    /// `rolled-back` (nothing of it had reached disk, the record is dropped)
    /// or `in-doubt` (the state cannot be decided — typically a corrupt
    /// index; the record stays and `store status` reports it). Retrying never
    /// duplicates an index row or an audit entry.
    pub fn recover_journal(&self) -> Result<Vec<Value>, StoreError> {
        let mut outcomes = Vec::new();
        for (path, record) in self.pending_journal()? {
            let op = record.get("op").and_then(Value::as_str).unwrap_or("").to_string();
            let id = record
                .get("receipt_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let stage = record.get("stage").and_then(Value::as_str).unwrap_or("").to_string();
            let outcome: Result<&'static str, StoreError> = match op.as_str() {
                "receipt.put" if validate_id(&id).is_ok() => {
                    let receipt_path = self.root.join(format!("receipts/{id}.json"));
                    if receipt_path.exists() {
                        read_json(&receipt_path)
                            .and_then(|receipt| self.complete_receipt_put(&id, &receipt))
                            .map(|()| "committed")
                    } else {
                        Ok("rolled-back")
                    }
                }
                "receipt.delete" if validate_id(&id).is_ok() => {
                    if self.root.join(format!("tombstones/{id}.json")).exists() {
                        self.complete_receipt_delete(&id, "journal-recovery")
                            .map(|()| "committed")
                    } else {
                        Ok("rolled-back")
                    }
                }
                _ => Err(StoreError::new(
                    "store.journal_unknown_op",
                    "journal record names an operation this store cannot replay",
                )),
            };
            let (state, reason) = match outcome {
                Ok(state) => {
                    fs::remove_file(&path)?;
                    let _ = self.audit(
                        "journal.recovered",
                        &id,
                        Some(&format!("{op} {state} from stage {stage}")),
                    );
                    (state, Value::Null)
                }
                Err(err) => ("in-doubt", string(err.code)),
            };
            outcomes.push(object([
                ("op", string(&op)),
                ("receipt_id", string(&id)),
                ("stage", string(&stage)),
                ("state", string(state)),
                ("reason_code", reason),
            ]));
        }
        Ok(outcomes)
    }

    /// Rebuild `index.json` from `receipts/` and `tombstones/`. The result is
    /// canonical (sorted by `created_at`, then `receipt_id`), which is also
    /// the order the normal write path keeps, so a rebuilt index is
    /// byte-identical to one that was never corrupted. Unfinished journal
    /// records are replayed afterwards, since a corrupt index is what leaves
    /// them in doubt.
    pub fn rebuild_index(&self) -> Result<Value, StoreError> {
        let _lock = self.lock_mutation()?;
        let mut items = Vec::new();
        let mut receipts = 0i64;
        let mut tombstones = 0i64;
        for id in self.list_named("receipts")? {
            let receipt = read_json(&self.root.join(format!("receipts/{id}.json")))?;
            items.push(index_entry(&receipt, false));
            receipts += 1;
        }
        for id in self.list_named("tombstones")? {
            let stone = read_json(&self.root.join(format!("tombstones/{id}.json")))?;
            items.push(index_entry(&stone, true));
            tombstones += 1;
        }
        // A tombstone supersedes a receipt row of the same id (a delete
        // interrupted before the receipt file was removed).
        let mut deduped: Vec<Value> = Vec::new();
        for item in items {
            let id = item.get("receipt_id").and_then(Value::as_str).unwrap_or("").to_string();
            let is_tomb = item.get("tombstone").and_then(Value::as_bool).unwrap_or(false);
            if let Some(pos) = deduped
                .iter()
                .position(|have| have.get("receipt_id").and_then(Value::as_str) == Some(id.as_str()))
            {
                if is_tomb {
                    deduped[pos] = item;
                }
            } else {
                deduped.push(item);
            }
        }
        self.write_index(deduped)?;
        // A rebuild that could not be audited is not a finished rebuild.
        self.audit("index.rebuilt", "index", None)?;
        let recovered = self.recover_journal()?;
        Ok(object([
            ("rebuilt", Value::Bool(true)),
            ("receipts", Value::Int(receipts)),
            ("tombstones", Value::Int(tombstones)),
            ("journal", array(recovered)),
        ]))
    }

    /// `store repair`: mend a torn audit tail and a stale head anchor, then
    /// rebuild the index and replay the journal. Everything here is
    /// deterministic and idempotent; a second run changes nothing.
    pub fn repair(&self) -> Result<Value, StoreError> {
        let _lock = self.lock_mutation()?;
        let audit = self.repair_audit()?;
        let index = self.rebuild_index()?;
        Ok(match index {
            Value::Object(mut map) => {
                map.insert("audit".to_string(), audit);
                Value::Object(map)
            }
            other => other,
        })
    }

    /// Whether the index can be read and how many rows it holds.
    fn index_status(&self) -> Value {
        match self.list_receipts() {
            Ok(Value::Array(items)) => object([
                ("status", string("ok")),
                ("rows", Value::Int(i64::try_from(items.len()).unwrap_or(0))),
            ]),
            Ok(_) => object([
                ("status", string("corrupt")),
                ("reason_code", string("store.index_corrupt")),
            ]),
            Err(err) => object([
                ("status", string("corrupt")),
                ("reason_code", string(err.code)),
                ("message", string(err.message)),
            ]),
        }
    }

    /// The store's recoverability state: index health, unfinished journal
    /// records (each `in-doubt` until `store repair` resolves it) and the
    /// advisory lock. Read-only.
    pub fn status(&self) -> Result<Value, StoreError> {
        let journal: Vec<Value> = self
            .pending_journal()?
            .into_iter()
            .map(|(path, record)| {
                object([
                    ("op", record.get("op").cloned().unwrap_or(Value::Null)),
                    (
                        "receipt_id",
                        record.get("receipt_id").cloned().unwrap_or(Value::Null),
                    ),
                    ("stage", record.get("stage").cloned().unwrap_or(Value::Null)),
                    ("state", string("in-doubt")),
                    (
                        "file",
                        string(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                    ),
                    ("parse_error", record.get("parse_error").cloned().unwrap_or(Value::Null)),
                ])
            })
            .collect();
        Ok(object([
            ("schema", string(STORE_STATUS_SCHEMA)),
            ("index", self.index_status()),
            (
                "journal",
                object([
                    ("in_doubt", Value::Int(i64::try_from(journal.len()).unwrap_or(0))),
                    ("records", array(journal)),
                ]),
            ),
            ("audit", self.audit_health()),
            ("lock", self.lock_status()),
            (
                "note",
                string("opening replays finished-or-not journal records only while holding the lock; what it cannot decide, and a torn audit tail or missing index, is resolved by `ctxpect store repair`"),
            ),
        ]))
    }

    /// Take the store's advisory lock for one mutation.
    ///
    /// This is a **same-machine advisory lock**: an OS file lock
    /// (`File::try_lock`, `flock` semantics) on `store/lock`, held until the
    /// returned guard is dropped and released by the kernel when the holder
    /// exits, so a crashed holder never leaves a stale lock behind. The file's
    /// content (holder pid and time) is informational. A live holder in
    /// another process makes the call fail with `store.busy`. It is not a
    /// mandatory lock and does not reach across hosts or network filesystems.
    /// Reentrant within one process.
    pub fn lock_mutation(&self) -> Result<MutationLock, StoreError> {
        let key = self.lock_key();
        if HELD_LOCKS.lock().is_ok_and(|held| held.contains(&key)) {
            return Ok(MutationLock { key: None, file: None });
        }
        let path = self.root.join(LOCK_FILE);
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        match file.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => {
                let holder = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| parse(&text).ok())
                    .and_then(|v| v.get("pid").and_then(Value::as_i64));
                return Err(StoreError::new(
                    "store.busy",
                    match holder {
                        Some(pid) => format!("another process (pid {pid}) holds the store lock"),
                        None => "another process holds the store lock".to_string(),
                    },
                ));
            }
            Err(fs::TryLockError::Error(err)) => return Err(err.into()),
        }
        let body = object([
            ("pid", Value::Int(i64::from(std::process::id()))),
            ("at", string(now_rfc3339())),
        ]);
        file.set_len(0)?;
        file.write_all(canonical_json(&body).as_bytes())?;
        if let Ok(mut held) = HELD_LOCKS.lock() {
            held.insert(key.clone());
        }
        Ok(MutationLock {
            key: Some(key),
            file: Some(file),
        })
    }

    fn lock_key(&self) -> PathBuf {
        fs::canonicalize(&self.root)
            .unwrap_or_else(|_| self.root.clone())
            .join(LOCK_FILE)
    }

    /// Who holds the advisory lock, if anyone: `this-process`,
    /// `other-process` or `none`. Probing takes and releases the lock when it
    /// is free, so the answer is what the OS reports, not what a file says.
    pub fn lock_status(&self) -> Value {
        let path = self.root.join(LOCK_FILE);
        let record = fs::read_to_string(&path)
            .ok()
            .and_then(|text| parse(&text).ok())
            .unwrap_or(Value::Null);
        let holder = if HELD_LOCKS.lock().is_ok_and(|held| held.contains(&self.lock_key())) {
            "this-process"
        } else {
            match fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path) {
                Ok(file) => match file.try_lock() {
                    Ok(()) => {
                        let _ = file.unlock();
                        "none"
                    }
                    Err(_) => "other-process",
                },
                Err(_) => "unknown",
            }
        };
        object([
            ("held", Value::Bool(holder != "none" && holder != "unknown")),
            ("holder", string(holder)),
            (
                "pid",
                if holder == "none" { Value::Null } else { record.get("pid").cloned().unwrap_or(Value::Null) },
            ),
            (
                "at",
                if holder == "none" { Value::Null } else { record.get("at").cloned().unwrap_or(Value::Null) },
            ),
            ("scope", string("same-machine advisory file lock (flock); released by the OS when the holder exits; not mandatory, not cross-host")),
        ])
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
        let _lock = self.lock_mutation()?;
        let path = self.root.join(format!("receipts/{id}.json"));
        if !path.exists() {
            return Err(StoreError::new("store.missing", format!("receipt {id}")));
        }
        let receipt = read_json(&path)?;
        let stone = tombstone(&receipt, &now_rfc3339(), reason)
            .map_err(|err| StoreError::new(err.code, err.message))?;
        let journal = self.journal_path("receipt.delete", id);
        self.journal_write(&journal, "receipt.delete", id, "begin")?;
        atomic_write(
            &self.root.join(format!("tombstones/{id}.json")),
            &canonical_json(&stone),
        )?;
        self.journal_write(&journal, "receipt.delete", id, "tombstone-written")?;
        fs::remove_file(&path)?;
        self.drop_derived(id)?;
        self.index_remove(id)?;
        self.journal_write(&journal, "receipt.delete", id, "indexed")?;
        self.audit("receipt.delete", id, Some(reason))?;
        fs::remove_file(&journal)?;
        Ok(stone)
    }

    /// The Receipt index. A file that cannot be read or parsed is reported as
    /// `store.index_corrupt`, never as an empty ledger.
    pub fn list_receipts(&self) -> Result<Value, StoreError> {
        self.read_index().map(Value::Array)
    }

    fn read_index(&self) -> Result<Vec<Value>, StoreError> {
        let path = self.root.join("index.json");
        let text = fs::read_to_string(&path).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                StoreError::new(
                    "store.index_missing",
                    "index.json is missing while the store holds documents; run `store repair` (a missing index is never read as an empty ledger)",
                )
            } else {
                StoreError::new(
                    "store.index_corrupt",
                    format!("index.json cannot be read ({}); run `store repair`", err.kind()),
                )
            }
        })?;
        match parse(&text) {
            Ok(Value::Array(items)) => Ok(items),
            Ok(_) => Err(StoreError::new(
                "store.index_corrupt",
                "index.json is not an array; run `store repair`",
            )),
            Err(err) => Err(StoreError::new(
                "store.index_corrupt",
                format!("index.json does not parse ({err}); run `store repair`"),
            )),
        }
    }

    fn write_index(&self, mut items: Vec<Value>) -> Result<(), StoreError> {
        items.sort_by(|a, b| {
            let key = |item: &Value| {
                (
                    item.get("created_at").and_then(Value::as_str).unwrap_or("").to_string(),
                    item.get("receipt_id").and_then(Value::as_str).unwrap_or("").to_string(),
                )
            };
            key(a).cmp(&key(b))
        });
        atomic_write(&self.root.join("index.json"), &canonical_json(&Value::Array(items)))
    }

    fn index_has(&self, id: &str) -> Result<bool, StoreError> {
        Ok(self
            .read_index()?
            .iter()
            .any(|item| item.get("receipt_id").and_then(Value::as_str) == Some(id)))
    }

    /// Write a named document. Like [`Store::put_receipt`], the write lands in
    /// the audit chain: a store mutation that leaves no trace is not
    /// reconstructable afterwards.
    pub fn put_named(&self, folder: &str, id: &str, value: &Value) -> Result<(), StoreError> {
        validate_id(id)?;
        validate_id(folder)?;
        let _lock = self.lock_mutation()?;
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
        let _lock = self.lock_mutation()?;
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
        let mut items = self.read_index()?;
        let id = receipt
            .get("receipt_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        items.retain(|item| item.get("receipt_id").and_then(Value::as_str) != Some(id));
        items.push(index_entry(receipt, false));
        self.write_index(items)
    }

    fn index_remove(&self, id: &str) -> Result<(), StoreError> {
        let mut items = self.read_index()?;
        for item in &mut items {
            if item.get("receipt_id").and_then(Value::as_str) == Some(id)
                && let Value::Object(map) = item
            {
                map.insert("tombstone".to_string(), Value::Bool(true));
            }
        }
        self.write_index(items)
    }

    /// Whether the audit log already records `action` on `target`.
    fn audit_has(&self, action: &str, target: &str) -> Result<bool, StoreError> {
        let path = self.root.join("audit/events.jsonl");
        if !path.exists() {
            return Ok(false);
        }
        let text = fs::read_to_string(path)?;
        Ok(text.lines().filter(|line| !line.trim().is_empty()).any(|line| {
            parse(line).is_ok_and(|entry| {
                entry.get("action").and_then(Value::as_str) == Some(action)
                    && entry.get("target").and_then(Value::as_str) == Some(target)
            })
        }))
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
        // Read-tail-then-append is only atomic under the store lock; two
        // unlocked writers would mint the same seq and break the chain.
        let _lock = self.lock_mutation()?;
        if self.audit_torn_tail()?.is_some() {
            return Err(StoreError::new(
                "store.audit_torn",
                "audit/events.jsonl ends in a torn line; run `store repair` before writing",
            ));
        }
        let previous = self.audit_tail()?;
        let key = self.continuity_key()?;
        // Appending on top of a cut log would move the anchor past the
        // evidence; a truncated or forged-anchor log is not written to.
        match self.audit_head(&key) {
            Err(code) => {
                return Err(StoreError::new(
                    "store.audit_anchor",
                    format!("audit/head.json is unusable ({code}); the log is not written to until this is resolved"),
                ));
            }
            Ok(Some((head_seq, head_mac))) => {
                let cut = match &previous {
                    None => true,
                    Some((seq, mac)) => head_seq > *seq || (head_seq == *seq && head_mac != *mac),
                };
                if cut {
                    return Err(StoreError::new(
                        "store.audit_truncated",
                        format!("audit/events.jsonl ends before the anchored seq {head_seq}; entries were cut off and the log is not written to"),
                    ));
                }
            }
            Ok(None) => {}
        }
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
        file.sync_all()?;
        self.write_audit_head(seq, &mac, &key)
    }

    /// The chain-tail anchor: `audit/head.json` records the last linked
    /// entry's `seq` and `mac` under its own MAC, so that cutting entries off
    /// the end of the log — which leaves a perfectly linked prefix — is
    /// detectable (`audit.truncated`), not just edits in the middle.
    fn write_audit_head(&self, seq: i64, mac: &str, key: &ContinuityKey) -> Result<(), StoreError> {
        let head_mac = hmac_sha256_hex(&key.secret, audit_head_input(seq, mac).as_bytes());
        atomic_write(
            &self.root.join("audit/head.json"),
            &canonical_json(&object([
                ("schema", string(AUDIT_HEAD_SCHEMA)),
                ("seq", Value::Int(seq)),
                ("mac", string(mac)),
                ("head_mac", string(head_mac)),
            ])),
        )
    }

    /// The recorded chain tail, if the head anchor exists and its MAC holds.
    /// `Err` here means the anchor is present but forged or malformed.
    fn audit_head(&self, key: &ContinuityKey) -> Result<Option<(i64, String)>, &'static str> {
        let path = self.root.join("audit/head.json");
        if !path.exists() {
            return Ok(None);
        }
        let Ok(head) = read_json(&path) else {
            return Err("audit.head_malformed");
        };
        let (Some(seq), Some(mac), Some(head_mac)) = (
            head.get("seq").and_then(Value::as_i64),
            head.get("mac").and_then(Value::as_str),
            head.get("head_mac").and_then(Value::as_str),
        ) else {
            return Err("audit.head_malformed");
        };
        if hmac_sha256_hex(&key.secret, audit_head_input(seq, mac).as_bytes()) != head_mac {
            return Err("audit.head_mismatch");
        }
        Ok(Some((seq, mac.to_string())))
    }

    /// Bytes after the last newline, if the log does not end on a line
    /// boundary: the shape a crashed append leaves behind.
    fn audit_torn_tail(&self) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.root.join("audit/events.jsonl");
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path)?;
        if bytes.is_empty() || bytes.ends_with(b"\n") {
            return Ok(None);
        }
        let start = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        Ok(Some(bytes[start..].to_vec()))
    }

    /// The audit log's own health: `ok`, `torn` (a partial last line),
    /// `unparseable` (a complete line that is not JSON) or `head-stale`
    /// (the anchor lags the log, as after a crash between append and
    /// anchor). All but `ok` are mended by `store repair`.
    pub fn audit_health(&self) -> Value {
        if let Ok(Some(torn)) = self.audit_torn_tail() {
            return object([
                ("status", string("torn")),
                ("reason_code", string("store.audit_torn")),
                ("torn_bytes", Value::Int(i64::try_from(torn.len()).unwrap_or(0))),
            ]);
        }
        match self.audit_tail() {
            Err(err) => object([
                ("status", string("unparseable")),
                ("reason_code", string(err.code)),
                ("message", string(err.message)),
            ]),
            Ok(tail) => {
                let head = self
                    .continuity_key()
                    .map_err(|err| err.code)
                    .and_then(|key| self.audit_head(&key));
                match (tail, head) {
                    (Some((seq, _)), Ok(Some((head_seq, _)))) if seq > head_seq => object([
                        ("status", string("head-stale")),
                        ("reason_code", string("audit.head_stale")),
                    ]),
                    (_, Err(code)) => object([("status", string("anchor-invalid")), ("reason_code", string(code))]),
                    _ => object([("status", string("ok"))]),
                }
            }
        }
    }

    /// Mend the audit log: a torn tail is moved aside into
    /// `audit/torn-<seq>.bin` (nothing is deleted) and the log is cut back
    /// to its last complete line; a head anchor that lags a chain which
    /// itself verifies is re-anchored. A chain that does not verify is
    /// left alone: repair does not launder tampering.
    fn repair_audit(&self) -> Result<Value, StoreError> {
        let mut actions = Vec::new();
        if let Some(torn) = self.audit_torn_tail()? {
            let path = self.root.join("audit/events.jsonl");
            let bytes = fs::read(&path)?;
            let keep = bytes.len() - torn.len();
            let aside = self.root.join(format!("audit/torn-{}.bin", now_rfc3339().replace(['.', ':'], "-")));
            atomic_write_bytes(&aside, &torn)?;
            let file = fs::OpenOptions::new().write(true).open(&path)?;
            file.set_len(u64::try_from(keep).unwrap_or(0))?;
            file.sync_all()?;
            actions.push(string(format!(
                "torn tail of {} bytes moved to {}",
                torn.len(),
                aside.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            )));
        }
        let key = self.continuity_key()?;
        let chain = self.audit_chain()?;
        let linked_ok = chain.get("verified").and_then(Value::as_bool) == Some(true)
            || chain.get("reason_code").and_then(Value::as_str) == Some("audit.head_stale")
            || chain.get("reason_code").and_then(Value::as_str) == Some("audit.head_missing");
        if linked_ok && let Some((seq, mac)) = self.audit_tail()? {
            let stale = !matches!(self.audit_head(&key), Ok(Some((head_seq, ref head_mac))) if head_seq == seq && *head_mac == mac);
            if stale {
                self.write_audit_head(seq, &mac, &key)?;
                actions.push(string(format!("head anchor re-anchored at seq {seq}")));
            }
        }
        if !actions.is_empty() {
            self.audit("audit.repaired", "audit", Some(&canonical_json(&array(actions.clone()))))?;
        }
        Ok(object([
            ("repaired", Value::Bool(!actions.is_empty())),
            ("actions", array(actions)),
        ]))
    }

    /// Sequence number and MAC of the last complete entry, if any.
    fn audit_tail(&self) -> Result<Option<(i64, String)>, StoreError> {
        let path = self.root.join("audit/events.jsonl");
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path)?;
        // Only complete lines take part in the chain; a torn tail is
        // reported by `audit_health` and mended by repair.
        let complete = match bytes.iter().rposition(|b| *b == b'\n') {
            Some(i) => &bytes[..=i],
            None => &bytes[..0],
        };
        let text = String::from_utf8_lossy(complete);
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
        let bytes = fs::read(path)?;
        let torn = !bytes.is_empty() && !bytes.ends_with(b"\n");
        let complete = match bytes.iter().rposition(|b| *b == b'\n') {
            Some(i) => &bytes[..=i],
            None => &bytes[..0],
        };
        let text = String::from_utf8_lossy(complete).into_owned();
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

        // The linked prefix verifying says nothing about entries cut off
        // its end; the head anchor does.
        let linked_tail = if expected_seq > 0 { Some((expected_seq - 1, expected_prev.clone())) } else { None };
        let mut anchor: Option<&'static str> = None;
        if broken.is_none() {
            match (self.audit_head(&key), linked_tail) {
                (Err(code), _) => anchor = Some(code),
                (Ok(None), Some(_)) => anchor = Some("audit.head_missing"),
                (Ok(Some((head_seq, head_mac))), Some((seq, mac))) => {
                    if head_seq > seq {
                        anchor = Some("audit.truncated");
                    } else if head_seq < seq {
                        anchor = Some("audit.head_stale");
                    } else if head_mac != mac {
                        anchor = Some("audit.truncated");
                    }
                }
                (Ok(Some(_)), None) => anchor = Some("audit.truncated"),
                (Ok(None), None) => {}
            }
        }
        if torn {
            anchor.get_or_insert("store.audit_torn");
        }
        let count = entries.len() as i64;
        let verdict = broken.or(anchor);
        Ok(object([
            ("schema", string("ctxpect-audit-chain-v1")),
            ("count", Value::Int(count)),
            ("verified", Value::Bool(verdict.is_none())),
            ("legacy_entries", Value::Int(legacy)),
            ("torn_tail", Value::Bool(torn)),
            (
                "reason_code",
                string(verdict.unwrap_or(if legacy > 0 {
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

/// One index row. A tombstone carries `created_at` and `coordinate` for
/// exactly this purpose; a tombstone written before it did yields empty
/// strings, which the rebuild reproduces deterministically.
fn index_entry(document: &Value, tombstone: bool) -> Value {
    let text = |value: Option<&Value>| string(value.and_then(Value::as_str).unwrap_or(""));
    object([
        ("receipt_id", text(document.get("receipt_id"))),
        ("receipt_kind", text(document.get("receipt_kind"))),
        ("created_at", text(document.get("created_at"))),
        ("harness", text(document.pointer(&["coordinate", "harness"]))),
        ("digest", text(document.pointer(&["manifest", "digest"]))),
        ("tombstone", Value::Bool(tombstone)),
    ])
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
    parse(&text).map_err(|err| {
        StoreError::new(
            "store.parse",
            format!(
                "{}: {err}",
                path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            ),
        )
    })
}

fn atomic_write(path: &Path, text: &str) -> Result<(), StoreError> {
    atomic_write_bytes(path, text.as_bytes())
}

/// Exclusive temp file + fsync + rename (see [`ctxpect_fs::write_atomic`]):
/// the temp name is per process and per write, so two writers never race
/// over one `.tmp`, and a pre-placed link at either name is refused.
fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    ctxpect_fs::write_atomic(path, bytes)?;
    Ok(())
}

/// The platform secure random source this ledger reads key material from.
#[cfg(unix)]
const SECURE_RANDOM_SOURCE: &str = "/dev/urandom";

/// Fill `buf` from the platform secure random source, or fail closed.
///
/// A continuity key derived from a clock would be predictable while looking
/// like a key; refusing to mint one is the only honest answer when the
/// source is unavailable (`store.random_unavailable`).
fn fill_random(buf: &mut [u8]) -> Result<(), StoreError> {
    #[cfg(unix)]
    {
        fill_random_from(Path::new(SECURE_RANDOM_SOURCE), buf)
    }
    #[cfg(not(unix))]
    {
        let _ = buf;
        Err(StoreError::new(
            "store.random_unavailable",
            "no platform secure random source is wired for this OS lane; refusing to mint a predictable continuity key",
        ))
    }
}

/// Read exactly `buf.len()` bytes from `source`. No fallback.
fn fill_random_from(source: &Path, buf: &mut [u8]) -> Result<(), StoreError> {
    use std::io::Read;
    let mut file = fs::File::open(source).map_err(|err| {
        StoreError::new(
            "store.random_unavailable",
            format!("secure random source is unavailable ({}); refusing to mint a predictable continuity key", err.kind()),
        )
    })?;
    file.read_exact(buf).map_err(|err| {
        StoreError::new(
            "store.random_unavailable",
            format!("secure random source returned too few bytes ({}); refusing to mint a predictable continuity key", err.kind()),
        )
    })
}

#[must_use]
pub fn now_rfc3339() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
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
        migrate_dev_inspect_v0(&snap, "one-shot", Some(&key), "t", "t", "r_sample1").unwrap()
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
    fn a_missing_random_source_fails_closed_instead_of_degrading() {
        let mut buf = [0u8; 32];
        let missing = std::env::temp_dir().join(format!(
            "cx-no-such-random-source-{}",
            std::process::id()
        ));
        let err = fill_random_from(&missing, &mut buf).expect_err("missing source");
        assert_eq!(err.code, "store.random_unavailable");
        assert!(buf.iter().all(|b| *b == 0), "nothing may be filled from a clock");

        // A source that runs dry is refused too, not padded.
        let short = std::env::temp_dir().join(format!("cx-short-random-{}", std::process::id()));
        fs::write(&short, [7u8; 4]).unwrap();
        let err = fill_random_from(&short, &mut buf).expect_err("short source");
        assert_eq!(err.code, "store.random_unavailable");
        let _ = fs::remove_file(short);
    }

    fn receipt_with_id(store: &Store, id: &str, created_at: &str) -> Value {
        let snap = parse(
            r#"{"schema":"dev-inspect-v0","receipt_kind":"development-snapshot","snapshot_digest":"x","scope":{"harness":"codex","version":"0.147.0","surface":"cli","os_lane":"macos-27-arm64","cwd":"<project>/"},"results":[],"unknown":[],"findings":[],"policy_result":{"verdict":"pass"}}"#,
        )
        .unwrap();
        let key = store.continuity_key().unwrap();
        migrate_dev_inspect_v0(&snap, "one-shot", Some(&key), created_at, created_at, id).unwrap()
    }

    fn audit_count(store: &Store, action: &str, target: &str) -> usize {
        let text = fs::read_to_string(store.root().join("audit/events.jsonl")).unwrap_or_default();
        text.lines()
            .filter_map(|line| parse(line).ok())
            .filter(|e| {
                e.get("action").and_then(Value::as_str) == Some(action)
                    && e.get("target").and_then(Value::as_str) == Some(target)
            })
            .count()
    }

    #[test]
    fn a_corrupt_index_is_named_and_rebuilt_byte_identically() {
        let (_dir, store) = scratch();
        for (id, at) in [("r_b", "2.0Z"), ("r_a", "1.0Z"), ("r_c", "3.0Z")] {
            store.put_receipt(&receipt_with_id(&store, id, at)).unwrap();
        }
        store.delete_receipt("r_b", "test").unwrap();
        let healthy = fs::read(store.root().join("index.json")).unwrap();

        fs::write(store.root().join("index.json"), "{not json").unwrap();
        let err = store.list_receipts().expect_err("corrupt");
        assert_eq!(err.code, "store.index_corrupt");
        // A put over a corrupt index fails loudly: the Receipt lands, the
        // journal stays, and nothing pretends the index is empty.
        let err = store.put_receipt(&receipt_with_id(&store, "r_d", "4.0Z")).expect_err("corrupt");
        assert_eq!(err.code, "store.index_corrupt");
        assert!(store.root().join("receipts/r_d.json").exists());
        let status = store.status().unwrap();
        assert_eq!(status.pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(1));
        assert_eq!(status.pointer(&["index", "status"]).and_then(Value::as_str), Some("corrupt"));

        // Reopening reports, but does not decide, the in-doubt record.
        let reopened = Store::open(store.root()).unwrap();
        assert_eq!(reopened.status().unwrap().pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(1));

        // Repair: r_d is now indexed and audited exactly once; the rest of
        // the index is byte-identical to the healthy one plus that row.
        let report = reopened.rebuild_index().unwrap();
        assert_eq!(report.get("receipts").and_then(Value::as_i64), Some(3));
        assert_eq!(report.get("tombstones").and_then(Value::as_i64), Some(1));
        assert_eq!(reopened.status().unwrap().pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(0));
        assert_eq!(audit_count(&reopened, "receipt.put", "r_d"), 1);
        let rebuilt = fs::read(reopened.root().join("index.json")).unwrap();
        assert_ne!(rebuilt, healthy, "r_d was added");
        reopened.delete_receipt("r_d", "test").unwrap();
        fs::remove_file(reopened.root().join("tombstones/r_d.json")).unwrap();
        reopened.rebuild_index().unwrap();
        assert_eq!(fs::read(reopened.root().join("index.json")).unwrap(), healthy, "byte-identical rebuild");
        // Retrying the original put is idempotent: no second row, no second entry.
        reopened.put_receipt(&receipt_with_id(&reopened, "r_a", "1.0Z")).unwrap();
        assert_eq!(audit_count(&reopened, "receipt.put", "r_a"), 1);
        assert_eq!(fs::read(reopened.root().join("index.json")).unwrap(), healthy);
    }

    #[cfg(unix)]
    #[test]
    fn a_failure_in_every_gap_of_the_three_writes_is_recovered_without_duplicates() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, store) = scratch();
        let root = store.root().to_path_buf();
        let mode = |rel: &str, m: u32| {
            fs::set_permissions(root.join(rel), fs::Permissions::from_mode(m)).unwrap();
        };

        // Gap 1: journal written, the Receipt file cannot land.
        mode("receipts", 0o555);
        let err = store.put_receipt(&receipt_with_id(&store, "r_g1", "1.0Z")).expect_err("io");
        mode("receipts", 0o755);
        assert_eq!(err.code, "store.io");
        assert_eq!(store.status().unwrap().pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(1));
        let reopened = Store::open(&root).unwrap();
        let status = reopened.status().unwrap();
        assert_eq!(status.pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(0));
        assert!(!root.join("receipts/r_g1.json").exists());
        assert_eq!(audit_count(&reopened, "receipt.put", "r_g1"), 0, "rolled back, never audited as put");
        assert!(reopened.list_receipts().unwrap().as_array().unwrap().is_empty());

        // Gap 2: Receipt written, the index cannot be rewritten.
        mode("", 0o555);
        let err = reopened.put_receipt(&receipt_with_id(&reopened, "r_g2", "2.0Z")).expect_err("io");
        mode("", 0o755);
        assert_eq!(err.code, "store.io");
        assert!(root.join("receipts/r_g2.json").exists());
        let reopened = Store::open(&root).unwrap();
        assert_eq!(reopened.status().unwrap().pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(0));
        let index = reopened.list_receipts().unwrap();
        assert_eq!(index.as_array().unwrap().len(), 1);
        assert_eq!(audit_count(&reopened, "receipt.put", "r_g2"), 1);

        // Gap 3: Receipt and index written, the audit append fails.
        reopened.audit("seed", "x", None).unwrap();
        mode("audit/events.jsonl", 0o444);
        let err = reopened.put_receipt(&receipt_with_id(&reopened, "r_g3", "3.0Z")).expect_err("io");
        mode("audit/events.jsonl", 0o644);
        assert_eq!(err.code, "store.io");
        assert_eq!(reopened.list_receipts().unwrap().as_array().unwrap().len(), 2);
        assert_eq!(audit_count(&reopened, "receipt.put", "r_g3"), 0);
        let reopened = Store::open(&root).unwrap();
        assert_eq!(reopened.status().unwrap().pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(0));
        assert_eq!(audit_count(&reopened, "receipt.put", "r_g3"), 1);
        assert_eq!(reopened.list_receipts().unwrap().as_array().unwrap().len(), 2, "no duplicate row");
        // A second recovery pass changes nothing.
        let reopened = Store::open(&root).unwrap();
        assert_eq!(audit_count(&reopened, "receipt.put", "r_g3"), 1);
        assert_eq!(reopened.audit_chain().unwrap().get("verified").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn the_advisory_lock_refuses_a_foreign_holder_and_is_reentrant_within_the_process() {
        let (_dir, store) = scratch();
        // Reentrant within this process.
        let outer = store.lock_mutation().unwrap();
        let inner = store.lock_mutation().unwrap();
        assert_eq!(store.lock_status().get("holder").and_then(Value::as_str), Some("this-process"));
        drop(inner);
        assert_eq!(store.lock_status().get("held"), Some(&Value::Bool(true)), "the inner guard owns nothing");
        drop(outer);
        assert_eq!(store.lock_status().get("held"), Some(&Value::Bool(false)));

        // A foreign holder: another open file description holding the OS
        // lock (what another process looks like to flock). Busy.
        let foreign = fs::OpenOptions::new().read(true).write(true).open(store.root().join("lock")).unwrap();
        foreign.try_lock().unwrap();
        let err = store.lock_mutation().expect_err("busy");
        assert_eq!(err.code, "store.busy");
        assert_eq!(store.lock_status().get("holder").and_then(Value::as_str), Some("other-process"));
        // The holder goes away: the OS releases the lock, nothing is stale.
        drop(foreign);
        let guard = store.lock_mutation().unwrap();
        assert_eq!(
            store.lock_status().get("pid").and_then(Value::as_i64),
            Some(i64::from(std::process::id()))
        );
        drop(guard);
    }

    #[test]
    fn cutting_entries_off_the_end_of_the_audit_log_is_detected() {
        let (_dir, store) = scratch();
        for n in 0..4 {
            store.audit("test.action", &format!("t-{n}"), None).unwrap();
        }
        let path = store.root().join("audit/events.jsonl");
        let original = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = original.lines().collect();
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("ok"));

        // The linked prefix still verifies on its own; the anchor says it is short.
        fs::write(&path, format!("{}\n{}\n{}\n", lines[0], lines[1], lines[2])).unwrap();
        let chain = store.audit_chain().unwrap();
        assert_eq!(chain.get("verified").and_then(Value::as_bool), Some(false));
        assert_eq!(chain.get("reason_code").and_then(Value::as_str), Some("audit.truncated"));

        // Emptying the log, or replacing it with an unlinked "legacy" line,
        // is truncation too — the anchor knows entries existed.
        fs::write(&path, "").unwrap();
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("audit.truncated"));
        fs::write(&path, "{\"action\":\"old\",\"at\":\"1\",\"detail\":null,\"target\":\"t\"}\n").unwrap();
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("audit.truncated"));

        // A forged anchor is caught by its own MAC.
        fs::write(&path, &original).unwrap();
        let head = store.root().join("audit/head.json");
        let forged = fs::read_to_string(&head).unwrap().replace("\"seq\":3", "\"seq\":2");
        fs::write(&head, forged).unwrap();
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("audit.head_mismatch"));

        // A missing anchor is reported, and repair re-anchors a chain that verifies.
        fs::remove_file(&head).unwrap();
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("audit.head_missing"));
        let report = store.repair().unwrap();
        assert_eq!(report.pointer(&["audit", "repaired"]).and_then(Value::as_bool), Some(true));
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("ok"));
        // Neither repair nor a further write launders a truncated log: the
        // store refuses to append past the anchored evidence.
        fs::write(&path, format!("{}\n{}\n", lines[0], lines[1])).unwrap();
        let err = store.repair().expect_err("cut log");
        assert_eq!(err.code, "store.audit_truncated");
        let err = store.audit("later", "z", None).expect_err("cut log");
        assert_eq!(err.code, "store.audit_truncated");
        assert_eq!(store.audit_chain().unwrap().get("reason_code").and_then(Value::as_str), Some("audit.truncated"));
    }

    #[test]
    fn a_torn_audit_tail_blocks_writes_and_is_mended_by_repair() {
        let (_dir, store) = scratch();
        store.audit("seed", "x", None).unwrap();
        let path = store.root().join("audit/events.jsonl");
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"action\":\"crashed-mid-write\"").unwrap();
        drop(file);

        let err = store.audit("next", "y", None).expect_err("torn");
        assert_eq!(err.code, "store.audit_torn");
        let status = store.status().unwrap();
        assert_eq!(status.pointer(&["audit", "status"]).and_then(Value::as_str), Some("torn"));
        let chain = store.audit_chain().unwrap();
        assert_eq!(chain.get("verified"), Some(&Value::Bool(false)));
        assert_eq!(chain.get("torn_tail"), Some(&Value::Bool(true)));

        let report = store.repair().unwrap();
        assert_eq!(report.pointer(&["audit", "repaired"]).and_then(Value::as_bool), Some(true));
        let aside: Vec<_> = fs::read_dir(store.root().join("audit"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("torn-"))
            .collect();
        assert_eq!(aside.len(), 1, "the torn bytes are kept, not deleted");
        assert_eq!(fs::read(aside[0].path()).unwrap(), b"{\"action\":\"crashed-mid-write\"");
        assert_eq!(store.status().unwrap().pointer(&["audit", "status"]).and_then(Value::as_str), Some("ok"));
        store.audit("next", "y", None).unwrap();
        let chain = store.audit_chain().unwrap();
        assert_eq!(chain.get("reason_code").and_then(Value::as_str), Some("ok"));
        assert_eq!(audit_count(&store, "audit.repaired", "audit"), 1);
        // A second repair changes nothing.
        assert_eq!(store.repair().unwrap().pointer(&["audit", "repaired"]).and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn a_missing_index_over_existing_receipts_is_never_read_as_empty() {
        let (_dir, store) = scratch();
        store.put_receipt(&receipt_with_id(&store, "r_a", "1.0Z")).unwrap();
        fs::remove_file(store.root().join("index.json")).unwrap();
        let reopened = Store::open(store.root()).unwrap();
        assert!(!reopened.root().join("index.json").exists(), "open does not mint an empty index over documents");
        let err = reopened.list_receipts().expect_err("missing");
        assert_eq!(err.code, "store.index_missing");
        assert_eq!(reopened.status().unwrap().pointer(&["index", "status"]).and_then(Value::as_str), Some("corrupt"));
        let report = reopened.repair().unwrap();
        assert_eq!(report.get("receipts").and_then(Value::as_i64), Some(1));
        assert_eq!(reopened.list_receipts().unwrap().as_array().unwrap().len(), 1);
        // A brand-new store still starts with an empty index.
        let fresh = tempdir::new("fresh");
        let new_store = Store::open(&fresh.path.join("s")).unwrap();
        assert!(new_store.list_receipts().unwrap().as_array().unwrap().is_empty());
    }

    #[test]
    fn opening_while_another_process_holds_the_lock_leaves_the_journal_alone() {
        let (_dir, store) = scratch();
        let root = store.root().to_path_buf();
        // A write that stopped after the Receipt landed: the journal says so.
        let receipt = receipt_with_id(&store, "r_live", "1.0Z");
        fs::write(root.join("receipts/r_live.json"), canonical_json(&receipt)).unwrap();
        fs::write(
            root.join("journal/receipt-put-r_live.json"),
            canonical_json(&object([
                ("schema", string(JOURNAL_SCHEMA)),
                ("op", string("receipt.put")),
                ("receipt_id", string("r_live")),
                ("stage", string("receipt-written")),
                ("at", string("1.0Z")),
            ])),
        )
        .unwrap();
        let foreign = fs::OpenOptions::new().read(true).write(true).open(root.join("lock")).unwrap();
        foreign.try_lock().unwrap();
        // The live writer's record is not a crash leftover: nothing is replayed.
        let opened = Store::open(&root).unwrap();
        assert!(root.join("journal/receipt-put-r_live.json").exists());
        assert_eq!(opened.status().unwrap().pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(1));
        assert_eq!(audit_count(&opened, "receipt.put", "r_live"), 0);
        drop(foreign);
        // Once the lock is free, opening finishes the write exactly once.
        let opened = Store::open(&root).unwrap();
        assert!(!root.join("journal/receipt-put-r_live.json").exists());
        assert_eq!(audit_count(&opened, "receipt.put", "r_live"), 1);
        assert_eq!(opened.list_receipts().unwrap().as_array().unwrap().len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_world_readable_continuity_key_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, store) = scratch();
        store.continuity_key().unwrap();
        let key = store.root().join("keys/continuity.key");
        assert_eq!(fs::metadata(&key).unwrap().permissions().mode() & 0o777, 0o600);
        fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
        let err = store.continuity_key().expect_err("wide");
        assert_eq!(err.code, "store.key_permissions");
        fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
        store.continuity_key().unwrap();
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
