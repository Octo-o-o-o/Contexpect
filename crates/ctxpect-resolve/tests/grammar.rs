//! Grammar fixtures for G1–G5 against a real filesystem.

use ctxpect_core::{TruthState, UnknownReason};
use ctxpect_fs::Root;
use ctxpect_resolve::{
    EdgeKind, PROJECT_DOC_MAX_BYTES, ResolveRequest, RootKind, layers_toward_cwd, resolve,
};
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
            "cx-res-{label}-{}-{}",
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

fn resolve_at(
    scratch: &Scratch,
    cwd_rel: &str,
    home: Option<&Root>,
) -> ctxpect_resolve::Resolution {
    let project = scratch.root();
    resolve(&ResolveRequest {
        project: &project,
        cwd_rel,
        codex_home: home,
        project_doc_max_bytes: PROJECT_DOC_MAX_BYTES,
    })
    .expect("resolve")
}

#[test]
fn g1_walks_root_toward_cwd_and_skips_files_below_cwd() {
    let scratch = Scratch::new("g1");
    scratch.write("AGENTS.md", "root\n");
    scratch.write("src/AGENTS.md", "src\n");
    scratch.write("src/app/AGENTS.md", "app\n");
    scratch.write("other/AGENTS.md", "other\n");

    let at_src = resolve_at(&scratch, "src", None);
    assert!(at_src.included);
    assert_eq!(
        at_src
            .edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::IncludedBy)
            .map(|edge| edge.path.as_str())
            .collect::<Vec<_>>(),
        vec!["AGENTS.md", "src/AGENTS.md"]
    );
    assert!(
        !at_src
            .native_paths_used
            .iter()
            .any(|path| path == "src/app/AGENTS.md" || path == "other/AGENTS.md")
    );
    assert_eq!(layers_toward_cwd("src"), vec!["", "src"]);

    let at_root = resolve_at(&scratch, "", None);
    assert_eq!(
        at_root
            .edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::IncludedBy)
            .map(|edge| edge.path.as_str())
            .collect::<Vec<_>>(),
        vec!["AGENTS.md"]
    );
}

#[test]
fn g2_override_marks_agents_overridden_and_shadowed() {
    let scratch = Scratch::new("g2");
    scratch.write("AGENTS.override.md", "override\n");
    scratch.write("AGENTS.md", "base\n");

    let resolution = resolve_at(&scratch, "", None);
    assert!(resolution.included);
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::IncludedBy
            && edge.path == "AGENTS.override.md"
            && edge.rule_id == "G1"
    }));
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::OverriddenBy
            && edge.path == "AGENTS.md"
            && edge.related_path.as_deref() == Some("AGENTS.override.md")
            && edge.rule_id == "G2"
    }));
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::ShadowedBy
            && edge.path == "AGENTS.md"
            && edge.related_path.as_deref() == Some("AGENTS.override.md")
            && edge.rule_id == "G2"
    }));
    assert!(
        !resolution
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::IncludedBy && edge.path == "AGENTS.md")
    );
}

#[test]
fn g3_does_not_truncate_when_aggregate_is_at_or_below_cap() {
    for size in [1_u64, PROJECT_DOC_MAX_BYTES] {
        let scratch = Scratch::new(&format!("g3-ok-{size}"));
        scratch.write("AGENTS.md", &"z".repeat(size as usize));
        let resolution = resolve_at(&scratch, "", None);
        assert!(resolution.included, "size={size}");
        assert!(!resolution.truncated, "size={size}");
        assert_eq!(resolution.aggregated_bytes, size, "size={size}");
        assert!(
            resolution
                .edges
                .iter()
                .all(|edge| edge.kind != EdgeKind::TruncatedAfter && edge.offset.is_none()),
            "size={size}"
        );
    }
}

#[test]
fn g3_truncates_aggregate_at_official_spec_cap() {
    let scratch = Scratch::new("g3");
    let oversized = "x".repeat(PROJECT_DOC_MAX_BYTES as usize + 100);
    scratch.write("AGENTS.md", &oversized);

    let resolution = resolve_at(&scratch, "", None);
    assert!(resolution.included);
    assert!(resolution.truncated);
    assert_eq!(resolution.aggregated_bytes, PROJECT_DOC_MAX_BYTES);
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::TruncatedAfter
            && edge.path == "AGENTS.md"
            && edge.offset == Some(PROJECT_DOC_MAX_BYTES)
            && edge.file_offset == Some(PROJECT_DOC_MAX_BYTES)
            && edge.rule_id == "G3"
    }));
}

#[test]
fn g4_ignore_excludes_as_product_rule_not_native() {
    let scratch = Scratch::new("g4");
    scratch.write("AGENTS.md", "keep-me-out\n");
    scratch.write(".ctxpect-ignore", "AGENTS.md\n");

    let resolution = resolve_at(&scratch, "", None);
    assert!(!resolution.included);
    assert_eq!(
        resolution.claims.primary.truth_state,
        ctxpect_core::TruthState::Absent
    );
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::ExcludedBy
            && edge.path == "AGENTS.md"
            && edge.related_path.as_deref() == Some(".ctxpect-ignore")
            && edge.note.as_deref() == Some("product-user-exclusion")
            && edge.rule_id == "G4"
    }));
    assert!(
        resolution
            .native_paths_used
            .iter()
            .any(|path| path == ".ctxpect-ignore")
    );
    assert!(
        resolution
            .native_paths_used
            .iter()
            .any(|path| path == "AGENTS.md")
    );
    let ignored = resolution
        .evidence
        .iter()
        .find(|item| item.path == "AGENTS.md")
        .expect("excluded AGENTS.md evidence");
    assert_eq!(ignored.content_digest, None);
    assert!(resolution.ignore_warnings.is_empty());
}

#[test]
fn g4_ignore_skips_absolute_and_parent_lines_with_line_warnings() {
    let scratch = Scratch::new("g4-skip");
    scratch.write("AGENTS.md", "keep-visible\n");
    scratch.write(".ctxpect-ignore", "/etc/passwd\n../x\n");

    let resolution = resolve_at(&scratch, "", None);
    assert!(resolution.included);
    assert_eq!(
        resolution
            .ignore_warnings
            .iter()
            .map(|item| item.line)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(
        resolution
            .ignore_warnings
            .iter()
            .all(|item| item.code == "ignore.invalid_line")
    );
    assert!(
        resolution
            .edges
            .iter()
            .all(|edge| edge.kind != EdgeKind::ExcludedBy)
    );
    let dumped = format!("{resolution:?}");
    assert!(!dumped.contains("/etc/passwd"));
    assert!(!dumped.contains("../x"));
}

#[test]
fn g5_without_codex_home_global_layer_is_permission_not_granted() {
    let scratch = Scratch::new("g5");
    scratch.write("AGENTS.md", "project\n");
    let resolution = resolve_at(&scratch, "", None);
    let global = resolution
        .layers
        .iter()
        .find(|layer| layer.id == "global")
        .expect("global layer");
    assert_eq!(global.unknown, Some(UnknownReason::PermissionNotGranted));
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::UnknownBecause
            && edge.rule_id == "G5"
            && edge.note.as_deref() == Some("permission_not_granted")
    }));
}

#[test]
fn g5_explicit_codex_home_includes_global_agents() {
    let project = Scratch::new("g5p");
    project.write("src/.keep", "x\n");
    let home = Scratch::new("g5h");
    home.write("AGENTS.md", "from-home\n");
    let home_root = home.root();
    let resolution = resolve_at(&project, "", Some(&home_root));
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::IncludedBy && edge.path == "AGENTS.md" && edge.rule_id == "G5"
    }));
    assert!(resolution.codex_home_inventory.is_some());
}

#[test]
fn explicit_codex_home_does_not_inventory_non_agents_files() {
    let project = Scratch::new("home-min-p");
    project.write("README.md", "x\n");
    let home = Scratch::new("home-min-h");
    home.write("AGENTS.md", "from-home\n");
    home.write(
        "sessions/2026.jsonl",
        "SENTINEL_SESSION_UNIQ_rsb03_do_not_read\n",
    );
    home.write("auth.json", "SENTINEL_AUTH_UNIQ_rsb03_do_not_read\n");
    home.write("config.toml", "SENTINEL_CONFIG_UNIQ_rsb03_do_not_read\n");
    let home_root = home.root();
    let resolution = resolve_at(&project, "", Some(&home_root));
    let inventory = resolution
        .codex_home_inventory
        .as_ref()
        .expect("granted home inventory");
    assert!(inventory.get("AGENTS.md").is_some());
    assert!(inventory.get("sessions/2026.jsonl").is_none());
    assert!(inventory.get("auth.json").is_none());
    assert!(inventory.get("config.toml").is_none());
    assert_eq!(inventory.entries.len(), 1);
}

#[test]
fn g3_zero_byte_file_after_exact_cap_is_not_truncated() {
    let scratch = Scratch::new("g3-zero-tail");
    scratch.write("AGENTS.md", &"z".repeat(PROJECT_DOC_MAX_BYTES as usize));
    scratch.write("src/AGENTS.md", "");
    let resolution = resolve_at(&scratch, "src", None);
    assert!(resolution.included);
    assert!(!resolution.truncated);
    assert_eq!(resolution.aggregated_bytes, PROJECT_DOC_MAX_BYTES);
    assert!(
        resolution
            .edges
            .iter()
            .all(|edge| edge.kind != EdgeKind::TruncatedAfter)
    );
}

#[test]
fn g3_one_byte_file_after_exact_cap_is_truncated_at_file_offset_zero() {
    let scratch = Scratch::new("g3-one-tail");
    scratch.write("AGENTS.md", &"z".repeat(PROJECT_DOC_MAX_BYTES as usize));
    scratch.write("src/AGENTS.md", "x");
    let resolution = resolve_at(&scratch, "src", None);
    assert!(resolution.included);
    assert!(resolution.truncated);
    assert_eq!(resolution.aggregated_bytes, PROJECT_DOC_MAX_BYTES);
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::TruncatedAfter
            && edge.path == "src/AGENTS.md"
            && edge.offset == Some(PROJECT_DOC_MAX_BYTES)
            && edge.file_offset == Some(0)
            && edge.rule_id == "G3"
    }));
}

#[test]
fn g5_global_layer_root_kind_is_codex_home() {
    let scratch = Scratch::new("g5-kind");
    scratch.write("AGENTS.md", "project\n");
    let without_home = resolve_at(&scratch, "", None);
    let global = without_home
        .layers
        .iter()
        .find(|layer| layer.id == "global")
        .expect("global");
    assert_eq!(global.root_kind, RootKind::CodexHome);
    let project_root = without_home
        .layers
        .iter()
        .find(|layer| layer.id == "project-root")
        .expect("project-root");
    assert_eq!(project_root.root_kind, RootKind::Project);

    let home = Scratch::new("g5-kind-h");
    home.write("AGENTS.md", "home\n");
    let home_root = home.root();
    let with_home = resolve_at(&scratch, "", Some(&home_root));
    let global = with_home
        .layers
        .iter()
        .find(|layer| layer.id == "global")
        .expect("global granted");
    assert_eq!(global.root_kind, RootKind::CodexHome);
}

fn assert_unknown_override_blocks_agents(resolution: &ctxpect_resolve::Resolution) {
    assert!(!resolution.included);
    assert_eq!(
        resolution.claims.primary.truth_state,
        TruthState::Indeterminate
    );
    assert!(
        resolution
            .edges
            .iter()
            .all(|edge| edge.kind != EdgeKind::IncludedBy),
        "{:?}",
        resolution.edges
    );
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::OverriddenBy
            && edge.path == "AGENTS.md"
            && edge.related_path.as_deref() == Some("AGENTS.override.md")
            && edge.note.as_deref() == Some("override-unknown")
            && edge.rule_id == "G2"
    }));
}

#[cfg(unix)]
#[test]
fn unknown_override_escape_symlink_blocks_same_layer_agents() {
    let outside = Scratch::new("ov-esc-out");
    outside.write("secret.txt", "SENTINEL_OVERRIDE_ESCAPE\n");
    let scratch = Scratch::new("ov-esc");
    std::os::unix::fs::symlink(
        outside.path.join("secret.txt"),
        scratch.path.join("AGENTS.override.md"),
    )
    .expect("symlink");
    scratch.write("AGENTS.md", "should-not-be-adopted\n");
    let resolution = resolve_at(&scratch, "", None);
    assert_unknown_override_blocks_agents(&resolution);
}

#[cfg(unix)]
#[test]
fn unknown_override_fifo_blocks_same_layer_agents() {
    let scratch = Scratch::new("ov-fifo");
    let fifo = scratch.path.join("AGENTS.override.md");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    assert!(made, "mkfifo must succeed for this required case");
    scratch.write("AGENTS.md", "should-not-be-adopted\n");
    let resolution = resolve_at(&scratch, "", None);
    assert_unknown_override_blocks_agents(&resolution);
}

#[test]
fn regular_override_still_adopts_override_and_marks_agents() {
    let scratch = Scratch::new("ov-ok");
    scratch.write("AGENTS.override.md", "override\n");
    scratch.write("AGENTS.md", "base\n");
    let resolution = resolve_at(&scratch, "", None);
    assert!(resolution.included);
    assert_eq!(resolution.claims.primary.truth_state, TruthState::Present);
    assert!(
        resolution
            .edges
            .iter()
            .any(|edge| { edge.kind == EdgeKind::IncludedBy && edge.path == "AGENTS.override.md" })
    );
    assert!(resolution.edges.iter().any(|edge| {
        edge.kind == EdgeKind::OverriddenBy
            && edge.path == "AGENTS.md"
            && edge.related_path.as_deref() == Some("AGENTS.override.md")
            && edge.note.is_none()
    }));
}

#[test]
fn emitted_claims_have_no_violations() {
    let scratch = Scratch::new("claims");
    scratch.write("AGENTS.md", "ok\n");
    let resolution = resolve_at(&scratch, "", None);
    for claim in resolution.claims.all() {
        assert!(claim.violations().is_empty(), "{:?}", claim.violations());
    }
}
