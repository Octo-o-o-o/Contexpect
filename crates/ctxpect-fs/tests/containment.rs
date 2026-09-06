//! Containment against a real filesystem.
//!
//! These cases create actual symlinks, sockets and FIFOs, because the defect they
//! guard against is the operating system resolving a path differently from what a
//! purely in-memory model would predict.

use ctxpect_fs::{read_contained, EntryKind, Refusal, Root, MAX_READ_BYTES};
use std::fs;
use std::path::{Path, PathBuf};

/// A scratch directory that removes itself.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Scratch {
        // Kept short: a Unix socket path must fit in SUN_LEN (~104 bytes), and one
        // of these scratch directories holds one.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let unique = format!("cx-{label}-{}-{}", std::process::id(), nanos % 1_000_000);
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("create scratch");
        Scratch {
            path: fs::canonicalize(&path).expect("canonicalize scratch"),
        }
    }

    fn join(&self, rel: &str) -> PathBuf {
        self.path.join(rel)
    }

    fn write(&self, rel: &str, contents: &str) -> PathBuf {
        let target = self.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(&target, contents).expect("write file");
        target
    }

    fn dir(&self, rel: &str) -> PathBuf {
        let target = self.join(rel);
        fs::create_dir_all(&target).expect("create dir");
        target
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

/// A1: `..` cannot climb out of the root.
#[test]
fn parent_traversal_is_refused() {
    let scratch = Scratch::new("parent");
    let root_dir = scratch.dir("root");
    scratch.write("outside.txt", "secret");
    let root = Root::new(&root_dir).expect("root");

    let error = root.contain("../outside.txt").expect_err("must refuse");
    assert!(
        matches!(error, Refusal::EscapesRoot { .. }),
        "expected an escape refusal, got {error:?}"
    );
}

/// A1: an absolute path outside the root is refused even though it exists.
#[test]
fn absolute_outside_path_is_refused() {
    let scratch = Scratch::new("absolute");
    let root_dir = scratch.dir("root");
    let outside = scratch.write("outside.txt", "secret");
    let root = Root::new(&root_dir).expect("root");

    let error = root.contain(&outside).expect_err("must refuse");
    assert!(matches!(error, Refusal::EscapesRoot { .. }), "{error:?}");
}

/// A1: a symlink whose target is outside the root is refused, not followed.
#[cfg(unix)]
#[test]
fn symlink_pointing_outside_is_refused() {
    let scratch = Scratch::new("symlink");
    let root_dir = scratch.dir("root");
    let outside = scratch.write("outside.txt", "secret");
    symlink(&outside, &root_dir.join("link.txt"));
    let root = Root::new(&root_dir).expect("root");

    let error = root.contain("link.txt").expect_err("must refuse");
    assert!(matches!(error, Refusal::EscapesRoot { .. }), "{error:?}");

    let read_error = read_contained(&root, "link.txt").expect_err("must refuse the read too");
    assert!(matches!(read_error, Refusal::EscapesRoot { .. }));
}

/// A1: a chain of symlinks ending outside is refused.
#[cfg(unix)]
#[test]
fn symlink_chain_to_outside_is_refused() {
    let scratch = Scratch::new("chain");
    let root_dir = scratch.dir("root");
    let outside = scratch.write("outside.txt", "secret");
    symlink(&outside, &scratch.join("hop.txt"));
    symlink(&scratch.join("hop.txt"), &root_dir.join("link.txt"));
    let root = Root::new(&root_dir).expect("root");

    let error = root.contain("link.txt").expect_err("must refuse");
    assert!(matches!(error, Refusal::EscapesRoot { .. }), "{error:?}");
}

/// A1: a symlink as an *intermediate* component is refused, not just a final one.
#[cfg(unix)]
#[test]
fn symlink_as_intermediate_directory_is_refused() {
    let scratch = Scratch::new("intermediate");
    let root_dir = scratch.dir("root");
    let outside_dir = scratch.dir("outside");
    fs::write(outside_dir.join("file.txt"), "secret").expect("write");
    symlink(&outside_dir, &root_dir.join("bridge"));
    let root = Root::new(&root_dir).expect("root");

    let error = root.contain("bridge/file.txt").expect_err("must refuse");
    assert!(matches!(error, Refusal::EscapesRoot { .. }), "{error:?}");
}

/// A1 control: a symlink that stays inside the root is allowed.
#[cfg(unix)]
#[test]
fn symlink_staying_inside_is_allowed() {
    let scratch = Scratch::new("inside-link");
    let root_dir = scratch.dir("root");
    let target = scratch.write("root/real.txt", "hello");
    symlink(&target, &root_dir.join("alias.txt"));
    let root = Root::new(&root_dir).expect("root");

    let resolved = root.contain("alias.txt").expect("inside links are fine");
    assert!(resolved.starts_with(root.path()));
    let content = read_contained(&root, "alias.txt").expect("read");
    assert_eq!(content.bytes, b"hello");
    assert!(!content.truncated);
}

/// A2: a socket is classified, never read.
#[cfg(unix)]
#[test]
fn socket_is_classified_and_not_read() {
    let scratch = Scratch::new("socket");
    let root_dir = scratch.dir("root");
    let socket_path = root_dir.join("live.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&socket_path).expect("bind socket");
    let root = Root::new(&root_dir).expect("root");

    assert_eq!(root.classify("live.sock").expect("classify"), EntryKind::Socket);
    let error = read_contained(&root, "live.sock").expect_err("must refuse");
    assert!(
        matches!(
            error,
            Refusal::NotRegular {
                kind: EntryKind::Socket
            }
        ),
        "{error:?}"
    );
}

/// A2: a FIFO is classified without the read blocking on a writer.
#[cfg(unix)]
#[test]
fn fifo_is_classified_and_not_read() {
    let scratch = Scratch::new("fifo");
    let root_dir = scratch.dir("root");
    let fifo_path = root_dir.join("pipe");

    // mkfifo has no std wrapper; the test may use a tool the product may not.
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !made {
        eprintln!("skipping: mkfifo unavailable");
        return;
    }

    let root = Root::new(&root_dir).expect("root");
    assert_eq!(root.classify("pipe").expect("classify"), EntryKind::Fifo);
    let error = read_contained(&root, "pipe").expect_err("must refuse before opening");
    assert!(
        matches!(
            error,
            Refusal::NotRegular {
                kind: EntryKind::Fifo
            }
        ),
        "{error:?}"
    );
}

/// A2: classification does not follow the final symlink, so a link to a device is
/// seen as a link rather than silently treated as its target.
#[cfg(unix)]
#[test]
fn classification_does_not_follow_the_final_symlink() {
    let scratch = Scratch::new("classify-link");
    let root_dir = scratch.dir("root");
    symlink(Path::new("/dev/null"), &root_dir.join("null-link"));
    let root = Root::new(&root_dir).expect("root");

    assert_eq!(
        root.classify("null-link").expect("classify"),
        EntryKind::Symlink
    );
    let error = read_contained(&root, "null-link").expect_err("must refuse");
    assert!(matches!(error, Refusal::EscapesRoot { .. }), "{error:?}");
}

/// A3: swapping a checked path for an outside symlink before the read must not
/// yield outside content.
///
/// What makes this hold is that the read targets the canonicalised path, so the
/// swap is refused by containment on the way in. The post-read re-check is not
/// what this test exercises — see the crate docs.
#[cfg(unix)]
#[test]
fn swap_to_outside_symlink_before_read_is_refused() {
    let scratch = Scratch::new("toctou");
    let root_dir = scratch.dir("root");
    scratch.write("root/target.txt", "inside");
    let outside = scratch.write("outside.txt", "SECRET-OUTSIDE");
    let root = Root::new(&root_dir).expect("root");

    // The path passes containment while it is still an ordinary file.
    let resolved = root.contain("target.txt").expect("initially contained");
    assert!(resolved.starts_with(root.path()));

    // The attacker swaps it for a link pointing out of the root.
    fs::remove_file(root_dir.join("target.txt")).expect("remove");
    symlink(&outside, &root_dir.join("target.txt"));

    let outcome = read_contained(&root, "target.txt");
    match outcome {
        Err(Refusal::EscapesRoot { .. }) => {}
        Err(other) => panic!("expected an escape refusal, got {other:?}"),
        Ok(content) => panic!(
            "read returned outside content: {:?}",
            String::from_utf8_lossy(&content.bytes)
        ),
    }
}

/// A3, the other half: containment is re-established on every read.
///
/// A path that passed `contain` earlier gets no standing credit. Handing the
/// previously-resolved path back after the name was pointed outside is still
/// refused, because the read resolves it again rather than trusting the earlier
/// result.
#[cfg(unix)]
#[test]
fn every_read_re_resolves_rather_than_trusting_an_earlier_check() {
    let scratch = Scratch::new("recheck");
    let root_dir = scratch.dir("root");
    let inside = scratch.write("root/target.txt", "INSIDE");
    let outside = scratch.write("outside.txt", "SECRET-OUTSIDE");
    let root = Root::new(&root_dir).expect("root");

    let resolved = root.contain("target.txt").expect("contained");
    let content = read_contained(&root, &resolved).expect("reads while it is still a file");
    assert_eq!(content.bytes, b"INSIDE");

    // Point the name outside, leaving the original bytes elsewhere.
    fs::rename(&inside, root_dir.join("moved.txt")).expect("rename");
    symlink(&outside, &root_dir.join("target.txt"));

    // The same previously-resolved path is now refused: no cached trust.
    let error = read_contained(&root, &resolved).expect_err("must re-resolve and refuse");
    match error {
        Refusal::EscapesRoot { resolved: target } => {
            assert!(
                target.ends_with("outside.txt"),
                "the refusal should name what the path now reaches, got {}",
                target.display()
            );
        }
        other => panic!("expected an escape refusal, got {other:?}"),
    }
}

/// A8: retained content is truncated at MAX_READ_BYTES; declared length is the
/// real size. Truncation does not stop whole-file digest coverage.
#[test]
fn oversized_files_are_reported_as_truncated() {
    let scratch = Scratch::new("large");
    let root_dir = scratch.dir("root");
    let size = usize::try_from(MAX_READ_BYTES).expect("fits") + 4096;
    fs::write(root_dir.join("big.bin"), vec![b'x'; size]).expect("write");
    let root = Root::new(&root_dir).expect("root");

    let content = read_contained(&root, "big.bin").expect("read");
    assert!(content.truncated, "an oversized file must be marked");
    assert_eq!(content.declared_len, size as u64);
    assert_eq!(content.bytes.len() as u64, MAX_READ_BYTES);
    assert_eq!(
        content.whole_digest.len(),
        64,
        "truncation is of retained content, not digest coverage"
    );
}

/// A8 control: a file at exactly the limit is not marked truncated.
#[test]
fn a_file_at_the_limit_is_not_truncated() {
    let scratch = Scratch::new("limit");
    let root_dir = scratch.dir("root");
    let size = usize::try_from(MAX_READ_BYTES).expect("fits");
    fs::write(root_dir.join("exact.bin"), vec![b'y'; size]).expect("write");
    let root = Root::new(&root_dir).expect("root");

    let content = read_contained(&root, "exact.bin").expect("read");
    assert!(!content.truncated);
    assert_eq!(content.bytes.len() as u64, MAX_READ_BYTES);
}

/// A missing path is unresolvable, and the message must not disclose an outside
/// target the caller could not otherwise see.
#[test]
fn missing_paths_are_unresolvable() {
    let scratch = Scratch::new("missing");
    let root_dir = scratch.dir("root");
    let root = Root::new(&root_dir).expect("root");

    let error = root.contain("nope.txt").expect_err("must refuse");
    assert!(matches!(error, Refusal::Unresolvable { .. }), "{error:?}");
}

/// The root itself must be a real directory.
#[test]
fn a_file_cannot_be_a_root() {
    let scratch = Scratch::new("bad-root");
    let file = scratch.write("plain.txt", "x");
    let error = Root::new(&file).expect_err("a file is not a root");
    assert!(matches!(error, Refusal::BadRoot { .. }), "{error:?}");
}

#[test]
fn a_missing_root_is_refused() {
    let scratch = Scratch::new("no-root");
    let error = Root::new(scratch.join("absent")).expect_err("must refuse");
    assert!(matches!(error, Refusal::BadRoot { .. }), "{error:?}");
}
