//! Negative cases for L01–L11 product loops.

use ctxpect_cli::{canonical_json, persist_inspect, parse, Store, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PASS_LAYERS: &str = r#"[{"layer":"organization","mode":"enforceable","rules":[]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]"#;

const LIVE_EXCEPTION: &str = r#"{"exception_id":"ex-planted","requester":"operator","scope":"project","state":"approved","expires_at":4102444800,"approver":"enrolled-out-of-band"}"#;

fn plant_pass_policy_and_live_exception(store: &Path) {
    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(store.join("policies/active.json"), PASS_LAYERS).unwrap();
    fs::create_dir_all(store.join("exceptions")).unwrap();
    fs::write(store.join("exceptions/ex-planted.json"), LIVE_EXCEPTION).unwrap();
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ctxpect"))
}

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
            "cx-loop-{label}-{}-{}",
            std::process::id(),
            nanos % 1_000_000
        ));
        fs::create_dir_all(&path).expect("mkdir");
        Scratch {
            path: fs::canonicalize(&path).expect("canon"),
        }
    }

    fn write(&self, rel: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let target = self.path.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(&target, contents).expect("write");
        target
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run(args: &[&str]) -> (i32, Value, String) {
    let output = Command::new(bin()).args(args).output().expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|_| {
        parse(&format!(
            "{{\"parse_failed\":true,\"stdout\":{}}}",
            serde_quote(&stdout)
        ))
        .unwrap_or(Value::Null)
    });
    let _ = stderr;
    (code, json, stdout)
}

fn serde_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

fn err_code(json: &Value) -> Option<&str> {
    json.pointer(&["error", "code"]).and_then(Value::as_str)
}

#[test]
fn l01_residue_is_not_installed_unknown_version_fail_closed_no_default_home() {
    let scratch = Scratch::new("l01");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write(".codex/config.toml", "model = \"x\"\n");
    let project = scratch.path.to_str().unwrap();
    let (code, json, _) = run(&["collect", "--json", "--project", project]);
    assert_eq!(code, 0, "{json:?}");
    // config_residue_is_installed / home_default_scan are constant honesty
    // flags on the envelope; they are not live counterexamples (L07 style).
    let residue = json.get("config_residue").and_then(Value::as_array).unwrap_or(&[]);
    assert!(
        residue.iter().any(|item| {
            item.get("path")
                .and_then(Value::as_str)
                .is_some_and(|p| p.contains(".codex"))
                && item.get("reason_code").and_then(Value::as_str) == Some("config_residue_only")
                && item.get("installed").and_then(Value::as_bool) == Some(false)
        }),
        "{json:?}"
    );
    assert_eq!(json.get("scanned_home").and_then(Value::as_bool), Some(false));

    let (code, json, _) = run(&[
        "inspect",
        "--json",
        "--project",
        project,
        "--version",
        "0.99.0",
    ]);
    assert_eq!(code, 3);
    assert_eq!(
        json.get("results")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|r| r.get("unknown_reason_code"))
            .and_then(Value::as_str),
        Some("unsupported_harness_version")
    );
}

#[test]
fn l02_shell_injection_and_missing_preflight_id() {
    let scratch = Scratch::new("l02");
    scratch.write("AGENTS.md", "hello\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "preflight",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--files",
        "$(rm -rf /)",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("launch.injection"));
    let (code, json, out) = run(&[
        "launch",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--execute",
    ]);
    assert_eq!(code, 1, "{out}");
    let code_s = err_code(&json).unwrap_or("");
    assert!(
        code_s == "launch.execute_refused" || code_s == "launch.preflight_required",
        "{json:?}"
    );
    let (code, json, _) = run(&[
        "launch",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
    ]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("launch.preflight_required"));
}

#[test]
fn l03_apply_without_policy_authorization() {
    let scratch = Scratch::new("l03");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--desired",
        "PWNED\n",
        "--target",
        "AGENTS.md",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "one\n"
    );

    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(
        store.join("policies/active.json"),
        r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"deny"}]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]"#,
    )
    .unwrap();
    let (code, json, out) = run(&[
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--desired",
        "PWNED\n",
        "--target",
        "AGENTS.md",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.denied"));
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "one\n"
    );
}

#[test]
fn l04_unknown_cannot_baseline() {
    let scratch = Scratch::new("l04");
    scratch.write("AGENTS.md", "hello\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "inspect",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
    ]);
    assert!(code == 0 || code == 3, "{out}");
    let id = json
        .get("formal_receipt_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!id.is_empty(), "{json:?}");
    let (code, json, _) = run(&[
        "diff",
        "--json",
        "--store",
        store_s,
        "--a",
        id,
        "--b",
        id,
    ]);
    assert_eq!(code, 0, "{json:?}");
    // byte_equality_is_verified / verified are constant honesty flags on the
    // diff envelope; they are not live counterexamples (L07 style).
    assert_eq!(
        json.get("baseline_allowed").and_then(Value::as_bool),
        Some(false)
    );
    let unknown = json.get("unknown_cells").and_then(Value::as_i64).unwrap_or(0);
    assert!(unknown > 0, "{json:?}");
}

#[test]
fn l04_relabel_is_not_migration() {
    let scratch = Scratch::new("l04relabel");
    let store = Store::open(&scratch.path.join("store")).expect("open");
    let fake = parse(
        r#"{"schema":"ctxpect-receipt-v1","receipt_kind":"development-snapshot","snapshot_digest":"x"}"#,
    )
    .expect("parse");
    let err = persist_inspect(&store, &fake, "one-shot").expect_err("relabel");
    assert_eq!(err.code(), "receipt.relabel_forbidden");
    match store.list_receipts().expect("index") {
        Value::Array(items) => assert!(items.is_empty(), "{items:?}"),
        other => panic!("expected empty index, got {other:?}"),
    }
}

#[test]
fn l05_secret_and_replay_and_transport_not_verified() {
    let scratch = Scratch::new("l05");
    scratch.write("AGENTS.md", "x\n");
    let dest = scratch.path.join("syncdest");
    fs::create_dir_all(&dest).unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let dest_s = dest.to_str().unwrap();
    let project = scratch.path.to_str().unwrap();
    let (code, json, out) = run(&[
        "sync",
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--dest",
        dest_s,
        "--id",
        "b1",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert!(!dest.join("current.json").exists());
    assert!(!dest.join("applied.jsonl").exists());

    plant_pass_policy_and_live_exception(&store);
    let (code, json, out) = run(&[
        "sync",
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--dest",
        dest_s,
        "--id",
        "b1",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(
        json.get("transport_success_is_verified")
            .and_then(Value::as_bool),
        Some(false)
    );
    let (code, json, _) = run(&[
        "sync",
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--dest",
        dest_s,
        "--id",
        "b1",
    ]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("sync.replay"));
}

#[test]
fn l06_personal_relax_and_detect_only() {
    let scratch = Scratch::new("l06");
    scratch.write(
        "policy.json",
        r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"deny"}]},{"layer":"user","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"allow"}]}]"#,
    );
    let (code, json, _) = run(&[
        "policy",
        "eval",
        "--json",
        "--from",
        scratch.path.join("policy.json").to_str().unwrap(),
    ]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("policy.personal_cannot_relax_required"));
}

// L07 uncovered: there is no asset copy executor, so the three named
// counterexamples (rewrite APM evaluator, dual authority, missing-license
// copy still executing) cannot be constructed as refusals. `assets --json`
// reports `assets.copy_unimplemented` instead of a self-asserted block flag.

#[test]
fn l08_import_does_not_invent_and_delete_drops_insights() {
    let scratch = Scratch::new("l08");
    scratch.write(
        "sess.json",
        r#"{"events":[{"type":"user","text":"hi"},{"type":"mystery","text":"invented?"}]}"#,
    );
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let from = scratch.path.join("sess.json");
    let from_s = from.to_str().unwrap();
    let (code, json, out) = run(&[
        "import",
        "--json",
        "--store",
        store_s,
        "--from",
        from_s,
        "--session",
        "s1",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert!(!store.join("sessions/s1.json").exists());

    plant_pass_policy_and_live_exception(&store);
    let (code, json, out) = run(&[
        "import",
        "--json",
        "--store",
        store_s,
        "--from",
        from_s,
        "--session",
        "s1",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    let timeline = json.get("timeline").and_then(Value::as_array).unwrap_or(&[]);
    assert_eq!(timeline.len(), 1, "{json:?}");
    assert_eq!(
        timeline[0].get("type").and_then(Value::as_str),
        Some("user")
    );
    let unknown = json.get("unknown").and_then(Value::as_array).unwrap_or(&[]);
    assert_eq!(unknown.len(), 1, "{json:?}");
    assert_eq!(
        json.pointer(&["occupancy", "status"]).and_then(Value::as_str),
        Some("unknown")
    );
    assert_eq!(
        json.pointer(&["occupancy", "reason_code"])
            .and_then(Value::as_str),
        Some("current_occupancy_not_reported")
    );
    let (code, json, _) = run(&[
        "sessions",
        "--json",
        "--store",
        store_s,
        "--session",
        "s1",
        "--reason",
        "delete",
    ]);
    assert_eq!(code, 0, "{json:?}");
    // insights_invalidated as a hardcoded true is not a live counterexample;
    // the insight document must actually be gone.
    assert!(!store.join("sessions/s1.json").exists());
    assert!(!store.join("insights/s1.json").exists());
}

#[test]
fn l09_advisor_requires_consent_and_does_not_unlock() {
    let (code, json, _) = run(&["advisor", "--json", "--text", "redacted"]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("advisor.consent_required"));
    let (code, json, _) = run(&[
        "advisor",
        "--json",
        "--reason",
        "consent",
        "--text",
        "ack",
    ]);
    assert_eq!(code, 0, "{json:?}");
    // is_claim / unlocks_treatment are constant honesty flags; this slice
    // cannot construct a candidate that enters Claim or unlocks Treatment.
    assert_eq!(
        json.get("kind").and_then(Value::as_str),
        Some("advisor-suggestion")
    );
}

#[test]
fn l10_single_pair_not_causal_and_n_locked() {
    let scratch = Scratch::new("l10");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, _) = run(&["experiment", "--json", "--n", "1", "--id", "e1"]);
    assert_eq!(code, 0, "{json:?}");
    assert_eq!(
        json.get("decision").and_then(Value::as_str),
        Some("inconclusive")
    );
    assert_eq!(json.get("causal").and_then(Value::as_bool), Some(false));
    let (code, json, out) = run(&[
        "experiment",
        "--json",
        "--n",
        "4",
        "--id",
        "e-lock",
        "--store",
        store_s,
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    let (code, json, out) = run(&[
        "experiment",
        "--json",
        "--n",
        "8",
        "--id",
        "e-lock",
        "--store",
        store_s,
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("effect.n_locked"));
}

#[test]
fn l11_ci_indeterminate_is_not_exit_0() {
    let scratch = Scratch::new("l11");
    scratch.write("AGENTS.md", "hello\n");
    let (code, json, _) = run(&[
        "ci",
        "--json",
        "--project",
        scratch.path.to_str().unwrap(),
        "--version",
        "0.99.0",
    ]);
    assert_eq!(code, 3, "{json:?}");
    // indeterminate_is_zero is a constant honesty flag; the live
    // counterexample is the non-zero CI exit for unknown version.
}

#[test]
fn doctor_does_not_treat_unknown_as_severity() {
    let scratch = Scratch::new("doc");
    scratch.write("AGENTS.md", "hello\n");
    let (code, json, _) = run(&[
        "doctor",
        "--json",
        "--project",
        scratch.path.to_str().unwrap(),
    ]);
    assert!(code == 0 || code == 3, "{json:?}");
    let findings = json.get("findings").and_then(Value::as_array).unwrap_or(&[]);
    for item in findings {
        let sev = item.get("severity").and_then(Value::as_str).unwrap_or("");
        assert!(sev == "confirmed" || sev == "suspected", "{item:?}");
        assert_eq!(
            item.get("unknown_is_severity").and_then(Value::as_bool),
            Some(false)
        );
    }
}

#[test]
fn policy_show_matches_apply_decision() {
    let scratch = Scratch::new("pol");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "policy",
        "show",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
    ]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(
        json.get("mutation_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json.get("reason_code").and_then(Value::as_str),
        Some("policy.unknown")
    );
    assert_eq!(json.get("verdict").and_then(Value::as_str), Some("unknown"));
    let (acode, ajson, _) = run(&[
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--desired",
        "PWNED\n",
        "--target",
        "AGENTS.md",
    ]);
    assert_eq!(acode, 1);
    assert_eq!(
        err_code(&ajson),
        json.get("reason_code").and_then(Value::as_str)
    );
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "one\n"
    );

    plant_pass_policy_and_live_exception(&store);
    let (code, json, out) = run(&[
        "policy",
        "show",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(
        json.get("mutation_allowed").and_then(Value::as_bool),
        Some(true)
    );
    let (acode, ajson, out) = run(&[
        "apply",
        "--json",
        "--project",
        project,
        "--store",
        store_s,
        "--desired",
        "ok\n",
        "--target",
        "AGENTS.md",
    ]);
    assert_eq!(acode, 0, "{out} {ajson:?}");
    let post = ajson
        .get("post_receipt_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!post.is_empty(), "{ajson:?}");
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "ok\n"
    );
}

#[test]
fn rollback_without_authorization_does_not_write() {
    let scratch = Scratch::new("rb");
    scratch.write("AGENTS.md", "one\n");
    let (code, json, out) = run(&[
        "rollback",
        "--json",
        "--project",
        scratch.path.to_str().unwrap(),
        "--store",
        scratch.path.join("store").to_str().unwrap(),
        "--id",
        "tx_none",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "one\n"
    );
}

#[test]
fn exception_approve_cannot_self_attest_role() {
    let scratch = Scratch::new("ex");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "exception",
        "request",
        "--json",
        "--store",
        store_s,
        "--id",
        "ex-1",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert!(!store.join("exceptions/ex-1.json").exists());

    fs::create_dir_all(store.join("exceptions")).unwrap();
    let planted = r#"{"exception_id":"ex-1","requester":"user","scope":"project","state":"requested","expires_at":4102444800,"approver":null}"#;
    fs::write(store.join("exceptions/ex-1.json"), planted).unwrap();
    let (code, json, out) = run(&[
        "exception",
        "approve",
        "--json",
        "--store",
        store_s,
        "--id",
        "ex-1",
        "--role",
        "lead",
        "--actor",
        "same-user",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(
        err_code(&json),
        Some("exception.identity_source_uncovered")
    );
    assert_eq!(
        fs::read_to_string(store.join("exceptions/ex-1.json")).unwrap(),
        planted
    );
}

#[test]
fn standard_signature_rejects_payload_and_mac_tamper() {
    let scratch = Scratch::new("std");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "standard",
        "publish",
        "--json",
        "--store",
        store_s,
        "--id",
        "std-local",
        "--text",
        "hello",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("signed").and_then(Value::as_bool), Some(true));
    let path = store.join("standards/std-local.json");
    let original = fs::read_to_string(&path).unwrap();
    let mut payload_tamper = parse(&original).unwrap();
    if let Value::Object(map) = &mut payload_tamper {
        map.insert("payload_digest".into(), Value::Str("00".repeat(32)));
    }
    fs::write(&path, canonical_json(&payload_tamper)).unwrap();
    let (code, json, out) = run(&[
        "standard",
        "status",
        "--json",
        "--store",
        store_s,
        "--id",
        "std-local",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("standard.signature_mismatch"));

    fs::write(&path, &original).unwrap();
    let mut sig_tamper = parse(&original).unwrap();
    if let Value::Object(map) = &mut sig_tamper
        && let Some(Value::Object(sig)) = map.get_mut("signature")
    {
        sig.insert("value".into(), Value::Str("aa".repeat(32)));
    }
    fs::write(&path, canonical_json(&sig_tamper)).unwrap();
    let (code, json, out) = run(&[
        "standard",
        "status",
        "--json",
        "--store",
        store_s,
        "--id",
        "std-local",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("standard.signature_mismatch"));
}

struct ChildGuard {
    child: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn child(&mut self) -> &mut std::process::Child {
        self.child.as_mut().expect("child")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn http_call(addr: &str, method: &str, path: &str, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let host = addr.rsplit_once(':').map(|(_, p)| p).unwrap_or("0");
    let mut stream = TcpStream::connect(addr).expect("connect");
    let length = if method == "GET" {
        String::new()
    } else {
        format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        )
    };
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{host}\r\nX-Ctxpect-Client: desktop\r\n{length}Connection: close\r\n\r\n{body}"
    );
    stream.write_all(req.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    let status = buf
        .split_whitespace()
        .nth(1)
        .and_then(|item| item.parse().ok())
        .unwrap_or(0);
    (status, buf)
}

fn http_json(raw: &str) -> Value {
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").trim();
    parse(body).unwrap_or(Value::Null)
}

#[test]
fn daemon_health_and_inspect_via_localhost() {
    use std::thread;
    use std::time::Duration;

    let scratch = Scratch::new("http");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let mut child = ChildGuard::new(
        Command::new(bin())
            .args([
                "daemon",
                "start",
                "--project",
                scratch.path.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--listen",
                "127.0.0.1:0",
            ])
            .spawn()
            .expect("daemon"),
    );
    let addr_file = store.join("daemon.addr");
    let mut listen = String::new();
    for _ in 0..80 {
        if child.child().try_wait().expect("try_wait").is_some() {
            panic!("daemon exited before bind");
        }
        if let Ok(text) = fs::read_to_string(&addr_file) {
            let text = text.trim();
            if !text.is_empty() {
                listen = text.to_string();
                break;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(!listen.is_empty(), "daemon did not write daemon.addr");
    let mut health = String::new();
    for _ in 0..80 {
        if child.child().try_wait().expect("try_wait").is_some() {
            panic!("daemon exited before health: {health}");
        }
        let (status, raw) = http_call(&listen, "GET", "/api/v1/health", "");
        health = raw;
        if status == 200 && health.contains("ok") {
            let settings_before = fs::read_to_string(store.join("settings.json")).unwrap();

            let (pstatus, praw) = http_call(&listen, "GET", "/api/v1/policy", "");
            assert_eq!(pstatus, 200, "{praw}");
            let policy = http_json(&praw);
            assert_eq!(
                policy.get("mutation_allowed").and_then(Value::as_bool),
                Some(false),
                "{praw}"
            );
            let policy_reason = policy
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            assert_eq!(policy_reason, "policy.unknown", "{praw}");

            let apply_body = r#"{"approved":true,"desired":"PWNED\n","target":"AGENTS.md"}"#;
            let (astatus, araw) = http_call(&listen, "POST", "/api/v1/apply", apply_body);
            assert_eq!(astatus, 400, "{araw}");
            let apply_json = http_json(&araw);
            assert_eq!(err_code(&apply_json), Some(policy_reason.as_str()), "{araw}");
            assert_eq!(
                fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
                "hello\n"
            );

            let (sstatus, sraw) = http_call(
                &listen,
                "PUT",
                "/api/v1/settings",
                r#"{"privacy_mode":"pwned"}"#,
            );
            assert_eq!(sstatus, 400, "{sraw}");
            assert!(sraw.contains("policy.unknown"), "{sraw}");
            assert_eq!(
                fs::read_to_string(store.join("settings.json")).unwrap(),
                settings_before
            );

            let (istatus, iraw) = http_call(
                &listen,
                "POST",
                "/api/v1/sessions/import",
                r#"{"events":[{"type":"user","text":"hi"}]}"#,
            );
            assert_eq!(istatus, 400, "{iraw}");
            assert!(iraw.contains("policy.unknown"), "{iraw}");
            let session_ids = fs::read_dir(store.join("sessions"))
                .map(|dir| dir.count())
                .unwrap_or(0);
            assert_eq!(session_ids, 0, "sessions written without authorization");

            let (estatus, eraw) = http_call(&listen, "POST", "/api/v1/exceptions", "{}");
            assert_eq!(estatus, 400, "{eraw}");
            assert!(eraw.contains("policy.unknown"), "{eraw}");
            assert!(!store.join("exceptions/ex-api.json").exists());

            let (rstatus, rraw) = http_call(
                &listen,
                "POST",
                "/api/v1/rollback",
                r#"{"tx_id":"tx_none","target":"AGENTS.md"}"#,
            );
            assert_eq!(rstatus, 400, "{rraw}");
            assert!(rraw.contains("policy.unknown"), "{rraw}");
            assert_eq!(
                fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
                "hello\n"
            );

            plant_pass_policy_and_live_exception(&store);
            let (pstatus, praw) = http_call(&listen, "GET", "/api/v1/policy", "");
            let policy_ok = http_json(&praw);
            assert_eq!(pstatus, 200, "{praw}");
            assert_eq!(
                policy_ok.get("mutation_allowed").and_then(Value::as_bool),
                Some(true),
                "{praw}"
            );

            let apply_ok_body = r#"{"desired":"from-http\n","target":"AGENTS.md"}"#;
            let (astatus, araw) = http_call(&listen, "POST", "/api/v1/apply", apply_ok_body);
            assert_eq!(astatus, 200, "{araw}");
            let apply_ok = http_json(&araw);
            let http_post = apply_ok
                .get("post_receipt_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(!http_post.is_empty(), "{araw}");
            assert_eq!(
                fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
                "from-http\n"
            );

            let (ccode, cjson, cout) = run(&[
                "apply",
                "--json",
                "--project",
                scratch.path.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--desired",
                "from-cli\n",
                "--target",
                "AGENTS.md",
            ]);
            assert_eq!(ccode, 0, "{cout} {cjson:?}");
            let cli_post = cjson
                .get("post_receipt_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(!cli_post.is_empty(), "{cjson:?}");
            assert!(
                store.join(format!("receipts/{http_post}.json")).is_file(),
                "http post receipt missing: {http_post}"
            );
            assert!(
                store.join(format!("receipts/{cli_post}.json")).exists(),
                "cli post receipt missing: {cli_post}"
            );
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("daemon health not reachable at {listen}: {health}");
}

#[allow(dead_code)]
fn _project(p: &Path) -> &Path {
    p
}
