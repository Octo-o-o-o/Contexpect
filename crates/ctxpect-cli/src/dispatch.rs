//! Product command dispatch (everything except the inspect slice).

use crate::args::{InspectArgs, ProductArgs};
use crate::catalog::{integrations_json, FAMILIES};
use crate::inspect::{inspect, InspectFailure};
use crate::jsonutil::{obj, s};
use crate::redact::{redact_json_envelope, RedactRoots};
use ctxpect_advisor::suggest;
use ctxpect_diff::{diff, EquivalenceProfile};
use ctxpect_doctor::diagnose;
use ctxpect_effect::{decide, run_local_instructions_probe, ExperimentContract};
use ctxpect_fs::Root;
use ctxpect_importer::{import_session, insight};
use ctxpect_policy::{
    authorize_mutation, evaluate, exception_status, new_exception, transition,
};
use ctxpect_projection::{apply as proj_apply, launch_argv, preview as proj_preview, rollback as proj_rollback, Intent};
use ctxpect_receipt::{
    migrate_dev_inspect_v0, reject_relabeled_snapshot, verify_local_continuity,
};
use ctxpect_schema::{
    array, canonical_json, digest_value, hmac_sha256_hex, object, parse, sha256_text, string, Value,
};
use ctxpect_store::{now_rfc3339, Store};
use ctxpect_sync::{apply_folder, bundle, preview_apply};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};


pub struct ProductReport {
    pub exit_code: i32,
    pub envelope: Value,
}

pub fn run_product(args: ProductArgs) -> Result<ProductReport, InspectFailure> {
    let command = args.command.clone();
    dispatch_product(args).map_err(|failure| failure.in_command(&command))
}

fn dispatch_product(args: ProductArgs) -> Result<ProductReport, InspectFailure> {
    match args.command.as_str() {
        "collect" | "inventory" => collect_cmd(&args),
        "doctor" => doctor_cmd(&args),
        "diff" => diff_cmd(&args),
        "receipt" => receipt_cmd(&args),
        "preflight" => preflight_cmd(&args),
        "launch" => launch_cmd(&args),
        "import" | "sessions" => sessions_cmd(&args),
        "daemon" => daemon_cmd(&args),
        "sync" => sync_cmd(&args),
        "intent" | "apply" | "rollback" => projection_cmd(&args),
        "standard" => standard_cmd(&args),
        "exception" => exception_cmd(&args),
        "assets" => assets_cmd(&args),
        "advisor" => advisor_cmd(&args),
        "experiment" => experiment_cmd(&args),
        "policy" => policy_cmd(&args),
        "align" => align_cmd(&args),
        "adapter" => adapter_cmd(&args),
        "ci" => ci_cmd(&args),
        other => Err(InspectFailure::Usage(crate::args::UsageError {
            code: "usage.unimplemented",
            message: format!("`{other}` is listed but not dispatched"),
            command: Some(other.to_string()),
        })),
    }
}

/// Raise a dispatch failure without an attribution; `run_product` stamps the
/// command that was actually running.
fn fail(code: &'static str, message: String) -> InspectFailure {
    InspectFailure::Io {
        code,
        message,
        command: None,
    }
}

fn ok(command: &str, exit: i32, body: Value) -> Result<ProductReport, InspectFailure> {
    let mut envelope = match body {
        Value::Object(map) => {
            let mut out = map;
            out.entry("schema_version".into()).or_insert(Value::Int(1));
            out.entry("command".into()).or_insert(string(command));
            out.entry("exit_code".into()).or_insert(Value::Int(i64::from(exit)));
            Value::Object(out)
        }
        other => object([
            ("schema_version", Value::Int(1)),
            ("command", string(command)),
            ("exit_code", Value::Int(i64::from(exit))),
            ("result", other),
        ]),
    };
    envelope = crate::jsonutil::with_snapshot_digest(envelope);
    Ok(ProductReport {
        exit_code: exit,
        envelope,
    })
}

fn open_store(args: &ProductArgs) -> Result<Store, InspectFailure> {
    let path = args
        .store
        .clone()
        .or_else(|| args.project.as_ref().map(|p| p.join(".ctxpect/store")))
        .ok_or_else(|| fail("usage.invalid", "`--store` or `--project` is required".into()))?;
    Store::open(&path).map_err(|err| fail(err.code, err.message))
}

pub(crate) fn now_unix() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(0)
}

pub(crate) fn load_policy_layers(store: &Store, project: Option<&Path>) -> Option<Value> {
    if let Ok(layers) = store.get_named("policies", "active") {
        return Some(layers);
    }
    if let Ok(ids) = store.list_named("policies") {
        for id in ids {
            if let Ok(layers) = store.get_named("policies", &id) {
                return Some(layers);
            }
        }
    }
    if let Some(project) = project {
        let path = project.join(".ctxpect/policy.json");
        if let Ok(text) = fs::read_to_string(path)
            && let Ok(layers) = parse(&text)
        {
            return Some(layers);
        }
    }
    None
}

fn load_exceptions(store: &Store) -> Vec<Value> {
    let Ok(ids) = store.list_named("exceptions") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for id in ids {
        if let Ok(rec) = store.get_named("exceptions", &id) {
            out.push(rec);
        }
    }
    out
}

fn store_mutation_decision(
    store: &Store,
    project: Option<&Path>,
) -> Result<Value, ctxpect_policy::PolicyError> {
    let layers = load_policy_layers(store, project);
    let exceptions = load_exceptions(store);
    authorize_mutation(layers.as_ref(), &exceptions, now_unix(), true)
}

/// Environment variable carrying the enrolled principal secret.
///
/// It is read from the environment, never from argv: argv is visible in
/// process listings and is captured by the redaction roots, and a secret
/// belongs in neither.
pub(crate) const PRINCIPAL_SECRET_ENV: &str = "CTXPECT_PRINCIPAL_SECRET";

/// Path of the version-controlled principal registry inside a project.
pub(crate) const PRINCIPAL_REGISTRY_REL: &str = ".ctxpect/principals.json";

fn load_principal_registry(project: Option<&Path>) -> Option<Value> {
    let path = project?.join(PRINCIPAL_REGISTRY_REL);
    let text = fs::read_to_string(path).ok()?;
    parse(&text).ok()
}

/// Verify the caller against the project's enrolled principal registry.
///
/// The result is auditable evidence, so both outcomes are recorded. The
/// secret itself is never part of the record.
fn verify_caller(
    store: &Store,
    args: &ProductArgs,
) -> Result<ctxpect_policy::PrincipalProof, InspectFailure> {
    let Some(principal_id) = args.principal.as_deref() else {
        return Err(fail(
            "principal.absent",
            format!(
                "`--principal <id>` and ${PRINCIPAL_SECRET_ENV} are required; `--actor` / `--role` are caller-attested and are not identity"
            ),
        ));
    };
    let registry = load_principal_registry(args.project.as_deref());
    let secret = std::env::var(PRINCIPAL_SECRET_ENV).unwrap_or_default();
    match ctxpect_policy::verify_principal(registry.as_ref(), principal_id, secret.as_bytes()) {
        Ok(proof) => {
            let _ = store.audit("principal.verified", "principal", Some(principal_id));
            Ok(proof)
        }
        Err(err) => {
            let _ = store.audit("principal.rejected", "principal", Some(err.code));
            Err(fail(err.code, err.message))
        }
    }
}

pub(crate) fn authorize_store_apply(
    store: &Store,
    project: Option<&Path>,
) -> Result<Value, InspectFailure> {
    match store_mutation_decision(store, project) {
        Ok(auth) => {
            let detail = auth
                .pointer(&["exception", "exception_id"])
                .and_then(Value::as_str);
            let _ = store.audit("mutation.allowed", "mutation", detail);
            Ok(auth)
        }
        Err(err) => {
            let _ = store.audit("mutation.denied", "mutation", Some(err.code));
            Err(fail(err.code, err.message))
        }
    }
}

/// Same source and decision as [`authorize_store_apply`], returned as a
/// viewer document. Top-level `verdict` is the mutation decision, not a
/// hardcoded five-layer pass.
pub(crate) fn effective_store_policy(store: &Store, project: Option<&Path>) -> Value {
    let layers = load_policy_layers(store, project);
    let now = now_unix();
    let layer_evaluation = layers
        .as_ref()
        .and_then(|item| evaluate(item, now).ok())
        .unwrap_or(Value::Null);
    match store_mutation_decision(store, project) {
        Ok(auth) => object([
            ("schema", string("ctxpect-policy-effective-v1")),
            ("mutation_allowed", Value::Bool(true)),
            ("reason_code", string("ok")),
            ("verdict", string("pass")),
            ("layers_present", Value::Bool(true)),
            ("layer_evaluation", auth.get("evaluation").cloned().unwrap_or(layer_evaluation)),
            (
                "exception_id",
                string(
                    auth.pointer(&["exception", "exception_id"])
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ),
            ),
        ]),
        Err(err) => {
            let verdict = match err.code {
                "policy.unknown" => "unknown",
                "policy.denied" => "deny",
                "policy.detect_only_not_enforceable" => "detect-only",
                "policy.approval_required" => "approval_required",
                "policy.indeterminate" => "indeterminate",
                _ => "fail-closed",
            };
            object([
                ("schema", string("ctxpect-policy-effective-v1")),
                ("mutation_allowed", Value::Bool(false)),
                ("reason_code", string(err.code)),
                ("verdict", string(verdict)),
                ("layers_present", Value::Bool(layers.is_some())),
                ("layer_evaluation", layer_evaluation),
                ("message", string(err.message)),
            ])
        }
    }
}

fn standard_unsigned(doc: &Value) -> Value {
    object([
        (
            "standard_id",
            string(doc.get("standard_id").and_then(Value::as_str).unwrap_or("")),
        ),
        (
            "payload_digest",
            string(
                doc.get("payload_digest")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ),
        ),
        (
            "signature_kind",
            string(
                doc.get("signature_kind")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ),
        ),
        (
            "org_identity",
            Value::Bool(
                doc.get("org_identity")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
    ])
}

/// Recompute the local-continuity MAC from the persisted record. `signed`
/// is the result of that comparison, not a literal written at publish time.
pub(crate) fn verify_standard_document(
    store: &Store,
    doc: &Value,
) -> Result<Value, InspectFailure> {
    let key = store
        .continuity_key()
        .map_err(|err| fail(err.code, err.message))?;
    let sig = doc
        .pointer(&["signature", "value"])
        .and_then(Value::as_str)
        .unwrap_or("");
    if sig.is_empty() {
        return Err(fail(
            "standard.unsigned",
            "standard has no local-continuity signature".into(),
        ));
    }
    let digest = digest_value(&standard_unsigned(doc));
    let expected = hmac_sha256_hex(&key.secret, digest.as_bytes());
    if sig != expected {
        return Err(fail(
            "standard.signature_mismatch",
            "local-continuity MAC did not verify against the store key".into(),
        ));
    }
    Ok(match doc.clone() {
        Value::Object(mut map) => {
            map.insert("signed".into(), Value::Bool(true));
            Value::Object(map)
        }
        other => other,
    })
}

fn secret_scan(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("begin private key")
        || lower.contains("aws_secret_access_key")
        || lower.contains("api_key=")
        || lower.contains("authorization: bearer ")
        || text.contains("ghp_")
        || text.contains("xoxb-")
        || text.contains("xoxp-")
        || text.contains("sk-")
}

fn inspect_args(args: &ProductArgs) -> Result<InspectArgs, InspectFailure> {
    let project = args
        .project
        .clone()
        .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))?;
    Ok(InspectArgs {
        json: true,
        offline: args.offline,
        project,
        cwd: args.cwd.clone(),
        harness: args.harness.clone(),
        surface: args.surface.clone(),
        version: args.version.clone(),
        version_explicit: args.version_explicit,
        codex_home: args.codex_home.clone(),
        require: if args.require.is_empty() {
            vec!["instructions".into()]
        } else {
            args.require.clone()
        },
        os_lane: args.os_lane.clone(),
        store: args.store.clone(),
    })
}

pub fn persist_inspect(
    store: &Store,
    snapshot: &Value,
    kind: &str,
) -> Result<Value, InspectFailure> {
    reject_relabeled_snapshot(snapshot)
        .map_err(|err| fail(err.code, err.message))?;
    let key = store
        .continuity_key()
        .map_err(|err| fail(err.code, err.message))?;
    let id = ctxpect_receipt::receipt_id_for(kind, &canonical_json(snapshot));
    let receipt = migrate_dev_inspect_v0(snapshot, kind, Some(&key), &now_rfc3339(), &id)
        .map_err(|err| fail(err.code, err.message))?;
    store
        .put_snapshot(&id, snapshot)
        .map_err(|err| fail(err.code, err.message))?;
    store
        .put_receipt(&receipt)
        .map_err(|err| fail(err.code, err.message))?;
    Ok(receipt)
}

fn collect_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let project = args
        .project
        .as_ref()
        .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))?;
    let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
    // `resource_limits.scan_files` is a setting, so it has to actually bound
    // the walk; storing it without enforcing it would make configuring it a
    // no-op.
    let file_limit = args
        .store
        .as_ref()
        .and_then(|path| Store::open(path).ok())
        .and_then(|store| store.settings().ok())
        .and_then(|settings| {
            settings
                .pointer(&["resource_limits", "scan_files"])
                .and_then(Value::as_i64)
        })
        .and_then(|value| usize::try_from(value).ok());
    let inventory = ctxpect_collect::scan_with_limit(&root, file_limit)
        .map_err(|err| fail("io.unresolvable", err.to_string()))?;
    let mut residue = Vec::new();
    let mut entries = Vec::new();
    for entry in &inventory.entries {
        let is_residue = entry.path.contains(".codex")
            || entry.path.contains(".claude")
            || entry.path.ends_with(".env");
        if is_residue {
            residue.push(object([
                ("path", string(&entry.path)),
                ("installed", Value::Bool(false)),
                ("reason_code", string("config_residue_only")),
            ]));
        }
        entries.push(object([
            ("path", string(&entry.path)),
            (
                "kind",
                string(format!("{:?}", entry.kind).to_lowercase()),
            ),
            (
                "content_digest",
                match &entry.content_digest {
                    Some(d) => string(d),
                    None => Value::Null,
                },
            ),
            (
                "withheld",
                match &entry.withheld {
                    Some(w) => string(w.tag()),
                    None => Value::Null,
                },
            ),
        ]));
    }
    ok(
        args.command.as_str(),
        0,
        object([
            ("inventory_digest", string(inventory.digest())),
            // A truncated walk is an unknown inventory, not a smaller one.
            // Saying so here keeps its digest from being read as a complete
            // manifest.
            ("complete", Value::Bool(!inventory.truncated)),
            (
                "file_limit",
                inventory
                    .file_limit
                    .and_then(|limit| i64::try_from(limit).ok())
                    .map_or(Value::Null, Value::Int),
            ),
            (
                "reason_code",
                string(if inventory.truncated {
                    "scan.file_limit_reached"
                } else {
                    "ok"
                }),
            ),
            ("entries", array(entries)),
            ("config_residue", array(residue)),
            (
                "scanned_home",
                Value::Bool(args.home.is_some()),
            ),
            (
                "home_default_scan",
                Value::Bool(false),
            ),
            (
                "config_residue_is_installed",
                Value::Bool(false),
            ),
        ]),
    )
}

fn doctor_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let inspect_args = inspect_args(args)?;
    let report = inspect(inspect_args)?;
    let diagnosis = diagnose(&report.envelope);
    let mut exit = report.exit_code;
    if args.fail_on.as_deref() == Some("confirmed")
        && let Some(Value::Int(n)) = diagnosis.pointer(&["counts", "confirmed"])
        && *n > 0
    {
        exit = 2;
    }
    let mut body = match diagnosis {
        Value::Object(map) => Value::Object(map),
        other => object([("doctor", other)]),
    };
    if let Value::Object(map) = &mut body {
        map.insert("inspect_exit_code".into(), Value::Int(i64::from(report.exit_code)));
        map.insert("snapshot_schema".into(), string("dev-inspect-v0"));
    }
    if let Some(store_path) = &args.store {
        let store = Store::open(store_path).map_err(|err| fail(err.code, err.message))?;
        let receipt = persist_inspect(&store, &report.envelope, "one-shot")?;
        if let Value::Object(map) = &mut body {
            map.insert(
                "receipt_id".into(),
                string(receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("")),
            );
        }
    }
    ok("doctor", exit, body)
}

fn diff_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let left_id = args
        .left
        .as_deref()
        .ok_or_else(|| fail("usage.invalid", "`--a <receipt-id>` is required".into()))?;
    let right_id = args
        .right
        .as_deref()
        .ok_or_else(|| fail("usage.invalid", "`--b <receipt-id>` is required".into()))?;
    let left = store
        .get_receipt(left_id)
        .map_err(|err| fail(err.code, err.message))?;
    let right = store
        .get_receipt(right_id)
        .map_err(|err| fail(err.code, err.message))?;
    let profile = EquivalenceProfile::from_wire(args.profile.as_deref().unwrap_or("strict"))
        .ok_or_else(|| fail("usage.invalid", "unknown equivalence profile".into()))?;
    ok("diff", 0, diff(&left, &right, profile))
}

fn receipt_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let sub = args.subcommand.as_deref().unwrap_or("show");
    match sub {
        "show" => {
            let id = args
                .receipt
                .as_deref()
                .or(args.id.as_deref())
                .ok_or_else(|| fail("usage.invalid", "`--receipt <id>` is required".into()))?;
            let receipt = store
                .get_receipt(id)
                .map_err(|err| fail(err.code, err.message))?;
            ok("receipt show", 0, receipt)
        }
        "verify" => {
            let id = args
                .receipt
                .as_deref()
                .or(args.id.as_deref())
                .ok_or_else(|| fail("usage.invalid", "`--receipt <id>` is required".into()))?;
            let receipt = store
                .get_receipt(id)
                .map_err(|err| fail(err.code, err.message))?;
            if receipt.get("tombstone").map(|v| v != &Value::Null).unwrap_or(false)
                && receipt.get("claims").and_then(Value::as_array).map(|a| a.is_empty()).unwrap_or(false)
            {
                return ok(
                    "receipt verify",
                    3,
                    object([
                        ("ok", Value::Bool(false)),
                        ("reason_code", string("receipt.tombstoned")),
                        (
                            "signature_vs_attachment",
                            string("tombstone preserves digest; evidence bodies are purged"),
                        ),
                    ]),
                );
            }
            let key = store
                .continuity_key()
                .map_err(|err| fail(err.code, err.message))?;
            verify_local_continuity(&receipt, &key).map_err(|err| fail(err.code, err.message))?;
            ok(
                "receipt verify",
                0,
                object([
                    ("ok", Value::Bool(true)),
                    ("kind", string("local-continuity")),
                    ("org_identity", Value::Bool(false)),
                ]),
            )
        }
        "export" => {
            let id = args
                .receipt
                .as_deref()
                .or(args.id.as_deref())
                .ok_or_else(|| fail("usage.invalid", "`--receipt <id>` is required".into()))?;
            let receipt = store
                .get_receipt(id)
                .map_err(|err| fail(err.code, err.message))?;
            let redacted = redact_json_envelope(receipt, &RedactRoots::default());
            if let Some(path) = &args.export {
                fs::write(path, canonical_json(&redacted))
                    .map_err(|err| fail("io.write", err.to_string()))?;
            }
            ok("receipt export", 0, redacted)
        }
        "redact" => {
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let id = args
                .receipt
                .as_deref()
                .or(args.id.as_deref())
                .ok_or_else(|| fail("usage.invalid", "`--receipt <id>` is required".into()))?;
            let stone = store
                .delete_receipt(id, args.reason.as_deref().unwrap_or("user-redact"))
                .map_err(|err| fail(err.code, err.message))?;
            ok("receipt redact", 0, stone)
        }
        other => Err(fail("usage.invalid", format!("unknown receipt subcommand `{other}`"))),
    }
}

fn preflight_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let files = if args.files.is_empty() {
        vec!["AGENTS.md".to_string()]
    } else {
        args.files.clone()
    };
    let program = match args.harness.as_str() {
        "codex" => "codex",
        "grok-build" => "grok",
        other => other,
    };
    let mut launch_args = vec!["exec", "--cd", "<project>"];
    for file in &files {
        launch_args.push(file.as_str());
    }
    let argv = launch_argv(program, &launch_args).map_err(|err| fail(err.code, err.message))?;
    let inspect_args = inspect_args(args)?;
    let report = inspect(inspect_args)?;
    let store = open_store(args)?;
    let receipt = persist_inspect(&store, &report.envelope, "preflight")?;
    let id = receipt
        .get("receipt_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let preflight = object([
        ("preflight_id", string(&id)),
        ("task", string(args.task.as_deref().unwrap_or(""))),
        (
            "files",
            array(files.iter().map(|f| string(f.as_str()))),
        ),
        (
            "launch_argv",
            array(argv.iter().map(|a| string(a.as_str()))),
        ),
        ("shell_string", Value::Null),
        ("agent_selected_probability", Value::Null),
        ("receipt_id", string(&id)),
        ("eligible_source", string("static-task-probe")),
    ]);
    // The Receipt above is an append-only observation. The preflight intent
    // is a planned change, so persisting it is a mutation.
    let _auth = authorize_store_apply(&store, args.project.as_deref())?;
    store
        .put_named("intents", &format!("preflight_{id}"), &preflight)
        .map_err(|err| fail(err.code, err.message))?;
    ok("preflight", report.exit_code, preflight)
}

fn launch_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    if args.execute {
        return Err(fail(
            "launch.execute_refused",
            "refusing to spawn a harness in this environment; copy the structured argv".into(),
        ));
    }
    let id = args
        .preflight_id
        .as_deref()
        .ok_or_else(|| fail("launch.preflight_required", "launch requires --preflight-id".into()))?;
    let store = open_store(args)?;
    let preflight = store
        .get_named("intents", &format!("preflight_{id}"))
        .map_err(|_| fail("launch.preflight_missing", format!("unknown preflight_id `{id}`")))?;
    let argv = preflight
        .get("launch_argv")
        .cloned()
        .unwrap_or_else(|| array([]));
    ok(
        "launch",
        0,
        object([
            ("preflight_id", string(id)),
            ("argv", argv),
            ("executed", Value::Bool(false)),
            ("copied", Value::Bool(true)),
        ]),
    )
}

fn sessions_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    if args.command == "import" || args.subcommand.as_deref() == Some("import") {
        let _auth = authorize_store_apply(&store, args.project.as_deref())?;
        let path = args
            .from
            .as_ref()
            .ok_or_else(|| fail("usage.invalid", "`--from <file>` is required".into()))?;
        let text = fs::read_to_string(path).map_err(|err| fail("io.missing", err.to_string()))?;
        let session_id = args
            .session
            .clone()
            .unwrap_or_else(|| format!("s_{}", &ctxpect_schema::sha256_text(&text)[..12]));
        let mapping = args.mapping.as_deref().unwrap_or("generic-json");
        let session = import_session(&text, mapping, &session_id)
            .map_err(|err| fail(err.code, err.message))?;
        store
            .put_named("sessions", &session_id, &session)
            .map_err(|err| fail(err.code, err.message))?;
        let note = insight(&session_id, "partial timeline imported; occupancy unknown");
        store
            .put_named("insights", &session_id, &note)
            .map_err(|err| fail(err.code, err.message))?;
        return ok("import", 0, session);
    }
    if args.id.as_deref() == Some("delete") || args.reason.as_deref() == Some("delete") {
        // handled below via --session + reason delete
    }
    if let Some(session_id) = args.session.as_deref().or(args.id.as_deref()) {
        if args.reason.as_deref() == Some("delete") || args.subcommand.as_deref() == Some("delete")
        {
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            store
                .delete_named("sessions", session_id)
                .map_err(|err| fail(err.code, err.message))?;
            store
                .delete_named("insights", session_id)
                .map_err(|err| fail(err.code, err.message))?;
            let insights_gone = store.get_named("insights", session_id).is_err();
            return ok(
                "sessions",
                0,
                object([
                    ("deleted", string(session_id)),
                    ("insights_invalidated", Value::Bool(insights_gone)),
                ]),
            );
        }
        let session = store
            .get_named("sessions", session_id)
            .map_err(|err| fail(err.code, err.message))?;
        return ok("sessions", 0, session);
    }
    let ids = store
        .list_named("sessions")
        .map_err(|err| fail(err.code, err.message))?;
    ok(
        "sessions",
        0,
        object([(
            "sessions",
            array(ids.into_iter().map(string)),
        )]),
    )
}

fn daemon_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    match args.subcommand.as_deref() {
        Some("start") => crate::http::serve(args),
        Some("status") => {
            let store = open_store(args)?;
            let pid_path = store.root().join("daemon.pid");
            let running = pid_path.exists();
            ok(
                "daemon status",
                0,
                object([
                    ("running", Value::Bool(running)),
                    ("oneshot_without_daemon", Value::Bool(true)),
                ]),
            )
        }
        Some("stop") => {
            let store = open_store(args)?;
            let pid_path = store.root().join("daemon.pid");
            if pid_path.exists() {
                let _ = fs::remove_file(pid_path);
            }
            ok("daemon stop", 0, object([("stopped", Value::Bool(true))]))
        }
        _ => Err(fail("usage.invalid", "daemon requires start|stop|status".into())),
    }
}

fn sync_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let settings = store.settings().map_err(|err| fail(err.code, err.message))?;
    let vault_required = settings.get("vault").and_then(Value::as_str) == Some("required");
    match args.subcommand.as_deref() {
        Some("preview") | Some("status") | Some("apply") => {
            let receipts = if let Some(id) = args.receipt.as_deref() {
                vec![store.get_receipt(id).map_err(|err| fail(err.code, err.message))?]
            } else {
                Vec::new()
            };
            let packed = bundle(
                args.id.as_deref().unwrap_or("bundle-local"),
                object([("desired", string("metadata-only"))]),
                &["env:CTXPECT_TOKEN"],
                receipts,
                vault_required,
            )
            .map_err(|err| fail(err.code, err.message))?;
            if args.subcommand.as_deref() == Some("preview") || args.subcommand.as_deref() == Some("status")
            {
                if let Some(dest) = &args.dest {
                    let preview = preview_apply(dest, &packed)
                        .map_err(|err| fail(err.code, err.message))?;
                    return ok("sync preview", 0, preview);
                }
                return ok("sync preview", 0, packed);
            }
            let dest = args
                .dest
                .as_ref()
                .ok_or_else(|| fail("usage.invalid", "`--dest` is required for sync apply".into()))?;
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let result = apply_folder(dest, &packed).map_err(|err| fail(err.code, err.message))?;
            ok("sync apply", 0, result)
        }
        _ => Err(fail("usage.invalid", "sync requires preview|apply|status".into())),
    }
}

fn projection_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let project = args
        .project
        .as_ref()
        .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))?;
    let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
    let intent = Intent {
        intent_id: args.id.clone().unwrap_or_else(|| "intent-1".into()),
        authority: args
            .authority
            .clone()
            .unwrap_or_else(|| "contexpect-native".into()),
        target_rel: args.target.clone().unwrap_or_else(|| "AGENTS.md".into()),
        desired: args.desired.clone().unwrap_or_else(|| "updated\n".into()),
    };
    if args.command == "intent" {
        let previewed = proj_preview(&root, &intent).map_err(|err| fail(err.code, err.message))?;
        return ok("intent", 0, previewed.to_value());
    }
    let store = open_store(args)?;
    let previewed = proj_preview(&root, &intent).map_err(|err| fail(err.code, err.message))?;
    if args.command == "apply" {
        // Authorize before persisting. Writing the intent first leaves a
        // record of a mutation that policy went on to refuse.
        let _auth = authorize_store_apply(&store, Some(project.as_path()))?;
        store
            .put_named("intents", &intent.intent_id, &intent.to_value())
            .map_err(|err| fail(err.code, err.message))?;
        let backup = ctxpect_projection::backup_dir(store.root(), &previewed.tx_id);
        let result = proj_apply(&root, &intent, &previewed, &backup, true)
            .map_err(|err| fail(err.code, err.message))?;
        let inspect_args = inspect_args(args)?;
        let report = inspect(inspect_args)?;
        let receipt = persist_inspect(&store, &report.envelope, "one-shot")?;
        return ok(
            "apply",
            0,
            object([
                ("transaction", result),
                (
                    "post_receipt_id",
                    string(receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("")),
                ),
            ]),
        );
    }
    let tx = args
        .id
        .as_deref()
        .ok_or_else(|| fail("usage.invalid", "`--id <tx>` is required for rollback".into()))?;
    let _auth = authorize_store_apply(&store, Some(project.as_path()))?;
    store
        .put_named("intents", &intent.intent_id, &intent.to_value())
        .map_err(|err| fail(err.code, err.message))?;
    let backup = ctxpect_projection::backup_dir(store.root(), tx);
    let result = proj_rollback(&root, &backup, &intent.target_rel)
        .map_err(|err| fail(err.code, err.message))?;
    // A rollback changes the project just as an apply does, so the state
    // after it is observed rather than assumed. Without this, the only
    // recorded observation would be the one from before the rollback.
    let post = post_receipt(args, &store)?;
    ok(
        "rollback",
        0,
        merge(result, [("post_receipt_id", string(&post))]),
    )
}

/// Merge extra fields into an object body.
fn merge<'a>(base: Value, extra: impl IntoIterator<Item = (&'a str, Value)>) -> Value {
    let mut map = match base {
        Value::Object(map) => map,
        other => {
            let mut map = std::collections::BTreeMap::new();
            map.insert("result".to_string(), other);
            map
        }
    };
    for (key, value) in extra {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}

fn standard_digest(doc: &Value) -> String {
    doc.get("manifest_digest")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn load_standard(store: &Store, id: &str) -> Result<Value, InspectFailure> {
    store.get_named("standards", id).map_err(|_| {
        fail(
            "standard.absent",
            format!("standard `{id}` is not present in this store"),
        )
    })
}

fn adoption_of(store: &Store, id: &str) -> Option<Value> {
    store.get_named("adoptions", id).ok()
}

fn require_adoption(store: &Store, id: &str) -> Result<Value, InspectFailure> {
    adoption_of(store, id).ok_or_else(|| {
        fail(
            "standard.not_adopted",
            format!("standard `{id}` is not adopted; run `standard adopt` first"),
        )
    })
}

/// An adoption record. `source_digest` is the standard this project tracks;
/// `pinned_digest` is set only when the adoption is pinned to it.
fn new_adoption(id: &str, state: &str, source_digest: &str, pinned: Option<&str>) -> Value {
    object([
        ("schema", string("ctxpect-adoption-v1")),
        ("standard_id", string(id)),
        ("state", string(state)),
        ("source_digest", string(source_digest)),
        (
            "pinned_digest",
            pinned.map_or(Value::Null, string),
        ),
    ])
}

/// Observe the project after a mutation and return the Receipt's id.
///
/// R05 requires a post-Receipt for every mutation. Apply and copy had one;
/// the rollbacks did not, so undoing a change left the last recorded
/// observation describing the state before the undo.
fn post_receipt(args: &ProductArgs, store: &Store) -> Result<String, InspectFailure> {
    let inspect_args = inspect_args(args)?;
    let report = inspect(inspect_args)?;
    let receipt = persist_inspect(store, &report.envelope, "one-shot")?;
    Ok(receipt
        .get("receipt_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string())
}

/// One standard's verified state and this project's adoption of it.
///
/// Shared by the CLI and the API so `standard status` and
/// `GET /api/v1/standards/:id` cannot answer the same question differently
/// (R04). They did: the API returned the stored document without the
/// adoption, so the same query gave two answers.
pub(crate) fn standard_status(store: &Store, id: &str) -> Result<Value, InspectFailure> {
    let adoption = store.get_named("adoptions", id).ok();
    match store.get_named("standards", id) {
        Ok(doc) => {
            let verified = verify_standard_document(store, &doc)?;
            Ok(merge(
                verified,
                [("adoption", adoption.unwrap_or(Value::Null))],
            ))
        }
        // Absence is reported as absence, not as an error.
        Err(_) => Ok(object([
            ("standard_id", string(id)),
            ("status", string("absent")),
            ("signed", Value::Bool(false)),
            ("adoption", adoption.unwrap_or(Value::Null)),
        ])),
    }
}

/// One exception's lifecycle state, including whether it currently grants.
///
/// Shared for the same reason: the API had no per-exception endpoint at all,
/// so a caller could list exceptions but not ask whether one of them grants.
pub(crate) fn exception_state(store: &Store, id: &str) -> Result<Value, InspectFailure> {
    let record = store
        .get_named("exceptions", id)
        .map_err(|err| fail(err.code, err.message))?;
    exception_status(&record, now_unix(), true).map_err(|err| fail(err.code, err.message))
}

fn standard_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let sub = args.subcommand.as_deref().unwrap_or("status");
    let id = args.id.clone().unwrap_or_else(|| "std-local".into());
    match sub {
        "validate" | "publish" => {
            // Publishing a standard writes a signed team contract into the
            // store. That is a mutation and goes through the one authority.
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let payload = args.text.clone().unwrap_or_else(|| id.clone());
            let unsigned = object([
                ("standard_id", string(&id)),
                ("payload_digest", string(sha256_text(&payload))),
                ("signature_kind", string("local-continuity")),
                ("org_identity", Value::Bool(false)),
            ]);
            if secret_scan(&payload) || secret_scan(&canonical_json(&unsigned)) {
                return Err(fail(
                    "standard.contains_secrets",
                    "standard payload contains secrets; refuse to publish".into(),
                ));
            }
            let digest = digest_value(&unsigned);
            let key = store
                .continuity_key()
                .map_err(|err| fail(err.code, err.message))?;
            let mac = hmac_sha256_hex(&key.secret, digest.as_bytes());
            let doc = object([
                ("standard_id", string(&id)),
                ("payload_digest", string(sha256_text(&payload))),
                ("signature_kind", string("local-continuity")),
                ("signature", object([
                    ("kind", string("local-continuity")),
                    ("value", string(&mac)),
                    ("key_id", string(&key.key_id)),
                    ("org_identity", Value::Bool(false)),
                ])),
                ("org_identity", Value::Bool(false)),
                ("contains_secrets", Value::Bool(false)),
                ("manifest_digest", string(&digest)),
            ]);
            store
                .put_named("standards", &id, &doc)
                .map_err(|err| fail(err.code, err.message))?;
            let stored = store
                .get_named("standards", &id)
                .map_err(|err| fail(err.code, err.message))?;
            let verified = verify_standard_document(&store, &stored)?;
            ok(&format!("standard {sub}"), 0, verified)
        }
        "status" => {
            let status = standard_status(&store, &id)?;
            ok("standard status", 0, status)
        }
        "preview" => {
            // Read-only by contract: it states what `adopt` would write and
            // writes nothing itself.
            let doc = load_standard(&store, &id)?;
            let verified = verify_standard_document(&store, &doc)?;
            let current = adoption_of(&store, &id);
            ok(
                "standard preview",
                0,
                merge(
                    verified,
                    [
                        ("current_adoption", current.clone().unwrap_or(Value::Null)),
                        (
                            "would_write",
                            new_adoption(&id, "adopted", &standard_digest(&doc), None),
                        ),
                        ("writes_state", Value::Bool(false)),
                    ],
                ),
            )
        }
        "adopt" => {
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let doc = load_standard(&store, &id)?;
            verify_standard_document(&store, &doc)?;
            if adoption_of(&store, &id).is_some() {
                return Err(fail(
                    "standard.already_adopted",
                    format!("standard `{id}` is already adopted; use `update` or `pin`"),
                ));
            }
            let record = new_adoption(&id, "adopted", &standard_digest(&doc), None);
            store
                .put_named("adoptions", &id, &record)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit("standard.adopt", "adoption", Some(&id));
            ok(
                "standard adopt",
                0,
                merge(record, [("previous_adoption", Value::Null)]),
            )
        }
        "pin" => {
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let doc = load_standard(&store, &id)?;
            verify_standard_document(&store, &doc)?;
            let previous = require_adoption(&store, &id)?;
            let digest = standard_digest(&doc);
            let record = new_adoption(&id, "pinned", &digest, Some(&digest));
            store
                .put_named("adoptions", &id, &record)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit("standard.pin", "adoption", Some(&id));
            ok(
                "standard pin",
                0,
                merge(record, [("previous_adoption", previous)]),
            )
        }
        "update" => {
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let doc = load_standard(&store, &id)?;
            verify_standard_document(&store, &doc)?;
            let previous = require_adoption(&store, &id)?;
            let from = previous
                .get("source_digest")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let to = standard_digest(&doc);
            if from == to {
                return Err(fail(
                    "standard.already_current",
                    format!("adoption of `{id}` already tracks the current standard"),
                ));
            }
            let was_pinned =
                previous.get("state").and_then(Value::as_str) == Some("pinned");
            let record = new_adoption(
                &id,
                if was_pinned { "pinned" } else { "adopted" },
                &to,
                was_pinned.then_some(to.as_str()),
            );
            store
                .put_named("adoptions", &id, &record)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit("standard.update", "adoption", Some(&id));
            ok(
                "standard update",
                0,
                merge(
                    record,
                    [
                        ("previous_adoption", previous),
                        ("from_digest", string(&from)),
                        ("to_digest", string(&to)),
                    ],
                ),
            )
        }
        "leave" | "rollback" | "revoke" => {
            // A destructive delete. It needs the same authority as any other
            // mutation, and it must not report success for a standard that
            // was never there.
            let _auth = authorize_store_apply(&store, args.project.as_deref())?;
            let removed = store
                .delete_named("standards", &id)
                .map_err(|err| fail(err.code, err.message))?;
            if !removed {
                return Err(fail(
                    "standard.absent",
                    format!("standard `{id}` is not present; nothing was removed"),
                ));
            }
            let _ = store.audit(&format!("standard.{sub}"), "standard", Some(&id));
            ok(
                &format!("standard {sub}"),
                0,
                object([
                    ("standard_id", string(&id)),
                    ("removed", Value::Bool(true)),
                    ("personal_files_preserved", Value::Bool(true)),
                ]),
            )
        }
        other => Err(fail("usage.invalid", format!("unknown standard subcommand `{other}`"))),
    }
}

fn exception_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let sub = args.subcommand.as_deref().unwrap_or("status");
    let id = args.id.clone().unwrap_or_else(|| "ex-1".into());
    match sub {
        "request" => {
            // Requesting an exception is authorized by the enrolled principal
            // registry, not by an approved exception. Gating it on
            // `authorize_store_apply` would require an approved exception in
            // order to ask for the first one.
            let proof = verify_caller(&store, args)?;
            if !proof.has_role("requester") {
                return Err(fail(
                    "exception.requester_role_required",
                    format!(
                        "principal `{}` is not enrolled as a requester",
                        proof.principal_id()
                    ),
                ));
            }
            if store.get_named("exceptions", &id).is_ok() {
                return Err(fail(
                    "exception.already_exists",
                    format!("exception `{id}` already exists; requesting again would overwrite its state"),
                ));
            }
            let rec = new_exception(&id, proof.principal_id(), "project", 9_999_999_999);
            store
                .put_named("exceptions", &id, &rec)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit("exception.requested", "exception", Some(&id));
            ok("exception request", 0, rec)
        }
        "approve" | "reject" | "revoke" => {
            // `--role` / `--actor` are caller-attested and are not identity;
            // the authority here is a verified enrolled principal.
            let proof = verify_caller(&store, args)?;
            let rec = store
                .get_named("exceptions", &id)
                .map_err(|err| fail(err.code, err.message))?;
            let next = match sub {
                "approve" => "approved",
                "reject" => "rejected",
                _ => "revoked",
            };
            let updated = transition(&rec, &proof, next, now_unix())
                .map_err(|err| fail(err.code, err.message))?;
            store
                .put_named("exceptions", &id, &updated)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit(
                &format!("exception.{next}"),
                "exception",
                Some(&format!("{id} by {}", proof.principal_id())),
            );
            ok(&format!("exception {sub}"), 0, updated)
        }
        "status" => {
            let rec = store
                .get_named("exceptions", &id)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = rec;
            let status = exception_state(&store, &id)?;
            ok("exception status", 0, status)
        }
        other => Err(fail("usage.invalid", format!("unknown exception subcommand `{other}`"))),
    }
}

/// Path of the version-controlled asset registry inside a project.
pub(crate) const ASSET_REGISTRY_REL: &str = ".ctxpect/assets.json";

pub(crate) fn load_asset_registry(project: Option<&Path>) -> Option<Value> {
    let text = fs::read_to_string(project?.join(ASSET_REGISTRY_REL)).ok()?;
    parse(&text).ok()
}

pub(crate) fn asset_lock(store: &Store) -> Value {
    let entries = store
        .list_named("assetlock")
        .unwrap_or_default()
        .iter()
        .filter_map(|id| store.get_named("assetlock", id).ok())
        .collect::<Vec<_>>();
    ctxpect_assets::lock_document(entries)
}

/// The assets overview, shared by the CLI and the API so both describe the
/// executor the same way.
pub(crate) fn assets_status(harness: &str, lock: Option<&Value>) -> Value {
    object([
        ("catalog", integrations_json(harness, true)),
        ("apm_authority", string("apm-unique-not-re-evaluated")),
        // The executor exists now; its limits are stated rather than implied.
        ("copy_executor", string("project-scoped")),
        (
            "copy_executor_scope",
            string(
                "copies registry-declared files into the project; fetches nothing and resolves no versions",
            ),
        ),
        ("pin_status", string("evidence-backed-unavailable")),
        ("lock", lock.cloned().unwrap_or(Value::Null)),
        ("sbom", lock.map_or(Value::Null, ctxpect_assets::sbom)),
    ])
}

fn assets_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let sub = args.subcommand.as_deref().unwrap_or("status");
    match sub {
        "status" | "list" => {
            let lock = args
                .store
                .as_ref()
                .and_then(|path| Store::open(path).ok())
                .map(|store| asset_lock(&store));
            ok("assets", 0, assets_status(&args.harness, lock.as_ref()))
        }
        "preview" | "copy" => {
            let project = args
                .project
                .as_ref()
                .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))?;
            let asset_id = args
                .id
                .as_deref()
                .ok_or_else(|| fail("usage.invalid", "`--id <asset>` is required".into()))?;
            let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
            let registry = load_asset_registry(Some(project.as_path()));
            let asset = ctxpect_assets::registered(registry.as_ref(), asset_id)
                .map_err(|err| fail(err.code, err.message))?;
            // Vetting happens in preview, so a refusal costs nothing.
            let plan = ctxpect_assets::preview(&root, &asset)
                .map_err(|err| fail(err.code, err.message))?;
            if sub == "preview" {
                return ok("assets preview", 0, plan.to_value());
            }

            let store = open_store(args)?;
            let _auth = authorize_store_apply(&store, Some(project.as_path()))?;
            let backup = ctxpect_assets::backup_dir(store.root(), &plan.tx_id);
            let meta = ctxpect_assets::apply(&root, &plan, &backup, true)
                .map_err(|err| fail(err.code, err.message))?;
            store
                .put_named("assetlock", asset_id, &ctxpect_assets::lock_entry(&plan))
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit("assets.copy", "asset", Some(asset_id));

            // Post-Receipt: the copy changed the project, so the state after it
            // is observed rather than assumed.
            let post = post_receipt(args, &store)?;
            ok(
                "assets copy",
                0,
                object([
                    ("transaction", meta),
                    ("plan", plan.to_value()),
                    ("post_receipt_id", string(&post)),
                    ("lock", asset_lock(&store)),
                ]),
            )
        }
        "rollback" => {
            let project = args
                .project
                .as_ref()
                .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))?;
            let tx = args
                .id
                .as_deref()
                .ok_or_else(|| fail("usage.invalid", "`--id <tx>` is required".into()))?;
            let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
            let store = open_store(args)?;
            let _auth = authorize_store_apply(&store, Some(project.as_path()))?;
            let backup = ctxpect_assets::backup_dir(store.root(), tx);
            let result = ctxpect_assets::rollback(&root, &backup)
                .map_err(|err| fail(err.code, err.message))?;
            let _ = store.audit("assets.rollback", "asset", Some(tx));
            let post = post_receipt(args, &store)?;
            ok(
                "assets rollback",
                0,
                merge(result, [("post_receipt_id", string(&post))]),
            )
        }
        "sbom" => {
            let store = open_store(args)?;
            ok("assets sbom", 0, ctxpect_assets::sbom(&asset_lock(&store)))
        }
        other => Err(fail(
            "usage.invalid",
            format!("unknown assets subcommand `{other}`"),
        )),
    }
}

fn advisor_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let consent = args.reason.as_deref() == Some("consent");
    let ack = args.text.as_deref() == Some("ack") || consent;
    let out = suggest(
        &["evidence-1"],
        args.text.as_deref().unwrap_or("redacted-payload"),
        consent,
        ack,
        args.adapter.as_deref().unwrap_or("none"),
    )
    .map_err(|err| fail(err.code, err.message))?;
    if let Some(store) = args.store.as_ref() {
        let store = Store::open(store).map_err(|err| fail(err.code, err.message))?;
        let _auth = authorize_store_apply(&store, args.project.as_deref())?;
        let id = out
            .get("candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("c_local");
        store
            .put_named("advisor", id, &out)
            .map_err(|err| fail(err.code, err.message))?;
    }
    ok("advisor", 0, out)
}

fn experiment_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let n_planned = args.n.unwrap_or(4);
    let experiment_id = args.id.clone().unwrap_or_else(|| "exp-local".into());
    let store = if let Some(path) = &args.store {
        Some(Store::open(path).map_err(|err| fail(err.code, err.message))?)
    } else {
        None
    };
    if let Some(store) = &store
        && let Ok(prev) = store.get_named("experiments", &experiment_id)
    {
        let prev_n = prev
            .pointer(&["contract", "n_planned"])
            .and_then(Value::as_i64)
            .or_else(|| prev.get("n_planned").and_then(Value::as_i64));
        if let Some(prev_n) = prev_n
            && prev_n != n_planned
        {
            return Err(fail(
                "effect.n_locked",
                "sample size cannot change after results are observed".into(),
            ));
        }
    }
    let contract = ExperimentContract {
        experiment_id: experiment_id.clone(),
        n_planned,
        margin: 1,
        metric: "instructions-present".into(),
    };
    let control: Vec<i64> = (0..n_planned)
        .map(|_| run_local_instructions_probe(false))
        .collect();
    let treatment: Vec<i64> = (0..n_planned)
        .map(|_| run_local_instructions_probe(true))
        .collect();
    let result = decide(&contract, &control, &treatment, false)
        .map_err(|err| fail(err.code, err.message))?;
    if let Some(store) = &store {
        let _auth = authorize_store_apply(store, args.project.as_deref())?;
        store
            .put_named("experiments", &experiment_id, &result)
            .map_err(|err| fail(err.code, err.message))?;
    }
    ok("experiment", 0, result)
}

fn policy_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    if let Some(path) = &args.from {
        let text = fs::read_to_string(path).map_err(|err| fail("io.missing", err.to_string()))?;
        let layers = parse(&text).map_err(|err| fail("policy.parse", err.to_string()))?;
        let result = evaluate(&layers, now_unix()).map_err(|err| fail(err.code, err.message))?;
        return ok(
            &format!("policy {}", args.subcommand.as_deref().unwrap_or("eval")),
            0,
            result,
        );
    }
    let store = open_store(args)?;
    let result = effective_store_policy(&store, args.project.as_deref());
    let exit = if result.get("mutation_allowed").and_then(Value::as_bool) == Some(true) {
        0
    } else {
        match result.get("verdict").and_then(Value::as_str) {
            Some("deny") => 2,
            _ => 3,
        }
    };
    ok(
        &format!("policy {}", args.subcommand.as_deref().unwrap_or("show")),
        exit,
        result,
    )
}

fn align_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    if args.left.is_some() && args.right.is_some() {
        return diff_cmd(args);
    }
    let index = store
        .list_receipts()
        .map_err(|err| fail(err.code, err.message))?;
    ok(
        "align status",
        0,
        object([
            ("receipts", index),
            ("byte_equality_is_pass", Value::Bool(false)),
        ]),
    )
}

fn adapter_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    match args.subcommand.as_deref() {
        Some("list") => ok("adapter list", 0, integrations_json(&args.harness, true)),
        Some("test") => ok(
            "adapter test",
            0,
            object([
                ("ran", Value::Bool(true)),
                ("families", Value::Int(i64::try_from(FAMILIES.len()).unwrap_or(18))),
                ("live_oracle_executed", Value::Bool(false)),
            ]),
        ),
        _ => Err(fail("usage.invalid", "adapter requires test|list".into())),
    }
}

fn ci_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let inspect_args = inspect_args(args)?;
    let report = inspect(inspect_args)?;
    let diagnosis = diagnose(&report.envelope);
    let mut exit = report.exit_code;
    if exit == 0
        && let Some(Value::Int(n)) = diagnosis.pointer(&["counts", "confirmed"])
        && *n > 0
    {
        exit = 2;
    }
    ok(
        "ci",
        exit,
        object([
            ("inspect_exit_code", Value::Int(i64::from(report.exit_code))),
            ("doctor", diagnosis),
            (
                "indeterminate_is_zero",
                Value::Bool(false),
            ),
        ]),
    )
}

pub fn redact_roots_from_product(args: &ProductArgs) -> RedactRoots {
    RedactRoots::new(args.project.as_deref(), args.codex_home.as_deref())
}

#[allow(dead_code)]
fn _use_obj_s() {
    let _ = obj([("k", s("v"))]);
}

pub fn listen_addr(args: &ProductArgs) -> String {
    args.listen
        .clone()
        .unwrap_or_else(|| "127.0.0.1:7420".to_string())
}

#[cfg(test)]
mod persist_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn relabel_snapshot_is_not_migration() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cx-disp-relabel-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("mkdir");
        let store = Store::open(&path).expect("open");
        let fake = parse(
            r#"{"schema":"ctxpect-receipt-v1","receipt_kind":"development-snapshot","snapshot_digest":"x"}"#,
        )
        .expect("parse");
        let err = persist_inspect(&store, &fake, "one-shot").expect_err("relabel");
        assert_eq!(err.code(), "receipt.relabel_forbidden");
        match store.list_receipts().expect("index") {
            Value::Array(items) => assert!(items.is_empty(), "{items:?}"),
            other => panic!("expected array index, got {other:?}"),
        }
        let _ = fs::remove_dir_all(path);
    }
}
