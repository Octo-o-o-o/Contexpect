//! Parity between the Rust truth model and the frozen acceptance contract.
//!
//! `acceptance/claim-validity-matrix.yaml` is the authority. If either side
//! drifts — a renamed axis value, a dropped reason code, an invariant that no
//! longer bites — these tests fail. That is the point: the contract and the code
//! must not be able to disagree silently.

use ctxpect_core::axes::{
    ClaimKind, Coverage, EffectDecision, KnowledgeStatus, LifecycleStage, Precision, Provenance,
    ReconciliationState, TruthState, UseEvidenceKind,
};
use ctxpect_core::claim::{Claim, ExperimentRef, InvariantId};
use ctxpect_core::reason::UnknownReason;
use ctxpect_schema::{parse, Value};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/ctxpect-core.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repository root")
        .to_path_buf()
}

fn matrix() -> Value {
    let path = repo_root().join("acceptance/claim-validity-matrix.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    parse(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn frozen_list<'a>(root: &'a Value, path: &[&str]) -> Vec<&'a str> {
    let mut cursor = root;
    for key in path {
        cursor = cursor
            .get(key)
            .unwrap_or_else(|| panic!("contract is missing {}", path.join(".")));
    }
    cursor
        .as_array()
        .unwrap_or_else(|| panic!("{} is not an array", path.join(".")))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{} holds a non-string", path.join(".")))
        })
        .collect()
}

fn assert_axis(root: &Value, axis: &str, actual: Vec<&str>) {
    let frozen = frozen_list(root, &["axes", axis]);
    assert_eq!(
        actual, frozen,
        "axis `{axis}` drifted from the frozen contract"
    );
}

#[test]
fn every_axis_matches_the_frozen_contract() {
    let root = matrix();
    assert_axis(
        &root,
        "claim_kind",
        ClaimKind::ALL.iter().map(|v| v.as_str()).collect(),
    );
    assert_axis(
        &root,
        "coverage",
        Coverage::ALL.iter().map(|v| v.as_str()).collect(),
    );
    assert_axis(
        &root,
        "knowledge_status",
        KnowledgeStatus::ALL.iter().map(|v| v.as_str()).collect(),
    );
    assert_axis(
        &root,
        "lifecycle_stage",
        LifecycleStage::ALL.iter().map(|v| v.as_str()).collect(),
    );
    assert_axis(
        &root,
        "precision",
        Precision::ALL.iter().map(|v| v.as_str()).collect(),
    );
    assert_axis(
        &root,
        "provenance",
        Provenance::ALL.iter().map(|v| v.as_str()).collect(),
    );
    assert_axis(
        &root,
        "truth_state",
        TruthState::ALL.iter().map(|v| v.as_str()).collect(),
    );
}

#[test]
fn reason_codes_states_and_kinds_match_the_frozen_contract() {
    let root = matrix();
    assert_eq!(
        UnknownReason::ALL
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>(),
        frozen_list(&root, &["unknown_reason_codes"]),
        "unknown reason codes drifted"
    );
    assert_eq!(
        ReconciliationState::ALL
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>(),
        frozen_list(&root, &["reconciliation_states"]),
        "reconciliation states drifted"
    );
    assert_eq!(
        UseEvidenceKind::ALL
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>(),
        frozen_list(&root, &["use_evidence_kinds"]),
        "use-evidence kinds drifted"
    );
}

#[test]
fn provenance_precedence_matches_the_frozen_order() {
    let root = matrix();
    let frozen = frozen_list(&root, &["precedence"]);
    let ours: Vec<&str> = {
        let mut all: Vec<Provenance> = Provenance::ALL.to_vec();
        all.sort_by_key(|p| p.precedence());
        all.into_iter().map(Provenance::as_str).collect()
    };
    assert_eq!(ours, frozen, "provenance precedence drifted");
}

#[test]
fn every_frozen_invariant_has_an_implementation() {
    let root = matrix();
    let frozen: Vec<&str> = root
        .get("invariants")
        .and_then(Value::as_array)
        .expect("invariants")
        .iter()
        .map(|item| item.get("id").and_then(Value::as_str).expect("invariant id"))
        .collect();

    let ours: Vec<&str> = InvariantId::ALL.iter().map(|id| id.as_str()).collect();
    assert_eq!(ours, frozen, "invariant set or order drifted");

    for id in &frozen {
        assert!(
            InvariantId::from_wire(id).is_some(),
            "no implementation for invariant `{id}`"
        );
    }
}

/// Build a claim from one example row of the frozen contract.
///
/// Fields the row does not mention take honest defaults, so an example that only
/// pins `claim_kind` + `lifecycle_stage` + `truth_state` is judged on exactly
/// that, not on incidental extras.
fn claim_from_example(row: &Value) -> Claim {
    let field = |key: &str| row.get(key).and_then(Value::as_str);
    let flag = |key: &str| row.get(key).and_then(Value::as_bool).unwrap_or(false);

    let truth_state = field("truth_state")
        .and_then(TruthState::from_wire)
        .unwrap_or(TruthState::Indeterminate);
    let lifecycle_stage = field("lifecycle_stage")
        .and_then(LifecycleStage::from_wire)
        .unwrap_or(LifecycleStage::Installed);
    let claim_kind = field("claim_kind")
        .and_then(ClaimKind::from_wire)
        .unwrap_or(ClaimKind::Observed);

    // Defaults are chosen so they never themselves trip an invariant: an example
    // must fail because of what it states, not because of what it omits.
    let provenance = field("provenance")
        .and_then(Provenance::from_wire)
        .unwrap_or(Provenance::NativeRuntime);
    let coverage = field("coverage")
        .and_then(Coverage::from_wire)
        .unwrap_or(Coverage::FullDeclaredSurface);
    let knowledge_status = field("knowledge_status")
        .and_then(KnowledgeStatus::from_wire)
        .unwrap_or(KnowledgeStatus::Current);
    let precision = field("precision")
        .and_then(Precision::from_wire)
        .unwrap_or(Precision::Exact);

    let decision = field("decision").and_then(EffectDecision::from_wire);
    let experiment = ExperimentRef {
        experiment_id: row
            .get("experiment_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            // A supported effect example that does not mention the citation is
            // testing something else; give it one so only the stated defect bites.
            .or_else(|| {
                (claim_kind == ClaimKind::Effect).then(|| "exp-fixture".to_string())
            }),
        contract_digest: row
            .get("contract_digest")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| (claim_kind == ClaimKind::Effect).then(|| "digest-fixture".to_string())),
    };

    Claim {
        claim_kind,
        coverage,
        knowledge_status,
        lifecycle_stage,
        precision,
        provenance,
        truth_state,
        use_evidence_kind: field("use_evidence_kind").and_then(UseEvidenceKind::from_wire),
        decision,
        experiment,
        unknown_reason: (truth_state == TruthState::Indeterminate)
            .then_some(UnknownReason::SurfaceNotExposed),
        capability_unexposed: flag("capability_unexposed"),
        has_timeline_events: flag("has_timeline_events"),
        contradicted_by_equal_coverage: flag("contradictory") || flag("two_current_same_coverage"),
        filled_from_higher_provenance_outside_coverage: flag("higher_provenance_partial"),
    }
}

fn examples(root: &Value, key: &str) -> Vec<Value> {
    root.get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("contract is missing {key}"))
        .to_vec()
}

#[test]
fn legal_examples_are_expressible() {
    let root = matrix();
    let rows = examples(&root, "legal_examples");
    assert!(!rows.is_empty(), "contract declares no legal examples");
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("<no id>");
        let claim = claim_from_example(&row);
        assert!(
            claim.is_expressible(),
            "legal example `{id}` was rejected: {:?}",
            claim.violations()
        );
    }
}

#[test]
fn illegal_examples_are_rejected() {
    let root = matrix();
    let rows = examples(&root, "illegal_examples");
    assert!(!rows.is_empty(), "contract declares no illegal examples");
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("<no id>");
        let claim = claim_from_example(&row);
        let violations = claim.violations();
        assert!(
            !violations.is_empty(),
            "illegal example `{id}` was accepted: {claim:?}"
        );
    }
}

/// The contract's own illegal example, spelled out so a reader can see the rule
/// it exercises without opening the artifact.
#[test]
fn static_resolver_cannot_claim_model_visible_present() {
    let claim = Claim {
        claim_kind: ClaimKind::Resolved,
        lifecycle_stage: LifecycleStage::ModelVisible,
        truth_state: TruthState::Present,
        ..Claim::indeterminate(
            ClaimKind::Resolved,
            LifecycleStage::ModelVisible,
            UnknownReason::SurfaceNotExposed,
        )
    };
    let violated: Vec<InvariantId> = claim.violations().into_iter().map(|v| v.invariant).collect();
    assert!(violated.contains(&InvariantId::ResolvedNoPresentRuntimeFacets));
}

#[test]
fn each_invariant_is_reachable_from_some_claim() {
    // A rule nobody can trip is a rule that does not exist. Every frozen
    // invariant must have at least one claim that violates exactly it.
    let cases: Vec<(InvariantId, Claim)> = vec![
        (
            InvariantId::ResolvedNoPresentRuntimeFacets,
            Claim {
                claim_kind: ClaimKind::Resolved,
                lifecycle_stage: LifecycleStage::UseEvidence,
                truth_state: TruthState::Present,
                ..honest()
            },
        ),
        (
            InvariantId::ModelVisiblePresentRequiresNative,
            Claim {
                lifecycle_stage: LifecycleStage::ModelVisible,
                truth_state: TruthState::Present,
                provenance: Provenance::OfficialSpec,
                ..honest()
            },
        ),
        (
            InvariantId::InternalAttributionNeverPresentWithoutNativeSemantics,
            Claim {
                lifecycle_stage: LifecycleStage::UseEvidence,
                use_evidence_kind: Some(UseEvidenceKind::InternalAttribution),
                truth_state: TruthState::Present,
                ..honest()
            },
        ),
        (
            InvariantId::OutcomeAffectingDecisionFromExperimentOnly,
            Claim {
                claim_kind: ClaimKind::Effect,
                lifecycle_stage: LifecycleStage::OutcomeAffecting,
                decision: Some(EffectDecision::SupportedBeneficial),
                experiment: ExperimentRef::default(),
                truth_state: TruthState::Indeterminate,
                unknown_reason: Some(UnknownReason::EvidenceStale),
                ..honest()
            },
        ),
        (
            InvariantId::InconclusiveIsNotAbsent,
            Claim {
                claim_kind: ClaimKind::Effect,
                lifecycle_stage: LifecycleStage::OutcomeAffecting,
                decision: Some(EffectDecision::Inconclusive),
                truth_state: TruthState::Absent,
                ..honest()
            },
        ),
        (
            InvariantId::UnexposedTimelineNoFakeEvents,
            Claim {
                capability_unexposed: true,
                has_timeline_events: true,
                truth_state: TruthState::Indeterminate,
                unknown_reason: Some(UnknownReason::SurfaceNotExposed),
                ..honest()
            },
        ),
        (
            InvariantId::ConflictKeepConflicted,
            Claim {
                contradicted_by_equal_coverage: true,
                knowledge_status: KnowledgeStatus::Current,
                ..honest()
            },
        ),
        (
            InvariantId::HigherProvenanceCannotFillUncoveredFields,
            Claim {
                filled_from_higher_provenance_outside_coverage: true,
                ..honest()
            },
        ),
        (
            InvariantId::AbsentRequiresSufficientCoverage,
            Claim {
                truth_state: TruthState::Absent,
                coverage: Coverage::Unknown,
                ..honest()
            },
        ),
        (
            InvariantId::IndeterminateRequiresUnknownReason,
            Claim {
                truth_state: TruthState::Indeterminate,
                unknown_reason: None,
                ..honest()
            },
        ),
    ];

    assert_eq!(
        cases.len(),
        InvariantId::ALL.len(),
        "every invariant needs a reachability case"
    );
    for (expected, claim) in cases {
        let violated: Vec<InvariantId> =
            claim.violations().into_iter().map(|v| v.invariant).collect();
        assert!(
            violated.contains(&expected),
            "invariant `{expected}` is unreachable; got {violated:?}"
        );
    }
}

/// A claim that trips nothing, so each reachability case isolates one rule.
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

#[test]
fn the_honest_baseline_trips_nothing() {
    assert!(honest().is_expressible(), "{:?}", honest().violations());
}
