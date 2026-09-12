//! A claim and the honesty invariants it must satisfy.
//!
//! The invariants are frozen by `acceptance/claim-validity-matrix.yaml`. They
//! exist to stop the product from overstating what it knows: a static resolver
//! may not report a runtime facet as present, an internal attribution is not
//! proof, "inconclusive" is not "absent", and a capability that was never
//! exposed produces no timeline events.

use crate::axes::{
    ClaimKind, Coverage, EffectDecision, KnowledgeStatus, LifecycleStage, Precision, Provenance,
    TruthState, UseEvidenceKind,
};
use crate::reason::UnknownReason;
use std::fmt;

/// The identifiers of the frozen invariants, in contract order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvariantId {
    ResolvedNoPresentRuntimeFacets,
    ModelVisiblePresentRequiresNative,
    InternalAttributionNeverPresentWithoutNativeSemantics,
    OutcomeAffectingDecisionFromExperimentOnly,
    InconclusiveIsNotAbsent,
    UnexposedTimelineNoFakeEvents,
    ConflictKeepConflicted,
    HigherProvenanceCannotFillUncoveredFields,
    AbsentRequiresSufficientCoverage,
    IndeterminateRequiresUnknownReason,
}

impl InvariantId {
    pub const ALL: &'static [InvariantId] = &[
        InvariantId::ResolvedNoPresentRuntimeFacets,
        InvariantId::ModelVisiblePresentRequiresNative,
        InvariantId::InternalAttributionNeverPresentWithoutNativeSemantics,
        InvariantId::OutcomeAffectingDecisionFromExperimentOnly,
        InvariantId::InconclusiveIsNotAbsent,
        InvariantId::UnexposedTimelineNoFakeEvents,
        InvariantId::ConflictKeepConflicted,
        InvariantId::HigherProvenanceCannotFillUncoveredFields,
        InvariantId::AbsentRequiresSufficientCoverage,
        InvariantId::IndeterminateRequiresUnknownReason,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            InvariantId::ResolvedNoPresentRuntimeFacets => "resolved-no-present-runtime-facets",
            InvariantId::ModelVisiblePresentRequiresNative => "model-visible-present-requires-native",
            InvariantId::InternalAttributionNeverPresentWithoutNativeSemantics => {
                "internal-attribution-never-present-without-native-semantics"
            }
            InvariantId::OutcomeAffectingDecisionFromExperimentOnly => {
                "outcome-affecting-decision-from-experiment-only"
            }
            InvariantId::InconclusiveIsNotAbsent => "inconclusive-is-not-absent",
            InvariantId::UnexposedTimelineNoFakeEvents => "unexposed-timeline-no-fake-events",
            InvariantId::ConflictKeepConflicted => "conflict-keep-conflicted",
            InvariantId::HigherProvenanceCannotFillUncoveredFields => {
                "higher-provenance-cannot-fill-uncovered-fields"
            }
            InvariantId::AbsentRequiresSufficientCoverage => "absent-requires-sufficient-coverage",
            InvariantId::IndeterminateRequiresUnknownReason => {
                "indeterminate-requires-unknown-reason"
            }
        }
    }

    #[must_use]
    pub fn from_wire(text: &str) -> Option<InvariantId> {
        InvariantId::ALL
            .iter()
            .copied()
            .find(|id| id.as_str() == text)
    }
}

impl fmt::Display for InvariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A violated invariant and why it was violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub invariant: InvariantId,
    pub detail: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.invariant, self.detail)
    }
}

/// The experiment a supported outcome-affecting decision must cite.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExperimentRef {
    pub experiment_id: Option<String>,
    pub contract_digest: Option<String>,
}

impl ExperimentRef {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(&self.experiment_id, Some(id) if !id.is_empty())
            && matches!(&self.contract_digest, Some(d) if !d.is_empty())
    }

    /// The required fields that are missing, in contract order.
    #[must_use]
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !matches!(&self.experiment_id, Some(id) if !id.is_empty()) {
            missing.push("experiment_id");
        }
        if !matches!(&self.contract_digest, Some(d) if !d.is_empty()) {
            missing.push("contract_digest");
        }
        missing
    }
}

/// Optional source metadata: who produced this claim, on what basis, when,
/// and from which source domain (C-F04: a claim must not manufacture its own
/// facts). Every field is optional; an absent `source` means the producer was
/// never recorded, which is itself honest — the claim must not invent one.
/// `source_domain` is derived from [`Provenance`] at the production site, not
/// asserted independently.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClaimSource {
    /// The rule/module that produced the claim (e.g. a resolver function).
    pub producer: Option<String>,
    /// The basis the claim rests on (an edge rule id, an evidence id).
    pub basis: Option<String>,
    /// When the claim was evaluated, when an explicit clock reading exists.
    pub evaluated_at: Option<String>,
    /// The trust domain the claim was produced in, derived from `provenance`.
    pub source_domain: Option<String>,
}

impl ClaimSource {
    /// Source metadata naming a producer, with the domain derived from the
    /// claim's provenance. Basis and evaluation time stay unset unless the
    /// production site can cite them.
    #[must_use]
    pub fn produced_by(producer: impl Into<String>, provenance: Provenance) -> Self {
        ClaimSource {
            producer: Some(producer.into()),
            basis: None,
            evaluated_at: None,
            source_domain: Some(source_domain_of(provenance).to_string()),
        }
    }
}

/// The trust domain a provenance axis belongs to (C-F04): native evidence,
/// static resolution, user attestation or a heuristic guess are different
/// domains and one never implies another.
#[must_use]
pub const fn source_domain_of(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::NativeRuntime | Provenance::NativeLog => "native",
        Provenance::HarnessSource | Provenance::OfficialSpec => "static-resolution",
        Provenance::UserAttested => "user-attested",
        Provenance::Heuristic => "heuristic",
    }
}

/// One assertion about one coordinate, with everything needed to judge honesty.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    pub claim_kind: ClaimKind,
    pub coverage: Coverage,
    pub knowledge_status: KnowledgeStatus,
    pub lifecycle_stage: LifecycleStage,
    pub precision: Precision,
    pub provenance: Provenance,
    pub truth_state: TruthState,
    /// Set when the claim rests on observed use.
    pub use_evidence_kind: Option<UseEvidenceKind>,
    /// Set when the claim reports an experiment outcome.
    pub decision: Option<EffectDecision>,
    pub experiment: ExperimentRef,
    /// Why the claim is indeterminate. Invariant 10 requires it whenever
    /// `truth_state` is `Indeterminate`; the closed vocabulary is enforced by the type.
    pub unknown_reason: Option<UnknownReason>,
    /// The harness never exposed this capability's surface.
    pub capability_unexposed: bool,
    /// The claim carries timeline events.
    pub has_timeline_events: bool,
    /// Two equally-covering current sources disagree.
    pub contradicted_by_equal_coverage: bool,
    /// This claim's fields were filled from a stronger but only partially
    /// covering source.
    pub filled_from_higher_provenance_outside_coverage: bool,
    /// Who produced this claim and on what basis (C-F04). `None` is the
    /// honest answer when no producer was recorded.
    pub source: Option<ClaimSource>,
}

impl Claim {
    /// A minimally honest claim: nothing asserted, nothing claimed.
    #[must_use]
    pub fn indeterminate(
        claim_kind: ClaimKind,
        lifecycle_stage: LifecycleStage,
        reason: UnknownReason,
    ) -> Self {
        Claim {
            claim_kind,
            coverage: Coverage::Unknown,
            knowledge_status: KnowledgeStatus::Unknown,
            lifecycle_stage,
            precision: Precision::NotApplicable,
            provenance: Provenance::Heuristic,
            truth_state: TruthState::Indeterminate,
            use_evidence_kind: None,
            decision: None,
            experiment: ExperimentRef::default(),
            unknown_reason: Some(reason),
            capability_unexposed: false,
            has_timeline_events: false,
            contradicted_by_equal_coverage: false,
            filled_from_higher_provenance_outside_coverage: false,
            source: None,
        }
    }

    /// Judge the claim against every frozen invariant.
    ///
    /// Returns the violations in contract order; an empty vector means the claim
    /// is expressible. This never repairs the claim: honesty is the caller's
    /// responsibility, and silently downgrading a claim would hide the defect.
    #[must_use]
    pub fn violations(&self) -> Vec<Violation> {
        let mut found = Vec::new();

        // 1. A resolver reads configuration; it cannot witness a runtime facet.
        // The contract allows only indeterminate or not-applicable here: asserting
        // that a runtime facet is *absent* is the same dishonesty as asserting it
        // is present, since a static read cannot establish either.
        if self.claim_kind == ClaimKind::Resolved
            && self.lifecycle_stage.is_runtime_facet()
            && !matches!(
                self.truth_state,
                TruthState::Indeterminate | TruthState::NotApplicable
            )
        {
            found.push(Violation {
                invariant: InvariantId::ResolvedNoPresentRuntimeFacets,
                detail: format!(
                    "resolved claim reports {} at {}; only indeterminate or not-applicable is honest here",
                    self.truth_state, self.lifecycle_stage
                ),
            });
        }

        // 2. "The model saw it" is a native-runtime fact, not an inference.
        if self.lifecycle_stage == LifecycleStage::ModelVisible
            && self.truth_state == TruthState::Present
        {
            if !self.provenance.is_native() {
                found.push(Violation {
                    invariant: InvariantId::ModelVisiblePresentRequiresNative,
                    detail: format!(
                        "model-visible present requires native-runtime or native-log provenance, got {}",
                        self.provenance
                    ),
                });
            }
            if !self.coverage.at_least(Coverage::PartialDeclaredSurface) {
                found.push(Violation {
                    invariant: InvariantId::ModelVisiblePresentRequiresNative,
                    detail: format!(
                        "model-visible present requires at least partial-declared-surface coverage, got {}",
                        self.coverage
                    ),
                });
            }
        }

        // 3. A model saying it used something is not evidence that it did.
        if self.use_evidence_kind == Some(UseEvidenceKind::InternalAttribution)
            && self.truth_state == TruthState::Present
        {
            found.push(Violation {
                invariant: InvariantId::InternalAttributionNeverPresentWithoutNativeSemantics,
                detail:
                    "internal-attribution cannot establish present; the honest default is indeterminate"
                        .to_string(),
            });
        }

        // 4. A claim about outcomes must come from an experiment, and cite it.
        if self.lifecycle_stage == LifecycleStage::OutcomeAffecting
            && self.decision.is_some_and(EffectDecision::is_supported)
        {
            if self.claim_kind != ClaimKind::Effect {
                found.push(Violation {
                    invariant: InvariantId::OutcomeAffectingDecisionFromExperimentOnly,
                    detail: format!(
                        "a supported outcome-affecting decision requires an effect claim, got {}",
                        self.claim_kind
                    ),
                });
            }
            let missing = self.experiment.missing_fields();
            if !missing.is_empty() {
                found.push(Violation {
                    invariant: InvariantId::OutcomeAffectingDecisionFromExperimentOnly,
                    detail: format!(
                        "a supported outcome-affecting decision must cite {}",
                        missing.join(" and ")
                    ),
                });
            }
        }

        // 5. Failing to find an effect is not finding its absence.
        if self.decision == Some(EffectDecision::Inconclusive)
            && self.truth_state == TruthState::Absent
        {
            found.push(Violation {
                invariant: InvariantId::InconclusiveIsNotAbsent,
                detail: "an inconclusive decision cannot be reported as absent".to_string(),
            });
        }

        // 6. No surface, no events. An empty timeline is the honest answer.
        if self.capability_unexposed {
            if self.truth_state != TruthState::Indeterminate {
                found.push(Violation {
                    invariant: InvariantId::UnexposedTimelineNoFakeEvents,
                    detail: format!(
                        "an unexposed capability must stay indeterminate, got {}",
                        self.truth_state
                    ),
                });
            }
            if self.has_timeline_events {
                found.push(Violation {
                    invariant: InvariantId::UnexposedTimelineNoFakeEvents,
                    detail: "an unexposed capability cannot carry timeline events".to_string(),
                });
            }
        }

        // 7. Disagreement is a finding, not something to quietly resolve.
        if self.contradicted_by_equal_coverage
            && self.knowledge_status != KnowledgeStatus::Conflicted
        {
            found.push(Violation {
                invariant: InvariantId::ConflictKeepConflicted,
                detail: format!(
                    "two current sources of equal coverage contradict each other; status must stay conflicted, got {}",
                    self.knowledge_status
                ),
            });
        }

        // 8. Outranking a source does not extend its reach.
        if self.filled_from_higher_provenance_outside_coverage {
            found.push(Violation {
                invariant: InvariantId::HigherProvenanceCannotFillUncoveredFields,
                detail:
                    "a higher-provenance partial source cannot override fields outside its coverage"
                        .to_string(),
            });
        }

        // 9. "Not found" is only meaningful if you looked far enough.
        if self.truth_state == TruthState::Absent && self.coverage == Coverage::Unknown {
            found.push(Violation {
                invariant: InvariantId::AbsentRequiresSufficientCoverage,
                detail: "absent cannot rest on unknown coverage".to_string(),
            });
        }

        // 10. "Unknown" is only honest if it says why. The vocabulary is closed,
        // so the caller must name one of the declared reasons rather than leaving
        // the gap unexplained.
        if self.truth_state == TruthState::Indeterminate && self.unknown_reason.is_none() {
            found.push(Violation {
                invariant: InvariantId::IndeterminateRequiresUnknownReason,
                detail: "an indeterminate claim must name a declared unknown reason".to_string(),
            });
        }

        found.sort_by_key(|violation| violation.invariant);
        found
    }

    /// True when the claim violates no invariant.
    #[must_use]
    pub fn is_expressible(&self) -> bool {
        self.violations().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Claim {
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
            source: None,
        }
    }

    fn ids(claim: &Claim) -> Vec<InvariantId> {
        claim.violations().into_iter().map(|v| v.invariant).collect()
    }

    #[test]
    fn a_plain_observed_claim_is_expressible() {
        assert!(base().is_expressible(), "{:?}", base().violations());
    }

    #[test]
    fn resolved_cannot_report_runtime_facets_present() {
        for stage in [
            LifecycleStage::ModelVisible,
            LifecycleStage::UseEvidence,
            LifecycleStage::OutcomeAffecting,
        ] {
            let claim = Claim {
                claim_kind: ClaimKind::Resolved,
                lifecycle_stage: stage,
                truth_state: TruthState::Present,
                ..base()
            };
            assert!(
                ids(&claim).contains(&InvariantId::ResolvedNoPresentRuntimeFacets),
                "{stage} must be rejected for a resolved claim"
            );
        }
    }

    #[test]
    fn resolved_may_report_static_stages_present() {
        for stage in [
            LifecycleStage::Installed,
            LifecycleStage::Discoverable,
            LifecycleStage::Eligible,
        ] {
            let claim = Claim {
                claim_kind: ClaimKind::Resolved,
                lifecycle_stage: stage,
                truth_state: TruthState::Present,
                ..base()
            };
            assert!(claim.is_expressible(), "{stage} should be expressible");
        }
    }

    #[test]
    fn resolved_cannot_report_runtime_facets_absent_either() {
        // The contract's allowed_truth_state is a whitelist, not just a ban on
        // `present`: a static resolver cannot establish absence at a runtime facet.
        for stage in [
            LifecycleStage::ModelVisible,
            LifecycleStage::UseEvidence,
            LifecycleStage::OutcomeAffecting,
        ] {
            let claim = Claim {
                claim_kind: ClaimKind::Resolved,
                lifecycle_stage: stage,
                truth_state: TruthState::Absent,
                coverage: Coverage::FullDeclaredSurface,
                ..base()
            };
            assert!(
                ids(&claim).contains(&InvariantId::ResolvedNoPresentRuntimeFacets),
                "{stage} absent must be rejected for a resolved claim"
            );
        }
    }

    #[test]
    fn resolved_may_report_not_applicable_at_runtime_facets() {
        for stage in [
            LifecycleStage::ModelVisible,
            LifecycleStage::UseEvidence,
            LifecycleStage::OutcomeAffecting,
        ] {
            let claim = Claim {
                claim_kind: ClaimKind::Resolved,
                lifecycle_stage: stage,
                truth_state: TruthState::NotApplicable,
                ..base()
            };
            assert!(claim.is_expressible(), "{stage}: {:?}", claim.violations());
        }
    }

    #[test]
    fn resolved_may_stay_indeterminate_at_runtime_facets() {
        let claim = Claim {
            claim_kind: ClaimKind::Resolved,
            lifecycle_stage: LifecycleStage::ModelVisible,
            truth_state: TruthState::Indeterminate,
            unknown_reason: Some(UnknownReason::SurfaceNotExposed),
            ..base()
        };
        assert!(claim.is_expressible(), "{:?}", claim.violations());
    }

    #[test]
    fn model_visible_present_needs_native_provenance_and_coverage() {
        let weak_source = Claim {
            lifecycle_stage: LifecycleStage::ModelVisible,
            truth_state: TruthState::Present,
            provenance: Provenance::Heuristic,
            ..base()
        };
        assert!(ids(&weak_source).contains(&InvariantId::ModelVisiblePresentRequiresNative));

        let weak_coverage = Claim {
            lifecycle_stage: LifecycleStage::ModelVisible,
            truth_state: TruthState::Present,
            coverage: Coverage::Unknown,
            ..base()
        };
        assert!(ids(&weak_coverage).contains(&InvariantId::ModelVisiblePresentRequiresNative));

        let ok = Claim {
            lifecycle_stage: LifecycleStage::ModelVisible,
            truth_state: TruthState::Present,
            provenance: Provenance::NativeLog,
            coverage: Coverage::PartialDeclaredSurface,
            ..base()
        };
        assert!(ok.is_expressible(), "{:?}", ok.violations());
    }

    #[test]
    fn internal_attribution_cannot_be_present() {
        let claim = Claim {
            lifecycle_stage: LifecycleStage::UseEvidence,
            use_evidence_kind: Some(UseEvidenceKind::InternalAttribution),
            truth_state: TruthState::Present,
            ..base()
        };
        assert!(
            ids(&claim)
                .contains(&InvariantId::InternalAttributionNeverPresentWithoutNativeSemantics)
        );

        let observed = Claim {
            use_evidence_kind: Some(UseEvidenceKind::InvocationObserved),
            ..claim.clone()
        };
        assert!(observed.is_expressible(), "{:?}", observed.violations());
    }

    #[test]
    fn supported_outcome_decisions_require_an_effect_experiment() {
        let wrong_kind = Claim {
            claim_kind: ClaimKind::Observed,
            lifecycle_stage: LifecycleStage::OutcomeAffecting,
            truth_state: TruthState::Indeterminate,
            unknown_reason: Some(UnknownReason::EvidenceStale),
            decision: Some(EffectDecision::SupportedBeneficial),
            experiment: ExperimentRef {
                experiment_id: Some("exp-1".into()),
                contract_digest: Some("d".into()),
            },
            ..base()
        };
        assert!(
            ids(&wrong_kind).contains(&InvariantId::OutcomeAffectingDecisionFromExperimentOnly)
        );

        let missing_citation = Claim {
            claim_kind: ClaimKind::Effect,
            experiment: ExperimentRef::default(),
            ..wrong_kind.clone()
        };
        assert!(
            ids(&missing_citation)
                .contains(&InvariantId::OutcomeAffectingDecisionFromExperimentOnly)
        );

        let complete = Claim {
            claim_kind: ClaimKind::Effect,
            experiment: ExperimentRef {
                experiment_id: Some("exp-1".into()),
                contract_digest: Some("d".into()),
            },
            ..wrong_kind
        };
        assert!(complete.is_expressible(), "{:?}", complete.violations());
    }

    #[test]
    fn inconclusive_is_not_absent() {
        let claim = Claim {
            claim_kind: ClaimKind::Effect,
            lifecycle_stage: LifecycleStage::OutcomeAffecting,
            decision: Some(EffectDecision::Inconclusive),
            truth_state: TruthState::Absent,
            ..base()
        };
        assert!(ids(&claim).contains(&InvariantId::InconclusiveIsNotAbsent));

        let honest = Claim {
            truth_state: TruthState::Indeterminate,
            unknown_reason: Some(UnknownReason::EvidenceStale),
            ..claim
        };
        assert!(honest.is_expressible(), "{:?}", honest.violations());
    }

    #[test]
    fn unexposed_capabilities_have_no_events_and_no_verdict() {
        let fabricated = Claim {
            capability_unexposed: true,
            truth_state: TruthState::Present,
            has_timeline_events: true,
            ..base()
        };
        let found = ids(&fabricated);
        assert_eq!(
            found
                .iter()
                .filter(|id| **id == InvariantId::UnexposedTimelineNoFakeEvents)
                .count(),
            2,
            "both the verdict and the fabricated events must be flagged"
        );

        let honest = Claim {
            capability_unexposed: true,
            truth_state: TruthState::Indeterminate,
            unknown_reason: Some(UnknownReason::SurfaceNotExposed),
            has_timeline_events: false,
            ..base()
        };
        assert!(honest.is_expressible(), "{:?}", honest.violations());
    }

    #[test]
    fn contradictions_stay_conflicted() {
        for status in [
            KnowledgeStatus::Current,
            KnowledgeStatus::Stale,
            KnowledgeStatus::Unknown,
        ] {
            let claim = Claim {
                contradicted_by_equal_coverage: true,
                knowledge_status: status,
                ..base()
            };
            assert!(
                ids(&claim).contains(&InvariantId::ConflictKeepConflicted),
                "{status} must not silently resolve a contradiction"
            );
        }
        let claim = Claim {
            contradicted_by_equal_coverage: true,
            knowledge_status: KnowledgeStatus::Conflicted,
            ..base()
        };
        assert!(claim.is_expressible(), "{:?}", claim.violations());
    }

    #[test]
    fn higher_provenance_cannot_fill_uncovered_fields() {
        let claim = Claim {
            filled_from_higher_provenance_outside_coverage: true,
            ..base()
        };
        assert!(ids(&claim).contains(&InvariantId::HigherProvenanceCannotFillUncoveredFields));
    }

    #[test]
    fn absent_requires_coverage() {
        let claim = Claim {
            truth_state: TruthState::Absent,
            coverage: Coverage::Unknown,
            ..base()
        };
        assert!(ids(&claim).contains(&InvariantId::AbsentRequiresSufficientCoverage));

        let covered = Claim {
            truth_state: TruthState::Absent,
            coverage: Coverage::PartialDeclaredSurface,
            ..base()
        };
        assert!(covered.is_expressible(), "{:?}", covered.violations());
    }

    #[test]
    fn indeterminate_without_a_reason_is_rejected() {
        let silent = Claim {
            truth_state: TruthState::Indeterminate,
            unknown_reason: None,
            ..base()
        };
        assert!(ids(&silent).contains(&InvariantId::IndeterminateRequiresUnknownReason));

        let stated = Claim {
            unknown_reason: Some(UnknownReason::SurfaceNotExposed),
            ..silent
        };
        assert!(stated.is_expressible(), "{:?}", stated.violations());
    }

    #[test]
    fn violations_are_reported_in_contract_order() {
        let claim = Claim {
            claim_kind: ClaimKind::Resolved,
            lifecycle_stage: LifecycleStage::ModelVisible,
            truth_state: TruthState::Present,
            provenance: Provenance::Heuristic,
            coverage: Coverage::Unknown,
            contradicted_by_equal_coverage: true,
            ..base()
        };
        let found = ids(&claim);
        let mut sorted = found.clone();
        sorted.sort_unstable();
        assert_eq!(found, sorted);
        assert!(found.contains(&InvariantId::ResolvedNoPresentRuntimeFacets));
        assert!(found.contains(&InvariantId::ModelVisiblePresentRequiresNative));
        assert!(found.contains(&InvariantId::ConflictKeepConflicted));
    }

    #[test]
    fn the_indeterminate_constructor_is_always_expressible() {
        for kind in ClaimKind::ALL {
            for stage in LifecycleStage::ALL {
                let claim =
                    Claim::indeterminate(*kind, *stage, UnknownReason::SurfaceNotExposed);
                assert!(
                    claim.is_expressible(),
                    "{kind}/{stage}: {:?}",
                    claim.violations()
                );
            }
        }
    }

    #[test]
    fn invariant_ids_round_trip() {
        for id in InvariantId::ALL {
            assert_eq!(InvariantId::from_wire(id.as_str()), Some(*id));
        }
        assert_eq!(InvariantId::from_wire("not-an-invariant"), None);
    }

    /// C-F04: source metadata is optional, never manufactured, and orthogonal
    /// to the honesty invariants; the domain is derived from provenance, not
    /// asserted independently.
    #[test]
    fn claim_source_is_optional_and_derives_its_domain_from_provenance() {
        let claim = base();
        assert_eq!(claim.source, None, "no producer was recorded; none is invented");

        let sourced = Claim {
            source: Some(ClaimSource::produced_by(
                "ctxpect-resolve::resolved_claim",
                Provenance::OfficialSpec,
            )),
            ..base()
        };
        let source = sourced.source.as_ref().expect("source");
        assert_eq!(source.producer.as_deref(), Some("ctxpect-resolve::resolved_claim"));
        assert_eq!(source.source_domain.as_deref(), Some("static-resolution"));
        assert_eq!(source.basis, None);
        assert_eq!(source.evaluated_at, None);
        // Presence or absence of source metadata changes no invariant verdict.
        assert_eq!(claim.violations(), sourced.violations());

        for (provenance, domain) in [
            (Provenance::NativeRuntime, "native"),
            (Provenance::NativeLog, "native"),
            (Provenance::HarnessSource, "static-resolution"),
            (Provenance::OfficialSpec, "static-resolution"),
            (Provenance::UserAttested, "user-attested"),
            (Provenance::Heuristic, "heuristic"),
        ] {
            assert_eq!(source_domain_of(provenance), domain, "{provenance}");
        }
    }
}
