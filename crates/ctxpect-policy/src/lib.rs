//! Five-layer context policy. Package/SBOM remains APM's unique authority.

use ctxpect_schema::{array, object, string, Value};

pub mod precondition;
pub mod principal;

pub use precondition::{
    evaluate as evaluate_preconditions, requires as precondition_required, ActionClass,
    Precondition, PreconditionReport,
};
pub use principal::{enrollment_digest, verify_principal, PrincipalProof, REGISTRY_SCHEMA};

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
    let mut required_allow: Vec<Value> = Vec::new();
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
                // A required rule whose effect is `allow` is a mandatory
                // permission, not a prohibition. It must not enter the deny
                // set, or the verdict inverts the rule it came from.
                required_allow.push(with_layer(rule, name));
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
        ("required_allow_rules", array(required_allow)),
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

/// What a mutation is about to do. Every field takes part in the covering
/// check: an exception approved for one project, one action and one target
/// does not authorize another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationScope {
    /// The mutation kind, e.g. `apply`, `rollback`, `assets.copy`,
    /// `sessions.import`, `standard.publish`.
    pub action: String,
    /// Digest of the project (or, without a project, the store) the mutation
    /// touches. Never the path itself: the record is publishable.
    pub project_digest: String,
    /// The concrete target inside that action: a project-relative path, an
    /// asset id, a session id, a standard id. `*` in a record covers any.
    pub target: String,
}

impl MutationScope {
    #[must_use]
    pub fn new(action: &str, project_digest: &str, target: &str) -> Self {
        Self {
            action: action.to_string(),
            project_digest: project_digest.to_string(),
            target: target.to_string(),
        }
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        object([
            ("action", string(&self.action)),
            ("project_digest", string(&self.project_digest)),
            ("target", string(&self.target)),
        ])
    }
}

/// The target wildcard an exception may carry. Actions and projects have no
/// wildcard: an exception is always for one action in one project.
pub const ANY_TARGET: &str = "*";

/// How long a policy source stays fresh without being re-read from its
/// authority, in seconds. Beyond this an approved exception is `stale` and
/// does not grant (`exception.offline_stale`).
pub const POLICY_FRESHNESS_SECS: i64 = 30 * 86_400;

/// Whether `record` covers `scope`. `Err` carries the reason code.
///
/// A record without scope fields is a pre-scope record and covers nothing:
/// widening it to "everything" would be exactly the defect this replaces.
pub fn exception_covers(record: &Value, scope: &MutationScope) -> Result<(), &'static str> {
    let action = record.get("action").and_then(Value::as_str);
    let project = record.get("project_digest").and_then(Value::as_str);
    let target = record.get("target").and_then(Value::as_str);
    let (Some(action), Some(project), Some(target)) = (action, project, target) else {
        return Err("exception.scope_missing");
    };
    if action.is_empty() || project.is_empty() || target.is_empty() {
        return Err("exception.scope_missing");
    }
    if action == ANY_TARGET || project == ANY_TARGET {
        return Err("exception.scope_wildcard_forbidden");
    }
    if action != scope.action {
        return Err("exception.action_mismatch");
    }
    if project != scope.project_digest {
        return Err("exception.project_mismatch");
    }
    if target != ANY_TARGET && target != scope.target {
        return Err("exception.target_mismatch");
    }
    Ok(())
}

/// Authorize a mutation. Missing policy, deny, detect-only, indeterminate,
/// and Unknown are fail-closed. A `pass` evaluation still requires a live
/// approved exception **that covers `scope`**; client-attested `approved`
/// flags are not an input.
pub fn authorize_mutation(
    layers: Option<&Value>,
    exceptions: &[Value],
    now: i64,
    offline_fresh: bool,
    scope: &MutationScope,
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
    let mut live_but_not_covering = 0usize;
    let mut refusals: Vec<&'static str> = Vec::new();
    for record in exceptions {
        let status = exception_status(record, now, offline_fresh)?;
        if status.get("grants").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        match exception_covers(record, scope) {
            Ok(()) => {
                granted = Some((record.clone(), status));
                break;
            }
            Err(reason) => {
                live_but_not_covering += 1;
                if !refusals.contains(&reason) {
                    refusals.push(reason);
                }
            }
        }
    }
    let Some((record, status)) = granted else {
        let detail = if live_but_not_covering > 0 {
            format!(
                "; {live_but_not_covering} live exception(s) exist but none covers action=`{}` target=`{}` in this project ({})",
                scope.action,
                scope.target,
                refusals.join(", ")
            )
        } else {
            String::new()
        };
        return Err(PolicyError::new(
            "policy.approval_required",
            format!(
                "apply requires a live approved exception covering this mutation; Unknown/absent approval is not an allow{detail}"
            ),
        ));
    };
    Ok(object([
        ("allowed", Value::Bool(true)),
        ("scope", scope.to_value()),
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

/// A requested exception, bound to one mutation scope and one expiry.
///
/// `expires_at` is absolute; the caller derives it from a required
/// `--expires-in`. There is no default lifetime.
pub fn new_exception(
    id: &str,
    requester: &str,
    scope: &MutationScope,
    expires_at: i64,
    created_at: i64,
    reason: Option<&str>,
) -> Value {
    object([
        ("exception_id", string(id)),
        ("requester", string(requester)),
        ("action", string(&scope.action)),
        ("project_digest", string(&scope.project_digest)),
        ("target", string(&scope.target)),
        ("state", string("requested")),
        ("created_at", Value::Int(created_at)),
        ("expires_at", Value::Int(expires_at)),
        (
            "reason",
            reason.map_or(Value::Null, string),
        ),
        ("approver", Value::Null),
    ])
}

/// States an exception may be moved to, and the state it must be in first.
/// Terminal states are terminal: nothing reopens a rejected or revoked
/// exception, so a revoked grant cannot be resurrected by a second call.
const TRANSITIONS: &[(&str, &str)] = &[
    ("requested", "approved"),
    ("requested", "rejected"),
    ("approved", "revoked"),
];

/// Move an exception to `next` on the authority of a verified principal.
///
/// The authority is a [`PrincipalProof`], which only [`verify_principal`] can
/// produce. A caller-attested `--role` / `--actor` is not accepted here and
/// has no parameter to arrive through.
pub fn transition(
    record: &Value,
    proof: &PrincipalProof,
    next: &str,
    now: i64,
) -> Result<Value, PolicyError> {
    if !proof.has_role("approver") {
        return Err(PolicyError::new(
            "exception.approver_role_required",
            format!(
                "principal `{}` is not enrolled as an approver",
                proof.principal_id()
            ),
        ));
    }

    let state = record
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("requested");
    if !TRANSITIONS
        .iter()
        .any(|(from, to)| *from == state && *to == next)
    {
        return Err(PolicyError::new(
            "exception.transition_invalid",
            format!("`{state}` cannot move to `{next}`"),
        ));
    }

    // Four eyes: the requester does not decide its own request unless the
    // registry enrolled that principal for self-approval in writing.
    let requester = record.get("requester").and_then(Value::as_str).unwrap_or("");
    if next == "approved" && requester == proof.principal_id() && !proof.self_approval() {
        return Err(PolicyError::new(
            "exception.self_approval_not_enrolled",
            format!(
                "principal `{}` requested this exception and is not enrolled for self-approval",
                proof.principal_id()
            ),
        ));
    }

    let Value::Object(map) = record else {
        return Err(PolicyError::new(
            "exception.malformed",
            "exception record must be an object",
        ));
    };
    let mut out = map.clone();
    out.insert("state".to_string(), string(next));
    out.insert("decided_by".to_string(), proof.to_value());
    out.insert("decided_at".to_string(), Value::Int(now));
    out.insert(
        "previous_state".to_string(),
        string(state),
    );
    Ok(Value::Object(out))
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
    fn required_allow_is_a_permission_not_a_deny() {
        // A single `required: true, effect: "allow"` rule is a mandatory
        // permission. Counting it as a required deny inverts the rule.
        let layers = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"allow"}]}]"#,
        )
        .unwrap();
        let result = evaluate(&layers, 1).unwrap();
        assert_eq!(result.get("verdict").and_then(Value::as_str), Some("pass"));
        assert_eq!(
            result.get("required_rules").and_then(Value::as_array).map(<[Value]>::len),
            Some(0)
        );
        assert_eq!(
            result
                .get("required_allow_rules")
                .and_then(Value::as_array)
                .map(<[Value]>::len),
            Some(1)
        );

        // It is an allow, so a mutation gated on this policy reaches the
        // approval check instead of being refused at the policy verdict.
        let err = authorize_mutation(Some(&layers), &[], 1, true, &scope()).expect_err("approval");
        assert_eq!(err.code, "policy.approval_required");

        // A required deny in the same stack still denies.
        let mixed = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"allow"},{"id":"r2","required":true,"effect":"deny"}]}]"#,
        )
        .unwrap();
        let denied = evaluate(&mixed, 1).unwrap();
        assert_eq!(denied.get("verdict").and_then(Value::as_str), Some("deny"));
        assert_eq!(
            denied.get("unique_authority").and_then(Value::as_str),
            Some("organization")
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

    fn scope() -> MutationScope {
        MutationScope::new("apply", "proj-a", "AGENTS.md")
    }

    fn covering(state: &str, expires_at: i64) -> Value {
        parse(&format!(
            r#"{{"state":"{state}","expires_at":{expires_at},"exception_id":"ex-1","action":"apply","project_digest":"proj-a","target":"AGENTS.md"}}"#
        ))
        .unwrap()
    }

    #[test]
    fn an_exception_covers_only_its_own_action_project_and_target() {
        let pass = parse(r#"[{"layer":"organization","mode":"enforceable","rules":[]}]"#).unwrap();
        let live = covering("approved", 99);
        assert!(authorize_mutation(Some(&pass), std::slice::from_ref(&live), 1, true, &scope()).is_ok());

        // Same record, another project: refused.
        let other_project = MutationScope::new("apply", "proj-b", "AGENTS.md");
        let err = authorize_mutation(Some(&pass), std::slice::from_ref(&live), 1, true, &other_project)
            .expect_err("project");
        assert_eq!(err.code, "policy.approval_required");
        assert!(err.message.contains("exception.project_mismatch"), "{}", err.message);

        // Same project, another action: refused.
        let other_action = MutationScope::new("assets.copy", "proj-a", "AGENTS.md");
        let err = authorize_mutation(Some(&pass), std::slice::from_ref(&live), 1, true, &other_action)
            .expect_err("action");
        assert!(err.message.contains("exception.action_mismatch"), "{}", err.message);

        // Same action and project, another target: refused; `*` covers any.
        let other_target = MutationScope::new("apply", "proj-a", "README.md");
        let err = authorize_mutation(Some(&pass), &[live], 1, true, &other_target)
            .expect_err("target");
        assert!(err.message.contains("exception.target_mismatch"), "{}", err.message);
        let any = parse(
            r#"{"state":"approved","expires_at":99,"exception_id":"ex-2","action":"apply","project_digest":"proj-a","target":"*"}"#,
        )
        .unwrap();
        assert!(authorize_mutation(Some(&pass), &[any], 1, true, &other_target).is_ok());

        // A pre-scope record (no action/project/target) covers nothing, and a
        // wildcard action or project is not a scope.
        let legacy = parse(r#"{"state":"approved","expires_at":99,"exception_id":"ex-3"}"#).unwrap();
        assert_eq!(exception_covers(&legacy, &scope()), Err("exception.scope_missing"));
        let wild = parse(
            r#"{"state":"approved","expires_at":99,"action":"*","project_digest":"proj-a","target":"*"}"#,
        )
        .unwrap();
        assert_eq!(exception_covers(&wild, &scope()), Err("exception.scope_wildcard_forbidden"));
    }

    #[test]
    fn a_requested_exception_records_its_scope_reason_and_expiry() {
        let rec = new_exception("ex-9", "alice", &scope(), 500, 100, Some("hotfix"));
        assert_eq!(rec.get("action").and_then(Value::as_str), Some("apply"));
        assert_eq!(rec.get("project_digest").and_then(Value::as_str), Some("proj-a"));
        assert_eq!(rec.get("target").and_then(Value::as_str), Some("AGENTS.md"));
        assert_eq!(rec.get("expires_at").and_then(Value::as_i64), Some(500));
        assert_eq!(rec.get("created_at").and_then(Value::as_i64), Some(100));
        assert_eq!(rec.get("reason").and_then(Value::as_str), Some("hotfix"));
        assert_eq!(rec.get("state").and_then(Value::as_str), Some("requested"));
    }

    #[test]
    fn mutation_is_fail_closed_without_layers_or_live_exception() {
        let err = authorize_mutation(None, &[], 1, true, &scope()).expect_err("unknown");
        assert_eq!(err.code, "policy.unknown");

        let empty = parse("[]").unwrap();
        let err = authorize_mutation(Some(&empty), &[], 1, true, &scope()).expect_err("empty");
        assert_eq!(err.code, "policy.unknown");

        let deny = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[{"id":"r1","required":true,"effect":"deny"}]}]"#,
        )
        .unwrap();
        let err = authorize_mutation(Some(&deny), &[], 1, true, &scope()).expect_err("deny");
        assert_eq!(err.code, "policy.denied");

        let detect = parse(
            r#"[{"layer":"organization","mode":"detect-only","rules":[{"id":"r1","required":true,"effect":"deny"}]}]"#,
        )
        .unwrap();
        let err = authorize_mutation(Some(&detect), &[], 1, true, &scope()).expect_err("detect");
        assert_eq!(err.code, "policy.detect_only_not_enforceable");

        let pass = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]"#,
        )
        .unwrap();
        let err = authorize_mutation(Some(&pass), &[], 1, true, &scope()).expect_err("approval");
        assert_eq!(err.code, "policy.approval_required");

        let expired = covering("approved", 10);
        let err = authorize_mutation(Some(&pass), &[expired], 11, true, &scope()).expect_err("expired");
        assert_eq!(err.code, "policy.approval_required");

        let live = covering("approved", 99);
        let ok = authorize_mutation(Some(&pass), &[live], 1, true, &scope()).unwrap();
        assert_eq!(ok.get("allowed").and_then(Value::as_bool), Some(true));
    }

    fn proof_for(id: &str, roles: &str, self_approval: bool) -> PrincipalProof {
        let secret = b"enrolled-secret";
        let reg = parse(&format!(
            r#"{{"schema":"ctxpect-principals-v1","principals":[{{"principal_id":"{id}","roles":{roles},"key_digest":"{}","self_approval":{self_approval}}}]}}"#,
            enrollment_digest(id, secret)
        ))
        .unwrap();
        verify_principal(Some(&reg), id, secret).unwrap()
    }

    #[test]
    fn only_an_enrolled_approver_moves_an_exception() {
        let rec = parse(r#"{"state":"requested","exception_id":"ex-1","requester":"alice"}"#)
            .unwrap();

        // A requester-only principal cannot approve.
        let requester_only = proof_for("bob", r#"["requester"]"#, false);
        let err = transition(&rec, &requester_only, "approved", 5).expect_err("role");
        assert_eq!(err.code, "exception.approver_role_required");
        assert_eq!(rec.get("state").and_then(Value::as_str), Some("requested"));

        // The requester does not approve its own request by default.
        let alice = proof_for("alice", r#"["requester","approver"]"#, false);
        let err = transition(&rec, &alice, "approved", 5).expect_err("self");
        assert_eq!(err.code, "exception.self_approval_not_enrolled");

        // A second enrolled approver can.
        let carol = proof_for("carol", r#"["approver"]"#, false);
        let approved = transition(&rec, &carol, "approved", 5).unwrap();
        assert_eq!(approved.get("state").and_then(Value::as_str), Some("approved"));
        assert_eq!(
            approved
                .pointer(&["decided_by", "principal_id"])
                .and_then(Value::as_str),
            Some("carol")
        );
        assert_eq!(
            approved
                .pointer(&["decided_by", "org_identity"])
                .and_then(Value::as_bool),
            Some(false)
        );

        // And that approval is what makes a mutation authorizable.
        let pass = parse(
            r#"[{"layer":"organization","mode":"enforceable","rules":[]}]"#,
        )
        .unwrap();
        let mut live = match approved.clone() {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        live.insert("expires_at".into(), Value::Int(99));
        live.insert("action".into(), string("apply"));
        live.insert("project_digest".into(), string("proj-a"));
        live.insert("target".into(), string("AGENTS.md"));
        let ok = authorize_mutation(Some(&pass), &[Value::Object(live)], 6, true, &scope()).unwrap();
        assert_eq!(ok.get("allowed").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn self_approval_requires_a_written_enrollment() {
        let rec = parse(r#"{"state":"requested","exception_id":"ex-1","requester":"solo"}"#)
            .unwrap();
        let solo = proof_for("solo", r#"["requester","approver"]"#, true);
        let approved = transition(&rec, &solo, "approved", 5).unwrap();
        assert_eq!(approved.get("state").and_then(Value::as_str), Some("approved"));
    }

    #[test]
    fn terminal_states_are_terminal() {
        let approver = proof_for("carol", r#"["approver"]"#, false);

        let rejected = parse(r#"{"state":"rejected","requester":"alice"}"#).unwrap();
        let err = transition(&rejected, &approver, "approved", 5).expect_err("reopen");
        assert_eq!(err.code, "exception.transition_invalid");

        // A revoked exception cannot be revived, which is what keeps a
        // revoked grant from being reused.
        let revoked = parse(r#"{"state":"revoked","requester":"alice"}"#).unwrap();
        let err = transition(&revoked, &approver, "approved", 5).expect_err("revive");
        assert_eq!(err.code, "exception.transition_invalid");

        // Approved goes to revoked, and a revoked record no longer grants.
        let approved = parse(r#"{"state":"approved","requester":"alice","expires_at":99}"#).unwrap();
        let now_revoked = transition(&approved, &approver, "revoked", 5).unwrap();
        let status = exception_status(&now_revoked, 6, true).unwrap();
        assert_eq!(status.get("grants").and_then(Value::as_bool), Some(false));
    }
}
