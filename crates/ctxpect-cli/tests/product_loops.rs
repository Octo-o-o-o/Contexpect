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

/// Run the binary holding an enrolled principal secret. The secret goes
/// through the environment, exactly as the product requires.
fn run_as(secret: &str, args: &[&str]) -> (i32, Value, String) {
    let output = Command::new(bin())
        .args(args)
        .env("CTXPECT_PRINCIPAL_SECRET", secret)
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|_| {
        parse(&format!(
            "{{\"parse_failed\":true,\"stdout\":{}}}",
            serde_quote(&stdout)
        ))
        .unwrap_or(Value::Null)
    });
    (code, json, stdout)
}

/// Enroll `alice` (requester) and `carol` (approver) in a project registry.
/// The digests are computed by the product's own enrollment function, so the
/// fixture cannot drift from the code that checks it.
fn enroll_principals(project: &Path) {
    fs::create_dir_all(project.join(".ctxpect")).unwrap();
    let registry = format!(
        r#"{{"schema":"ctxpect-principals-v1","principals":[{{"principal_id":"alice","roles":["requester"],"key_digest":"{}"}},{{"principal_id":"carol","roles":["approver"],"key_digest":"{}"}}]}}"#,
        ctxpect_policy::enrollment_digest("alice", b"alice-secret"),
        ctxpect_policy::enrollment_digest("carol", b"carol-secret"),
    );
    fs::write(project.join(".ctxpect/principals.json"), registry).unwrap();
    fs::write(project.join(".ctxpect/policy.json"), PASS_LAYERS).unwrap();
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

    // Persisting an experiment result is a store mutation. Without authority
    // it is refused and nothing is written.
    let (code, json, out) = run(&[
        "experiment", "--json", "--n", "4", "--id", "e-lock", "--store", store_s,
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert!(!store.join("experiments/e-lock.json").exists());

    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);
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
    let project_s = scratch.path.to_str().unwrap();
    enroll_principals(&scratch.path);

    // Naming yourself is not identity: no principal, no request.
    let (code, json, out) = run(&[
        "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
        "ex-1",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("principal.absent"));
    assert!(!store.join("exceptions/ex-1.json").exists());

    // Holding the wrong secret is not identity either.
    let (code, json, out) = run_as(
        "not-the-enrolled-secret",
        &[
            "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-1", "--principal", "alice",
        ],
    );
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("principal.secret_mismatch"));
    assert!(!store.join("exceptions/ex-1.json").exists());

    // An enrolled requester may request.
    let (code, json, out) = run_as(
        "alice-secret",
        &[
            "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-1", "--principal", "alice",
        ],
    );
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("requester").and_then(Value::as_str), Some("alice"));

    // `--role` / `--actor` remain caller-attested and grant nothing: alice is
    // enrolled as a requester only, and claiming a role does not change that.
    let (code, json, out) = run_as(
        "alice-secret",
        &[
            "exception", "approve", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-1", "--principal", "alice", "--role", "lead", "--actor", "same-user",
        ],
    );
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("exception.approver_role_required"));
    let after_denied = fs::read_to_string(store.join("exceptions/ex-1.json")).unwrap();
    assert!(after_denied.contains("\"requested\""), "{after_denied}");

    // A mutation is still fail-closed while nothing is approved.
    let (code, json, out) = run(&[
        "standard", "publish", "--json", "--store", store_s, "--project", project_s, "--id",
        "std-x", "--text", "hello",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.approval_required"));

    // An enrolled approver, who is not the requester, can approve.
    let (code, json, out) = run_as(
        "carol-secret",
        &[
            "exception", "approve", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-1", "--principal", "carol",
        ],
    );
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("state").and_then(Value::as_str), Some("approved"));
    assert_eq!(
        json.pointer(&["decided_by", "principal_id"]).and_then(Value::as_str),
        Some("carol")
    );
    // The decision is recorded as a local principal, never as an org identity.
    assert_eq!(
        json.pointer(&["decided_by", "org_identity"]).and_then(Value::as_bool),
        Some(false)
    );

    // That approval is what unblocks the mutation — the bootstrap deadlock is
    // gone, and the gate itself is not.
    let (code, json, out) = run(&[
        "standard", "publish", "--json", "--store", store_s, "--project", project_s, "--id",
        "std-x", "--text", "hello",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");

    // Revoking closes it again.
    let (code, _json, _out) = run_as(
        "carol-secret",
        &[
            "exception", "revoke", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-1", "--principal", "carol",
        ],
    );
    assert_eq!(code, 0);
    let (code, json, out) = run(&[
        "standard", "publish", "--json", "--store", store_s, "--project", project_s, "--id",
        "std-y", "--text", "hello",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.approval_required"));
}

#[test]
fn standard_signature_rejects_payload_and_mac_tamper() {
    let scratch = Scratch::new("std");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    // Publishing a standard is a store mutation and needs the same authority
    // as any other; this test is about signatures, so grant it up front.
    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);
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

/// Like [`http_call`], with the request's headers under the caller's control.
///
/// `http_call` always sends a well-formed loopback `Host`, which is exactly
/// what the origin checks are supposed to accept; testing what they reject
/// needs a way to send something else.
fn http_call_headers(addr: &str, path: &str, extra_headers: &str) -> (u16, String) {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let mut stream = TcpStream::connect(addr).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\n{extra_headers}Connection: close\r\n\r\n");
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

/// Start a daemon on an ephemeral port and return it with its address.
fn start_daemon(scratch: &Scratch, store: &Path) -> (ChildGuard, String) {
    use std::thread;
    use std::time::Duration;

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
    for _ in 0..80 {
        let (status, raw) = http_call(&listen, "GET", "/api/v1/health", "");
        if status == 200 && raw.contains("ok") {
            return (child, listen);
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("daemon never became healthy");
}

/// Same as [`start_daemon`], with a UI root so the static file server runs.
fn start_daemon_with_ui(scratch: &Scratch, store: &Path, ui_root: &Path) -> (ChildGuard, String) {
    use std::thread;
    use std::time::Duration;

    let mut child = ChildGuard::new(
        Command::new(bin())
            .args([
                "daemon",
                "start",
                "--project",
                scratch.path.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--ui-root",
                ui_root.to_str().unwrap(),
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
    for _ in 0..80 {
        let (status, raw) = http_call(&listen, "GET", "/api/v1/health", "");
        if status == 200 && raw.contains("ok") {
            return (child, listen);
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("daemon never became healthy");
}

fn get_json(listen: &str, path: &str) -> (u16, Value) {
    let (status, raw) = http_call(listen, "GET", path, "");
    (status, http_json(&raw))
}

/// `/monitor`, `/sync`, `/team/compliance` and `/care-plan/:id` used to answer
/// with fixed constants. Each assertion below would pass against those
/// constants only by accident, and the staleness flip cannot pass at all.
#[test]
fn read_only_endpoints_report_computed_state_not_constants() {
    let scratch = Scratch::new("endpoints");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon(&scratch, &store);

    // ---- Before any inspect: unknown, and unknown is not `false` or `0`. ----
    let (status, monitor) = get_json(&listen, "/api/v1/monitor");
    assert_eq!(status, 200);
    assert_eq!(
        monitor.pointer(&["staleness", "status"]).and_then(Value::as_str),
        Some("unknown"),
        "{monitor:?}"
    );
    assert_eq!(
        monitor.get("current_receipt_id").cloned(),
        Some(Value::Null),
        "{monitor:?}"
    );

    let (status, compliance) = get_json(&listen, "/api/v1/team/compliance");
    assert_eq!(status, 200);
    // Absent evidence must not be reported as a clean zero.
    assert_eq!(compliance.get("unknown"), Some(&Value::Null), "{compliance:?}");
    assert_eq!(compliance.get("drift"), Some(&Value::Null), "{compliance:?}");
    assert_eq!(
        compliance.pointer(&["freshness", "status"]).and_then(Value::as_str),
        Some("unknown"),
        "{compliance:?}"
    );
    // The privacy invariants hold regardless.
    assert_eq!(compliance.get("redacted").and_then(Value::as_bool), Some(true));
    assert_eq!(
        compliance.get("member_bodies_included").and_then(Value::as_bool),
        Some(false)
    );

    // A care plan for a Receipt that does not exist is an error, not a plan.
    let (status, plan) = get_json(&listen, "/api/v1/care-plan/f_anything");
    assert_eq!(status, 400, "{plan:?}");
    assert_eq!(err_code(&plan), Some("api.no_current_receipt"));

    // ---- After an inspect. ----
    let (status, _) = http_call(
        &listen,
        "POST",
        "/api/v1/inspect",
        &format!(
            r#"{{"project":{},"harness":"codex"}}"#,
            serde_quote(scratch.path.to_str().unwrap())
        ),
    );
    assert_eq!(status, 200);

    // Evidence is unchanged, so the Receipt is current — and something was
    // actually compared to reach that verdict.
    let (_, monitor) = get_json(&listen, "/api/v1/monitor");
    assert_eq!(
        monitor.pointer(&["staleness", "status"]).and_then(Value::as_str),
        Some("current"),
        "{monitor:?}"
    );
    assert!(
        monitor
            .pointer(&["staleness", "compared"])
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 0,
        "a `current` verdict with nothing compared is a constant: {monitor:?}"
    );

    // An unknown finding id is not found, rather than answered generically.
    let (status, plan) = get_json(&listen, "/api/v1/care-plan/f_not_a_real_finding");
    assert_eq!(status, 400, "{plan:?}");
    assert_eq!(err_code(&plan), Some("api.not_found"));

    // A real finding's plan is read off that finding.
    let (_, doctor) = get_json(&listen, "/api/v1/doctor");
    let finding = doctor
        .get("findings")
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
        .expect("at least one finding");
    let finding_id = finding
        .get("finding_id")
        .and_then(Value::as_str)
        .expect("finding_id");
    let (status, plan) = get_json(&listen, &format!("/api/v1/care-plan/{finding_id}"));
    assert_eq!(status, 200, "{plan:?}");
    assert_eq!(
        plan.get("finding_id").and_then(Value::as_str),
        Some(finding_id)
    );
    assert_eq!(plan.get("title").cloned(), finding.get("title").cloned());
    assert_eq!(
        plan.get("treatment_locked").cloned(),
        finding.pointer(&["treatment", "locked"]).cloned(),
        "the plan must mirror this finding's treatment: {plan:?}"
    );
    assert_eq!(
        plan.get("authority").cloned(),
        finding.pointer(&["placement", "authority"]).cloned()
    );

    // ---- Change the project: the verdict must flip. ----
    scratch.write("AGENTS.md", "hello changed\n");
    let (_, monitor) = get_json(&listen, "/api/v1/monitor");
    assert_eq!(
        monitor.pointer(&["staleness", "status"]).and_then(Value::as_str),
        Some("stale"),
        "changed evidence must make the Receipt stale: {monitor:?}"
    );
    let changed: Vec<&str> = monitor
        .pointer(&["staleness", "changed_evidence"])
        .and_then(Value::as_array)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(changed.contains(&"AGENTS.md"), "{changed:?}");

    // ---- Sync reports its real state and stays honest about E2EE. ----
    let (status, sync) = get_json(&listen, "/api/v1/sync");
    assert_eq!(status, 200);
    assert_eq!(
        sync.get("encryption").and_then(Value::as_str),
        Some("unavailable"),
        "E2EE is not implemented and must not be reported as available"
    );
    assert_eq!(
        sync.get("transport_success_is_verified").and_then(Value::as_bool),
        Some(false)
    );
    // One Receipt exists by now, so this count is computed, not a literal 0.
    assert_eq!(
        sync.get("syncable_receipts").and_then(Value::as_i64),
        Some(1),
        "{sync:?}"
    );
}

/// Settings are validated by the store, so every client gets the same answer.
#[test]
fn settings_are_validated_whole_and_invariants_are_not_editable() {
    let scratch = Scratch::new("settings");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);
    let (_child, listen) = start_daemon(&scratch, &store);

    let (status, schema) = get_json(&listen, "/api/v1/settings/schema");
    assert_eq!(status, 200);
    let fields = schema.get("fields").expect("fields");
    assert!(fields.get("privacy_mode").is_some(), "{schema:?}");
    assert_eq!(
        fields
            .pointer(&["unmask_does_not_grant_egress", "kind"])
            .and_then(Value::as_str),
        Some("const_bool"),
        "the egress invariant must be published as fixed, not as a preference"
    );

    let base = r#""privacy_mode":"default","screenshot_privacy":false,"copy_confirm":true,"locale":"zh-CN","retention_days":30,"vault":"metadata-only","notifications":true,"resource_limits":{"daemon_rss_mb":512,"scan_files":100000},"analysis_adapter":"none""#;
    let before = fs::read_to_string(store.join("settings.json")).unwrap();

    // R07: revealing masked text never grants egress, and settings cannot
    // assert otherwise.
    let (code, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/settings",
        &format!(r#"{{{base},"unmask_does_not_grant_egress":false}}"#),
    );
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("settings.invariant_not_editable"), "{raw}");

    // A value outside the declared set is refused.
    let (code, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/settings",
        r#"{"privacy_mode":"pwned","screenshot_privacy":false,"copy_confirm":true,"locale":"zh-CN","retention_days":30,"vault":"metadata-only","notifications":true,"resource_limits":{"daemon_rss_mb":512,"scan_files":100000},"analysis_adapter":"none","unmask_does_not_grant_egress":true}"#,
    );
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("settings.value_not_allowed"), "{raw}");

    // Naming an adapter this slice does not implement is refused too.
    let (code, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/settings",
        r#"{"privacy_mode":"default","screenshot_privacy":false,"copy_confirm":true,"locale":"zh-CN","retention_days":30,"vault":"metadata-only","notifications":true,"resource_limits":{"daemon_rss_mb":512,"scan_files":100000},"analysis_adapter":"gpt-5","unmask_does_not_grant_egress":true}"#,
    );
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("settings.value_not_allowed"), "{raw}");

    // Nothing above was stored.
    assert_eq!(
        fs::read_to_string(store.join("settings.json")).unwrap(),
        before,
        "a refused settings write must not land"
    );

    // A complete, valid document is accepted.
    let (code, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/settings",
        &format!(r#"{{{base},"unmask_does_not_grant_egress":true}}"#),
    );
    assert_eq!(code, 200, "{raw}");

    // Fields are not merged: a missing one is an error, not a default.
    let (code, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/settings",
        r#"{"privacy_mode":"screenshot"}"#,
    );
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("settings.field_missing"), "{raw}");
}

/// R05 requires a post-Receipt for every mutation, undo included.
///
/// Apply and copy had one; the rollbacks did not, so undoing a change left
/// the last recorded observation describing the state *before* the undo —
/// exactly the state that no longer held.
#[test]
fn a_rollback_is_followed_by_an_observation() {
    let scratch = Scratch::new("r05");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("vendor/skill-a.md", "skill body\n");
    let digest = ctxpect_schema::sha256_hex(b"skill body\n");
    scratch.write(
        ".ctxpect/assets.json",
        format!(
            r#"{{"schema":"ctxpect-assets-v1","assets":[{{"asset_id":"skill-a","origin":"project:vendor/skill-a.md","license":"MIT","digest":"{digest}","target_rel":".ctxpect/skills/skill-a.md"}}]}}"#
        ),
    );
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let project_s = scratch.path.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);

    let post_id = |value: &Value| -> String {
        value
            .get("post_receipt_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };

    // Asset copy, then undo it.
    let (code, copied, out) = run(&[
        "assets", "copy", "--json", "--project", project_s, "--store", store_s, "--id", "skill-a",
    ]);
    assert_eq!(code, 0, "{out} {copied:?}");
    assert!(!post_id(&copied).is_empty(), "copy needs a post-Receipt");
    let tx = copied
        .pointer(&["transaction", "tx_id"])
        .and_then(Value::as_str)
        .expect("tx_id")
        .to_string();

    let (code, undone, out) = run(&[
        "assets", "rollback", "--json", "--project", project_s, "--store", store_s, "--id", &tx,
    ]);
    assert_eq!(code, 0, "{out} {undone:?}");
    assert!(
        !post_id(&undone).is_empty(),
        "an asset rollback needs a post-Receipt too: {undone:?}"
    );

    // Projection apply, then undo it. This one changes a file the inspect
    // actually observes, so the Receipt ids must differ and then come back.
    let (code, applied, out) = run(&["apply", "--json", "--project", project_s, "--store", store_s]);
    assert_eq!(code, 0, "{out} {applied:?}");
    let after_apply = post_id(&applied);
    assert!(!after_apply.is_empty(), "apply needs a post-Receipt");

    let tx = applied
        .pointer(&["transaction", "tx_id"])
        .and_then(Value::as_str)
        .map(str::to_string)
        .expect("tx_id");
    let (code, undone, out) = run(&[
        "rollback", "--json", "--project", project_s, "--store", store_s, "--id", &tx,
    ]);
    assert_eq!(code, 0, "{out} {undone:?}");
    let after_rollback = post_id(&undone);
    assert!(
        !after_rollback.is_empty(),
        "a projection rollback needs a post-Receipt too: {undone:?}"
    );
    // The undo restored the observed file, so the observation returns to what
    // it was — which is what makes the rollback checkable rather than merely
    // reported.
    assert_ne!(
        after_apply, after_rollback,
        "apply and rollback observed the same state, so nothing was verified"
    );
}

/// R04: the same question gets the same answer from the CLI and the API.
///
/// It did not. `GET /api/v1/standards/:id` returned the stored document
/// while `standard status` returned the verified standard plus this
/// project's adoption, and there was no per-exception endpoint at all. Both
/// now call the same function, which is the only way this stays true.
#[test]
fn the_cli_and_the_api_answer_the_same_question_identically() {
    let scratch = Scratch::new("r04");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let project_s = scratch.path.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);

    let (code, _, out) = run(&[
        "standard", "publish", "--json", "--store", store_s, "--project", project_s, "--id",
        "std-a", "--text", "rules",
    ]);
    assert_eq!(code, 0, "{out}");

    let (_child, listen) = start_daemon(&scratch, &store);

    // Fields that legitimately differ between invocations.
    let strip = |value: &Value| -> Value {
        fn walk(value: &Value) -> Value {
            match value {
                Value::Object(map) => Value::Object(
                    map.iter()
                        .filter(|(key, _)| {
                            !matches!(
                                key.as_str(),
                                "created_at"
                                    | "at"
                                    | "snapshot_digest"
                                    | "command"
                                    | "exit_code"
                                    | "schema_version"
                            )
                        })
                        .map(|(key, item)| (key.clone(), walk(item)))
                        .collect(),
                ),
                Value::Array(items) => Value::Array(items.iter().map(walk).collect()),
                other => other.clone(),
            }
        }
        walk(value)
    };

    for (label, cli_args, api_path) in [
        (
            "standard status",
            vec![
                "standard", "status", "--json", "--store", store_s, "--project", project_s, "--id",
                "std-a",
            ],
            "/api/v1/standards/std-a",
        ),
        (
            // Absence must be reported the same way too; the API used to
            // turn it into an error.
            "standard status (absent)",
            vec![
                "standard", "status", "--json", "--store", store_s, "--project", project_s, "--id",
                "ghost",
            ],
            "/api/v1/standards/ghost",
        ),
        (
            "exception status",
            vec![
                "exception", "status", "--json", "--store", store_s, "--project", project_s,
                "--id", "ex-planted",
            ],
            "/api/v1/exceptions/ex-planted",
        ),
    ] {
        let (_, cli_json, cli_out) = run(&cli_args);
        let (status, api_json) = get_json(&listen, api_path);
        assert_eq!(status, 200, "{label}: {api_json:?}");
        assert_eq!(
            strip(&cli_json),
            strip(&api_json),
            "{label}: CLI and API disagree\nCLI: {cli_out}"
        );
    }

    // Receipt detail: same document from both.
    let (code, receipts, out) = run(&["receipt", "list", "--json", "--store", store_s]);
    let receipt_id = if code == 0 {
        receipts
            .get("receipts")
            .and_then(Value::as_array)
            .and_then(<[Value]>::first)
            .and_then(|item| item.get("receipt_id"))
            .and_then(Value::as_str)
            .map(str::to_string)
    } else {
        // `receipt list` may not exist; fall back to the store index.
        let _ = out;
        fs::read_to_string(store.join("index.json"))
            .ok()
            .and_then(|text| parse(&text).ok())
            .and_then(|value| {
                value
                    .as_array()
                    .and_then(<[Value]>::first)
                    .and_then(|item| item.get("receipt_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
    };

    if let Some(id) = receipt_id {
        let (_, cli_json, cli_out) = run(&["receipt", "show", "--json", "--store", store_s, "--id", &id]);
        let (status, api_json) = get_json(&listen, &format!("/api/v1/receipts/{id}"));
        assert_eq!(status, 200, "{api_json:?}");
        assert_eq!(
            strip(&cli_json),
            strip(&api_json),
            "receipt show: CLI and API disagree\nCLI: {cli_out}"
        );

        // Diagnosis: the CLI also runs an inspect, so it carries that step's
        // metadata. The diagnosis itself must match, and the API must name
        // the Receipt it rests on — a diagnosis that does not is uncheckable.
        let drop_inspect_meta = |value: &Value| -> Value {
            match value {
                Value::Object(map) => Value::Object(
                    map.iter()
                        .filter(|(key, _)| {
                            !matches!(key.as_str(), "inspect_exit_code" | "snapshot_schema")
                        })
                        .map(|(key, item)| (key.clone(), item.clone()))
                        .collect(),
                ),
                other => other.clone(),
            }
        };
        let (_, cli_doc, _) = run(&[
            "doctor", "--json", "--project", project_s, "--store", store_s,
        ]);
        let (status, api_doc) = get_json(&listen, &format!("/api/v1/doctor?receipt_id={id}"));
        assert_eq!(status, 200, "{api_doc:?}");
        assert!(
            api_doc.get("receipt_id").and_then(Value::as_str).is_some(),
            "the API diagnosis must name its Receipt: {api_doc:?}"
        );
        assert_eq!(
            strip(&drop_inspect_meta(&cli_doc)),
            strip(&drop_inspect_meta(&api_doc)),
            "doctor: the diagnosis itself must match"
        );
    }
}

/// Security headers are present, and no cross-origin grant is advertised.
#[test]
fn responses_carry_security_headers_and_no_cors_grant() {
    let scratch = Scratch::new("headers");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon(&scratch, &store);

    let (status, raw) = http_call(&listen, "GET", "/api/v1/health", "");
    assert_eq!(status, 200, "{raw}");
    for header in [
        "Content-Security-Policy:",
        "frame-ancestors 'none'",
        "X-Content-Type-Options: nosniff",
        "X-Frame-Options: DENY",
        "Referrer-Policy: no-referrer",
        "Cache-Control: no-store",
    ] {
        assert!(raw.contains(header), "missing `{header}`: {raw}");
    }
    // Nothing needs cross-origin access: the UI is same-origin, and the dev
    // server proxies `/api`. A header that grants nothing but looks like a
    // policy is worse than none, because it invites being "fixed" later.
    assert!(
        !raw.contains("Access-Control-Allow-Origin"),
        "no cross-origin grant should be advertised: {raw}"
    );
    // Inline styles must stay allowed: React sets them via the `style` prop.
    assert!(raw.contains("style-src 'self' 'unsafe-inline'"), "{raw}");
    // Inline scripts must not be.
    assert!(raw.contains("script-src 'self';"), "{raw}");
}

/// A relative `--ui-root` resolves the same as an absolute one.
#[test]
fn a_relative_ui_root_still_serves_files() {
    let scratch = Scratch::new("relroot");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("uiroot/index.html", "<!doctype html><title>ui</title>");
    scratch.write("uiroot/assets/app.js", "export const x = 1;\n");
    let store = scratch.path.join("store");

    // Started from inside the scratch directory with a relative root, which
    // is how `--ui-root packages/ui/dist` is normally passed.
    let mut child = ChildGuard::new(
        Command::new(bin())
            .current_dir(&scratch.path)
            .args([
                "daemon",
                "start",
                "--project",
                scratch.path.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--ui-root",
                "uiroot",
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
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(!listen.is_empty(), "daemon did not write daemon.addr");

    let (status, raw) = http_call(&listen, "GET", "/", "");
    assert_eq!(status, 200, "index through a relative root: {raw}");
    assert!(raw.contains("<title>ui</title>"), "{raw}");

    let (status, raw) = http_call(&listen, "GET", "/assets/app.js", "");
    assert_eq!(status, 200, "asset through a relative root: {raw}");
    assert!(raw.contains("export const x"), "{raw}");

    // The SPA fallback still applies, and traversal is still refused.
    let (status, raw) = http_call(&listen, "GET", "/doctor", "");
    assert_eq!(status, 200, "{raw}");
    assert!(raw.contains("<title>ui</title>"), "{raw}");
    let (status, raw) = http_call(&listen, "GET", "/../../etc/passwd", "");
    assert_eq!(status, 400, "{raw}");
}

/// Host and Origin are matched exactly, not by prefix.
///
/// A prefix test accepts `127.0.0.1.evil.com`, which is the standard
/// DNS-rebinding bypass: the attacker registers that name, points it at
/// 127.0.0.1, and the browser then sends a Host header that passes while the
/// page's origin belongs to the attacker.
#[test]
fn loopback_checks_match_the_host_exactly() {
    let scratch = Scratch::new("origin");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon(&scratch, &store);
    let port = listen.rsplit(':').next().unwrap_or("0").to_string();

    for host in [
        "127.0.0.1.evil.com",
        "localhost.evil.com",
        "127.0.0.1evil.com",
        "evil.com",
    ] {
        let (status, raw) =
            http_call_headers(&listen, "/api/v1/health", &format!("Host: {host}\r\n"));
        assert_eq!(status, 403, "host `{host}` must be refused: {raw}");
        assert!(raw.contains("api.host"), "host `{host}`: {raw}");
    }

    // A missing Host is refused rather than assumed local: HTTP/1.1 requires
    // it, and "absent" is not evidence of anything.
    let (status, raw) = http_call_headers(&listen, "/api/v1/health", "");
    assert_eq!(status, 403, "{raw}");
    assert!(raw.contains("api.host"), "{raw}");

    for origin in [
        "http://127.0.0.1.evil.com",
        "http://localhost.evil.com",
        // No scheme at all is not an origin.
        "127.0.0.1",
    ] {
        let (status, raw) = http_call_headers(
            &listen,
            "/api/v1/health",
            &format!("Host: 127.0.0.1:{port}\r\nOrigin: {origin}\r\n"),
        );
        assert_eq!(status, 403, "origin `{origin}` must be refused: {raw}");
        assert!(raw.contains("api.origin"), "origin `{origin}`: {raw}");
    }

    // The legitimate loopback forms still work, with and without an Origin.
    for headers in [
        format!("Host: 127.0.0.1:{port}\r\n"),
        format!("Host: localhost:{port}\r\n"),
        format!("Host: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n"),
        format!("Host: localhost:{port}\r\nOrigin: http://localhost:{port}\r\n"),
    ] {
        let (status, raw) = http_call_headers(&listen, "/api/v1/health", &headers);
        assert_eq!(status, 200, "headers `{headers}` should be accepted: {raw}");
    }
}

/// The static file server must not hand out files from outside the UI root.
///
/// A textual `..` check does not make a path safe: a symlink inside the root
/// points outside it without the request ever containing `..`.
#[cfg(unix)]
#[test]
fn a_symlink_inside_the_ui_root_cannot_read_outside_it() {
    let scratch = Scratch::new("uiroot");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("uiroot/index.html", "<!doctype html><title>ui</title>");
    scratch.write("secret.txt", "SECRET-OUTSIDE-UIROOT\n");
    std::os::unix::fs::symlink(
        scratch.path.join("secret.txt"),
        scratch.path.join("uiroot/leak.html"),
    )
    .expect("symlink");

    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon_with_ui(&scratch, &store, &scratch.path.join("uiroot"));

    let (status, raw) = http_call(&listen, "GET", "/leak.html", "");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("api.path"), "{raw}");
    assert!(
        !raw.contains("SECRET-OUTSIDE-UIROOT"),
        "a symlink must not read outside the UI root: {raw}"
    );

    // Normal serving still works, including the SPA fallback for app routes.
    let (status, raw) = http_call(&listen, "GET", "/", "");
    assert_eq!(status, 200, "{raw}");
    assert!(raw.contains("<title>ui</title>"), "{raw}");
    let (status, raw) = http_call(&listen, "GET", "/doctor", "");
    assert_eq!(status, 200, "{raw}");
    assert!(raw.contains("<title>ui</title>"), "index fallback: {raw}");

    // A literal traversal is still refused.
    let (status, raw) = http_call(&listen, "GET", "/../../etc/passwd", "");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("api.path"), "{raw}");
}

/// A request body that is not an object carries none of the fields a handler
/// reads, so accepting it would run with silent defaults and report success.
#[test]
fn a_non_object_request_body_is_refused_rather_than_defaulted() {
    let scratch = Scratch::new("body");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon(&scratch, &store);

    for body in ["[1,2,3]", "\"hello\"", "42", "null"] {
        let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", body);
        assert_eq!(status, 400, "body {body}: {raw}");
        assert!(raw.contains("api.body_not_object"), "body {body}: {raw}");
    }

    // Malformed JSON is a different failure and keeps its own code.
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", "{not json");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("api.parse"), "{raw}");

    // An empty body means "no parameters", which is legitimate.
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", "");
    assert_eq!(status, 200, "{raw}");
}

/// The asset copy executor vets before it writes, and every refusal happens
/// in preview so nothing is left behind.
#[test]
fn asset_copy_is_vetted_authorized_and_reversible() {
    let scratch = Scratch::new("assets");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("vendor/skill-a.md", "skill body\n");
    scratch.write("vendor/other.md", "different bytes\n");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let project_s = scratch.path.to_str().unwrap();

    // The digest is computed by the product's own hash, so the fixture cannot
    // drift from what the executor checks.
    let digest = ctxpect_schema::sha256_hex(b"skill body\n");
    scratch.write(
        ".ctxpect/assets.json",
        format!(
            r#"{{"schema":"ctxpect-assets-v1","assets":[
                {{"asset_id":"skill-a","origin":"project:vendor/skill-a.md","license":"MIT","digest":"{digest}","target_rel":".ctxpect/skills/skill-a.md"}},
                {{"asset_id":"no-license","origin":"project:vendor/other.md","digest":"{digest}","target_rel":".ctxpect/skills/x.md"}},
                {{"asset_id":"look-alike","origin":"project:vendor/other.md","license":"MIT","digest":"{digest}","target_rel":".ctxpect/skills/y.md"}}
            ]}}"#
        ),
    );

    let preview = |id: &str| {
        run(&[
            "assets", "preview", "--json", "--project", project_s, "--id", id,
        ])
    };

    // Unregistered, unlicensed, and right-name-wrong-bytes are three distinct
    // refusals, and none of them writes anything.
    let (code, json, out) = preview("ghost");
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("assets.not_registered"));

    let (code, json, out) = preview("no-license");
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("assets.license_unknown"));

    let (code, json, out) = preview("look-alike");
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("assets.digest_mismatch"));

    let (code, json, out) = preview("skill-a");
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("license").and_then(Value::as_str), Some("MIT"));
    assert!(
        !scratch.path.join(".ctxpect/skills/skill-a.md").exists(),
        "preview must not write"
    );

    // Copying is a mutation and is fail-closed without authority.
    let copy = || {
        run(&[
            "assets", "copy", "--json", "--project", project_s, "--store", store_s, "--id",
            "skill-a",
        ])
    };
    let (code, json, out) = copy();
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert!(!scratch.path.join(".ctxpect/skills/skill-a.md").exists());

    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);

    let (code, json, out) = copy();
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(
        fs::read_to_string(scratch.path.join(".ctxpect/skills/skill-a.md")).unwrap(),
        "skill body\n"
    );
    // A copy changes the project, so it is followed by an observation.
    assert!(
        !json
            .get("post_receipt_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty(),
        "{json:?}"
    );
    let tx = json
        .pointer(&["transaction", "tx_id"])
        .and_then(Value::as_str)
        .expect("tx_id")
        .to_string();

    // The lock records provenance, and the SBOM does not claim a conformance
    // it does not have.
    let (code, sbom, out) = run(&["assets", "sbom", "--json", "--store", store_s]);
    assert_eq!(code, 0, "{out} {sbom:?}");
    assert_eq!(sbom.get("component_count").and_then(Value::as_i64), Some(1));
    assert_eq!(
        sbom.get("unlicensed_components").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        sbom.get("spdx_or_cyclonedx").and_then(Value::as_str),
        Some("not-emitted")
    );

    // Rollback removes the file the copy created.
    let (code, json, out) = run(&[
        "assets", "rollback", "--json", "--project", project_s, "--store", store_s, "--id", &tx,
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert!(!scratch.path.join(".ctxpect/skills/skill-a.md").exists());
}

/// The same executor over the API, including that no path comes from the
/// request.
#[test]
fn asset_api_takes_no_path_from_the_request() {
    let scratch = Scratch::new("assetsapi");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("vendor/skill-a.md", "skill body\n");
    let digest = ctxpect_schema::sha256_hex(b"skill body\n");
    scratch.write(
        ".ctxpect/assets.json",
        format!(
            r#"{{"schema":"ctxpect-assets-v1","assets":[{{"asset_id":"skill-a","origin":"project:vendor/skill-a.md","license":"MIT","digest":"{digest}","target_rel":".ctxpect/skills/skill-a.md"}}]}}"#
        ),
    );
    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon(&scratch, &store);

    // The body is ignored: an asset id names a registry entry, and the
    // registry decides both source and destination.
    let (status, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/assets/skill-a/preview",
        r#"{"target_rel":"/etc/passwd","origin":"project:../../etc/passwd"}"#,
    );
    assert_eq!(status, 200, "{raw}");
    let plan = http_json(&raw);
    assert_eq!(
        plan.get("target_rel").and_then(Value::as_str),
        Some(".ctxpect/skills/skill-a.md"),
        "the request must not be able to choose a destination: {raw}"
    );

    // Copy is fail-closed without authority.
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", "{}");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("policy."), "{raw}");
    assert!(!scratch.path.join(".ctxpect/skills/skill-a.md").exists());

    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", "{}");
    assert_eq!(status, 200, "{raw}");
    assert!(scratch.path.join(".ctxpect/skills/skill-a.md").exists());

    // The overview now reports the executor and the lock rather than a
    // hardcoded "unimplemented".
    let (_, overview) = get_json(&listen, "/api/v1/assets");
    assert_eq!(
        overview.get("copy_executor").and_then(Value::as_str),
        Some("project-scoped")
    );
    assert_eq!(
        overview.pointer(&["sbom", "component_count"]).and_then(Value::as_i64),
        Some(1),
        "{overview:?}"
    );
}

/// Sync over the API: preview is read-only, apply is a mutation, and the
/// destination is never taken from the request.
#[test]
fn sync_api_previews_before_applying_and_fixes_its_destination() {
    let scratch = Scratch::new("syncapi");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let (_child, listen) = start_daemon(&scratch, &store);

    // A destination cannot be smuggled in through the bundle id.
    let (code, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/sync/preview",
        r#"{"bundle_id":"../../etc/passwd"}"#,
    );
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("sync.bundle_id_invalid"), "{raw}");

    // Preview is read-only and works without an approved exception.
    let (code, raw) = http_call(&listen, "POST", "/api/v1/sync/preview", r#"{"bundle_id":"b1"}"#);
    assert_eq!(code, 200, "{raw}");
    let preview = http_json(&raw);
    assert_eq!(preview.get("transport").and_then(Value::as_str), Some("ready"));
    assert_eq!(
        preview.get("semantic").and_then(Value::as_str),
        Some("not-verified")
    );
    assert!(!store.join("sync/folder/current.json").exists(), "preview must not write");

    // Apply is a mutation and is fail-closed without authority.
    let (code, raw) = http_call(&listen, "POST", "/api/v1/sync/apply", r#"{"bundle_id":"b1"}"#);
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("policy."), "{raw}");
    assert!(!store.join("sync/folder/current.json").exists(), "refused apply must not write");

    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);

    let (code, raw) = http_call(&listen, "POST", "/api/v1/sync/apply", r#"{"bundle_id":"b1"}"#);
    assert_eq!(code, 200, "{raw}");
    let applied = http_json(&raw);
    assert_eq!(applied.get("transport").and_then(Value::as_str), Some("success"));
    // R06: moving bytes is not verifying meaning.
    assert_eq!(
        applied
            .get("transport_success_is_verified")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        applied.get("semantic").and_then(Value::as_str),
        Some("structural-only")
    );
    assert_eq!(applied.get("dest_is_fixed").and_then(Value::as_bool), Some(true));
    assert!(store.join("sync/folder/current.json").exists());

    // Replay of the same bundle is refused.
    let (code, raw) = http_call(&listen, "POST", "/api/v1/sync/apply", r#"{"bundle_id":"b1"}"#);
    assert_eq!(code, 400, "{raw}");
    assert!(raw.contains("sync.replay"), "{raw}");

    // A different bundle over an occupied destination is a conflict, and
    // last-write-wins is refused.
    let (code, raw) = http_call(&listen, "POST", "/api/v1/sync/apply", r#"{"bundle_id":"b2"}"#);
    assert_eq!(code, 200, "{raw}");
    let conflict = http_json(&raw);
    assert_eq!(conflict.get("conflict").and_then(Value::as_bool), Some(true));
    assert_eq!(
        conflict.get("last_write_wins").and_then(Value::as_bool),
        Some(false)
    );
}

/// Team compliance counts what the store actually holds.
#[test]
fn team_compliance_counts_real_standards_and_exceptions() {
    let scratch = Scratch::new("compliance");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let project_s = scratch.path.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_pass_policy_and_live_exception(&store);

    let (code, json, out) = run(&[
        "standard", "publish", "--json", "--store", store_s, "--project", project_s, "--id",
        "std-a", "--text", "rules",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    let (code, json, out) = run(&[
        "standard", "adopt", "--json", "--store", store_s, "--project", project_s, "--id", "std-a",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");

    let (_child, listen) = start_daemon(&scratch, &store);
    let (status, compliance) = get_json(&listen, "/api/v1/team/compliance");
    assert_eq!(status, 200);

    assert_eq!(
        compliance.pointer(&["standard_status", "total"]).and_then(Value::as_i64),
        Some(1),
        "{compliance:?}"
    );
    // `signed` is re-derived by verifying the stored record, not read off a
    // `signed` field the record could simply claim.
    assert_eq!(
        compliance.pointer(&["standard_status", "signed"]).and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        compliance.pointer(&["standard_status", "adopted"]).and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        compliance
            .pointer(&["standard_status", "standards"])
            .and_then(Value::as_array)
            .and_then(<[Value]>::first)
            .and_then(|item| item.get("adoption_state"))
            .and_then(Value::as_str),
        Some("adopted")
    );
    // The planted exception is live and is counted as granting.
    assert_eq!(
        compliance.pointer(&["exception", "live"]).and_then(Value::as_i64),
        Some(1),
        "{compliance:?}"
    );

    // Tampering with the stored standard must drop the signed count: the
    // endpoint verifies, it does not trust the record's own claim.
    let path = store.join("standards/std-a.json");
    let original = fs::read_to_string(&path).unwrap();
    let mut tampered = parse(&original).unwrap();
    if let Value::Object(map) = &mut tampered {
        map.insert("payload_digest".into(), Value::Str("00".repeat(32)));
    }
    fs::write(&path, canonical_json(&tampered)).unwrap();
    let (_, compliance) = get_json(&listen, "/api/v1/team/compliance");
    assert_eq!(
        compliance.pointer(&["standard_status", "signed"]).and_then(Value::as_i64),
        Some(0),
        "a tampered standard must not count as signed: {compliance:?}"
    );
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

            // The API cannot carry an enrolled principal secret, so it cannot
            // establish the identity the exception lifecycle requires. It says
            // so instead of minting an exception with a made-up requester.
            let (estatus, eraw) = http_call(&listen, "POST", "/api/v1/exceptions", "{}");
            assert_eq!(estatus, 400, "{eraw}");
            assert!(eraw.contains("api.identity_required"), "{eraw}");
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
