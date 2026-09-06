//! Corpus-driven inspect tests. Expected values are read from the frozen
//! jsonl files at runtime; they are not copied into this file.

use ctxpect_cli::{
    RedactRoots, Value, canonical_json, inspect, parse, parse_args, redact_json_envelope,
    with_snapshot_digest,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ctxpect"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn load_jsonl(path: &Path) -> Vec<Value> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse(line).unwrap_or_else(|error| panic!("parse {line}: {error}")))
        .collect()
}

fn inspect_project(project: &Path, extra: &[&str]) -> (i32, Value) {
    let mut args = vec![
        "inspect",
        "--json",
        "--offline",
        "--project",
        project.to_str().expect("utf8"),
    ];
    args.extend_from_slice(extra);
    let output = Command::new(bin())
        .args(&args)
        .output()
        .expect("spawn ctxpect");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    (code, json)
}

fn first_result(json: &Value) -> &Value {
    json.get("results")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .expect("results[0]")
}

const AXES: &[&str] = &[
    "claim_kind",
    "coverage",
    "knowledge_status",
    "lifecycle_stage",
    "precision",
    "provenance",
    "truth_state",
];

fn assert_output_and_claim(id: &str, result: &Value, row: &Value) {
    let expected_output = row.get("expected_output").expect("expected_output");
    for key in [
        "included",
        "native_paths_used",
        "truth_state",
        "unknown_reason_code",
    ] {
        assert_eq!(
            result.get(key),
            expected_output.get(key),
            "{id} expected_output.{key}"
        );
    }
    let expected_claim = row.get("expected_claim").expect("expected_claim");
    let claim = result.get("claim").expect("claim");
    for axis in AXES {
        assert_eq!(
            claim.get(axis),
            expected_claim.get(axis),
            "{id} claim.{axis}"
        );
    }
    if expected_claim.get("unknown_reason_code").is_some() {
        assert_eq!(
            claim.get("unknown_reason_code"),
            expected_claim.get("unknown_reason_code"),
            "{id} claim.unknown_reason_code"
        );
    }
}

#[test]
fn corpus_macos_instruction_cases_match_expected_output_and_claim() {
    let root = repo_root();
    let path = root.join("acceptance/corpus/development/static/codex__cli.jsonl");
    let wanted = [
        "dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:positive",
        "dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:negative",
        "dev:static:codex/0.147.0/cli/macos-27-arm64:include:00",
    ];
    let mut seen = 0usize;
    for row in load_jsonl(&path) {
        let id = row.get("id").and_then(Value::as_str).unwrap_or_default();
        if !wanted.contains(&id) {
            continue;
        }
        seen += 1;
        let input = root.join(
            row.get("input_path")
                .and_then(Value::as_str)
                .expect("input_path"),
        );
        let (code, json) = inspect_project(&input, &[]);
        let result = first_result(&json);
        assert_output_and_claim(id, result, &row);
        let truth = result.get("truth_state").and_then(Value::as_str);
        match truth {
            Some("present") => assert_eq!(code, 0, "{id}"),
            Some("absent") => assert_eq!(code, 2, "{id}"),
            Some("indeterminate") => assert_eq!(code, 3, "{id}"),
            other => panic!("{id} unexpected truth_state {other:?}"),
        }
    }
    assert_eq!(seen, 3, "expected the three related macos cases");
}

#[test]
fn corpus_unknown_honesty_cells_match_expected_reason() {
    let root = repo_root();
    let path = root.join("acceptance/corpus/development/static/unknown-honesty-cells.jsonl");
    let mut seen = 0usize;
    for row in load_jsonl(&path) {
        if row.get("family_id").and_then(Value::as_str) != Some("codex") {
            continue;
        }
        if row.get("capability_id").and_then(Value::as_str) != Some("instructions") {
            continue;
        }
        seen += 1;
        let id = row.get("id").and_then(Value::as_str).unwrap_or_default();
        let input = root.join(
            row.get("input_path")
                .and_then(Value::as_str)
                .expect("input_path"),
        );
        let os_lane = row.get("os_lane").and_then(Value::as_str).expect("os_lane");
        let surface = row.get("surface").and_then(Value::as_str).expect("surface");
        let version = row.get("version").and_then(Value::as_str).expect("version");
        let extra = [
            "--os-lane",
            os_lane,
            "--surface",
            surface,
            "--version",
            version,
        ];
        let (code, json) = inspect_project(&input, &extra);
        assert_eq!(code, 3, "{id}");
        let result = first_result(&json);
        assert_output_and_claim(id, result, &row);
        assert_eq!(
            result.get("truth_state").and_then(Value::as_str),
            Some("indeterminate"),
            "{id}"
        );
    }
    assert_eq!(seen, 8, "expected eight codex instructions honesty cells");
}

#[test]
fn corpus_macos_and_negative_envelopes_are_unchanged_by_redact_boundary() {
    let root = repo_root();
    let path = root.join("acceptance/corpus/development/static/codex__cli.jsonl");
    let wanted = [
        "dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:positive",
        "dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:negative",
        "dev:static:codex/0.147.0/cli/macos-27-arm64:include:00",
    ];
    let mut seen = 0usize;
    for row in load_jsonl(&path) {
        let id = row.get("id").and_then(Value::as_str).unwrap_or_default();
        if !wanted.contains(&id) {
            continue;
        }
        seen += 1;
        let input = root.join(
            row.get("input_path")
                .and_then(Value::as_str)
                .expect("input_path"),
        );
        let parsed = parse_args([
            "ctxpect",
            "inspect",
            "--offline",
            "--project",
            input.to_str().expect("utf8"),
        ])
        .expect("parse");
        let roots = RedactRoots::from_inspect_args(&parsed);
        let report = inspect(parsed)
            .unwrap_or_else(|err| panic!("inspect {id}: {} {}", err.code(), err.message()));
        let before = with_snapshot_digest(report.envelope.clone());
        let after = redact_json_envelope(report.envelope, &roots);
        assert_eq!(
            canonical_json(&before),
            canonical_json(&after),
            "{id} redact boundary changed corpus JSON"
        );
        assert_eq!(
            before.get("snapshot_digest"),
            after.get("snapshot_digest"),
            "{id} digest moved under redact boundary"
        );
        let warnings = after
            .get("warnings")
            .and_then(Value::as_array)
            .expect("warnings");
        assert!(
            warnings.iter().all(|item| {
                item.get("code").and_then(Value::as_str) != Some("redaction.residual_absolute_path")
            }),
            "{id} residual warning on corpus"
        );
    }
    assert_eq!(seen, 3, "expected the three related macos cases");
}
