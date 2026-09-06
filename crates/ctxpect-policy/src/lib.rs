//! Five-layer context policy. Package/SBOM remains APM's unique authority.

use ctxpect_schema::{array, object, string, Value};

pub const LAYERS: &[&str] = &[
    "organization",
    "team",
    "project",
    "user",
    "session",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyError {
    pub code: &'static str,
    pub message: String,
}

impl PolicyError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Evaluate stacked layers. Lower layers cannot relax a required deny.
pub fn evaluate(layers: &Value, now: i64) -> Result<Value, PolicyError> {
    let items = layers
        .as_array()
        .ok_or_else(|| PolicyError::new("policy.layers_array", "layers must be an array"))?;
    let mut effective_required: Vec<Value> = Vec::new();
    let mut detect_only = Vec::new();
    let mut authority = "none";
    let mut reasons = Vec::new();

    for (index, layer) in items.iter().enumerate() {
        let name = layer
            .get("layer")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !LAYERS.contains(&name) {
            return Err(PolicyError::new(
                "policy.layer_unknown",
                format!("layer `{name}` is not one of the five context layers"),
            ));
        }
        if index > 0 {
            let prev = items[index - 1]
                .get("layer")
                .and_then(Value::as_str)
                .unwrap_or("");
            let prev_rank = LAYERS.iter().position(|item| *item == prev).unwrap_or(0);
            let rank = LAYERS.iter().position(|item| *item == name).unwrap_or(0);
            if rank < prev_rank {
                return Err(PolicyError::new(
                    "policy.layer_order",
                    "layers must be organization → team → project → user → session",
                ));
            }
        }
        let mode = layer
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("enforceable");
        let rules = layer.get("rules").and_then(Value::as_array).unwrap_or(&[]);
        for rule in rules {
            let required = rule
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let effect = rule.get("effect").and_then(Value::as_str).unwrap_or("allow");
            if required && effect == "deny" {
                if mode == "detect-only" {
                    detect_only.push(rule.clone());
                    reasons.push(object([
                        ("code", string("policy.detect_only_not_enforceable")),
                        ("layer", string(name)),
                    ]));
                } else {
                    if lower_relaxes(&effective_required, rule, name) {
                        return Err(PolicyError::new(
                            "policy.personal_cannot_relax_required",
                            "user/session layers cannot relax a required deny from a higher layer",
                        ));
                    }
                    effective_required.push(with_layer(rule, name));
                    authority = name;
                }
            } else if mode == "detect-only" {
                detect_only.push(rule.clone());
            } else if required {
                if lower_relaxes(&effective_required, rule, name) {
                    return Err(PolicyError::new(
                        "policy.personal_cannot_relax_required",
                        "user/session layers cannot relax a required deny from a higher layer",
                    ));
                }
                effective_required.push(with_layer(rule, name));
                authority = name;
            }
        }
    }

    let mut verdict = "pass";
    if !effective_required.is_empty() {
        verdict = "deny";
    }
    if verdict == "pass" && !detect_only.is_empty() {
        verdict = "detect-only";
    }

    let _ = now;
    Ok(object([
        ("schema", string("ctxpect-policy-eval-v1")),
        ("verdict", string(verdict)),
        ("enforceable", Value::Bool(verdict == "deny" || verdict == "pass")),
        (
            "detect_only_is_enforceable",
            Value::Bool(false),
        ),
        ("unique_authority", string(authority)),
        ("required_rules", array(effective_required)),
        ("detect_only_rules", array(detect_only)),
        ("reasons", array(reasons)),
        (
            "package_authority",
            string("apm-unique-not-re-evaluated"),
        ),
    ]))
}

fn with_layer(rule: &Value, layer: &str) -> Value {
    let mut map = match rule {
        Value::Object(map) => map.clone(),
        _ => Default::default(),
    };
    map.insert("layer".to_string(), string(layer));
    Value::Object(map)
}

fn lower_relaxes(existing: &[Value], incoming: &Value, layer: &str) -> bool {
    if layer != "user" && layer != "session" {
        return false;
    }
    let incoming_id = incoming.get("id").and_then(Value::as_str);
    let incoming_effect = incoming.get("effect").and_then(Value::as_str);
    existing.iter().any(|rule| {
        rule.get("id").and_then(Value::as_str) == incoming_id
            && rule.get("effect").and_then(Value::as_str) == Some("deny")
            && incoming_effect == Some("allow")
    })
}

/// Authorize a mutation. Missing policy, deny, detect-only, indeterminate,
/// and Unknown are fail-closed. A `pass` evaluation still requires a live
/// approved exception; client-attested `approved` flags are not an input.
pub fn authorize_mutation(
    layers: Option<&Value>,
    exceptions: &[Value],
    now: i64,
    offline_fresh: bool,
) -> Result<Value, PolicyError> {
    let Some(layers) = layers else {
        return Err(PolicyError::new(
            "policy.unknown",
            "no policy layers are present; mutation is fail-closed",
        ));
    };
    let items = layers
        .as_array()
        .ok_or_else(|| PolicyError::new("policy.layers_array", "layers must be an array"))?;
    if items.is_empty() {
        return Err(PolicyError::new(
            "policy.unknown",
            "policy layers are empty; mutation is fail-closed",
        ));
    }
    let evaluation = evaluate(layers, now)?;
    let verdict = evaluation
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("indeterminate");
    match verdict {
        "deny" => {
            return Err(PolicyError::new(
                "policy.denied",
                "policy verdict is deny; mutation is fail-closed",
            ));
        }
        "detect-only" => {
            return Err(PolicyError::new(
                "policy.detect_only_not_enforceable",
                "detect-only is not an allow for mutation",
            ));
        }
        "pass" => {}
        other => {
            return Err(PolicyError::new(
                "policy.indeterminate",
                format!("policy verdict `{other}` is not an allow; mutation is fail-closed"),
            ));
        }
    }

    let mut granted: Option<(Value, Value)> = None;
    for record in exceptions {
        let status = exception_status(record, now, offline_fresh)?;
        if status.get("grants").and_then(Value::as_bool) == Some(true) {
            granted = Some((record.clone(), status));
            break;
        }
    }
    let Some((record, status)) = granted else {
        return Err(PolicyError::new(
            "policy.approval_required",
            "apply requires a live approved exception; Unknown/absent approval is not an allow",
        ));
    };
    Ok(object([
        ("allowed", Value::Bool(true)),
        ("evaluation", evaluation),
        ("exception", record),
        ("exception_status", status),
    ]))
}

/// Exception lifecycle. Expired or offline-stale exceptions do not grant.
pub fn exception_status(record: &Value, now: i64, offline_fresh: bool) -> Result<Value, PolicyError> {
    let state = record.get("state").and_then(Value::as_str).unwrap_or("requested");
    let expires = record.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
    let mut live = state.to_string();
    let mut grant = false;
    let mut reason = "ok";
    if state == "approved" {
        if expires > 0 && now >= expires {
            live = "expired".to_string();
            reason = "exception.expired";
        } else if !offline_fresh {
            live = "stale".to_string();
            reason = "exception.offline_stale";
        } else {
            grant = true;
        }
    }
    if state == "revoked" {
        grant = false;
        reason = "exception.revoked";
    }
    if state == "rejected" {
        grant = false;
        reason = "exception.rejected";
    }
    Ok(object([
        ("state", string(live)),
        ("grants", Value::Bool(grant)),
        ("reason_code", string(reason)),
        (
            "reuse_of_expired",
            Value::Bool(false),
        ),
    ]))
}

pub fn new_exception(id: &str, requester: &str, scope: &str, expires_at: i64) -> Value {
    object([
        ("exception_id", string(id)),
        ("requester", string(requester)),
        ("scope", string(scope)),
        ("state", string("requested")),
        ("expires_at", Value::Int(expires_at)),
        ("approver", Value::Null),
    ])
}

pub fn transition(
    record: &Value,
    actor: &str,
    actor_role: &str,
    next: &str,
) -> Result<Value, PolicyError> {
    let _ = (record, actor, actor_role, next);
    Err(PolicyError::new(
        "exception.identity_source_uncovered",
        "caller-attested role is not authorization evidence; this slice has no enrolled principal source, so approve/reject/revoke is fail-closed",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    #[test]
    fn personal_cannot_relax_required_and_detect_only_is_not_enforceable() {
        let layers = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"deny"}]},{"layer":"user","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"allow"}]}]"#,
        )
        .unwrap();
        let err = evaluate(&layers, 1).expect_err("relax");
        assert_eq!(err.code, "policy.personal_cannot_relax_required");

        let detect = parse(
            r#"[{"layer":"organization","mode":"detect-only","rules":[{"id":"r1","required":true,"effect":"deny"}]}]"#,
        )
        .unwrap();
        let result = evaluate(&detect, 1).unwrap();
        assert_eq!(result.get("verdict").and_then(Value::as_str), Some("detect-only"));
        assert_eq!(
            result
                .get("detect_only_is_enforceable")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn expired_and_stale_exceptions_do_not_grant() {
        let rec = parse(r#"{"state":"approved","expires_at":10}"#).unwrap();
        let expired = exception_status(&rec, 11, true).unwrap();
        assert_eq!(expired.get("grants").and_then(Value::as_bool), Some(false));
        assert_eq!(
            expired.get("reason_code").and_then(Value::as_str),
            Some("exception.expired")
        );
        let stale = exception_status(&rec, 1, false).unwrap();
        assert_eq!(stale.get("grants").and_then(Value::as_bool), Some(false));
        assert_eq!(
            stale.get("reason_code").and_then(Value::as_str),
            Some("exception.offline_stale")
        );
    }

    #[test]
    fn mutation_is_fail_closed_without_layers_or_live_exception() {
        let err = authorize_mutation(None, &[], 1, true).expect_err("unknown");
        assert_eq!(err.code, "policy.unknown");

        let empty = parse("[]").unwrap();
        let err = authorize_mutation(Some(&empty), &[], 1, true).expect_err("empty");
        assert_eq!(err.code, "policy.unknown");

        let deny = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"deny"}]}]"#,
        )
        .unwrap();
        let err = authorize_mutation(Some(&deny), &[], 1, true).expect_err("deny");
        assert_eq!(err.code, "policy.denied");

        let detect = parse(
            r#"[{"layer":"organization","mode":"detect-only","rules":[{"id":"r1","required":true,"effect":"deny"}]}]"#,
        )
        .unwrap();
        let err = authorize_mutation(Some(&detect), &[], 1, true).expect_err("detect");
        assert_eq!(err.code, "policy.detect_only_not_enforceable");

        let pass = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]"#,
        )
        .unwrap();
        let err = authorize_mutation(Some(&pass), &[], 1, true).expect_err("approval");
        assert_eq!(err.code, "policy.approval_required");

        let expired = parse(r#"{"state":"approved","expires_at":10}"#).unwrap();
        let err = authorize_mutation(Some(&pass), &[expired], 11, true).expect_err("expired");
        assert_eq!(err.code, "policy.approval_required");

        let live = parse(r#"{"state":"approved","expires_at":99,"exception_id":"ex-1"}"#).unwrap();
        let ok = authorize_mutation(Some(&pass), &[live], 1, true).unwrap();
        assert_eq!(ok.get("allowed").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn caller_attested_role_cannot_approve() {
        let rec = parse(r#"{"state":"requested","exception_id":"ex-1"}"#).unwrap();
        let err = transition(&rec, "same-user", "lead", "approved").expect_err("identity");
        assert_eq!(err.code, "exception.identity_source_uncovered");
        assert_eq!(rec.get("state").and_then(Value::as_str), Some("requested"));
    }
}
