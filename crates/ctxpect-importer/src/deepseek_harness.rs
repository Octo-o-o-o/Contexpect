//! Native importer for DeepSeek Harness (DSH) session logs, mapping
//! `deepseek-harness-cli`.
//!
//! # Input contract
//!
//! Exactly the artifact `@deepseek-ai/dsh-session-persistence-jsonl` writes at
//! the pinned checkout (`acceptance/corpus/development/native/…/meta.json`
//! records the SHA) with `compression: 'none'` and `packChunks: false`: a
//! header line (`type: 'session'`, `version: 0`) followed by one JSON event
//! per line (`packages/session/session-persistence-jsonl/src/format.ts`,
//! `toHeaderLine` / `eventLines`). Two other encodings exist and are refused
//! rather than partially read, because reading them needs a Zstandard
//! decoder or the packed-row codec and this workspace carries no third-party
//! dependency:
//!
//! - a Zstandard frame (`.jsonl.zstd`) → `import.encoding_unsupported`
//! - packed chunk rows (`text-chunks` / `reasoning-chunks` /
//!   `tool-call-chunks`, `packages/core/session/src/chunk-rows.ts`) →
//!   `import.encoding_unsupported`
//!
//! A header with another `version` is `import.format_version_unsupported`
//! (`types.ts` `SESSION_FORMAT_VERSION`: "incompatible logs are rejected, no
//! migration").
//!
//! # What is reconstructed
//!
//! For every `request/header` event the importer rebuilds what the loop sent:
//! the canonical header (`request-header.ts` `canonicalHeader` /
//! `foldRequestHeader`) and the derived message list (`surface.ts`
//! `foldSurface` + `deriveEventMessage`, the fold `Session.deriveMessages`
//! runs in `index.ts`). Only **digests** of those leave this function: the
//! store is metadata-only, and bodies are compared in tests, in memory.
//!
//! A `request/header` proves the request was **prepared**
//! (`agent-loop/src/agent.ts:508-517` appends it before dispatch). Dispatch
//! is proven only by provider output in the same turn/step: the loop opens
//! the stream at `agent.ts:364` and appends `assistant/chunk` per chunk at
//! `agent.ts:368` (or an `assistant/message` at `agent.ts:376` / the normal
//! assembly path). `dispatch_evidence` is the seq of that event, or `null`.
//!
//! Usage is shown exactly as the provider reported it on `assistant/message`
//! (`types.ts` `SessionEventMap['assistant/message'].usage`); cumulative input
//! is not occupancy, and `request/context.contextWindow` is capacity, not use.
//!
//! Precise partial answers, each with its own reason code and each covered
//! by a red-first negative in `native_conformance.rs`:
//!
//! - `seq_discontinuity` — the seq contract (`format.ts` `consumeEventLine`)
//!   is broken; reconstruction stops at the gap and the range is recorded.
//! - `import_parse_failed` — an event type outside the pinned vocabulary
//!   (`known-event-types.ts`) without the `ignorable` marker; DSH refuses to
//!   interpret such a log (`storage-contract.ts` `validateStoredEvents`),
//!   and so does this importer from that event on.
//! - `request_prefix_incomplete` — provider output in a step that logged no
//!   `request/header`.
//! - `provenance_incomplete` — a surface replacement whose
//!   `sourceEventSeqs` do not cover the shadowed nodes or name a seq that
//!   does not exist (`surface.ts` `assertProvenance` / `replacementRange`).
//! - `TOOL_OUTCOME_UNKNOWN` / `TOOL_NOT_STARTED` — a tail turn left open; the
//!   closers `repair.ts` `interruptedTurnClosers` would synthesize are
//!   listed, never appended.

use crate::ImportError;
use ctxpect_schema::{array, canonical_json, object, parse_preserving_numbers, sha256_hex, sha256_text, string, NumberLexeme, Value};
use std::collections::BTreeMap;

/// The mapping this importer serves.
pub const MAPPING_ID: &str = "deepseek-harness-cli";
/// `SESSION_FORMAT_VERSION` at the pinned checkout (`types.ts`).
pub const DSH_FORMAT_VERSION: i64 = 0;
/// Recorded on every evidence entry so a claim names the code that read it.
pub const IMPORTER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Zstandard frame magic (RFC 8878 §3.1.1).
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
/// Packed storage rows (`chunk-rows.ts` `ChunkRow`). Not events.
const PACKED_ROW_TYPES: &[&str] = &["text-chunks", "reasoning-chunks", "tool-call-chunks"];
/// The three message-producing types (`surface.ts` `SURFACE_EVENT_TYPES`).
const SURFACE_TYPES: &[&str] = &["user/message", "assistant/message", "tool/result"];
/// Recovery codes (`repair.ts`).
pub const TOOL_OUTCOME_UNKNOWN: &str = "TOOL_OUTCOME_UNKNOWN";
pub const TOOL_NOT_STARTED: &str = "TOOL_NOT_STARTED";

/// `KNOWN_SESSION_EVENT_TYPES` at the pinned checkout
/// (`packages/core/session/src/known-event-types.ts`). An event outside this
/// set is refused unless it carries `ignorable: true`.
const KNOWN_EVENT_TYPES: &[&str] = &[
    "agent-preset/selected",
    "agent/inbox/spliced",
    "approval/asked",
    "approval/decided",
    "approval/policy",
    "assistant/chunk",
    "assistant/message",
    "command/done",
    "command/run",
    "compaction/end",
    "compaction/prune",
    "compaction/start",
    "compaction/summary",
    "feedback/record",
    "goal/change",
    "hook/invoked",
    "hook/result",
    "llm/retry",
    "llm/retry-started",
    "model/selection",
    "permission/preset",
    "plan/mode",
    "request/context",
    "request/header",
    "sandbox/mode",
    "schedule/change",
    "session-log-deepseek/delivery-accepted",
    "session/end-seed",
    "session/title",
    "session/title-llm-request",
    "step/end",
    "step/start",
    "subagent/descriptor",
    "subagent/model-selection-policy",
    "team/member",
    "team/message/delivered",
    "team/message/queued",
    "team/task",
    "todo/write",
    "tool-workflow/agent-end",
    "tool-workflow/agent-start",
    "tool-workflow/run-end",
    "tool-workflow/run-start",
    "tool/call",
    "tool/code-dispatch",
    "tool/code-dispatch-start",
    "tool/result",
    "turn/end",
    "turn/start",
    "user/message",
    "web/deepseek-search-llm-request",
];

fn err(code: &'static str, message: impl Into<String>) -> ImportError {
    ImportError {
        code,
        message: message.into(),
    }
}

fn int(n: usize) -> Value {
    Value::Int(i64::try_from(n).unwrap_or(0))
}

/// One accepted log line.
struct Event {
    seq: usize,
    kind: String,
    data: Value,
    surface_op: Option<Value>,
    /// Decoded from storage form (`seq-ranges.ts` `decodeSeqRanges`).
    source_event_seqs: Option<Vec<usize>>,
}

/// Port of `seq-ranges.ts` `decodeSeqRanges(value, maxEntries)`: a
/// storage-form list of seqs and inclusive `[start, end]` pairs, expanded.
/// `max_entries` is the current event's seq, as in DSH: a source list can
/// only name earlier events, so a range is bounded *before* it is expanded
/// and `[[0, 9223372036854775807]]` is an error, not an allocation.
fn decode_seq_ranges(value: &Value, max_entries: usize) -> Result<Vec<usize>, String> {
    let items = value.as_array().ok_or("sourceEventSeqs must be an array")?;
    let mut out = Vec::new();
    let mut has_range = false;
    for entry in items {
        if out.len() >= max_entries {
            return Err("sourceEventSeqs names more events than precede this one".into());
        }
        match entry {
            Value::Int(n) if *n >= 0 => out.push(usize::try_from(*n).unwrap_or(0)),
            Value::Array(pair) if pair.len() == 2 => {
                let (Some(start), Some(end)) = (pair[0].as_i64(), pair[1].as_i64()) else {
                    return Err("sourceEventSeqs ranges must be [start, end] pairs".into());
                };
                if start < 0 || end < start {
                    return Err("sourceEventSeqs ranges require 0 <= start <= end".into());
                }
                let span = usize::try_from(end - start).unwrap_or(usize::MAX).saturating_add(1);
                if span > max_entries - out.len() {
                    return Err("sourceEventSeqs range is longer than the events that precede this one".into());
                }
                for seq in start..=end {
                    out.push(usize::try_from(seq).unwrap_or(0));
                }
                has_range = true;
            }
            _ => return Err("sourceEventSeqs must contain non-negative integers or [start, end] pairs".into()),
        }
    }
    if has_range && !out.windows(2).all(|w| w[1] > w[0]) {
        return Err("sourceEventSeqs ranges must be strictly increasing".into());
    }
    // `surface.ts:242`: a seq cited twice is a provenance error, ranges or not.
    let mut seen = out.clone();
    seen.sort_unstable();
    if seen.windows(2).any(|w| w[0] == w[1]) {
        return Err("sourceEventSeqs cites the same seq twice".into());
    }
    Ok(out)
}

/// Parse one DSH line. A DSH log is written by `JSON.stringify`, whose
/// numbers may be non-integers (`config.temperature`), so the number-
/// preserving parser is used and every preserved lexeme must have the
/// shape `JSON.stringify` gives a double. That shape is what makes
/// re-emitting the lexeme verbatim reproduce the JS writer's bytes — and
/// therefore makes `header_digest` equal to what DSH itself would hash. A
/// number JS would never write (`1.0`, `1E5`, `1e-07`) means the line was
/// not written by DSH's serializer, and the line is refused as a parse
/// failure rather than digested under a false equivalence.
fn parse_line(line: &str) -> Result<Value, ()> {
    let value = parse_preserving_numbers(line).map_err(|_| ())?;
    fn js_shaped(value: &Value) -> bool {
        match value {
            Value::Number(lexeme) => NumberLexeme::is_js_shortest_form(lexeme),
            Value::Array(items) => items.iter().all(js_shaped),
            Value::Object(map) => map.values().all(js_shaped),
            _ => true,
        }
    }
    if js_shaped(&value) { Ok(value) } else { Err(()) }
}

/// Port of `request-header.ts` `canonicalHeader`: empty `system` and empty
/// `tools` become absent; `adapterDefaults` survives only when it asserts a
/// field. Everything else is passed through untouched.
fn canonical_header(header: &Value) -> Value {
    let mut out = BTreeMap::new();
    if let Some(config) = header.get("config") {
        out.insert("config".to_string(), config.clone());
    }
    if let Some(defaults) = header.get("adapterDefaults")
        && (defaults.get("reasoningEffort") == Some(&Value::Bool(true))
            || defaults.get("maxTokens") == Some(&Value::Bool(true)))
    {
        out.insert("adapterDefaults".to_string(), defaults.clone());
    }
    if let Some(system) = header.get("system").and_then(Value::as_str)
        && !system.is_empty()
    {
        out.insert("system".to_string(), string(system));
    }
    if let Some(tools) = header.get("tools").and_then(Value::as_array)
        && !tools.is_empty()
    {
        out.insert("tools".to_string(), Value::Array(tools.to_vec()));
    }
    Value::Object(out)
}

/// Port of `surface.ts` `deriveEventMessage`: the message a surface node
/// projects to, or `None` for an empty-content `assistant/message` (it only
/// hosts usage) and for anything that is not a surface event.
fn derive_message(event: &Event) -> Option<&Value> {
    match event.kind.as_str() {
        "user/message" => Some(&event.data),
        "assistant/message" => {
            let message = event.data.get("message")?;
            let empty = message
                .get("content")
                .and_then(Value::as_array)
                .is_none_or(<[Value]>::is_empty);
            if empty { None } else { Some(message) }
        }
        "tool/result" => event.data.get("message"),
        _ => None,
    }
}

/// Collapse an ordered seq list into inclusive `[start, end]` ranges.
fn ranges(seqs: &[usize]) -> Value {
    let mut out = Vec::new();
    let mut i = 0;
    while i < seqs.len() {
        let start = seqs[i];
        let mut end = start;
        while i + 1 < seqs.len() && seqs[i + 1] == end + 1 {
            i += 1;
            end = seqs[i];
        }
        out.push(array([int(start), int(end)]));
        i += 1;
    }
    array(out)
}

/// One request = one step (DSH's request-reconstruction theorem): the
/// messages are the surface at the `step/start` boundary; the header is the
/// latest `request/header` snapshot logged before the step's first provider
/// output. DSH appends a `request/header` only for initial / resume, a
/// header change, or a series start (`agent.ts:505-518`), so most steps in a
/// real log carry no header of their own and must not be read as "no
/// request".
struct Request {
    /// The `step/start` seq — the anchor of the request.
    seq: usize,
    turn: Option<i64>,
    step: Option<i64>,
    /// The effective `request/header` event, if any was logged in the prefix.
    header_seq: Option<usize>,
    /// Whether that header was logged inside this step (before dispatch).
    header_logged_in_step: bool,
    reason: String,
    starts_series: bool,
    header_digest: Option<String>,
    message_digests: Vec<String>,
    surface_nodes: Vec<usize>,
    replaced: Vec<Value>,
    dispatch_evidence: Option<usize>,
    usage: Option<Value>,
}

/// The latest `request/header` snapshot seen while folding.
#[derive(Clone)]
struct HeaderState {
    seq: usize,
    digest: String,
    reason: String,
    starts_series: bool,
}

/// Import a DSH JSONL session artifact. See the module docs for the contract.
pub fn import_deepseek_harness(bytes: &[u8], session_id: &str) -> Result<Value, ImportError> {
    if bytes.len() >= 4 && bytes[..4] == ZSTD_MAGIC {
        return Err(err(
            "import.encoding_unsupported",
            "this artifact is a Zstandard frame (.jsonl.zstd); only the plain JSONL encoding is readable without a third-party decoder — write the log with compression 'none'",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| err("import_parse_failed", "session artifact is not UTF-8 text"))?;
    let input_digest = sha256_hex(bytes);
    let mut lines = text.split('\n');
    let header_line = lines
        .next()
        .filter(|line| !line.trim().is_empty())
        .ok_or_else(|| err("import_parse_failed", "empty or header-less session log"))?;
    let header = parse_line(header_line).map_err(|_| err("import_parse_failed", "header line is not valid JSON"))?;
    if header.get("type").and_then(Value::as_str) != Some("session") {
        return Err(err("import_parse_failed", "first line is not a session header"));
    }
    // `format.ts` `isHeaderLine`: a header carries id, createdAt and
    // delegationDepth; DSH refuses to open a log without them.
    if header.get("id").and_then(Value::as_str).is_none()
        || header.get("createdAt").and_then(Value::as_i64).is_none()
        || header.get("delegationDepth").and_then(Value::as_i64).is_none()
    {
        return Err(err(
            "import_parse_failed",
            "session header lacks id / createdAt / delegationDepth (format.ts isHeaderLine)",
        ));
    }
    let version = header.get("version").and_then(Value::as_i64);
    if version != Some(DSH_FORMAT_VERSION) {
        let found = match header.get("version") {
            None => "<missing>".to_string(),
            Some(Value::Int(v)) => v.to_string(),
            Some(_) => "<not an integer>".to_string(),
        };
        return Err(err(
            "import.format_version_unsupported",
            format!(
                "session header version {found} is not the pinned format version {DSH_FORMAT_VERSION}; no migration is provided"
            ),
        ));
    }
    let cwd = header.get("cwd").and_then(Value::as_str).unwrap_or("");
    let dsh_id = header.get("id").and_then(Value::as_str).unwrap_or("");

    // ---- Lines → events (envelope, encoding, seq contract) ----
    let mut events: Vec<Event> = Vec::new();
    let mut timeline = Vec::new();
    let mut unknown = Vec::new();
    let mut stopped_at: Option<usize> = None;
    let mut line_no = 1usize;
    // `format.ts` reads a log line by line: a blank line inside the log is
    // corruption, and a final line without its newline is a torn tail
    // (an append that did not finish); neither is a line to skip.
    let body_lines: Vec<&str> = lines.collect();
    let torn_tail = !text.ends_with('\n');
    let last_index = body_lines.len().saturating_sub(1);
    for (index, line) in body_lines.iter().enumerate() {
        line_no += 1;
        let expected = events.len();
        if line.is_empty() && index == last_index {
            // The empty string after the final newline.
            continue;
        }
        if line.trim().is_empty() {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("line", int(line_no)),
                ("detail", string("blank line inside the log; DSH reads it as corruption and so does this importer from here on")),
            ]));
            stopped_at = Some(expected);
            break;
        }
        if torn_tail && index == last_index {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("line", int(line_no)),
                ("detail", string("the last line has no newline: a torn tail from an unfinished append, not an event")),
            ]));
            stopped_at = Some(expected);
            break;
        }
        let Ok(value) = parse_line(line) else {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("line", int(line_no)),
                ("detail", string("line is not valid JSON; reconstruction stops here")),
            ]));
            stopped_at = Some(expected);
            break;
        };
        let kind = value.get("type").and_then(Value::as_str).unwrap_or("").to_string();
        if PACKED_ROW_TYPES.contains(&kind.as_str()) {
            return Err(err(
                "import.encoding_unsupported",
                format!(
                    "line {line_no} is a packed chunk row (`{kind}`); only unpacked logs (packChunks: false) are readable without the row codec"
                ),
            ));
        }
        let seq = value.get("seq").and_then(Value::as_i64);
        let Some(seq) = seq.and_then(|s| usize::try_from(s).ok()) else {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("line", int(line_no)),
                ("detail", string("event envelope lacks a non-negative integer seq")),
            ]));
            stopped_at = Some(expected);
            break;
        };
        // `surface_op` lets a viewer separate the human transcript
        // (append-origin surface events, `surface.ts` isAppendSurfaceEvent)
        // from replacement copies without any body.
        // `surface.ts` `isReplaceOp`: exactly {op: "replace", start, end}.
        let replace_shape_ok = match value.get("surfaceOp") {
            Some(Value::Object(map)) => {
                map.len() == 3
                    && map.get("op").and_then(Value::as_str) == Some("replace")
                    && map.contains_key("start")
                    && map.contains_key("end")
            }
            _ => true,
        };
        let surface_op = match value.get("surfaceOp") {
            Some(Value::Str(op)) => string(op),
            Some(Value::Object(_)) => string("replace"),
            _ => Value::Null,
        };
        timeline.push(object([
            ("seq", int(seq)),
            ("type", string(if KNOWN_EVENT_TYPES.contains(&kind.as_str()) { kind.as_str() } else { "<unknown>" })),
            ("len", int(line.len())),
            ("digest", string(sha256_text(line))),
            ("surface_op", surface_op),
        ]));
        if seq != expected {
            unknown.push(object([
                ("reason_code", string("seq_discontinuity")),
                ("expected_seq", int(expected)),
                ("found_seq", int(seq)),
                ("line", int(line_no)),
                ("detail", string("the seq contract is broken; reconstruction stops at the gap")),
            ]));
            stopped_at = Some(expected);
            break;
        }
        if kind.is_empty() || value.get("data").is_none() || value.get("time").and_then(Value::as_i64).is_none() {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("seq", int(seq)),
                ("detail", string("event envelope lacks type, time or data")),
            ]));
            stopped_at = Some(expected);
            break;
        }
        if !KNOWN_EVENT_TYPES.contains(&kind.as_str()) {
            if value.get("ignorable") == Some(&Value::Bool(true)) {
                // The writer marked it safe to skip (`types.ts` `ignorable`).
                unknown.push(object([
                    ("reason_code", string("import_event_ignorable_skipped")),
                    ("seq", int(seq)),
                    ("type_len", int(kind.len())),
                ]));
                events.push(Event { seq, kind: "<ignorable>".into(), data: Value::Null, surface_op: None, source_event_seqs: None });
                continue;
            }
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("seq", int(seq)),
                ("type_len", int(kind.len())),
                ("detail", string("event type outside the pinned DSH vocabulary and not marked ignorable; DSH refuses to interpret such a log and so does this importer from here on")),
            ]));
            stopped_at = Some(seq);
            break;
        }
        if !replace_shape_ok {
            unknown.push(object([
                ("reason_code", string("provenance_incomplete")),
                ("seq", int(seq)),
                ("detail", string("surfaceOp object is not exactly {op: \"replace\", start, end} (surface.ts isReplaceOp)")),
            ]));
            stopped_at = Some(seq);
            break;
        }
        let source_event_seqs = match value.get("sourceEventSeqs") {
            None => None,
            Some(raw) => match decode_seq_ranges(raw, seq) {
                Ok(seqs) => Some(seqs),
                Err(detail) => {
                    unknown.push(object([
                        ("reason_code", string("provenance_incomplete")),
                        ("seq", int(seq)),
                        ("detail", string(detail)),
                    ]));
                    stopped_at = Some(seq);
                    break;
                }
            },
        };
        events.push(Event {
            seq,
            kind,
            data: value.get("data").cloned().unwrap_or(Value::Null),
            surface_op: value.get("surfaceOp").cloned(),
            source_event_seqs,
        });
    }

    // ---- Fold: surface, headers, dispatch evidence, tail ----
    let mut nodes: Vec<usize> = Vec::new();
    let mut replacements: Vec<Value> = Vec::new();
    let mut requests: Vec<Request> = Vec::new();
    let mut context_window: Option<i64> = None;
    let mut open_turn: Option<i64> = None;
    let mut open_step: Option<i64> = None;
    // callId → (step, tool/call seq), in transcript order (`repair.ts`
    // relies on Map insertion order for the closers it synthesises).
    let mut pending_calls: Vec<(String, (i64, Option<usize>))> = Vec::new();
    // (turn, step) → index into `requests`, for the step's provider output.
    let mut request_by_step: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    let mut latest_header: Option<HeaderState> = None;
    let mut prefix_incomplete: BTreeMap<(i64, i64), usize> = BTreeMap::new();

    'fold: for index in 0..events.len() {
        let event = &events[index];
        let seq = event.seq;
        let turn = event.data.get("turn").and_then(Value::as_i64);
        let step = event.data.get("step").and_then(Value::as_i64);
        match event.kind.as_str() {
            "turn/start" => {
                open_turn = turn;
                open_step = None;
                pending_calls.clear();
            }
            "turn/end" => {
                open_turn = None;
                open_step = None;
                pending_calls.clear();
            }
            "step/start" => {
                open_step = step;
                // The request this step makes: messages at this boundary,
                // header = latest snapshot (updated below if this step logs
                // its own header before dispatch).
                let mut digests = Vec::new();
                for node in &nodes {
                    if let Some(message) = events.get(*node).and_then(derive_message) {
                        digests.push(sha256_text(&canonical_json(message)));
                    }
                }
                requests.push(Request {
                    seq,
                    turn,
                    step,
                    header_seq: latest_header.as_ref().map(|h| h.seq),
                    header_logged_in_step: false,
                    reason: latest_header.as_ref().map(|h| h.reason.clone()).unwrap_or_default(),
                    starts_series: false,
                    header_digest: latest_header.as_ref().map(|h| h.digest.clone()),
                    message_digests: digests,
                    surface_nodes: nodes.clone(),
                    replaced: replacements.clone(),
                    dispatch_evidence: None,
                    usage: None,
                });
                if let (Some(t), Some(s)) = (turn, step) {
                    request_by_step.insert((t, s), requests.len() - 1);
                }
            }
            "step/end" => {
                pending_calls.clear();
                open_step = None;
            }
            "request/context" => {
                context_window = event.data.get("contextWindow").and_then(Value::as_i64);
            }
            "request/header" => {
                let header = canonical_header(event.data.get("header").unwrap_or(&Value::Null));
                let state = HeaderState {
                    seq,
                    digest: sha256_text(&canonical_json(&header)),
                    reason: header_reason(event.data.get("reason")),
                    starts_series: event.data.get("startsSeries") == Some(&Value::Bool(true)),
                };
                // A header logged inside the open step before its dispatch
                // is that step's header (`foldRequestHeader` over the prefix
                // up to the first provider output).
                if let (Some(t), Some(s)) = (open_turn, open_step)
                    && let Some(index) = request_by_step.get(&(t, s))
                    && let Some(request) = requests.get_mut(*index)
                    && request.dispatch_evidence.is_none()
                {
                    request.header_seq = Some(seq);
                    request.header_logged_in_step = true;
                    request.reason = state.reason.clone();
                    request.starts_series = state.starts_series;
                    request.header_digest = Some(state.digest.clone());
                }
                latest_header = Some(state);
            }
            "assistant/chunk" | "assistant/message" => {
                if let (Some(t), Some(s)) = (turn, step) {
                    match request_by_step.get(&(t, s)).and_then(|i| requests.get_mut(*i)) {
                        Some(request) if request.header_digest.is_some() => {
                            request.dispatch_evidence.get_or_insert(seq);
                            if event.kind == "assistant/message"
                                && let Some(usage) = event.data.get("usage")
                            {
                                request.usage = Some(usage_value(usage));
                            }
                        }
                        // Provider output with no request/header anywhere in
                        // the prefix (or no step/start): the request cannot
                        // be reconstructed.
                        _ => {
                            prefix_incomplete.entry((t, s)).or_insert(seq);
                        }
                    }
                }
            }
            "tool/call" => {
                if let Some(id) = event.data.get("callId").and_then(Value::as_str)
                    && let Some((_, entry)) = pending_calls.iter_mut().find(|(call, _)| call == id)
                {
                    entry.1 = Some(seq);
                }
            }
            _ => {}
        }
        if event.kind == "assistant/message"
            && let Some(blocks) = event.data.pointer(&["message", "content"]).and_then(Value::as_array)
        {
            for block in blocks {
                if block.get("type").and_then(Value::as_str) == Some("tool-call")
                    && let Some(id) = block.get("id").and_then(Value::as_str)
                {
                    match pending_calls.iter_mut().find(|(call, _)| call == id) {
                        Some((_, entry)) => *entry = (step.unwrap_or(0), None),
                        None => pending_calls.push((id.to_string(), (step.unwrap_or(0), None))),
                    }
                }
            }
        }
        if event.kind == "tool/result"
            && let Some(id) = event.data.pointer(&["message", "source", "callId"]).and_then(Value::as_str)
        {
            pending_calls.retain(|(call, _)| call != id);
        }

        // Surface transition (`surface.ts` planSurfaceEvent / applySurfacePlan).
        if SURFACE_TYPES.contains(&event.kind.as_str()) {
            let Some(op) = &event.surface_op else {
                unknown.push(object([
                    ("reason_code", string("provenance_incomplete")),
                    ("seq", int(seq)),
                    ("detail", string("surface-eligible event without a surfaceOp marker")),
                ]));
                stopped_at = Some(seq);
                break 'fold;
            };
            let shadowed: Vec<usize> = if op.as_str() == Some("append") {
                Vec::new()
            } else {
                let start = op.get("start").and_then(Value::as_i64).and_then(|v| usize::try_from(v).ok());
                let end = op.get("end").and_then(Value::as_i64).and_then(|v| usize::try_from(v).ok());
                let (Some(start_seq), Some(end_seq)) = (start, end) else {
                    unknown.push(object([
                        ("reason_code", string("provenance_incomplete")),
                        ("seq", int(seq)),
                        ("detail", string("replace surfaceOp lacks start/end")),
                    ]));
                    stopped_at = Some(seq);
                    break 'fold;
                };
                let start_idx = nodes.iter().position(|n| *n == start_seq);
                let end_idx = nodes.iter().position(|n| *n == end_seq);
                match (start_idx, end_idx) {
                    (Some(a), Some(b)) if a <= b => nodes[a..=b].to_vec(),
                    _ => {
                        unknown.push(object([
                            ("reason_code", string("provenance_incomplete")),
                            ("seq", int(seq)),
                            ("detail", string("replace range names seqs that are not current surface nodes")),
                        ]));
                        stopped_at = Some(seq);
                        break 'fold;
                    }
                }
            };
            // Provenance (`assertProvenance`): every cited seq exists and is
            // earlier; a replacement cites every shadowed node.
            if let Some(sources) = &event.source_event_seqs
                && (sources.iter().any(|s| *s >= seq) || (sources.is_empty() && event.kind != "assistant/message"))
            {
                unknown.push(object([
                    ("reason_code", string("provenance_incomplete")),
                    ("seq", int(seq)),
                    ("detail", string("sourceEventSeqs must reference earlier events (and may be empty only on assistant/message)")),
                ]));
                stopped_at = Some(seq);
                break 'fold;
            }
            let cited: Vec<usize> = event.source_event_seqs.clone().unwrap_or_default();
            if let Some(missing) = shadowed.iter().find(|s| !cited.contains(s)) {
                unknown.push(object([
                    ("reason_code", string("provenance_incomplete")),
                    ("seq", int(seq)),
                    ("missing_source_seq", int(*missing)),
                    ("detail", string("surface replace: sourceEventSeqs must include every shadowed surface node")),
                ]));
                stopped_at = Some(seq);
                break 'fold;
            }
            if event.kind == "tool/result" && op.as_str() != Some("append") && shadowed.len() != 1 {
                unknown.push(object([
                    ("reason_code", string("provenance_incomplete")),
                    ("seq", int(seq)),
                    ("detail", string("tool/result surface replacement must rewrite exactly one current node")),
                ]));
                stopped_at = Some(seq);
                break 'fold;
            }
            if op.as_str() == Some("append") {
                nodes.push(seq);
            } else {
                let a = nodes.iter().position(|n| Some(*n) == shadowed.first().copied()).unwrap_or(0);
                nodes.splice(a..a + shadowed.len(), [seq]);
                replacements.push(object([
                    ("seq", int(seq)),
                    ("start", op.get("start").cloned().unwrap_or(Value::Null)),
                    ("end", op.get("end").cloned().unwrap_or(Value::Null)),
                    ("shadowed", array(shadowed.iter().map(|s| int(*s)))),
                ]));
            }
        }
    }
    for ((t, s), seq) in &prefix_incomplete {
        unknown.push(object([
            ("reason_code", string("request_prefix_incomplete")),
            ("seq", int(*seq)),
            ("turn", Value::Int(*t)),
            ("step", Value::Int(*s)),
            ("detail", string("provider output with no request/header anywhere in the log prefix (or outside any step); the request this output answers cannot be reconstructed")),
        ]));
    }

    // ---- Tail (`repair.ts` interruptedTurnClosers), listed not appended ----
    let mut closers = Vec::new();
    if open_turn.is_some() && !events.is_empty() {
        for (_, (step, call_seq)) in &pending_calls {
            closers.push(object([
                ("type", string("tool/result")),
                ("error_code", string(if call_seq.is_some() { TOOL_OUTCOME_UNKNOWN } else { TOOL_NOT_STARTED })),
                ("step", Value::Int(*step)),
                ("call_seq", call_seq.map_or(Value::Null, int)),
            ]));
        }
        if let Some(step) = open_step {
            closers.push(object([("type", string("step/end")), ("step", Value::Int(step))]));
        }
        closers.push(object([("type", string("turn/end")), ("reason", string("interrupted"))]));
    }
    let interrupted = !closers.is_empty();

    // ---- Requests, claims, evidence ----
    let evidence_id = |seq: usize| sha256_text(&format!("{input_digest}:{seq}"));
    let mut request_docs = Vec::new();
    let mut claims = Vec::new();
    let mut evidence = Vec::new();
    let mut cumulative_input: i64 = 0;
    for request in &requests {
        // A step that never had a header is not a request DSH could have
        // dispatched; it is listed with `header_digest: null` and no claims.
        let Some(header_seq) = request.header_seq else {
            request_docs.push(object([
                ("seq", int(request.seq)),
                ("turn", request.turn.map_or(Value::Null, Value::Int)),
                ("step", request.step.map_or(Value::Null, Value::Int)),
                ("header_seq", Value::Null),
                ("header_logged_in_step", Value::Bool(false)),
                ("reason", Value::Null),
                ("header_digest", Value::Null),
                ("message_count", int(request.message_digests.len())),
                ("dispatch_evidence", request.dispatch_evidence.map_or(Value::Null, int)),
                ("unknown_reasons", array([string("no request/header in the log prefix; the step cannot be read as a prepared request")])),
            ]));
            continue;
        };
        let header_evidence = evidence_id(header_seq);
        evidence.push(evidence_entry(&header_evidence, header_seq, "request/header", &timeline));
        let dispatch_evidence = request.dispatch_evidence.map(|seq| {
            let id = evidence_id(seq);
            let kind = events.get(seq).map_or("", |e| e.kind.as_str()).to_string();
            evidence.push(evidence_entry(&id, seq, &kind, &timeline));
            (seq, id)
        });
        if let Some(usage) = &request.usage {
            cumulative_input = cumulative_input
                .saturating_add(usage.get("input_tokens").and_then(Value::as_i64).unwrap_or(0).max(0));
        }
        let mut reasons = Vec::new();
        if dispatch_evidence.is_none() {
            reasons.push(string("runtime_snapshot_missing: no provider output in this turn/step, so dispatch is not evidenced; the request is prepared only"));
        }
        if stopped_at.is_some_and(|stop| request.seq >= stop) {
            reasons.push(string("reconstruction stopped before this request"));
        }
        request_docs.push(object([
            ("seq", int(request.seq)),
            ("turn", request.turn.map_or(Value::Null, Value::Int)),
            ("step", request.step.map_or(Value::Null, Value::Int)),
            ("header_seq", int(header_seq)),
            ("header_logged_in_step", Value::Bool(request.header_logged_in_step)),
            ("reason", string(&request.reason)),
            ("starts_series", Value::Bool(request.starts_series)),
            ("header_digest", string(request.header_digest.as_deref().unwrap_or(""))),
            ("message_count", int(request.message_digests.len())),
            ("message_digests", array(request.message_digests.iter().map(string))),
            ("surface_nodes", array(request.surface_nodes.iter().map(|s| int(*s)))),
            ("source_seq_ranges", ranges(&request.surface_nodes)),
            ("replaced_ranges", array(request.replaced.clone())),
            ("dispatch_evidence", dispatch_evidence.as_ref().map_or(Value::Null, |(seq, _)| int(*seq))),
            ("prepared_evidence_id", string(&header_evidence)),
            ("dispatch_evidence_id", dispatch_evidence.as_ref().map_or(Value::Null, |(_, id)| string(id))),
            ("usage", request.usage.clone().unwrap_or(Value::Null)),
            ("unknown_reasons", array(reasons)),
        ]));
        // prepared: the header was logged before dispatch (agent.ts:508-517).
        claims.push(claim(
            request.seq,
            "eligible",
            "present",
            None,
            &header_evidence,
            "request/header logged before dispatch — the request was prepared",
        ));
        // model-visible: only provider output proves the request reached the model.
        match &dispatch_evidence {
            Some((_, id)) => claims.push(claim(
                request.seq,
                "model-visible",
                "present",
                None,
                id,
                "assistant/chunk or assistant/message in the same turn/step (agent.ts:364-368) — the derived surface was dispatched",
            )),
            None => claims.push(claim(
                request.seq,
                "model-visible",
                "indeterminate",
                Some("runtime_snapshot_missing"),
                &header_evidence,
                "no provider output for this request; a header alone proves preparation, not dispatch",
            )),
        }
        for stage in ["use-evidence", "outcome-affecting"] {
            claims.push(claim(
                request.seq,
                stage,
                "indeterminate",
                Some("runtime_snapshot_missing"),
                &header_evidence,
                "not covered by the session log (field-to-claim: Unknown)",
            ));
        }
    }

    let reconstruction_complete = stopped_at.is_none();
    let has_defects = unknown.iter().any(|item| {
        item.get("reason_code").and_then(Value::as_str) != Some("import_event_ignorable_skipped")
    });
    Ok(object([
        ("schema", string("ctxpect-session-v1")),
        ("session_id", string(session_id)),
        ("mapping_id", string(MAPPING_ID)),
        (
            "format",
            object([
                ("harness", string("deepseek-harness")),
                ("version", Value::Int(DSH_FORMAT_VERSION)),
                ("encoding", string("jsonl")),
                ("compression", string("none")),
                ("packed_chunks", Value::Bool(false)),
                ("importer_version", string(IMPORTER_VERSION)),
            ]),
        ),
        (
            "header",
            object([
                ("dsh_session_id_digest", string(sha256_text(dsh_id))),
                ("created_at", int_only(header.get("createdAt"))),
                ("cwd_len", int(cwd.len())),
                ("cwd_digest", string(sha256_text(cwd))),
                ("is_seeded", Value::Bool(header.get("seedLength").is_some())),
                ("delegation_depth", int_only(header.get("delegationDepth"))),
            ]),
        ),
        ("event_count", int(events.len())),
        ("timeline", array(timeline)),
        ("requests", array(request_docs)),
        ("final_surface", array(nodes.iter().map(|s| int(*s)))),
        ("replacements", array(replacements)),
        ("claims", array(claims)),
        ("evidence", array(evidence)),
        ("unknown", array(unknown)),
        (
            "reconstruction",
            object([
                ("complete", Value::Bool(reconstruction_complete)),
                ("stopped_at_seq", stopped_at.map_or(Value::Null, int)),
            ]),
        ),
        ("partial", Value::Bool(!reconstruction_complete || has_defects || interrupted)),
        (
            "tail",
            object([
                ("interrupted", Value::Bool(interrupted)),
                ("open_turn", open_turn.map_or(Value::Null, Value::Int)),
                ("open_step", open_step.map_or(Value::Null, Value::Int)),
                ("closers", array(closers)),
                ("closers_appended", Value::Bool(false)),
            ]),
        ),
        (
            "usage",
            object([
                ("provider_reported", Value::Bool(true)),
                ("cumulative_input_tokens", Value::Int(cumulative_input)),
                ("cumulative_input_is_occupancy", Value::Bool(false)),
                ("context_window", context_window.map_or(Value::Null, Value::Int)),
                ("context_window_is_used", Value::Bool(false)),
            ]),
        ),
        (
            "occupancy",
            object([
                ("status", string("unknown")),
                ("reason_code", string("current_occupancy_not_reported")),
            ]),
        ),
        ("bodies_stored", Value::Bool(false)),
        ("invented_events", Value::Bool(false)),
        ("digest", string(&input_digest)),
    ]))
}

/// The `reason` a request/header carries, from DSH's closed vocabulary
/// (`initial`, `resume`, `change`, `series`). Anything else is recorded by
/// length only: the record is metadata-only and a free string is a body.
fn header_reason(value: Option<&Value>) -> String {
    match value.and_then(Value::as_str) {
        Some(reason @ ("initial" | "resume" | "change" | "series")) => reason.to_string(),
        Some(other) => format!("unknown:len={}", other.len()),
        None => String::new(),
    }
}

/// A field that must be an integer in the persisted record; any other JSON
/// value (a string, an object carrying text) is dropped to `null` rather
/// than copied through.
fn int_only(value: Option<&Value>) -> Value {
    match value {
        Some(Value::Int(n)) => Value::Int(*n),
        _ => Value::Null,
    }
}

fn usage_value(usage: &Value) -> Value {
    let field = |camel: &str| int_only(usage.get(camel));
    object([
        ("input_tokens", field("inputTokens")),
        ("output_tokens", field("outputTokens")),
        ("total_tokens", field("totalTokens")),
        ("cache_read_tokens", field("cacheReadTokens")),
        ("cache_write_tokens", field("cacheWriteTokens")),
        ("reasoning_tokens", field("reasoningTokens")),
        ("source", string("provider-reported")),
    ])
}

/// The minimal evidence record (C19 subset): id, collection method, importer
/// version, the event it names and that line's digest. Fields this importer
/// cannot fill are `unknown`, not invented.
fn evidence_entry(id: &str, seq: usize, kind: &str, timeline: &[Value]) -> Value {
    let digest = timeline
        .get(seq)
        .and_then(|row| row.get("digest"))
        .cloned()
        .unwrap_or(Value::Null);
    object([
        ("evidence_id", string(id)),
        ("collection", string("import")),
        ("importer_version", string(IMPORTER_VERSION)),
        ("mapping_id", string(MAPPING_ID)),
        ("seq", int(seq)),
        ("event_type", string(kind)),
        ("content_digest", digest),
        ("collected_at", string("unknown")),
        ("quality", string("unknown")),
        ("root", Value::Null),
        ("path", Value::Null),
    ])
}

fn claim(seq: usize, stage: &str, truth: &str, reason: Option<&str>, evidence_id: &str, note: &str) -> Value {
    object([
        ("capability_id", string("session-request")),
        ("request_seq", int(seq)),
        ("lifecycle_stage", string(stage)),
        // field-to-claim: outcome-affecting is an effect claim (it needs an
        // Effect Lab contract, never a log); the other stages are observed.
        ("claim_kind", string(if stage == "outcome-affecting" { "effect" } else { "observed" })),
        ("truth_state", string(truth)),
        ("provenance", string("native-log")),
        (
            "coverage",
            string(if truth == "present" { "full-declared-surface" } else { "unknown" }),
        ),
        ("precision", string(if truth == "present" { "exact" } else { "not-applicable" })),
        ("knowledge_status", string(if truth == "present" { "current" } else { "unknown" })),
        ("unknown_reason_code", reason.map_or(Value::Null, string)),
        ("evidence_id", string(evidence_id)),
        ("note", string(note)),
    ])
}
