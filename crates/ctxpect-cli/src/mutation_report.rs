//! C-F03 execution-chain wiring for projection `apply` / `rollback`.
//!
//! The policy precondition table (`ctxpect_policy::precondition`) says which
//! evidence a limited static fix needs before it runs. This module is the
//! bridge between that table and the two mutation entry points, shared by
//! the CLI and the HTTP API so both give the same answer (R04):
//!
//! * on success, the result names which product check satisfied each
//!   required precondition and keeps the verification axes apart
//!   (operation / static re-verification / runtime / effect / trust) — an
//!   "applied, runtime not observed" outcome is reported as exactly that;
//! * on refusal, the reason code is mapped back to the precondition that
//!   was not met, so a caller learns *which* evidence is missing rather than
//!   only that the write did not happen.
//!
//! Every `satisfied` value is the name of a check the apply path actually
//! ran; the report is produced only after those checks passed and the write
//! committed. `missing` is computed by the table, not hardcoded, and a unit
//! test pins the `satisfied` keys to the table's `required()` order so the
//! two cannot drift apart silently.

use crate::inspect::InspectFailure;
use ctxpect_policy::precondition::{evaluate, ActionClass};
use ctxpect_schema::{array, object, string, Value};

/// Which product check satisfies each required precondition of a projection
/// `apply`, in table order.
pub(crate) const APPLY_SATISFIED: &[(&str, &str)] = &[
    // contained_target + probe_target: inside the root, a regular file, not
    // a control path (`.git/`, `.ctxpect/`, the store).
    ("target-identity", "projection.contained_target"),
    // The target's bytes still hash to the preview's `current_digest`.
    ("original-bytes", "projection.current_digest"),
    // Desired bytes hash to `desired_digest`; the secret gate passed.
    ("fix-semantics", "projection.desired_digest"),
    // A live approved exception covers action, project and target.
    ("permission", "policy.exception_covers_scope"),
    // The policy layers evaluated to `pass`.
    ("policy-eval", "policy.layers_pass"),
    // A persisted preview for this project, still in state `previewed`.
    ("approved-plan", "projection.preview_previewed"),
];

/// Same for `rollback`: undoing a transaction is a limited static fix too.
pub(crate) const ROLLBACK_SATISFIED: &[(&str, &str)] = &[
    ("target-identity", "projection.contained_target"),
    // The target still holds the transaction's `after_digest` (or was never
    // touched by a pending apply).
    ("original-bytes", "projection.after_digest"),
    // Backup bytes hash to the recorded `before_digest`.
    ("fix-semantics", "projection.backup_digest"),
    ("permission", "policy.exception_covers_scope"),
    ("policy-eval", "policy.layers_pass"),
    // A `tx.json` for this project that was not already rolled back.
    ("approved-plan", "projection.tx_record"),
];

/// The precondition a refusal code stands for, or `None` when the refusal
/// is not about missing evidence (`store.busy` is contention, not a gap).
pub(crate) fn precondition_for_refusal(code: &str) -> Option<&'static str> {
    match code {
        "projection.preview_missing"
        | "projection.preview_scope"
        | "projection.tx_consumed"
        | "projection.export_only"
        | "projection.no_tx"
        | "projection.rollback_scope"
        | "projection.already_rolled_back" => Some("approved-plan"),
        "projection.concurrent_hash" | "projection.rollback_conflict" => Some("original-bytes"),
        "projection.preview_malformed"
        | "projection.contains_secrets"
        | "projection.backup_corrupt" => Some("fix-semantics"),
        "projection.not_a_file"
        | "projection.control_path"
        | "projection.path"
        | "projection.escapes"
        | "projection.io"
        | "projection.parse" => Some("target-identity"),
        "policy.unknown"
        | "policy.indeterminate"
        | "policy.denied"
        | "policy.detect_only_not_enforceable" => Some("policy-eval"),
        "policy.approval_required" => Some("permission"),
        _ => None,
    }
}

/// The `preconditions` object for a completed mutation: the table evaluated
/// against the checks that ran.
fn preconditions(satisfied: &[(&str, &str)]) -> Value {
    let class = ActionClass::LimitedStaticFix;
    let ids: Vec<&str> = satisfied.iter().map(|(id, _)| *id).collect();
    let report = evaluate(class, &ids);
    object([
        ("action_class", string(class.as_str())),
        ("complete", Value::Bool(report.is_complete())),
        (
            "satisfied",
            object(satisfied.iter().map(|(id, check)| (*id, string(*check)))),
        ),
        ("missing", array(report.missing.iter().map(|id| string(*id)))),
        (
            "may_remain_unknown",
            array(class.may_remain_unknown().iter().map(|id| string(*id))),
        ),
    ])
}

/// The two objects appended to a successful `apply` / `rollback` result.
/// `operation` is the transaction's recorded state; `post_receipt_id` is the
/// static observation taken after the write.
pub(crate) fn mutation_outcome(
    satisfied: &[(&str, &str)],
    operation: &str,
    post_receipt_id: &str,
) -> [(&'static str, Value); 2] {
    [
        ("preconditions", preconditions(satisfied)),
        (
            "verification",
            object([
                ("operation", string(operation)),
                ("static_reverification", string("observed")),
                ("post_receipt_id", string(post_receipt_id)),
                ("runtime_verification", string("not-observed")),
                ("effect", string("not-evaluated")),
                ("trust", string("local-continuity")),
            ]),
        ),
    ]
}

/// Attach the missing precondition to a refusal raised on the apply /
/// rollback path. Failures the table does not explain pass through as is.
pub(crate) fn attach_precondition(failure: InspectFailure) -> InspectFailure {
    match precondition_for_refusal(failure.code()) {
        Some(precondition) => failure.refused_for(precondition),
        None => failure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_policy::precondition::requires;

    fn table_ids() -> Vec<&'static str> {
        ActionClass::LimitedStaticFix
            .required()
            .iter()
            .map(|item| item.id)
            .collect()
    }

    #[test]
    fn satisfied_keys_follow_the_precondition_table_in_order() {
        for satisfied in [APPLY_SATISFIED, ROLLBACK_SATISFIED] {
            let keys: Vec<&str> = satisfied.iter().map(|(id, _)| *id).collect();
            assert_eq!(keys, table_ids());
        }
    }

    #[test]
    fn every_mapped_refusal_names_a_required_precondition() {
        for code in [
            "projection.preview_missing",
            "projection.preview_scope",
            "projection.tx_consumed",
            "projection.export_only",
            "projection.no_tx",
            "projection.rollback_scope",
            "projection.already_rolled_back",
            "projection.concurrent_hash",
            "projection.rollback_conflict",
            "projection.preview_malformed",
            "projection.contains_secrets",
            "projection.backup_corrupt",
            "projection.not_a_file",
            "projection.control_path",
            "projection.path",
            "projection.escapes",
            "projection.io",
            "projection.parse",
            "policy.unknown",
            "policy.indeterminate",
            "policy.denied",
            "policy.detect_only_not_enforceable",
            "policy.approval_required",
        ] {
            let id = precondition_for_refusal(code).unwrap_or_else(|| panic!("{code} unmapped"));
            assert!(requires(ActionClass::LimitedStaticFix, id), "{code} -> {id}");
        }
        assert_eq!(precondition_for_refusal("store.busy"), None);
        assert_eq!(precondition_for_refusal("usage.invalid"), None);
    }

    #[test]
    fn a_completed_mutation_reports_no_missing_evidence_and_the_unknown_axes() {
        let [(_, preconditions), (_, verification)] =
            mutation_outcome(APPLY_SATISFIED, "committed", "r1");
        assert_eq!(preconditions.get("complete"), Some(&Value::Bool(true)));
        assert_eq!(preconditions.get("missing"), Some(&array([])));
        assert_eq!(
            preconditions.get("may_remain_unknown"),
            Some(&array([string("outcome")]))
        );
        assert_eq!(
            verification.get("runtime_verification").and_then(Value::as_str),
            Some("not-observed")
        );
        assert_eq!(verification.get("operation").and_then(Value::as_str), Some("committed"));
    }
}
