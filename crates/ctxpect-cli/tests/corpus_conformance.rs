//! Static-corpus conformance gate (required gate `corpus-conformance`).
//!
//! The runner lives in `ctxpect_cli::conformance` and is shared with
//! `ctxpect adapter test`; this test runs it over the whole development
//! static corpus and holds the frozen assertions: 1,924 rows, every row in
//! exactly one bucket, codex `instructions` 3/3, claude-code `instructions`
//! 6/6 with its other 54 rows unimplemented, and no pass without an
//! implementation. The report separates implemented passes from
//! unknown-honesty passes and states what was not executed (sealed, live,
//! oracle).

use ctxpect_cli::conformance::run_static_corpus;
use ctxpect_resolve::implemented_capabilities;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn every_static_corpus_row_is_counted_and_no_implemented_cell_fails() {
    let root = repo_root();
    let report = run_static_corpus(&root, None).unwrap_or_else(|err| panic!("{} {}", err.code, err.message));
    print!("{}", report.render_table());

    assert_eq!(report.files.len(), 21, "expected 21 static corpus files, found {}", report.files.len());
    assert_eq!(report.total, 1924, "the static corpus denominator is 1,924 rows");
    let totals = report.totals();
    assert_eq!(
        totals.implemented_pass + totals.unknown_honesty_pass + totals.fail + totals.unimplemented,
        report.total,
        "every row lands in exactly one bucket"
    );
    assert!(
        report.failures.is_empty(),
        "{} implemented rows failed:\n{}",
        report.failures.len(),
        report.failures.join("\n")
    );
    // What this gate does not execute is stated, not implied.
    assert!(report.not_executed.sealed > 0 && report.not_executed.live > 0 && report.not_executed.oracle > 0, "{:?}", report.not_executed);
    assert!(!report.corpus_digest.is_empty() && !report.matrix_digest.is_empty());

    // Codex instructions at its anchor coordinate: 3/3, all implemented passes.
    let codex = report
        .cells
        .get(&("codex/0.147.0/cli/macos-27-arm64".to_string(), "instructions".to_string()))
        .expect("codex instructions cell");
    assert_eq!((codex.total, codex.pass(), codex.unimplemented), (3, 3, 0), "{codex:?}");
    assert_eq!(codex.unknown_honesty_pass, 0, "anchor rows are not honesty rows");

    // Claude Code instructions at its anchor coordinate: 6/6, and the other
    // 54 rows of that coordinate are unimplemented, not passed.
    let claude = report
        .cells
        .get(&("claude-code/2.1.259/cli/macos-27-arm64".to_string(), "instructions".to_string()))
        .expect("claude-code instructions cell");
    assert_eq!((claude.total, claude.pass(), claude.unimplemented), (6, 6, 0), "{claude:?}");
    let claude_other: usize = report
        .cells
        .iter()
        .filter(|((coordinate, capability), _)| {
            coordinate == "claude-code/2.1.259/cli/macos-27-arm64" && capability != "instructions"
        })
        .map(|(_, cell)| {
            assert_eq!(cell.pass(), 0, "{cell:?}");
            cell.unimplemented
        })
        .sum();
    assert_eq!(claude_other, 54, "the remaining 54 claude-code rows are unimplemented");

    // Honesty passes exist (the implemented families' honesty coordinates)
    // and are never mixed into the implemented count.
    assert!(totals.unknown_honesty_pass > 0, "{totals:?}");
    for ((coordinate, capability), cell) in &report.cells {
        let family = coordinate.split('/').next().unwrap_or("");
        if !implemented_capabilities(family).contains(&capability.as_str()) {
            assert_eq!(cell.pass(), 0, "{coordinate} {capability} passed without an implementation");
            assert_eq!(cell.unimplemented, cell.total, "{coordinate} {capability}");
        }
    }
}
