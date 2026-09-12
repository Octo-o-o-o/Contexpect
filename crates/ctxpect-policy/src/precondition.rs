//! Action-class precondition table (C-F03; Final Recommendations 2026-09-11
//! §4.3): which evidence each operation requires before it runs, and what may
//! stay Unknown while it does.
//!
//! Fail-closed is not lowered here; it is bound precisely to the conditions
//! the action needs. A write is refused when a *required* precondition of its
//! class is Unknown; evidence the table lists as may-stay-unknown (for a
//! limited static fix: whether the fix improves task quality) never blocks
//! the action on its own.
//!
//! Pure data and pure functions: no IO, no clock, no schema `Value`. This
//! module is the contract anchor the execution chain (projection apply,
//! session import, standard publish) wires against; that wiring is a
//! separate work package.

/// The five operation classes of the §4.3 precondition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionClass {
    /// Read-only inspection of a project or receipt.
    ReadOnlyInspection,
    /// A bounded static fix to bytes on disk (Doctor treatment, apply).
    LimitedStaticFix,
    /// Claiming a model-visible fact.
    ModelVisibleClaim,
    /// Claiming a runtime fix has been verified.
    RuntimeFixVerifiedClaim,
    /// Publishing or adopting a team standard.
    PublishOrAdoptStandard,
}

/// One required precondition: a stable machine id plus the evidence that
/// satisfies it, in words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Precondition {
    /// Stable id, e.g. `target-identity`. Callers match on this, never on
    /// the prose.
    pub id: &'static str,
    /// The evidence this precondition names.
    pub evidence: &'static str,
}

const READ_ONLY_REQUIRED: &[Precondition] = &[
    Precondition { id: "read-scope", evidence: "an explicit read scope" },
    Precondition { id: "parse-contract", evidence: "a supported parse contract" },
    Precondition { id: "resource-limits", evidence: "resource limits" },
];
const READ_ONLY_MAY_UNKNOWN: &[&str] = &["runtime-surface"];

const STATIC_FIX_REQUIRED: &[Precondition] = &[
    Precondition { id: "target-identity", evidence: "the target's identity" },
    Precondition { id: "original-bytes", evidence: "the original bytes" },
    Precondition { id: "fix-semantics", evidence: "the fix semantics" },
    Precondition { id: "permission", evidence: "permission to write" },
    Precondition { id: "policy-eval", evidence: "a passing policy evaluation" },
    Precondition { id: "approved-plan", evidence: "an approved plan" },
];
const STATIC_FIX_MAY_UNKNOWN: &[&str] = &["outcome"];

const MODEL_VISIBLE_REQUIRED: &[Precondition] = &[
    Precondition { id: "native-fields", evidence: "the native fields the claim reads" },
    Precondition { id: "evidence-source", evidence: "the evidence source" },
    Precondition { id: "coordinate-coverage", evidence: "coordinate coverage" },
    Precondition { id: "temporal-coverage", evidence: "temporal coverage" },
];
const MODEL_VISIBLE_MAY_UNKNOWN: &[&str] = &["internal-attribution"];

const RUNTIME_VERIFIED_REQUIRED: &[Precondition] = &[
    Precondition {
        id: "post-change-runtime-evidence",
        evidence: "fresh evidence from the modified runtime instance",
    },
    Precondition { id: "required-checks", evidence: "the required checks" },
];
const RUNTIME_VERIFIED_MAY_UNKNOWN: &[&str] = &["out-of-contract-surfaces"];

const PUBLISH_REQUIRED: &[Precondition] = &[
    Precondition { id: "publisher-trust", evidence: "publisher trust" },
    Precondition { id: "version-lineage", evidence: "the version lineage" },
    Precondition { id: "permission", evidence: "permission to publish" },
    Precondition { id: "applicable-coordinates", evidence: "the applicable coordinates" },
    Precondition { id: "exceptions", evidence: "the recorded exceptions" },
];
const PUBLISH_MAY_UNKNOWN: &[&str] = &["private-context"];

impl ActionClass {
    /// Every class, in table order.
    pub const ALL: &[ActionClass] = &[
        ActionClass::ReadOnlyInspection,
        ActionClass::LimitedStaticFix,
        ActionClass::ModelVisibleClaim,
        ActionClass::RuntimeFixVerifiedClaim,
        ActionClass::PublishOrAdoptStandard,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ActionClass::ReadOnlyInspection => "read-only-inspection",
            ActionClass::LimitedStaticFix => "limited-static-fix",
            ActionClass::ModelVisibleClaim => "model-visible-claim",
            ActionClass::RuntimeFixVerifiedClaim => "runtime-fix-verified-claim",
            ActionClass::PublishOrAdoptStandard => "publish-or-adopt-standard",
        }
    }

    /// The evidence this class requires before it runs. Unknown on any of
    /// these blocks the action.
    #[must_use]
    pub const fn required(self) -> &'static [Precondition] {
        match self {
            ActionClass::ReadOnlyInspection => READ_ONLY_REQUIRED,
            ActionClass::LimitedStaticFix => STATIC_FIX_REQUIRED,
            ActionClass::ModelVisibleClaim => MODEL_VISIBLE_REQUIRED,
            ActionClass::RuntimeFixVerifiedClaim => RUNTIME_VERIFIED_REQUIRED,
            ActionClass::PublishOrAdoptStandard => PUBLISH_REQUIRED,
        }
    }

    /// Evidence ids that may stay Unknown without blocking this class. The
    /// result is marked partial/Unknown where the gap shows, but the action
    /// itself is not refused for it.
    #[must_use]
    pub const fn may_remain_unknown(self) -> &'static [&'static str] {
        match self {
            ActionClass::ReadOnlyInspection => READ_ONLY_MAY_UNKNOWN,
            ActionClass::LimitedStaticFix => STATIC_FIX_MAY_UNKNOWN,
            ActionClass::ModelVisibleClaim => MODEL_VISIBLE_MAY_UNKNOWN,
            ActionClass::RuntimeFixVerifiedClaim => RUNTIME_VERIFIED_MAY_UNKNOWN,
            ActionClass::PublishOrAdoptStandard => PUBLISH_MAY_UNKNOWN,
        }
    }
}

/// Whether `evidence_id` names a required precondition of `class`. An id the
/// table does not know is not required by that class.
#[must_use]
pub fn requires(class: ActionClass, evidence_id: &str) -> bool {
    class.required().iter().any(|item| item.id == evidence_id)
}

/// The outcome of checking one class against the evidence on hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreconditionReport {
    pub class: ActionClass,
    /// Required preconditions whose evidence is not in the satisfied set, in
    /// table order. Empty means the action's preconditions are met.
    pub missing: Vec<&'static str>,
}

impl PreconditionReport {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Evaluate `class` against the evidence ids known to be satisfied. Ids not
/// in the table are ignored: they decide nothing here.
#[must_use]
pub fn evaluate(class: ActionClass, satisfied: &[&str]) -> PreconditionReport {
    let missing = class
        .required()
        .iter()
        .filter(|item| !satisfied.contains(&item.id))
        .map(|item| item.id)
        .collect();
    PreconditionReport { class, missing }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(class: ActionClass) -> Vec<&'static str> {
        class.required().iter().map(|item| item.id).collect()
    }

    #[test]
    fn every_class_is_complete_when_all_required_evidence_is_satisfied() {
        for class in ActionClass::ALL {
            let satisfied = ids(*class);
            let report = evaluate(*class, &satisfied);
            assert!(report.is_complete(), "{}: {:?}", class.as_str(), report.missing);
            assert!(report.missing.is_empty());
        }
    }

    #[test]
    fn every_class_fails_closed_when_nothing_is_satisfied() {
        for class in ActionClass::ALL {
            let report = evaluate(*class, &[]);
            assert!(!report.is_complete(), "{}", class.as_str());
            assert_eq!(report.missing, ids(*class));
        }
    }

    #[test]
    fn a_missing_precondition_is_named_and_only_it() {
        for class in ActionClass::ALL {
            let all = ids(*class);
            for (index, dropped) in all.iter().enumerate() {
                let satisfied: Vec<&str> = all
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .map(|(_, id)| *id)
                    .collect();
                let report = evaluate(*class, &satisfied);
                assert_eq!(report.missing, vec![*dropped], "{}", class.as_str());
            }
        }
    }

    #[test]
    fn may_remain_unknown_never_satisfies_a_requirement() {
        for class in ActionClass::ALL {
            for id in class.may_remain_unknown() {
                assert!(
                    !requires(*class, id),
                    "{} lists {id} as both required and may-stay-unknown",
                    class.as_str()
                );
                // Evidence that may stay Unknown does not count as satisfied.
                let report = evaluate(*class, &[*id]);
                assert!(!report.is_complete(), "{}", class.as_str());
            }
        }
    }

    #[test]
    fn the_static_fix_class_binds_to_the_c_f03_row() {
        // Outcome evidence may stay Unknown for a limited static fix; target
        // identity, permission, policy and the approved plan may not.
        assert!(!requires(ActionClass::LimitedStaticFix, "outcome"));
        assert!(ActionClass::LimitedStaticFix
            .may_remain_unknown()
            .contains(&"outcome"));
        for id in ["target-identity", "permission", "policy-eval", "approved-plan"] {
            assert!(requires(ActionClass::LimitedStaticFix, id), "{id}");
        }
        // Runtime-surface evidence is not a static-fix precondition either:
        // a missing runtime snapshot does not block reading or fixing bytes.
        assert!(!requires(ActionClass::LimitedStaticFix, "runtime-surface"));
    }

    #[test]
    fn unknown_evidence_ids_decide_nothing() {
        let report = evaluate(ActionClass::ReadOnlyInspection, &["nonsense"]);
        assert_eq!(report.missing, ids(ActionClass::ReadOnlyInspection));
        assert!(!requires(ActionClass::ModelVisibleClaim, "nonsense"));
    }
}
