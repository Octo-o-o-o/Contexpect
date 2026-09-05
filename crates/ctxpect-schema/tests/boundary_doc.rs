//! Interlocks between the boundary table, the rendered documentation and the two
//! parsers.
//!
//! # What these tests observe
//!
//! - `BOUNDARY.md` equals `render_markdown()` byte for byte.
//! - Each row's `we_accept` / `python_accepts` equals what the two parsers do.
//! - Each rendered section's membership equals the set computed from the fields.
//! - `src/lib.rs` carries no hand-written prose about the boundary.
//! - Seven documents named by earlier reviews are still present.
//!
//! # What they do not observe
//!
//! - **Whether a `Rationale`'s fixed sentence is true.** `Rationale::permits`
//!   constrains which variant a row may cite, not what that variant says. A false
//!   sentence in `Rationale::as_prose` passes every test in this file.
//! - Documentation anywhere outside this crate.
//! - Whether `BOUNDARY` is complete against the two parsers. Rows are the cases
//!   someone wrote down; completeness is a review activity, not a test here.
//!
//! The first item is a known open surface, not an oversight. It is recorded in
//! `docs/plan/contexpect-wp02-core/DEFERRED-P2.md`.

use ctxpect_schema::boundary::{render_markdown, BoundaryCase, Rationale, Section, BOUNDARY};
use ctxpect_schema::parse;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    })
}

fn python_accepts(bin: &str, root: &Path, document: &str) -> bool {
    let script = "import json,sys\ntry:\n    json.loads(sys.stdin.read())\n    print('yes')\nexcept Exception:\n    print('no')\n";
    let mut child = Command::new(bin)
        .args(["-c", script])
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn python");
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(document.as_bytes()).expect("write");
    }
    let out = child.wait_with_output().expect("python");
    String::from_utf8_lossy(&out.stdout).trim() == "yes"
}

/// Link 1: the documentation is exactly the render of the table.
///
/// `lib.rs` includes `BOUNDARY.md` as its own docs, so editing the prose — in any
/// way, including reversing a sentence's meaning while keeping its keywords —
/// fails here. That was the gap review-4 exploited in the previous, substring-based
/// guard.
#[test]
fn documentation_is_rendered_from_the_table() {
    let on_disk = std::fs::read_to_string(crate_root().join("BOUNDARY.md"))
        .expect("BOUNDARY.md must exist; it is included by lib.rs");
    let rendered = render_markdown();
    assert_eq!(
        on_disk, rendered,
        "BOUNDARY.md is out of date. Regenerate it:\n    \
         cargo run -p ctxpect-schema --example render_boundary > crates/ctxpect-schema/BOUNDARY.md"
    );
}

/// Link 2: every row states this parser's real behaviour.
#[test]
fn every_row_states_our_actual_behaviour() {
    for case in BOUNDARY {
        let accepted = parse(case.document).is_ok();
        assert_eq!(
            accepted, case.we_accept,
            "the table says we_accept={} for {}, observed {accepted}",
            case.we_accept, case.document
        );
    }
}

/// Link 3: every row states Python's real behaviour.
#[test]
fn every_row_states_pythons_actual_behaviour() {
    let Some(bin) = python() else {
        panic!("no python interpreter available to check the boundary table");
    };
    let root = crate_root();
    for case in BOUNDARY {
        let accepted = python_accepts(bin, &root, case.document);
        assert_eq!(
            accepted, case.python_accepts,
            "the table says python_accepts={} for {}, observed {accepted}",
            case.python_accepts, case.document
        );
    }
}

/// The rendered prose must actually carry every row, so a row cannot be quietly
/// dropped from the documentation while staying in the table.
#[test]
fn the_render_mentions_every_row() {
    let rendered = render_markdown();
    for case in BOUNDARY {
        assert!(
            rendered.contains(case.document),
            "{} is in the table but not in the rendered docs",
            case.document
        );
        assert!(
            rendered.contains(case.rationale.as_prose()),
            "the rationale for {} is missing from the rendered docs",
            case.document
        );
    }
}

/// Each rendered section contains exactly the rows whose fields put them there.
///
/// review-5 inverted two loops in `render_markdown` so real divergences were
/// published under "Rejected by both" while every test stayed green. Comparing
/// membership, rather than checking that the headings exist, is what catches that.
#[test]
fn rendered_section_membership_equals_the_computed_set() {
    let rendered = render_markdown();

    for section in Section::ALL {
        let expected: Vec<&str> = BOUNDARY
            .iter()
            .filter(|c| c.section() == *section)
            .map(|c| c.document)
            .collect();

        let body = section_body(&rendered, section.heading());
        let listed: Vec<&str> = BOUNDARY
            .iter()
            .map(|c| c.document)
            .filter(|doc| body.contains(&format!("`{doc}`")))
            .collect();

        assert_eq!(
            listed, expected,
            "section {:?} lists {listed:?} but its computed membership is {expected:?}",
            section
        );
    }
}

/// The text under `heading`, up to the next `## ` heading or end of document.
fn section_body(rendered: &str, heading: &str) -> String {
    let start = rendered
        .find(heading)
        .unwrap_or_else(|| panic!("missing section {heading}"))
        + heading.len();
    let rest = &rendered[start..];
    match rest.find("\n## ") {
        Some(end) => rest[..end].to_string(),
        None => rest.to_string(),
    }
}

/// `lib.rs` must not carry hand-written prose about the boundary.
///
/// review-5 added a contradicting `//!` block beside the generated section and
/// every test stayed green. The boundary may only reach the crate docs through
/// the generated file.
#[test]
fn lib_rs_carries_no_hand_written_boundary_prose() {
    let src = include_str!("../src/lib.rs");
    assert!(
        src.contains(r#"#![doc = include_str!("../BOUNDARY.md")]"#),
        "lib.rs must include the generated boundary document"
    );

    // Terms that belong to the boundary. They may appear in the generated file,
    // never in a hand-written line of lib.rs. Matched on word boundaries so
    // "acceptance fixture" does not read as a claim about what is accepted.
    const BOUNDARY_TERMS: &[&str] = &[
        "float", "floats", "surrogate", "surrogates", "i64", "nan", "infinity",
        "byte-compatible", "rejects", "rejected", "accepts", "accepted",
        "arbitrary precision", "python",
    ];

    for (index, line) in src.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//!") {
            continue;
        }
        let lowered = trimmed.to_lowercase();
        for term in BOUNDARY_TERMS {
            assert!(
                !mentions_word(&lowered, term),
                "lib.rs:{} hand-writes boundary prose ({term:?}): {trimmed}\n\
                 The boundary belongs in BOUNDARY, rendered into BOUNDARY.md.",
                index + 1
            );
        }
    }
}

/// Whether `haystack` contains `needle` as a whole word.
fn mentions_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphabetic();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphabetic();
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

#[test]
fn word_matching_does_not_fire_on_substrings() {
    // The guard is only useful if it distinguishes "acceptance fixture" from a
    // claim about what the parser accepts.
    assert!(!mentions_word("an acceptance fixture", "accepts"));
    assert!(!mentions_word("an acceptance fixture", "accepted"));
    assert!(mentions_word("floats are accepted here", "accepted"));
    assert!(mentions_word("no floats", "floats"));
    assert!(!mentions_word("floating", "float"));
}

/// The table's contents are pinned exactly.
///
/// review-5 deleted a real divergence row that an earlier, illustrative list did
/// not happen to name, and every test stayed green. Pinning the whole set means a
/// row can only leave the table by editing this list too — a deliberate act rather
/// than an omission.
#[test]
fn the_table_contains_exactly_these_documents() {
    const EXPECTED: &[&str] = &[
        r#"{"v":1000000000000000.25}"#,
        r#"{"v":1.0}"#,
        r#"{"v":1e16}"#,
        r#"{"v":9223372036854775808}"#,
        r#"{"v":NaN}"#,
        r#"{"v":Infinity}"#,
        r#"{"v":"\ud800"}"#,
        r#"{"v":01}"#,
        r#"{"v":"\u+041"}"#,
        r#"{"v":1.}"#,
        r#"{"v":0.e5}"#,
        "{\"v\":\"a\u{1}b\"}",
        r#"{"v":1,}"#,
        r#"{'v':1}"#,
        r#"{"v":[1,{"k":"值"},null,true,-7]}"#,
    ];

    let actual: Vec<&str> = BOUNDARY.iter().map(|c| c.document).collect();
    assert_eq!(
        actual, EXPECTED,
        "the boundary table changed. If that is intended, update EXPECTED here too; \
         every row was put there by a review finding."
    );
}

/// Every rationale the closed set defines is actually used, so a variant cannot
/// quietly become dead while the case it described disappears.
#[test]
fn every_rationale_variant_is_exercised() {
    for variant in [
        Rationale::FloatRoundingTies,
        Rationale::FloatLiteral,
        Rationale::IntegerBeyondI64,
        Rationale::NonJsonLiteral,
        Rationale::LoneSurrogate,
        Rationale::InvalidJsonSyntax,
        Rationale::SupportedSubset,
    ] {
        assert!(
            BOUNDARY.iter().any(|c| c.rationale == variant),
            "{variant:?} is defined but no row uses it"
        );
    }
}

/// Sanity: `BoundaryCase::is_divergence` is not vacuous.
#[test]
fn is_divergence_distinguishes_the_two_kinds() {
    let diverging = BoundaryCase {
        document: "x",
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::FloatLiteral,
    };
    let agreeing = BoundaryCase {
        document: "x",
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    };
    assert!(diverging.is_divergence());
    assert!(!agreeing.is_divergence());
}
