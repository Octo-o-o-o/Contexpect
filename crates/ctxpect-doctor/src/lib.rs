//! Deterministic Doctor. LLM output is never a finding.

use ctxpect_policy::precondition::{self, ActionClass};
use ctxpect_schema::{array, object, sha256_text, string, Value};

pub mod rules;
pub mod secrets;
pub mod suppressions;

pub use rules::{
    Declaration, civil_from_days, days_from_civil, is_blocking_rule, parse_civil_date,
    project_findings, project_findings_in, render_project_findings, ProjectFinding, ScannedFile,
    BLOCKING_RULES, NON_BLOCKING_RULES, STALE_CUTOFF,
};
pub use secrets::{contains_secret, secret_literal, SecretClass};
pub use suppressions::{
    apply_suppressions, SuppressionInput, SUPPRESSIONS_PATH, SUPPRESSIONS_SCHEMA,
    SUPPRESSION_INVALID_RULE,
};

/// Confirmation state of a finding. Distinct from severity and from Unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    Confirmed,
    Suspected,
}

impl Confirmation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Confirmation::Confirmed => "confirmed",
            Confirmation::Suspected => "suspected",
        }
    }
}

/// Severity of a finding. Unknown is not a severity value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Confirmed,
    Suspected,
}

impl Severity {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Severity::Confirmed => "confirmed",
            Severity::Suspected => "suspected",
        }
    }
}

/// Run Doctor rules against a formal Receipt or a development snapshot.
#[must_use]
pub fn diagnose(receipt: &Value) -> Value {
    let mut findings = Vec::new();
    let mut unknown_count = 0i64;

    if let Some(items) = receipt.get("unknown").and_then(Value::as_array) {
        unknown_count += i64::try_from(items.len()).unwrap_or(0);
        for item in items {
            let code = item
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            findings.push(finding(FindingSpec {
                rule_id: "D-UNKNOWN-SURFACE",
                confirmation: Confirmation::Suspected,
                severity: Severity::Suspected,
                title: &format!("Unknown surface ({code})"),
                affected: item
                    .get("family")
                    .and_then(Value::as_str)
                    .unwrap_or("coordinate"),
                impact: "unknown",
                next: "Collect the next evidence named on this claim; do not guess present/absent.",
                treatment_locked: true,
                // The unknown surface is the lock reason (target identity,
                // permission or coordinate class evidence is missing).
                lock_reason: code,
                evidence_state: "indeterminate",
                reason_code: code,
                facet: "unknown-reason",
            }));
        }
    }

    let facets_owned = receipt.get("facets").cloned().or_else(|| {
        receipt
            .get("results")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("facets"))
            .cloned()
    });
    if let Some(facets) = facets_owned.as_ref().and_then(Value::as_object) {
        for (name, claim) in facets {
            let truth = claim.get("truth_state").and_then(Value::as_str);
            let reason = claim
                .get("unknown_reason_code")
                .and_then(Value::as_str)
                .unwrap_or("");
            if truth == Some("indeterminate") {
                unknown_count += 1;
                // C-F03 (Final Recommendations §4.3): a Doctor treatment is a
                // limited static fix, and whether an indeterminate facet locks
                // it is decided by the policy precondition table, not by a
                // blanket lock. model-visible / use-evidence map to
                // runtime-surface evidence and outcome-affecting to the
                // outcome question; neither is required pre-evidence for a
                // static fix, so these facets are reported and counted as
                // Unknown but stay suspected and unlocked. Unknown target
                // identity, permission or policy still locks, via
                // D-UNKNOWN-SURFACE / D-UNSUPPORTED-VERSION.
                let locked = static_fix_locks(name);
                findings.push(finding(FindingSpec {
                    rule_id: &format!("D-FACET-{}", name.to_ascii_uppercase()),
                    confirmation: Confirmation::Suspected,
                    severity: Severity::Suspected,
                    title: &format!("Facet `{name}` is indeterminate"),
                    affected: name,
                    impact: "indeterminate",
                    next: next_for_reason(reason),
                    treatment_locked: locked,
                    lock_reason: if locked { reason } else { "none" },
                    evidence_state: "indeterminate",
                    reason_code: reason,
                    facet: name,
                }));
            }
        }
    }

    if let Some(items) = receipt.get("findings").and_then(Value::as_array) {
        for item in items {
            if item.get("kind").and_then(Value::as_str) == Some("truncated-after") {
                // Same meaning as the corpus rule `cap_truncation`; the id is
                // kept and the alias recorded (docs/process/doctor-rule-map.md).
                findings.push(with_aliases(finding(FindingSpec {
                    rule_id: "D-TRUNCATED",
                    confirmation: Confirmation::Confirmed,
                    severity: Severity::Confirmed,
                    title: "Instruction aggregation was truncated",
                    affected: "instructions",
                    impact: "truncated",
                    next: "Inspect the truncated-after offset; a config.toml override is not read in this slice.",
                    treatment_locked: false,
                    lock_reason: "none",
                    evidence_state: "static-resolution",
                    reason_code: "G3",
                    facet: "instructions",
                }), &["cap_truncation"]));
            }
        }
    }

    if let Some(items) = receipt.get("explanation").and_then(Value::as_array) {
        for item in items {
            match item.get("kind").and_then(Value::as_str) {
                Some("excluded") => findings.push(finding(FindingSpec {
                    rule_id: "D-IGNORE-G4",
                    confirmation: Confirmation::Confirmed,
                    severity: Severity::Confirmed,
                    // C-F01: an observation-scope exclusion; the harness's
                    // native load state stays unknown, it is not absent.
                    title: "A required instruction file was excluded from Contexpect's observation scope by .ctxpect-ignore",
                    affected: "instructions",
                    impact: "excluded",
                    next: "Remove the ignore line if Contexpect should read this file. Whether the harness still loads it natively is unknown (observation_scope_excluded) until native evidence exists.",
                    treatment_locked: false,
                    lock_reason: "none",
                    evidence_state: "static-resolution",
                    reason_code: "G4",
                    facet: "instructions",
                })),
                Some("unknown")
                    if item.get("unknown_reason_code").and_then(Value::as_str)
                        == Some("permission_not_granted") =>
                {
                    findings.push(finding(FindingSpec {
                        rule_id: "D-HOME-NOT-GRANTED",
                        confirmation: Confirmation::Confirmed,
                        severity: Severity::Confirmed,
                        title: "Global Codex home was not granted",
                        affected: "instructions",
                        impact: "permission",
                        next: "Pass an explicit --codex-home directory. HOME/CODEX_HOME are not consulted.",
                        treatment_locked: false,
                        lock_reason: "none",
                        evidence_state: "static-resolution",
                        reason_code: "permission_not_granted",
                        facet: "instructions",
                    }));
                }
                Some("unknown")
                    if item.get("unknown_reason_code").and_then(Value::as_str)
                        == Some("unsupported_harness_version") =>
                {
                    findings.push(finding(FindingSpec {
                        rule_id: "D-UNSUPPORTED-VERSION",
                        confirmation: Confirmation::Confirmed,
                        severity: Severity::Confirmed,
                        title: "Harness version is outside the frozen coordinate",
                        affected: "coordinate",
                        impact: "unsupported-version",
                        next: "Inspect with a frozen version. Unknown versions fail closed and do not borrow the latest grammar.",
                        treatment_locked: true,
                        lock_reason: "unsupported_harness_version",
                        evidence_state: "indeterminate",
                        reason_code: "unsupported_harness_version",
                        facet: "coordinate",
                    }));
                }
                _ => {}
            }
        }
    }

    let mut confirmed = 0i64;
    let mut suspected = 0i64;
    for item in &findings {
        match item.get("confirmation").and_then(Value::as_str) {
            Some("confirmed") => confirmed += 1,
            Some("suspected") => suspected += 1,
            _ => {}
        }
    }

    object([
        ("schema", string("ctxpect-doctor-v1")),
        ("engine", string("deterministic-rules")),
        ("advisor_is_authority", Value::Bool(false)),
        (
            "counts",
            object([
                ("confirmed", Value::Int(confirmed)),
                ("suspected", Value::Int(suspected)),
                ("unknown", Value::Int(unknown_count)),
                (
                    "note",
                    string(
                        "Confirmed/Suspected count findings by confirmation state. Unknown counts knowledge-status/evidence gaps and is not a severity. Counts may overlap a finding that is both confirmed and has an Unknown facet.",
                    ),
                ),
            ]),
        ),
        ("findings", array(findings)),
    ])
}

fn with_aliases(finding: Value, aliases: &[&str]) -> Value {
    match finding {
        Value::Object(mut map) => {
            map.insert(
                "aliases".into(),
                array(aliases.iter().map(|alias| string(*alias))),
            );
            Value::Object(map)
        }
        other => other,
    }
}

/// Merge project-content findings into a diagnosis and recount.
#[must_use]
pub fn with_project_findings(diagnosis: Value, project: &[Value]) -> Value {
    let Value::Object(mut map) = diagnosis else {
        return diagnosis;
    };
    let mut findings = map
        .get("findings")
        .and_then(Value::as_array)
        .map(<[Value]>::to_vec)
        .unwrap_or_default();
    findings.extend(project.iter().cloned());
    let mut confirmed = 0i64;
    let mut suspected = 0i64;
    let mut blocking = 0i64;
    for item in &findings {
        match item.get("confirmation").and_then(Value::as_str) {
            Some("confirmed") => confirmed += 1,
            Some("suspected") => suspected += 1,
            _ => {}
        }
        if item.get("blocking").and_then(Value::as_bool) == Some(true) {
            blocking += 1;
        }
    }
    if let Some(Value::Object(counts)) = map.get_mut("counts") {
        counts.insert("confirmed".into(), Value::Int(confirmed));
        counts.insert("suspected".into(), Value::Int(suspected));
        counts.insert("blocking".into(), Value::Int(blocking));
    }
    map.insert("findings".into(), array(findings));
    map.insert(
        "project_rules".into(),
        object([
            ("namespace", string("doctor-corpus")),
            ("blocking_rules", array(BLOCKING_RULES.iter().map(|r| string(*r)))),
            ("non_blocking_rules", array(NON_BLOCKING_RULES.iter().map(|r| string(*r)))),
        ]),
    );
    Value::Object(map)
}

struct FindingSpec<'a> {
    rule_id: &'a str,
    confirmation: Confirmation,
    severity: Severity,
    title: &'a str,
    affected: &'a str,
    impact: &'a str,
    next: &'a str,
    treatment_locked: bool,
    /// Why treatment is locked; `"none"` when it is not. Locks name the
    /// missing precondition evidence, per the policy precondition table.
    lock_reason: &'a str,
    /// `indeterminate` when the finding's own evidence is indeterminate;
    /// independent of whether treatment is locked.
    evidence_state: &'a str,
    reason_code: &'a str,
    facet: &'a str,
}

/// Whether an indeterminate facet is required pre-evidence for a limited
/// static fix — the action a Doctor treatment is. Routed through the policy
/// precondition table (C-F03): runtime-surface (`model-visible`,
/// `use-evidence`) and outcome (`outcome-affecting`) evidence are not
/// required for that class, so they do not lock treatment.
fn static_fix_locks(facet: &str) -> bool {
    let evidence = match facet {
        "model-visible" | "use-evidence" => "runtime-surface",
        "outcome-affecting" => "outcome",
        other => other,
    };
    precondition::requires(ActionClass::LimitedStaticFix, evidence)
}

fn finding(spec: FindingSpec<'_>) -> Value {
    let FindingSpec {
        rule_id,
        confirmation,
        severity,
        title,
        affected,
        impact,
        next,
        treatment_locked,
        lock_reason,
        evidence_state,
        reason_code,
        facet,
    } = spec;
    let id = format!("f_{}", &sha256_text(&format!("{rule_id}|{title}|{affected}"))[..12]);
    object([
        ("finding_id", string(id)),
        ("rule_id", string(rule_id)),
        ("title", string(title)),
        ("confirmation", string(confirmation.as_str())),
        ("severity", string(severity.as_str())),
        ("unknown_is_severity", Value::Bool(false)),
        ("affected_surfaces", array([string(affected)])),
        ("evidence_state", string(evidence_state)),
        ("impact", string(impact)),
        ("first_seen", string("current-receipt")),
        ("reason_code", string(reason_code)),
        ("facet", string(facet)),
        (
            "treatment",
            object([
                ("locked", Value::Bool(treatment_locked)),
                ("lock_reason", string(lock_reason)),
                (
                    "unlocks_via_export",
                    Value::Bool(false),
                ),
                (
                    "unlocks_via_advisor",
                    Value::Bool(false),
                ),
                (
                    "unlocks_via_user_attestation_alone",
                    Value::Bool(false),
                ),
            ]),
        ),
        (
            "next_evidence",
            array([object([
                ("action", string(next)),
                ("kind", string("evidence")),
            ])]),
        ),
        (
            "placement",
            object([
                ("authority", string("project-file")),
                ("target", string(affected)),
                ("loss", string("none-until-preview")),
            ]),
        ),
    ])
}

fn next_for_reason(reason: &str) -> &'static str {
    match reason {
        "runtime_snapshot_missing" => {
            "Collect a native runtime snapshot (Codex debug prompt-input on 0.147.0, or Grok inspect --json)."
        }
        "permission_not_granted" => "Grant an explicit --codex-home root.",
        "unsupported_harness_version" => "Re-run against a frozen version coordinate.",
        "not_installed" => "Install is not inferred from config residue. Provide a live executable or keep Unknown.",
        "connector_required" => "Provide the declared connector; do not treat config residue as connected.",
        "config_residue_only" => "Config residue is not an installation. Leave installed=Unknown/indeterminate.",
        _ => "Collect the evidence named by the reason code; do not guess.",
    }
}

/// The one blocking judgement `ctxpect ci` and `ctxpect doctor --fail-on`
/// share, as an exit code: 2 when a finding blocks, else 0.
///
/// A finding blocks when it carries `blocking: true` (a corpus-namespace rule
/// whose precision on the frozen Doctor corpus is 1.00), or when the caller
/// asked to fail on `confirmed` and a confirmed finding exists. Suspected and
/// Unknown never block on their own.
#[must_use]
pub fn blocking_exit(diagnosis: &Value, fail_on: Option<&str>) -> i32 {
    let findings = diagnosis
        .get("findings")
        .and_then(Value::as_array)
        .unwrap_or(&[]);
    // A suppressed finding (see `suppressions`) is still reported and
    // counted, but it does not block; the *active* counts decide.
    let any_blocking = findings.iter().any(|item| {
        item.get("blocking").and_then(Value::as_bool) == Some(true)
            && item.get("suppressed").and_then(Value::as_bool) != Some(true)
    });
    if any_blocking {
        return 2;
    }
    let confirmed = diagnosis
        .pointer(&["counts", "active_confirmed"])
        .or_else(|| diagnosis.pointer(&["counts", "confirmed"]))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if fail_on == Some("confirmed") && confirmed > 0 {
        return 2;
    }
    0
}

/// PlacementRecommendation is deterministic and independent of Advisor text.
#[must_use]
pub fn placement_for(finding: &Value) -> Value {
    finding
        .get("placement")
        .cloned()
        .unwrap_or_else(|| {
            object([
                ("authority", string("unknown")),
                ("target", string("unknown")),
                ("loss", string("unknown")),
            ])
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    #[test]
    fn unknown_is_not_a_severity_and_unknown_surface_locks_treatment() {
        let receipt = parse(
            r#"{"facets":{"model-visible":{"truth_state":"indeterminate","unknown_reason_code":"runtime_snapshot_missing"}},"unknown":[{"reason_code":"not_installed","family":"cline"}],"explanation":[],"findings":[]}"#,
        )
        .unwrap();
        let report = diagnose(&receipt);
        let counts = report.get("counts").unwrap();
        assert!(counts.get("unknown").and_then(Value::as_i64).unwrap() >= 1);
        let findings = report.get("findings").and_then(Value::as_array).unwrap();
        // An unknown surface (target identity / permission class) still locks.
        assert!(findings.iter().any(|item| {
            item.pointer(&["treatment", "locked"])
                .and_then(Value::as_bool)
                == Some(true)
                && item
                    .pointer(&["treatment", "unlocks_via_advisor"])
                    .and_then(Value::as_bool)
                    == Some(false)
        }));
        for item in findings {
            assert_eq!(
                item.get("unknown_is_severity").and_then(Value::as_bool),
                Some(false)
            );
            let sev = item.get("severity").and_then(Value::as_str).unwrap();
            assert!(sev == "confirmed" || sev == "suspected", "{sev}");
        }
    }

    /// C-F03: indeterminate runtime-surface and outcome facets are not
    /// required pre-evidence for a limited static fix, so they no longer
    /// lock treatment nor read as confirmed; the finding and the Unknown
    /// count stay.
    #[test]
    fn outcome_and_runtime_surface_unknowns_do_not_lock_a_static_fix() {
        let receipt = parse(
            r#"{"facets":{
                "model-visible":{"truth_state":"indeterminate","unknown_reason_code":"runtime_snapshot_missing"},
                "use-evidence":{"truth_state":"indeterminate","unknown_reason_code":"runtime_snapshot_missing"},
                "outcome-affecting":{"truth_state":"indeterminate","unknown_reason_code":"runtime_snapshot_missing"}
            },"unknown":[],"explanation":[],"findings":[]}"#,
        )
        .unwrap();
        let report = diagnose(&receipt);
        assert_eq!(
            report.pointer(&["counts", "unknown"]).and_then(Value::as_i64),
            Some(3)
        );
        assert_eq!(
            report.pointer(&["counts", "confirmed"]).and_then(Value::as_i64),
            Some(0)
        );
        let findings = report.get("findings").and_then(Value::as_array).unwrap();
        assert_eq!(findings.len(), 3, "{findings:?}");
        for item in findings {
            assert!(item
                .get("rule_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with("D-FACET-")), "{item:?}");
            assert_eq!(
                item.get("confirmation").and_then(Value::as_str),
                Some("suspected")
            );
            assert_eq!(
                item.get("severity").and_then(Value::as_str),
                Some("suspected")
            );
            assert_eq!(
                item.pointer(&["treatment", "locked"]).and_then(Value::as_bool),
                Some(false)
            );
            assert_eq!(
                item.pointer(&["treatment", "lock_reason"]).and_then(Value::as_str),
                Some("none")
            );
            // The evidence gap is still reported as indeterminate; only the
            // lock and the confirmation label changed.
            assert_eq!(
                item.get("evidence_state").and_then(Value::as_str),
                Some("indeterminate")
            );
        }
        // `--fail-on confirmed` does not fire on outcome/visibility unknowns.
        assert_eq!(blocking_exit(&report, Some("confirmed")), 0);
    }

    /// C-F03 the other way: identity/permission/coordinate unknowns keep
    /// locking treatment, and a genuinely confirmed finding still fails
    /// `--fail-on confirmed`.
    #[test]
    fn identity_permission_and_coordinate_unknowns_still_lock() {
        let receipt = parse(
            r#"{"facets":{},"unknown":[{"reason_code":"permission_not_granted","family":"codex"}],
                "explanation":[
                    {"kind":"unknown","unknown_reason_code":"unsupported_harness_version"},
                    {"kind":"excluded"}
                ],"findings":[]}"#,
        )
        .unwrap();
        let report = diagnose(&receipt);
        let findings = report.get("findings").and_then(Value::as_array).unwrap();
        let by_rule = |rule: &str| {
            findings
                .iter()
                .find(|item| item.get("rule_id").and_then(Value::as_str) == Some(rule))
                .unwrap_or_else(|| panic!("{rule} missing: {findings:?}"))
        };
        let surface = by_rule("D-UNKNOWN-SURFACE");
        assert_eq!(
            surface.pointer(&["treatment", "locked"]).and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            surface.pointer(&["treatment", "lock_reason"]).and_then(Value::as_str),
            Some("permission_not_granted")
        );
        let version = by_rule("D-UNSUPPORTED-VERSION");
        assert_eq!(
            version.pointer(&["treatment", "locked"]).and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            version.pointer(&["treatment", "lock_reason"]).and_then(Value::as_str),
            Some("unsupported_harness_version")
        );
        // A confirmed finding (the excluded instruction file) still blocks
        // under `--fail-on confirmed`; the locked unknowns are suspected and
        // do not decide it.
        assert_eq!(blocking_exit(&report, Some("confirmed")), 2);
        assert_eq!(blocking_exit(&report, None), 0);
    }
}
