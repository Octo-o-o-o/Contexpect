//! Passive asset discovery.
//!
//! "Passive" is the load-bearing word: this crate reads declarations and inert
//! content. It never starts an MCP server, runs a hook, executes a custom command
//! or loads plugin code. A repository being scanned may contain programs written
//! to run during a scan, and none of them get the chance.
//!
//! Excluded paths still appear in the inventory, marked with why they were
//! excluded. Omitting them entirely would make an exclusion indistinguishable
//! from an absence.
//!
//! # What the tests observe
//!
//! - Scanning a tree containing executable hooks leaves no marker file, and the
//!   crate's own source contains no process-creation API.
//! - `.git`, dependency directories, build output, `.env` and private keys are
//!   listed without content or content digest.
//! - Two scans of one tree produce a byte-identical digest; mtime alone does not
//!   move it; an equal-length content change does, as does a change past the
//!   retained-content cap.
//! - Entries come back sorted, and the digest is order-sensitive by construction.
//! - Every field in the digest body moves the digest when changed, one at a time.
//! - Symlinks and non-regular entries are recorded as what they are, not as
//!   ordinary content. Oversized files still carry a whole-file content digest;
//!   `truncated` means retained content was capped, not that digest coverage
//!   stopped.
//!
//! # What they do not observe
//!
//! - Whether the exclusion list is the right one. It encodes the defaults the
//!   requirements name; judging that list is a product decision.
//! - Anything about identity across devices. This crate assigns a per-scan path
//!   identity only; stable cross-device asset identity is a later work package.
//! - Archive contents, remote roots, or containers. None are opened here.
//! - Whether a file's *meaning* was understood. Discovery records bytes and
//!   classification, not interpretation.
//! - **That the sorting works.** Order-independence rests on sorting each
//!   directory's children and the final entry list, but on this platform
//!   `read_dir` already returns names in order: removing either sort — or both —
//!   leaves every test green. review-3 established this by measurement. The sorts
//!   are a defence for filesystems that promise no order, and no test here
//!   exercises them.
//! - **Symlink resolution semantics beyond the recorded target.** The tests cover
//!   what a symlink entry contains; they do not cover what the operating system
//!   would do if the link were followed, because this crate never follows one.
//! - Whether withholding a multiply-linked file is the right trade. It refuses
//!   content whenever a second name exists, including legitimate hard links. That
//!   is a deliberate choice for safety, not a validated product decision.
//! - Total I/O, process RSS, or scan duration. The 1 MiB figure is the retained
//!   content prefix, not a stop-after-N-bytes I/O budget.

use ctxpect_fs::{read_contained, EntryKind, FileIdentity, Refusal, Root};
use ctxpect_schema::{canonical_json, sha256_text, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory names never descended into.
pub const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    ".venv",
    "__pycache__",
];

/// Exact file names whose content is never read.
pub const EXCLUDED_FILES: &[&str] = &[
    ".env",
    ".netrc",
    "credentials",
    ".git-credentials",
    ".npmrc",
    ".pypirc",
    ".dockercfg",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "identity",
];

/// Name prefixes whose content is never read, so `.env.production` is covered
/// along with `.env`.
pub const EXCLUDED_PREFIXES: &[&str] = &[".env.", "id_rsa.", "id_ed25519."];

/// Extensions whose content is never read, whatever the base name.
///
/// review-1 found `server.pem` and `private.key` read in full because matching was
/// on exact base names only.
pub const EXCLUDED_EXTENSIONS: &[&str] = &[
    "pem", "key", "p12", "pfx", "jks", "keystore", "asc", "gpg", "kdbx",
];

/// Whether a file's content must be withheld, by name alone.
///
/// Matching is case-insensitive: the declared OS lane is macOS, whose default
/// filesystem is case-insensitive, so `.ENV` reaches the same bytes as `.env`.
#[must_use]
pub fn is_excluded_file(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    if EXCLUDED_FILES.contains(&lowered.as_str()) {
        return true;
    }
    if EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| lowered.starts_with(prefix))
    {
        return true;
    }
    match lowered.rsplit_once('.') {
        Some((_, extension)) => EXCLUDED_EXTENSIONS.contains(&extension),
        None => false,
    }
}

/// Whether a directory must not be descended into. Case-insensitive, as above.
#[must_use]
pub fn is_excluded_dir(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    EXCLUDED_DIRS.contains(&lowered.as_str())
}

/// Why an entry carries no content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Withheld {
    /// A directory on the exclusion list.
    ExcludedDirectory,
    /// A file on the exclusion list: secrets and credentials.
    ExcludedFile,
    /// A regular file with more than one hard link.
    ///
    /// The same bytes are reachable under another name — possibly outside the
    /// root, possibly as an alias of an excluded file. A hard link is not a path
    /// indirection, so containment cannot say which; the content is not collected.
    ///
    /// This single rule covers both cases: sharing an inode with an excluded file
    /// necessarily means a link count above one.
    MultiplyLinked,
    /// Not a regular file.
    NotRegular(EntryKind),
    /// Refused by containment.
    Refused(String),
}

impl Withheld {
    /// A short, stable tag used in the digest and in reports.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Withheld::ExcludedDirectory => "excluded-directory",
            Withheld::ExcludedFile => "excluded-file",
            Withheld::MultiplyLinked => "multiply-linked",
            Withheld::NotRegular(_) => "not-regular",
            Withheld::Refused(_) => "refused",
        }
    }
}

/// One discovered entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Path relative to the scan root, with `/` separators.
    pub path: String,
    pub kind: EntryKind,
    /// SHA-256 of the content, when content was read.
    pub content_digest: Option<String>,
    /// Real length in bytes, when known.
    pub len: Option<u64>,
    /// Set when retained content was capped rather than kept whole.
    ///
    /// Truncation applies to the kept prefix, not to digest coverage. A present
    /// [`Entry::content_digest`] still hashes the entire file.
    pub truncated: bool,
    /// Set when no content was read, with the reason.
    pub withheld: Option<Withheld>,
    /// For a symlink, its target as written, whether or not it resolves.
    pub link_target: Option<String>,
    /// Hard links pointing at this file's inode; > 1 means an alias exists.
    pub link_count: Option<u64>,
    /// Filesystem identity, reported for diagnosis.
    ///
    /// No collection decision reads it; `link_count` covers the alias cases. It is
    /// here so a reader can tell which two paths share an inode.
    ///
    /// Deliberately absent from the digest, and a test pins that: inode numbers are
    /// not stable across machines or even re-creation, so including them would make
    /// identical content digest differently.
    pub identity: Option<FileIdentity>,
}

impl Entry {
    /// The fields that define this entry for digest purposes.
    ///
    /// Deliberately excludes mtime, inode and traversal position: a scan of the
    /// same bytes must digest the same regardless of when or how it walked.
    fn digest_value(&self) -> Value {
        let mut map = BTreeMap::new();
        map.insert("path".to_string(), Value::Str(self.path.clone()));
        map.insert(
            "kind".to_string(),
            Value::Str(format!("{:?}", self.kind).to_lowercase()),
        );
        map.insert(
            "content_digest".to_string(),
            match &self.content_digest {
                Some(digest) => Value::Str(digest.clone()),
                None => Value::Null,
            },
        );
        map.insert(
            "len".to_string(),
            match self.len {
                Some(len) => Value::Int(i64::try_from(len).unwrap_or(i64::MAX)),
                None => Value::Null,
            },
        );
        map.insert("truncated".to_string(), Value::Bool(self.truncated));
        map.insert(
            "withheld".to_string(),
            match &self.withheld {
                Some(reason) => Value::Str(reason.tag().to_string()),
                None => Value::Null,
            },
        );
        map.insert(
            "link_target".to_string(),
            match &self.link_target {
                Some(target) => Value::Str(target.clone()),
                None => Value::Null,
            },
        );
        map.insert(
            "link_count".to_string(),
            match self.link_count {
                Some(count) => Value::Int(i64::try_from(count).unwrap_or(i64::MAX)),
                None => Value::Null,
            },
        );
        Value::Object(map)
    }
}

/// The result of one scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    /// Entries sorted by path, so the digest does not depend on walk order.
    pub entries: Vec<Entry>,
    /// True when the walk stopped at the configured file limit.
    ///
    /// A truncated inventory is not a smaller inventory — it is an unknown
    /// one. Callers must not derive a manifest digest from it as though the
    /// scan had completed.
    pub truncated: bool,
    /// The limit that applied, when one did.
    pub file_limit: Option<usize>,
}

impl Inventory {
    /// A digest over the entries, stable across scans of unchanged content.
    #[must_use]
    pub fn digest(&self) -> String {
        let items: Vec<Value> = self.entries.iter().map(Entry::digest_value).collect();
        sha256_text(&canonical_json(&Value::Array(items)))
    }

    /// Entries whose content was read.
    #[must_use]
    pub fn with_content(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.content_digest.is_some())
            .collect()
    }

    /// Look up one entry by its relative path.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Paths that share a content digest, as duplicate groups.
    ///
    /// Reported, never acted upon: identical bytes are not proof of one asset.
    #[must_use]
    pub fn duplicate_groups(&self) -> Vec<Vec<&str>> {
        let mut by_digest: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for entry in &self.entries {
            if let Some(digest) = &entry.content_digest {
                by_digest
                    .entry(digest.as_str())
                    .or_default()
                    .push(entry.path.as_str());
            }
        }
        by_digest
            .into_values()
            .filter(|paths| paths.len() > 1)
            .collect()
    }
}

/// Walk `root` and record what is there, without executing anything.
pub fn scan(root: &Root) -> Result<Inventory, Refusal> {
    scan_with_limit(root, None)
}

/// Scan, stopping after `file_limit` entries when one is given.
///
/// Stopping is recorded rather than hidden: `Inventory::truncated` says the
/// walk did not finish, so a caller cannot mistake a partial listing for a
/// complete one. The limit counts entries visited, which is what the
/// `resource_limits.scan_files` setting is about.
pub fn scan_with_limit(root: &Root, file_limit: Option<usize>) -> Result<Inventory, Refusal> {
    scan_inventory(root, file_limit, true)
}

/// List names and lengths without reading file contents. This is not a content
/// fingerprint: Doctor uses it to select bounded reads, not as a Receipt digest.
/// Exclusions and symlink handling are identical to the content collector.
pub fn scan_metadata(root: &Root) -> Result<Inventory, Refusal> {
    scan_inventory(root, None, false)
}

fn scan_inventory(root: &Root, file_limit: Option<usize>, read_content: bool) -> Result<Inventory, Refusal> {
    let mut entries = Vec::new();
    let mut truncated = false;
    walk(root, root.path(), &mut entries, file_limit, &mut truncated, read_content)?;

    // Sorting by path is what makes the digest independent of directory order.
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Inventory {
        entries,
        truncated,
        file_limit,
    })
}

fn relative(root: &Root, path: &Path) -> String {
    path.strip_prefix(root.path())
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn walk(
    root: &Root,
    dir: &Path,
    out: &mut Vec<Entry>,
    file_limit: Option<usize>,
    truncated: &mut bool,
    read_content: bool,
) -> Result<(), Refusal> {
    if file_limit.is_some_and(|limit| out.len() >= limit) {
        *truncated = true;
        return Ok(());
    }
    let listing = fs::read_dir(dir).map_err(|error| Refusal::Unresolvable {
        detail: format!("{}: {error}", dir.display()),
    })?;

    let mut children: Vec<PathBuf> = Vec::new();
    for item in listing {
        let item = item.map_err(|error| Refusal::Unresolvable {
            detail: format!("{}: {error}", dir.display()),
        })?;
        children.push(item.path());
    }
    // Sorted so a scan is reproducible even before the final sort.
    children.sort();

    for child in children {
        if file_limit.is_some_and(|limit| out.len() >= limit) {
            *truncated = true;
            return Ok(());
        }
        let rel = relative(root, &child);
        let name = child
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        let meta = match fs::symlink_metadata(&child) {
            Ok(meta) => meta,
            Err(error) => {
                // A vanished entry is recorded, not fatal: a scan of a live tree
                // will meet these, and dropping them silently would hide churn.
                out.push(Entry {
                    path: rel,
                    kind: EntryKind::Other,
                    content_digest: None,
                    len: None,
                    truncated: false,
                    withheld: Some(Withheld::Refused(error.to_string())),
                    link_target: None,
                    link_count: None,
                    identity: None,
                });
                continue;
            }
        };
        let kind = EntryKind::from_symlink_metadata(&meta);

        if kind == EntryKind::Directory {
            if is_excluded_dir(&name) {
                out.push(Entry {
                    path: rel,
                    kind,
                    content_digest: None,
                    len: None,
                    truncated: false,
                    withheld: Some(Withheld::ExcludedDirectory),
                    link_target: None,
                    link_count: None,
                    identity: None,
                });
                continue;
            }
            out.push(Entry {
                path: rel,
                kind,
                content_digest: None,
                len: None,
                truncated: false,
                withheld: None,
                link_target: None,
                link_count: None,
                identity: None,
            });
            walk(root, &child, out, file_limit, truncated, read_content)?;
            if *truncated {
                return Ok(());
            }
            continue;
        }

        if kind == EntryKind::Symlink {
            // Recorded as a link, never followed here. Its target is reported as
            // written so a reader can see where it points without the scanner
            // having gone there.
            let target = fs::read_link(&child)
                .map(|target| target.to_string_lossy().into_owned())
                .ok();
            out.push(Entry {
                path: rel,
                kind,
                content_digest: None,
                len: None,
                truncated: false,
                withheld: Some(Withheld::NotRegular(kind)),
                link_target: target,
                link_count: None,
                identity: None,
            });
            continue;
        }

        if kind != EntryKind::File {
            out.push(Entry {
                path: rel,
                kind,
                content_digest: None,
                len: None,
                truncated: false,
                withheld: Some(Withheld::NotRegular(kind)),
                link_target: None,
                link_count: None,
                identity: None,
            });
            continue;
        }

        if is_excluded_file(&name) {
            out.push(Entry {
                path: rel,
                kind,
                content_digest: None,
                len: None,
                truncated: false,
                withheld: Some(Withheld::ExcludedFile),
                link_target: None,
                link_count: None,
                identity: None,
            });
            continue;
        }

        if !read_content {
            out.push(Entry {
                path: rel,
                kind,
                content_digest: None,
                len: Some(meta.len()),
                truncated: false,
                withheld: None,
                link_target: None,
                link_count: None,
                identity: FileIdentity::from_metadata(&meta),
            });
            continue;
        }

        match read_contained(root, &child) {
            Ok(content) => {
                // More than one link means these bytes are reachable elsewhere,
                // possibly outside the root, and containment cannot say where.
                let multiply_linked = content.link_count > 1;
                out.push(Entry {
                    path: rel,
                    kind,
                    content_digest: (!multiply_linked).then(|| content.whole_digest.clone()),
                    len: (!multiply_linked).then_some(content.declared_len),
                    truncated: !multiply_linked && content.truncated,
                    withheld: multiply_linked.then_some(Withheld::MultiplyLinked),
                    link_target: None,
                    link_count: Some(content.link_count),
                    identity: content.identity,
                });
            }
            Err(refusal) => out.push(Entry {
                path: rel,
                kind,
                content_digest: None,
                len: None,
                truncated: false,
                withheld: Some(Withheld::Refused(refusal.to_string())),
                link_target: None,
                link_count: None,
                identity: None,
            }),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withheld_tags_are_stable_and_distinct() {
        let tags = [
            Withheld::ExcludedDirectory.tag(),
            Withheld::ExcludedFile.tag(),
            Withheld::NotRegular(EntryKind::Socket).tag(),
            Withheld::Refused("x".into()).tag(),
        ];
        let mut sorted = tags.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), tags.len(), "tags must be distinct");
    }

    #[test]
    fn the_refusal_detail_does_not_reach_the_digest() {
        // Two scans that refused for different reasons must still digest the same
        // if everything else matches: the detail can name an outside path.
        let base = Entry {
            path: "a".into(),
            kind: EntryKind::File,
            content_digest: None,
            len: None,
            truncated: false,
            withheld: Some(Withheld::Refused("one detail".into())),
            link_target: None,
            link_count: None,
            identity: None,
        };
        let other = Entry {
            withheld: Some(Withheld::Refused("a completely different detail".into())),
            ..base.clone()
        };
        assert_eq!(
            canonical_json(&base.digest_value()),
            canonical_json(&other.digest_value())
        );
    }

    #[test]
    fn exclusion_lists_have_no_duplicates() {
        for list in [EXCLUDED_DIRS, EXCLUDED_FILES] {
            let mut sorted = list.to_vec();
            let total = sorted.len();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), total);
        }
    }
}
