//! Pins the Rust judgement to the contract's invariant *bodies*, not just their ids.
//!
//! `contract_parity.rs` checks that the set and order of invariants match. That is
//! not enough: review-1 found that invariant 1 declares
//! `allowed_truth_state: [indeterminate, not-applicable]` while the implementation
//! only banned `present`, so a resolved claim could assert a runtime facet was
//! *absent*. The id list was identical, so nothing went red.
//!
//! These tests read the constraint fields themselves and enumerate the axes, so a
//! constraint that exists in the contract but not in the code fails here.

use ctxpect_core::axes::{
    ClaimKind, Coverage, EffectDecision, KnowledgeStatus, LifecycleStage, Precision, Provenance,
    TruthState, UseEvidenceKind,
};
use ctxpect_core::claim::{Claim, ExperimentRef, InvariantId};
use ctxpect_schema::{parse, Value};
use std::path::PathBuf;

fn matrix() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repository root")
        .join("acceptance/claim-validity-matrix.yaml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    parse(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn invariant(id: &str) -> Value {
    matrix()
        .get("invariants")
        .and_then(Value::as_array)
        .expect("invariants")
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(id))
        .unwrap_or_else(|| panic!("contract has no invariant `{id}`"))
        .clone()
}

/// A claim that trips no invariant, so each case isolates one rule.
fn honest() -> Claim {
    Claim {
        claim_kind: ClaimKind::Observed,
        coverage: Coverage::FullDeclaredSurface,
        knowledge_status: KnowledgeStatus::Current,
        lifecycle_stage: LifecycleStage::Installed,
        precision: Precision::Exact,
        provenance: Provenance::NativeRuntime,
        truth_state: TruthState::Present,
        use_evidence_kind: None,
        decision: None,
        experiment: ExperimentRef::default(),
        unknown_reason: None,
        capability_unexposed: false,
        has_timeline_events: false,
        contradicted_by_equal_coverage: false,
        filled_from_higher_provenance_outside_coverage: false,
    }
}

fn violates(claim: &Claim, id: InvariantId) -> bool {
    claim.violations().iter().any(|v| v.invariant == id)
}

fn wire_list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .map(|v| v.as_str_list().into_iter().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Invariant 1: `if {claim_kind: resolved, lifecycle_stage: [runtime facets]}` with
/// both `forbidden_truth_state` and `allowed_truth_state`. Every truth state
/// outside the allowlist must be rejected, every state inside it accepted.
#[test]
fn resolved_runtime_facets_honour_the_declared_truth_state_sets() {
    let inv = invariant("resolved-no-present-runtime-facets");
    let allowed = wire_list(&inv, "allowed_truth_state");
    let forbidden = wire_list(&inv, "forbidden_truth_state");
    assert!(
        !allowed.is_empty(),
        "contract declares no allowed_truth_state; this test would be vacuous"
    );

    let stages: Vec<LifecycleStage> = inv
        .get("if")
        .and_then(|c| c.get("lifecycle_stage"))
        .map(|v| {
            v.as_str_list()
                .into_iter()
                .map(|s| {
                    LifecycleStage::from_wire(s).unwrap_or_else(|| panic!("unknown stage `{s}`"))
                })
                .collect()
        })
        .expect("invariant 1 declares lifecycle stages");
    assert_eq!(stages.len(), 3, "expected the three runtime facets");

    for stage in stages {
        for state in TruthState::ALL {
            let claim = Claim {
                claim_kind: ClaimKind::Resolved,
                lifecycle_stage: stage,
                truth_state: *state,
                // Keep every other axis honest so only this rule can bite.
                provenance: Provenance::NativeRuntime,
                coverage: Coverage::FullDeclaredSurface,
                ..honest()
            };
            let tripped = violates(&claim, InvariantId::ResolvedNoPresentRuntimeFacets);
            let wire = state.as_str();

            if allowed.iter().any(|a| a == wire) {
                assert!(
                    !tripped,
                    "{stage}/{wire} is in allowed_truth_state but was rejected"
                );
            } else {
                assert!(
                    tripped,
                    "{stage}/{wire} is outside allowed_truth_state but was accepted"
                );
            }
            if forbidden.iter().any(|f| f == wire) {
                assert!(tripped, "{stage}/{wire} is forbidden but was accepted");
            }
        }
    }
}

/// Invariant 2: `model-visible` + `present` requires native provenance and at
/// least the declared minimum coverage.
#[test]
fn model_visible_present_honours_declared_provenance_and_coverage() {
    let inv = invariant("model-visible-present-requires-native");
    let required: Vec<Provenance> = wire_list(&inv, "required_provenance")
        .iter()
        .map(|s| Provenance::from_wire(s).unwrap_or_else(|| panic!("unknown provenance `{s}`")))
        .collect();
    let minimum = inv
        .get("minimum_coverage")
        .and_then(Value::as_str)
        .and_then(Coverage::from_wire)
        .expect("invariant 2 declares a minimum coverage");
    assert!(!required.is_empty(), "no required provenance declared");

    for provenance in Provenance::ALL {
        for coverage in Coverage::ALL {
            let claim = Claim {
                lifecycle_stage: LifecycleStage::ModelVisible,
                truth_state: TruthState::Present,
                provenance: *provenance,
                coverage: *coverage,
                ..honest()
            };
            let tripped = violates(&claim, InvariantId::ModelVisiblePresentRequiresNative);
            let satisfies = required.contains(provenance) && coverage.at_least(minimum);
            assert_eq!(
                !tripped,
                satisfies,
                "{provenance}/{coverage}: contract says satisfies={satisfies}, implementation tripped={tripped}"
            );
        }
    }
}

/// Invariant 3: an internal attribution may never be reported as present, and the
/// contract's `default_truth_state` is the honest fallback.
#[test]
fn internal_attribution_honours_its_forbidden_truth_state() {
    let inv = invariant("internal-attribution-never-present-without-native-semantics");
    let forbidden = wire_list(&inv, "forbidden_truth_state");
    let default = inv
        .get("default_truth_state")
        .and_then(Value::as_str)
        .and_then(TruthState::from_wire)
        .expect("invariant 3 declares a default truth state");
    assert!(!forbidden.is_empty());

    for state in TruthState::ALL {
        let claim = Claim {
            lifecycle_stage: LifecycleStage::UseEvidence,
            use_evidence_kind: Some(UseEvidenceKind::InternalAttribution),
            truth_state: *state,
            coverage: Coverage::FullDeclaredSurface,
            ..honest()
        };
        let tripped = violates(
            &claim,
            InvariantId::InternalAttributionNeverPresentWithoutNativeSemantics,
        );
        assert_eq!(
            tripped,
            forbidden.iter().any(|f| f == state.as_str()),
            "internal-attribution/{state} disagreed with forbidden_truth_state"
        );
    }

    // The declared default must itself be expressible, and it must be the honest
    // one: review-3 found this assertion still passing when the contract's default
    // was changed to `absent`, because "not forbidden" is weaker than "is the
    // fallback the contract names".
    assert!(
        !forbidden.iter().any(|f| f == default.as_str()),
        "default_truth_state {default} is also listed as forbidden"
    );
    assert_eq!(
        default,
        TruthState::Indeterminate,
        "an unproven attribution's honest fallback is indeterminate, not {default}"
    );
    let fallback = Claim {
        lifecycle_stage: LifecycleStage::UseEvidence,
        use_evidence_kind: Some(UseEvidenceKind::InternalAttribution),
        truth_state: default,
        unknown_reason: Some(ctxpect_core::UnknownReason::SurfaceNotExposed),
        ..honest()
    };
    assert!(
        fallback.is_expressible(),
        "the contract's default_truth_state must be fully expressible: {:?}",
        fallback.violations()
    );
}

/// Invariant 4: the declared decisions require the declared claim kind and fields.
#[test]
fn outcome_affecting_honours_declared_decisions_and_required_fields() {
    let inv = invariant("outcome-affecting-decision-from-experiment-only");
    let decisions: Vec<EffectDecision> = inv
        .get("if")
        .and_then(|c| c.get("decision"))
        .map(|v| {
            v.as_str_list()
                .into_iter()
                .map(|s| {
                    EffectDecision::from_wire(s).unwrap_or_else(|| panic!("unknown decision `{s}`"))
                })
                .collect()
        })
        .expect("invariant 4 declares decisions");
    let required_kind = inv
        .get("required_claim_kind")
        .and_then(Value::as_str)
        .and_then(ClaimKind::from_wire)
        .expect("invariant 4 declares a required claim kind");
    let required_fields = wire_list(&inv, "required_fields");
    assert_eq!(decisions.len(), 3, "expected the three supported decisions");
    assert!(required_fields.contains(&"experiment_id".to_string()));
    assert!(required_fields.contains(&"contract_digest".to_string()));

    let cited = ExperimentRef {
        experiment_id: Some("exp-1".into()),
        contract_digest: Some("digest-1".into()),
    };

    for decision in &decisions {
        // Wrong claim kind must trip.
        for kind in ClaimKind::ALL {
            let claim = Claim {
                claim_kind: *kind,
                lifecycle_stage: LifecycleStage::OutcomeAffecting,
                decision: Some(*decision),
                experiment: cited.clone(),
                truth_state: TruthState::Indeterminate,
                unknown_reason: Some(ctxpect_core::UnknownReason::EvidenceStale),
                ..honest()
            };
            let tripped = violates(
                &claim,
                InvariantId::OutcomeAffectingDecisionFromExperimentOnly,
            );
            assert_eq!(
                tripped,
                *kind != required_kind,
                "{kind}/{decision}: required_claim_kind is {required_kind}"
            );
        }

        // Each required field, individually missing, must trip.
        for omitted in &required_fields {
            let mut experiment = cited.clone();
            match omitted.as_str() {
                "experiment_id" => experiment.experiment_id = None,
                "contract_digest" => experiment.contract_digest = None,
                other => panic!("unhandled required field `{other}`"),
            }
            let claim = Claim {
                claim_kind: required_kind,
                lifecycle_stage: LifecycleStage::OutcomeAffecting,
                decision: Some(*decision),
                experiment,
                truth_state: TruthState::Indeterminate,
                unknown_reason: Some(ctxpect_core::UnknownReason::EvidenceStale),
                ..honest()
            };
            assert!(
                violates(
                    &claim,
                    InvariantId::OutcomeAffectingDecisionFromExperimentOnly
                ),
                "a missing `{omitted}` must trip invariant 4 for {decision}"
            );
        }
    }

    // `inconclusive` is outside the declared decision set and must not trip it.
    let inconclusive = Claim {
        claim_kind: ClaimKind::Observed,
        lifecycle_stage: LifecycleStage::OutcomeAffecting,
        decision: Some(EffectDecision::Inconclusive),
        experiment: ExperimentRef::default(),
        truth_state: TruthState::Indeterminate,
        unknown_reason: Some(ctxpect_core::UnknownReason::EvidenceStale),
        ..honest()
    };
    assert!(!violates(
        &inconclusive,
        InvariantId::OutcomeAffectingDecisionFromExperimentOnly
    ));
}

/// Invariant 5: an inconclusive decision may not be reported as the forbidden state.
#[test]
fn inconclusive_honours_its_forbidden_truth_state() {
    let inv = invariant("inconclusive-is-not-absent");
    let forbidden = wire_list(&inv, "forbidden_truth_state");
    assert!(!forbidden.is_empty());

    for state in TruthState::ALL {
        let claim = Claim {
            claim_kind: ClaimKind::Effect,
            lifecycle_stage: LifecycleStage::OutcomeAffecting,
            decision: Some(EffectDecision::Inconclusive),
            truth_state: *state,
            coverage: Coverage::FullDeclaredSurface,
            unknown_reason: (*state == TruthState::Indeterminate)
                .then_some(ctxpect_core::UnknownReason::EvidenceStale),
            ..honest()
        };
        assert_eq!(
            violates(&claim, InvariantId::InconclusiveIsNotAbsent),
            forbidden.iter().any(|f| f == state.as_str()),
            "inconclusive/{state} disagreed with forbidden_truth_state"
        );
    }
}

/// Invariant 6: an unexposed capability is pinned to the declared truth state and
/// may not carry synthetic events.
#[test]
fn unexposed_capability_honours_required_truth_state_and_event_ban() {
    let inv = invariant("unexposed-timeline-no-fake-events");
    let required = wire_list(&inv, "required_truth_state");
    let forbids_events = inv
        .get("forbid_synthetic_events")
        .and_then(Value::as_bool)
        .expect("invariant 6 declares forbid_synthetic_events");
    assert!(!required.is_empty());
    assert!(forbids_events);

    for state in TruthState::ALL {
        let claim = Claim {
            capability_unexposed: true,
            truth_state: *state,
            has_timeline_events: false,
            unknown_reason: (*state == TruthState::Indeterminate)
                .then_some(ctxpect_core::UnknownReason::SurfaceNotExposed),
            ..honest()
        };
        assert_eq!(
            !violates(&claim, InvariantId::UnexposedTimelineNoFakeEvents),
            required.iter().any(|r| r == state.as_str()),
            "unexposed/{state} disagreed with required_truth_state"
        );
    }

    let with_events = Claim {
        capability_unexposed: true,
        truth_state: TruthState::Indeterminate,
        unknown_reason: Some(ctxpect_core::UnknownReason::SurfaceNotExposed),
        has_timeline_events: true,
        ..honest()
    };
    assert!(
        violates(&with_events, InvariantId::UnexposedTimelineNoFakeEvents),
        "synthetic events must be forbidden"
    );
}

/// Invariant 7: a contradiction must be kept as the declared status, never resolved
/// silently.
#[test]
fn contradiction_honours_required_knowledge_status() {
    let inv = invariant("conflict-keep-conflicted");
    let required = inv
        .get("required_knowledge_status")
        .and_then(Value::as_str)
        .and_then(KnowledgeStatus::from_wire)
        .expect("invariant 7 declares a required knowledge status");
    assert_eq!(
        inv.get("silent_choice").and_then(Value::as_bool),
        Some(false),
        "the contract must forbid a silent choice"
    );

    for status in KnowledgeStatus::ALL {
        let claim = Claim {
            contradicted_by_equal_coverage: true,
            knowledge_status: *status,
            ..honest()
        };
        assert_eq!(
            violates(&claim, InvariantId::ConflictKeepConflicted),
            *status != required,
            "contradiction/{status} disagreed with required_knowledge_status"
        );
    }
}

/// Invariant 9: `absent` may not rest on the forbidden coverage levels.
#[test]
fn absent_honours_forbidden_coverage() {
    let inv = invariant("absent-requires-sufficient-coverage");
    let forbidden = wire_list(&inv, "forbidden_coverage");
    assert!(!forbidden.is_empty());

    for coverage in Coverage::ALL {
        let claim = Claim {
            truth_state: TruthState::Absent,
            coverage: *coverage,
            ..honest()
        };
        assert_eq!(
            violates(&claim, InvariantId::AbsentRequiresSufficientCoverage),
            forbidden.iter().any(|f| f == coverage.as_str()),
            "absent/{coverage} disagreed with forbidden_coverage"
        );
    }
}

/// Invariant 10: an indeterminate claim must name a reason, drawn from the
/// contract's closed vocabulary.
#[test]
fn indeterminate_honours_required_reason_and_vocabulary() {
    let inv = invariant("indeterminate-requires-unknown-reason");
    let required_fields = wire_list(&inv, "required_fields");
    assert!(
        required_fields.contains(&"unknown_reason".to_string()),
        "invariant 10 must require the unknown_reason field"
    );

    let vocabulary_key = inv
        .get("required_vocabulary")
        .and_then(Value::as_str)
        .expect("invariant 10 declares a vocabulary");
    let frozen: Vec<String> = matrix()
        .get(vocabulary_key)
        .map(|v| v.as_str_list().into_iter().map(str::to_string).collect())
        .unwrap_or_else(|| panic!("contract has no `{vocabulary_key}` list"));
    assert!(!frozen.is_empty());

    // The Rust vocabulary is the closed enum; it must equal the frozen list, so a
    // reason outside the contract is unrepresentable rather than merely unchecked.
    let ours: Vec<String> = ctxpect_core::UnknownReason::ALL
        .iter()
        .map(|r| r.as_str().to_string())
        .collect();
    assert_eq!(ours, frozen, "unknown reason vocabulary drifted");

    // Missing reason trips; every declared reason satisfies it.
    let missing = Claim {
        truth_state: TruthState::Indeterminate,
        unknown_reason: None,
        ..honest()
    };
    assert!(
        violates(&missing, InvariantId::IndeterminateRequiresUnknownReason),
        "an indeterminate claim without a reason must be rejected"
    );
    for reason in ctxpect_core::UnknownReason::ALL {
        let stated = Claim {
            truth_state: TruthState::Indeterminate,
            unknown_reason: Some(*reason),
            ..honest()
        };
        assert!(
            !violates(&stated, InvariantId::IndeterminateRequiresUnknownReason),
            "{reason} is a declared reason and must satisfy invariant 10"
        );
    }

    // Other truth states are outside the `if`, so a missing reason is fine there.
    for state in TruthState::ALL {
        if *state == TruthState::Indeterminate {
            continue;
        }
        let claim = Claim {
            truth_state: *state,
            unknown_reason: None,
            coverage: Coverage::FullDeclaredSurface,
            ..honest()
        };
        assert!(
            !violates(&claim, InvariantId::IndeterminateRequiresUnknownReason),
            "{state} is outside invariant 10's condition"
        );
    }
}

/// Invariant 8: a higher-provenance partial source may not override fields outside
/// its coverage. review-2 found `cannot_override` sitting in the consumed list with
/// no test actually reading it, so the key was whitelisted rather than checked.
#[test]
fn higher_provenance_honours_its_cannot_override_scope() {
    let inv = invariant("higher-provenance-cannot-fill-uncovered-fields");
    let scope = inv
        .get("cannot_override")
        .and_then(Value::as_str)
        .expect("invariant 8 declares what cannot be overridden");
    assert_eq!(
        scope, "fields_outside_its_coverage",
        "the declared override scope changed; the judgement below must follow it"
    );
    let condition = inv
        .get("if")
        .and_then(|c| c.get("higher_provenance_partial"))
        .and_then(Value::as_bool)
        .expect("invariant 8 declares its condition");
    assert!(condition, "the condition is expected to be a positive flag");

    // The flag is the modelled form of "filled from outside its coverage".
    for flagged in [true, false] {
        let claim = Claim {
            filled_from_higher_provenance_outside_coverage: flagged,
            ..honest()
        };
        assert_eq!(
            violates(&claim, InvariantId::HigherProvenanceCannotFillUncoveredFields),
            flagged,
            "invariant 8 must trip exactly when the condition holds"
        );
    }

    // It must hold regardless of how strong the source claims to be: outranking
    // another source does not extend its reach.
    for provenance in Provenance::ALL {
        let claim = Claim {
            filled_from_higher_provenance_outside_coverage: true,
            provenance: *provenance,
            coverage: Coverage::PartialDeclaredSurface,
            ..honest()
        };
        assert!(
            violates(&claim, InvariantId::HigherProvenanceCannotFillUncoveredFields),
            "{provenance} must not be exempt from invariant 8"
        );
    }
}

/// Every constraint key the contract uses must be consumed by some test above.
/// A new key appearing in the contract means a new semantic the code may be
/// ignoring — exactly how invariant 1's allowlist was missed.
#[test]
fn every_contract_constraint_key_is_accounted_for() {
    const CONSUMED: &[&str] = &[
        "id",
        "if",
        "allowed_truth_state",
        "forbidden_truth_state",
        "required_truth_state",
        "default_truth_state",
        "required_provenance",
        "minimum_coverage",
        "required_claim_kind",
        "required_fields",
        "required_knowledge_status",
        "forbidden_coverage",
        "forbid_synthetic_events",
        "silent_choice",
        "cannot_override",
        "required_vocabulary",
    ];

    let root = matrix();
    let invariants = root
        .get("invariants")
        .and_then(Value::as_array)
        .expect("invariants");

    let mut unknown = Vec::new();
    for inv in invariants {
        let Value::Object(map) = inv else {
            panic!("invariant is not an object");
        };
        for key in map.keys() {
            if !CONSUMED.contains(&key.as_str()) {
                unknown.push(format!(
                    "{}::{key}",
                    inv.get("id").and_then(Value::as_str).unwrap_or("<no id>")
                ));
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "contract uses constraint keys no test consumes: {unknown:?}"
    );
}
