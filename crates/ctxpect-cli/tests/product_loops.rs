//! Negative cases for L01–L11 product loops.

use ctxpect_cli::{canonical_json, persist_inspect, parse, project_scope_digest, Store, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PASS_LAYERS: &str = r#"[{"layer":"organization","mode":"enforceable","rules":[]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]"#;

/// Plant a passing five-layer policy and one live approved exception **per
/// action**, each bound to `project` (or, without one, to the store) with
/// target `*`. An exception approved for one action does not cover another,
/// so a test names exactly the mutations it performs.
fn plant_grants(store: &Path, project: Option<&Path>, actions: &[&str]) {
    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(store.join("policies/active.json"), PASS_LAYERS).unwrap();
    fs::create_dir_all(store.join("exceptions")).unwrap();
    let digest = project_scope_digest(store, project);
    for action in actions {
        let id = format!("ex-planted-{}", action.replace('.', "-"));
        let record = format!(
            r#"{{"exception_id":"{id}","requester":"operator","action":"{action}","project_digest":"{digest}","target":"*","state":"approved","created_at":1,"expires_at":4102444800,"reason":"test grant","approver":"enrolled-out-of-band"}}"#
        );
        fs::write(store.join(format!("exceptions/{id}.json")), record).unwrap();
    }
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

/// `intent preview` then `apply --tx`, the only apply path the product has.
/// Returns the preview envelope and the apply result.
fn preview_then_apply(project: &str, store: &str, target: &str, desired: &str) -> (i32, Value, String) {
    let (code, preview, out) = run(&[
        "intent", "preview", "--json", "--project", project, "--store", store, "--target", target,
        "--desired", desired,
    ]);
    assert_eq!(code, 0, "preview failed: {out} {preview:?}");
    let tx = preview.get("tx_id").and_then(Value::as_str).expect("tx_id").to_string();
    run(&["apply", "--json", "--project", project, "--store", store, "--tx", &tx])
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
    // `apply` without a persisted preview is a usage error, not a policy
    // question: there is nothing to bind the apply to.
    let (code, json, out) = run(&[
        "apply", "--json", "--project", project, "--store", store_s, "--desired", "PWNED\n",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("usage.invalid"));
    let (code, json, out) = preview_then_apply(project, store_s, "AGENTS.md", "PWNED\n");
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
    let (code, json, out) = preview_then_apply(project, store_s, "AGENTS.md", "PWNED\n");
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

    plant_grants(&store, Some(&scratch.path), &["sync.apply"]);
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

/// L07, partly covered.
///
/// This note used to say the loop was entirely uncovered because no asset
/// copy executor existed. One now does, which changes what can be built:
///
/// - **missing-license copy still executing** — now a real counterexample,
///   asserted below and again in `asset_copy_is_vetted_authorized_and_reversible`.
/// - **rewrite APM evaluator** — still not constructible. APM remains the
///   unique authority and is not re-evaluated here, so there is no evaluator
///   to rewrite; `apm_authority` stays a stated fact, not a checked one.
/// - **dual authority** — still not constructible. There is exactly one copy
///   executor, and a second would have to exist before its absence could be
///   demonstrated.
///
/// The two remaining ones stay uncovered rather than being simulated.
#[test]
fn l07_an_unlicensed_asset_is_refused_before_anything_is_written() {
    let scratch = Scratch::new("l07");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("vendor/thing.md", "body\n");
    let digest = ctxpect_schema::sha256_hex(b"body\n");
    // Registered, correctly digested, correctly targeted — and unlicensed.
    scratch.write(
        ".ctxpect/assets.json",
        format!(
            r#"{{"schema":"ctxpect-assets-v1","assets":[{{"asset_id":"unlicensed","origin":"project:vendor/thing.md","digest":"{digest}","target_rel":".ctxpect/skills/thing.md"}}]}}"#
        ),
    );
    let project_s = scratch.path.to_str().unwrap();

    let (code, json, out) = run(&[
        "assets", "preview", "--json", "--project", project_s, "--id", "unlicensed",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("assets.license_unknown"));
    // Refused during vetting, so nothing was written on the way to failing.
    assert!(
        !scratch.path.join(".ctxpect/skills/thing.md").exists(),
        "an unlicensed asset must not be partially copied"
    );
}


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

    plant_grants(&store, None, &["sessions.import", "sessions.delete"]);
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

const RUNS_CONTRACT: &str = r#"{"schema":"experiment-contract-v1","experiment_id":"e-lock","primary_outcome":"task-pass","margin_pp":10,"pairing":"paired-by-task","n_planned":NPLANNED,"alpha":"0.05","power":"0.80","multiplicity":"none","itt":"count-as-fail","locked":{"code_digest":"c","model":"m","harness":"h","tool_availability_digest":"t"},"frozen_at":"100.0Z","invalidation":["harness update"]}"#;

/// A runs document with `pairs` control/treatment pairs (control fails,
/// treatment passes) under a contract planning `n_planned` pairs.
fn runs_document(n_planned: i64, pairs: usize) -> String {
    let mut runs = Vec::new();
    for index in 0..pairs {
        for (arm, outcome) in [("control", "fail"), ("treatment", "pass")] {
            runs.push(format!(
                r#"{{"run_id":"{arm}-{index}","arm":"{arm}","task_id":"t{index}","outcome":"{outcome}","code_digest":"c","model":"m","harness":"h","tool_availability_digest":"t","started_at":"101.0Z","ended_at":"102.0Z"}}"#
            ));
        }
    }
    format!(
        r#"{{"schema":"ctxpect-effect-runs-v1","contract":{},"runs":[{}]}}"#,
        RUNS_CONTRACT.replace("NPLANNED", &n_planned.to_string()),
        runs.join(",")
    )
}

/// L10 / T2: without per-run results nothing was executed; a single pair is
/// inconclusive; a valid document is still inconclusive because the product
/// carries no estimator; persisting needs authority; the planned sample
/// size cannot change afterwards.
#[test]
fn l10_single_pair_not_causal_and_n_locked() {
    let scratch = Scratch::new("l10");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();

    // No runs: not executed, no decision, exit 3.
    let (code, json, out) = run(&["experiment", "--json", "--id", "e1"]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.get("executed"), Some(&Value::Bool(false)));
    assert_eq!(json.get("decision"), Some(&Value::Null));
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("effect.runs_required"));
    assert_eq!(json.get("causal"), Some(&Value::Bool(false)));

    // A single pair under a contract planning four: inconclusive, n insufficient.
    let one = scratch.write("one.json", runs_document(4, 1));
    let (code, json, out) = run(&["experiment", "--json", "--runs", one.to_str().unwrap()]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.get("executed"), Some(&Value::Bool(true)));
    assert_eq!(json.get("decision").and_then(Value::as_str), Some("inconclusive"));
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("effect.n_insufficient"));
    assert_eq!(json.get("runs").and_then(Value::as_array).map(<[Value]>::len), Some(2));

    // Four valid pairs: the frozen exact estimator runs and finds four
    // discordant pairs are not evidence (p = 0.125); the numbers are shown.
    let four = scratch.write("four.json", runs_document(4, 4));
    let (code, json, out) = run(&["experiment", "--json", "--runs", four.to_str().unwrap()]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.get("decision").and_then(Value::as_str), Some("inconclusive"));
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("effect.estimator_inconclusive"));
    assert_eq!(json.pointer(&["estimator", "available"]), Some(&Value::Bool(true)));
    assert_eq!(json.pointer(&["estimator", "name"]).and_then(Value::as_str), Some("paired-exact-binomial-v2"));
    assert_eq!(json.pointer(&["estimator", "detail", "p_value"]).and_then(Value::as_str), Some("0.125000"));
    // Ten discordant pairs favouring treatment: supported, exit 0.
    let ten = scratch.write("ten.json", runs_document(10, 10));
    let (code, json, out) = run(&["experiment", "--json", "--runs", ten.to_str().unwrap()]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("decision").and_then(Value::as_str), Some("supported-beneficial"));
    assert_eq!(json.get("causal"), Some(&Value::Bool(false)));

    // A drifted confounder invalidates the experiment with its own reason.
    let drifted = scratch.write("drift.json", runs_document(4, 4).replacen("\"model\":\"m\"", "\"model\":\"other\"", 1));
    let (code, json, _) = run(&["experiment", "--json", "--runs", drifted.to_str().unwrap()]);
    assert_eq!(code, 3);
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("effect.confounder_drift"));

    // A malformed document is not an experiment.
    let bad = scratch.write("bad.json", r#"{"schema":"ctxpect-effect-runs-v1","contract":{"schema":"experiment-contract-v1"},"runs":[]}"#);
    let (code, json, _) = run(&["experiment", "--json", "--runs", bad.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("effect.contract_invalid"));

    // Persisting an experiment result is a store mutation. Without authority
    // it is refused and nothing is written.
    let (code, json, out) = run(&["experiment", "--json", "--runs", four.to_str().unwrap(), "--store", store_s]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.unknown"));
    assert!(!store.join("experiments/e-lock.json").exists());

    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, None, &["experiment.persist"]);
    let (code, json, out) = run(&["experiment", "--json", "--runs", four.to_str().unwrap(), "--store", store_s]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert!(store.join("experiments/e-lock.json").exists());
    // The planned sample size is locked once results are recorded.
    let eight = scratch.write("eight.json", runs_document(8, 8));
    let (code, json, out) = run(&["experiment", "--json", "--runs", eight.to_str().unwrap(), "--store", store_s]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("effect.n_locked"));
    // `--n` cannot override the frozen contract either.
    let (code, json, _) = run(&["experiment", "--json", "--runs", four.to_str().unwrap(), "--n", "2"]);
    assert_eq!(code, 1);
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
    let (acode, ajson, _) = preview_then_apply(project, store_s, "AGENTS.md", "PWNED\n");
    assert_eq!(acode, 1);
    assert_eq!(
        err_code(&ajson),
        json.get("reason_code").and_then(Value::as_str)
    );
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "one\n"
    );

    plant_grants(&store, Some(&scratch.path), &["apply"]);
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
    let (acode, ajson, out) = preview_then_apply(project, store_s, "AGENTS.md", "ok\n");
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
        "tx_0000000000000000",
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

    // An enrolled requester may request, for one action with one lifetime.
    let (code, json, out) = run_as(
        "alice-secret",
        &[
            "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-1", "--principal", "alice", "--action", "standard.publish", "--expires-in", "3600",
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
    plant_grants(&store, None, &["standard.publish"]);
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

/// C31: an approved exception is bound to one action, one project and one
/// target. The same record must not authorize another project or action.
#[test]
fn an_exception_for_one_project_and_action_does_not_authorize_another() {
    let a = Scratch::new("scope-a");
    let b = Scratch::new("scope-b");
    a.write("AGENTS.md", "a\n");
    b.write("AGENTS.md", "b\n");
    let store = a.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    // Approved for project A, action `apply`, any target.
    plant_grants(&store, Some(&a.path), &["apply"]);

    // Project B, same action: refused, and its file is untouched.
    let (code, json, out) = preview_then_apply(b.path.to_str().unwrap(), store_s, "AGENTS.md", "x\n");
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.approval_required"));
    assert!(
        json.pointer(&["error", "message"])
            .and_then(Value::as_str)
            .is_some_and(|m| m.contains("exception.project_mismatch")),
        "{json:?}"
    );
    assert_eq!(fs::read_to_string(b.path.join("AGENTS.md")).unwrap(), "b\n");

    // Project A, another action (`sessions.import`): refused too.
    a.write("sess.json", r#"{"events":[{"type":"user","text":"hi"}]}"#);
    let (code, json, out) = run(&[
        "import", "--json", "--project", a.path.to_str().unwrap(), "--store", store_s, "--from",
        a.path.join("sess.json").to_str().unwrap(), "--session", "s-scope",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.approval_required"));
    assert!(
        json.pointer(&["error", "message"])
            .and_then(Value::as_str)
            .is_some_and(|m| m.contains("exception.action_mismatch")),
        "{json:?}"
    );
    assert!(!store.join("sessions/s-scope.json").exists());

    // Project A, the approved action: allowed.
    let (code, json, out) = preview_then_apply(a.path.to_str().unwrap(), store_s, "AGENTS.md", "x\n");
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(fs::read_to_string(a.path.join("AGENTS.md")).unwrap(), "x\n");
}

/// C32: `apply --tx` is bound to the previewed digest. A target edited after
/// the preview is not overwritten by an apply that recomputed nothing.
#[test]
fn apply_is_bound_to_the_persisted_preview_not_a_recomputed_one() {
    let scratch = Scratch::new("bind");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["apply"]);

    let (code, preview, out) = run(&[
        "intent", "preview", "--json", "--project", project, "--store", store_s, "--target",
        "AGENTS.md", "--desired", "two\n",
    ]);
    assert_eq!(code, 0, "{out} {preview:?}");
    assert_eq!(preview.get("persisted").and_then(Value::as_bool), Some(true));
    let tx = preview.get("tx_id").and_then(Value::as_str).unwrap().to_string();
    assert!(store.join(format!("previews/{tx}.json")).is_file());

    // The user edits the file between preview and apply.
    scratch.write("AGENTS.md", "edited meanwhile\n");
    let (code, json, out) = run(&["apply", "--json", "--project", project, "--store", store_s, "--tx", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.concurrent_hash"));
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "edited meanwhile\n"
    );
    // An unknown transaction id is a missing preview, not a silent recompute.
    let (code, json, _) = run(&["apply", "--json", "--project", project, "--store", store_s, "--tx", "tx_nope"]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("projection.preview_missing"));
}

/// C33: rollback re-hashes the target and refuses a concurrent edit; a file
/// the apply created is removed, not emptied.
/// T1(a): a persisted preview is bound to the project it was computed in and
/// is consumed by its apply. Project B, holding its own apply exception,
/// cannot apply project A's transaction; A's own transaction cannot be
/// applied twice, and a rollback does not make it applicable again.
#[test]
fn a_preview_is_bound_to_its_project_and_consumed_by_its_apply() {
    let a = Scratch::new("scope-a");
    let b = Scratch::new("scope-b");
    a.write("AGENTS.md", "same\n");
    b.write("AGENTS.md", "same\n");
    let store = a.path.join("store");
    let store_s = store.to_str().unwrap();
    let project_a = a.path.to_str().unwrap();
    let project_b = b.path.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    // Both projects hold their own grants in the shared store.
    plant_grants(&store, Some(&a.path), &["apply", "rollback"]);
    let digest_b = project_scope_digest(&store, Some(&b.path));
    for action in ["apply", "rollback"] {
        let id = format!("ex-b-{action}");
        fs::write(
            store.join(format!("exceptions/{id}.json")),
            format!(
                r#"{{"exception_id":"{id}","requester":"operator","action":"{action}","project_digest":"{digest_b}","target":"*","state":"approved","created_at":1,"expires_at":4102444800,"reason":"test grant","approver":"enrolled-out-of-band"}}"#
            ),
        )
        .unwrap();
    }

    let (code, preview, out) = run(&[
        "intent", "preview", "--json", "--project", project_a, "--store", store_s, "--target",
        "AGENTS.md", "--desired", "from-a\n",
    ]);
    assert_eq!(code, 0, "{out}");
    let tx = preview.get("tx_id").and_then(Value::as_str).unwrap().to_string();
    assert_eq!(preview.get("state").and_then(Value::as_str), Some("previewed"));
    assert_eq!(
        preview.get("project_digest").and_then(Value::as_str),
        Some(project_scope_digest(&store, Some(&a.path)).as_str())
    );

    // B cannot consume A's transaction, and A's record is untouched.
    let (code, json, out) = run(&["apply", "--json", "--project", project_b, "--store", store_s, "--tx", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.preview_scope"));
    assert_eq!(fs::read_to_string(b.path.join("AGENTS.md")).unwrap(), "same\n");
    assert!(!store.join("apply").join(&tx).exists(), "no transaction directory was written for B");

    // A applies once.
    let (code, json, out) = run(&["apply", "--json", "--project", project_a, "--store", store_s, "--tx", &tx]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(fs::read_to_string(a.path.join("AGENTS.md")).unwrap(), "from-a\n");
    // ... and not twice.
    let (code, json, out) = run(&["apply", "--json", "--project", project_a, "--store", store_s, "--tx", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.tx_consumed"));
    // A rollback closes the transaction; it does not reopen the preview.
    let (code, json, out) = run(&["rollback", "--json", "--project", project_a, "--store", store_s, "--id", &tx]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(fs::read_to_string(a.path.join("AGENTS.md")).unwrap(), "same\n");
    let (code, json, out) = run(&["apply", "--json", "--project", project_a, "--store", store_s, "--tx", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.tx_consumed"));
    // A fresh preview is a fresh transaction: it never reuses the consumed
    // id, so the closed record and its backup are left intact.
    let (code, preview, _) = run(&[
        "intent", "preview", "--json", "--project", project_a, "--store", store_s, "--target",
        "AGENTS.md", "--desired", "from-a\n",
    ]);
    assert_eq!(code, 0);
    assert_ne!(preview.get("tx_id").and_then(Value::as_str), Some(tx.as_str()));
    assert_eq!(preview.get("state").and_then(Value::as_str), Some("previewed"));
    let closed = ctxpect_schema::parse(
        &fs::read_to_string(Path::new(store_s).join("apply").join(&tx).join("tx.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(closed.get("state").and_then(Value::as_str), Some("rolled-back"));
    // A caller-supplied id that is not a transaction id never reaches a path.
    let (code, json, _) = run(&["rollback", "--json", "--project", project_a, "--store", store_s, "--id", "../../evil"]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("store.bad_id"));
}

/// T1(b): a persistence failure under `inspect --store` is reported and
/// exits 1; `store status` shows the in-doubt record; `store repair`
/// rebuilds the index and the retry lands the Receipt exactly once.
#[test]
fn inspect_store_reports_a_persistence_failure_and_repair_recovers_it() {
    let scratch = Scratch::new("persist");
    scratch.write("AGENTS.md", "hello\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let (code, json, _) = run(&["inspect", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{json:?}");
    assert!(json.get("formal_receipt_id").and_then(Value::as_str).is_some());

    fs::write(store.join("index.json"), "{not json").unwrap();
    scratch.write("AGENTS.md", "changed\n");
    let (code, json, out) = run(&["inspect", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(json.get("formal_receipt_id"), Some(&Value::Null));
    assert_eq!(
        json.pointer(&["persist_error", "code"]).and_then(Value::as_str),
        Some("store.index_corrupt")
    );

    let (code, json, out) = run(&["store", "status", "--json", "--store", store_s]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.pointer(&["index", "status"]).and_then(Value::as_str), Some("corrupt"));
    assert_eq!(json.pointer(&["journal", "in_doubt"]).and_then(Value::as_i64), Some(1));

    let (code, json, out) = run(&["store", "repair", "--json", "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("rebuilt"), Some(&Value::Bool(true)));
    assert_eq!(json.pointer(&["status", "journal", "in_doubt"]).and_then(Value::as_i64), Some(0));
    let (code, json, out) = run(&["inspect", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    let id = json.get("formal_receipt_id").and_then(Value::as_str).unwrap().to_string();
    let index = parse(&fs::read_to_string(store.join("index.json")).unwrap()).unwrap();
    let rows = index.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{index:?}");
    assert_eq!(
        rows.iter().filter(|r| r.get("receipt_id").and_then(Value::as_str) == Some(id.as_str())).count(),
        1
    );
    let audit = fs::read_to_string(store.join("audit/events.jsonl")).unwrap();
    let puts = audit
        .lines()
        .filter(|line| line.contains("\"action\":\"receipt.put\"") && line.contains(&format!("\"target\":\"{id}\"")))
        .count();
    assert_eq!(puts, 1, "one receipt.put audit entry for the retried put");
}

/// T1(b): the same-machine advisory lock. A live holder makes a mutation
/// fail with `store.busy`; a dead holder is stale and taken over; and two
/// real `ctxpect` processes applying the same transaction at once produce
/// exactly one success.
#[cfg(unix)]
#[test]
fn a_live_lock_holder_refuses_a_mutation_and_concurrent_applies_yield_one_success() {
    let scratch = Scratch::new("lock");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["apply", "rollback"]);
    let (code, preview, out) = run(&[
        "intent", "preview", "--json", "--project", project, "--store", store_s, "--target",
        "AGENTS.md", "--desired", "two\n",
    ]);
    assert_eq!(code, 0, "{out}");
    let tx = preview.get("tx_id").and_then(Value::as_str).unwrap().to_string();

    // A foreign process holds the lock: this test process takes the OS lock
    // on store/lock, exactly as another ctxpect would.
    fs::write(store.join("lock"), "").unwrap();
    let holder = fs::OpenOptions::new().read(true).write(true).open(store.join("lock")).unwrap();
    holder.try_lock().unwrap();
    let (code, json, out) = run(&["apply", "--json", "--project", project, "--store", store_s, "--tx", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("store.busy"));
    assert_eq!(fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(), "one\n");
    let (_, status, _) = run(&["store", "status", "--json", "--store", store_s]);
    assert_eq!(status.pointer(&["lock", "held"]), Some(&Value::Bool(true)));
    assert_eq!(status.pointer(&["lock", "holder"]).and_then(Value::as_str), Some("other-process"));
    drop(holder);
    let (_, status, _) = run(&["store", "status", "--json", "--store", store_s]);
    assert_eq!(status.pointer(&["lock", "held"]), Some(&Value::Bool(false)));

    // Two real processes race for the same transaction.
    let mut children: Vec<std::process::Child> = (0..2)
        .map(|_| {
            Command::new(bin())
                .args(["apply", "--json", "--project", project, "--store", store_s, "--tx", &tx])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    let mut successes = 0;
    let mut refusals = Vec::new();
    for child in children.iter_mut() {
        let output = child.wait_with_output_ref();
        if output.0 == 0 {
            successes += 1;
        } else {
            refusals.push(err_code(&output.1).unwrap_or("?").to_string());
        }
    }
    assert_eq!(successes, 1, "exactly one apply may land; refusals: {refusals:?}");
    assert!(
        refusals.iter().all(|code| {
            matches!(code.as_str(), "store.busy" | "projection.tx_consumed" | "projection.concurrent_hash")
        }),
        "{refusals:?}"
    );
    assert_eq!(fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(), "two\n");
    let (_, status, _) = run(&["store", "status", "--json", "--store", store_s]);
    assert_eq!(status.pointer(&["lock", "held"]), Some(&Value::Bool(false)), "released after the mutation");
}

trait WaitOutput {
    fn wait_with_output_ref(&mut self) -> (i32, Value);
}

impl WaitOutput for std::process::Child {
    fn wait_with_output_ref(&mut self) -> (i32, Value) {
        use std::io::Read;
        let mut stdout = String::new();
        if let Some(mut pipe) = self.stdout.take() {
            let _ = pipe.read_to_string(&mut stdout);
        }
        let code = self.wait().ok().and_then(|s| s.code()).unwrap_or(255);
        (code, parse(stdout.trim()).unwrap_or(Value::Null))
    }
}

/// T1(b): a transaction interrupted after its record landed is listed and
/// judged by `apply status` / `store status` without rewriting the target.
#[cfg(unix)]
#[test]
fn pending_transactions_are_judged_not_rewritten() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new("pending");
    scratch.write("locked/AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["apply", "rollback"]);
    let (code, preview, _) = run(&[
        "intent", "preview", "--json", "--project", project, "--store", store_s, "--target",
        "locked/AGENTS.md", "--desired", "two\n",
    ]);
    assert_eq!(code, 0);
    let tx = preview.get("tx_id").and_then(Value::as_str).unwrap().to_string();
    fs::set_permissions(scratch.path.join("locked"), fs::Permissions::from_mode(0o555)).unwrap();
    let (code, json, _) = run(&["apply", "--json", "--project", project, "--store", store_s, "--tx", &tx]);
    fs::set_permissions(scratch.path.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("projection.io"));

    let (code, json, out) = run(&["apply", "status", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    let txs = json.get("transactions").and_then(Value::as_array).unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].get("tx_id").and_then(Value::as_str), Some(tx.as_str()));
    assert_eq!(txs[0].get("judgement").and_then(Value::as_str), Some("aborted"));

    fs::write(scratch.path.join("locked/AGENTS.md"), "someone else\n").unwrap();
    let (code, json, _) = run(&["store", "status", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 3, "in-doubt is not a clean status: {json:?}");
    let txs = json.get("transactions").and_then(Value::as_array).unwrap();
    assert_eq!(txs[0].get("judgement").and_then(Value::as_str), Some("in-doubt"));
    assert_eq!(fs::read_to_string(scratch.path.join("locked/AGENTS.md")).unwrap(), "someone else\n");
}

/// T1(c)/(d): `ci --store` persists a `ci` Receipt and `collect --store` a
/// `device-baseline` one; a Receipt signed without `signed_at` verifies as
/// `receipt.signature_legacy`, and one whose `created_at` was edited fails.
#[test]
fn ci_and_collect_persist_their_kinds_and_verify_covers_the_times() {
    let scratch = Scratch::new("kinds");
    scratch.write("AGENTS.md", "hello\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(store.join("policies/active.json"), PASS_LAYERS).unwrap();

    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    let ci_id = json.get("receipt_id").and_then(Value::as_str).unwrap().to_string();
    assert_eq!(json.get("receipt_persisted"), Some(&Value::Bool(true)));
    let (code, receipt, _) = run(&["receipt", "show", "--json", "--store", store_s, "--receipt", &ci_id]);
    assert_eq!(code, 0);
    assert_eq!(receipt.get("receipt_kind").and_then(Value::as_str), Some("ci"));

    let (code, json, out) = run(&["collect", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    let base_id = json.get("receipt_id").and_then(Value::as_str).unwrap().to_string();
    let (code, receipt, _) = run(&["receipt", "show", "--json", "--store", store_s, "--receipt", &base_id]);
    assert_eq!(code, 0);
    assert_eq!(receipt.get("receipt_kind").and_then(Value::as_str), Some("device-baseline"));
    assert!(
        receipt.get("evidence").and_then(Value::as_array).is_some_and(|e| {
            e.iter().any(|item| item.get("path").and_then(Value::as_str) == Some("AGENTS.md"))
        }),
        "{receipt:?}"
    );
    // Without a store, neither command claims a Receipt.
    let (_, json, _) = run(&["collect", "--json", "--project", project]);
    assert_eq!(json.get("receipt_id"), Some(&Value::Null));
    assert_eq!(json.get("receipt_persisted"), Some(&Value::Bool(false)));

    // Verify covers the times.
    let (code, json, out) = run(&["receipt", "verify", "--json", "--store", store_s, "--receipt", &ci_id]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("signed_at_trusted"), Some(&Value::Bool(false)));
    let path = store.join(format!("receipts/{ci_id}.json"));
    let original = fs::read_to_string(&path).unwrap();
    let stored = parse(&original).unwrap();
    let created = stored.get("created_at").and_then(Value::as_str).unwrap().to_string();
    fs::write(&path, original.replace(&format!("\"created_at\":\"{created}\""), "\"created_at\":\"0.0Z\"")).unwrap();
    let (code, json, _) = run(&["receipt", "verify", "--json", "--store", store_s, "--receipt", &ci_id]);
    assert_eq!(code, 1, "{json:?}");
    assert_eq!(err_code(&json), Some("receipt.signature_mismatch"));
    // A legacy signature (no signed_at) is named, not waved through.
    let signed = stored.pointer(&["signature", "signed_at"]).and_then(Value::as_str).unwrap().to_string();
    fs::write(&path, original.replace(&format!("\"signed_at\":\"{signed}\","), "")).unwrap();
    let (code, json, out) = run(&["receipt", "verify", "--json", "--store", store_s, "--receipt", &ci_id]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.get("ok"), Some(&Value::Bool(false)));
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("receipt.signature_legacy"));
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn sha256_file(path: &Path) -> String {
    ctxpect_schema::sha256_hex(&fs::read(path).unwrap())
}

/// Copy the codex static corpus (matrix, manifest, rows and the inputs the
/// rows name) into a scratch acceptance tree so a golden row can be tampered
/// with outside the repository.
fn copy_codex_corpus(into: &Path) {
    let root = repo_root();
    for rel in ["acceptance/compatibility-matrix.yaml", "acceptance/corpus-manifest.json", "acceptance/corpus/development/static/codex__cli.jsonl"] {
        let target = into.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(root.join(rel), target).unwrap();
    }
    let rows = fs::read_to_string(root.join("acceptance/corpus/development/static/codex__cli.jsonl")).unwrap();
    for line in rows.lines().filter(|l| !l.trim().is_empty()) {
        let row = parse(line).unwrap();
        let input = row.get("input_path").and_then(Value::as_str).unwrap();
        copy_tree(&root.join(input), &into.join(input));
    }
}

fn copy_tree(from: &Path, to: &Path) {
    if from.is_dir() {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        }
    } else if from.is_file() {
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(from, to).unwrap();
    }
}

/// T3: `adapter test --adapter <family>` really runs that family's static
/// rows through the shared runner and binds the result to the corpus, the
/// matrix and the resolver version; a family without an implementation is
/// `unimplemented`, never a pass; and a tampered golden row is reported by
/// both the gate runner and the command.
#[test]
fn adapter_test_runs_the_family_corpus_and_binds_digests() {
    let root = repo_root();
    let root_s = root.to_str().unwrap();
    let (code, json, _) = run(&["adapter", "test", "--json", "--adapter", "not-a-family", "--from", root_s]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("usage.invalid"));

    // An unimplemented family: every row executed, none implemented.
    let (code, json, out) = run(&["adapter", "test", "--json", "--adapter", "deepseek-harness", "--from", root_s]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.get("ran"), Some(&Value::Bool(true)));
    assert_eq!(json.get("decision").and_then(Value::as_str), Some("unimplemented"));
    assert_eq!(json.get("implemented_rows").and_then(Value::as_i64), Some(0));
    assert!(json.get("executed_rows").and_then(Value::as_i64).unwrap_or(0) >= 60, "{json:?}");
    assert_eq!(json.pointer(&["counts", "implemented_pass"]).and_then(Value::as_i64), Some(0));
    assert_eq!(json.pointer(&["counts", "unknown_honesty_pass"]).and_then(Value::as_i64), Some(0));
    assert_eq!(json.get("live_oracle_executed"), Some(&Value::Bool(false)));

    // The anchor family: implemented rows pass and the digests bind the run.
    let (code, json, out) = run(&["adapter", "test", "--json", "--adapter", "codex", "--from", root_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("decision").and_then(Value::as_str), Some("pass"));
    // C-F01: the `.ctxpect-ignore` row is a honesty pass and per the contract
    // does not count as implemented, so codex instructions is 2 + 1.
    assert!(json.get("implemented_rows").and_then(Value::as_i64).unwrap_or(0) >= 2, "{json:?}");
    assert_eq!(json.pointer(&["counts", "fail"]).and_then(Value::as_i64), Some(0));
    assert_eq!(
        json.get("matrix_digest").and_then(Value::as_str),
        Some(sha256_file(&root.join("acceptance/compatibility-matrix.yaml")).as_str())
    );
    let files = json.get("files").and_then(Value::as_array).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].get("path").and_then(Value::as_str), Some("acceptance/corpus/development/static/codex__cli.jsonl"));
    assert_eq!(
        files[0].get("sha256").and_then(Value::as_str),
        Some(sha256_file(&root.join("acceptance/corpus/development/static/codex__cli.jsonl")).as_str())
    );
    assert!(json.get("corpus_digest").and_then(Value::as_str).is_some_and(|d| d.len() == 64));
    assert_eq!(json.get("resolver_version").and_then(Value::as_str), Some(ctxpect_resolve::VERSION));
    assert!(json.pointer(&["not_executed", "sealed"]).and_then(Value::as_i64).unwrap_or(0) > 0);

    // Mutation: a tampered implemented row is reported by both paths.
    let scratch = Scratch::new("mutant");
    copy_codex_corpus(&scratch.path);
    let jsonl = scratch.path.join("acceptance/corpus/development/static/codex__cli.jsonl");
    let original = fs::read_to_string(&jsonl).unwrap();
    let tampered: Vec<String> = original
        .lines()
        .map(|line| {
            if line.contains("\"id\":\"dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:positive\"") {
                line.replacen("\"truth_state\":\"present\"", "\"truth_state\":\"absent\"", 1)
            } else {
                line.to_string()
            }
        })
        .collect();
    assert_ne!(tampered.join("\n"), original.trim_end());
    fs::write(&jsonl, tampered.join("\n") + "\n").unwrap();
    let (code, json, out) = run(&["adapter", "test", "--json", "--adapter", "codex", "--from", scratch.path.to_str().unwrap()]);
    assert_eq!(code, 2, "{out} {json:?}");
    assert_eq!(json.get("decision").and_then(Value::as_str), Some("fail"));
    assert!(json.get("failures").and_then(Value::as_array).is_some_and(|f| {
        f.iter().any(|item| item.as_str().is_some_and(|t| t.contains("instructions:positive")))
    }), "{json:?}");
    let gate = ctxpect_cli::conformance::run_static_corpus(&scratch.path, None).unwrap();
    assert!(gate.failures.iter().any(|f| f.contains("instructions:positive")), "{:?}", gate.failures);
    assert_eq!(gate.totals().fail, 1);
}

/// T4(f): the request-evidence endpoint follows the session's life. A native
/// artifact imported through the API is readable at `/sessions/:id/requests`
/// without any body; once the session is deleted, requests and insights are
/// gone with it.
#[test]
fn native_session_requests_follow_import_and_delete() {
    let scratch = Scratch::new("native-api");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["sessions.import", "sessions.delete"]);
    let (_child, listen) = start_daemon(&scratch, &store);
    let jsonl = fs::read_to_string(
        repo_root().join("acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1/session.jsonl"),
    )
    .unwrap();
    let body = canonical_json(&parse(&format!(
        r#"{{"session_id":"s-native","mapping_id":"deepseek-harness-cli","jsonl":{}}}"#,
        serde_quote(&jsonl)
    ))
    .unwrap());
    let (status, raw) = http_call(&listen, "POST", "/api/v1/sessions/import", &body);
    assert_eq!(status, 200, "{raw}");
    let (status, requests) = get_json(&listen, "/api/v1/sessions/s-native/requests");
    assert_eq!(status, 200, "{requests:?}");
    let list = requests.get("requests").and_then(Value::as_array).unwrap();
    assert_eq!(list.len(), 4);
    assert_eq!(list[0].get("dispatch_evidence").and_then(Value::as_i64), Some(6));
    // The header-less step reuses the previous snapshot and is still listed.
    assert_eq!(list[2].get("header_seq").and_then(Value::as_i64), Some(14));
    assert_eq!(list[2].get("header_logged_in_step"), Some(&Value::Bool(false)));
    assert_eq!(requests.get("bodies_stored"), Some(&Value::Bool(false)));
    assert_eq!(requests.pointer(&["tail", "interrupted"]), Some(&Value::Bool(true)));
    let text = canonical_json(&requests);
    assert!(!text.contains("List the files") && !text.contains("/tmp/ctxpect-fixture"), "{text}");
    // Without the artifact the mapping has nothing to read.
    let (status, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/sessions/import",
        r#"{"session_id":"s-bare","mapping_id":"deepseek-harness-cli"}"#,
    );
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("import_parse_failed"), "{raw}");

    // Delete through the CLI on the same store: the endpoint stops answering.
    let store_s = store.to_str().unwrap();
    let (code, json, out) = run(&[
        "sessions", "--json", "--store", store_s, "--project", scratch.path.to_str().unwrap(), "--session", "s-native", "--reason", "delete",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    let (status, raw) = http_call(&listen, "GET", "/api/v1/sessions/s-native/requests", "");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("store.missing"), "{raw}");
    let (status, raw) = http_call(&listen, "GET", "/api/v1/sessions/s-native", "");
    assert_eq!(status, 400, "{raw}");
    assert!(!store.join("insights/s-native.json").exists());
}

#[test]
fn rollback_preserves_a_later_edit_and_removes_a_created_file() {
    let scratch = Scratch::new("rbc");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["apply", "rollback"]);

    let (code, applied, out) = preview_then_apply(project, store_s, "AGENTS.md", "two\n");
    assert_eq!(code, 0, "{out} {applied:?}");
    let tx = applied
        .pointer(&["transaction", "tx_id"])
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    assert_eq!(
        applied.pointer(&["transaction", "state"]).and_then(Value::as_str),
        Some("committed")
    );
    scratch.write("AGENTS.md", "user edit after apply\n");
    let (code, json, out) = run(&["rollback", "--json", "--project", project, "--store", store_s, "--id", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.rollback_conflict"));
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "user edit after apply\n"
    );

    // A brand-new file: the transaction records it did not exist, and the
    // rollback deletes it rather than leaving an empty file behind.
    let (code, applied, out) = preview_then_apply(project, store_s, "docs/NEW.md", "fresh\n");
    assert_eq!(code, 0, "{out} {applied:?}");
    assert_eq!(
        applied.pointer(&["transaction", "existed_before"]).and_then(Value::as_bool),
        Some(false)
    );
    let tx = applied
        .pointer(&["transaction", "tx_id"])
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    assert_eq!(fs::read_to_string(scratch.path.join("docs/NEW.md")).unwrap(), "fresh\n");
    let (code, json, out) = run(&["rollback", "--json", "--project", project, "--store", store_s, "--id", &tx]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("action").and_then(Value::as_str), Some("removed-created-file"));
    assert!(!scratch.path.join("docs/NEW.md").exists(), "created file must be removed, not emptied");
}

/// C34: an imported session carrying a secret and an absolute path leaves
/// neither byte string anywhere in the store.
#[test]
fn import_keeps_no_secret_or_path_bytes_in_the_store() {
    let scratch = Scratch::new("imp");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, None, &["sessions.import"]);
    let secret = "sk-abcdefghijklmnopqrstuvwxyz0123";
    let home = "/Users/someone/private/notes";
    scratch.write(
        "sess.json",
        format!(
            r#"{{"events":[{{"type":"user","text":"token {secret} in {home}"}},{{"type":"{secret}","text":"x"}}]}}"#
        ),
    );
    let (code, json, out) = run(&[
        "import", "--json", "--store", store_s, "--from",
        scratch.path.join("sess.json").to_str().unwrap(), "--session", "s-priv", "--mapping",
        "codex-cli",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("bodies_stored").and_then(Value::as_bool), Some(false));

    fn walk(dir: &Path, needles: &[&str]) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, needles);
            } else if let Ok(text) = fs::read_to_string(&path) {
                for needle in needles {
                    assert!(!text.contains(needle), "{} carries {needle}", path.display());
                }
            }
        }
    }
    walk(&store, &[secret, home, "/Users/"]);

    // An undeclared mapping is refused rather than accepted as generic.
    let (code, json, _) = run(&[
        "import", "--json", "--store", store_s, "--from",
        scratch.path.join("sess.json").to_str().unwrap(), "--session", "s-map", "--mapping",
        "made-up",
    ]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("import.mapping_unknown"));
}

/// C35: a desired payload carrying a credential shape is refused at preview,
/// so no backup directory and no transaction record ever exist for it.
#[test]
fn a_secret_bearing_apply_is_refused_before_any_backup_exists() {
    let scratch = Scratch::new("secret");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["apply"]);
    let (code, json, out) = run(&[
        "intent", "preview", "--json", "--project", project, "--store", store_s, "--target",
        "AGENTS.md", "--desired", "token=ghp_fixture_not_a_real_secret_00\n",
    ]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.contains_secrets"));
    assert!(
        fs::read_dir(store.join("apply")).map(|d| d.count()).unwrap_or(0) == 0,
        "no backup may exist for a refused apply"
    );
    assert_eq!(fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(), "one\n");
    assert!(
        fs::read_dir(store.join("previews")).map(|d| d.count()).unwrap_or(0) == 0,
        "a refused preview is not persisted"
    );
}

/// C38: `ci` gates on the store's effective policy. No policy is exit 3,
/// a required deny is exit 2.
#[test]
fn ci_gates_on_store_policy_and_never_reads_no_policy_as_pass() {
    let scratch = Scratch::new("cigate");
    scratch.write("AGENTS.md", "hello\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();

    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 3, "{out} {json:?}");
    assert_eq!(json.get("policy_exit_code").and_then(Value::as_i64), Some(3));
    assert_eq!(
        json.pointer(&["policy", "reason_code"]).and_then(Value::as_str),
        Some("policy.unknown")
    );

    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(
        store.join("policies/active.json"),
        r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"deny"}]}]"#,
    )
    .unwrap();
    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 2, "{out} {json:?}");
    assert_eq!(json.get("policy_exit_code").and_then(Value::as_i64), Some(2));

    fs::write(store.join("policies/active.json"), PASS_LAYERS).unwrap();
    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(json.get("policy_exit_code").and_then(Value::as_i64), Some(0), "{out} {json:?}");
    // The inspect verdict still applies: the instructions are present, so
    // with a passing policy and no blocking finding the run is green.
    assert_eq!(code, 0, "{out} {json:?}");

    // Without `--store`, `ci` does not create one in the project.
    let (code, _, _) = run(&["ci", "--json", "--project", project]);
    assert_eq!(code, 3);
    assert!(!scratch.path.join(".ctxpect/store").exists(), "ci must not create a store");
}

/// C22: an exception needs an explicit lifetime and action, keeps its
/// reason, and expires.
#[test]
fn exception_request_requires_a_lifetime_and_the_exception_expires() {
    let scratch = Scratch::new("exp");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    let project_s = scratch.path.to_str().unwrap();
    enroll_principals(&scratch.path);

    let (code, json, out) = run_as(
        "alice-secret",
        &[
            "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-t", "--principal", "alice", "--action", "apply",
        ],
    );
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("usage.invalid"));
    assert!(!store.join("exceptions/ex-t.json").exists());

    let (code, json, out) = run_as(
        "alice-secret",
        &[
            "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-t", "--principal", "alice", "--expires-in", "1", "--reason", "hotfix",
        ],
    );
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("usage.invalid"), "--action is required");

    let (code, json, out) = run_as(
        "alice-secret",
        &[
            "exception", "request", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-t", "--principal", "alice", "--action", "apply", "--target", "AGENTS.md",
            "--expires-in", "1", "--reason", "hotfix",
        ],
    );
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("reason").and_then(Value::as_str), Some("hotfix"));
    assert_eq!(json.get("action").and_then(Value::as_str), Some("apply"));
    assert_eq!(json.get("target").and_then(Value::as_str), Some("AGENTS.md"));
    // `created_at` is a time field and is stripped from the printed envelope;
    // the stored record carries it.
    let stored = parse(&fs::read_to_string(store.join("exceptions/ex-t.json")).unwrap()).unwrap();
    let created = stored.get("created_at").and_then(Value::as_i64).unwrap();
    assert_eq!(stored.get("expires_at").and_then(Value::as_i64), Some(created + 1));

    let (code, _, out) = run_as(
        "carol-secret",
        &[
            "exception", "approve", "--json", "--store", store_s, "--project", project_s, "--id",
            "ex-t", "--principal", "carol",
        ],
    );
    assert_eq!(code, 0, "{out}");
    std::thread::sleep(std::time::Duration::from_millis(2100));
    let (code, json, out) = run(&[
        "exception", "status", "--json", "--store", store_s, "--project", project_s, "--id", "ex-t",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("state").and_then(Value::as_str), Some("expired"));
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("exception.expired"));
    assert_eq!(json.get("grants").and_then(Value::as_bool), Some(false));
}

/// C22: `offline_fresh` is computed from the policy source's refresh time,
/// not written as `true`. A stale policy source makes a live exception
/// `stale`, and it no longer grants.
#[test]
fn an_exception_over_a_stale_policy_source_does_not_grant() {
    let scratch = Scratch::new("stale");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["apply"]);
    let policy_file = store.join("policies/active.json");
    let sixty_days_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 86_400);
    fs::File::options()
        .write(true)
        .open(&policy_file)
        .unwrap()
        .set_modified(sixty_days_ago)
        .unwrap();

    let (code, json, out) = run(&[
        "exception", "status", "--json", "--store", store_s, "--project", project, "--id",
        "ex-planted-apply",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("state").and_then(Value::as_str), Some("stale"));
    assert_eq!(json.get("reason_code").and_then(Value::as_str), Some("exception.offline_stale"));
    let (code, json, out) = preview_then_apply(project, store_s, "AGENTS.md", "two\n");
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("policy.approval_required"));
    assert_eq!(fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(), "one\n");
}

/// C15: `adapter test`, `daemon stop` and `daemon status` no longer assert
/// actions that did not happen.
#[test]
fn constants_are_replaced_by_honest_unimplemented_answers() {
    let scratch = Scratch::new("honest");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();

    // `adapter test` without a family is a usage error, not a constant.
    let (code, json, out) = run(&["adapter", "test", "--json"]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("usage.invalid"));

    // No daemon: status says so from a probe, not from a pid file.
    fs::create_dir_all(&store).unwrap();
    fs::write(store.join("daemon.pid"), "424242").unwrap();
    let (code, json, out) = run(&["daemon", "status", "--json", "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("running").and_then(Value::as_bool), Some(false));
    assert_eq!(json.get("pid_file_present").and_then(Value::as_bool), Some(true));
    assert_eq!(json.get("probe").and_then(Value::as_str), Some("no_addr"));

    let (code, json, out) = run(&["daemon", "stop", "--json", "--store", store_s]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("daemon.control_missing"));
    assert_eq!(fs::read_to_string(store.join("daemon.pid")).unwrap(), "424242");

    // A live daemon: status finds it; stop confirms release of the owner lock.
    let (_child, _listen) = start_daemon(&scratch, &store);
    let (code, json, out) = run(&["daemon", "status", "--json", "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("running").and_then(Value::as_bool), Some(true));
    assert_eq!(json.get("probe").and_then(Value::as_str), Some("reachable"));
    let (code, json, out) = run(&["daemon", "stop", "--json", "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("stopped").and_then(Value::as_bool), Some(true));
    assert_eq!(json.get("signal_sent").and_then(Value::as_bool), Some(false));
    assert!(!store.join("daemon.control").exists());
}

/// C14: `intent validate|show|project|preview`, `sync status|preview` and
/// `align status|diff` are distinct operations with truthful `command`
/// fields, not aliases of one branch.
#[test]
fn subcommands_are_distinct_operations_not_aliases() {
    let scratch = Scratch::new("subs");
    scratch.write("AGENTS.md", "one\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();

    let (code, validate, out) = run(&[
        "intent", "validate", "--json", "--project", project, "--authority", "not-a-writer",
    ]);
    assert_eq!(code, 2, "{out} {validate:?}");
    assert_eq!(validate.get("command").and_then(Value::as_str), Some("intent validate"));
    assert_eq!(validate.get("valid").and_then(Value::as_bool), Some(false));
    assert_eq!(validate.get("reason_code").and_then(Value::as_str), Some("projection.authority"));

    let (code, show, _) = run(&["intent", "show", "--json", "--project", project, "--desired", "x\n"]);
    assert_eq!(code, 0);
    assert_eq!(show.get("command").and_then(Value::as_str), Some("intent show"));
    assert_eq!(show.get("schema").and_then(Value::as_str), Some("canonical-intent-v1"));
    assert!(show.get("tx_id").is_none(), "show renders the intent, not a preview");

    let (code, projected, _) = run(&["intent", "project", "--json", "--project", project, "--desired", "x\n"]);
    assert_eq!(code, 0);
    assert_eq!(projected.get("command").and_then(Value::as_str), Some("intent project"));
    assert_eq!(projected.get("persisted").and_then(Value::as_bool), Some(false));
    assert!(!store.join("previews").exists() || fs::read_dir(store.join("previews")).unwrap().count() == 0);

    let (code, previewed, _) = run(&[
        "intent", "preview", "--json", "--project", project, "--store", store_s, "--desired", "x\n",
    ]);
    assert_eq!(code, 0);
    assert_eq!(previewed.get("command").and_then(Value::as_str), Some("intent preview"));
    assert_eq!(previewed.get("persisted").and_then(Value::as_bool), Some(true));
    assert_eq!(fs::read_dir(store.join("previews")).unwrap().count(), 1);

    let (code, status, _) = run(&["sync", "status", "--json", "--store", store_s]);
    assert_eq!(code, 0);
    assert_eq!(status.get("command").and_then(Value::as_str), Some("sync status"));
    assert_eq!(status.get("schema").and_then(Value::as_str), Some("ctxpect-sync-status-v1"));
    let (code, preview, _) = run(&["sync", "preview", "--json", "--store", store_s, "--id", "b1"]);
    assert_eq!(code, 0);
    assert_eq!(preview.get("command").and_then(Value::as_str), Some("sync preview"));
    assert!(preview.get("bundle_id").is_some() || preview.get("receipts").is_some(), "{preview:?}");
    assert_ne!(canonical_json(&status), canonical_json(&preview));

    let (code, aligned, _) = run(&["align", "status", "--json", "--store", store_s]);
    assert_eq!(code, 0);
    assert_eq!(aligned.get("command").and_then(Value::as_str), Some("align status"));
    let (code, json, _) = run(&["align", "diff", "--json", "--store", store_s]);
    assert_eq!(code, 1);
    assert_eq!(err_code(&json), Some("usage.invalid"));
    assert_eq!(json.get("command").and_then(Value::as_str), Some("align"));
}

/// C29: a named analysis adapter that does not exist is refused, not echoed.
#[test]
fn advisor_refuses_an_unimplemented_adapter() {
    let (code, json, _) = run(&[
        "advisor", "--json", "--reason", "consent", "--text", "ack", "--adapter", "gpt-x",
    ]);
    assert_eq!(code, 1, "{json:?}");
    assert_eq!(err_code(&json), Some("advisor.adapter_unavailable"));
}

/// C8 / C9 / C17 / C40 over the daemon: routes are exact on method, the
/// catalog follows the session coordinate, imports do not overwrite each
/// other, and a diagnosis asked for by Receipt id stays that Receipt's.
#[test]
fn api_routes_are_exact_and_follow_the_session_coordinate() {
    let scratch = Scratch::new("routes");
    scratch.write("AGENTS.md", "hello\n");
    let store = scratch.path.join("store");
    fs::create_dir_all(&store).unwrap();
    plant_grants(&store, Some(&scratch.path), &["sessions.import"]);
    let (_child, listen) = start_daemon(&scratch, &store);

    // C17: a known path under another method is 405, not a fallthrough.
    let (status, raw) = http_call(&listen, "POST", "/api/v1/care-plan/f_x", "{}");
    assert_eq!(status, 405, "{raw}");
    assert!(raw.contains("api.method_not_allowed"), "{raw}");
    let (status, raw) = http_call(&listen, "DELETE", "/api/v1/receipts/r/verify", "");
    assert_eq!(status, 405, "{raw}");
    let (status, _) = http_call(&listen, "GET", "/api/v1/no-such-thing", "");
    assert_eq!(status, 400);

    // C9: the catalog follows the session coordinate.
    let (_, coord) = get_json(&listen, "/api/v1/coordinate");
    assert_eq!(coord.get("harness").and_then(Value::as_str), Some("codex"));
    let active = |catalog: &Value| -> Vec<String> {
        catalog
            .get("families")
            .and_then(Value::as_array)
            .unwrap_or(&[])
            .iter()
            .filter(|f| f.get("active_coordinate").and_then(Value::as_bool) == Some(true))
            .map(|f| f.get("family_id").and_then(Value::as_str).unwrap_or("").to_string())
            .collect()
    };
    let (_, catalog) = get_json(&listen, "/api/v1/integrations");
    assert_eq!(active(&catalog), vec!["codex".to_string()]);
    let (status, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/inspect",
        &format!(r#"{{"project":"{}","harness":"claude-code","version":"2.1.259"}}"#, scratch.path.display()),
    );
    assert_eq!(status, 200, "{raw}");
    let (_, coord) = get_json(&listen, "/api/v1/coordinate");
    assert_eq!(coord.get("harness").and_then(Value::as_str), Some("claude-code"));
    let (_, catalog) = get_json(&listen, "/api/v1/integrations");
    assert_eq!(active(&catalog), vec!["claude-code".to_string()]);
    let (_, entry) = get_json(&listen, "/api/v1/integrations/claude-code");
    assert_eq!(entry.get("active_coordinate").and_then(Value::as_bool), Some(true));

    // C8: two imports are two sessions; the id comes from the body or the
    // content digest, never a constant.
    let (status, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/sessions/import",
        r#"{"session_id":"s-one","mapping_id":"codex-cli","events":[{"type":"user","text":"a"}]}"#,
    );
    assert_eq!(status, 200, "{raw}");
    let (status, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/sessions/import",
        r#"{"events":[{"type":"user","text":"b"}]}"#,
    );
    assert_eq!(status, 200, "{raw}");
    let (_, sessions) = get_json(&listen, "/api/v1/sessions");
    let ids = sessions.get("sessions").and_then(Value::as_array).unwrap();
    assert_eq!(ids.len(), 2, "{sessions:?}");
    assert!(ids.iter().any(|id| id.as_str() == Some("s-one")));
    assert!(!ids.iter().any(|id| id.as_str() == Some("s_api")), "no hardcoded id");
    let (status, raw) = http_call(
        &listen,
        "POST",
        "/api/v1/sessions/import",
        r#"{"mapping_id":"made-up","events":[]}"#,
    );
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("import.mapping_unknown"), "{raw}");

    // C40: the diagnosis for an older Receipt is not overwritten by a newer
    // inspect when the caller names the Receipt.
    let project_body = format!(r#"{{"project":"{}","harness":"codex","version":"0.147.0"}}"#, scratch.path.display());
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &project_body);
    assert_eq!(status, 200, "{raw}");
    let first = http_json(&raw)
        .pointer(&["receipt", "receipt_id"])
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    scratch.write("AGENTS.md", "changed\n");
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &project_body);
    assert_eq!(status, 200, "{raw}");
    let second = http_json(&raw)
        .pointer(&["receipt", "receipt_id"])
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    assert_ne!(first, second);
    let (status, doc) = get_json(&listen, &format!("/api/v1/doctor?receipt_id={first}"));
    assert_eq!(status, 200, "{doc:?}");
    assert_eq!(doc.get("receipt_id").and_then(Value::as_str), Some(first.as_str()));
    let (_, current) = get_json(&listen, "/api/v1/doctor");
    assert_eq!(current.get("receipt_id").and_then(Value::as_str), Some(second.as_str()));
}

/// C1 / S4: `ci` and `doctor --fail-on` share one blocking judgement, and a
/// blocking corpus rule (a secret literal) turns the project red in both.
#[test]
fn ci_and_doctor_block_on_the_same_project_rule() {
    let scratch = Scratch::new("block");
    scratch.write("AGENTS.md", "hello\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(store.join("policies/active.json"), PASS_LAYERS).unwrap();

    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("doctor_exit_code").and_then(Value::as_i64), Some(0));

    // A credential shape in project text is a blocking finding.
    scratch.write("NOTES.md", "token=ghp_fixture_not_a_real_secret_42\n");
    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 2, "{out} {json:?}");
    assert_eq!(json.get("doctor_exit_code").and_then(Value::as_i64), Some(2));
    let findings = json.pointer(&["doctor", "findings"]).and_then(Value::as_array).unwrap();
    assert!(findings.iter().any(|f| {
        f.get("rule_id").and_then(Value::as_str) == Some("secret_literal")
            && f.get("blocking").and_then(Value::as_bool) == Some(true)
            && f.get("path").and_then(Value::as_str) == Some("NOTES.md")
    }), "{findings:?}");

    // The CLI doctor reports the same finding and the same exit.
    let (code, json, out) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 2, "{out} {json:?}");
    assert_eq!(json.pointer(&["counts", "blocking"]).and_then(Value::as_i64), Some(1));

    // A non-blocking rule (stale frontmatter) is reported without exit 2.
    fs::remove_file(scratch.path.join("NOTES.md")).unwrap();
    scratch.write("OLD.md", "---\nupdated: 2019-01-01\n---\nold\n");
    let (code, json, out) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 0, "{out} {json:?}");
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    assert!(findings.iter().any(|f| {
        f.get("rule_id").and_then(Value::as_str) == Some("stale")
            && f.get("blocking").and_then(Value::as_bool) == Some(false)
    }), "{findings:?}");
    // `--fail-on confirmed` still fails on a confirmed finding; same function.
    let (code, _, _) = run(&["doctor", "--json", "--project", project, "--fail-on", "confirmed"]);
    assert_eq!(code, 2);
}

/// C-F05: `stale` is judged against an explicit `as_of`. The CLI defaults to
/// the wall-clock date and `--as-of YYYY-MM-DD` pins it (suppression expiry
/// follows the pinned date's noon-UTC reading); an impossible date fails
/// closed at parse.
#[test]
fn doctor_stale_is_judged_against_the_explicit_as_of_date() {
    let scratch = Scratch::new("asof");
    scratch.write("AGENTS.md", "hello\n");
    // 359 days before 2026-09-04, 366 days before 2026-09-11.
    scratch.write("OLD.md", "---\nupdated: 2025-09-10\n---\nold\n");
    // Stale under any of the dates used here; carries the suppression.
    scratch.write("STALE.md", "---\nupdated: 2019-01-01\n---\nolder\n");
    scratch.write(
        ".ctxpect/doctor-suppressions.json",
        r#"{"schema":"ctxpect-doctor-suppressions-v1","suppressions":[{"rule_id":"stale","path":"STALE.md","owner":"alice","reason":"accepted legacy doc","expires_at":"2026-09-05T00:00:00Z"}]}"#,
    );
    let project = scratch.path.to_str().unwrap();

    // As of the frozen reference date OLD.md is fresh; the suppression on
    // STALE.md is still valid (expiry is past the date's noon-UTC reading).
    let (code, json, out) = run(&["doctor", "--json", "--project", project, "--as-of", "2026-09-04"]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("as_of").and_then(Value::as_str), Some("2026-09-04"));
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    assert!(
        !findings.iter().any(|f| f.get("rule_id").and_then(Value::as_str) == Some("stale")
            && f.get("path").and_then(Value::as_str) == Some("OLD.md")),
        "{findings:?}"
    );
    let suppressed = findings
        .iter()
        .find(|f| f.get("rule_id").and_then(Value::as_str) == Some("stale")
            && f.get("path").and_then(Value::as_str) == Some("STALE.md"))
        .expect("STALE.md stale finding");
    assert_eq!(suppressed.get("suppressed").and_then(Value::as_bool), Some(true), "{suppressed:?}");
    // 2026-09-04T12:00:00Z: the pinned date's noon-UTC reading.
    assert_eq!(
        json.pointer(&["suppressions", "evaluated_at_secs"]).and_then(Value::as_i64),
        Some(1_788_523_200)
    );

    // One week later the same bytes cross the 365-day threshold, the finding
    // names the date it was judged against, and the suppression is expired,
    // so it suppresses nothing and is itself reported.
    let (code, json, out) = run(&["doctor", "--json", "--project", project, "--as-of", "2026-09-11"]);
    assert_eq!(code, 0, "{out} {json:?}");
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    let stale = findings
        .iter()
        .find(|f| f.get("rule_id").and_then(Value::as_str) == Some("stale")
            && f.get("path").and_then(Value::as_str) == Some("OLD.md"))
        .expect("OLD.md stale finding");
    assert_eq!(stale.get("blocking").and_then(Value::as_bool), Some(false));
    assert!(
        stale.get("title").and_then(Value::as_str).unwrap().contains("2026-09-11"),
        "{stale:?}"
    );
    let revived = findings
        .iter()
        .find(|f| f.get("rule_id").and_then(Value::as_str) == Some("stale")
            && f.get("path").and_then(Value::as_str) == Some("STALE.md"))
        .expect("STALE.md stale finding");
    assert_eq!(revived.get("suppressed").and_then(Value::as_bool), Some(false), "{revived:?}");
    assert!(findings.iter().any(|f| {
        f.get("rule_id").and_then(Value::as_str) == Some("doctor.suppression_invalid")
    }), "{findings:?}");

    // Without `--as-of` the wall clock is the reference: 2019-01-01 is stale
    // whenever this test runs, and the output names the actual date.
    let (code, json, out) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 0, "{out} {json:?}");
    let as_of = json.get("as_of").and_then(Value::as_str).expect("as_of named");
    assert!(as_of.len() == 10 && as_of.as_bytes()[4] == b'-', "{as_of}");
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    assert!(findings.iter().any(|f| f.get("rule_id").and_then(Value::as_str) == Some("stale")
        && f.get("path").and_then(Value::as_str) == Some("OLD.md")), "{findings:?}");

    // An impossible date is refused at parse: exit 1, usage.invalid.
    let (code, json, _) = run(&["doctor", "--json", "--project", project, "--as-of", "2026-02-30"]);
    assert_eq!(code, 1, "{json:?}");
    assert_eq!(
        json.pointer(&["error", "code"]).and_then(Value::as_str),
        Some("usage.invalid")
    );
}

/// D2: a suppression is an owned, time-boxed, evidence-bound acceptance of
/// a finding. The finding stays reported; only the active counts and the
/// exit change; one changed byte revives it; CLI, ci and API answer alike.
#[test]
fn a_suppression_keeps_the_finding_visible_and_is_bound_to_the_reviewed_bytes() {
    let scratch = Scratch::new("suppress");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("NOTES.md", "token=ghp_fixture_not_a_real_secret_77\n");
    let project = scratch.path.to_str().unwrap();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap();
    fs::create_dir_all(store.join("policies")).unwrap();
    fs::write(store.join("policies/active.json"), PASS_LAYERS).unwrap();

    let (code, _, _) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 2);

    let digest = ctxpect_schema::sha256_hex(b"token=ghp_fixture_not_a_real_secret_77\n");
    let suppressions = format!(
        r#"{{"schema":"ctxpect-doctor-suppressions-v1","suppressions":[{{"rule_id":"secret_literal","path":"NOTES.md","owner":"alice","reason":"documentation fixture token","expires_at":"2099-01-01T00:00:00Z","evidence_digest":"{digest}"}}]}}"#
    );
    scratch.write(".ctxpect/doctor-suppressions.json", &suppressions);
    let (code, json, out) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 0, "{out} {json:?}");
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    let secret = findings
        .iter()
        .find(|f| f.get("rule_id").and_then(Value::as_str) == Some("secret_literal"))
        .expect("the finding is still reported");
    assert_eq!(secret.get("suppressed"), Some(&Value::Bool(true)));
    assert_eq!(secret.get("confirmation").and_then(Value::as_str), Some("confirmed"));
    assert_eq!(secret.pointer(&["suppression", "owner"]).and_then(Value::as_str), Some("alice"));
    assert_eq!(json.pointer(&["counts", "blocking"]).and_then(Value::as_i64), Some(1));
    assert_eq!(json.pointer(&["counts", "active_blocking"]).and_then(Value::as_i64), Some(0));
    assert_eq!(json.pointer(&["suppressions", "applied"]).and_then(Value::as_i64), Some(1));
    assert_eq!(json.pointer(&["suppressions", "file_digest"]).and_then(Value::as_str).map(str::len), Some(64));
    // ci judges the same active counts.
    let (code, json, out) = run(&["ci", "--json", "--project", project, "--store", store_s]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("doctor_exit_code").and_then(Value::as_i64), Some(0));

    // The API gives the same answer (R04).
    let (mut child, listen) = start_daemon(&scratch, &store);
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &format!(r#"{{"project":"{project}"}}"#));
    assert_eq!(status, 200, "{raw}");
    let (status, diag) = get_json(&listen, "/api/v1/doctor");
    assert_eq!(status, 200, "{diag:?}");
    assert_eq!(diag.pointer(&["counts", "active_blocking"]).and_then(Value::as_i64), Some(0));
    assert_eq!(diag.pointer(&["suppressions", "applied"]).and_then(Value::as_i64), Some(1));
    let _ = child.child().kill();

    // One changed byte: the acceptance no longer covers the bytes on disk.
    scratch.write("NOTES.md", "token=ghp_fixture_not_a_real_secret_78\n");
    let (code, json, out) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 2, "{out} {json:?}");
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    assert!(findings.iter().any(|f| {
        f.get("rule_id").and_then(Value::as_str) == Some("doctor.suppression_invalid")
            && f.get("blocking") == Some(&Value::Bool(false))
    }), "{findings:?}");
    assert_eq!(json.pointer(&["counts", "active_blocking"]).and_then(Value::as_i64), Some(1));
    // An expired suppression is invalid the same way.
    scratch.write("NOTES.md", "token=ghp_fixture_not_a_real_secret_77\n");
    scratch.write(".ctxpect/doctor-suppressions.json", suppressions.replace("2099-01-01", "2001-01-01"));
    let (code, json, _) = run(&["doctor", "--json", "--project", project]);
    assert_eq!(code, 2);
    assert_eq!(json.pointer(&["suppressions", "invalid"]).and_then(Value::as_i64), Some(1));
}

/// D4: the daemon observes only the root it was started for; Receipts are
/// bound to the root they were observed in and an explicit `receipt_id` is
/// never quietly replaced by the current one.
#[test]
fn the_daemon_scans_only_its_root_and_reads_receipts_only_from_it() {
    let scratch = Scratch::new("scope");
    scratch.write("AGENTS.md", "hello\n");
    scratch.write("sub/AGENTS.md", "inner\n");
    let store = scratch.path.join("store");
    let (mut child, listen) = start_daemon(&scratch, &store);
    let project = scratch.path.to_str().unwrap().to_string();

    // Inside the root: fine. Outside: refused, nothing observed.
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &format!(r#"{{"project":"{project}/sub"}}"#));
    assert_eq!(status, 200, "{raw}");
    let receipts_before = fs::read_dir(store.join("receipts")).unwrap().count();
    for outside in [std::env::temp_dir().to_str().unwrap().to_string(), format!("{project}/../"), "/".to_string()] {
        let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &format!(r#"{{"project":"{outside}"}}"#));
        assert_eq!(status, 400, "{outside}: {raw}");
        assert!(raw.contains("api.project_scope"), "{raw}");
    }
    // A symlink inside the root pointing outside is still outside.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(std::env::temp_dir(), scratch.path.join("out-link")).unwrap();
        let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &format!(r#"{{"project":"{project}/out-link"}}"#));
        assert_eq!(status, 400, "{raw}");
    }
    // A codex_home the daemon was not started with is refused too.
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &format!(r#"{{"project":"{project}","codex_home":"{project}/sub"}}"#));
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("api.project_scope"), "{raw}");
    assert_eq!(fs::read_dir(store.join("receipts")).unwrap().count(), receipts_before, "refused requests observe nothing");

    // A Receipt observed in another project (same store) is not rescanned
    // against this root; an unknown id is an error, not the current one.
    let other = Scratch::new("scope-other");
    other.write("AGENTS.md", "elsewhere\n");
    let (code, json, out) = run(&["inspect", "--json", "--project", other.path.to_str().unwrap(), "--store", store.to_str().unwrap()]);
    assert_eq!(code, 0, "{out} {json:?}");
    let foreign = json.get("formal_receipt_id").and_then(Value::as_str).unwrap().to_string();
    let (status, raw) = http_call(&listen, "GET", &format!("/api/v1/doctor?receipt_id={foreign}"), "");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("api.receipt_scope"), "{raw}");
    let (status, monitor) = get_json(&listen, &format!("/api/v1/monitor?receipt_id={foreign}"));
    assert_eq!(status, 200);
    assert_eq!(monitor.pointer(&["staleness", "reason_code"]).and_then(Value::as_str), Some("api.receipt_scope"));
    let (status, raw) = http_call(&listen, "GET", "/api/v1/care-plan/f_x?receipt_id=r_nope", "");
    assert_eq!(status, 400, "{raw}");
    assert!(raw.contains("store.missing"), "{raw}");
    let _ = child.child().kill();
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
    start_daemon_with_home(scratch, store, None)
}

fn start_daemon_with_home(scratch: &Scratch, store: &Path, home: Option<&Path>) -> (ChildGuard, String) {
    use std::thread;
    use std::time::Duration;

    let mut command = Command::new(bin());
    command.args([
                "daemon",
                "start",
                "--project",
                scratch.path.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--listen",
                "127.0.0.1:0",
            ]);
    if let Some(home) = home {
        command.arg("--codex-home").arg(home);
    }
    let mut child = ChildGuard::new(command.spawn().expect("daemon"));
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

#[test]
fn browser_inspect_inherits_only_the_home_granted_at_daemon_start() {
    let project = Scratch::new("http-inherit-project");
    let home = Scratch::new("http-inherit-home");
    project.write("AGENTS.md", "project rules\n");
    home.write("AGENTS.md", "global rules\n");
    let store = project.path.join("store");
    let (_daemon, listen) = start_daemon_with_home(&project, &store, Some(&home.path));
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", "{}");
    assert_eq!(status, 200, "{raw}");
    let receipt = http_json(&raw).get("receipt").cloned().unwrap();
    let layers = receipt.get("layers").and_then(Value::as_array).unwrap();
    let global = layers.iter().find(|layer| layer.get("id").and_then(Value::as_str) == Some("global")).unwrap();
    assert_eq!(global.get("adopted").and_then(Value::as_str), Some("AGENTS.md"));
    assert_eq!(global.get("unknown_reason_code"), Some(&Value::Null));
    let (status, raw) = http_call(&listen, "POST", "/api/v1/inspect", &format!(r#"{{"codex_home":"{}"}}"#, project.path.display()));
    assert_ne!(status, 200, "unexpected scope expansion: {raw}");
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
    plant_grants(&store, Some(&scratch.path), &["settings.put"]);
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
    plant_grants(
        &store,
        Some(&scratch.path),
        &["assets.copy", "assets.rollback", "apply", "rollback"],
    );

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
    let (code, applied, out) = preview_then_apply(project_s, store_s, "AGENTS.md", "updated\n");
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
    plant_grants(&store, Some(&scratch.path), &["standard.publish"]);

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
                "--id", "ex-planted-standard-publish",
            ],
            "/api/v1/exceptions/ex-planted-standard-publish",
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
    plant_grants(&store, Some(&scratch.path), &["assets.copy", "assets.rollback"]);

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

    // A repeated CLI copy owns a new backup. Roll back newest first even
    // when both copies installed the same bytes, then restore prior provenance.
    let first_record = fs::read(store.join(format!("apply/{tx}/tx.json"))).unwrap();
    let (code, second, out) = copy();
    assert_eq!(code, 0, "{out}");
    let tx2 = second.pointer(&["transaction", "tx_id"]).and_then(Value::as_str).unwrap();
    assert_ne!(tx2, tx);
    assert_eq!(fs::read(store.join(format!("apply/{tx}/tx.json"))).unwrap(), first_record);
    let (code, refusal, out) = run(&[
        "assets", "rollback", "--json", "--project", project_s, "--store", store_s, "--id", &tx,
    ]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(err_code(&refusal), Some("assets.lock_conflict"));
    let (code, _, out) = run(&[
        "assets", "rollback", "--json", "--project", project_s, "--store", store_s, "--id", tx2,
    ]);
    assert_eq!(code, 0, "{out}");
    let lock = parse(&fs::read_to_string(store.join("assetlock/skill-a.json")).unwrap()).unwrap();
    assert_eq!(lock.get("tx_id").and_then(Value::as_str), Some(tx.as_str()));

    // Rollback removes the file the copy created.
    let (code, json, out) = run(&[
        "assets", "rollback", "--json", "--project", project_s, "--store", store_s, "--id", &tx,
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert!(!scratch.path.join(".ctxpect/skills/skill-a.md").exists());
    let (code, sbom, out) = run(&["assets", "sbom", "--json", "--store", store_s]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(sbom.get("component_count"), Some(&Value::Int(0)));
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
    plant_grants(&store, Some(&scratch.path), &["assets.copy", "assets.rollback"]);
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", "{}");
    assert_eq!(status, 400, "{raw}");
    assert_eq!(err_code(&http_json(&raw)), Some("assets.preview_required"));
    let preview_id = plan.get("tx_id").and_then(Value::as_str).unwrap();
    let body = format!(r#"{{"preview_id":"{preview_id}","target_rel":"/etc/passwd"}}"#);
    // A target edited after the visible preview must survive the API call.
    scratch.write(".ctxpect/skills/skill-a.md", "user edit after preview\n");
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
    assert_eq!(status, 400, "{raw}");
    assert_eq!(err_code(&http_json(&raw)), Some("assets.concurrent_hash"));
    assert_eq!(fs::read_to_string(scratch.path.join(".ctxpect/skills/skill-a.md")).unwrap(), "user edit after preview\n");

    let (_, fresh) = http_call(&listen, "POST", "/api/v1/assets/skill-a/preview", "{}");
    let fresh = http_json(&fresh);
    let tx = fresh.get("tx_id").and_then(Value::as_str).unwrap();
    assert_ne!(tx, preview_id);
    let body = format!(r#"{{"preview_id":"{tx}"}}"#);
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
    assert_eq!(status, 200, "{raw}");
    let copied = http_json(&raw);
    assert_eq!(copied.get("runtime_verification").and_then(Value::as_str), Some("not-observed"));
    let post = copied.get("post_receipt_id").and_then(Value::as_str).expect("post Receipt");
    let (status, _) = get_json(&listen, &format!("/api/v1/receipts/{post}"));
    assert_eq!(status, 200);
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
    assert_eq!(status, 400, "{raw}");
    assert_eq!(err_code(&http_json(&raw)), Some("assets.preview_consumed"));
    assert_eq!(fs::read_to_string(store.join(format!("apply/{tx}/before"))).unwrap_or_default(), "user edit after preview\n");
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
    // Rollback cannot overwrite a later external edit, and successful rollback
    // appends its own observation without changing the copy's old Receipt.
    let old_receipt = fs::read(store.join(format!("receipts/{post}.json"))).unwrap();
    scratch.write(".ctxpect/skills/skill-a.md", "later edit\n");
    let (status, raw) = http_call(&listen, "POST", &format!("/api/v1/assets/{tx}/rollback"), "{}");
    assert_eq!(status, 400, "{raw}");
    assert_eq!(err_code(&http_json(&raw)), Some("assets.rollback_conflict"));
    scratch.write(".ctxpect/skills/skill-a.md", "skill body\n");
    let (status, raw) = http_call(&listen, "POST", &format!("/api/v1/assets/{tx}/rollback"), "{}");
    assert_eq!(status, 200, "{raw}");
    assert!(http_json(&raw).get("post_receipt_id").is_some());
    assert_eq!(fs::read(store.join(format!("receipts/{post}.json"))).unwrap(), old_receipt);
    assert_eq!(fs::read_to_string(scratch.path.join(".ctxpect/skills/skill-a.md")).unwrap(), "user edit after preview\n");
    let (_, overview) = get_json(&listen, "/api/v1/assets");
    assert_eq!(overview.pointer(&["sbom", "component_count"]), Some(&Value::Int(0)));

    // Metadata changes need a new preview even when the source bytes match.
    let (_, fresh) = http_call(&listen, "POST", "/api/v1/assets/skill-a/preview", "{}");
    let fresh = http_json(&fresh);
    let tx = fresh.get("tx_id").and_then(Value::as_str).unwrap();
    let body = format!(r#"{{"preview_id":"{tx}"}}"#);
    let registry_path = scratch.path.join(".ctxpect/assets.json");
    let registry = fs::read_to_string(&registry_path).unwrap();
    fs::write(&registry_path, registry.replace("MIT", "Apache-2.0")).unwrap();
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
    assert_eq!(status, 400, "{raw}");
    assert_eq!(err_code(&http_json(&raw)), Some("assets.preview_changed"));
    fs::write(&registry_path, registry).unwrap();

    let preview_path = store.join(format!("assetpreviews/{tx}.json"));
    let record = parse(&fs::read_to_string(&preview_path).unwrap()).unwrap();
    for (key, value, reason) in [
        ("expires_at", Value::Int(1), "assets.preview_expired"),
        ("project_digest", ctxpect_schema::string("different-project"), "assets.preview_scope"),
    ] {
        let mut changed = record.clone();
        if let Value::Object(ref mut fields) = changed { fields.insert(key.into(), value); }
        fs::write(&preview_path, canonical_json(&changed)).unwrap();
        let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
        assert_eq!(status, 400, "{raw}");
        assert_eq!(err_code(&http_json(&raw)), Some(reason));
    }
    fs::write(&preview_path, canonical_json(&record)).unwrap();

    // A post-observation storage failure is not reported as a failed write:
    // the caller sees the committed transaction and must not repeat it.
    fs::rename(store.join("receipts"), store.join("saved-receipts")).unwrap();
    fs::write(store.join("receipts"), "blocked for fault injection").unwrap();
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
    assert_eq!(status, 200, "{raw}");
    let copied = http_json(&raw);
    assert_eq!(copied.get("static_verification").and_then(Value::as_str), Some("unavailable"));
    assert!(copied.get("post_receipt_error").is_some());
    assert_eq!(fs::read_to_string(scratch.path.join(".ctxpect/skills/skill-a.md")).unwrap(), "skill body\n");
    fs::remove_file(store.join("receipts")).unwrap();
    fs::rename(store.join("saved-receipts"), store.join("receipts")).unwrap();
    let (_, fresh) = http_call(&listen, "POST", "/api/v1/assets/skill-a/preview", "{}");
    let fresh = http_json(&fresh);
    let tx = fresh.get("tx_id").and_then(Value::as_str).unwrap();
    let body = format!(r#"{{"preview_id":"{tx}"}}"#);
    fs::rename(store.join("assetlock"), store.join("saved-assetlock")).unwrap();
    fs::write(store.join("assetlock"), "blocked for fault injection").unwrap();
    let before = fs::read(scratch.path.join(".ctxpect/skills/skill-a.md")).unwrap();
    let (status, raw) = http_call(&listen, "POST", "/api/v1/assets/skill-a/copy", &body);
    assert_eq!(status, 400, "{raw}");
    assert_eq!(fs::read(scratch.path.join(".ctxpect/skills/skill-a.md")).unwrap(), before);
    assert!(!store.join(format!("apply/{tx}")).exists(), "prior lock must be readable before writing");
    fs::remove_file(store.join("assetlock")).unwrap();
    fs::rename(store.join("saved-assetlock"), store.join("assetlock")).unwrap();
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
    plant_grants(&store, Some(&scratch.path), &["sync.apply"]);

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
    plant_grants(&store, Some(&scratch.path), &["standard.publish", "standard.adopt"]);

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
    // The two planted exceptions are live and are counted as granting.
    assert_eq!(
        compliance.pointer(&["exception", "live"]).and_then(Value::as_i64),
        Some(2),
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

            // Preview first (read-only for the project), then apply the
            // persisted preview. A client-supplied `approved` is not input.
            let preview_body = r#"{"desired":"PWNED\n","target":"AGENTS.md"}"#;
            let (pvstatus, pvraw) = http_call(&listen, "POST", "/api/v1/intent/preview", preview_body);
            assert_eq!(pvstatus, 200, "{pvraw}");
            let pwned_tx = http_json(&pvraw)
                .get("tx_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            assert!(!pwned_tx.is_empty(), "{pvraw}");
            let apply_body = format!(r#"{{"approved":true,"tx_id":"{pwned_tx}"}}"#);
            let (astatus, araw) = http_call(&listen, "POST", "/api/v1/apply", &apply_body);
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
                r#"{"tx_id":"tx_0000000000000000","target":"AGENTS.md"}"#,
            );
            assert_eq!(rstatus, 400, "{rraw}");
            assert!(rraw.contains("policy.unknown"), "{rraw}");
            assert_eq!(
                fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
                "hello\n"
            );

            plant_grants(
                &store,
                Some(&scratch.path),
                &["apply", "settings.put", "sessions.import", "rollback"],
            );
            let (pstatus, praw) = http_call(&listen, "GET", "/api/v1/policy", "");
            let policy_ok = http_json(&praw);
            assert_eq!(pstatus, 200, "{praw}");
            assert_eq!(
                policy_ok.get("mutation_allowed").and_then(Value::as_bool),
                Some(true),
                "{praw}"
            );

            let (pvstatus, pvraw) = http_call(
                &listen,
                "POST",
                "/api/v1/intent/preview",
                r#"{"desired":"from-http\n","target":"AGENTS.md"}"#,
            );
            assert_eq!(pvstatus, 200, "{pvraw}");
            let ok_tx = http_json(&pvraw)
                .get("tx_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let apply_ok_body = format!(r#"{{"tx_id":"{ok_tx}"}}"#);
            let (astatus, araw) = http_call(&listen, "POST", "/api/v1/apply", &apply_ok_body);
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

            let (ccode, cjson, cout) = preview_then_apply(
                scratch.path.to_str().unwrap(),
                store.to_str().unwrap(),
                "AGENTS.md",
                "from-cli\n",
            );
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


#[test]
fn frontend_bootstrap_is_scoped_read_only_and_preserves_unknown() {
    let scratch = Scratch::new("frontend-bootstrap");
    scratch.write("AGENTS.md", "hello bootstrap\n");
    let store = scratch.path.join("store");
    let (child, listen) = start_daemon(&scratch, &store);
    let (code, empty) = get_json(&listen, "/api/v1/status");
    assert_eq!(code, 200, "{empty:?}");
    let schema = parse(include_str!("../../../docs/schemas/ctxpect-status-v1.schema.json")).unwrap();
    ctxpect_schema::validate(&schema, &empty).unwrap();
    assert_eq!(empty.get("selected_receipt"), Some(&Value::Null));
    assert_eq!(empty.get("doctor_counts"), Some(&Value::Null));
    assert_eq!(empty.pointer(&["staleness", "status"]).and_then(Value::as_str), Some("unknown"));
    assert_eq!(empty.pointer(&["daemon", "listen"]).and_then(Value::as_str), Some(listen.as_str()));
    assert!(!canonical_json(&empty).contains(scratch.path.to_str().unwrap()));
    assert_eq!(get_json(&listen, "/api/v1/receipts").1.get("receipts").and_then(Value::as_array).unwrap().len(), 0);

    let (code, raw) = http_call(&listen, "POST", "/api/v1/inspect", "{}");
    assert_eq!(code, 200, "{raw}");
    let receipt = http_json(&raw).get("receipt").unwrap().clone();
    let id = receipt.get("receipt_id").and_then(Value::as_str).unwrap();
    let (code, status) = get_json(&listen, "/api/v1/status");
    assert_eq!(code, 200, "{status:?}");
    ctxpect_schema::validate(&schema, &status).unwrap();
    let mut invalid = status.clone();
    if let Value::Object(map) = &mut invalid {
        map.insert("staleness".into(), parse(r#"{"status":"fresh","reason_code":"fake"}"#).unwrap());
    }
    assert!(ctxpect_schema::validate(&schema, &invalid).is_err());
    assert_eq!(status.get("selection").and_then(Value::as_str), Some("session-current"));
    assert_eq!(status.pointer(&["selected_receipt", "receipt_id"]).and_then(Value::as_str), Some(id));
    assert_eq!(status.pointer(&["staleness", "status"]).and_then(Value::as_str), Some("current"));
    assert_eq!(status.get("doctor_counts"), get_json(&listen, &format!("/api/v1/doctor?receipt_id={id}")).1.get("counts"));
    scratch.write("AGENTS.md", "changed bootstrap\n");
    assert_eq!(get_json(&listen, "/api/v1/status").1.pointer(&["staleness", "status"]).and_then(Value::as_str), Some("stale"));
    fs::remove_file(scratch.path.join("AGENTS.md")).unwrap();
    assert_eq!(get_json(&listen, "/api/v1/status").1.pointer(&["staleness", "status"]).and_then(Value::as_str), Some("unknown"));
    scratch.write("AGENTS.md", "hello bootstrap\n");
    // A newer Receipt from another harness must not win after a restart.
    let (code, raw) = http_call(&listen, "POST", "/api/v1/inspect", r#"{"harness":"claude-code","version":"2.1.259"}"#);
    assert_eq!(code, 200, "{raw}");
    drop(child);
    let _ = fs::remove_file(store.join("daemon.addr"));
    let (child, listen) = start_daemon(&scratch, &store);
    let status = get_json(&listen, "/api/v1/status").1;
    assert_eq!(status.get("selection").and_then(Value::as_str), Some("latest-matching"), "{status:?}");
    assert_eq!(status.pointer(&["selected_receipt", "receipt_id"]).and_then(Value::as_str), Some(id));
    // Selecting history for the browser does not mutate the daemon session.
    assert_eq!(get_json(&listen, "/api/v1/doctor").1.pointer(&["error", "code"]).and_then(Value::as_str), Some("api.no_current_receipt"));
    drop(child);
    let _ = fs::remove_file(store.join("daemon.addr"));
    let foreign = Scratch::new("frontend-foreign");
    foreign.write("AGENTS.md", "other project\n");
    let (child, listen) = start_daemon(&foreign, &store);
    assert_eq!(get_json(&listen, "/api/v1/status").1.get("selected_receipt"), Some(&Value::Null));
    drop(child);
    let _ = fs::remove_file(store.join("daemon.addr"));
    let store_handle = Store::open(&store).unwrap();
    store_handle.delete_receipt(id, "test").unwrap();
    let (_child, listen) = start_daemon(&scratch, &store);
    assert_eq!(get_json(&listen, "/api/v1/status").1.get("selected_receipt"), Some(&Value::Null));
    fs::write(store.join("index.json"), "broken").unwrap();
    assert_eq!(get_json(&listen, "/api/v1/status").1.pointer(&["error", "code"]).and_then(Value::as_str), Some("store.index_corrupt"));
}

/// A tombstoned *current* Receipt is not a 400: status falls back to the
/// latest matching history, and to an honest empty selection when no
/// candidate remains.
#[test]
fn status_falls_back_to_history_when_the_current_receipt_is_tombstoned() {
    let scratch = Scratch::new("status-tombstone");
    scratch.write("AGENTS.md", "v1\n");
    let store = scratch.path.join("store");
    let (child, listen) = start_daemon(&scratch, &store);
    let (code, raw) = http_call(&listen, "POST", "/api/v1/inspect", "{}");
    assert_eq!(code, 200, "{raw}");
    let first = http_json(&raw)
        .pointer(&["receipt", "receipt_id"])
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    scratch.write("AGENTS.md", "v2\n");
    let (code, raw) = http_call(&listen, "POST", "/api/v1/inspect", "{}");
    assert_eq!(code, 200, "{raw}");
    let second = http_json(&raw)
        .pointer(&["receipt", "receipt_id"])
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    assert_ne!(first, second);
    let (code, status) = get_json(&listen, "/api/v1/status");
    assert_eq!(code, 200, "{status:?}");
    assert_eq!(status.get("selection").and_then(Value::as_str), Some("session-current"));
    assert_eq!(
        status.pointer(&["selected_receipt", "receipt_id"]).and_then(Value::as_str),
        Some(second.as_str())
    );
    assert_eq!(
        status.get("diagnosis_basis").and_then(Value::as_str),
        Some("receipt-and-current-project-scan")
    );

    // Deleting the current Receipt falls back to the latest matching one.
    let store_handle = Store::open(&store).unwrap();
    store_handle.delete_receipt(&second, "test").unwrap();
    let (code, status) = get_json(&listen, "/api/v1/status");
    assert_eq!(code, 200, "{status:?}");
    assert_eq!(status.get("selection").and_then(Value::as_str), Some("latest-matching"), "{status:?}");
    assert_eq!(
        status.pointer(&["selected_receipt", "receipt_id"]).and_then(Value::as_str),
        Some(first.as_str())
    );

    // With no matching history left, the selection is honestly `none` and
    // no diagnosis basis is claimed.
    store_handle.delete_receipt(&first, "test").unwrap();
    let (code, status) = get_json(&listen, "/api/v1/status");
    assert_eq!(code, 200, "{status:?}");
    assert_eq!(status.get("selection").and_then(Value::as_str), Some("none"));
    assert_eq!(status.get("selected_receipt"), Some(&Value::Null));
    assert_eq!(status.get("diagnosis_basis"), Some(&Value::Null));
    drop(child);
    let _ = fs::remove_file(store.join("daemon.addr"));
}

#[test]
fn frontend_sessions_metadata_is_additive_and_has_no_bodies() {
    let scratch = Scratch::new("frontend-sessions");
    let store = scratch.path.join("store");
    let store_handle = Store::open(&store).unwrap();
    let record = ctxpect_importer::import_session_bytes(
        br#"{"events":[{"type":"user","text":"PRIVATE_SENTINEL"}]}"#,
        "codex-cli", "s-meta",
    ).unwrap();
    store_handle.put_named("sessions", "s-meta", &record).unwrap();
    let (_child, listen) = start_daemon(&scratch, &store);
    let (code, data) = get_json(&listen, "/api/v1/sessions");
    assert_eq!(code, 200, "{data:?}");
    assert_eq!(data.get("sessions").and_then(Value::as_array).unwrap()[0].as_str(), Some("s-meta"));
    let summary = &data.get("session_summaries").and_then(Value::as_array).unwrap()[0];
    assert_eq!(summary.get("mapping_id").and_then(Value::as_str), Some("codex-cli"));
    assert_eq!(summary.get("event_count").and_then(Value::as_i64), Some(1));
    assert_eq!(summary.get("bodies_stored"), Some(&Value::Bool(false)));
    assert_eq!(summary.get("partial"), Some(&Value::Bool(true)));
    assert_eq!(summary.as_object().unwrap().len(), 5);
    assert!(!canonical_json(&data).contains("PRIVATE_SENTINEL"));
    assert!(!canonical_json(&data).contains("timeline"));
}

/// The Codex-anchored vertical loop, end to end: a real-shaped project
/// (multi-layer AGENTS.md chain, a `.ctxpect-ignore` exclusion, frontmatter)
/// goes inspect -> doctor -> intent preview -> authorized apply -> rollback,
/// and each stage's honesty boundary is asserted rather than assumed.
///
/// Run evidence is stated, not staged. The only declared Codex native oracle
/// is `codex debug prompt-input`, whose pinned recording lives at
/// `acceptance/corpus/development/oracle/codex__debug-prompt-input.jsonl`
/// (`live_tested: false`); it is *named* by the inspect explanation as the
/// next evidence and is not executed here, so the runtime facets stay
/// indeterminate with `runtime_snapshot_missing` and this test asserts
/// exactly that instead of a fabricated runtime claim.
#[test]
fn codex_static_to_safe_write_loop_closes_end_to_end() {
    let scratch = Scratch::new("e2e");
    // `updated: 2025-01-01` is >365 days before the pinned as-of below; the
    // static fix is a frontmatter date bump on the same observed file.
    let original = "---\nupdated: 2025-01-01\n---\nroot rules\n";
    let fixed = "---\nupdated: 2026-09-01\n---\nroot rules\n";
    scratch.write("AGENTS.md", original);
    scratch.write("sub/AGENTS.md", "sub rules\n");
    scratch.write(".ctxpect-ignore", "sub/AGENTS.md\n");
    scratch.write("codex-home/AGENTS.md", "global rules\n");
    let project = scratch.path.to_str().unwrap().to_string();
    let cwd = scratch.path.join("sub").to_str().unwrap().to_string();
    let home = scratch.path.join("codex-home").to_str().unwrap().to_string();
    let store = scratch.path.join("store");
    let store_s = store.to_str().unwrap().to_string();

    // (a) inspect: static resolution over the declared roots only.
    let (code, json, out) = run(&[
        "inspect", "--json", "--project", &project, "--cwd", &cwd, "--codex-home", &home,
        "--store", &store_s,
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    let result = &json.get("results").and_then(Value::as_array).unwrap()[0];
    assert_eq!(result.get("truth_state").and_then(Value::as_str), Some("present"));
    assert_eq!(result.pointer(&["parse", "included"]), Some(&Value::Bool(true)));
    let edges = result.get("edges").and_then(Value::as_array).unwrap();
    // The nested chain: the global (codex-home) and project-root files are
    // adopted; the cwd layer's file is excluded by the product-side ignore.
    assert!(
        edges.iter().any(|e| e.get("kind").and_then(Value::as_str) == Some("included-by")
            && e.get("rule_id").and_then(Value::as_str) == Some("G5")),
        "{edges:?}"
    );
    assert!(
        edges.iter().any(|e| e.get("kind").and_then(Value::as_str) == Some("included-by")
            && e.get("rule_id").and_then(Value::as_str) == Some("G1")
            && e.get("path").and_then(Value::as_str) == Some("AGENTS.md")),
        "{edges:?}"
    );
    let excluded = edges
        .iter()
        .find(|e| e.get("kind").and_then(Value::as_str) == Some("excluded-by"))
        .expect("G4 exclusion edge");
    assert_eq!(excluded.get("rule_id").and_then(Value::as_str), Some("G4"));
    assert_eq!(excluded.get("path").and_then(Value::as_str), Some("sub/AGENTS.md"));
    assert_eq!(
        excluded.get("note").and_then(Value::as_str),
        Some("product-user-exclusion")
    );
    let layers = result.get("layers").and_then(Value::as_array).unwrap();
    assert_eq!(layers.len(), 3, "{layers:?}");
    assert_eq!(layers[2].get("id").and_then(Value::as_str), Some("sub"));
    assert_eq!(layers[2].get("adopted"), Some(&Value::Null));
    // The explanation shows the T02a semantics: the exclusion narrows
    // Contexpect's observation scope; the native load state stays unknown.
    let explanation = json.get("explanation").and_then(Value::as_array).unwrap();
    let shown = explanation
        .iter()
        .find(|e| e.get("kind").and_then(Value::as_str) == Some("excluded"))
        .expect("excluded explanation");
    let why = shown.get("why").and_then(Value::as_str).unwrap_or("");
    assert!(
        why.contains("observation_scope_excluded") && why.contains("not a Codex native rule"),
        "{why}"
    );
    // The recorded native oracle is named as the next evidence — only named.
    let i04 = explanation
        .iter()
        .find(|e| e.get("rule_id").and_then(Value::as_str) == Some("I04"))
        .expect("I04 runtime-facet explanation");
    assert!(
        i04.get("next_evidence")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("codex debug prompt-input"),
        "{i04:?}"
    );
    // No runtime evidence was collected: every runtime facet stays
    // indeterminate with its reason code, and installed is not inferred.
    let facets = result.get("facets").expect("facets");
    for facet in ["model-visible", "use-evidence", "outcome-affecting"] {
        let claim = facets.get(facet).unwrap_or(&Value::Null);
        assert_eq!(
            claim.get("truth_state").and_then(Value::as_str),
            Some("indeterminate"),
            "{facet}"
        );
        assert_eq!(
            claim.get("unknown_reason_code").and_then(Value::as_str),
            Some("runtime_snapshot_missing"),
            "{facet}"
        );
    }
    assert_eq!(
        facets
            .pointer(&["installed", "unknown_reason_code"])
            .and_then(Value::as_str),
        Some("runtime_snapshot_missing")
    );
    let old_receipt_id = json
        .get("formal_receipt_id")
        .and_then(Value::as_str)
        .expect("formal_receipt_id")
        .to_string();
    let old_receipt_path = store.join(format!("receipts/{old_receipt_id}.json"));
    assert!(old_receipt_path.is_file());

    // T02a at the claim level: when the exclusion removes the only candidate
    // on the chain, the claim is indeterminate/observation_scope_excluded
    // (exit 3) — never absent.
    let excluded_only = Scratch::new("e2e-excluded");
    excluded_only.write("pkg/AGENTS.md", "only\n");
    excluded_only.write(".ctxpect-ignore", "pkg/AGENTS.md\n");
    let (code, json, out) = run(&[
        "inspect",
        "--json",
        "--project",
        excluded_only.path.to_str().unwrap(),
        "--cwd",
        excluded_only.path.join("pkg").to_str().unwrap(),
    ]);
    assert_eq!(code, 3, "{out} {json:?}");
    let result = &json.get("results").and_then(Value::as_array).unwrap()[0];
    assert_eq!(
        result.get("truth_state").and_then(Value::as_str),
        Some("indeterminate")
    );
    assert_eq!(
        result.get("unknown_reason_code").and_then(Value::as_str),
        Some("observation_scope_excluded")
    );
    assert_eq!(result.pointer(&["parse", "included"]), Some(&Value::Null));

    // (b) doctor against the pinned as-of date: the stale frontmatter is a
    // real, non-blocking, statically fixable finding.
    let (code, json, out) = run(&[
        "doctor", "--json", "--project", &project, "--cwd", &cwd, "--codex-home", &home,
        "--as-of", "2026-09-04",
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(json.get("as_of").and_then(Value::as_str), Some("2026-09-04"));
    let findings = json.get("findings").and_then(Value::as_array).unwrap();
    let stale = findings
        .iter()
        .find(|f| f.get("rule_id").and_then(Value::as_str) == Some("stale"))
        .expect("stale finding");
    assert_eq!(stale.get("path").and_then(Value::as_str), Some("AGENTS.md"));
    assert_eq!(stale.get("blocking").and_then(Value::as_bool), Some(false));
    assert!(
        stale
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("2026-09-04"),
        "{stale:?}"
    );
    // The exclusion is reported for what it is (observation scope), unlocked.
    let ignored = findings
        .iter()
        .find(|f| f.get("rule_id").and_then(Value::as_str) == Some("D-IGNORE-G4"))
        .expect("D-IGNORE-G4 finding");
    assert_eq!(
        ignored.pointer(&["treatment", "locked"]).and_then(Value::as_bool),
        Some(false)
    );
    // C-F03: indeterminate runtime-surface and outcome facets do not lock a
    // limited static fix; the evidence gap is still reported as indeterminate.
    for facet in ["D-FACET-MODEL-VISIBLE", "D-FACET-USE-EVIDENCE", "D-FACET-OUTCOME-AFFECTING"] {
        let finding = findings
            .iter()
            .find(|f| f.get("rule_id").and_then(Value::as_str) == Some(facet))
            .unwrap_or_else(|| panic!("{facet} missing: {findings:?}"));
        assert_eq!(
            finding.pointer(&["treatment", "locked"]).and_then(Value::as_bool),
            Some(false),
            "{facet}"
        );
        assert_eq!(
            finding.pointer(&["treatment", "lock_reason"]).and_then(Value::as_str),
            Some("none"),
            "{facet}"
        );
        assert_eq!(
            finding.get("evidence_state").and_then(Value::as_str),
            Some("indeterminate"),
            "{facet}"
        );
        assert_eq!(
            finding.get("confirmation").and_then(Value::as_str),
            Some("suspected"),
            "{facet}"
        );
    }
    assert_eq!(json.pointer(&["counts", "blocking"]).and_then(Value::as_i64), Some(0));

    // (c) intent preview: the fix is frozen into a persisted, digested plan.
    // There is no doctor->preview bridge yet; target/desired are explicit.
    // The desired bytes begin with `---`, so they must go inline: a separate
    // token starting with `--` is refused as a flag.
    let desired_arg = format!("--desired={fixed}");
    let (code, preview, out) = run(&[
        "intent", "preview", "--json", "--project", &project, "--store", &store_s, "--target",
        "AGENTS.md", &desired_arg,
    ]);
    assert_eq!(code, 0, "{out} {preview:?}");
    assert_eq!(preview.get("persisted").and_then(Value::as_bool), Some(true));
    let tx = preview.get("tx_id").and_then(Value::as_str).expect("tx_id").to_string();
    let original_digest = ctxpect_schema::sha256_hex(original.as_bytes());
    let fixed_digest = ctxpect_schema::sha256_hex(fixed.as_bytes());
    assert_eq!(
        preview.get("current_digest").and_then(Value::as_str),
        Some(original_digest.as_str())
    );
    assert_eq!(
        preview.get("desired_digest").and_then(Value::as_str),
        Some(fixed_digest.as_str())
    );
    let persisted =
        parse(&fs::read_to_string(store.join(format!("previews/{tx}.json"))).unwrap()).unwrap();
    assert_eq!(
        persisted.get("desired_digest").and_then(Value::as_str),
        Some(fixed_digest.as_str())
    );

    // (d) Authorized apply: the frozen bytes land, a backup and the audit
    // trail exist, and a new Receipt observes the written state.
    plant_grants(&store, Some(&scratch.path), &["apply", "rollback"]);
    let old_receipt_bytes = fs::read(&old_receipt_path).unwrap();
    let (code, applied, out) = run(&[
        "apply", "--json", "--project", &project, "--store", &store_s, "--cwd", &cwd,
        "--codex-home", &home, "--tx", &tx,
    ]);
    assert_eq!(code, 0, "{out} {applied:?}");
    assert_eq!(
        applied.pointer(&["transaction", "state"]).and_then(Value::as_str),
        Some("committed")
    );
    assert_eq!(fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(), fixed);
    let tx_dir = store.join("apply").join(&tx);
    assert_eq!(fs::read(tx_dir.join("before")).unwrap(), original.as_bytes());
    let record = parse(&fs::read_to_string(tx_dir.join("tx.json")).unwrap()).unwrap();
    assert_eq!(record.get("state").and_then(Value::as_str), Some("committed"));
    let post_id = applied
        .get("post_receipt_id")
        .and_then(Value::as_str)
        .expect("post_receipt_id")
        .to_string();
    assert!(store.join(format!("receipts/{post_id}.json")).is_file());
    assert_ne!(post_id, old_receipt_id);
    let audit = fs::read_to_string(store.join("audit/events.jsonl")).unwrap();
    assert!(
        audit
            .lines()
            .any(|line| line.contains("\"action\":\"projection.apply\"") && line.contains(&tx)),
        "{audit}"
    );
    assert!(
        audit
            .lines()
            .any(|line| line.contains("\"action\":\"receipt.put\"") && line.contains(&post_id)),
        "{audit}"
    );
    // The pre-apply Receipt is byte-identical afterwards.
    assert_eq!(fs::read(&old_receipt_path).unwrap(), old_receipt_bytes);
    // The new Receipt is exactly what a fresh inspect of the written state
    // produces: it reflects the write, not a re-issued old observation.
    let (code, json, out) = run(&[
        "inspect", "--json", "--project", &project, "--cwd", &cwd, "--codex-home", &home,
        "--store", &store_s,
    ]);
    assert_eq!(code, 0, "{out} {json:?}");
    assert_eq!(
        json.get("formal_receipt_id").and_then(Value::as_str),
        Some(post_id.as_str())
    );

    // (e) Rollback restores the original bytes, and the observation after the
    // undo is again the pre-apply Receipt: the instruction evidence is back
    // to what it was, so the id comes back too.
    let (code, undone, out) = run(&[
        "rollback", "--json", "--project", &project, "--store", &store_s, "--cwd", &cwd,
        "--codex-home", &home, "--id", &tx,
    ]);
    assert_eq!(code, 0, "{out} {undone:?}");
    assert_eq!(
        undone.get("action").and_then(Value::as_str),
        Some("restored-previous-bytes")
    );
    assert_eq!(fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(), original);
    assert_eq!(
        undone.get("post_receipt_id").and_then(Value::as_str),
        Some(old_receipt_id.as_str())
    );
    assert_eq!(fs::read(&old_receipt_path).unwrap(), old_receipt_bytes);

    // (f) Negative half: a target edited after preview is refused
    // (projection.concurrent_hash), and a consumed transaction cannot be
    // re-applied.
    let (code, preview2, out) = run(&[
        "intent", "preview", "--json", "--project", &project, "--store", &store_s, "--target",
        "AGENTS.md", &desired_arg,
    ]);
    assert_eq!(code, 0, "{out} {preview2:?}");
    let tx2 = preview2.get("tx_id").and_then(Value::as_str).unwrap().to_string();
    assert_ne!(tx2, tx);
    scratch.write("AGENTS.md", "edited between preview and apply\n");
    let (code, json, out) = run(&["apply", "--json", "--project", &project, "--store", &store_s, "--tx", &tx2]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.concurrent_hash"));
    assert_eq!(
        fs::read_to_string(scratch.path.join("AGENTS.md")).unwrap(),
        "edited between preview and apply\n"
    );
    let (code, json, out) = run(&["apply", "--json", "--project", &project, "--store", &store_s, "--tx", &tx]);
    assert_eq!(code, 1, "{out} {json:?}");
    assert_eq!(err_code(&json), Some("projection.tx_consumed"));
}
