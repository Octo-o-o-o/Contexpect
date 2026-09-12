//! Cross-language canonicalisation parity.
//!
//! A Receipt written by the Rust product must verify against digests frozen by
//! the Python acceptance generator, so `canonical_json` and `digest_value` have
//! to agree with `contexpect_contract.canonical_json` / `_digest_obj` byte for
//! byte. The vectors below are checked against real fixture digests and against
//! Python itself when an interpreter is available.
//!
//! The documented boundary between the two implementations lives in
//! `tests/boundary_doc.rs`, which interlocks the boundary table, the rendered crate
//! documentation and both parsers.

use ctxpect_schema::{canonical_json, digest_value, parse, sha256_text};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repository root")
        .to_path_buf()
}

/// Documents that exercise the parts of canonicalisation most likely to diverge:
/// key ordering, non-ASCII, escapes, nesting, empty containers, negative and
/// large integers, booleans and null.
const VECTORS: &[&str] = &[
    "{}",
    "[]",
    r#"{"b":1,"a":2,"C":3,"_":4}"#,
    r#"{"nested":{"z":[1,2,{"y":null}],"a":{"b":{"c":true}}}}"#,
    r#"{"unicode":"语义对齐 · Contexpect","emoji":"😀"}"#,
    r#"{"escapes":"quote\" backslash\\ newline\n tab\t cr\r"}"#,
    r#"{"control":"\u0001\u001f","short":"\b\f"}"#,
    r#"{"numbers":[0,-1,42,9007199254740991,-9007199254740991]}"#,
    r#"{"booleans":[true,false],"null":null}"#,
    r#"{"empty_string":"","empty_obj":{},"empty_arr":[]}"#,
    r#"{"mixed":[{"k":"v"},[],{},null,true,0,""]}"#,
    r#"{"slash":"a/b","already_escaped":"a\/b"}"#,
    r#"{"key with spaces":1,"key\"quoted":2,"键":3}"#,
];

fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    })
}

fn python_digest(bin: &str, root: &Path, document: &str) -> Option<(String, String)> {
    let script = r#"
import hashlib, json, sys
sys.path.insert(0, sys.argv[1])
from contexpect_contract import canonical_json
obj = json.loads(sys.stdin.read())
text = canonical_json(obj)
print(text)
print(hashlib.sha256(text.encode("utf-8")).hexdigest())
"#;
    let scripts_dir = root.join("scripts");
    let mut child = Command::new(bin)
        .args(["-c", script, scripts_dir.to_str()?])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    {
        use std::io::Write as _;
        // Take and drop the handle so the child sees EOF and stops reading.
        let mut stdin = child.stdin.take()?;
        stdin.write_all(document.as_bytes()).ok()?;
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        panic!(
            "python canonicalisation failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    // The canonical text may itself contain newlines only via escapes, so the
    // digest is the final line and the text is everything before it.
    let (text, digest) = stdout.trim_end_matches('\n').rsplit_once('\n')?;
    Some((text.to_string(), digest.to_string()))
}

#[test]
fn canonical_form_and_digest_match_python() {
    let root = repo_root();
    let Some(bin) = python() else {
        // The gate runs where Python is present; skipping here would hide drift,
        // so fail loudly rather than silently passing.
        panic!("no python interpreter available to check cross-language parity");
    };

    for document in VECTORS {
        let ours = parse(document).unwrap_or_else(|e| panic!("parse {document}: {e}"));
        let our_text = canonical_json(&ours);
        let our_digest = digest_value(&ours);

        let (their_text, their_digest) =
            python_digest(bin, &root, document).expect("python parity helper");

        assert_eq!(
            our_text, their_text,
            "canonical text diverged for {document}"
        );
        assert_eq!(
            our_digest, their_digest,
            "digest diverged for {document}"
        );
        assert_eq!(
            our_digest,
            sha256_text(&our_text),
            "digest must be the hash of the canonical text"
        );
    }
}

/// Floats are outside the supported subset. This test pins the boundary from both
/// sides: we reject them, and where Python accepts them the divergence is real —
/// Rust's shortest representation rounds half away from zero, Python's to even.
#[test]
fn floats_are_rejected_rather_than_silently_divergent() {
    for document in [
        r#"{"v":1.0}"#,
        r#"{"v":0.5}"#,
        r#"{"v":1e16}"#,
        r#"{"v":5e-324}"#,
        r#"{"v":1000000000000000.25}"#,
        r#"{"v":[1.5,2.5]}"#,
    ] {
        assert!(
            parse(document).is_err(),
            "float document {document} must be rejected, not canonicalised"
        );
    }

    // Trailing-decimal forms Python also rejects: both sides agree here.
    for document in [r#"{"k":1.}"#, "1.", "0.e5", "[1.]"] {
        assert!(parse(document).is_err(), "{document} must be rejected");
    }
}

#[test]
fn frozen_fixture_digests_reproduce() {
    // Every semantic-team fixture carries the digest the generator computed for
    // it. Recomputing from the file content must reproduce it exactly.
    let dir = repo_root().join("acceptance/semantic-team");
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));

    let mut checked = 0usize;
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read fixture");
        let value = parse(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

        let Some(recorded) = value.get("digest").and_then(|d| d.as_str()) else {
            continue;
        };

        // The generator digests the fixture with its own `digest` field removed.
        let ctxpect_schema::Value::Object(map) = &value else {
            panic!("fixture is not an object: {}", path.display());
        };
        let mut body = map.clone();
        body.remove("digest");
        let recomputed = digest_value(&ctxpect_schema::Value::Object(body));

        assert_eq!(
            recomputed,
            recorded,
            "digest mismatch for {}",
            path.display()
        );
        checked += 1;
    }
    // The fixture tree is 8 scenarios x 2 polarities plus 1 extra positive
    // (ST2-same-bytes-pos, 2026-09-12 C-F02 ST2 revision) plus 5 malformed cases.
    // A loose floor would let a silently deleted fixture pass.
    assert_eq!(checked, 22, "the semantic-team fixture tree changed size");
}
