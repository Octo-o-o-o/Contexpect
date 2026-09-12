//! Passivity, exclusion and digest stability, against a real filesystem.

use ctxpect_collect::{scan, scan_metadata, scan_with_limit, Entry, Inventory, Withheld};
use ctxpect_fs::{EntryKind, Root};
use std::fs;
use std::path::PathBuf;

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
            "cx-col-{label}-{}-{}",
            std::process::id(),
            nanos % 1_000_000
        ));
        fs::create_dir_all(&path).expect("create scratch");
        Scratch {
            path: fs::canonicalize(&path).expect("canonicalize"),
        }
    }

    fn write(&self, rel: &str, contents: &str) -> PathBuf {
        let target = self.path.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(&target, contents).expect("write");
        target
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

#[test]
fn metadata_scan_keeps_exclusions_and_never_claims_content_digests() {
    let scratch = Scratch::new("metadata");
    scratch.write("AGENTS.md", "rules\n");
    scratch.write(".env", "do not read\n");
    scratch.write("target/hidden", "excluded\n");
    let large = fs::File::create(scratch.path.join("large.bin")).unwrap();
    large.set_len(1024 * 1024 * 1024).unwrap();
    let inventory = scan_metadata(&scratch.root()).unwrap();
    assert!(inventory.with_content().is_empty());
    assert_eq!(inventory.get("large.bin").unwrap().len, Some(1024 * 1024 * 1024));
    assert_eq!(inventory.get(".env").unwrap().withheld, Some(Withheld::ExcludedFile));
    assert!(inventory.get("target/hidden").is_none());
    assert_eq!(inventory.get("target").unwrap().withheld, Some(Withheld::ExcludedDirectory));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("../outside", scratch.path.join("escape")).unwrap();
        let inventory = scan_metadata(&scratch.root()).unwrap();
        assert_eq!(inventory.get("escape").unwrap().kind, EntryKind::Symlink);
        assert_eq!(inventory.get("escape").unwrap().link_target.as_deref(), Some("../outside"));
    }
}

/// A4: a tree full of programs designed to run during a scan leaves no trace.
///
/// Each planted program writes a marker if it ever executes. The markers must not
/// exist after the scan.
#[test]
fn scanning_never_executes_what_it_finds() {
    let scratch = Scratch::new("passive");
    let marker_dir = scratch.path.join("markers");
    fs::create_dir_all(&marker_dir).expect("markers");

    // A git-style hook, a shell script, an executable "custom command", and an
    // MCP server declaration pointing at a runnable binary.
    let planted = [
        (".githooks/pre-commit", "hook"),
        ("scripts/setup.sh", "script"),
        (".claude/commands/deploy.sh", "command"),
        ("hooks/on-scan", "onscan"),
    ];
    for (rel, marker) in planted {
        let path = scratch.write(
            rel,
            &format!(
                "#!/bin/sh\ntouch '{}'\n",
                marker_dir.join(marker).display()
            ),
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    }
    scratch.write(
        ".mcp.json",
        &format!(
            r#"{{"mcpServers":{{"evil":{{"command":"/bin/sh","args":["-c","touch {}"]}}}}}}"#,
            marker_dir.join("mcp").display()
        ),
    );

    let inventory = scan(&scratch.root()).expect("scan");

    let leftovers: Vec<String> = fs::read_dir(&marker_dir)
        .expect("read markers")
        .filter_map(|item| item.ok())
        .map(|item| item.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        leftovers.is_empty(),
        "the scan executed something: {leftovers:?}"
    );

    // The declarations were still discovered — passivity is not blindness.
    assert!(inventory.get(".mcp.json").is_some());
    assert!(inventory.get("scripts/setup.sh").is_some());
}

/// A4: no product crate on the scan path creates processes.
///
/// The behavioural test above only proves the programs it planted did not run.
/// review-1 showed the earlier version of this test covering one crate, so
/// injecting `std::process::Command` into `ctxpect-fs` left every test green.
/// It now walks every product source file a scan can reach.
#[test]
fn no_product_crate_on_the_scan_path_creates_processes() {
    const FORBIDDEN: &[&str] = &[
        "process::Command",
        "Command::new",
        "process::abort",
        "libc::system",
        "libc::exec",
    ];

    let crates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .to_path_buf();

    let mut checked = 0usize;
    let mut crates_seen = 0usize;
    // Every product crate in the workspace, discovered rather than listed:
    // review-3 noted a hard-coded list would let a fifth crate slip past.
    // Test files are excluded — this very file spawns `mkfifo`, and tests are not
    // the scan path.
    let mut crate_dirs: Vec<PathBuf> = fs::read_dir(&crates_dir)
        .expect("read crates dir")
        .flatten()
        .map(|item| item.path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    crate_dirs.sort();

    for crate_dir in crate_dirs {
        crates_seen += 1;
        let src = crate_dir.join("src");
        let mut stack = vec![src.clone()];
        while let Some(dir) = stack.pop() {
            let listing = match fs::read_dir(&dir) {
                Ok(listing) => listing,
                Err(_) => continue,
            };
            for item in listing.flatten() {
                let path = item.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let source = fs::read_to_string(&path).expect("read source");
                checked += 1;
                // ADR 0007 permits only the bounded CLI executor to spawn.
                // Passive collectors/resolvers and future modules retain the guard.
                let relative = path.strip_prefix(&crates_dir).unwrap().to_string_lossy();
                if relative == "ctxpect-cli/src/tool_process.rs" { continue; }
                if !["ctxpect-cli/src/lib.rs", "ctxpect-cli/src/dispatch.rs",
                    "ctxpect-cli/src/native_oracle.rs", "ctxpect-cli/src/secure_sync.rs",
                    "ctxpect-cli/src/effect_runner.rs"].contains(&relative.as_ref()) {
                    let passive_source = if relative == "ctxpect-cli/src/http.rs" {
                        source.replace("crate::tool_process::private_write", "private_file_write")
                    } else { source.clone() };
                    assert!(!passive_source.contains("tool_process"), "passive path references executor: {relative}");
                }
                for forbidden in FORBIDDEN {
                    assert!(
                        !source.contains(forbidden),
                        "{} references {forbidden:?}; nothing on the scan path may start a process",
                        path.display()
                    );
                }
            }
        }
    }
    assert!(
        crates_seen >= 4,
        "expected to discover the product crates, saw {crates_seen}"
    );
    assert!(
        checked >= 8,
        "expected to inspect the product sources, saw {checked} files"
    );
}

/// A5: excluded directories and files are listed without content.
#[test]
fn excluded_paths_are_listed_without_content() {
    let scratch = Scratch::new("excluded");
    scratch.write(".git/config", "[core]");
    scratch.write("node_modules/pkg/index.js", "module.exports = 1");
    scratch.write("target/debug/artifact", "binary");
    scratch.write(".env", "API_KEY=super-secret");
    scratch.write("id_rsa", "-----BEGIN PRIVATE KEY-----");
    scratch.write("README.md", "hello");

    let inventory = scan(&scratch.root()).expect("scan");

    for excluded in [".git", "node_modules", "target"] {
        let entry = inventory
            .get(excluded)
            .unwrap_or_else(|| panic!("{excluded} must still be listed"));
        assert_eq!(entry.withheld, Some(Withheld::ExcludedDirectory));
        assert!(entry.content_digest.is_none());
    }
    // Nothing beneath an excluded directory was visited at all.
    assert!(inventory.get(".git/config").is_none());
    assert!(inventory.get("node_modules/pkg/index.js").is_none());

    for excluded in [".env", "id_rsa"] {
        let entry = inventory
            .get(excluded)
            .unwrap_or_else(|| panic!("{excluded} must still be listed"));
        assert_eq!(entry.withheld, Some(Withheld::ExcludedFile));
        assert!(
            entry.content_digest.is_none(),
            "{excluded} must carry no content digest"
        );
        assert!(entry.len.is_none());
    }

    // A normal file is read.
    let readme = inventory.get("README.md").expect("README listed");
    assert!(readme.content_digest.is_some());
}

/// A5: the secret's bytes never reach the inventory in any form.
#[test]
fn secret_content_is_absent_from_the_whole_inventory() {
    let scratch = Scratch::new("secret");
    scratch.write(".env", "API_KEY=super-secret-value");
    scratch.write("ok.txt", "ordinary");

    let inventory = scan(&scratch.root()).expect("scan");
    let rendered = format!("{inventory:?}");
    assert!(
        !rendered.contains("super-secret-value"),
        "secret content leaked into the inventory"
    );
    // Nor via a digest an attacker could confirm by guessing the file.
    let secret_digest = ctxpect_schema::sha256_text("API_KEY=super-secret-value");
    assert!(!rendered.contains(&secret_digest));
}

/// A6: two scans of one tree agree, byte for byte.
#[test]
fn repeated_scans_produce_the_same_digest() {
    let scratch = Scratch::new("stable");
    scratch.write("a.md", "alpha");
    scratch.write("nested/b.md", "beta");
    scratch.write("nested/deeper/c.md", "gamma");

    let root = scratch.root();
    let first = scan(&root).expect("scan");
    let second = scan(&root).expect("scan");
    assert_eq!(first.digest(), second.digest());
    assert_eq!(first.entries, second.entries);
}

/// A6: touching mtime without changing bytes does not move the digest.
#[test]
fn mtime_alone_does_not_change_the_digest() {
    let scratch = Scratch::new("mtime");
    let path = scratch.write("a.md", "alpha");
    let root = scratch.root();
    let before = scan(&root).expect("scan").digest();

    // Rewrite identical bytes, which updates mtime.
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&path, "alpha").expect("rewrite");

    assert_eq!(scan(&root).expect("scan").digest(), before);
}

/// A6: changing a collected file's content changes the digest — at equal length.
///
/// review-1 found the earlier version of this test changing "alpha" to "alpha!",
/// so the length alone moved the digest and the content field was never exercised.
/// Deleting `content_digest` from the digest body left it green.
#[test]
fn equal_length_content_changes_move_the_digest() {
    let scratch = Scratch::new("equal-len");
    let path = scratch.write("a.md", "alpha");
    let root = scratch.root();
    let before = scan(&root).expect("scan").digest();

    fs::write(&path, "ALPHA").expect("rewrite");
    assert_eq!(
        fs::metadata(&path).expect("meta").len(),
        5,
        "the rewrite must keep the length identical"
    );
    assert_ne!(
        scan(&root).expect("scan").digest(),
        before,
        "an equal-length content change must move the digest"
    );
}

/// A6: a change past the retained-content cap still moves the digest.
///
/// review-1 found the digest covering only the retained prefix, so a large file
/// could be edited past the cap invisibly.
#[test]
fn changes_beyond_the_read_cap_move_the_digest() {
    let scratch = Scratch::new("beyond-cap");
    let cap = usize::try_from(ctxpect_fs::MAX_READ_BYTES).expect("fits");
    let path = scratch.path.join("big.bin");
    let mut bytes = vec![b'A'; cap + 4096];
    fs::write(&path, &bytes).expect("write");

    let root = scratch.root();
    let before = scan(&root).expect("scan").digest();

    // Change the last byte, well past the cap, keeping the length identical.
    let last = bytes.len() - 1;
    bytes[last] = b'Z';
    fs::write(&path, &bytes).expect("rewrite");
    assert_eq!(fs::metadata(&path).expect("meta").len() as usize, cap + 4096);

    assert_ne!(
        scan(&root).expect("scan").digest(),
        before,
        "a change past the retained-content cap must still move the digest"
    );
}

/// A6: every field in the digest body moves the digest.
///
/// review-2 found the previous version of this test asserting on `Entry` struct
/// fields rather than on the digest, so seven of the eight body fields could still
/// be deleted with everything green. This one changes one field at a time and
/// compares digests, which is the property the name claims.
#[test]
fn every_digest_body_field_moves_the_digest() {
    fn base() -> Entry {
        Entry {
            path: "a.md".into(),
            kind: EntryKind::File,
            content_digest: Some("d".repeat(64)),
            len: Some(7),
            truncated: false,
            withheld: None,
            link_target: None,
            link_count: Some(1),
            identity: None,
        }
    }
    fn digest_of(entry: Entry) -> String {
        Inventory {
            entries: vec![entry],
                      truncated: false,
                      file_limit: None,
                  }
        .digest()
    }

    let baseline = digest_of(base());

    // Each mutation touches exactly one field.
    let mutations: Vec<(&str, Entry)> = vec![
        ("path", Entry { path: "b.md".into(), ..base() }),
        ("kind", Entry { kind: EntryKind::Directory, ..base() }),
        ("content_digest", Entry { content_digest: Some("e".repeat(64)), ..base() }),
        ("content_digest-to-none", Entry { content_digest: None, ..base() }),
        ("len", Entry { len: Some(8), ..base() }),
        ("len-to-none", Entry { len: None, ..base() }),
        ("truncated", Entry { truncated: true, ..base() }),
        ("withheld", Entry { withheld: Some(Withheld::ExcludedFile), ..base() }),
        ("link_target", Entry { link_target: Some("t".into()), ..base() }),
        ("link_count", Entry { link_count: Some(2), ..base() }),
        ("link_count-to-none", Entry { link_count: None, ..base() }),
    ];

    for (label, mutated) in mutations {
        assert_ne!(
            digest_of(mutated),
            baseline,
            "changing {label} must move the digest; it is absent from the digest body"
        );
    }
}

/// A6: `identity` is deliberately outside the digest body.
///
/// Inode numbers are not stable across machines or re-creation, so a digest that
/// included them would differ for identical content. This pins that exclusion, so
/// adding it to the body later fails here rather than silently destabilising every
/// digest.
#[test]
fn filesystem_identity_stays_out_of_the_digest() {
    let with_identity = Entry {
        path: "a.md".into(),
        kind: EntryKind::File,
        content_digest: Some("d".repeat(64)),
        len: Some(7),
        truncated: false,
        withheld: None,
        link_target: None,
        link_count: Some(1),
        identity: Some(ctxpect_fs::FileIdentity {
            device: 42,
            inode: 4242,
        }),
    };
    let without_identity = Entry {
        identity: None,
        ..with_identity.clone()
    };
    let other_identity = Entry {
        identity: Some(ctxpect_fs::FileIdentity {
            device: 7,
            inode: 7777,
        }),
        ..with_identity.clone()
    };

    let digest = |entry: Entry| Inventory { entries: vec![entry], truncated: false, file_limit: None }.digest();
    assert_eq!(digest(with_identity.clone()), digest(without_identity));
    assert_eq!(digest(with_identity), digest(other_identity));
}

/// A5+A7: a hard link to an excluded file does not smuggle its bytes in./// A5+A7: a hard link to an excluded file does not smuggle its bytes in.
///
/// review-1 showed `ln .env notes.md` producing a content digest for the secret
/// under an innocuous name. A hard link is not a path indirection, so containment
/// cannot see it; the alias is caught by filesystem identity instead.
#[cfg(unix)]
#[test]
fn a_hard_link_to_an_excluded_file_is_withheld() {
    let scratch = Scratch::new("alias");
    let secret = scratch.write(".env", "API_KEY=aliased-secret");
    fs::hard_link(&secret, scratch.path.join("notes.md")).expect("hard link");

    let inventory = scan(&scratch.root()).expect("scan");
    let alias = inventory.get("notes.md").expect("alias listed");
    assert!(
        alias.content_digest.is_none(),
        "the alias must not carry the excluded file's digest"
    );
    assert_eq!(
        alias.withheld,
        Some(Withheld::MultiplyLinked),
        "sharing an inode with an excluded file means a link count above one"
    );

    let rendered = format!("{inventory:?}");
    assert!(!rendered.contains("aliased-secret"));
    assert!(!rendered.contains(&ctxpect_schema::sha256_text("API_KEY=aliased-secret")));
}

/// A5+A7: a hard link to a file outside the root does not smuggle it in either.
///
/// Containment works on paths, and a hard link has none to follow. What is
/// observable is the link count, so a multiply-linked file's content is withheld.
#[cfg(unix)]
#[test]
fn a_hard_link_to_outside_content_is_withheld() {
    let scratch = Scratch::new("outside-link");
    let root_dir = scratch.path.join("root");
    fs::create_dir_all(&root_dir).expect("root dir");
    let outside = scratch.write("outside-secret.txt", "OUTSIDE-CONTENT");
    fs::hard_link(&outside, root_dir.join("innocent.md")).expect("hard link");

    let root = Root::new(&root_dir).expect("root");
    let inventory = scan(&root).expect("scan");
    let entry = inventory.get("innocent.md").expect("listed");

    assert_eq!(
        entry.withheld,
        Some(Withheld::MultiplyLinked),
        "a multiply-linked file must be withheld, not collected"
    );
    assert!(entry.content_digest.is_none());
    assert_eq!(entry.link_count, Some(2), "the link count must be reported");

    let rendered = format!("{inventory:?}");
    assert!(
        !rendered.contains("OUTSIDE-CONTENT"),
        "outside content reached the inventory through a hard link"
    );
}

/// A7 control: an ordinary single-linked file is collected normally.
#[test]
fn a_single_linked_file_is_collected() {
    let scratch = Scratch::new("single");
    scratch.write("normal.md", "ordinary content");
    let inventory = scan(&scratch.root()).expect("scan");
    let entry = inventory.get("normal.md").expect("listed");
    assert!(entry.withheld.is_none());
    assert!(entry.content_digest.is_some());
    assert_eq!(entry.link_count, Some(1));
}

/// A5: the exclusion list covers sensitive files beyond exact base names.
///
/// review-1 read `server.pem`, `private.key`, `id_dsa` and `.env.production` in
/// full, because matching was on exact names only.
#[test]
fn sensitive_files_are_excluded_by_extension_and_prefix() {
    let scratch = Scratch::new("sensitive");
    let sensitive = [
        "server.pem",
        "private.key",
        "keystore.p12",
        "bundle.pfx",
        "id_dsa",
        ".env.production",
        ".env.local",
        ".git-credentials",
        ".npmrc",
        "id_rsa.bak",
        "secrets.gpg",
        "vault.kdbx",
    ];
    for name in sensitive {
        scratch.write(name, &format!("SENSITIVE-{name}"));
    }
    scratch.write("readme.md", "public");

    let inventory = scan(&scratch.root()).expect("scan");
    for name in sensitive {
        let entry = inventory
            .get(name)
            .unwrap_or_else(|| panic!("{name} must be listed"));
        assert_eq!(
            entry.withheld,
            Some(Withheld::ExcludedFile),
            "{name} must be excluded"
        );
        assert!(entry.content_digest.is_none(), "{name} must have no digest");
    }
    let rendered = format!("{inventory:?}");
    for name in sensitive {
        assert!(
            !rendered.contains(&format!("SENSITIVE-{name}")),
            "{name} content leaked"
        );
    }
    assert!(inventory.get("readme.md").unwrap().content_digest.is_some());
}

/// A5: exclusion is case-insensitive, matching the declared macOS lane.
///
/// review-1 found `.ENV` and `ID_RSA` read in full on a case-insensitive
/// filesystem, where they name the same bytes as the lowercase forms.
#[test]
fn exclusion_is_case_insensitive() {
    let scratch = Scratch::new("case");
    for name in [".ENV", "ID_RSA", "Server.PEM", ".Git-Credentials"] {
        scratch.write(name, &format!("SECRET-{name}"));
    }

    let inventory = scan(&scratch.root()).expect("scan");
    for entry in &inventory.entries {
        if entry.kind == EntryKind::File {
            assert_eq!(
                entry.withheld,
                Some(Withheld::ExcludedFile),
                "{} must be excluded regardless of case",
                entry.path
            );
        }
    }
    let rendered = format!("{inventory:?}");
    assert!(!rendered.contains("SECRET-"));
}

/// A5: an excluded directory is skipped whatever its case.
#[test]
fn excluded_directories_are_case_insensitive() {
    let scratch = Scratch::new("case-dir");
    scratch.write("Node_Modules/pkg/index.js", "module.exports = 1");
    scratch.write("keep.md", "kept");

    let inventory = scan(&scratch.root()).expect("scan");
    assert!(
        inventory.get("Node_Modules/pkg/index.js").is_none(),
        "an excluded directory must not be descended into whatever its case"
    );
    assert!(inventory.get("keep.md").is_some());
}

/// A6: entries come back sorted.
///
/// This observes the post-condition, not the code that produces it. On APFS
/// `read_dir` already returns names in order, so removing both `sort` calls leaves
/// this green — the sorting is a defence for filesystems that make no such
/// promise, and this platform cannot exercise it. Recorded in the crate's
/// "do not observe" list rather than claimed as coverage.
#[test]
fn entries_come_back_sorted_regardless_of_depth() {
    let scratch = Scratch::new("sorted-deep");
    for name in [
        "z.md",
        "a.md",
        "m/n.md",
        "m/a.md",
        "b.md",
        "m/z/deep.md",
        "0.md",
    ] {
        scratch.write(name, "x");
    }

    let inventory = scan(&scratch.root()).expect("scan");
    let paths: Vec<&str> = inventory.entries.iter().map(|e| e.path.as_str()).collect();
    let mut expected = paths.clone();
    expected.sort_unstable();
    assert_eq!(
        paths, expected,
        "entries must be sorted; the digest is built in this order"
    );
}

/// A6: the digest is a function of the entry set, not of its order.
///
/// Built directly rather than through a scan, because the filesystem will not hand
/// back an unsorted listing on this platform. Feeding the same entries in a
/// different order must not change the digest — that is the property `scan`'s
/// sorting exists to guarantee.
#[test]
fn digest_is_independent_of_entry_order() {
    fn entry(path: &str, digest: &str) -> Entry {
        Entry {
            path: path.into(),
            kind: EntryKind::File,
            content_digest: Some(digest.repeat(64)),
            len: Some(1),
            truncated: false,
            withheld: None,
            link_target: None,
            link_count: Some(1),
            identity: None,
        }
    }
    let a = entry("a.md", "1");
    let b = entry("b.md", "2");
    let c = entry("c.md", "3");

    let sorted = Inventory {
        entries: vec![a.clone(), b.clone(), c.clone()],
                               truncated: false,
                               file_limit: None,
                           };
    let shuffled = Inventory {
        entries: vec![c, a, b],
                                 truncated: false,
                                 file_limit: None,
                             };
    assert_ne!(
        sorted.digest(),
        shuffled.digest(),
        "the digest is order-sensitive by construction, which is why `scan` sorts"
    );

    // And a scan of the same tree twice gives the same order, hence same digest.
    let scratch = Scratch::new("order-stable");
    for name in ["c.md", "a.md", "b.md"] {
        scratch.write(name, name);
    }
    let root = scratch.root();
    assert_eq!(
        scan(&root).expect("scan").digest(),
        scan(&root).expect("scan").digest()
    );
}

/// A7: a symlink is recorded as a link, never as a copy of its target./// A7: a symlink is recorded as a link, never as a copy of its target.
///
/// review-3 found this crate had no test creating a real symlink at all — the
/// case had been written earlier and lost in an edit — so six single-point
/// mutations of the symlink branch survived, including one that gave a link to
/// `.env` the excluded file's own content digest.
#[cfg(unix)]
#[test]
fn a_symlink_is_recorded_as_a_link_not_a_copy() {
    let scratch = Scratch::new("symlink");
    let target = scratch.write("real.md", "target content");
    std::os::unix::fs::symlink(&target, scratch.path.join("alias.md")).expect("symlink");

    let inventory = scan(&scratch.root()).expect("scan");
    let alias = inventory.get("alias.md").expect("alias listed");

    assert_eq!(alias.kind, EntryKind::Symlink, "kind must say symlink");
    assert!(
        alias.content_digest.is_none(),
        "a symlink must not carry its target's content digest"
    );
    assert!(alias.len.is_none(), "a symlink has no content length");
    assert_eq!(
        alias.withheld,
        Some(Withheld::NotRegular(EntryKind::Symlink)),
        "the reason must name what it is"
    );
    assert!(
        alias
            .link_target
            .as_deref()
            .is_some_and(|target| target.ends_with("real.md")),
        "the target must be reported as written, got {:?}",
        alias.link_target
    );

    // The real file is still collected normally.
    let real = inventory.get("real.md").expect("target listed");
    assert!(real.content_digest.is_some());
    assert!(real.link_target.is_none());
}

/// A5: a symlink pointing at an excluded file gets none of its content.
///
/// This is the case review-3 demonstrated: routing symlinks through the ordinary
/// content path handed a link to `.env` the secret's digest, and every test stayed
/// green.
#[cfg(unix)]
#[test]
fn a_symlink_to_an_excluded_file_carries_no_digest() {
    let scratch = Scratch::new("symlink-env");
    let secret = scratch.write(".env", "API_KEY=linked-secret");
    std::os::unix::fs::symlink(&secret, scratch.path.join("notes.md")).expect("symlink");

    let inventory = scan(&scratch.root()).expect("scan");
    let link = inventory.get("notes.md").expect("link listed");
    assert!(
        link.content_digest.is_none(),
        "a link to an excluded file must not carry its digest"
    );

    let rendered = format!("{inventory:?}");
    assert!(!rendered.contains("linked-secret"), "secret content leaked");
    assert!(
        !rendered.contains(&ctxpect_schema::sha256_text("API_KEY=linked-secret")),
        "the excluded file's digest leaked through the link"
    );
}

/// A7: a symlink never joins a duplicate group.
///
/// Grouping a link with its target would assert they are the same asset, which
/// A7 forbids: identical bytes are not proof of one asset, and a link is not even
/// a second copy.
#[cfg(unix)]
#[test]
fn a_symlink_never_joins_a_duplicate_group() {
    let scratch = Scratch::new("symlink-dup");
    let target = scratch.write("real.md", "shared bytes");
    std::os::unix::fs::symlink(&target, scratch.path.join("alias.md")).expect("symlink");

    let inventory = scan(&scratch.root()).expect("scan");
    for group in inventory.duplicate_groups() {
        assert!(
            !group.contains(&"alias.md"),
            "a symlink joined a duplicate group: {group:?}"
        );
    }
}

/// A1+A7: a symlink pointing outside the root is recorded, never followed.
#[cfg(unix)]
#[test]
fn a_symlink_pointing_outside_is_recorded_without_content() {
    let scratch = Scratch::new("symlink-out");
    let root_dir = scratch.path.join("root");
    fs::create_dir_all(&root_dir).expect("root dir");
    let outside = scratch.write("outside.txt", "OUTSIDE-SECRET");
    std::os::unix::fs::symlink(&outside, root_dir.join("escape.md")).expect("symlink");

    let root = Root::new(&root_dir).expect("root");
    let inventory = scan(&root).expect("scan");
    let entry = inventory.get("escape.md").expect("listed");

    assert_eq!(entry.kind, EntryKind::Symlink);
    assert!(entry.content_digest.is_none());
    assert!(
        !format!("{inventory:?}").contains("OUTSIDE-SECRET"),
        "outside content reached the inventory through a symlink"
    );
}

/// A2+A7: a dangling symlink is recorded, and the scan continues past it.
#[cfg(unix)]
#[test]
fn a_dangling_symlink_does_not_stop_the_scan() {
    let scratch = Scratch::new("symlink-dangle");
    scratch.write("before.md", "a");
    std::os::unix::fs::symlink(
        scratch.path.join("does-not-exist.md"),
        scratch.path.join("dangling.md"),
    )
    .expect("symlink");
    scratch.write("zafter.md", "b");

    let inventory = scan(&scratch.root()).expect("scan must continue past a dangling link");
    let entry = inventory.get("dangling.md").expect("listed");
    assert_eq!(entry.kind, EntryKind::Symlink);
    assert!(entry.content_digest.is_none());
    assert!(entry.link_target.is_some(), "the target is reported as written");
    assert!(inventory.get("before.md").is_some());
    assert!(inventory.get("zafter.md").is_some());
}

/// A7: a symlink to a directory is recorded as a link, and is not descended into.
#[cfg(unix)]
#[test]
fn a_symlink_to_a_directory_is_not_descended() {
    let scratch = Scratch::new("symlink-dir");
    scratch.write("real/inner.md", "inner content");
    std::os::unix::fs::symlink(scratch.path.join("real"), scratch.path.join("mirror"))
        .expect("symlink");

    let inventory = scan(&scratch.root()).expect("scan");
    let link = inventory.get("mirror").expect("link listed");
    assert_eq!(link.kind, EntryKind::Symlink);
    assert!(
        inventory.get("mirror/inner.md").is_none(),
        "a symlinked directory must not be walked a second time"
    );
    assert!(inventory.get("real/inner.md").is_some());
}

/// A7: duplicate content is reported, not resolved into one asset.
#[test]
fn duplicate_content_is_reported_not_merged() {
    let scratch = Scratch::new("dupes");
    scratch.write("one.md", "same bytes");
    scratch.write("nested/two.md", "same bytes");
    scratch.write("unique.md", "different");

    let inventory = scan(&scratch.root()).expect("scan");
    let groups = inventory.duplicate_groups();
    assert_eq!(groups.len(), 1, "expected one duplicate group: {groups:?}");
    assert_eq!(groups[0], vec!["nested/two.md", "one.md"]);

    // Both remain separate entries.
    assert!(inventory.get("one.md").is_some());
    assert!(inventory.get("nested/two.md").is_some());
}

/// A2 in the collector: a socket is listed as non-regular, and the scan continues.
#[cfg(unix)]
#[test]
fn non_regular_entries_do_not_stop_the_scan() {
    let scratch = Scratch::new("sock");
    scratch.write("before.md", "a");
    let _listener =
        std::os::unix::net::UnixListener::bind(scratch.path.join("live.sock")).expect("bind");
    scratch.write("zafter.md", "b");

    let inventory = scan(&scratch.root()).expect("scan must continue past a socket");
    let socket = inventory.get("live.sock").expect("socket listed");
    assert_eq!(socket.kind, EntryKind::Socket);
    assert_eq!(
        socket.withheld,
        Some(Withheld::NotRegular(EntryKind::Socket))
    );
    assert!(inventory.get("before.md").is_some());
    assert!(inventory.get("zafter.md").is_some());
}

/// A8: an oversized file is marked truncated and keeps its real length.
/// Truncation is of retained content; the content digest still covers the file.
#[test]
fn oversized_files_are_marked_truncated() {
    let scratch = Scratch::new("big");
    let size = usize::try_from(ctxpect_fs::MAX_READ_BYTES).expect("fits") + 10;
    fs::write(scratch.path.join("big.bin"), vec![b'z'; size]).expect("write");

    let inventory = scan(&scratch.root()).expect("scan");
    let entry = inventory.get("big.bin").expect("listed");
    assert!(entry.truncated);
    assert_eq!(entry.len, Some(size as u64));
    assert_eq!(
        entry.content_digest.as_ref().map(String::len),
        Some(64),
        "truncation marks retained content; the whole-file digest remains"
    );
}

/// An empty root scans to an empty inventory with a stable digest.
#[test]
fn an_empty_root_is_stable() {
    let scratch = Scratch::new("empty");
    let root = scratch.root();
    let first = scan(&root).expect("scan");
    assert!(first.entries.is_empty());
    assert_eq!(first.digest(), scan(&root).expect("scan").digest());
}

/// Entries come back sorted, so callers see a stable order too.
#[test]
fn entries_are_sorted_by_path() {
    let scratch = Scratch::new("sorted");
    for name in ["z.md", "a.md", "m/n.md", "b.md"] {
        scratch.write(name, "x");
    }
    let inventory = scan(&scratch.root()).expect("scan");
    let paths: Vec<&str> = inventory.entries.iter().map(|e| e.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted);
}

#[test]
fn a_file_limit_truncates_and_says_so() {
    // `resource_limits.scan_files` used to be stored and validated but never
    // enforced: configuring it changed nothing.
    let scratch = Scratch::new("limit");
    for n in 0..12 {
        scratch.write(&format!("dir{}/file{}.txt", n % 3, n), "x");
    }
    let root = scratch.root();

    let full = scan(&root).expect("scan");
    assert!(!full.truncated, "an unlimited scan is not truncated");
    assert_eq!(full.file_limit, None);
    let complete = full.entries.len();
    assert!(complete >= 12, "expected the files plus their directories");

    let limited = scan_with_limit(&root, Some(5)).expect("scan");
    assert!(limited.truncated, "hitting the limit must be recorded");
    assert_eq!(limited.file_limit, Some(5));
    assert!(
        limited.entries.len() <= 5,
        "walk must stop at the limit, got {}",
        limited.entries.len()
    );

    // A limit above the tree size changes nothing.
    let generous = scan_with_limit(&root, Some(complete + 100)).expect("scan");
    assert!(!generous.truncated);
    assert_eq!(generous.entries.len(), complete);

    // A truncated inventory is a different inventory: its digest must not
    // equal the complete one, or a partial scan could pass as a full one.
    assert_ne!(limited.digest(), full.digest());
}
