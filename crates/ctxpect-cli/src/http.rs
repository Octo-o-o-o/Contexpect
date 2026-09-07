//! Localhost HTTP API. UI does not scan the disk.

use crate::args::{InspectArgs, ProductArgs};
use crate::catalog::{family_entry, integrations_json};
use crate::dispatch::{
    authorize_store_apply, effective_store_policy, listen_addr, persist_inspect,
    verify_standard_document, ProductReport,
};
use crate::inspect::inspect;
use crate::jsonutil::with_snapshot_digest;
use ctxpect_advisor::suggest;
use ctxpect_collect::scan;
use ctxpect_diff::{diff, EquivalenceProfile};
use ctxpect_doctor::diagnose;
use ctxpect_effect::{decide, run_local_instructions_probe, ExperimentContract};
use ctxpect_fs::Root;
use ctxpect_importer::import_session;
use ctxpect_projection::{apply as proj_apply, preview as proj_preview, rollback as proj_rollback, Intent};
use ctxpect_schema::{array, canonical_json, object, parse, string, Value};
use ctxpect_store::Store;

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
        ("GET", "/api/v1/assets") => json_ok(object([
            ("catalog", integrations_json("codex", true)),
            ("apm_authority", string("apm-unique-not-re-evaluated")),
            ("copy_executor", string("unimplemented")),
            ("reason_code", string("assets.copy_unimplemented")),
        ])),
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
        ("GET", "/api/v1/monitor") => json_ok(object([
            ("mode", string("oneshot")),
            ("daemon_required", Value::Bool(false)),
            ("stale", Value::Bool(false)),
        ])),
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
        ("GET", "/api/v1/sync") => json_ok(object([
            ("encryption", string("unavailable")),
            ("transport_success_is_verified", Value::Bool(false)),
        ])),
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
        ("GET", "/api/v1/team/compliance") => json_ok(object([
            ("redacted", Value::Bool(true)),
            ("member_bodies_included", Value::Bool(false)),
            ("fields", array([
                string("disclosure"),
                string("standard_status"),
                string("drift"),
                string("unknown"),
                string("freshness"),
                string("exception"),
            ])),
        ])),
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
    let parsed = match parse(body) {
        Ok(v) => v,
        Err(err) => return json_err("api.parse", &err.to_string()),
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
    let parsed = parse(body).unwrap_or_else(|_| object::<String>([]));
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
    let parsed = parse(body).unwrap_or_else(|_| object::<String>([]));
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

fn care_plan_api(method: &str, path: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    let finding_id = path.trim_start_matches("/api/v1/care-plan/");
    let _ = (method, body, state);
    json_ok(object([
        ("finding_id", string(finding_id)),
        ("treatment_locked", Value::Bool(true)),
        ("unlocks_via_advisor", Value::Bool(false)),
        ("authority", string("contexpect-native")),
        ("preview_required", Value::Bool(true)),
    ]))
}

fn apply_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    let parsed = parse(body).unwrap_or_else(|_| object::<String>([]));
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
    let parsed = parse(body).unwrap_or_else(|_| object::<String>([]));
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
    let file = root.join(rel);
    match fs::read_to_string(&file) {
        Ok(text) => {
            let ctype = if rel.ends_with(".js") {
                "application/javascript"
            } else if rel.ends_with(".css") {
                "text/css"
            } else {
                "text/html; charset=utf-8"
            };
            (200, ctype, text)
        }
        Err(_) => {
            let index = root.join("index.html");
            match fs::read_to_string(index) {
                Ok(text) => (200, "text/html; charset=utf-8", text),
                Err(_) => json_err("api.not_found", path),
            }
        }
    }
}

#[allow(dead_code)]
fn _path(p: &Path) -> &Path {
    p
}
