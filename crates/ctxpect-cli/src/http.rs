//! Localhost HTTP API. UI does not scan the disk.

use crate::args::{InspectArgs, ProductArgs};
use crate::catalog::{family_entry, integrations_json};
use crate::dispatch::{persist_inspect_in,
    asset_lock, assets_status, authorize_store_apply, effective_store_policy, exception_state,
    listen_addr, load_asset_registry, load_preview, mark_preview, now_unix,
    persist_preview, policy_fresh, policy_query_scope, project_scope_digest, standard_status,
    verify_standard_document, Authorization, ProductReport,
};
use crate::inspect::inspect;
use crate::jsonutil::with_snapshot_digest;
use ctxpect_advisor::suggest;
use ctxpect_collect::scan;
use ctxpect_diff::{diff, EquivalenceProfile};
use ctxpect_doctor::diagnose;
use ctxpect_effect::{decide as effect_decide, not_executed, RunsDocument};
use ctxpect_fs::{Refusal, Root};
use ctxpect_importer::{deepseek_harness, import_session_bytes};
use ctxpect_policy::exception_status;
use ctxpect_projection::{
    apply as proj_apply, preview as proj_preview, read_tx, rollback as proj_rollback,
    target_outside_store, valid_tx_id, Intent, PREVIEW_APPLIED, PREVIEW_ROLLED_BACK,
};
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

/// The harness coordinate this daemon session is currently about. Set from
/// the daemon's own `--harness/--surface/--version/--os-lane` and moved by
/// each `POST /api/v1/inspect`; catalog endpoints read it rather than a
/// hardcoded family.
#[derive(Clone)]
struct Coordinate {
    harness: String,
    version: String,
    surface: String,
    os_lane: String,
}

impl Coordinate {
    fn to_value(&self) -> Value {
        object([
            ("harness", string(&self.harness)),
            ("version", string(&self.version)),
            ("surface", string(&self.surface)),
            ("os_lane", string(&self.os_lane)),
        ])
    }
}

struct AppState {
    store: Store,
    project: Option<PathBuf>,
    /// The scope digest of `project` (see `project_scope_digest`); Receipts
    /// carry the same digest, which is how a Receipt is known to belong to
    /// the root this daemon rescans.
    project_digest: Option<String>,
    /// The Codex home the daemon was started with; a request may name only
    /// this one.
    codex_home: Option<PathBuf>,
    ui_root: Option<PathBuf>,
    listen: String,
    generation: AtomicU64,
    current_receipt: Mutex<Option<String>>,
    coordinate: Mutex<Coordinate>,
}

/// Whether a Receipt was observed in the root this daemon serves.
enum ReceiptScope {
    Matches,
    Mismatch,
    /// The Receipt carries no project digest (persisted without a project).
    Unknown,
}

fn receipt_scope(state: &AppState, receipt: &Value) -> ReceiptScope {
    let recorded = receipt
        .pointer(&["coordinate", "project_digest"])
        .and_then(Value::as_str)
        .filter(|d| !d.is_empty() && *d != "unknown");
    match (recorded, state.project_digest.as_deref()) {
        (Some(recorded), Some(ours)) if recorded == ours => ReceiptScope::Matches,
        (Some(_), Some(_)) => ReceiptScope::Mismatch,
        _ => ReceiptScope::Unknown,
    }
}

/// The Receipt a request names explicitly, or the session's current one.
/// An explicit id that does not exist is an error — never a fallback to
/// whatever is current.
fn selected_receipt(state: &AppState, receipt_id: Option<&str>) -> Result<Value, ctxpect_store::StoreError> {
    match receipt_id {
        Some(id) => state.store.get_receipt(id),
        None => current_receipt(state),
    }
}

/// `selected_receipt`, refused when the Receipt is not from this root.
fn scoped_receipt(state: &AppState, receipt_id: Option<&str>) -> Result<Value, ctxpect_store::StoreError> {
    let receipt = selected_receipt(state, receipt_id)?;
    match receipt_scope(state, &receipt) {
        ReceiptScope::Matches => Ok(receipt),
        ReceiptScope::Mismatch => Err(ctxpect_store::StoreError {
            code: "api.receipt_scope",
            message: "this Receipt was observed in another project; the daemon does not read it against its own root".into(),
        }),
        ReceiptScope::Unknown => Err(ctxpect_store::StoreError {
            code: "api.receipt_scope_unknown",
            message: "this Receipt carries no project scope; it cannot be tied to the daemon's root".into(),
        }),
    }
}

impl AppState {
    fn coordinate(&self) -> Coordinate {
        self.coordinate.lock().map(|c| c.clone()).unwrap_or_else(|_| Coordinate {
            harness: "unknown".into(),
            version: "unknown".into(),
            surface: "unknown".into(),
            os_lane: "unknown".into(),
        })
    }
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
    let project_digest = args
        .project
        .as_deref()
        .map(|p| project_scope_digest(store.root(), Some(p)));
    let state = Arc::new(AppState {
        store,
        project: args.project.clone(),
        project_digest,
        codex_home: args.codex_home.clone(),
        ui_root,
        listen: bound.to_string(),
        generation: AtomicU64::new(1),
        current_receipt: Mutex::new(None),
        coordinate: Mutex::new(Coordinate {
            harness: args.harness.clone(),
            version: args.version.clone(),
            surface: args.surface.clone(),
            os_lane: args.os_lane.clone(),
        }),
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

/// Largest request head (request line + headers) accepted.
const MAX_HEADER_BYTES: usize = 64 * 1024;
/// Largest request body accepted; a native session log for import fits
/// comfortably, anything larger is refused with 413 rather than truncated.
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
/// How long one socket read may wait. An idle connection (a browser's
/// preconnect, a client that never sends) must not hold the single-threaded
/// daemon forever.
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn crlfcrlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Read one request completely: the head up to the blank line, then exactly
/// `Content-Length` body bytes. One `read()` is not a request — a body that
/// arrives in a second segment used to be silently dropped and the handler
/// ran with `{}`.
fn read_request(stream: &mut TcpStream) -> Result<(String, Vec<u8>), (u16, &'static str)> {
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut chunk = [0u8; 8192];
    let mut read_more = |stream: &mut TcpStream, buf: &mut Vec<u8>| -> Result<(), (u16, &'static str)> {
        match stream.read(&mut chunk) {
            Ok(0) => Err((400, "api.bad_request")),
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                Ok(())
            }
            Err(err) if matches!(err.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                Err((408, "api.timeout"))
            }
            Err(_) => Err((400, "api.bad_request")),
        }
    };
    let header_end = loop {
        if let Some(pos) = crlfcrlf(&buf) {
            break pos;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err((431, "api.header_too_large"));
        }
        read_more(stream, &mut buf)?;
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut body = buf[header_end + 4..].to_vec();
    let mut content_length = 0usize;
    for line in head.lines().skip(1) {
        let Some((k, v)) = line.split_once(':') else { continue };
        match k.trim().to_ascii_lowercase().as_str() {
            "content-length" => {
                content_length = v.trim().parse::<usize>().map_err(|_| (400, "api.bad_request"))?;
            }
            "transfer-encoding" => return Err((501, "api.transfer_encoding_unsupported")),
            _ => {}
        }
    }
    if content_length > MAX_BODY_BYTES {
        return Err((413, "api.payload_too_large"));
    }
    while body.len() < content_length {
        read_more(stream, &mut body)?;
    }
    body.truncate(content_length);
    Ok((head, body))
}

fn handle_client(mut stream: TcpStream, state: &AppState) -> Result<(), ()> {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let (head, body_bytes) = match read_request(&mut stream) {
        Ok(parts) => parts,
        Err((status, code)) => {
            return write_res(
                &mut stream,
                status,
                "application/json",
                &format!(r#"{{"error":{{"code":"{code}"}}}}"#),
            );
        }
    };
    let head = head.as_str();
    let body_text = String::from_utf8_lossy(&body_bytes);
    let body: &str = &body_text;
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
    // A missing Host is refused rather than assumed local: HTTP/1.1 requires
    // it, and "absent" is not evidence of anything.
    if !is_loopback_authority(&host) {
        return write_res(&mut stream, 403, "application/json", r#"{"error":{"code":"api.host"}}"#);
    }
    let origin = headers.get("origin").cloned().unwrap_or_default();
    if !origin.is_empty() && !is_loopback_origin(&origin) {
        return write_res(&mut stream, 403, "application/json", r#"{"error":{"code":"api.origin"}}"#);
    }
    if method == "OPTIONS" {
        // A preflight-style probe answers for known paths only, with no body.
        let path_only = path.split('?').next().unwrap_or(path);
        let known = !path_only.starts_with("/api/v1/")
            || !matches!(match_route("GET", path_only), RouteMatch::NotFound);
        return if known {
            write_res(&mut stream, 204, "text/plain", "")
        } else {
            write_res(&mut stream, 400, "application/json", r#"{"error":{"code":"api.not_found"}}"#)
        };
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
    // HEAD is GET without the body: same routing, same headers, same length.
    let head_only = method == "HEAD";
    let (status, ctype, body_out) = route(if head_only { "GET" } else { method }, path, body, state);
    write_response(&mut stream, status, ctype, &body_out, head_only)
}

/// The host part of an authority, without the port.
///
/// IPv6 literals are bracketed (`[::1]:7420`), so the first `:` is not the
/// port separator there.
fn authority_host(authority: &str) -> &str {
    match authority.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    }
}

/// Whether an authority names this machine's loopback interface.
///
/// The comparison is exact. A prefix test would accept `127.0.0.1.evil.com`,
/// which is the standard DNS-rebinding bypass: an attacker registers that
/// name, points it at 127.0.0.1, and the browser then sends a Host header
/// that passes while the page's origin is the attacker's.
fn is_loopback_authority(authority: &str) -> bool {
    matches!(authority_host(authority), "127.0.0.1" | "localhost" | "::1")
}

/// Whether an `Origin` names a loopback HTTP origin. Scheme and host are both
/// checked; the host part is matched exactly, for the reason above.
fn is_loopback_origin(origin: &str) -> bool {
    let Some((scheme, authority)) = origin.split_once("://") else {
        return false;
    };
    if scheme != "http" && scheme != "https" {
        return false;
    }
    is_loopback_authority(authority)
}

fn write_res(stream: &mut TcpStream, status: u16, ctype: &str, body: &str) -> Result<(), ()> {
    write_response(stream, status, ctype, body, false)
}

fn write_response(stream: &mut TcpStream, status: u16, ctype: &str, body: &str, head_only: bool) -> Result<(), ()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        _ => "Error",
    };
    let resp = format!(
        // No `Access-Control-Allow-Origin`: nothing needs cross-origin access.
        // The UI is served by this daemon, and the dev server proxies `/api`,
        // so both are same-origin. The header used to be present but omitted
        // the port, so it matched no real origin — a rule that looked like a
        // policy while granting nothing, and that someone would eventually
        // "fix" into a real grant.
        //
        // `frame-ancestors 'none'` keeps this API out of a frame on another
        // page; `style-src` allows inline styles because React sets them
        // through the `style` prop.
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {ctype}\r\n\
         Content-Length: {}\r\n\
         Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n\
         X-Content-Type-Options: nosniff\r\n\
         X-Frame-Options: DENY\r\n\
         Referrer-Policy: no-referrer\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n{}",
        body.len(),
        if head_only { "" } else { body }
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

/// Authorize one mutation. The returned [`Authorization`] holds the store's
/// advisory lock; callers bind it for the duration of their writes.
fn require_mutation(
    state: &AppState,
    action: &str,
    target: &str,
) -> Result<Authorization, (u16, &'static str, String)> {
    match authorize_store_apply(&state.store, state.project.as_deref(), action, target) {
        Ok(auth) => Ok(auth),
        Err(err) => Err(json_err(err.code(), &err.message())),
    }
}

fn route(method: &str, path: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    let path_only = path.split('?').next().unwrap_or(path);
    if path_only.starts_with("/api/v1/") {
        return api(method, path_only, path, body, state);
    }
    static_file(path_only, state)
}

/// Every routed endpoint, as `(method, path pattern)`. A `:name` segment
/// matches exactly one non-empty segment. Routing is exact on **both**
/// dimensions: a known path under another method is `api.method_not_allowed`
/// (405), never a fallthrough into some handler that ignores the method.
///
/// The UI page contracts are checked against this table by method and path,
/// so an endpoint that is not here cannot be declared by a page.
pub const ROUTE_TABLE: &[(&str, &str)] = &[
    ("GET", "/api/v1/health"),
    ("GET", "/api/v1/status"),
    ("GET", "/api/v1/coordinate"),
    ("POST", "/api/v1/inspect"),
    ("GET", "/api/v1/receipts"),
    ("GET", "/api/v1/receipts/:id"),
    ("POST", "/api/v1/receipts/:id/verify"),
    ("POST", "/api/v1/receipts/:id/delete"),
    ("GET", "/api/v1/doctor"),
    ("GET", "/api/v1/diff"),
    ("GET", "/api/v1/integrations"),
    ("GET", "/api/v1/integrations/:id"),
    ("GET", "/api/v1/assets"),
    ("GET", "/api/v1/assets/:id"),
    ("POST", "/api/v1/assets/:id/preview"),
    ("POST", "/api/v1/assets/:id/copy"),
    ("POST", "/api/v1/assets/:id/rollback"),
    ("GET", "/api/v1/settings/schema"),
    ("GET", "/api/v1/settings"),
    ("PUT", "/api/v1/settings"),
    ("POST", "/api/v1/settings"),
    ("GET", "/api/v1/sessions"),
    ("GET", "/api/v1/sessions/:id"),
    ("GET", "/api/v1/sessions/:id/requests"),
    ("POST", "/api/v1/sessions/import"),
    ("GET", "/api/v1/monitor"),
    ("GET", "/api/v1/policy"),
    ("GET", "/api/v1/exceptions"),
    ("GET", "/api/v1/exceptions/:id"),
    ("POST", "/api/v1/exceptions"),
    ("GET", "/api/v1/standards"),
    ("GET", "/api/v1/standards/:id"),
    ("GET", "/api/v1/sync"),
    ("POST", "/api/v1/sync/preview"),
    ("POST", "/api/v1/sync/apply"),
    ("POST", "/api/v1/advisor"),
    ("GET", "/api/v1/lab"),
    ("GET", "/api/v1/lab/:id"),
    ("POST", "/api/v1/lab"),
    ("GET", "/api/v1/team/compliance"),
    ("GET", "/api/v1/care-plan/:id"),
    ("POST", "/api/v1/collect"),
    ("POST", "/api/v1/intent/preview"),
    ("POST", "/api/v1/apply"),
    ("POST", "/api/v1/rollback"),
];

/// Match `path` against a pattern, returning the `:id` segment if the
/// pattern has one. Literal segments must match exactly.
fn match_pattern<'a>(pattern: &str, path: &'a str) -> Option<Option<&'a str>> {
    let mut want = pattern.trim_start_matches('/').split('/');
    let mut have = path.trim_start_matches('/').split('/');
    let mut id = None;
    loop {
        match (want.next(), have.next()) {
            (None, None) => return Some(id),
            (Some(w), Some(h)) => {
                if let Some(_name) = w.strip_prefix(':') {
                    if h.is_empty() {
                        return None;
                    }
                    id = Some(h);
                } else if w != h {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

enum RouteMatch<'a> {
    /// The matched pattern and the `:id` segment, if any.
    Found(&'static str, Option<&'a str>),
    MethodNotAllowed(Vec<&'static str>),
    NotFound,
}

/// Exact `(method, path)` routing. Literal patterns win over `:id` patterns
/// for the same path, so `POST /sessions/import` is not read as a session
/// named `import`.
fn match_route<'a>(method: &str, path: &'a str) -> RouteMatch<'a> {
    let mut allowed: Vec<&'static str> = Vec::new();
    let mut found: Option<(&'static str, Option<&'a str>, bool)> = None;
    // When a literal pattern names this path, the `:id` patterns do not
    // apply to it at all: `GET /sessions/import` is 405 (only POST is
    // routed there), not a lookup of a session called `import`.
    let literal_names_path = ROUTE_TABLE
        .iter()
        .any(|(_, pattern)| !pattern.contains(':') && match_pattern(pattern, path).is_some());
    for (m, pattern) in ROUTE_TABLE {
        if literal_names_path && pattern.contains(':') {
            continue;
        }
        let Some(id) = match_pattern(pattern, path) else {
            continue;
        };
        if !allowed.contains(m) {
            allowed.push(m);
        }
        if *m != method {
            continue;
        }
        let literal = !pattern.contains(':');
        match &found {
            Some((_, _, was_literal)) if *was_literal => {}
            _ => found = Some((pattern, id, literal)),
        }
    }
    match found {
        Some((pattern, id, _)) => RouteMatch::Found(pattern, id),
        None if !allowed.is_empty() => RouteMatch::MethodNotAllowed(allowed),
        None => RouteMatch::NotFound,
    }
}

fn api(method: &str, path: &str, full: &str, body: &str, state: &AppState) -> (u16, &'static str, String) {
    let (pattern, id) = match match_route(method, path) {
        RouteMatch::Found(pattern, id) => (pattern, id.unwrap_or("")),
        RouteMatch::MethodNotAllowed(allowed) => {
            return (
                405,
                "application/json",
                canonical_json(&object([(
                    "error",
                    object([
                        ("code", string("api.method_not_allowed")),
                        (
                            "message",
                            string(format!("{method} is not routed for {path}; allowed: {}", allowed.join(", "))),
                        ),
                        ("allowed", array(allowed.iter().map(|m| string(*m)))),
                    ]),
                )])),
            );
        }
        RouteMatch::NotFound => return json_err("api.not_found", path),
    };
    let coordinate = state.coordinate();
    match (method, pattern) {
        ("GET", "/api/v1/health") => json_ok(object([
            ("ok", Value::Bool(true)),
            ("generation", Value::Int(i64::try_from(state.generation.load(Ordering::SeqCst)).unwrap_or(0))),
        ])),
        ("GET", "/api/v1/coordinate") => json_ok(merge_fields(
            coordinate.to_value(),
            [
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
                ("generation", Value::Int(i64::try_from(state.generation.load(Ordering::SeqCst)).unwrap_or(0))),
                ("ui_computes_claims", Value::Bool(false)),
            ],
        )),
        ("GET", "/api/v1/status") => status_api(state),
        ("POST", "/api/v1/inspect") => inspect_api(body, state),
        ("GET", "/api/v1/receipts") => match state.store.list_receipts() {
            Ok(v) => json_ok(object([("receipts", v)])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("GET", "/api/v1/receipts/:id") => match state.store.get_receipt(id) {
            Ok(v) => json_ok(v),
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", "/api/v1/receipts/:id/delete") => {
            let _auth = match require_mutation(state, "receipt.delete", id) {
                Ok(auth) => auth,
                Err(denied) => return denied,
            };
            match state.store.delete_receipt(id, "api") {
                Ok(v) => json_ok(v),
                Err(err) => json_err(err.code, &err.message),
            }
        }
        // R04: the same function the CLI's `receipt verify` calls, so a
        // tombstone or a legacy signature gets the same answer on both.
        ("POST", "/api/v1/receipts/:id/verify") => match crate::dispatch::verify_receipt_report(&state.store, id) {
            Ok((_, report)) => json_ok(report),
            Err(err) => json_err(err.code(), &err.message()),
        },
        ("GET", "/api/v1/doctor") => doctor_api(full, state),
        ("GET", "/api/v1/diff") => diff_api(full, state),
        // Catalog endpoints follow the session coordinate (C01), not a
        // hardcoded family.
        ("GET", "/api/v1/integrations") => json_ok(integrations_json(&coordinate.harness, true)),
        ("GET", "/api/v1/integrations/:id") | ("GET", "/api/v1/assets/:id") => {
            match family_entry(id, &coordinate.harness, true) {
                Some(v) => json_ok(v),
                None => json_err("catalog.unknown", path),
            }
        }
        ("GET", "/api/v1/assets") => {
            json_ok(assets_status(&coordinate.harness, Some(&asset_lock(&state.store))))
        }
        ("POST", "/api/v1/assets/:id/preview")
        | ("POST", "/api/v1/assets/:id/copy")
        | ("POST", "/api/v1/assets/:id/rollback") => assets_api(path, state),
        // Published so the UI renders its editor from the definition the
        // store enforces, instead of a hardcoded copy that can drift.
        ("GET", "/api/v1/settings/schema") => json_ok(ctxpect_store::settings_schema()),
        ("GET", "/api/v1/settings") => match state.store.settings() {
            Ok(v) => json_ok(v),
            Err(err) => json_err(err.code, &err.message),
        },
        ("PUT", "/api/v1/settings") | ("POST", "/api/v1/settings") => {
            let _auth = match require_mutation(state, "settings.put", "settings") {
                Ok(auth) => auth,
                Err(denied) => return denied,
            };
            let mut v = match object_body(body) {
                Ok(value) => value,
                Err(refusal) => return refusal,
            };
            // `GET /settings` decorates the document with `snapshot_digest`;
            // putting that answer back is the natural round trip, so the
            // decoration is not a field the store has to know about.
            if let Value::Object(map) = &mut v {
                map.remove("snapshot_digest");
            }
            if let Err(err) = state.store.put_settings(v.clone()) {
                return json_err(err.code, &err.message);
            }
            json_ok(v)
        }
        ("GET", "/api/v1/sessions/:id") => named_get(state, "sessions", id),
        ("GET", "/api/v1/sessions/:id/requests") => session_requests_api(state, id),
        ("GET", "/api/v1/sessions") => match state.store.list_named("sessions") {
            Ok(ids) => {
                let mut summaries = Vec::new();
                for id in &ids {
                    let session = match state.store.get_named("sessions", id) {
                        Ok(session) => session,
                        Err(err) => return json_err(err.code, &err.message),
                    };
                    summaries.push(object([
                        ("session_id", string(id)),
                        ("mapping_id", session.get("mapping_id").cloned().unwrap_or(Value::Null)),
                        ("event_count", session.get("timeline").and_then(Value::as_array)
                            .map_or(Value::Null, |events| Value::Int(events.len() as i64))),
                        ("bodies_stored", session.get("bodies_stored").cloned().unwrap_or(Value::Null)),
                        ("partial", session.get("partial").cloned().unwrap_or(Value::Null)),
                    ]));
                }
                json_ok(object([
                    ("sessions", array(ids.into_iter().map(string))),
                    ("session_summaries", array(summaries)),
                ]))
            },
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", "/api/v1/sessions/import") => sessions_import_api(body, state),
        ("GET", "/api/v1/monitor") => json_ok(monitor_status(state, query(full, "receipt_id").as_deref())),
        ("GET", "/api/v1/policy") => {
            let (action, target) = policy_query_scope(
                query(full, "action").as_deref(),
                query(full, "target").map(|t| url_decode(&t)).as_deref(),
            );
            json_ok(effective_store_policy(
                &state.store,
                state.project.as_deref(),
                &action,
                &target,
            ))
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
        // Same function the CLI's `standard status` calls: one question, one
        // answer (R04).
        ("GET", "/api/v1/standards/:id") => match standard_status(&state.store, id) {
            Ok(value) => json_ok(value),
            Err(err) => json_err(err.code(), &err.message()),
        },
        ("GET", "/api/v1/exceptions/:id") => {
            match exception_state(&state.store, state.project.as_deref(), id) {
                Ok(value) => json_ok(value),
                Err(err) => json_err(err.code(), &err.message()),
            }
        }
        ("GET", "/api/v1/standards") => match state.store.list_named("standards") {
            Ok(ids) => json_ok(object([("standards", array(ids.into_iter().map(string)))])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("GET", "/api/v1/sync") => json_ok(crate::dispatch::sync_status_doc(&state.store)),
        ("POST", "/api/v1/sync/preview") => sync_api(body, state, false),
        ("POST", "/api/v1/sync/apply") => sync_api(body, state, true),
        ("POST", "/api/v1/advisor") => advisor_api(body),
        ("GET", "/api/v1/lab/:id") => named_get(state, "experiments", id),
        // The list carries each experiment's execution state and decision, so
        // a not-executed experiment reads as such without opening it.
        ("GET", "/api/v1/lab") => match state.store.list_named("experiments") {
            Ok(ids) => json_ok(object([
                (
                    "experiments",
                    array(ids.iter().map(|id| {
                        let doc = state.store.get_named("experiments", id).unwrap_or(Value::Null);
                        object([
                            ("experiment_id", string(id)),
                            ("executed", doc.get("executed").cloned().unwrap_or(Value::Null)),
                            ("decision", doc.get("decision").cloned().unwrap_or(Value::Null)),
                            ("reason_code", doc.get("reason_code").cloned().unwrap_or(Value::Null)),
                        ])
                    })),
                ),
                ("single_ab_is_causal", Value::Bool(false)),
            ])),
            Err(err) => json_err(err.code, &err.message),
        },
        ("POST", "/api/v1/lab") => lab_api(body, state),
        ("GET", "/api/v1/team/compliance") => json_ok(team_compliance(state, query(full, "receipt_id").as_deref())),
        ("GET", "/api/v1/care-plan/:id") => care_plan_api(id, state, query(full, "receipt_id").as_deref()),
        ("POST", "/api/v1/collect") => collect_api(state),
        ("POST", "/api/v1/intent/preview") => intent_preview_api(body, state),
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
    // The daemon observes the root it was started for and its subdirectories,
    // nothing else: a page or local process reaching the loopback API must
    // not be able to point it at an arbitrary directory. Without `--project`
    // it serves records only.
    let project = match (parsed.get("project").and_then(Value::as_str), state.project.as_deref()) {
        (Some(requested), Some(root)) => match Root::new(root).and_then(|r| r.contain(requested)) {
            Ok(contained) => contained,
            Err(_) => {
                return json_err(
                    "api.project_scope",
                    "project must be the daemon's --project root or a directory inside it",
                );
            }
        },
        (None, Some(root)) => root.to_path_buf(),
        (Some(_), None) => {
            return json_err(
                "api.project_scope",
                "this daemon was started without --project and does not scan directories on request",
            );
        }
        (None, None) => return json_err("usage.invalid", "--project required"),
    };
    if let Some(requested) = parsed.get("codex_home").and_then(Value::as_str) {
        let same = state.codex_home.as_deref().is_some_and(|ours| {
            fs::canonicalize(ours).ok() == fs::canonicalize(requested).ok()
        });
        if !same {
            return json_err(
                "api.project_scope",
                "codex_home may only name the Codex home the daemon was started with (--codex-home)",
            );
        }
    }
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    // The request moves the session coordinate; fields it omits keep the
    // session's current value rather than snapping back to a hardcoded family.
    let previous = state.coordinate();
    let coordinate = Coordinate {
        harness: parsed
            .get("harness")
            .and_then(Value::as_str)
            .unwrap_or(&previous.harness)
            .to_string(),
        surface: parsed
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or(&previous.surface)
            .to_string(),
        version: parsed
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or(&previous.version)
            .to_string(),
        os_lane: parsed
            .get("os_lane")
            .and_then(Value::as_str)
            .unwrap_or(&previous.os_lane)
            .to_string(),
    };
    if let Ok(mut current) = state.coordinate.lock() {
        *current = coordinate.clone();
    }
    let args = InspectArgs {
        json: true,
        offline: true,
        project: project.clone(),
        cwd: None,
        harness: coordinate.harness.clone(),
        surface: coordinate.surface.clone(),
        version: coordinate.version.clone(),
        version_explicit: parsed.get("version").is_some(),
        codex_home: parsed
            .get("codex_home")
            .and_then(Value::as_str)
            .map(PathBuf::from),
        require: vec!["instructions".into()],
        os_lane: coordinate.os_lane.clone(),
        store: None,
    };
    match inspect(args) {
        Ok(report) => {
            let latest = state.generation.load(Ordering::SeqCst);
            let stale = latest != generation;
            match persist_inspect_in(&state.store, &report.envelope, "one-shot", Some(project.as_path())) {
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

/// Bootstrap is a read-only selection. It never moves the daemon's current
/// Receipt, and never substitutes another project's history for this root.
fn status_api(state: &AppState) -> (u16, &'static str, String) {
    let coordinate = state.coordinate();
    let matches = |receipt: &Value| {
        receipt.get("tombstone").is_none_or(|value| *value == Value::Null)
            && matches!(receipt_scope(state, receipt), ReceiptScope::Matches)
            && [
                ("harness", coordinate.harness.as_str()),
                ("version", coordinate.version.as_str()),
                ("surface", coordinate.surface.as_str()),
                ("os_lane", coordinate.os_lane.as_str()),
            ].iter().all(|(key, expected)| {
                receipt.pointer(&["coordinate", key]).and_then(Value::as_str) == Some(*expected)
            })
    };
    let (selected, selection) = match current_receipt(state) {
        Ok(receipt) if matches(&receipt) => (Some(receipt), "session-current"),
        Ok(_) => return json_err("api.receipt_scope", "current Receipt does not match the daemon coordinate"),
        Err(err) if err.code == "api.no_current_receipt" => {
            let index = match state.store.list_receipts() {
                Ok(index) => index,
                Err(err) => return json_err(err.code, &err.message),
            };
            let mut candidates = Vec::new();
            for row in index.as_array().unwrap_or(&[]) {
                let Some(id) = row.get("receipt_id").and_then(Value::as_str) else {
                    return json_err("store.index_corrupt", "Receipt index row has no id");
                };
                let receipt = match state.store.get_receipt(id) {
                    Ok(receipt) => receipt,
                    Err(err) => return json_err(err.code, &err.message),
                };
                if matches(&receipt) {
                    let Some(time) = receipt.get("created_at").and_then(Value::as_str)
                        .and_then(ctxpect_effect::epoch_seconds) else {
                        return json_err("receipt.time_invalid", "matching Receipt has an unreadable creation time");
                    };
                    let fractional = receipt.get("created_at").and_then(Value::as_str)
                        .and_then(|text| text.split_once('.'))
                        .map(|(_, tail)| tail.chars().take_while(char::is_ascii_digit).collect::<String>())
                        .unwrap_or_default().trim_end_matches('0').to_string();
                    // Fractional seconds compare lexically after removing trailing
                    // zeroes; equal observation times use the id as a stable tie.
                    candidates.push(((time, fractional, id.to_string()), receipt));
                }
            }
            let receipt = candidates.into_iter().max_by(|a, b| a.0.cmp(&b.0)).map(|(_, receipt)| receipt);
            let selection = if receipt.is_some() { "latest-matching" } else { "none" };
            (receipt, selection)
        }
        Err(err) => return json_err(err.code, &err.message),
    };
    let (summary, counts, staleness) = if let Some(receipt) = selected {
        let id = receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("");
        let diagnosis = match diagnose_with_project(state, &receipt) {
            Ok(diagnosis) => diagnosis,
            Err(err) => return err,
        };
        let monitor = monitor_status(state, Some(id));
        (
            object([
                ("receipt_id", string(id)),
                ("receipt_kind", receipt.get("receipt_kind").cloned().unwrap_or(Value::Null)),
                ("created_at", receipt.get("created_at").cloned().unwrap_or(Value::Null)),
                ("digest", receipt.pointer(&["manifest", "digest"]).cloned().unwrap_or(Value::Null)),
            ]),
            diagnosis.get("counts").cloned().unwrap_or(Value::Null),
            monitor.get("staleness").cloned().unwrap_or(Value::Null),
        )
    } else {
        (Value::Null, Value::Null, object([
            ("status", string("unknown")),
            ("reason_code", string("api.no_matching_receipt")),
        ]))
    };
    json_ok(object([
        ("schema", string("ctxpect-status-v1")),
        ("project", string(if state.project.is_some() { "<project>" } else { "unset" })),
        ("coordinate", coordinate.to_value()),
        ("generation", Value::Int(state.generation.load(Ordering::SeqCst) as i64)),
        ("daemon", object([
            ("version", string(env!("CARGO_PKG_VERSION"))),
            ("listen", string(&state.listen)),
        ])),
        ("selection", string(selection)),
        ("selected_receipt", summary),
        ("doctor_counts", counts),
        ("diagnosis_basis", string("receipt-and-current-project-scan")),
        ("staleness", staleness),
    ]))
}

fn doctor_api(full: &str, state: &AppState) -> (u16, &'static str, String) {
    let id = query(full, "receipt_id");
    let receipt = selected_receipt(state, id.as_deref());
    match receipt {
        Ok(v) => {
            let mut diagnosis = match diagnose_with_project(state, &v) {
                Ok(diagnosis) => diagnosis,
                Err(refusal) => return refusal,
            };
            // A diagnosis that does not name the observation it rests on
            // cannot be checked against that observation. The CLI reports
            // this; the API did not, so a caller using the session's current
            // Receipt had no way to tell which one that was.
            if let Value::Object(map) = &mut diagnosis {
                map.insert(
                    "receipt_id".into(),
                    v.get("receipt_id").cloned().unwrap_or(Value::Null),
                );
            }
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

/// The same diagnosis the CLI reports: Receipt-derived findings plus the
/// project-content rules over the daemon's declared project (R04).
fn diagnose_with_project(state: &AppState, receipt: &Value) -> Result<Value, (u16, &'static str, String)> {
    let base = diagnose(receipt);
    let Some(project) = state.project.as_deref() else {
        return Ok(base);
    };
    // The project rules rescan *this* root; a Receipt observed elsewhere is
    // refused, and one whose scope is unknown gets no project rules rather
    // than the daemon's root by default.
    match receipt_scope(state, receipt) {
        ReceiptScope::Matches => {}
        ReceiptScope::Mismatch => {
            return Err(json_err(
                "api.receipt_scope",
                "this Receipt was observed in another project; its diagnosis is not rescanned against this daemon's root",
            ));
        }
        ReceiptScope::Unknown => {
            let mut out = base;
            if let Value::Object(map) = &mut out {
                map.insert(
                    "project_rules".into(),
                    object([
                        ("scanned", Value::Bool(false)),
                        ("reason_code", string("api.receipt_scope_unknown")),
                        ("note", string("the Receipt carries no project scope; project-content rules were not run against this daemon's root")),
                    ]),
                );
            }
            return Ok(out);
        }
    }
    // A project that cannot be scanned yields no diagnosis, not a diagnosis
    // with the project rules quietly missing (the CLI answers the same way).
    let root = Root::new(project).map_err(|err| json_err("io.missing", &err.to_string()))?;
    crate::dispatch::diagnosis_for_root(base, &root).map_err(|err| json_err(err.code(), &err.message()))
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

/// `POST /api/v1/lab { experiment_id?, runs?: <ctxpect-effect-runs-v1> }`.
/// Same semantics as `ctxpect experiment`: no runs document → not executed,
/// nothing persisted; a runs document → judged, persisted under
/// `experiment.persist` authority.
fn lab_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let Some(runs) = parsed.get("runs") else {
        let experiment_id = parsed
            .get("experiment_id")
            .and_then(Value::as_str)
            .unwrap_or("exp-api");
        return json_ok(not_executed(experiment_id, parsed.get("n").and_then(Value::as_i64)));
    };
    let document = match RunsDocument::from_value(runs) {
        Ok(document) => document,
        Err(err) => return json_err(err.code, &err.message),
    };
    let experiment_id = parsed
        .get("experiment_id")
        .and_then(Value::as_str)
        .unwrap_or(document.contract.experiment_id.as_str())
        .to_string();
    if experiment_id != document.contract.experiment_id {
        return json_err(
            "effect.runs_invalid",
            "experiment_id does not name the experiment the runs document's contract was frozen for",
        );
    }
    if let Ok(prev) = state.store.get_named("experiments", &experiment_id)
        && let Some(prev_n) = prev.pointer(&["contract", "n_planned"]).and_then(Value::as_i64)
        && prev_n != document.contract.n_planned
    {
        return json_err(
            "effect.n_locked",
            "sample size cannot change after results are observed",
        );
    }
    // Recording an experiment result writes into the store.
    let _auth = match require_mutation(state, "experiment.persist", &experiment_id) {
        Ok(auth) => auth,
        Err(denied) => return denied,
    };
    let result = effect_decide(&document);
    if let Err(err) = state.store.put_named("experiments", &experiment_id, &result) {
        // A result the store refused to keep must not be returned as if it
        // had been recorded.
        return json_err(err.code, &err.message);
    }
    json_ok(result)
}

/// Observe the project after a mutation and return the Receipt's id.
fn observe_after_mutation(
    state: &AppState,
    project: &Path,
) -> Result<String, (u16, &'static str, String)> {
    let coordinate = state.coordinate();
    let args = InspectArgs {
        json: true,
        offline: true,
        project: project.to_path_buf(),
        cwd: None,
        harness: coordinate.harness,
        surface: coordinate.surface,
        version: coordinate.version,
        version_explicit: false,
        codex_home: None,
        require: vec!["instructions".into()],
        os_lane: coordinate.os_lane,
        store: None,
    };
    let report = inspect(args).map_err(|err| json_err(err.code(), &err.message()))?;
    let receipt = persist_inspect_in(&state.store, &report.envelope, "one-shot", Some(std::path::Path::new(project)))
        .map_err(|err| json_err(err.code(), &err.message()))?;
    Ok(receipt
        .get("receipt_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string())
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
        if !valid_tx_id(id) {
            return json_err("store.bad_id", "rollback needs a transaction id of the form tx_<16 hex>");
        }
        let _auth = match require_mutation(state, "assets.rollback", id) {
            Ok(auth) => auth,
            Err(denied) => return denied,
        };
        let backup = ctxpect_assets::backup_dir(state.store.root(), id);
        return match ctxpect_assets::rollback(&root, &backup) {
            Ok(value) => {
                let _ = state.store.audit("assets.rollback", "asset", Some(id));
                match observe_after_mutation(state, &project) {
                    Ok(post) => json_ok(merge_fields(value, [("post_receipt_id", string(&post))])),
                    Err(refusal) => refusal,
                }
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
            let _auth = match require_mutation(state, "assets.copy", id) {
                Ok(auth) => auth,
                Err(denied) => return denied,
            };
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
    let _auth = if apply {
        match require_mutation(state, "sync.apply", bundle_id) {
            Ok(auth) => Some(auth),
            Err(denied) => return denied,
        }
    } else {
        None
    };

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

/// Whether the current Receipt still describes the project on disk.
///
/// Staleness is re-derived by hashing the evidence the Receipt actually
/// declared. The endpoint previously reported `stale: false` unconditionally,
/// which is the one answer that cannot be wrong-flagged and therefore says
/// nothing.
fn monitor_status(state: &AppState, receipt_id: Option<&str>) -> Value {
    let receipt = match scoped_receipt(state, receipt_id) {
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
fn team_compliance(state: &AppState, receipt_id: Option<&str>) -> Value {
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
        let fresh = policy_fresh(&state.store, state.project.as_deref(), now);
        let Ok(status) = exception_status(&record, now, fresh) else {
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
    let receipt = selected_receipt(state, receipt_id).ok();
    let diagnosis = receipt.as_ref().and_then(|r| diagnose_with_project(state, r).ok());
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
            // Audit integrity belongs in a compliance view: a log that can be
            // edited without trace is not evidence of anything.
            "audit",
            state.store.audit_chain().map_or_else(
                |err| object([("verified", Value::Null), ("reason_code", string(err.code))]),
                |chain| {
                    object([
                        ("verified", chain.get("verified").cloned().unwrap_or(Value::Null)),
                        ("count", chain.get("count").cloned().unwrap_or(Value::Null)),
                        (
                            "legacy_entries",
                            chain.get("legacy_entries").cloned().unwrap_or(Value::Null),
                        ),
                        (
                            "reason_code",
                            chain.get("reason_code").cloned().unwrap_or(Value::Null),
                        ),
                        // A local MAC is not an organization attestation.
                        ("org_identity", Value::Bool(false)),
                    ])
                },
            ),
        ),
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
fn care_plan_api(finding_id: &str, state: &AppState, receipt_id: Option<&str>) -> (u16, &'static str, String) {
    if finding_id.is_empty() {
        return json_err("api.not_found", "care-plan requires a finding id");
    }
    let receipt = match selected_receipt(state, receipt_id) {
        Ok(receipt) => receipt,
        Err(err) => return json_err(err.code, &err.message),
    };
    let diagnosis = match diagnose_with_project(state, &receipt) {
        Ok(diagnosis) => diagnosis,
        Err(refusal) => return refusal,
    };
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

/// Import a session over the API.
///
/// The session id comes from the body or is derived from the content digest,
/// and the mapping from the body; nothing is hardcoded, so two imports land
/// as two sessions and a declared mapping is required.
fn sessions_import_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
    let parsed = match object_body(body) {
        Ok(value) => value,
        Err(refusal) => return refusal,
    };
    let mapping = parsed
        .get("mapping_id")
        .and_then(Value::as_str)
        .unwrap_or("generic-json");
    // The default id is the artifact's content digest — for a native log the
    // digest of the `jsonl` string, the same bytes the CLI hashes from the
    // file — so the same session lands under the same id from either entry.
    let session_id = parsed
        .get("session_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            let content = if mapping == deepseek_harness::MAPPING_ID {
                parsed.get("jsonl").and_then(Value::as_str).unwrap_or("")
            } else {
                body
            };
            format!("s_{}", &ctxpect_schema::sha256_text(content)[..12])
        });
    let _auth = match require_mutation(state, "sessions.import", &session_id) {
        Ok(auth) => auth,
        Err(denied) => return denied,
    };
    // A native JSONL artifact travels in `jsonl` (it is not itself a JSON
    // object); every other mapping is the JSON body.
    let native = if mapping == deepseek_harness::MAPPING_ID {
        match parsed.get("jsonl").and_then(Value::as_str) {
            Some(text) => Some(text.to_string()),
            None => {
                return json_err(
                    "import_parse_failed",
                    "mapping deepseek-harness-cli takes the session artifact as the `jsonl` string field",
                );
            }
        }
    } else {
        None
    };
    let bytes: &[u8] = native.as_deref().map_or(body.as_bytes(), str::as_bytes);
    match import_session_bytes(bytes, mapping, &session_id) {
        Ok(session) => {
            if let Err(err) = crate::dispatch::refuse_replacing_a_different_session(&state.store, &session_id, &session) {
                return json_err(err.code(), &err.message());
            }
            if let Err(err) = state.store.put_named("sessions", &session_id, &session) {
                return json_err(err.code, &err.message);
            }
            // The same derived insight the CLI writes (R04: one answer).
            let note = ctxpect_importer::insight(&session_id, "partial timeline imported; occupancy unknown");
            if let Err(err) = state.store.put_named("insights", &session_id, &note) {
                return json_err(err.code, &err.message);
            }
            json_ok(session)
        }
        Err(err) => json_err(err.code, &err.message),
    }
}

/// The intent a preview request names. `target` and `desired` are required:
/// a preview computed from silent defaults would be a persisted plan the
/// caller never asked for, and `apply` needs nothing but its `tx_id`.
fn intent_from_body(parsed: &Value) -> Result<Intent, (u16, &'static str, String)> {
    let required = |key: &str| -> Result<String, (u16, &'static str, String)> {
        parsed
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| json_err("usage.invalid", &format!("intent preview requires `{key}`")))
    };
    Ok(Intent {
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
        target_rel: required("target")?,
        desired: required("desired")?,
    })
}

/// Compute and persist a projection preview. Read-only for the project; the
/// persisted record is what `POST /api/v1/apply` is bound to.
fn intent_preview_api(body: &str, state: &AppState) -> (u16, &'static str, String) {
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
    let intent = match intent_from_body(&parsed) {
        Ok(intent) => intent,
        Err(refusal) => return refusal,
    };
    let scope = project_scope_digest(state.store.root(), Some(project.as_path()));
    if let Err(err) = target_outside_store(&root, &intent.target_rel, state.store.root()) {
        return json_err(err.code, &err.message);
    }
    match proj_preview(&root, &intent, &scope) {
        Ok(preview) => {
            if let Err(err) = persist_preview(&state.store, &preview) {
                return json_err(err.code(), &err.message());
            }
            json_ok(merge_fields(
                preview.to_value(),
                [("persisted", Value::Bool(true))],
            ))
        }
        Err(err) => json_err(err.code, &err.message),
    }
}

/// Apply a persisted preview. The body names the transaction; the intent,
/// desired bytes and frozen digests come from the store, so the apply is
/// bound to what the caller previewed and nothing else.
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
    let Some(tx) = parsed.get("tx_id").and_then(Value::as_str) else {
        return json_err(
            "usage.invalid",
            "apply requires `tx_id` from POST /api/v1/intent/preview; the preview is not recomputed",
        );
    };
    let scope = project_scope_digest(state.store.root(), Some(project.as_path()));
    // The lock is taken before the preview's state is read (see the CLI).
    let _guard = match state.store.lock_mutation() {
        Ok(guard) => guard,
        Err(err) => return json_err(err.code, &err.message),
    };
    let preview = match load_preview(&state.store, tx, &scope) {
        Ok(preview) => preview,
        Err(err) => return json_err(err.code(), &err.message()),
    };
    if let Err(err) = target_outside_store(&root, &preview.target_rel, state.store.root()) {
        return json_err(err.code, &err.message);
    }
    // Client-supplied `approved` is not authorization evidence and is ignored.
    let _auth = match require_mutation(state, "apply", &preview.target_rel) {
        Ok(auth) => auth,
        Err(denied) => return denied,
    };
    match proj_apply(
        &root,
        &preview,
        &ctxpect_projection::backup_dir(state.store.root(), &preview.tx_id),
        true,
    ) {
        Ok(v) => {
            // Recorded after the apply landed (see the CLI).
            let intent = Intent {
                intent_id: preview.intent_id.clone(),
                authority: preview.authority.clone(),
                target_rel: preview.target_rel.clone(),
                desired: preview.desired.clone(),
            };
            if let Err(err) = state
                .store
                .put_named("intents", &intent.intent_id, &intent.to_value())
            {
                return json_err(err.code, &err.message);
            }
            if let Err(err) = mark_preview(&state.store, &preview.tx_id, PREVIEW_APPLIED) {
                return json_err(err.code(), &err.message());
            }
            let _ = state.store.audit("projection.apply", "projection", Some(&preview.tx_id));
            match observe_after_mutation(state, project) {
                Ok(post) => json_ok(object([
                    ("transaction", v),
                    ("post_receipt_id", string(&post)),
                ])),
                Err(refusal) => refusal,
            }
        }
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

/// The request-evidence view of one imported session: its reconstructed
/// requests, tail and unknowns. Absent once the session is deleted.
fn session_requests_api(state: &AppState, id: &str) -> (u16, &'static str, String) {
    match state.store.get_named("sessions", id) {
        Ok(session) => json_ok(object([
            ("session_id", string(id)),
            ("mapping_id", session.get("mapping_id").cloned().unwrap_or(Value::Null)),
            ("requests", session.get("requests").cloned().unwrap_or_else(|| array([]))),
            ("tail", session.get("tail").cloned().unwrap_or(Value::Null)),
            ("unknown", session.get("unknown").cloned().unwrap_or_else(|| array([]))),
            ("partial", session.get("partial").cloned().unwrap_or(Value::Bool(true))),
            ("bodies_stored", Value::Bool(false)),
        ])),
        Err(err) => json_err(err.code, &err.message),
    }
}

fn named_get(state: &AppState, folder: &str, id: &str) -> (u16, &'static str, String) {
    match state.store.get_named(folder, id) {
        Ok(v) => json_ok(v),
        Err(err) => json_err(err.code, &err.message),
    }
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
    let tx = parsed.get("tx_id").and_then(Value::as_str).unwrap_or("");
    if !valid_tx_id(tx) {
        return json_err("store.bad_id", "rollback needs `tx_id` of the form tx_<16 hex>");
    }
    let _guard = match state.store.lock_mutation() {
        Ok(guard) => guard,
        Err(err) => return json_err(err.code, &err.message),
    };
    let backup = ctxpect_projection::backup_dir(state.store.root(), tx);
    // The transaction record names the target; authorization is bound to it.
    let target = read_tx(&backup)
        .ok()
        .and_then(|meta| meta.get("target_rel").and_then(Value::as_str).map(str::to_string))
        .or_else(|| parsed.get("target").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "AGENTS.md".to_string());
    let _auth = match require_mutation(state, "rollback", &target) {
        Ok(auth) => auth,
        Err(denied) => return denied,
    };
    let scope = project_scope_digest(state.store.root(), Some(project.as_path()));
    match proj_rollback(&root, &backup, &target, &scope) {
        // R05: a rollback changes the project, so the state after it is
        // observed rather than assumed.
        Ok(v) => {
            if let Err(err) = mark_preview(&state.store, tx, PREVIEW_ROLLED_BACK) {
                return json_err(err.code(), &err.message());
            }
            let _ = state.store.audit("projection.rollback", "projection", Some(tx));
            match observe_after_mutation(state, project) {
                Ok(post) => json_ok(merge_fields(v, [("post_receipt_id", string(&post))])),
                Err(refusal) => refusal,
            }
        }
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
    // Pass the relative part: `contain` joins a relative candidate onto the
    // canonical root itself. Passing `root.join(rel)` double-joins when the
    // configured root is relative — which is how `--ui-root packages/ui/dist`
    // stopped resolving.
    let file = match contained_root.contain(rel) {
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
                .contain("index.html")
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
