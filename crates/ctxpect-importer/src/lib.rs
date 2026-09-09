//! Session importers. Missing events stay missing; bodies stay out of the store.
//!
//! The store is metadata-only by default (settings `vault: metadata-only`),
//! and this slice has no vault, so a session record carries per-event
//! **length and digest**, never a body preview. A preview is produced only
//! when the caller asks for one and supplies the redactor that scrubs it;
//! nothing here decides on its own that a body is safe to keep.

pub mod deepseek_harness;

use ctxpect_schema::{array, object, parse, sha256_text, string, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportError {
    pub code: &'static str,
    pub message: String,
}

impl ImportError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// The declared field-to-claim mappings (`acceptance/field-to-claim/*.yaml`),
/// by file stem. An import names one of these or is refused
/// (`import.mapping_unknown`). `generic-json` is the mapping every entry
/// shares: an `events[]` array of typed items.
pub const MAPPINGS: &[&str] = &[
    "generic-json",
    "aider-cli",
    "claude-code-cli",
    "claude-code-cloud",
    "cline-cli",
    "codex-cli",
    "codex-cloud",
    "codex-desktop",
    "coze-connector",
    "cursor-agent-cli",
    "cursor-cloud",
    "cursor-ide",
    "deepseek-harness-cli",
    "gemini-cli-cli",
    "github-copilot-cli-cli",
    "goose-cli",
    "grok-build-cli",
    "grok-build-tui",
    "kimi-code-cli",
    "kiro-cli-app",
    "opencode-cli",
    "openhands-sdk",
    "qwen-code-cli",
    "windsurf-ide",
    "zcode-app",
];

/// Event types the generic mapping understands. Anything else is recorded
/// as `import_parse_failed`, not synthesized into a timeline row.
const KNOWN_EVENT_TYPES: &[&str] = &["user", "assistant", "tool", "system", "compaction"];

/// What may be kept of an event body.
#[derive(Clone, Copy)]
pub enum BodyPolicy<'a> {
    /// Length and digest only. The default, and the only policy the product
    /// uses while the vault is unimplemented.
    MetadataOnly,
    /// A short preview, passed through the caller's redactor first.
    RedactedPreview(&'a dyn Fn(&str) -> String),
}

/// Import a JSON session document using a declared mapping.
///
/// Unknown event types become `import_parse_failed` claims, not synthesized
/// timeline rows. Session-cumulative input is not occupancy.
pub fn import_session(text: &str, mapping_id: &str, session_id: &str) -> Result<Value, ImportError> {
    import_session_with(text, mapping_id, session_id, BodyPolicy::MetadataOnly)
}

/// Import raw artifact bytes. The `deepseek-harness-cli` mapping reads the
/// harness's native JSONL (see [`deepseek_harness`]); every other mapping
/// takes the generic UTF-8 JSON document.
pub fn import_session_bytes(bytes: &[u8], mapping_id: &str, session_id: &str) -> Result<Value, ImportError> {
    if mapping_id == deepseek_harness::MAPPING_ID {
        return deepseek_harness::import_deepseek_harness(bytes, session_id);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ImportError::new("import_parse_failed", "session document is not UTF-8 text"))?;
    import_session(text, mapping_id, session_id)
}

pub fn import_session_with(
    text: &str,
    mapping_id: &str,
    session_id: &str,
    bodies: BodyPolicy<'_>,
) -> Result<Value, ImportError> {
    if !MAPPINGS.contains(&mapping_id) {
        return Err(ImportError::new(
            "import.mapping_unknown",
            "mapping_id must name a declared field-to-claim mapping (acceptance/field-to-claim/*.yaml)",
        ));
    }
    let doc = parse(text).map_err(|err| ImportError::new("import_parse_failed", err.to_string()))?;
    let events = doc
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| ImportError::new("import_parse_failed", "events array required"))?;
    let mut timeline = Vec::new();
    let mut unknown = Vec::new();
    let mut input_units: i64 = 0;
    for (index, event) in events.iter().enumerate() {
        let index_value = Value::Int(i64::try_from(index).unwrap_or(0));
        let kind = event.get("type").and_then(Value::as_str);
        let Some(kind) = kind else {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("index", index_value),
            ]));
            continue;
        };
        if !KNOWN_EVENT_TYPES.contains(&kind) {
            // The unrecognised type is not echoed: it is caller-controlled
            // text and would otherwise be the one body fragment that lands
            // in the store verbatim.
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("type_len", Value::Int(i64::try_from(kind.len()).unwrap_or(0))),
                ("index", index_value),
            ]));
            continue;
        }
        let body = event.get("text").and_then(Value::as_str).unwrap_or("");
        input_units += i64::try_from(body.len()).unwrap_or(0);
        let mut row = vec![
            ("index", index_value),
            ("type", string(kind)),
            ("mapping", string(mapping_id)),
            ("body_len", Value::Int(i64::try_from(body.len()).unwrap_or(0))),
            ("body_digest", string(sha256_text(body))),
        ];
        match bodies {
            BodyPolicy::MetadataOnly => {
                row.push(("redacted_preview", Value::Null));
                row.push(("body_policy", string("metadata-only")));
            }
            BodyPolicy::RedactedPreview(redact) => {
                let preview: String = body.chars().take(40).collect();
                let preview = if body.chars().count() > 40 {
                    format!("{preview}…")
                } else {
                    preview
                };
                row.push(("redacted_preview", string(redact(&preview))));
                row.push(("body_policy", string("redacted-preview")));
            }
        }
        timeline.push(object(row));
    }
    Ok(object([
        ("schema", string("ctxpect-session-v1")),
        ("session_id", string(session_id)),
        ("mapping_id", string(mapping_id)),
        (
            "bodies_stored",
            Value::Bool(matches!(bodies, BodyPolicy::RedactedPreview(_))),
        ),
        ("timeline", array(timeline)),
        ("unknown", array(unknown)),
        ("partial", Value::Bool(true)),
        (
            "cumulative_input_units",
            Value::Int(input_units),
        ),
        (
            "cumulative_input_is_occupancy",
            Value::Bool(false),
        ),
        (
            "occupancy",
            object([
                ("status", string("unknown")),
                ("reason_code", string("current_occupancy_not_reported")),
            ]),
        ),
        (
            "invented_events",
            Value::Bool(false),
        ),
        (
            "digest",
            string(sha256_text(text)),
        ),
    ]))
}

/// Insights bound to a session id. Deleting the session must drop these.
#[must_use]
pub fn insight(session_id: &str, text: &str) -> Value {
    object([
        ("session_id", string(session_id)),
        ("text", string(text)),
        ("kind", string("historical-insight")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::canonical_json;

    #[test]
    fn does_not_invent_events_and_input_is_not_occupancy() {
        let doc = r#"{"events":[{"type":"user","text":"hi"},{"type":"mystery"}]}"#;
        let session = import_session(doc, "generic-json", "s1").unwrap();
        let timeline = session.get("timeline").and_then(Value::as_array).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(
            timeline[0].get("type").and_then(Value::as_str),
            Some("user")
        );
        assert_eq!(
            session.pointer(&["occupancy", "status"]).and_then(Value::as_str),
            Some("unknown")
        );
        assert_eq!(
            session
                .pointer(&["occupancy", "reason_code"])
                .and_then(Value::as_str),
            Some("current_occupancy_not_reported")
        );
        let unknown = session.get("unknown").and_then(Value::as_array).unwrap();
        assert_eq!(unknown.len(), 1);
    }

    #[test]
    fn metadata_only_keeps_no_body_and_unknown_types_are_not_echoed() {
        let doc = r#"{"events":[{"type":"user","text":"token sk-abcdefghijklmnopqrstuvwxyz0123 at /Users/someone/secret"},{"type":"sk-zzzzzzzzzzzzzzzzzzzzzzzz","text":"x"}]}"#;
        let session = import_session(doc, "codex-cli", "s2").unwrap();
        let text = canonical_json(&session);
        assert!(!text.contains("sk-"), "{text}");
        assert!(!text.contains("/Users/"), "{text}");
        assert_eq!(session.get("bodies_stored").and_then(Value::as_bool), Some(false));
        let timeline = session.get("timeline").and_then(Value::as_array).unwrap();
        assert_eq!(timeline[0].get("redacted_preview"), Some(&Value::Null));
        assert_eq!(timeline[0].get("body_len").and_then(Value::as_i64), Some(64));
    }

    #[test]
    fn a_preview_goes_through_the_callers_redactor() {
        let doc = r#"{"events":[{"type":"user","text":"see /Users/someone/file"}]}"#;
        let redact = |text: &str| text.replace("/Users/someone", "<home>");
        let session =
            import_session_with(doc, "generic-json", "s3", BodyPolicy::RedactedPreview(&redact))
                .unwrap();
        let timeline = session.get("timeline").and_then(Value::as_array).unwrap();
        assert_eq!(
            timeline[0].get("redacted_preview").and_then(Value::as_str),
            Some("see <home>/file")
        );
        assert_eq!(session.get("bodies_stored").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn an_undeclared_mapping_is_refused() {
        let err = import_session(r#"{"events":[]}"#, "made-up", "s4").expect_err("mapping");
        assert_eq!(err.code, "import.mapping_unknown");
    }
}
