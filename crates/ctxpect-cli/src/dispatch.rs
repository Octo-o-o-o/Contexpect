//! Product command dispatch (everything except the inspect slice).

use crate::args::{InspectArgs, ProductArgs};
use crate::catalog::{integrations_json, FAMILIES};
use crate::inspect::{inspect, InspectFailure};
use crate::jsonutil::{obj, s};
use crate::redact::{redact_json_envelope, RedactRoots};
use ctxpect_advisor::suggest;
use ctxpect_diff::{diff, EquivalenceProfile};
use ctxpect_doctor::{diagnose, render_project_findings, with_project_findings, ScannedFile};
use ctxpect_effect::{decide as effect_decide, not_executed, RunsDocument};
use ctxpect_fs::Root;
use ctxpect_importer::{import_session_bytes, insight};
use ctxpect_policy::{
    authorize_mutation, evaluate, exception_status, new_exception, transition, MutationScope,
    ANY_TARGET, POLICY_FRESHNESS_SECS,
};
use ctxpect_projection::{
    apply as proj_apply, launch_argv, pending_transactions, preview as proj_preview, read_tx,
    rollback as proj_rollback, target_outside_store, valid_tx_id, Intent, Preview, PREVIEW_APPLIED,
    PREVIEW_ROLLED_BACK,
};
use ctxpect_receipt::{
    migrate_dev_inspect_v0, reject_relabeled_snapshot, verify_local_continuity,
};
use ctxpect_schema::{
    array, canonical_json, digest_value, hmac_sha256_hex, object, parse, sha256_text, string, Value,
};
use ctxpect_store::{now_rfc3339, MutationLock, Store};
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
        "store" => store_cmd(&args),
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

/// The policy layers in force and the file they were read from.
///
/// Order: the store's `active` policy, any other stored policy, then the
/// project's `.ctxpect/policy.json`. The path is what freshness is measured
/// on: a policy is as fresh as the last time its source was written.
pub(crate) fn policy_source(store: &Store, project: Option<&Path>) -> Option<(Value, std::path::PathBuf)> {
    if let Ok(layers) = store.get_named("policies", "active") {
        return Some((layers, store.root().join("policies/active.json")));
    }
    if let Ok(ids) = store.list_named("policies") {
        for id in ids {
            if let Ok(layers) = store.get_named("policies", &id) {
                return Some((layers, store.root().join(format!("policies/{id}.json"))));
            }
        }
    }
    if let Some(project) = project {
        let path = project.join(".ctxpect/policy.json");
        if let Ok(text) = fs::read_to_string(&path)
            && let Ok(layers) = parse(&text)
        {
            return Some((layers, path));
        }
    }
    None
}

pub(crate) fn load_policy_layers(store: &Store, project: Option<&Path>) -> Option<Value> {
    policy_source(store, project).map(|(layers, _)| layers)
}

/// Whether the policy source was refreshed within [`POLICY_FRESHNESS_SECS`].
///
/// Without a policy source there is nothing to be fresh, so the answer is
/// `false`; the caller's policy check fails closed before this matters.
pub(crate) fn policy_fresh(store: &Store, project: Option<&Path>, now: i64) -> bool {
    let Some((_, path)) = policy_source(store, project) else {
        return false;
    };
    let Ok(meta) = fs::metadata(&path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let refreshed = modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    now.saturating_sub(refreshed) <= POLICY_FRESHNESS_SECS
}

/// Digest that identifies the project a mutation touches; the store root
/// stands in when no project is declared. Never a path: it is recorded on
/// exception records, which are publishable.
pub(crate) fn project_digest(store: &Store, project: Option<&Path>) -> String {
    project_scope_digest(store.root(), project)
}

/// The scope digest for a store root and optional project. Public so tests
/// and tooling can compute the digest an exception must carry.
#[must_use]
pub fn project_scope_digest(store_root: &Path, project: Option<&Path>) -> String {
    match project {
        Some(path) => {
            let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            sha256_text(&format!("project:{}", canonical.display()))
        }
        None => {
            let root = fs::canonicalize(store_root).unwrap_or_else(|_| store_root.to_path_buf());
            sha256_text(&format!("store:{}", root.display()))
        }
    }
}

pub(crate) fn mutation_scope(
    store: &Store,
    project: Option<&Path>,
    action: &str,
    target: &str,
) -> MutationScope {
    MutationScope::new(action, &project_digest(store, project), target)
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
    scope: &MutationScope,
) -> Result<Value, ctxpect_policy::PolicyError> {
    let layers = load_policy_layers(store, project);
    let exceptions = load_exceptions(store);
    let now = now_unix();
    authorize_mutation(
        layers.as_ref(),
        &exceptions,
        now,
        policy_fresh(store, project, now),
        scope,
    )
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

/// A granted mutation: the policy decision plus the store's advisory lock,
/// held until this value is dropped. Every mutation entry binds it for the
/// duration of its writes (`let _auth = authorize_store_apply(..)?;`).
#[derive(Debug)]
pub(crate) struct Authorization {
    #[allow(dead_code)]
    pub decision: Value,
    _lock: MutationLock,
}

/// The one authority every mutation passes through.
///
/// `action` and `target` are part of the decision: an approved exception
/// grants only the action, project and target it was approved for. The
/// store's same-machine advisory lock is taken here (`store.busy` when
/// another live process holds it) and released when the returned
/// [`Authorization`] is dropped.
pub(crate) fn authorize_store_apply(
    store: &Store,
    project: Option<&Path>,
    action: &str,
    target: &str,
) -> Result<Authorization, InspectFailure> {
    let lock = store.lock_mutation().map_err(|err| fail(err.code, err.message))?;
    let scope = mutation_scope(store, project, action, target);
    match store_mutation_decision(store, project, &scope) {
        Ok(auth) => {
            let detail = auth
                .pointer(&["exception", "exception_id"])
                .and_then(Value::as_str)
                .map(|id| format!("{action} {target} via {id}"));
            let _ = store.audit("mutation.allowed", "mutation", detail.as_deref());
            Ok(Authorization {
                decision: auth,
                _lock: lock,
            })
        }
        Err(err) => {
            let _ = store.audit(
                "mutation.denied",
                "mutation",
                Some(&format!("{action} {target}: {}", err.code)),
            );
            Err(fail(err.code, err.message))
        }
    }
}

/// Same source and decision as [`authorize_store_apply`], returned as a
/// viewer document for one scope. Top-level `verdict` is the mutation
/// decision for that scope, not a hardcoded five-layer pass.
pub(crate) fn effective_store_policy(
    store: &Store,
    project: Option<&Path>,
    action: &str,
    target: &str,
) -> Value {
    let layers = load_policy_layers(store, project);
    let now = now_unix();
    let layer_evaluation = layers
        .as_ref()
        .and_then(|item| evaluate(item, now).ok())
        .unwrap_or(Value::Null);
    let scope = mutation_scope(store, project, action, target);
    let fresh = policy_fresh(store, project, now);
    match store_mutation_decision(store, project, &scope) {
        Ok(auth) => object([
            ("schema", string("ctxpect-policy-effective-v1")),
            ("scope", scope.to_value()),
            ("mutation_allowed", Value::Bool(true)),
            ("reason_code", string("ok")),
            ("verdict", string("pass")),
            ("layers_present", Value::Bool(true)),
            ("policy_source_fresh", Value::Bool(fresh)),
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
                ("scope", scope.to_value()),
                ("mutation_allowed", Value::Bool(false)),
                ("reason_code", string(err.code)),
                ("verdict", string(verdict)),
                ("layers_present", Value::Bool(layers.is_some())),
                ("policy_source_fresh", Value::Bool(fresh)),
                ("layer_evaluation", layer_evaluation),
                ("message", string(err.message)),
            ])
        }
    }
}

/// The action and target a `policy show` / `GET /api/v1/policy` question is
/// about. Defaults match `apply` on the default projection target so the
/// viewer and the mutation answer the same question.
pub(crate) fn policy_query_scope(action: Option<&str>, target: Option<&str>) -> (String, String) {
    (
        action.unwrap_or("apply").to_string(),
        target.unwrap_or("AGENTS.md").to_string(),
    )
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

/// Standard-document MAC rule v2: a domain-labelled canonical object, so a
/// Receipt or audit MAC minted with the same store key cannot be presented
/// as a standard signature. Documents published under v1 (no `algorithm`
/// tag, MAC over the bare digest) keep verifying under v1.
pub(crate) const STANDARD_SIGNATURE_ALGORITHM_V2: &str = "hmac-sha256/ctxpect-standard-v2";

fn standard_mac_input_v2(digest: &str) -> String {
    canonical_json(&object([
        ("digest", string(digest)),
        ("domain", string("ctxpect/standard/v2")),
    ]))
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
    // Rule selection by the persisted algorithm tag. The tag is NOT part of
    // the signed input: `standard_unsigned` keeps only `standard_id`,
    // `payload_digest`, `signature_kind` and `org_identity`, so the tag
    // itself is not covered by the MAC. A cross-rule presentation (a v1 MAC
    // offered as v2, or the reverse) still fails because the two rules MAC
    // different inputs: v1 MACs the bare digest, v2 MACs a domain-labelled
    // canonical object, so the expected values cannot coincide.
    let input = match doc.pointer(&["signature", "algorithm"]).and_then(Value::as_str) {
        None | Some("hmac-sha256") => digest.clone(),
        Some(STANDARD_SIGNATURE_ALGORITHM_V2) => standard_mac_input_v2(&digest),
        Some(other) => {
            return Err(fail(
                "standard.signature_algorithm_unknown",
                format!("signature.algorithm `{other}` is not a rule this verifier knows"),
            ));
        }
    };
    let expected = hmac_sha256_hex(&key.secret, input.as_bytes());
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
    ctxpect_doctor::contains_secret(text)
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

/// Importing under an id that already holds a *different* session record
/// is refused (`store.session_exists`): the same artifact again is
/// idempotent, a different one is not a silent replacement.
pub(crate) fn refuse_replacing_a_different_session(
    store: &Store,
    session_id: &str,
    session: &Value,
) -> Result<(), InspectFailure> {
    if let Ok(existing) = store.get_named("sessions", session_id)
        && canonical_json(&existing) != canonical_json(session)
    {
        return Err(fail(
            "store.session_exists",
            format!("session `{session_id}` already holds a different record; delete it first or import under another id"),
        ));
    }
    Ok(())
}

/// The one `receipt verify` answer, for the CLI and the API alike (R04).
/// A tombstone is `ok: false` / `receipt.tombstoned` (exit 3): its
/// signature is the original's and never verifies, but "deleted" is the
/// answer, not "tampered". A Receipt signed before `signed_at` existed is
/// `ok: false` / `receipt.signature_legacy` (exit 3). A digest or MAC
/// mismatch is an error.
pub(crate) fn verify_receipt_report(store: &Store, id: &str) -> Result<(i32, Value), InspectFailure> {
    let receipt = store
        .get_receipt(id)
        .map_err(|err| fail(err.code, err.message))?;
    if receipt.get("tombstone").map(|v| v != &Value::Null).unwrap_or(false)
        && receipt.get("claims").and_then(Value::as_array).map(|a| a.is_empty()).unwrap_or(false)
    {
        return Ok((
            3,
            object([
                ("ok", Value::Bool(false)),
                ("reason_code", string("receipt.tombstoned")),
                (
                    "signature_vs_attachment",
                    string("tombstone preserves digest; evidence bodies are purged"),
                ),
            ]),
        ));
    }
    let key = store
        .continuity_key()
        .map_err(|err| fail(err.code, err.message))?;
    match verify_local_continuity(&receipt, &key) {
        Ok(()) => {}
        Err(err) if err.code == "receipt.signature_legacy" => {
            return Ok((
                3,
                object([
                    ("ok", Value::Bool(false)),
                    ("reason_code", string(err.code)),
                    ("message", string(err.message)),
                    ("kind", string("local-continuity")),
                    ("org_identity", Value::Bool(false)),
                ]),
            ));
        }
        Err(err) => return Err(fail(err.code, err.message)),
    }
    Ok((
        0,
        object([
            ("ok", Value::Bool(true)),
            ("kind", string("local-continuity")),
            ("org_identity", Value::Bool(false)),
            (
                "signed_at",
                receipt.pointer(&["signature", "signed_at"]).cloned().unwrap_or(Value::Null),
            ),
            // Local clock readings are covered by the MAC but are not a
            // trusted time source.
            ("signed_at_trusted", Value::Bool(false)),
            ("covers", array([string("manifest.digest"), string("created_at"), string("signed_at")])),
        ]),
    ))
}

/// Persist a snapshot as a Receipt bound to the project it was observed in:
/// `scope.project_digest` (the same path digest exceptions and previews
/// bind to) travels into `coordinate.project_digest`, so a consumer — the
/// daemon's Doctor, Monitor, care plan — can tell whether a Receipt belongs
/// to the root it is about to rescan. Without a project the digest stays
/// `unknown`; it is never guessed from the store.
pub(crate) fn persist_inspect_in(
    store: &Store,
    snapshot: &Value,
    kind: &str,
    project: Option<&Path>,
) -> Result<Value, InspectFailure> {
    let stamped = match project {
        Some(project) => stamp_project_scope(snapshot, &project_digest(store, Some(project))),
        None => snapshot.clone(),
    };
    persist_inspect(store, &stamped, kind)
}

/// The snapshot with `scope.project_digest` set (a copy; the input is not
/// changed).
pub(crate) fn stamp_project_scope(snapshot: &Value, digest: &str) -> Value {
    let mut out = snapshot.clone();
    if let Value::Object(map) = &mut out
        && let Some(Value::Object(scope)) = map.get_mut("scope")
    {
        scope.insert("project_digest".into(), string(digest));
    }
    out
}

pub fn persist_inspect(
    store: &Store,
    snapshot: &Value,
    kind: &str,
) -> Result<Value, InspectFailure> {
    reject_relabeled_snapshot(snapshot)
        .map_err(|err| fail(err.code, err.message))?;
    // Persisting a Receipt is a store mutation: it takes the advisory lock
    // (reentrant when the caller already holds it through an Authorization).
    let _lock = store.lock_mutation().map_err(|err| fail(err.code, err.message))?;
    let key = store
        .continuity_key()
        .map_err(|err| fail(err.code, err.message))?;
    let id = ctxpect_receipt::receipt_id_for(kind, &canonical_json(snapshot));
    // One clock reading serves as both creation and signing time; both are
    // display-only local times covered by the MAC.
    let now = now_rfc3339();
    let receipt = migrate_dev_inspect_v0(snapshot, kind, Some(&key), &now, &now, &id)
        .map_err(|err| fail(err.code, err.message))?;
    store
        .put_snapshot(&id, snapshot)
        .map_err(|err| fail(err.code, err.message))?;
    store
        .put_receipt(&receipt)
        .map_err(|err| fail(err.code, err.message))?;
    // First write wins in the store: the document returned is the one on
    // disk, which for a repeated snapshot is the *earlier* Receipt (older
    // created_at, older MAC), not the one just constructed.
    store
        .get_receipt(&id)
        .map_err(|err| fail(err.code, err.message))
}

/// Largest file the Doctor content rules read in full. Larger files are
/// scanned by name only; the collector still digests them whole.
const DOCTOR_READ_CAP: u64 = 1_048_576;

/// The project as the Doctor content rules see it: every regular, non-withheld
/// file the passive collector found, read through containment, plus symlinks
/// with their targets as written. Nothing is executed or extracted.
pub fn scan_project_for_doctor(root: &Root) -> Result<Vec<ScannedFile>, InspectFailure> {
    let inventory =
        ctxpect_collect::scan(root).map_err(|err| fail("io.unresolvable", err.to_string()))?;
    let mut files = Vec::new();
    for entry in &inventory.entries {
        match entry.kind {
            ctxpect_fs::EntryKind::Symlink => files.push(ScannedFile {
                path: entry.path.clone(),
                bytes: None,
                link_target: entry.link_target.clone(),
                is_symlink: true,
            }),
            ctxpect_fs::EntryKind::File => {
                let readable = entry.withheld.is_none()
                    && entry.len.is_none_or(|len| len <= DOCTOR_READ_CAP);
                let bytes = if readable {
                    ctxpect_fs::read_contained(root, root.path().join(&entry.path))
                        .ok()
                        .filter(|content| !content.truncated && content.link_count <= 1)
                        .map(|content| content.bytes)
                } else {
                    None
                };
                files.push(ScannedFile {
                    path: entry.path.clone(),
                    bytes,
                    link_target: None,
                    is_symlink: false,
                });
            }
            _ => {}
        }
    }
    Ok(files)
}

/// The evaluation clock for a Doctor run. Without `--as-of` both readings
/// come from the wall clock; with it, the `stale` rule judges against that
/// civil date and suppression expiry against its noon-UTC reading
/// (`evaluated_at_secs` in the output shows which), so a run is reproducible
/// and the date a stale finding names is the date it was judged against.
fn doctor_clock(as_of: Option<(i64, u32, u32)>) -> ((i64, u32, u32), i64) {
    match as_of {
        Some((year, month, day)) => (
            (year, month, day),
            ctxpect_doctor::days_from_civil(year, month, day) * 86_400 + 43_200,
        ),
        None => {
            let now_secs = i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            )
            .unwrap_or(0);
            (ctxpect_doctor::civil_from_days(now_secs.div_euclid(86_400)), now_secs)
        }
    }
}

/// Doctor content findings for a project, rendered. `stale` is judged
/// against the wall-clock date; callers with an explicit reference date use
/// [`ctxpect_doctor::project_findings_in`] directly.
pub fn project_doctor_findings(root: &Root) -> Result<Vec<Value>, InspectFailure> {
    let files = scan_project_for_doctor(root)?;
    let (as_of, _) = doctor_clock(None);
    Ok(render_project_findings(&ctxpect_doctor::project_findings_in(&files, Some(root.path()), as_of)))
}

/// Receipt-derived findings plus project-content findings, with the
/// project's suppressions applied: the one diagnosis `doctor`, `ci` and the
/// API report (R04). `as_of` pins the evaluation clock (see [`doctor_clock`]);
/// `None` reads the wall clock.
pub(crate) fn diagnosis_for_root(base: Value, root: &Root, as_of: Option<(i64, u32, u32)>) -> Result<Value, InspectFailure> {
    let files = scan_project_for_doctor(root)?;
    let (as_of, now_secs) = doctor_clock(as_of);
    let project_findings =
        render_project_findings(&ctxpect_doctor::project_findings_in(&files, Some(root.path()), as_of));
    let merged = with_project_findings(base, &project_findings);
    // Evidence digests bind a suppression to the reviewed bytes.
    let mut evidence = std::collections::BTreeMap::new();
    for file in &files {
        if let Some(bytes) = &file.bytes {
            evidence.insert(file.path.clone(), ctxpect_schema::sha256_hex(bytes));
        } else if let Some(target) = &file.link_target {
            evidence.insert(file.path.clone(), sha256_text(target));
        }
    }
    let suppressions_path = root.path().join(ctxpect_doctor::SUPPRESSIONS_PATH);
    let (document, file_error, file_digest) = if suppressions_path.exists() {
        match ctxpect_fs::read_contained(root, &suppressions_path) {
            Ok(content) => match parse(&String::from_utf8_lossy(&content.bytes)) {
                Ok(doc) => (Some(doc), None, Some(ctxpect_schema::sha256_hex(&content.bytes))),
                Err(err) => (None, Some(format!("does not parse: {err}")), Some(ctxpect_schema::sha256_hex(&content.bytes))),
            },
            Err(err) => (None, Some(format!("cannot be read within the project: {err}")), None),
        }
    } else {
        (None, None, None)
    };
    Ok(ctxpect_doctor::apply_suppressions(
        merged,
        &ctxpect_doctor::SuppressionInput {
            document: document.as_ref(),
            file_error,
            file_digest,
            evidence: &evidence,
            now_secs,
        },
    ))
}

/// Receipt-derived findings plus project-content findings, the diagnosis
/// both `doctor` and `ci` (and the API) report. `as_of` is the explicit
/// evaluation clock (`--as-of`); `None` reads the wall clock.
pub(crate) fn full_diagnosis(snapshot: &Value, project: &Path, as_of: Option<(i64, u32, u32)>) -> Result<Value, InspectFailure> {
    let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
    diagnosis_for_root(diagnose(snapshot), &root, as_of)
}

fn collect_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    if args.execute || args.adapter.is_some() {
        if !args.execute {return Err(fail("native.execute_required","native capture requires explicit --execute".into()));}
        let store=open_store(args)?;
        let _auth=authorize_store_apply(&store,args.project.as_deref(),"native.capture",&args.harness)?;
        let key=store.continuity_key().map_err(|e|fail(e.code,e.message))?;
        let observation=crate::native_oracle::capture(args,&key.secret).map_err(|c|fail(c,"native oracle refused; version, profile and actual output must match".into()))?;
        let id=format!("native_{}",&sha256_text(&canonical_json(&observation))[..32]);
        store.put_named("nativeobservations",&id,&observation).map_err(|e|fail(e.code,e.message))?;
        return ok("collect",0,object([("observation_id",string(id)),("native_observation",observation),("receipt_id",Value::Null)]));
    }
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
    let mut evidence = Vec::new();
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
        evidence.push(object([
            ("root", string("project")),
            ("path", string(&entry.path)),
            (
                "content_digest",
                match &entry.content_digest {
                    Some(d) => string(d),
                    None => Value::Null,
                },
            ),
        ]));
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
    // `--store` persists the inventory as a `device-baseline` Receipt: the
    // inventory entries are its evidence, the inventory digest its snapshot
    // digest. Without a store there is no Receipt and the output says so.
    let baseline = match args.store.as_ref() {
        Some(path) => {
            let store = Store::open(path).map_err(|err| fail(err.code, err.message))?;
            let snapshot = object([
                ("schema", string(ctxpect_receipt::DEV_INSPECT_SCHEMA)),
                ("receipt_kind", string(ctxpect_receipt::DEV_INSPECT_KIND)),
                ("snapshot_digest", string(inventory.digest())),
                (
                    "scope",
                    object([
                        ("harness", string(&args.harness)),
                        ("version", string(&args.version)),
                        ("surface", string(&args.surface)),
                        ("os_lane", string(&args.os_lane)),
                        ("cwd", string("<project>/")),
                        ("roots", object([("project", string("<project>"))])),
                    ]),
                ),
                (
                    "results",
                    array([object([
                        ("capability_id", string("inventory")),
                        ("evidence", array(evidence.clone())),
                        ("edges", array([])),
                        ("layers", array([])),
                    ])]),
                ),
                ("unknown", array([])),
                ("findings", array([])),
                (
                    "policy_result",
                    object([("verdict", string("indeterminate")), ("reason_code", string("inventory_only"))]),
                ),
                ("required", array([])),
                (
                    "warnings",
                    if inventory.truncated {
                        array([object([("reason_code", string("scan.file_limit_reached"))])])
                    } else {
                        array([])
                    },
                ),
            ]);
            let receipt = persist_inspect_in(&store, &snapshot, "device-baseline", args.project.as_deref())?;
            Some(
                receipt
                    .get("receipt_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )
        }
        None => None,
    };
    ok(
        args.command.as_str(),
        0,
        object([
            ("inventory_digest", string(inventory.digest())),
            (
                "receipt_id",
                baseline.as_deref().map_or(Value::Null, string),
            ),
            ("receipt_kind", string("device-baseline")),
            ("receipt_persisted", Value::Bool(baseline.is_some())),
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
    let diagnosis = full_diagnosis(&report.envelope, &inspect_args_project(args)?, args.as_of)?;
    // The same judgement `ci` applies (C1): one function, two entry points.
    let exit = combine_exits(&[
        report.exit_code,
        ctxpect_doctor::blocking_exit(&diagnosis, args.fail_on.as_deref()),
    ]);
    let mut body = match diagnosis {
        Value::Object(map) => Value::Object(map),
        other => object([("doctor", other)]),
    };
    if let Value::Object(map) = &mut body {
        map.insert("inspect_exit_code".into(), Value::Int(i64::from(report.exit_code)));
        map.insert("snapshot_schema".into(), string("dev-inspect-v0"));
        // The date `stale` was judged against: the `--as-of` value, else the
        // wall-clock date. Naming it keeps a re-analysis of old evidence
        // distinguishable from a fresh observation (C-F05).
        let (as_of, _) = doctor_clock(args.as_of);
        map.insert(
            "as_of".into(),
            string(format!("{:04}-{:02}-{:02}", as_of.0, as_of.1, as_of.2)),
        );
    }
    if let Some(store_path) = &args.store {
        let store = Store::open(store_path).map_err(|err| fail(err.code, err.message))?;
        let receipt = persist_inspect_in(&store, &report.envelope, "one-shot", args.project.as_deref())?;
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
            let (exit, report) = verify_receipt_report(&store, id)?;
            ok("receipt verify", exit, report)
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
            let id = args
                .receipt
                .as_deref()
                .or(args.id.as_deref())
                .ok_or_else(|| fail("usage.invalid", "`--receipt <id>` is required".into()))?;
            let _auth = authorize_store_apply(&store, args.project.as_deref(), "receipt.redact", id)?;
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
    let receipt = persist_inspect_in(&store, &report.envelope, "preflight", args.project.as_deref())?;
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
    let _auth = authorize_store_apply(&store, args.project.as_deref(), "preflight.intent", &id)?;
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
        let path = args
            .from
            .as_ref()
            .ok_or_else(|| fail("usage.invalid", "`--from <file>` is required".into()))?;
        // Bytes, not text: a native artifact may be an encoding the importer
        // has to refuse by its magic bytes rather than fail to decode.
        let bytes = fs::read(path).map_err(|err| fail("io.missing", err.to_string()))?;
        // The default id is the content digest, so importing the same file
        // twice lands in the same session record.
        let session_id = args
            .session
            .clone()
            .unwrap_or_else(|| format!("s_{}", &ctxpect_schema::sha256_hex(&bytes)[..12]));
        let _auth =
            authorize_store_apply(&store, args.project.as_deref(), "sessions.import", &session_id)?;
        let mapping = args.mapping.as_deref().unwrap_or("generic-json");
        // The store is metadata-only: no body preview is kept (see importer).
        let session = import_session_bytes(&bytes, mapping, &session_id)
            .map_err(|err| fail(err.code, err.message))?;
        refuse_replacing_a_different_session(&store, &session_id, &session)?;
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
            let _auth =
                authorize_store_apply(&store, args.project.as_deref(), "sessions.delete", session_id)?;
            let existed = store
                .delete_named("sessions", session_id)
                .map_err(|err| fail(err.code, err.message))?;
            if !existed {
                // Reporting a deletion that removed nothing reads as a
                // destructive action that never happened.
                return Err(fail("store.missing", format!("sessions/{session_id}.json")));
            }
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

/// Probe the daemon this store's `daemon.addr` names: a TCP connect plus a
/// loopback `GET /api/v1/health`. `None` when no address was recorded.
fn probe_daemon(store: &Store) -> Option<bool> {
    use std::io::{Read, Write};
    let addr = fs::read_to_string(store.root().join("daemon.addr")).ok()?;
    let addr = addr.trim().to_string();
    if addr.is_empty() {
        return None;
    }
    let Ok(mut stream) = std::net::TcpStream::connect_timeout(
        &addr.parse().ok()?,
        std::time::Duration::from_millis(300),
    ) else {
        return Some(false);
    };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
    let request = format!("GET /api/v1/health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return Some(false);
    }
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    Some(text.starts_with("HTTP/1.1 200") && text.contains("\"ok\":true"))
}

fn daemon_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    match args.subcommand.as_deref() {
        Some("start") => crate::http::serve(args),
        Some("status") => {
            let store = open_store(args)?;
            let pid_path = store.root().join("daemon.pid");
            // A pid file is a claim, not a process. The daemon is running when
            // it answers on the address it recorded.
            let probe = probe_daemon(&store);
            let running = probe == Some(true);
            ok(
                "daemon status",
                0,
                object([
                    ("running", Value::Bool(running)),
                    ("pid_file_present", Value::Bool(pid_path.exists())),
                    (
                        "probe",
                        string(match probe {
                            Some(true) => "reachable",
                            Some(false) => "unreachable",
                            None => "no_addr",
                        }),
                    ),
                    ("oneshot_without_daemon", Value::Bool(true)),
                ]),
            )
        }
        Some("stop") => {
            let store=open_store(args)?;
            let stopped=stop_owned_daemon(&store)?;
            ok("daemon stop",if stopped {0} else {3},object([
                ("stopped",Value::Bool(stopped)),("shutdown_requested",Value::Bool(true)),
                ("signal_sent",Value::Bool(false)),
                ("reason_code",string(if stopped {"daemon.stopped"} else {"daemon.shutdown_pending"})),
            ]))
        }
        _ => Err(fail("usage.invalid", "daemon requires start|stop|status".into())),
    }
}

/// Sync state read from a store. Shared by `sync status` and `GET /api/v1/sync`.
///
/// The default HTTP transport has no key profile. Encrypted CLI groups are
/// listed separately; historical application is not current signature trust.
pub(crate) fn sync_status_doc(store: &Store) -> Value {
    let settings = store.settings().unwrap_or(Value::Null);
    let vault_required = settings.get("vault").and_then(Value::as_str) == Some("required");
    let bundles = store.list_named("sync").unwrap_or_default();
    let secure_groups = match store.list_named("secureheads") {
        Ok(ids) => array(ids.iter().map(|id| {
            match store.get_named("secureheads",id) {
                Ok(record) => object([
                    ("group",string(id)),
                    ("generation",record.pointer(&["document","envelope","generation"]).cloned().unwrap_or(Value::Null)),
                    ("epoch",record.pointer(&["document","envelope","epoch"]).cloned().unwrap_or(Value::Null)),
                    ("semantic",string("structural-only")),("native_projection",string("not-applied")),
                    ("current_signature_trust",string("not-rechecked")),
                ]),
                Err(_) => object([("group",string(id)),("reason_code",string("sync.state_unreadable"))]),
            }
        })),
        Err(_) => Value::Null,
    };
    let receipts = match store.list_receipts() {
        Ok(Value::Array(items)) => i64::try_from(items.len()).unwrap_or(0),
        _ => 0,
    };
    object([
        ("schema", string("ctxpect-sync-status-v1")),
        ("encryption", string("unavailable")),
        ("encryption_reason_code", string("sync.profile_required")),
        ("external_encryption_adapter", string("age-ssh-v1")),
        ("encrypted_entrypoint", string("cli-profile")),
        ("secure_groups", secure_groups),
        // A transport that succeeded moved bytes; it did not verify meaning.
        ("transport_success_is_verified", Value::Bool(false)),
        ("vault_required", Value::Bool(vault_required)),
        ("local_bundles", Value::Int(i64::try_from(bundles.len()).unwrap_or(0))),
        (
            "bundle_ids",
            array(bundles.iter().map(|id| string(id.as_str())).collect::<Vec<_>>()),
        ),
        ("syncable_receipts", Value::Int(receipts)),
        (
            "remote",
            object([
                ("configured", Value::Bool(false)),
                ("reason_code", string("sync.no_remote_transport")),
            ]),
        ),
    ])
}

/// `sync status` reports the store's sync state; `sync preview` packs a
/// bundle and (with `--dest`) previews its application; `sync apply` applies.
/// Three operations, three envelopes.
fn sync_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    if args.adapter.as_deref() == Some("age-ssh-v1") { return secure_sync_cmd(args); }
    if args.adapter.is_some() || args.subcommand.as_deref() == Some("seal") {
        return Err(fail("sync.adapter_required", "encrypted sync requires --adapter age-ssh-v1".into()));
    }
    let store = open_store(args)?;
    let settings = store.settings().map_err(|err| fail(err.code, err.message))?;
    let vault_required = settings.get("vault").and_then(Value::as_str) == Some("required");
    match args.subcommand.as_deref() {
        Some("status") => ok("sync status", 0, sync_status_doc(&store)),
        Some("preview") | Some("apply") => {
            let receipts = if let Some(id) = args.receipt.as_deref() {
                vec![store.get_receipt(id).map_err(|err| fail(err.code, err.message))?]
            } else {
                Vec::new()
            };
            let bundle_id = args.id.as_deref().unwrap_or("bundle-local");
            let packed = bundle(
                bundle_id,
                object([("desired", string("metadata-only"))]),
                &["env:CTXPECT_TOKEN"],
                receipts,
                vault_required,
            )
            .map_err(|err| fail(err.code, err.message))?;
            if args.subcommand.as_deref() == Some("preview") {
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
            let _auth = authorize_store_apply(&store, args.project.as_deref(), "sync.apply", bundle_id)?;
            let result = apply_folder(dest, &packed).map_err(|err| fail(err.code, err.message))?;
            ok("sync apply", 0, result)
        }
        _ => Err(fail("usage.invalid", "sync requires preview|apply|status".into())),
    }
}

fn intent_from_args(args: &ProductArgs) -> Intent {
    Intent {
        intent_id: args.id.clone().unwrap_or_else(|| "intent-1".into()),
        authority: args
            .authority
            .clone()
            .unwrap_or_else(|| "contexpect-native".into()),
        target_rel: args.target.clone().unwrap_or_else(|| "AGENTS.md".into()),
        desired: args.desired.clone().unwrap_or_else(|| "updated\n".into()),
    }
}

/// Persist a preview so a later `apply --tx` is bound to exactly it.
pub(crate) fn persist_preview(store: &Store, preview: &Preview) -> Result<(), InspectFailure> {
    store
        .put_named("previews", &preview.tx_id, &preview.to_record())
        .map_err(|err| fail(err.code, err.message))
}

/// Load the preview an apply must be bound to, and refuse one computed for
/// another project (`projection.preview_scope`) or already consumed
/// (`projection.tx_consumed`).
pub(crate) fn load_preview(store: &Store, tx: &str, project_digest: &str) -> Result<Preview, InspectFailure> {
    let record = store.get_named("previews", tx).map_err(|_| {
        fail(
            "projection.preview_missing",
            format!("no persisted preview `{tx}`; run `intent preview` first"),
        )
    })?;
    let preview = Preview::from_record(&record).map_err(|err| fail(err.code, err.message))?;
    preview
        .check_applicable(project_digest)
        .map_err(|err| fail(err.code, err.message))?;
    Ok(preview)
}

/// Record that a preview was consumed (`applied`) or closed (`rolled-back`).
/// A missing record (a rollback of a transaction whose preview is gone) is
/// not an error: there is nothing to consume.
pub(crate) fn mark_preview(store: &Store, tx: &str, state: &str) -> Result<(), InspectFailure> {
    let Ok(record) = store.get_named("previews", tx) else {
        return Ok(());
    };
    let preview = Preview::from_record(&record).map_err(|err| fail(err.code, err.message))?;
    persist_preview(store, &preview.with_state(state))
}

/// Whether a store status document, plus the judged projection
/// transactions, leaves anything undecided: an unhealthy index or audit
/// log, an in-doubt journal record, or a transaction that is neither
/// recovered nor aborted.
fn store_in_doubt(status: &Value, transactions: &[Value]) -> bool {
    status.pointer(&["journal", "in_doubt"]).and_then(Value::as_i64).unwrap_or(0) > 0
        || status.pointer(&["index", "status"]).and_then(Value::as_str) != Some("ok")
        || status.pointer(&["audit", "status"]).and_then(Value::as_str) != Some("ok")
        || transactions.iter().any(|tx| {
            !matches!(
                tx.get("judgement").and_then(Value::as_str),
                Some("committed-recovered" | "aborted")
            )
        })
}

/// `store status` reports index health, unfinished journal records, the
/// advisory lock and pending projection transactions (judged, not
/// rewritten). `store repair` rebuilds the index from `receipts/` and
/// `tombstones/` and replays the journal.
fn store_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let transactions = match args.project.as_ref().map(Root::new) {
        Some(Ok(root)) => pending_transactions(
            &root,
            &store.root().join("apply"),
            &project_digest(&store, args.project.as_deref()),
        )
        .map_err(|err| fail(err.code, err.message))?,
        _ => Vec::new(),
    };
    match args.subcommand.as_deref() {
        Some("status") => {
            let status = store.status().map_err(|err| fail(err.code, err.message))?;
            let in_doubt = store_in_doubt(&status, &transactions);
            ok(
                "store status",
                if in_doubt { 3 } else { 0 },
                merge(
                    status,
                    [
                        ("transactions", array(transactions)),
                        (
                            "transactions_note",
                            string("pending transactions are judged against the target's current digest; `rollback --id` closes a record, nothing is rewritten here"),
                        ),
                    ],
                ),
            )
        }
        Some("repair") => {
            let report = store.repair().map_err(|err| fail(err.code, err.message))?;
            let status = store.status().map_err(|err| fail(err.code, err.message))?;
            // What repair could not resolve is still in doubt: exit 3, like
            // `status`, rather than a 0 that reads as "mended".
            let in_doubt = store_in_doubt(&status, &transactions);
            ok(
                "store repair",
                if in_doubt { 3 } else { 0 },
                merge(report, [("status", status), ("transactions", array(transactions))]),
            )
        }
        _ => Err(fail("usage.invalid", "store requires status|repair".into())),
    }
}

/// `intent validate|show|project|preview`, `apply --tx`, `rollback --id`.
///
/// The four `intent` subcommands are four operations: `validate` checks the
/// intent's shape without touching the filesystem, `show` renders the
/// CanonicalIntent, `project` computes the projection preview without
/// persisting it, and `preview` computes **and persists** it so that
/// `apply --tx <id>` can be bound to the exact digests the user saw.
fn projection_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let project = args
        .project
        .as_ref()
        .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))?;
    let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
    if args.command == "intent" {
        let intent = intent_from_args(args);
        let sub = args.subcommand.as_deref().unwrap_or("preview");
        return match sub {
            "validate" => {
                let (valid, reason) = match intent.validate() {
                    Ok(()) => (true, Value::Null),
                    Err(err) => (false, string(err.code)),
                };
                ok(
                    "intent validate",
                    if valid { 0 } else { 2 },
                    object([
                        ("valid", Value::Bool(valid)),
                        ("reason_code", reason),
                        ("intent", intent.to_value()),
                        ("touches_filesystem", Value::Bool(false)),
                    ]),
                )
            }
            "show" => ok("intent show", 0, intent.to_value()),
            "project" => {
                let store_root = args.store.clone().unwrap_or_else(|| project.join(".ctxpect/store"));
                let scope = project_scope_digest(&store_root, Some(project.as_path()));
                target_outside_store(&root, &intent.target_rel, &store_root)
                    .map_err(|err| fail(err.code, err.message))?;
                let previewed =
                    proj_preview(&root, &intent, &scope).map_err(|err| fail(err.code, err.message))?;
                ok(
                    "intent project",
                    0,
                    merge(previewed.to_value(), [("persisted", Value::Bool(false))]),
                )
            }
            "preview" => {
                let store = open_store(args)?;
                let scope = project_digest(&store, Some(project.as_path()));
                target_outside_store(&root, &intent.target_rel, store.root())
                    .map_err(|err| fail(err.code, err.message))?;
                let previewed =
                    proj_preview(&root, &intent, &scope).map_err(|err| fail(err.code, err.message))?;
                persist_preview(&store, &previewed)?;
                ok(
                    "intent preview",
                    0,
                    merge(
                        previewed.to_value(),
                        [
                            ("persisted", Value::Bool(true)),
                            ("apply_with", string(format!("apply --tx {}", previewed.tx_id))),
                        ],
                    ),
                )
            }
            other => Err(fail("usage.invalid", format!("unknown intent subcommand `{other}`"))),
        };
    }
    let store = open_store(args)?;
    if args.command == "apply" && args.subcommand.as_deref() == Some("status") {
        // Read-only judgement of pending transactions; see `store status`.
        let transactions = pending_transactions(
            &root,
            &store.root().join("apply"),
            &project_digest(&store, Some(project.as_path())),
        )
        .map_err(|err| fail(err.code, err.message))?;
        let in_doubt = transactions.iter().any(|tx| {
            !matches!(
                tx.get("judgement").and_then(Value::as_str),
                Some("committed-recovered" | "aborted")
            )
        });
        return ok(
            "apply status",
            if in_doubt { 3 } else { 0 },
            object([
                ("transactions", array(transactions)),
                ("target_rewritten", Value::Bool(false)),
            ]),
        );
    }
    if args.command == "apply" {
        let tx = args.tx.as_deref().ok_or_else(|| {
            fail(
                "usage.invalid",
                "`apply` requires `--tx <id>` from `intent preview`; it does not recompute the preview".into(),
            )
        })?;
        // The lock is taken before the preview's state is read, so no other
        // process can consume or roll back this transaction in between.
        let _guard = store.lock_mutation().map_err(|err| fail(err.code, err.message))?;
        let previewed = load_preview(&store, tx, &project_digest(&store, Some(project.as_path())))?;
        target_outside_store(&root, &previewed.target_rel, store.root())
            .map_err(|err| fail(err.code, err.message))?;
        // Authorize before persisting. Writing the intent first leaves a
        // record of a mutation that policy went on to refuse.
        let _auth = authorize_store_apply(
            &store,
            Some(project.as_path()),
            "apply",
            &previewed.target_rel,
        )?;
        let backup = ctxpect_projection::backup_dir(store.root(), &previewed.tx_id);
        let result = proj_apply(&root, &previewed, &backup, true)
            .map_err(|err| fail(err.code, err.message))?;
        // The CanonicalIntent record lands after the apply succeeded: an
        // apply refused for a moved target leaves no record of an intent
        // that was never realised.
        let intent = Intent {
            intent_id: previewed.intent_id.clone(),
            authority: previewed.authority.clone(),
            target_rel: previewed.target_rel.clone(),
            desired: previewed.desired.clone(),
        };
        store
            .put_named("intents", &intent.intent_id, &intent.to_value())
            .map_err(|err| fail(err.code, err.message))?;
        // The preview is consumed by its apply.
        mark_preview(&store, &previewed.tx_id, PREVIEW_APPLIED)?;
        let _ = store.audit("projection.apply", "projection", Some(&previewed.tx_id));
        let post = post_receipt(args, &store)?;
        return ok(
            "apply",
            0,
            object([
                ("transaction", result),
                ("post_receipt_id", string(&post)),
            ]),
        );
    }
    let tx = args
        .id
        .as_deref()
        .ok_or_else(|| fail("usage.invalid", "`--id <tx>` is required for rollback".into()))?;
    if !valid_tx_id(tx) {
        return Err(fail("store.bad_id", "`--id` must be a transaction id of the form tx_<16 hex>".into()));
    }
    let _guard = store.lock_mutation().map_err(|err| fail(err.code, err.message))?;
    let backup = ctxpect_projection::backup_dir(store.root(), tx);
    // The transaction record names the target; authorization is bound to it.
    let target_rel = read_tx(&backup)
        .ok()
        .and_then(|meta| meta.get("target_rel").and_then(Value::as_str).map(str::to_string))
        .or_else(|| args.target.clone())
        .unwrap_or_else(|| "AGENTS.md".into());
    let _auth = authorize_store_apply(&store, Some(project.as_path()), "rollback", &target_rel)?;
    let result = proj_rollback(
        &root,
        &backup,
        &target_rel,
        &project_digest(&store, Some(project.as_path())),
    )
    .map_err(|err| fail(err.code, err.message))?;
    // A rolled-back transaction stays consumed: re-applying needs a fresh preview.
    mark_preview(&store, tx, PREVIEW_ROLLED_BACK)?;
    let _ = store.audit("projection.rollback", "projection", Some(tx));
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
    let receipt = persist_inspect_in(store, &report.envelope, "one-shot", args.project.as_deref())?;
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
pub(crate) fn exception_state(
    store: &Store,
    project: Option<&Path>,
    id: &str,
) -> Result<Value, InspectFailure> {
    let record = store
        .get_named("exceptions", id)
        .map_err(|err| fail(err.code, err.message))?;
    let now = now_unix();
    let status = exception_status(&record, now, policy_fresh(store, project, now))
        .map_err(|err| fail(err.code, err.message))?;
    Ok(merge(
        status,
        [
            ("exception_id", string(id)),
            ("action", record.get("action").cloned().unwrap_or(Value::Null)),
            ("target", record.get("target").cloned().unwrap_or(Value::Null)),
            ("expires_at", record.get("expires_at").cloned().unwrap_or(Value::Null)),
        ],
    ))
}

fn standard_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    let sub = args.subcommand.as_deref().unwrap_or("status");
    let id = args.id.clone().unwrap_or_else(|| "std-local".into());
    match sub {
        "validate" | "publish" => {
            // Publishing a standard writes a signed team contract into the
            // store. That is a mutation and goes through the one authority.
            let _auth =
                authorize_store_apply(&store, args.project.as_deref(), &format!("standard.{sub}"), &id)?;
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
            let mac = hmac_sha256_hex(&key.secret, standard_mac_input_v2(&digest).as_bytes());
            let doc = object([
                ("standard_id", string(&id)),
                ("payload_digest", string(sha256_text(&payload))),
                ("signature_kind", string("local-continuity")),
                ("signature", object([
                    ("kind", string("local-continuity")),
                    ("algorithm", string(STANDARD_SIGNATURE_ALGORITHM_V2)),
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
            let _auth = authorize_store_apply(&store, args.project.as_deref(), "standard.adopt", &id)?;
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
            let _auth = authorize_store_apply(&store, args.project.as_deref(), "standard.pin", &id)?;
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
            let _auth = authorize_store_apply(&store, args.project.as_deref(), "standard.update", &id)?;
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
            let _auth =
                authorize_store_apply(&store, args.project.as_deref(), &format!("standard.{sub}"), &id)?;
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
            // An exception is for one action in one project, with an
            // explicit lifetime. None of these has a default.
            let action = args.action.as_deref().ok_or_else(|| {
                fail(
                    "usage.invalid",
                    "`exception request` requires `--action <mutation>` (e.g. apply, assets.copy, sessions.import)".into(),
                )
            })?;
            let expires_in = args.expires_in.ok_or_else(|| {
                fail(
                    "usage.invalid",
                    "`exception request` requires `--expires-in <seconds>`; exceptions have no default lifetime".into(),
                )
            })?;
            let target = args.target.as_deref().unwrap_or(ANY_TARGET);
            let scope = mutation_scope(&store, args.project.as_deref(), action, target);
            let now = now_unix();
            let rec = new_exception(
                &id,
                proof.principal_id(),
                &scope,
                now.saturating_add(expires_in),
                now,
                args.reason.as_deref(),
            );
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
            let status = exception_state(&store, args.project.as_deref(), &id)?;
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

/// Save the prior provenance record before any copy can change the target.
pub(crate) fn prepare_asset_lock(store: &Store, plan: &ctxpect_assets::CopyPlan) -> Result<(), InspectFailure> {
    // Validate the folder too: get_named reports a missing child when its
    // parent is a regular file, which is not an absent provenance record.
    store.list_named("assetlock").map_err(|err| fail(err.code, err.message))?;
    match store.get_named("assetlockbefore", &plan.tx_id) {
        Ok(_) => return Err(fail("assets.tx_exists", "copy transaction already has provenance history; use a new preview".into())),
        Err(err) if err.code == "store.missing" => {},
        Err(err) => return Err(fail(err.code, err.message)),
    }
    let previous = match store.get_named("assetlock", &plan.asset.asset_id) {
        Ok(value) => value,
        Err(err) if err.code == "store.missing" => Value::Null,
        Err(err) => return Err(fail(err.code, err.message)),
    };
    store.put_named("assetlockbefore", &plan.tx_id, &object([
        ("asset_id", string(&plan.asset.asset_id)),
        ("previous", previous),
    ])).map_err(|err| fail(err.code, err.message))
}

/// Rollback restores both the bytes and their recorded provenance. A later
/// copy owns a different lock entry even when it installed identical bytes.
pub(crate) fn rollback_asset_in(root: &Root, store: &Store, tx: &str) -> Result<Value, InspectFailure> {
    let history = store.get_named("assetlockbefore", tx).map_err(|err| {
        if err.code == "store.missing" {
            fail("assets.lock_history_missing", "copy transaction has no prior provenance record; inspect its backup before manual recovery".into())
        } else { fail(err.code, err.message) }
    })?;
    let asset_id = history.get("asset_id").and_then(Value::as_str)
        .ok_or_else(|| fail("assets.lock_history_invalid", "rollback history has no asset id".into()))?;
    let previous = history.get("previous")
        .ok_or_else(|| fail("assets.lock_history_invalid", "rollback history has no previous lock record".into()))?;
    match store.get_named("assetlock", asset_id) {
        Ok(current) if current.get("tx_id").and_then(Value::as_str) == Some(tx) || &current == previous => {},
        Ok(_) => return Err(fail("assets.lock_conflict", "a later copy owns this asset; roll it back first".into())),
        Err(err) if err.code == "store.missing" => {},
        Err(err) => return Err(fail(err.code, err.message)),
    }
    let result = ctxpect_assets::rollback(root, &ctxpect_assets::backup_dir(store.root(), tx))
        .map_err(|err| fail(err.code, err.message))?;
    let restored = if previous == &Value::Null {
        store.delete_named("assetlock", asset_id).map(|_| ())
    } else {
        store.put_named("assetlock", asset_id, previous)
    };
    Ok(match restored {
        Ok(()) => result,
        Err(err) => merge(result, [("asset_lock_error", object([
            ("code", string(err.code)), ("message", string(err.message)),
        ]))]),
    })
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
            let _auth = authorize_store_apply(&store, Some(project.as_path()), "assets.copy", asset_id)?;
            prepare_asset_lock(&store, &plan)?;
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
            if !valid_tx_id(tx) {
                return Err(fail("store.bad_id", "`--id` must be a transaction id of the form tx_<16 hex>".into()));
            }
            let root = Root::new(project).map_err(|err| fail("io.missing", err.to_string()))?;
            let store = open_store(args)?;
            let _auth = authorize_store_apply(&store, Some(project.as_path()), "assets.rollback", tx)?;
            let result = rollback_asset_in(&root, &store, tx)?;
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
        let id = out
            .get("candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("c_local");
        let _auth = authorize_store_apply(&store, args.project.as_deref(), "advisor.persist", id)?;
        store
            .put_named("advisor", id, &out)
            .map_err(|err| fail(err.code, err.message))?;
    }
    ok("advisor", 0, out)
}

/// `experiment [--runs <file>] [--id <experiment>] [--store <dir>]`.
///
/// Without `--runs` nothing was executed and the answer says so (exit 3, no
/// decision, nothing persisted). With a runs document the contract inside it
/// is judged; the result is persisted only under `experiment.persist`
/// authority, and the planned sample size of a persisted experiment cannot
/// change afterwards (`effect.n_locked`).
fn experiment_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    if args.execute || args.adapter.is_some() {
        if !args.execute || args.adapter.as_deref()!=Some("command-v1") || args.runs.is_some() {
            return Err(fail("effect.runner_usage", "runner requires --execute --adapter command-v1 --from request.json; --runs is an import path".into()));
        }
        let path=args.from.as_ref().ok_or_else(||fail("effect.runner_usage", "--from runner request required".into()))?;
        let request=crate::secure_sync::read_json(path).map_err(|_|fail("effect.runner_invalid","runner request unreadable".into()))?;
        let plan=crate::effect_runner::Plan::parse(request).map_err(|c|fail(c,"runner pre-registration refused".into()))?;
        if args.id.as_ref().is_some_and(|id|id!=&plan.contract.experiment_id) || args.n.is_some_and(|n|n!=plan.contract.n_planned) {
            return Err(fail("effect.contract_locked","CLI flags differ from frozen runner contract".into()));
        }
        let store=open_store(args)?;
        let _auth=authorize_store_apply(&store,args.project.as_deref(),"experiment.execute",&plan.contract.experiment_id)?;
        let _persist=authorize_store_apply(&store,args.project.as_deref(),"experiment.persist",&plan.contract.experiment_id)?;
        if store.list_named("experiments").map_err(|e|fail(e.code,e.message))?.contains(&plan.contract.experiment_id) {
            return Err(fail("effect.execution_exists","experiment id already has results; no rerun".into()));
        }
        let result=crate::effect_runner::execute(&plan,&store).map_err(|c|fail(c,"runner stopped; persisted job retains attempts and completed results".into()))?;
        return ok("experiment",ctxpect_effect::exit_code(&result),result);
    }
    let store = if let Some(path) = &args.store {
        Some(Store::open(path).map_err(|err| fail(err.code, err.message))?)
    } else {
        None
    };
    let Some(runs_path) = args.runs.as_ref() else {
        let experiment_id = args.id.clone().unwrap_or_else(|| "exp-local".into());
        let result = not_executed(&experiment_id, args.n);
        return ok("experiment", ctxpect_effect::exit_code(&result), result);
    };
    let text = fs::read_to_string(runs_path).map_err(|err| fail("io.missing", err.to_string()))?;
    let value = parse(&text).map_err(|err| fail("effect.runs_invalid", err.to_string()))?;
    let document = RunsDocument::from_value(&value).map_err(|err| fail(err.code, err.message))?;
    let experiment_id = args
        .id
        .clone()
        .unwrap_or_else(|| document.contract.experiment_id.clone());
    if experiment_id != document.contract.experiment_id {
        return Err(fail(
            "effect.runs_invalid",
            "`--id` does not name the experiment the runs document's contract was frozen for".into(),
        ));
    }
    if let Some(n) = args.n
        && n != document.contract.n_planned
    {
        return Err(fail(
            "effect.n_locked",
            "`--n` differs from the frozen contract; sample size is not a command-line choice".into(),
        ));
    }
    if let Some(store) = &store
        && let Ok(prev) = store.get_named("experiments", &experiment_id)
        && let Some(prev_contract) = prev.get("contract")
    {
        if prev_contract.get("n_planned").and_then(Value::as_i64) != Some(document.contract.n_planned) {
            return Err(fail(
                "effect.n_locked",
                "sample size cannot change after results are observed".into(),
            ));
        }
        // The whole pre-registered contract is frozen, not only its sample
        // size: a different margin, alpha, ITT rule or locked confounder is
        // a different experiment and gets a different id.
        if canonical_json(prev_contract) != canonical_json(&document.contract.to_value()) {
            return Err(fail(
                "effect.contract_locked",
                "the frozen contract differs from the one already persisted for this experiment id; a changed protocol is a new experiment".into(),
            ));
        }
    }
    let result = effect_decide(&document);
    if let Some(store) = &store {
        let _auth =
            authorize_store_apply(store, args.project.as_deref(), "experiment.persist", &experiment_id)?;
        store
            .put_named("experiments", &experiment_id, &result)
            .map_err(|err| fail(err.code, err.message))?;
    }
    ok("experiment", ctxpect_effect::exit_code(&result), result)
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
    let (action, target) = policy_query_scope(args.action.as_deref(), args.target.as_deref());
    let result = effective_store_policy(&store, args.project.as_deref(), &action, &target);
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

/// `align status` lists the Receipts available for alignment; `align diff`
/// compares two of them and requires both ids.
fn align_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let store = open_store(args)?;
    match args.subcommand.as_deref().unwrap_or("status") {
        "diff" => {
            if args.left.is_none() || args.right.is_none() {
                return Err(fail(
                    "usage.invalid",
                    "`align diff` requires `--a <receipt-id>` and `--b <receipt-id>`".into(),
                ));
            }
            let report = diff_cmd(args)?;
            ok(
                "align diff",
                report.exit_code,
                merge(report.envelope, [("command", string("align diff"))]),
            )
        }
        _ => {
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
    }
}

/// `adapter test --adapter <family> [--from <repo-root>]` runs that family's
/// development static corpus through the same runner as the
/// `corpus-conformance` gate and binds the result to the corpus, the
/// compatibility matrix and the resolver version. `implemented_rows`
/// counts rows the resolver actually answered (implemented pass or fail);
/// an unknown-honesty pass is reported but earns no credit towards
/// "implemented", so a family whose only passes are honesty cells is still
/// `unimplemented` (exit 3). No live oracle is executed here.
fn adapter_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    match args.subcommand.as_deref() {
        Some("list") => ok("adapter list", 0, integrations_json(&args.harness, true)),
        Some("test") => {
            let Some(family) = args.adapter.as_deref() else {
                return Err(fail(
                    "usage.invalid",
                    "`adapter test` requires `--adapter <family>` (one of the 18 catalog families)".into(),
                ));
            };
            if !FAMILIES.iter().any(|item| item.id == family) {
                return Err(fail("usage.invalid", "`--adapter` must name a catalog family".into()));
            }
            let root = args
                .from
                .clone()
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let report = crate::conformance::run_static_corpus(&root, Some(family))
                .map_err(|err| fail(err.code, err.message))?;
            if report.total == 0 {
                return Err(fail(
                    "adapter.corpus_missing",
                    "no development static corpus rows for this family under `--from` (default: current directory)".into(),
                ));
            }
            let totals = report.totals();
            let implemented_rows = totals.implemented_pass + totals.fail;
            let (decision, exit) = if implemented_rows == 0 {
                ("unimplemented", 3)
            } else if totals.fail > 0 {
                ("fail", 2)
            } else {
                ("pass", 0)
            };
            let body = merge(
                report.to_value(),
                [
                    ("ran", Value::Bool(true)),
                    ("family_id", string(family)),
                    ("executed_rows", Value::Int(i64::try_from(report.total).unwrap_or(0))),
                    ("implemented_rows", Value::Int(i64::try_from(implemented_rows).unwrap_or(0))),
                    ("decision", string(decision)),
                    (
                        "reason_code",
                        string(match decision {
                            "unimplemented" => "adapter.family_unimplemented",
                            "fail" => "adapter.rows_failed",
                            _ => "ok",
                        }),
                    ),
                ],
            );
            ok("adapter test", exit, body)
        }
        _ => Err(fail("usage.invalid", "adapter requires test|list".into())),
    }
}

/// The store `ci` reads policy from, if one exists. `ci` never creates one:
/// a CI run must not leave a `.ctxpect/store` behind in the project.
fn existing_store(args: &ProductArgs) -> Result<Option<Store>, InspectFailure> {
    if let Some(path) = &args.store {
        return Store::open(path).map(Some).map_err(|err| fail(err.code, err.message));
    }
    let Some(project) = &args.project else {
        return Ok(None);
    };
    let default = project.join(".ctxpect/store");
    if default.is_dir() {
        return Store::open(&default).map(Some).map_err(|err| fail(err.code, err.message));
    }
    Ok(None)
}

/// The policy verdict `ci` gates on. No policy is Unknown, not pass.
fn ci_policy_verdict(store: Option<&Store>, project: Option<&Path>) -> Value {
    let now = now_unix();
    let layers = match store {
        Some(store) => load_policy_layers(store, project),
        None => project.and_then(|dir| {
            fs::read_to_string(dir.join(".ctxpect/policy.json"))
                .ok()
                .and_then(|text| parse(&text).ok())
        }),
    };
    let Some(layers) = layers else {
        return object([
            ("verdict", string("unknown")),
            ("reason_code", string("policy.unknown")),
            ("exit_code", Value::Int(3)),
            ("layers_present", Value::Bool(false)),
        ]);
    };
    match evaluate(&layers, now) {
        Ok(evaluation) => {
            let verdict = evaluation
                .get("verdict")
                .and_then(Value::as_str)
                .unwrap_or("indeterminate")
                .to_string();
            let (exit, reason) = match verdict.as_str() {
                "pass" => (0, "ok"),
                "deny" => (2, "policy.denied"),
                "detect-only" => (3, "policy.detect_only_not_enforceable"),
                _ => (3, "policy.indeterminate"),
            };
            object([
                ("verdict", string(&verdict)),
                ("reason_code", string(reason)),
                ("exit_code", Value::Int(exit)),
                ("layers_present", Value::Bool(true)),
                ("evaluation", evaluation),
            ])
        }
        Err(err) => object([
            ("verdict", string("indeterminate")),
            ("reason_code", string(err.code)),
            ("exit_code", Value::Int(3)),
            ("layers_present", Value::Bool(true)),
            ("message", string(err.message)),
        ]),
    }
}

/// Combine independent verdict exits: any indeterminate → 3, else any
/// deterministic failure → 2, else 0. Same rule as the inspect exit.
fn combine_exits(codes: &[i32]) -> i32 {
    if codes.contains(&3) {
        3
    } else if codes.contains(&2) {
        2
    } else {
        0
    }
}

fn inspect_args_project(args: &ProductArgs) -> Result<std::path::PathBuf, InspectFailure> {
    args.project
        .clone()
        .ok_or_else(|| fail("usage.invalid", "`--project` is required".into()))
}

fn ci_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    let inspect_args = inspect_args(args)?;
    let report = inspect(inspect_args)?;
    let diagnosis = full_diagnosis(&report.envelope, &inspect_args_project(args)?, args.as_of)?;
    let doctor_exit = ctxpect_doctor::blocking_exit(&diagnosis, args.fail_on.as_deref());
    let store = existing_store(args)?;
    let policy = ci_policy_verdict(store.as_ref(), args.project.as_deref());
    let policy_exit = i32::try_from(policy.get("exit_code").and_then(Value::as_i64).unwrap_or(3))
        .unwrap_or(3);
    let exit = combine_exits(&[report.exit_code, doctor_exit, policy_exit]);
    // With a store, the observation this verdict rests on is persisted as a
    // `ci` kind Receipt so the verdict can be checked later. `ci` still
    // creates no store.
    let receipt_id = match store.as_ref() {
        Some(store) => Some(
            persist_inspect_in(store, &report.envelope, "ci", args.project.as_deref())?
                .get("receipt_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        ),
        None => None,
    };
    ok(
        "ci",
        exit,
        object([
            ("receipt_id", receipt_id.as_deref().map_or(Value::Null, string)),
            ("receipt_kind", string("ci")),
            ("receipt_persisted", Value::Bool(receipt_id.is_some())),
            ("inspect_exit_code", Value::Int(i64::from(report.exit_code))),
            ("doctor_exit_code", Value::Int(i64::from(doctor_exit))),
            ("policy_exit_code", Value::Int(i64::from(policy_exit))),
            ("policy", policy),
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


fn secure_sync_cmd(args: &ProductArgs) -> Result<ProductReport, InspectFailure> {
    use crate::secure_sync::{self, Profile};
    let error = |code| {
        fail(
            code,
            "encrypted sync refused; local profile and signed chain must match".into(),
        )
    };
    let store = open_store(args)?;
    let profile_path = args
        .profile
        .as_deref()
        .ok_or_else(|| error("sync.profile_required"))?;
    let profile = Profile::load(Path::new(profile_path)).map_err(error)?;
    let _guard = store.lock_mutation().map_err(|e| fail(e.code, e.message))?;
    let state_path = store
        .root()
        .join("secureheads")
        .join(format!("{}.json", profile.group));
    let state = if state_path
        .try_exists()
        .map_err(|_| error("sync.state_unreadable"))?
    {
        Some(
            store
                .get_named("secureheads", &profile.group)
                .map_err(|e| fail(e.code, e.message))?,
        )
    } else {
        None
    };
    let head = state
        .as_ref()
        .and_then(|s| s.pointer(&["document", "envelope"]));
    if args.subcommand.as_deref() == Some("status") {
        return ok(
            "sync status",
            0,
            object([
                ("adapter", string("age-ssh-v1")),
                ("group", string(&profile.group)),
                (
                    "head",
                    head.map(|h| string(sha256_text(&canonical_json(h))))
                        .unwrap_or(Value::Null),
                ),
                ("semantic", string("structural-only")),
                ("runtime_verification", string("not-observed")),
            ]),
        );
    }
    let input_path = args
        .from
        .as_ref()
        .ok_or_else(|| error("sync.input_required"))?;
    let input = secure_sync::read_json(input_path).map_err(error)?;
    if args.subcommand.as_deref() == Some("seal") {
        let _auth =
            authorize_store_apply(&store, args.project.as_deref(), "sync.seal", &profile.group)?;
        let dest = args
            .dest
            .as_ref()
            .ok_or_else(|| error("sync.destination_required"))?;
        let document = secure_sync::seal(&profile, &input, head).map_err(error)?;
        crate::tool_process::private_write(dest, canonical_json(&document).as_bytes())
            .map_err(|_| error("sync.export_failed"))?;
        return ok(
            "sync seal",
            0,
            object([
                ("transport", string("encrypted-export")),
                ("semantic", string("not-verified")),
                (
                    "envelope_digest",
                    string(sha256_text(&canonical_json(&document))),
                ),
                ("sender_head_advanced", Value::Bool(false)),
            ]),
        );
    }
    if !matches!(args.subcommand.as_deref(), Some("preview" | "apply")) {
        return Err(error("usage.invalid"));
    }
    let _auth = if args.subcommand.as_deref() == Some("apply") {
        Some(authorize_store_apply(
            &store,
            args.project.as_deref(),
            "sync.apply",
            &profile.group,
        )?)
    } else {
        None
    };
    let payload = secure_sync::open(&profile, &input).map_err(error)?;
    secure_sync::check_chain(&input, head).map_err(error)?;
    let state_digest = sha256_text(&canonical_json(&state.clone().unwrap_or(Value::Null)));
    let envelope_digest = sha256_text(&canonical_json(&input));
    if args.subcommand.as_deref() == Some("preview") {
        let tx = format!(
            "sync_{}",
            &sha256_text(&format!(
                "{}:{}:{}",
                envelope_digest,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
                std::process::id()
            ))[..24]
        );
        let preview = object([
            ("profile_digest", string(&profile.digest)),
            ("state_digest", string(state_digest)),
            ("envelope_digest", string(envelope_digest)),
            ("expires_at", Value::Int(now_unix() + 900)),
            ("consumed", Value::Bool(false)),
        ]);
        store
            .put_named("securepreviews", &tx, &preview)
            .map_err(|e| fail(e.code, e.message))?;
        let mut result = secure_sync::preview_summary(&payload, &input);
        if let Value::Object(m) = &mut result {
            m.insert("tx_id".into(), string(tx));
        }
        return ok("sync preview", 0, result);
    }
    let tx = args
        .tx
        .as_deref()
        .ok_or_else(|| error("sync.preview_required"))?;
    let mut preview = store
        .get_named("securepreviews", tx)
        .map_err(|_| error("sync.preview_required"))?;
    if preview.get("profile_digest").and_then(Value::as_str) != Some(&profile.digest)
        || preview.get("state_digest").and_then(Value::as_str) != Some(&state_digest)
        || preview.get("envelope_digest").and_then(Value::as_str) != Some(&envelope_digest)
        || preview
            .get("expires_at")
            .and_then(Value::as_i64)
            .is_none_or(|t| t <= now_unix())
        || preview.get("consumed").and_then(Value::as_bool) != Some(false)
    {
        return Err(error("sync.preview_changed"));
    }
    if let Value::Object(m) = &mut preview {
        m.insert("consumed".into(), Value::Bool(true));
    }
    store
        .put_named("securepreviews", tx, &preview)
        .map_err(|e| fail(e.code, e.message))?;
    // A single atomic local record advances the encrypted head and metadata.
    // Plaintext remains out of the store; native projection is a separate step.
    if let Some(previous) = &state {
        store
            .put_named("securehistory", &state_digest, previous)
            .map_err(|e| fail(e.code, e.message))?;
    }
    store
        .put_named(
            "secureheads",
            &profile.group,
            &object([
                ("document", input.clone()),
                (
                    "asset_metadata",
                    secure_sync::preview_summary(&payload, &input),
                ),
                ("previous_state_digest", string(state_digest)),
            ]),
        )
        .map_err(|e| fail(e.code, e.message))?;
    let mut result = secure_sync::preview_summary(&payload, &input);
    if let Value::Object(m) = &mut result {
        m.insert("transport".into(), string("applied-encrypted-local-head"));
        m.insert("native_projection".into(), string("not-applied"));
    }
    ok("sync apply", 0, result)
}

fn stop_owned_daemon(store: &Store) -> Result<bool, InspectFailure> {
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};
    let control =
        crate::secure_sync::read_json(&store.root().join("daemon.control")).map_err(|_| {
            fail(
                "daemon.control_missing",
                "no readable local daemon control record".into(),
            )
        })?;
    let address = control
        .get("address")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<std::net::SocketAddr>().ok())
        .filter(|a| a.ip().is_loopback())
        .ok_or_else(|| {
            fail(
                "daemon.control_invalid",
                "daemon address must be loopback".into(),
            )
        })?;
    let token = control
        .get("token")
        .and_then(Value::as_str)
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| {
            fail(
                "daemon.control_invalid",
                "invalid daemon control record".into(),
            )
        })?;
    let mut stream = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|_| {
            fail(
                "daemon.unreachable",
                "daemon control endpoint is unreachable".into(),
            )
        })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|_| fail("daemon.io", "cannot set read timeout".into()))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|_| fail("daemon.io", "cannot set write timeout".into()))?;
    let body = canonical_json(&object([("token", string(token))]));
    let request = format!(
        "POST /api/v1/shutdown HTTP/1.1\r\nHost: {address}\r\nX-Ctxpect-Client: desktop\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|_| fail("daemon.io", "shutdown request failed".into()))?;
    let mut response = String::new();
    stream
        .take(65536)
        .read_to_string(&mut response)
        .map_err(|_| fail("daemon.io", "shutdown response unavailable".into()))?;
    if !response.starts_with("HTTP/1.1 200 ") {
        return Err(fail(
            "daemon.control_refused",
            "daemon rejected shutdown".into(),
        ));
    }
    let acknowledged = response
        .split_once("\r\n\r\n")
        .and_then(|(_, body)| parse(body).ok())
        .is_some_and(|body| {
            body.get("shutdown_accepted").and_then(Value::as_bool) == Some(true)
                && body.get("control_id").and_then(Value::as_str) == Some(&sha256_text(token))
        });
    if !acknowledged {
        return Err(fail(
            "daemon.control_refused",
            "shutdown acknowledgement did not match this daemon".into(),
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(store.root().join("daemon.lock"))
            .map_err(|_| {
                fail(
                    "daemon.lock_unavailable",
                    "cannot verify daemon exit".into(),
                )
            })?;
        if lock.try_lock().is_ok() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
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
