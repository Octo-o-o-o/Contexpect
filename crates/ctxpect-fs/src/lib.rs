//! Path containment and refusal of unsafe filesystem entries.
//!
//! A passive scan reads whatever a repository happens to contain, including
//! things placed there to make a scanner misbehave. Every read this crate permits
//! is anchored to a declared root: a path that resolves outside it is an error,
//! not a skipped entry, because silently skipping turns an attack into a gap in
//! the inventory.
//!
//! # What the tests observe
//!
//! - A path resolving outside the root is refused, for `..`, absolute paths, a
//!   symlink to an outside target, a chain of symlinks, and a symlink used as an
//!   intermediate directory.
//! - Sockets and FIFOs are classified and never read.
//! - A path that passed containment earlier is resolved again on each read, so a
//!   name swapped for an outside link in between is refused, never followed.
//! - Files past [`MAX_READ_BYTES`] keep only that many bytes in memory, are marked
//!   truncated, and still report their real size. The digest covers the whole
//!   file, including bytes past the cap.
//!
//! # What they do not observe
//!
//! - Windows junctions and case-insensitive collisions. The declared OS lane is
//!   `macos-27-arm64`; the other lanes are not exercised here.
//! - Archive contents. Nothing in this crate opens an archive.
//! - Whether the root itself is a sensible place to scan. Choosing the root is the
//!   caller's decision.
//! - Total I/O, process RSS, or scan duration. [`MAX_READ_BYTES`] caps only the
//!   retained-content prefix; a single-link regular file is still streamed to EOF.
//! - **The post-read re-check.** No test here drives it. Reads go through the
//!   canonicalised path, so the swap a single-process test can stage is already
//!   refused earlier, by containment. Removing the re-check leaves every test in
//!   this crate green. It is kept as defence in depth against a swap of the
//!   resolved path itself, which this test suite cannot stage deterministically.
//! - Races generally. Concurrent modification during a scan is observed only as
//!   whatever the filesystem reports at each call.
//! - **Device files.** `EntryKind` has `Device` and `Other` variants, but no test
//!   creates one: `mknod` needs privileges these tests do not assume. Deleting the
//!   device branch leaves this crate green. Only regular files are ever read, so a
//!   misclassified device is still withheld — but the branch itself is unobserved.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Cap on retained file content kept in memory (1 MiB).
///
/// A regular file may still be streamed to EOF so the digest covers every byte.
/// This is not a total I/O, process RSS, or scan-duration limit.
pub const MAX_READ_BYTES: u64 = 1 << 20;

/// Why a path was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The path resolves outside the declared root.
    EscapesRoot { resolved: PathBuf },
    /// The entry is not a regular file or directory.
    NotRegular { kind: EntryKind },
    /// The path could not be resolved.
    Unresolvable { detail: String },
    /// The root itself is missing or not a directory.
    BadRoot { detail: String },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::EscapesRoot { resolved } => {
                write!(f, "path resolves outside the root: {}", resolved.display())
            }
            Refusal::NotRegular { kind } => write!(f, "entry is {kind}, not a regular file"),
            Refusal::Unresolvable { detail } => write!(f, "path cannot be resolved: {detail}"),
            Refusal::BadRoot { detail } => write!(f, "unusable scan root: {detail}"),
        }
    }
}

impl std::error::Error for Refusal {}

/// What kind of thing a path points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Socket,
    Fifo,
    Device,
    Other,
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            EntryKind::File => "a regular file",
            EntryKind::Directory => "a directory",
            EntryKind::Symlink => "a symlink",
            EntryKind::Socket => "a socket",
            EntryKind::Fifo => "a FIFO",
            EntryKind::Device => "a device file",
            EntryKind::Other => "an unrecognised entry",
        };
        f.write_str(name)
    }
}

impl EntryKind {
    /// Classify from metadata that did **not** follow symlinks.
    #[must_use]
    pub fn from_symlink_metadata(meta: &fs::Metadata) -> EntryKind {
        let file_type = meta.file_type();
        if file_type.is_symlink() {
            return EntryKind::Symlink;
        }
        if file_type.is_dir() {
            return EntryKind::Directory;
        }
        if file_type.is_file() {
            return EntryKind::File;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt as _;
            if file_type.is_socket() {
                return EntryKind::Socket;
            }
            if file_type.is_fifo() {
                return EntryKind::Fifo;
            }
            if file_type.is_block_device() || file_type.is_char_device() {
                return EntryKind::Device;
            }
        }
        EntryKind::Other
    }

    /// Whether this crate will read the entry's bytes.
    #[must_use]
    pub const fn is_readable_content(self) -> bool {
        matches!(self, EntryKind::File)
    }
}

/// A scan root, with its canonical form resolved once.
#[derive(Debug, Clone)]
pub struct Root {
    canonical: PathBuf,
}

impl Root {
    /// Resolve `path` into a scan root.
    ///
    /// The root is canonicalised here so later comparisons are against a real,
    /// symlink-free prefix rather than whatever the caller typed.
    pub fn new(path: impl AsRef<Path>) -> Result<Root, Refusal> {
        let path = path.as_ref();
        let canonical = fs::canonicalize(path).map_err(|error| Refusal::BadRoot {
            detail: format!("{}: {error}", path.display()),
        })?;
        if !canonical.is_dir() {
            return Err(Refusal::BadRoot {
                detail: format!("{} is not a directory", canonical.display()),
            });
        }
        Ok(Root { canonical })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.canonical
    }

    /// Resolve `candidate` and confirm it stays inside this root.
    ///
    /// `candidate` may be relative to the root or absolute. Resolution follows
    /// symlinks — including symlinks on intermediate components — so a link
    /// pointing outside is caught rather than followed.
    pub fn contain(&self, candidate: impl AsRef<Path>) -> Result<PathBuf, Refusal> {
        let candidate = candidate.as_ref();
        let joined = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.canonical.join(candidate)
        };

        let resolved = fs::canonicalize(&joined).map_err(|error| {
            // A missing path is unresolvable, not contained. The lexical form is
            // reported because it is the only description available once
            // canonicalisation has failed; note that it *resolves* `..`, so it can
            // name an outside location more plainly than the caller's input did.
            Refusal::Unresolvable {
                detail: format!("{}: {error}", lexical_normalize(&joined).display()),
            }
        })?;

        if resolved == self.canonical || resolved.starts_with(&self.canonical) {
            Ok(resolved)
        } else {
            Err(Refusal::EscapesRoot { resolved })
        }
    }

    /// Classify an entry without following the final symlink.
    pub fn classify(&self, candidate: impl AsRef<Path>) -> Result<EntryKind, Refusal> {
        let candidate = candidate.as_ref();
        let joined = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.canonical.join(candidate)
        };
        let meta = fs::symlink_metadata(&joined).map_err(|error| Refusal::Unresolvable {
            detail: format!("{}: {error}", lexical_normalize(&joined).display()),
        })?;
        Ok(EntryKind::from_symlink_metadata(&meta))
    }
}

/// The outcome of reading a contained file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContent {
    /// Content kept in memory, capped at [`MAX_READ_BYTES`].
    pub bytes: Vec<u8>,
    /// The file's real length, which exceeds `bytes.len()` when truncated.
    pub declared_len: u64,
    /// True when retained `bytes` were capped at [`MAX_READ_BYTES`].
    ///
    /// Truncation applies to the kept prefix, not to digest coverage:
    /// [`FileContent::whole_digest`] still hashes the entire file.
    pub truncated: bool,
    /// SHA-256 over the **whole** file, including bytes past the cap.
    ///
    /// Computed by streaming, so a change beyond the cap still moves it. Digesting
    /// only the retained prefix would leave the tail of a large file invisible.
    ///
    /// Empty when `link_count > 1`: the content is withheld in that case, so the
    /// file is never read and there is nothing to digest.
    pub whole_digest: String,
    /// How many hard links point at this file's inode.
    ///
    /// Greater than one means the same bytes are reachable under another name,
    /// possibly outside the scan root. Path containment cannot see such an alias,
    /// because a hard link is not a path indirection.
    pub link_count: u64,
    /// Filesystem identity, reported for diagnosis.
    ///
    /// Nothing in this crate branches on it. An earlier version used it to detect
    /// aliases of excluded files; that was removed once review showed the link
    /// count already covers every such case. It is kept because a reader chasing
    /// two names for one inode needs it, not because a decision depends on it.
    pub identity: Option<FileIdentity>,
}

/// A file's identity on its filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
}

impl FileIdentity {
    /// Read identity from metadata, where the platform exposes it.
    #[must_use]
    pub fn from_metadata(meta: &fs::Metadata) -> Option<FileIdentity> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            Some(FileIdentity {
                device: meta.dev(),
                inode: meta.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = meta;
            None
        }
    }
}

fn link_count_of(meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        meta.nlink()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        1
    }
}

/// Read a file that lies inside `root`.
///
/// Containment is established here, on every call. A path that passed [`Root::contain`]
/// earlier carries no standing credit: it is resolved again, so a name swapped for
/// an outside link between the two calls is refused rather than followed.
///
/// The re-check after reading is defence in depth for a swap of the resolved path
/// during the read itself. See the crate docs for what the tests do not show
/// about that.
pub fn read_contained(root: &Root, candidate: impl AsRef<Path>) -> Result<FileContent, Refusal> {
    let resolved = root.contain(candidate)?;

    let kind = EntryKind::from_symlink_metadata(&fs::symlink_metadata(&resolved).map_err(
        |error| Refusal::Unresolvable {
            detail: format!("{}: {error}", resolved.display()),
        },
    )?);
    if !kind.is_readable_content() {
        return Err(Refusal::NotRegular { kind });
    }

    let meta = fs::metadata(&resolved).map_err(|error| Refusal::Unresolvable {
        detail: format!("{}: {error}", resolved.display()),
    })?;
    let link_count = link_count_of(&meta);
    let identity = FileIdentity::from_metadata(&meta);

    if link_count > 1 {
        // A caller will withhold this content anyway, so reading it would be a
        // full pass over a file whose bytes get discarded. review-3 noted a
        // hard-linked large file was streamed in full before being dropped.
        return Ok(FileContent {
            bytes: Vec::new(),
            declared_len: meta.len(),
            truncated: false,
            whole_digest: String::new(),
            link_count,
            identity,
        });
    }

    // Stream the file so the digest covers every byte while memory stays capped.
    let (bytes, declared_len, whole_digest) = stream_digest(&resolved)?;

    // Re-resolve after reading: if the path was swapped for an outside target in
    // between, the second resolution no longer matches and the read is discarded.
    let after = fs::canonicalize(&resolved).map_err(|error| Refusal::Unresolvable {
        detail: format!("{}: {error}", resolved.display()),
    })?;
    if after != resolved || !after.starts_with(root.path()) {
        return Err(Refusal::EscapesRoot { resolved: after });
    }

    Ok(FileContent {
        truncated: declared_len > MAX_READ_BYTES,
        bytes,
        declared_len,
        whole_digest,
        link_count,
        identity,
    })
}

/// Read a file in chunks, keeping at most [`MAX_READ_BYTES`] while digesting all
/// of it. Returns the retained prefix, the real length, and the whole-file digest.
///
/// The 64 KiB read buffer is a fixed streaming buffer. It is not a second
/// retained-content cap and is not a total I/O budget.
fn stream_digest(path: &Path) -> Result<(Vec<u8>, u64, String), Refusal> {
    use std::io::Read as _;

    let mut file = fs::File::open(path).map_err(|error| Refusal::Unresolvable {
        detail: format!("{}: {error}", path.display()),
    })?;
    let cap = usize::try_from(MAX_READ_BYTES).unwrap_or(usize::MAX);
    let mut kept: Vec<u8> = Vec::new();
    let mut hasher = ctxpect_schema::Hasher::new();
    let mut total: u64 = 0;
    let mut chunk = vec![0u8; 64 * 1024];

    loop {
        let read = file.read(&mut chunk).map_err(|error| Refusal::Unresolvable {
            detail: format!("{}: {error}", path.display()),
        })?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        // The digest sees every byte; memory holds only the capped prefix.
        hasher.update(&chunk[..read]);
        if kept.len() < cap {
            let room = cap - kept.len();
            kept.extend_from_slice(&chunk[..read.min(room)]);
        }
    }

    Ok((kept, total, hasher.finish()))
}

/// Collapse `.` and `..` without touching the filesystem.
///
/// Used only to describe a path in an error; resolution itself always goes
/// through [`fs::canonicalize`], which is what makes symlinks visible.
///
/// This is a convenience for error text, not a containment check. It resolves
/// `..` lexically, so its output can point outside the root — never use it to
/// decide whether a path is contained.
#[must_use]
pub fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Write `bytes` to `target` through an exclusively created sibling temp
/// file, fsync it, and rename it into place.
///
/// The temp file is opened with `create_new`, so a pre-placed symlink or file
/// at the temp name is an error rather than a path the bytes follow; its name
/// carries the pid and a per-process counter, so two writers never share one.
/// A `target` that already exists must be a regular file: a symlink (dangling
/// or not), FIFO, socket or directory is refused instead of written through.
/// Callers that want to write *through* a symlink to a contained regular file
/// resolve it first (see [`Root::contain`]) and pass the resolved path.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    match fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{}: refusing to write through a symlink", target.display()),
            ));
        }
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{}: not a regular file", target.display()),
            ));
        }
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let name = target
        .file_name()
        .map(|item| item.to_string_lossy().into_owned())
        .unwrap_or_else(|| "target".to_string());
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!(
        ".{name}.{}.{serial}.ctxpect-tmp",
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(err) = written {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    if let Err(err) = fs::rename(&tmp, target) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    Ok(())
}

/// Confirm `dir` exists as a real directory: not a symlink, not a file.
///
/// Used before writing transaction records under a directory whose name an
/// attacker could have pre-created as a link elsewhere.
pub fn real_dir(dir: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: refusing a symlinked directory", dir.display()),
        ));
    }
    if !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: not a directory", dir.display()),
        ));
    }
    Ok(())
}

/// Whether an `io::Error` means "there is nothing here".
#[must_use]
pub fn is_missing(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_normalize_collapses_traversal() {
        assert_eq!(
            lexical_normalize(Path::new("/a/b/../c/./d")),
            PathBuf::from("/a/c/d")
        );
        assert_eq!(lexical_normalize(Path::new("a/../..")), PathBuf::from(""));
    }

    #[test]
    fn readable_content_is_only_regular_files() {
        assert!(EntryKind::File.is_readable_content());
        for other in [
            EntryKind::Directory,
            EntryKind::Symlink,
            EntryKind::Socket,
            EntryKind::Fifo,
            EntryKind::Device,
            EntryKind::Other,
        ] {
            assert!(!other.is_readable_content(), "{other} must not be read");
        }
    }

    #[test]
    fn refusals_describe_themselves() {
        let escape = Refusal::EscapesRoot {
            resolved: PathBuf::from("/etc/passwd"),
        };
        assert!(escape.to_string().contains("outside the root"));
        let not_regular = Refusal::NotRegular {
            kind: EntryKind::Fifo,
        };
        assert!(not_regular.to_string().contains("FIFO"));
    }
}
