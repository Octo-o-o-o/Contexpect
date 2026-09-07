//! Localhost HTTP API. UI does not scan the disk.

use crate::args::{InspectArgs, ProductArgs};
use crate::catalog::{family_entry, integrations_json};
use crate::dispatch::{
    asset_lock, assets_status, authorize_store_apply, effective_store_policy, listen_addr,
    load_asset_registry, now_unix, persist_inspect, verify_standard_document, ProductReport,
};
use crate::inspect::inspect;
use crate::jsonutil::with_snapshot_digest;
use ctxpect_advisor::suggest;
use ctxpect_collect::scan;
use ctxpect_diff::{diff, EquivalenceProfile};
use ctxpect_doctor::diagnose;
use ctxpect_effect::{decide, run_local_instructions_probe, ExperimentContract};
use ctxpect_fs::{Refusal, Root};
use ctxpect_importer::import_session;
use ctxpect_policy::exception_status;
use ctxpect_projection::{apply as proj_apply, preview as proj_preview, rollback as proj_rollback, Intent};
use ctxpect_schema::{array, canonical_json, object, parse, string, Value};
use ctxpect_store::Store;
use ctxpect_sync::{apply_folder, bundle, preview_apply};

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

struct AppState {
    store: Store,
    project: Option<PathBuf>,
    ui_root: Option<PathBuf>,
    generation: AtomicU64,
    current_receipt: Mutex<Option<String>>,
}

pub fn serve(args: &ProductArgs) -> Result<ProductReport, crate::inspect::InspectFailure> {
    let addr = listen_addr(args);
    if !addr.starts_with("127.0.0.1:") && !addr.starts_with("localhost:") {
        return Err(crate::inspect::InspectFailure::Io {
            command: Some("daemon".into()),
            code: "api.bind_refused",
            message: "daemon listens on 127.0.0.1 only".into(),
        });
    }
    let store = {
        let path = args
            .store
            .clone()
            .or_else(|| args.project.as_ref().map(|p| p.join(".ctxpect/store")))
            .ok_or_else(|| crate::inspect::InspectFailure::Io {
            command: Some("daemon".into()),
                code: "usage.invalid",
                message: "`--store` or `--project` is required".into(),
            })?;
        Store::open(&path).map_err(|err| crate::inspect::InspectFailure::Io {
            command: Some("daemon".into()),
            code: err.code,
            message: err.message,
        })?
    };
    let _ = fs::write(store.root().join("daemon.pid"), format!("{}", std::process::id()));
    let listener = TcpListener::bind(&addr).map_err(|err| crate::inspect::InspectFailure::Io {
            command: Some("daemon".into()),
        code: "api.bind",
        message: err.to_string(),
    })?;
    let bound = listener
        .local_addr()
        .map_err(|err| crate::inspect::InspectFailure::Io {
            command: Some("daemon".into()),
            code: "api.bind",
            message: err.to_string(),
        })?;
    let _ = fs::write(store.root().join("daemon.addr"), bound.to_string());
    let ui_root = args.ui_root.clone().or_else(|| {
        let p = PathBuf::from("packages/ui/dist");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    });
    let state = Arc::new(AppState {
        store,
        project: args.project.clone(),
        ui_root,
        generation: AtomicU64::new(1),
        current_receipt: Mutex::new(None),
    });
    if args.oneshot {
        // Serve until a single /api/v1/shutdown or process signal. For tests,
        // callers should use a child process. Here we accept connections until
        // the parent stops us.
    }
    eprintln!("ctxpect daemon listening on http://{bound} (localhost API; UI does not scan disk)");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                let _ = handle_client(stream, &state);
            }
            Err(_) => continue,
        }
    }
    Ok(ProductReport {
        exit_code: 0,
        envelope: object([("stopped", Value::Bool(true))]),
    })
}

fn handle_client(mut stream: TcpStream, state: &AppState) -> Result<(), ()> {
    let mut buf = [0u8; 65536];
    let n = stream.read(&mut buf).map_err(|_| ())?;
    let raw = String::from_utf8_lossy(&buf[..n]);
    let Some(header_end) = raw.find("\r\n\r\n") else {
        return write_res(&mut stream, 400, "application/json", r#"{"error":{"code":"api.bad_request"}}"#);
    };
    let head = &raw[..header_end];
    let body = &raw[header_end + 4..];
    let mut lines = head.lines();
    let req = lines.next().unwrap_or("");
    let mut parts = req.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let host = headers.get("host").cloned().unwrap_or_default();
    if !host.starts_with("127.0.0.1") && !host.starts_with("localhost") && !host.is_empty() {
        return write_res(&mut stream, 403, "application/json", r#"{"error":{"code":"api.host"}}"#);
    }
    let origin = headers.get("origin").cloned().unwrap_or_default();
    if !origin.is_empty()
        && !origin.starts_with("http://127.0.0.1")
        && !origin.starts_with("http://localhost")
    {
        return write_res(&mut stream, 403, "application/json", r#"{"error":{"code":"api.origin"}}"#);
    }
    if method != "GET" && method != "HEAD" {
        let client = headers.get("x-ctxpect-client").map(String::as_str).unwrap_or("");
        if client != "desktop" && origin.is_empty() {
            return write_res(
                &mut stream,
                403,
                "application/json",
                r#"{"error":{"code":"api.csrf"}}"#,
            );
        }
    }
    let (status, ctype, body_out) = route(method, path, body, state);
    write_res(&mut stream, status, ctype, &body_out)
}

fn write_res(stream: &mut TcpStream, status: u16, ctype: &str, body: &str) -> Result<(), ()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: http://127.0.0.1\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).map_err(|_| ())?;
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Write);
    Ok(())
}

fn json_ok(value: Value) -> (u16, &'static str, String) {
    (200, "application/json", canonical_json(&with_snapshot_digest(value)))
}

fn json_err(code: &str, message: &str) -> (u16, &'static str, String) {
    (
        400,
        "application/json",
        canonical_json(&object([
            (
                "error",
                object([("code", string(code)), ("message", string(message))]),
            ),
        ])),
    )
}

/// Parse a request body that is expected to carry named parameters.
///
/// An empty body means "no parameters" and is fine. Anything else must be a
/// JSON **object**: a body that parses as an array or a scalar carries none
/// of the fields the handler reads, so accepting it would run the request
/// with silent defaults and report success for parameters the caller never
/// actually sent.
fn object_body(body: &str) -> Result<Value, (u16, &'static str, String)> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(object::<String>([]));
    }
    match parse(trimmed) {
        Ok(value @ Value::Object(_)) => Ok(value),
        Ok(_) => Err(json_err(
            "api.body_not_object",
            "request body must be a JSON object",
        )),
        Err(err) => Err(json_err("api.parse", &err.to_string())),
    }
}

fn require_mutation(state: &AppState) -> Result<Value, (u16, &'static str, String)> {
    match authorize_store_apply(&state.store, state.project.as_deref()) {
        Ok(auth) => Ok(auth),
        Err(err) => Err(json_err(err.code(), &err.message())),
    }
}

fn route(method: &str, path: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    let path_only = path.split('?').next().unwrap_or(path);
    if method == "OPTIONS" {
        return (200, "text/plain", String::new());
    }
    if path_only.starts_with("/api/v1/") {
        return api(method, path_only, path, body, state);
    }
    static_file(path_only, state)
}

fn api(method: &str, path: &str, full: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    match (method, path) {
        ("GET", "/api/v1/health") => json_ok(object([
            ("ok", Value::Bool(true)),
            ("generation", Value::Int(i64::try_from(state.generation.load(Ordering::SeqCst)).unwrap_or(0))),
        ])),
        ("GET", "/api/v1/coordinate") => json_ok(object([
            (
                "project",
                string(
                    state
                        .project
                        .as_ref()
                        .map(|_p| "<project>")
                        .unwrap_or("unset"),
                ),
            ),
            ("ui_computes_claims", Value::Bool(false)),
        ])),
        ("POST", "/api/v1/inspect") => inspect_api(body, state),
        ("GET", "/api/v1/receipts") => match state.store.list_receipts() {
            Ok(v) => json_ok(object([("receipts", v)])),
            Err(err) => json_err(err.code, &err.message),
        },
        (m, p) if p.starts_with("/api/v1/receipts/") => receipt_api(m, p, body, state),
        ("GET", "/api/v1/doctor") => doctor_api(full, state),
        ("GET", "/api/v1/diff") => diff_api(full, state),
        ("GET", "/api/v1/integrations") => json_ok(integrations_json("codex", true)),
        ("GET", p) if p.starts_with("/api/v1/assets/") => match strip_id(p, "/api/v1/assets/").and_then(|id| family_entry(id, "codex", true)) {
            Some(v) => json_ok(v),
            None => json_err("catalog.unknown", p),
        },
        ("GET", "/api/v1/assets") => {
            json_ok(assets_status("codex", Some(&asset_lock(&state.store))))
        }
        ("POST", p) if p.starts_with("/api/v1/assets/") => assets_api(p, state),
        // Published so the UI renders its editor from the definition the
        // store enforces, instead of a hardcoded copy that can drift.
        ("GET", "/api/v1/settings/schema") => json_ok(ctxpect_store::settings_schema()),
        ("GET", "/api/v1/settings") => match state.store.settings() {
            Ok(v) => json_ok(v),
            Err(err) => json_err(err.code, &err.message),
        },
        ("PUT", "/api/v1/settings") | ("POST", "/api/v1/settings") => {
            if let Err(denied) = require_mutation(state) {
                return denied;
            }
            match parse(body) {
                Ok(v) => {
                    if let Err(err) = state.store.put_settings(v.clone()) {
                        return json_err(err.code, &err.message);
                    }
                    json_ok(v)
                }
                Err(err) => json_err("api.parse", &err.to_string()),
            }
        }
        ("GET", p) if p.starts_with("/api/v1/sessions/") => match strip_id(p, "/api/v1/sessions/") {
            Some(id) => named_get(state, "sessions", id),
            None => json_err("api.not_found", p),
        },
        ("GET", "/api/v1/sessions") => match state.store.list_named("sessions") {
            Ok(ids) => json_ok(object([("sessions", array(ids.into_iter().map(string)))])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", "/api/v1/sessions/import") => {
            if let Err(denied) = require_mutation(state) {
                return denied;
            }
            match parse(body) {
                Ok(_) => match import_session(body, "generic-json", "s_api") {
                    Ok(session) => {
                        let id = session
                            .get("session_id")
                            .and_then(Value::as_str)
                            .unwrap_or("s_api");
                        if let Err(err) = state.store.put_named("sessions", id, &session) {
                            return json_err(err.code, &err.message);
                        }
                        json_ok(session)
                    }
                    Err(err) => json_err(err.code, &err.message),
                },
                Err(err) => json_err("import_parse_failed", &err.to_string()),
            }
        }
        ("GET", "/api/v1/monitor") => json_ok(monitor_status(state)),
        ("GET", "/api/v1/policy") => {
            json_ok(effective_store_policy(&state.store, state.project.as_deref()))
        }
        ("GET", "/api/v1/exceptions") => match state.store.list_named("exceptions") {
            Ok(ids) => json_ok(object([("exceptions", array(ids.into_iter().map(string)))])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", "/api/v1/exceptions") => {
            // The exception lifecycle is gated on a verified enrolled
            // principal, and that needs the enrolled secret. The secret must
            // not travel in an HTTP request, so this endpoint cannot
            // establish identity and says so instead of minting an
            // exception whose requester is the literal string "user".
            json_err(
                "api.identity_required",
                "creating an exception requires a verified enrolled principal;                  use `ctxpect exception request --principal <id>` with                  $CTXPECT_PRINCIPAL_SECRET, which the daemon API cannot carry",
            )
        }
        ("GET", p) if p.starts_with("/api/v1/standards/") => match strip_id(p, "/api/v1/standards/") {
            Some(id) => standard_get(state, id),
            None => json_err("api.not_found", p),
        },
        ("GET", "/api/v1/standards") => match state.store.list_named("standards") {
            Ok(ids) => json_ok(object([("standards", array(ids.into_iter().map(string)))])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("GET", "/api/v1/sync") => json_ok(sync_status(state)),
        ("POST", "/api/v1/sync/preview") => sync_api(body, state, false),
        ("POST", "/api/v1/sync/apply") => sync_api(body, state, true),
        ("POST", "/api/v1/advisor") => advisor_api(body),
        ("GET", p) if p.starts_with("/api/v1/lab/") => match strip_id(p, "/api/v1/lab/") {
            Some(id) => named_get(state, "experiments", id),
            None => json_err("api.not_found", p),
        },
        ("GET", "/api/v1/lab") => match state.store.list_named("experiments") {
            Ok(ids) => json_ok(object([
                ("experiments", array(ids.into_iter().map(string))),
                ("single_ab_is_causal", Value::Bool(false)),
            ])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", "/api/v1/lab") => lab_api(body, state),
        ("GET", "/api/v1/team/compliance") => json_ok(team_compliance(state)),
        (m, p) if p.starts_with("/api/v1/care-plan/") => care_plan_api(m, p, body, state),
        ("GET", p) if p.starts_with("/api/v1/integrations/") => match strip_id(p, "/api/v1/integrations/").and_then(|id| family_entry(id, "codex", true)) {
            Some(v) => json_ok(v),
            None => json_err("catalog.unknown", p),
        },
        ("POST", "/api/v1/collect") => collect_api(state),
        ("POST", "/api/v1/apply") => apply_api(body, state),
        ("POST", "/api/v1/rollback") => rollback_api(body, state),
        _ => json_err("api.not_found", path),
    }
}

fn inspect_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let project = parsed
        .get("project")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| state.project.clone());
    let Some(project) = project else {
        return json_err("usage.invalid", "--project required");
    };
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let args = InspectArgs {
        json: true,
        offline: true,
        project,
        cwd: None,
        harness: parsed
            .get("harness")
            .and_then(Value::as_str)
            .unwrap_or("codex")
            .to_string(),
        surface: parsed
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or("cli")
            .to_string(),
        version: parsed
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("0.147.0")
            .to_string(),
        version_explicit: parsed.get("version").is_some(),
        codex_home: parsed
            .get("codex_home")
            .and_then(Value::as_str)
            .map(PathBuf::from),
        require: vec!["instructions".into()],
        os_lane: parsed
            .get("os_lane")
            .and_then(Value::as_str)
            .unwrap_or("macos-27-arm64")
            .to_string(),
        store: None,
    };
    match inspect(args) {
        Ok(report) => {
            let latest = state.generation.load(Ordering::SeqCst);
            let stale = latest != generation;
            match persist_inspect(&state.store, &report.envelope, "one-shot") {
                Ok(receipt) => {
                    let id = receipt
                        .get("receipt_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if !stale && let Ok(mut cur) = state.current_receipt.lock() {
                        *cur = Some(id.clone());
                    }
                    let symptom = parsed
                        .get("symptom")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    json_ok(object([
                        ("stale", Value::Bool(stale)),
                        ("receipt", receipt),
                        ("snapshot_schema", string("dev-inspect-v0")),
                        ("inspect_exit_code", Value::Int(i64::from(report.exit_code))),
                        ("symptom", string(symptom)),
                    ]))
                }
                Err(err) => json_err(err.code(), &err.message()),
            }
        }
        Err(err) => json_err(err.code(), &err.message()),
    }
}

fn receipt_api(method: &str, path: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    let rest = path.trim_start_matches("/api/v1/receipts/");
    let mut segs = rest.split('/');
    let id = segs.next().unwrap_or("");
    let action = segs.next();
    match (method, action) {
        ("GET", None) => match state.store.get_receipt(id) {
            Ok(v) => json_ok(v),
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", Some("delete")) => {
            if let Err(denied) = require_mutation(state) {
                return denied;
            }
            match state.store.delete_receipt(id, "api") {
                Ok(v) => json_ok(v),
                Err(err) => json_err(err.code, &err.message),
            }
        }
        ("POST", Some("verify")) => match state.store.get_receipt(id) {
            Ok(v) => match state.store.continuity_key() {
                Ok(key) => match ctxpect_receipt::verify_local_continuity(&v, &key) {
                    Ok(()) => json_ok(object([("ok", Value::Bool(true)), ("org_identity", Value::Bool(false))])),
                    Err(err) => json_err(err.code, &err.message),
                },
                Err(err) => json_err(err.code, &err.message),
            },
            Err(err) => json_err(err.code, &err.message),
        },
        _ => {
            let _ = body;
            json_err("api.not_found", path)
        }
    }
}

fn doctor_api(full: &str, state: &AppState) -> (u16, &'static str, String) {
    let id = query(full, "receipt_id");
    let receipt = if let Some(id) = id {
        state.store.get_receipt(&id)
    } else if let Ok(cur) = state.current_receipt.lock() {
        match cur.as_deref() {
            Some(id) => state.store.get_receipt(id),
            None => Err(ctxpect_store::StoreError {
                code: "store.missing",
                message: "no current receipt".into(),
            }),
        }
    } else {
        Err(ctxpect_store::StoreError {
            code: "store.missing",
            message: "no current receipt".into(),
        })
    };
    match receipt {
        Ok(v) => {
            let mut diagnosis = diagnose(&v);
            if let Some(symptom) = query(full, "symptom")
                && let Value::Object(map) = &mut diagnosis
            {
                map.insert("symptom".into(), string(url_decode(&symptom)));
            }
            json_ok(diagnosis)
        }
        Err(err) => json_err(err.code, &err.message),
    }
}

fn diff_api(full: &str, state: &AppState) -> (u16, &'static str, String) {
    let a = query(full, "a");
    let b = query(full, "b");
    let (Some(a), Some(b)) = (a, b) else {
        return json_err("usage.invalid", "a and b required");
    };
    let left = match state.store.get_receipt(&a) {
        Ok(v) => v,
        Err(err) => return json_err(err.code, &err.message),
    };
    let right = match state.store.get_receipt(&b) {
        Ok(v) => v,
        Err(err) => return json_err(err.code, &err.message),
    };
    json_ok(diff(&left, &right, EquivalenceProfile::Strict))
}

fn advisor_api(body: &str) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let consent = parsed.get("consent").and_then(Value::as_bool).unwrap_or(false);
    let ack = parsed
        .get("preview_ack")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match suggest(&["e1"], "redacted", consent, ack, "none") {
        Ok(v) => json_ok(v),
        Err(err) => json_err(err.code, &err.message),
    }
}

fn lab_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    // Running an experiment writes its result into the store.
    if let Err(denied) = require_mutation(state) {
        return denied;
    }
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let n = parsed.get("n").and_then(Value::as_i64).unwrap_or(4);
    let experiment_id = parsed
        .get("experiment_id")
        .and_then(Value::as_str)
        .unwrap_or("exp-api")
        .to_string();
    let changed = parsed
        .get("n_changed_after_results")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Ok(prev) = state.store.get_named("experiments", &experiment_id) {
        let prev_n = prev
            .pointer(&["contract", "n_planned"])
            .and_then(Value::as_i64)
            .or_else(|| prev.get("n_planned").and_then(Value::as_i64));
        if let Some(prev_n) = prev_n
            && prev_n != n
        {
            return json_err(
                "effect.n_locked",
                "sample size cannot change after results are observed",
            );
        }
    }
    let contract = ExperimentContract {
        experiment_id: experiment_id.clone(),
        n_planned: n,
        margin: 1,
        metric: "instructions-present".into(),
    };
    let control: Vec<i64> = (0..n).map(|_| run_local_instructions_probe(false)).collect();
    let treatment: Vec<i64> = (0..n).map(|_| run_local_instructions_probe(true)).collect();
    match decide(&contract, &control, &treatment, changed) {
        Ok(v) => {
            if let Err(err) = state.store.put_named("experiments", &experiment_id, &v) {
                // A result the store refused to keep must not be returned as
                // if it had been recorded.
                return json_err(err.code, &err.message);
            }
            json_ok(v)
        }
        Err(err) => json_err(err.code, &err.message),
    }
}

/// Asset preview, copy and rollback over the API.
///
/// The asset id names a registry entry, never a path: the registry decides
/// both where bytes come from and where they land, so nothing here lets a
/// request choose a filesystem location.
// The body is unused: the asset id in the path names a registry entry, and
// the registry — not the request — decides source and destination.
fn assets_api(path: &str, state: &AppState) -> (u16, &'static str, String) {
    let rest = path.trim_start_matches("/api/v1/assets/");
    let mut segs = rest.split('/');
    let id = segs.next().unwrap_or("");
    let action = segs.next().unwrap_or("");
    let Some(project) = state.project.clone() else {
        return json_err(
            "api.project_required",
            "asset operations need a declared project root",
        );
    };
    let Ok(root) = Root::new(&project) else {
        return json_err("io.missing", "project root is unreadable");
    };

    if action == "rollback" {
        if let Err(denied) = require_mutation(state) {
            return denied;
        }
        let backup = ctxpect_assets::backup_dir(state.store.root(), id);
        return match ctxpect_assets::rollback(&root, &backup) {
            Ok(value) => {
                let _ = state.store.audit("assets.rollback", "asset", Some(id));
                json_ok(value)
            }
            Err(err) => json_err(err.code, &err.message),
        };
    }

    let registry = load_asset_registry(Some(project.as_path()));
    let asset = match ctxpect_assets::registered(registry.as_ref(), id) {
        Ok(asset) => asset,
        Err(err) => return json_err(err.code, &err.message),
    };
    // Vetting happens before any write, so a refusal leaves nothing behind.
    let plan = match ctxpect_assets::preview(&root, &asset) {
        Ok(plan) => plan,
        Err(err) => return json_err(err.code, &err.message),
    };
    match action {
        "preview" => json_ok(plan.to_value()),
        "copy" => {
            if let Err(denied) = require_mutation(state) {
                return denied;
            }
            let backup = ctxpect_assets::backup_dir(state.store.root(), &plan.tx_id);
            let meta = match ctxpect_assets::apply(&root, &plan, &backup, true) {
                Ok(meta) => meta,
                Err(err) => return json_err(err.code, &err.message),
            };
            if let Err(err) =
                state
                    .store
                    .put_named("assetlock", id, &ctxpect_assets::lock_entry(&plan))
            {
                return json_err(err.code, &err.message);
            }
            let _ = state.store.audit("assets.copy", "asset", Some(id));
            json_ok(merge_fields(
                meta,
                [
                    ("plan", plan.to_value()),
                    ("lock", asset_lock(&state.store)),
                ],
            ))
        }
        other => json_err("api.not_found", &format!("unknown asset action `{other}`")),
    }
}

/// The sync target this API operates on.
///
/// Fixed to a directory inside the store. A destination taken from the
/// request body would be an arbitrary filesystem write driven by page
/// content; cross-device transport therefore stays on the CLI, where the
/// user names the destination explicitly.
fn api_sync_dest(state: &AppState) -> PathBuf {
    state.store.root().join("sync/folder")
}

/// A bundle id is echoed into an append-only log, so it must not be able to
/// forge a line there or travel anywhere as a path.
fn valid_bundle_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        && !id.contains("..")
}

/// Preview or apply a folder-transport sync bundle.
///
/// Preview is read-only. Apply is a store mutation and goes through the one
/// authority. Neither reports transport success as semantic verification.
fn sync_api(body: &str, state: &AppState, apply: bool) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let bundle_id = parsed
        .get("bundle_id")
        .and_then(Value::as_str)
        .unwrap_or("bundle-local");
    if !valid_bundle_id(bundle_id) {
        return json_err("sync.bundle_id_invalid", "bundle_id must be a short id");
    }
    if apply && let Err(denied) = require_mutation(state) {
        return denied;
    }

    let settings = state.store.settings().unwrap_or(Value::Null);
    let vault_required = settings.get("vault").and_then(Value::as_str) == Some("required");

    // Only an explicitly named Receipt travels; the bundle never sweeps the
    // ledger on its own.
    let receipts = match parsed.get("receipt_id").and_then(Value::as_str) {
        Some(id) => match state.store.get_receipt(id) {
            Ok(receipt) => vec![receipt],
            Err(err) => return json_err(err.code, &err.message),
        },
        None => Vec::new(),
    };

    let packed = match bundle(
        bundle_id,
        object([("desired", string("metadata-only"))]),
        &["env:CTXPECT_TOKEN"],
        receipts,
        vault_required,
    ) {
        Ok(value) => value,
        Err(err) => return json_err(err.code, &err.message),
    };

    let dest = api_sync_dest(state);
    let outcome = if apply {
        apply_folder(&dest, &packed)
    } else {
        preview_apply(&dest, &packed)
    };
    match outcome {
        Ok(value) => json_ok(merge_fields(
            value,
            [
                ("bundle", packed),
                ("dest", string("<store>/sync/folder")),
                (
                    "dest_is_fixed",
                    Value::Bool(true),
                ),
                (
                    "cross_device_transport",
                    string("cli-only"),
                ),
            ],
        )),
        Err(err) => json_err(err.code, &err.message),
    }
}

fn merge_fields<'a>(base: Value, extra: impl IntoIterator<Item = (&'a str, Value)>) -> Value {
    let mut map = match base {
        Value::Object(map) => map,
        other => {
            let mut map = BTreeMap::new();
            map.insert("result".to_string(), other);
            map
        }
    };
    for (key, value) in extra {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}

/// Sync state read from this store.
///
/// `encryption: "unavailable"` is kept because it is true: E2EE is not
/// implemented in this slice. What was missing is everything else — the
/// endpoint reported no actual state at all.
fn sync_status(state: &AppState) -> Value {
    let settings = state.store.settings().unwrap_or(Value::Null);
    let vault_required = settings.get("vault").and_then(Value::as_str) == Some("required");
    let bundles = state.store.list_named("sync").unwrap_or_default();
    let receipts = match state.store.list_receipts() {
        Ok(Value::Array(items)) => items.len() as i64,
        _ => 0,
    };

    object([
        ("schema", string("ctxpect-sync-status-v1")),
        // Not implemented, and reported as such rather than as "off".
        ("encryption", string("unavailable")),
        ("encryption_reason_code", string("sync.e2ee_unimplemented")),
        // A transport that succeeded moved bytes; it did not verify meaning.
        ("transport_success_is_verified", Value::Bool(false)),
        ("vault_required", Value::Bool(vault_required)),
        ("local_bundles", Value::Int(bundles.len() as i64)),
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

/// Whether the current Receipt still describes the project on disk.
///
/// Staleness is re-derived by hashing the evidence the Receipt actually
/// declared. The endpoint previously reported `stale: false` unconditionally,
/// which is the one answer that cannot be wrong-flagged and therefore says
/// nothing.
fn monitor_status(state: &AppState) -> Value {
    let receipt = match current_receipt(state) {
        Ok(receipt) => receipt,
        Err(err) => {
            return object([
                ("schema", string("ctxpect-monitor-v1")),
                // Oneshot is an architectural fact: this product observes on
                // demand and does not keep a watcher running.
                ("mode", string("oneshot")),
                ("daemon_required", Value::Bool(false)),
                ("current_receipt_id", Value::Null),
                (
                    "staleness",
                    object([
                        ("status", string("unknown")),
                        ("reason_code", string(err.code)),
                        ("changed_evidence", array([])),
                        ("unreadable_evidence", array([])),
                    ]),
                ),
            ]);
        }
    };

    let evidence = receipt
        .get("evidence")
        .and_then(Value::as_array)
        .unwrap_or(&[]);
    let root = state.project.as_deref().and_then(|path| Root::new(path).ok());

    let mut changed = Vec::new();
    let mut unreadable = Vec::new();
    let mut compared = 0i64;
    for item in evidence {
        let path = item.get("path").and_then(Value::as_str).unwrap_or("");
        let recorded = item.get("content_digest").and_then(Value::as_str).unwrap_or("");
        let in_project = item.get("root").and_then(Value::as_str) == Some("project");
        if path.is_empty() || recorded.is_empty() {
            unreadable.push(object([
                ("path", string(path)),
                ("reason_code", string("evidence.incomplete")),
            ]));
            continue;
        }
        // Evidence outside the declared project root has no path this process
        // is allowed to re-read, so it is unknown rather than unchanged.
        let Some(root) = root.as_ref().filter(|_| in_project) else {
            unreadable.push(object([
                ("path", string(path)),
                ("reason_code", string("evidence.root_not_declared")),
            ]));
            continue;
        };
        match ctxpect_fs::read_contained(root, root.path().join(path)) {
            Ok(content) => {
                // `whole_digest` is the same value the Receipt recorded, so
                // the comparison cannot drift on hashing details. It is empty
                // for a multiply-linked file, whose content is withheld.
                if content.whole_digest.is_empty() {
                    unreadable.push(object([
                        ("path", string(path)),
                        ("reason_code", string("evidence.content_withheld")),
                    ]));
                    continue;
                }
                compared += 1;
                if content.whole_digest != recorded {
                    changed.push(string(path));
                }
            }
            Err(refusal) => unreadable.push(object([
                ("path", string(path)),
                ("reason_code", string(refusal_code(&refusal))),
            ])),
        }
    }

    // Any evidence we could not re-read makes the verdict unknown: a partial
    // comparison must not be reported as "current".
    let status = if !unreadable.is_empty() {
        "unknown"
    } else if !changed.is_empty() {
        "stale"
    } else if compared > 0 {
        "current"
    } else {
        "unknown"
    };
    let reason = match status {
        "stale" => "monitor.evidence_changed",
        "current" => "monitor.evidence_unchanged",
        _ if compared == 0 && evidence.is_empty() => "monitor.no_evidence_declared",
        _ => "monitor.evidence_unreadable",
    };

    object([
        ("schema", string("ctxpect-monitor-v1")),
        ("mode", string("oneshot")),
        ("daemon_required", Value::Bool(false)),
        (
            "current_receipt_id",
            receipt.get("receipt_id").cloned().unwrap_or(Value::Null),
        ),
        (
            "staleness",
            object([
                ("status", string(status)),
                ("reason_code", string(reason)),
                ("compared", Value::Int(compared)),
                ("changed_evidence", array(changed)),
                ("unreadable_evidence", array(unreadable)),
            ]),
        ),
    ])
}

fn refusal_code(refusal: &ctxpect_fs::Refusal) -> &'static str {
    match refusal {
        ctxpect_fs::Refusal::EscapesRoot { .. } => "evidence.escapes_root",
        _ => "evidence.unreadable",
    }
}

/// Team compliance, computed from this store.
///
/// The endpoint previously answered with the list of field *names* it would
/// one day report. That reads as a compliance report while asserting nothing,
/// so each field now carries either a computed status or an explicit unknown.
fn team_compliance(state: &AppState) -> Value {
    let standard_ids = state.store.list_named("standards").unwrap_or_default();
    let mut standards = Vec::new();
    let mut signed_count = 0i64;
    let mut adopted_count = 0i64;
    for id in &standard_ids {
        let doc = state.store.get_named("standards", id).ok();
        // Signature is re-derived from the stored record, never read off a
        // `signed` field the record could simply claim.
        let signed = doc
            .as_ref()
            .is_some_and(|doc| verify_standard_document(&state.store, doc).is_ok());
        if signed {
            signed_count += 1;
        }
        let adoption = state.store.get_named("adoptions", id).ok();
        if adoption.is_some() {
            adopted_count += 1;
        }
        standards.push(object([
            ("standard_id", string(id.as_str())),
            ("signed", Value::Bool(signed)),
            (
                "adoption_state",
                adoption
                    .as_ref()
                    .and_then(|item| item.get("state").cloned())
                    .unwrap_or_else(|| string("not-adopted")),
            ),
            (
                "pinned_digest",
                adoption
                    .as_ref()
                    .and_then(|item| item.get("pinned_digest").cloned())
                    .unwrap_or(Value::Null),
            ),
        ]));
    }

    let now = now_unix();
    let exception_ids = state.store.list_named("exceptions").unwrap_or_default();
    let mut live_exceptions = Vec::new();
    for id in &exception_ids {
        let Ok(record) = state.store.get_named("exceptions", id) else {
            continue;
        };
        let Ok(status) = exception_status(&record, now, true) else {
            continue;
        };
        if status.get("grants").and_then(Value::as_bool) == Some(true) {
            live_exceptions.push(object([
                ("exception_id", string(id.as_str())),
                (
                    "requester",
                    record.get("requester").cloned().unwrap_or(Value::Null),
                ),
                (
                    "decided_by",
                    record.get("decided_by").cloned().unwrap_or(Value::Null),
                ),
            ]));
        }
    }

    // Drift, unknown and freshness are properties of a Receipt. Without one in
    // this session they are unknown, not zero.
    let receipt = current_receipt(state).ok();
    let diagnosis = receipt.as_ref().map(diagnose);
    let unknown_cells = diagnosis
        .as_ref()
        .and_then(|item| item.pointer(&["counts", "unknown"]).cloned())
        .unwrap_or(Value::Null);

    object([
        ("schema", string("ctxpect-team-compliance-v1")),
        // Metadata only: member Receipt bodies are never part of this report.
        ("redacted", Value::Bool(true)),
        ("member_bodies_included", Value::Bool(false)),
        (
            "disclosure",
            object([
                ("scope", string("this-store-only")),
                (
                    "note",
                    string(
                        "Counts describe the local store. This slice has no team transport, so it is not a roll-up across members.",
                    ),
                ),
            ]),
        ),
        (
            "standard_status",
            object([
                ("total", Value::Int(standard_ids.len() as i64)),
                ("signed", Value::Int(signed_count)),
                ("adopted", Value::Int(adopted_count)),
                ("standards", array(standards)),
            ]),
        ),
        (
            "exception",
            object([
                ("total", Value::Int(exception_ids.len() as i64)),
                ("live", Value::Int(live_exceptions.len() as i64)),
                ("granting", array(live_exceptions)),
            ]),
        ),
        (
            "drift",
            diagnosis
                .as_ref()
                .and_then(|item| item.get("counts").cloned())
                .unwrap_or(Value::Null),
        ),
        ("unknown", unknown_cells),
        (
            "freshness",
            receipt.as_ref().map_or_else(
                || {
                    object([
                        ("status", string("unknown")),
                        ("reason_code", string("api.no_current_receipt")),
                    ])
                },
                |receipt| {
                    object([
                        ("status", string("current-session-receipt")),
                        (
                            "receipt_id",
                            receipt.get("receipt_id").cloned().unwrap_or(Value::Null),
                        ),
                    ])
                },
            ),
        ),
    ])
}

/// The Receipt this daemon session is currently showing, if any.
///
/// There is no fallback to "whatever is newest in the store": a care plan or
/// a staleness verdict computed against a Receipt the user is not looking at
/// would be a different question's answer.
fn current_receipt(state: &AppState) -> Result<Value, ctxpect_store::StoreError> {
    let Ok(cur) = state.current_receipt.lock() else {
        return Err(ctxpect_store::StoreError {
            code: "api.state_unavailable",
            message: "current receipt state is unavailable".into(),
        });
    };
    match cur.as_deref() {
        Some(id) => state.store.get_receipt(id),
        None => Err(ctxpect_store::StoreError {
            code: "api.no_current_receipt",
            message: "no inspect has run in this session yet".into(),
        }),
    }
}

/// Findings live on the *diagnosis*, not on the Receipt: `receipt.findings`
/// is empty and `diagnose()` is what derives them.
fn findings_of(diagnosis: &Value) -> &[Value] {
    diagnosis
        .get("findings")
        .and_then(Value::as_array)
        .unwrap_or(&[])
}

/// The care plan for one finding, read off that finding.
///
/// Every field here comes from the Receipt. The endpoint previously answered
/// with the same four constants for any id, including ids that did not exist,
/// which made an unknown finding indistinguishable from a locked one.
fn care_plan_api(method: &str, path: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    let _ = (method, body);
    let finding_id = path.trim_start_matches("/api/v1/care-plan/");
    if finding_id.is_empty() {
        return json_err("api.not_found", "care-plan requires a finding id");
    }
    let receipt = match current_receipt(state) {
        Ok(receipt) => receipt,
        Err(err) => return json_err(err.code, &err.message),
    };
    let diagnosis = diagnose(&receipt);
    let Some(finding) = findings_of(&diagnosis).iter().find(|item| {
        item.get("finding_id").and_then(Value::as_str) == Some(finding_id)
    }) else {
        return json_err(
            "api.not_found",
            &format!("`{finding_id}` is not a finding on the current Receipt"),
        );
    };

    let treatment = finding.get("treatment").cloned().unwrap_or(Value::Null);
    let placement = finding.get("placement").cloned().unwrap_or(Value::Null);
    let flag = |key: &str| {
        treatment
            .get(key)
            .and_then(Value::as_bool)
            .map_or(Value::Null, Value::Bool)
    };
    json_ok(object([
        ("finding_id", string(finding_id)),
        (
            "receipt_id",
            string(receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("")),
        ),
        ("title", finding.get("title").cloned().unwrap_or(Value::Null)),
        (
            "confirmation",
            finding.get("confirmation").cloned().unwrap_or(Value::Null),
        ),
        (
            "severity",
            finding.get("severity").cloned().unwrap_or(Value::Null),
        ),
        ("treatment_locked", flag("locked")),
        (
            "lock_reason",
            treatment.get("lock_reason").cloned().unwrap_or(Value::Null),
        ),
        ("unlocks_via_advisor", flag("unlocks_via_advisor")),
        ("unlocks_via_export", flag("unlocks_via_export")),
        (
            "unlocks_via_user_attestation_alone",
            flag("unlocks_via_user_attestation_alone"),
        ),
        (
            "authority",
            placement.get("authority").cloned().unwrap_or(Value::Null),
        ),
        ("loss", placement.get("loss").cloned().unwrap_or(Value::Null)),
        ("target", placement.get("target").cloned().unwrap_or(Value::Null)),
        (
            "next_evidence",
            finding.get("next_evidence").cloned().unwrap_or(Value::Null),
        ),
        // A plan is a preview until the user applies it; that is a product
        // invariant, not something read off the finding.
        ("preview_required", Value::Bool(true)),
    ]))
}

fn apply_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let Some(project) = state.project.as_ref() else {
        return json_err("usage.invalid", "project required");
    };
    let Ok(root) = Root::new(project) else {
        return json_err("io.missing", "project");
    };
    let intent = Intent {
        intent_id: parsed
            .get("intent_id")
            .and_then(Value::as_str)
            .unwrap_or("intent-api")
            .to_string(),
        authority: parsed
            .get("authority")
            .and_then(Value::as_str)
            .unwrap_or("contexpect-native")
            .to_string(),
        target_rel: parsed
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("AGENTS.md")
            .to_string(),
        desired: parsed
            .get("desired")
            .and_then(Value::as_str)
            .unwrap_or("updated\n")
            .to_string(),
    };
    // Client-supplied `approved` is not authorization evidence and is ignored.
    match authorize_store_apply(&state.store, Some(project.as_path())) {
        Ok(_) => {}
        Err(err) => return json_err(err.code(), &err.message()),
    }
    match proj_preview(&root, &intent) {
        Ok(preview) => match proj_apply(
            &root,
            &intent,
            &preview,
            &ctxpect_projection::backup_dir(state.store.root(), &preview.tx_id),
            true,
        ) {
            Ok(v) => {
                let inspect_args = InspectArgs {
                    json: true,
                    offline: true,
                    project: project.clone(),
                    cwd: None,
                    harness: "codex".into(),
                    surface: "cli".into(),
                    version: "0.147.0".into(),
                    version_explicit: false,
                    codex_home: None,
                    require: vec!["instructions".into()],
                    os_lane: "macos-27-arm64".into(),
                    store: None,
                };
                match inspect(inspect_args) {
                    Ok(report) => match persist_inspect(&state.store, &report.envelope, "one-shot")
                    {
                        Ok(receipt) => json_ok(object([
                            ("transaction", v),
                            (
                                "post_receipt_id",
                                string(
                                    receipt
                                        .get("receipt_id")
                                        .and_then(Value::as_str)
                                        .unwrap_or(""),
                                ),
                            ),
                        ])),
                        Err(err) => json_err(err.code(), &err.message()),
                    },
                    Err(err) => json_err(err.code(), &err.message()),
                }
            }
            Err(err) => json_err(err.code, &err.message),
        },
        Err(err) => json_err(err.code, &err.message),
    }
}

fn collect_api(state: &AppState) -> (u16, &'static str, String) {
    let Some(project) = state.project.as_ref() else {
        return json_err("usage.invalid", "project required");
    };
    let Ok(root) = Root::new(project) else {
        return json_err("io.missing", "project");
    };
    match scan(&root) {
        Ok(inventory) => json_ok(object([
            ("inventory_digest", string(inventory.digest())),
            (
                "entry_count",
                Value::Int(i64::try_from(inventory.entries.len()).unwrap_or(0)),
            ),
        ])),
        Err(err) => json_err("io.unresolvable", &err.to_string()),
    }
}

fn named_get(state: &AppState, folder: &str, id: &str) -> (u16, &'static str, String) {
    match state.store.get_named(folder, id) {
        Ok(v) => json_ok(v),
        Err(err) => json_err(err.code, &err.message),
    }
}

fn standard_get(state: &AppState, id: &str) -> (u16, &'static str, String) {
    match state.store.get_named("standards", id) {
        Ok(doc) => match verify_standard_document(&state.store, &doc) {
            Ok(v) => json_ok(v),
            Err(err) => json_err(err.code(), &err.message()),
        },
        Err(err) => json_err(err.code, &err.message),
    }
}

fn strip_id<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    path.strip_prefix(prefix)
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn url_decode(input: &str) -> String {
    let mut bytes = Vec::new();
    let raw = input.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'%' if i + 2 < raw.len() => {
                let hex = &input[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    bytes.push(byte);
                    i += 3;
                    continue;
                }
                bytes.push(b'%');
                i += 1;
            }
            b'+' => {
                bytes.push(b' ');
                i += 1;
            }
            b => {
                bytes.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn rollback_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let Some(project) = state.project.as_ref() else {
        return json_err("usage.invalid", "project required");
    };
    let Ok(root) = Root::new(project) else {
        return json_err("io.missing", "project");
    };
    if let Err(denied) = require_mutation(state) {
        return denied;
    }
    let tx = parsed.get("tx_id").and_then(Value::as_str).unwrap_or("");
    let target = parsed.get("target").and_then(Value::as_str).unwrap_or("AGENTS.md");
    match proj_rollback(&root, &ctxpect_projection::backup_dir(state.store.root(), tx), target) {
        Ok(v) => json_ok(v),
        Err(err) => json_err(err.code, &err.message),
    }
}

fn query(full: &str, key: &str) -> Option<String> {
    let q = full.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == key
        {
            return Some(v.to_string());
        }
    }
    None
}

fn static_file(path: &str, state: &AppState) -> (u16, &'static str, String) {
    let Some(root) = &state.ui_root else {
        return (
            200,
            "text/html; charset=utf-8",
            "<!doctype html><meta charset=utf-8><title>Contexpect</title><p>UI dist not built. Run packages/ui build, then pass --ui-root. API is at /api/v1/health.</p>".into(),
        );
    };
    let rel = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
    if rel.contains("..") {
        return json_err("api.path", "rejected");
    }
    // A textual `..` check does not make a path safe: a symlink inside the UI
    // root points outside it without the request ever containing `..`. The
    // same containment check that guards project reads guards this one —
    // it canonicalises first, so it sees where the path actually lands.
    let Ok(contained_root) = Root::new(root) else {
        return json_err("api.path", "ui root is unreadable");
    };
    let file = match contained_root.contain(root.join(rel)) {
        Ok(file) => Some(file),
        Err(Refusal::EscapesRoot { .. }) => {
            return json_err("api.path", "rejected");
        }
        // Unresolvable means "no such file", which is the SPA's own routes
        // arriving here. Those fall through to index.html.
        Err(_) => None,
    };
    match file.and_then(|file| fs::read_to_string(&file).ok()) {
        Some(text) => {
            let ctype = if rel.ends_with(".js") {
                "application/javascript"
            } else if rel.ends_with(".css") {
                "text/css"
            } else {
                "text/html; charset=utf-8"
            };
            (200, ctype, text)
        }
        None => {
            // index.html is resolved through the same check, so a symlinked
            // index cannot smuggle content in either.
            match contained_root
                .contain(root.join("index.html"))
                .ok()
                .and_then(|index| fs::read_to_string(index).ok())
            {
                Some(text) => (200, "text/html; charset=utf-8", text),
                None => json_err("api.not_found", path),
            }
        }
    }
}

#[allow(dead_code)]
fn _path(p: &Path) -> &Path {
    p
}
