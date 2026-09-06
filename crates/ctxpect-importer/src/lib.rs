//! Session importers. Missing events stay missing.

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

/// Import a JSON session document using a declared mapping.
///
/// Unknown event types become `import_parse_failed` claims, not synthesized
/// timeline rows. Session-cumulative input is not occupancy.
pub fn import_session(text: &str, mapping_id: &str, session_id: &str) -> Result<Value, ImportError> {
    let doc = parse(text).map_err(|err| ImportError::new("import_parse_failed", err.to_string()))?;
    let events = doc
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| ImportError::new("import_parse_failed", "events array required"))?;
    let mut timeline = Vec::new();
    let mut unknown = Vec::new();
    let mut input_units: i64 = 0;
    for (index, event) in events.iter().enumerate() {
        let kind = event.get("type").and_then(Value::as_str);
        let Some(kind) = kind else {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("index", Value::Int(i64::try_from(index).unwrap_or(0))),
            ]));
            continue;
        };
        if !matches!(kind, "user" | "assistant" | "tool" | "system" | "compaction") {
            unknown.push(object([
                ("reason_code", string("import_parse_failed")),
                ("type", string(kind)),
                ("index", Value::Int(i64::try_from(index).unwrap_or(0))),
            ]));
            continue;
        }
        let body = event.get("text").and_then(Value::as_str).unwrap_or("");
        input_units += i64::try_from(body.len()).unwrap_or(0);
        timeline.push(object([
            ("index", Value::Int(i64::try_from(index).unwrap_or(0))),
            ("type", string(kind)),
            ("redacted_preview", string(redact_preview(body))),
            ("mapping", string(mapping_id)),
        ]));
    }
    Ok(object([
        ("schema", string("ctxpect-session-v1")),
        ("session_id", string(session_id)),
        ("mapping_id", string(mapping_id)),
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

fn redact_preview(body: &str) -> String {
    let trimmed: String = body.chars().take(40).collect();
    if body.chars().count() > 40 {
        format!("{trimmed}…")
    } else {
        trimmed
    }
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
}
