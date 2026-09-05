//! The claim-validity axes.
//!
//! These are frozen by `acceptance/claim-validity-matrix.yaml`. The parity test
//! in `tests/contract_parity.rs` reads that artifact and fails if either side
//! drifts, so the wire strings below are a contract, not an implementation detail.

use std::fmt;

/// Declares the wire strings for an axis and their frozen order.
macro_rules! axis {
    (
        $(#[$meta:meta])*
        $name:ident { $( $variant:ident => $wire:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $variant ),+
        }

        impl $name {
            /// Every value, in the order the frozen contract declares them.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// The wire string used in Receipts, fixtures and contracts.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $wire ),+
                }
            }

            /// Parse a wire string. Unknown input is rejected rather than guessed:
            /// an unrecognised axis value must fail closed, never fall back.
            #[must_use]
            pub fn from_wire(text: &str) -> Option<$name> {
                match text {
                    $( $wire => Some($name::$variant), )+
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

axis! {
    /// How a claim was established.
    ClaimKind {
        Resolved => "resolved",
        Observed => "observed",
        Effect => "effect",
    }
}

axis! {
    /// How much of the declared surface the evidence actually covers.
    Coverage {
        FullDeclaredSurface => "full-declared-surface",
        PartialDeclaredSurface => "partial-declared-surface",
        Unknown => "unknown",
    }
}

axis! {
    /// Freshness and consistency of what is known.
    KnowledgeStatus {
        Current => "current",
        Stale => "stale",
        Conflicted => "conflicted",
        Unknown => "unknown",
    }
}

axis! {
    /// How far along the pipeline the claim reaches. Later stages are runtime
    /// facets a static resolver cannot see.
    LifecycleStage {
        Installed => "installed",
        Discoverable => "discoverable",
        Eligible => "eligible",
        ModelVisible => "model-visible",
        UseEvidence => "use-evidence",
        OutcomeAffecting => "outcome-affecting",
    }
}

axis! {
    /// How precise the value is.
    Precision {
        Exact => "exact",
        Derived => "derived",
        Estimated => "estimated",
        NotApplicable => "not-applicable",
    }
}

axis! {
    /// Where the evidence came from, strongest first.
    Provenance {
        NativeRuntime => "native-runtime",
        NativeLog => "native-log",
        HarnessSource => "harness-source",
        OfficialSpec => "official-spec",
        Heuristic => "heuristic",
        UserAttested => "user-attested",
    }
}

axis! {
    /// What the claim asserts about the world.
    TruthState {
        Present => "present",
        Absent => "absent",
        Indeterminate => "indeterminate",
        NotApplicable => "not-applicable",
    }
}

axis! {
    /// Whether a projection was reconciled against the harness itself.
    ReconciliationState {
        Verified => "verified",
        StructuralOnly => "structural-only",
        Indeterminate => "indeterminate",
        Failed => "failed",
    }
}

axis! {
    /// The kinds of evidence that a capability was actually used.
    UseEvidenceKind {
        InvocationObserved => "invocation-observed",
        ReferenceObserved => "reference-observed",
        BehaviorConsistent => "behavior-consistent",
        InternalAttribution => "internal-attribution",
    }
}

axis! {
    /// The verdict of an effect experiment.
    EffectDecision {
        SupportedBeneficial => "supported-beneficial",
        SupportedHarmful => "supported-harmful",
        SupportedEquivalentWithinMargin => "supported-equivalent-within-margin",
        Inconclusive => "inconclusive",
    }
}

impl Coverage {
    /// Ranks coverage for "at least this much" comparisons.
    /// `Unknown` is the weakest: it cannot satisfy any minimum.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Coverage::Unknown => 0,
            Coverage::PartialDeclaredSurface => 1,
            Coverage::FullDeclaredSurface => 2,
        }
    }

    /// True when this coverage is at least `minimum`.
    #[must_use]
    pub const fn at_least(self, minimum: Coverage) -> bool {
        self.rank() >= minimum.rank()
    }
}

impl Provenance {
    /// Precedence rank; lower is stronger, matching the frozen precedence list.
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Provenance::NativeRuntime => 0,
            Provenance::NativeLog => 1,
            Provenance::HarnessSource => 2,
            Provenance::OfficialSpec => 3,
            Provenance::Heuristic => 4,
            Provenance::UserAttested => 5,
        }
    }

    /// True when this source outranks `other`.
    #[must_use]
    pub const fn outranks(self, other: Provenance) -> bool {
        self.precedence() < other.precedence()
    }

    /// Native evidence is the only kind that can witness a runtime facet.
    #[must_use]
    pub const fn is_native(self) -> bool {
        matches!(self, Provenance::NativeRuntime | Provenance::NativeLog)
    }
}

impl LifecycleStage {
    /// Stages a static resolver cannot witness: they are runtime facets.
    #[must_use]
    pub const fn is_runtime_facet(self) -> bool {
        matches!(
            self,
            LifecycleStage::ModelVisible
                | LifecycleStage::UseEvidence
                | LifecycleStage::OutcomeAffecting
        )
    }
}

impl EffectDecision {
    /// A decision that asserts a direction of effect, as opposed to `Inconclusive`.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        !matches!(self, EffectDecision::Inconclusive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_strings_round_trip() {
        for value in TruthState::ALL {
            assert_eq!(TruthState::from_wire(value.as_str()), Some(*value));
        }
        for value in Provenance::ALL {
            assert_eq!(Provenance::from_wire(value.as_str()), Some(*value));
        }
    }

    #[test]
    fn unknown_wire_strings_fail_closed() {
        assert_eq!(TruthState::from_wire("Present"), None);
        assert_eq!(TruthState::from_wire("maybe"), None);
        assert_eq!(Coverage::from_wire(""), None);
        assert_eq!(LifecycleStage::from_wire("model_visible"), None);
    }

    #[test]
    fn coverage_unknown_satisfies_no_minimum() {
        assert!(!Coverage::Unknown.at_least(Coverage::PartialDeclaredSurface));
        assert!(!Coverage::Unknown.at_least(Coverage::FullDeclaredSurface));
        assert!(Coverage::PartialDeclaredSurface.at_least(Coverage::PartialDeclaredSurface));
        assert!(Coverage::FullDeclaredSurface.at_least(Coverage::PartialDeclaredSurface));
    }

    #[test]
    fn provenance_precedence_is_strict_and_ordered() {
        let ranks: Vec<u8> = Provenance::ALL.iter().map(|p| p.precedence()).collect();
        assert_eq!(ranks, vec![0, 1, 2, 3, 4, 5]);
        assert!(Provenance::NativeRuntime.outranks(Provenance::Heuristic));
        assert!(!Provenance::Heuristic.outranks(Provenance::NativeRuntime));
        assert!(!Provenance::NativeRuntime.outranks(Provenance::NativeRuntime));
    }

    #[test]
    fn only_native_provenance_is_native() {
        assert!(Provenance::NativeRuntime.is_native());
        assert!(Provenance::NativeLog.is_native());
        for other in [
            Provenance::HarnessSource,
            Provenance::OfficialSpec,
            Provenance::Heuristic,
            Provenance::UserAttested,
        ] {
            assert!(!other.is_native(), "{other} must not count as native");
        }
    }

    #[test]
    fn runtime_facets_are_the_last_three_stages() {
        assert!(!LifecycleStage::Installed.is_runtime_facet());
        assert!(!LifecycleStage::Discoverable.is_runtime_facet());
        assert!(!LifecycleStage::Eligible.is_runtime_facet());
        assert!(LifecycleStage::ModelVisible.is_runtime_facet());
        assert!(LifecycleStage::UseEvidence.is_runtime_facet());
        assert!(LifecycleStage::OutcomeAffecting.is_runtime_facet());
    }
}
