//! AnalysisAdapter boundary. Suggestions are candidates, not claims.

use ctxpect_schema::{array, object, sha256_text, string, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisorError {
    pub code: &'static str,
    pub message: String,
}

impl AdvisorError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Produce candidates from a redacted payload after explicit consent.
pub fn suggest(
    evidence_ids: &[&str],
    payload_preview: &str,
    consent: bool,
    preview_ack: bool,
    adapter: &str,
) -> Result<Value, AdvisorError> {
    if !consent {
        return Err(AdvisorError::new(
            "advisor.consent_required",
            "refusing to send or generate without explicit consent",
        ));
    }
    if !preview_ack {
        return Err(AdvisorError::new(
            "advisor.preview_required",
            "redacted payload preview must be acknowledged",
        ));
    }
    if payload_preview.contains("/Users/") || payload_preview.contains("/home/") {
        return Err(AdvisorError::new(
            "advisor.unredacted",
            "payload still contains host home paths",
        ));
    }
    let local = adapter == "none" || adapter.is_empty();
    let candidate_id = format!(
        "c_{}",
        &sha256_text(&format!("{}|{}", evidence_ids.join(","), payload_preview))[..12]
    );
    Ok(object([
        ("schema", string("ctxpect-advisor-candidate-v1")),
        ("candidate_id", string(candidate_id)),
        ("kind", string("advisor-suggestion")),
        ("is_claim", Value::Bool(false)),
        ("enters_policy", Value::Bool(false)),
        ("enters_ci", Value::Bool(false)),
        ("enters_baseline", Value::Bool(false)),
        ("enters_reconciliation", Value::Bool(false)),
        ("unlocks_treatment", Value::Bool(false)),
        (
            "adapter",
            string(if local { "local-heuristic-not-llm" } else { adapter }),
        ),
        (
            "payload_preview",
            string(payload_preview),
        ),
        (
            "candidates",
            array([object([
                ("text", string("Collect native evidence for model-visible before any write.")),
                ("intent_hint", string("care-plan")),
            ])]),
        ),
        (
            "evidence_ids",
            array(evidence_ids.iter().map(|id| string(*id))),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_are_not_claims_and_consent_is_required() {
        let err = suggest(&["e1"], "redacted", false, true, "none").expect_err("consent");
        assert_eq!(err.code, "advisor.consent_required");
        let out = suggest(&["e1"], "redacted body", true, true, "none").unwrap();
        assert_eq!(out.get("is_claim").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("unlocks_treatment").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("enters_baseline").and_then(Value::as_bool), Some(false));
    }
}
