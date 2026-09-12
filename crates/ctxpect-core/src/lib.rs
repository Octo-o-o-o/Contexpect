//! Contexpect's claim-validity truth model.
//!
//! The product's value rests on not overstating what it knows. This crate carries
//! the axes a claim is expressed on, the closed vocabulary of reasons a fact can
//! be unknown, and the honesty invariants every claim must satisfy — all frozen by
//! `acceptance/claim-validity-matrix.yaml` and checked against it by
//! `tests/contract_parity.rs`.
//!
//! Status: the product runtime is not implemented. Collectors, resolvers and the
//! CLI arrive in later work packages; this crate is the model they will speak.

pub mod axes;
pub mod claim;
pub mod reason;

pub use axes::{
    ClaimKind, Coverage, EffectDecision, KnowledgeStatus, LifecycleStage, Precision, Provenance,
    ReconciliationState, TruthState, UseEvidenceKind,
};
pub use claim::{Claim, ClaimSource, ExperimentRef, InvariantId, Violation, source_domain_of};
pub use reason::UnknownReason;
