//! Diff two or more Receipts under an EquivalenceProfile.
//!
//! Text/hash equality is recorded as `files-equal` and is never mapped to
//! `verified`. Unknown cells cannot form a passing baseline.

use ctxpect_schema::{array, object, string, Value};

/// Profiles that may ignore device-local fields. They must not ignore asset
/// identity, intent revision, scope, native target, loss, policy, or required claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquivalenceProfile {
    Strict,
    IgnoreDeviceLocal,
}

impl EquivalenceProfile {
    #[must_use]
    pub fn from_wire(text: &str) -> Option<Self> {
        match text {
            "strict" => Some(Self::Strict),
            "ignore-device-local" => Some(Self::IgnoreDeviceLocal),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::IgnoreDeviceLocal => "ignore-device-local",
        }
    }
}

/// Compare two Receipts. `same_domain` is false when kinds/coordinates are incomparable.
#[must_use]
pub fn diff(left: &Value, right: &Value, profile: EquivalenceProfile) -> Value {
    let left_kind = left.get("receipt_kind").and_then(Value::as_str).unwrap_or("");
    let right_kind = right.get("receipt_kind").and_then(Value::as_str).unwrap_or("");
    let same_kind = left_kind == right_kind && !left_kind.is_empty();
    let left_harness = left
        .pointer(&["coordinate", "harness"])
        .and_then(Value::as_str)
        .unwrap_or("");
    let right_harness = right
        .pointer(&["coordinate", "harness"])
        .and_then(Value::as_str)
        .unwrap_or("");

    let left_digest = left
        .pointer(&["manifest", "digest"])
        .and_then(Value::as_str)
        .unwrap_or("");
    let right_digest = right
        .pointer(&["manifest", "digest"])
        .and_then(Value::as_str)
        .unwrap_or("");
    let files_equal = !left_digest.is_empty() && left_digest == right_digest;

    let mut facet_rows = Vec::new();
    let names = [
        "installed",
        "discoverable",
        "eligible",
        "model-visible",
        "use-evidence",
        "outcome-affecting",
    ];
    let mut unknown_cells = 0i64;
    let mut mismatches = 0i64;
    for name in names {
        let l = facet_truth(left, name);
        let r = facet_truth(right, name);
        let l_unknown = l == "indeterminate" || l == "unknown" || l.is_empty();
        let r_unknown = r == "indeterminate" || r == "unknown" || r.is_empty();
        if l_unknown || r_unknown {
            unknown_cells += 1;
        }
        let comparable = same_kind && !l_unknown && !r_unknown;
        let equal = comparable && l == r;
        if comparable && !equal {
            mismatches += 1;
        }
        facet_rows.push(object([
            ("facet", string(name)),
            ("left", string(l)),
            ("right", string(r)),
            ("equal", Value::Bool(equal)),
            (
                "unknown_blocks_baseline",
                Value::Bool(l_unknown || r_unknown),
            ),
        ]));
    }

    object([
        ("schema", string("ctxpect-diff-v1")),
        ("profile", string(profile.as_str())),
        ("same_kind", Value::Bool(same_kind)),
        (
            "same_domain",
            Value::Bool(same_kind && left_harness == right_harness),
        ),
        ("left_kind", string(left_kind)),
        ("right_kind", string(right_kind)),
        ("files_equal", Value::Bool(files_equal)),
        (
            "byte_equality_is_verified",
            Value::Bool(false),
        ),
        (
            "reconciliation",
            string(if !same_kind || unknown_cells > 0 {
                "indeterminate"
            } else if mismatches == 0 {
                "structural-only"
            } else {
                "failed"
            }),
        ),
        (
            "verified",
            Value::Bool(false),
        ),
        (
            "baseline_allowed",
            Value::Bool(unknown_cells == 0 && mismatches == 0 && same_kind),
        ),
        (
            "note",
            string(
                "files_equal records digest identity only. Unknown cells cannot create a passing baseline. Native verified requires a declared oracle, which this diff does not invent.",
            ),
        ),
        ("facets", array(facet_rows)),
        ("unknown_cells", Value::Int(unknown_cells)),
        ("mismatches", Value::Int(mismatches)),
    ])
}

fn facet_truth<'a>(receipt: &'a Value, name: &'a str) -> &'a str {
    receipt
        .pointer(&["facets", name, "truth_state"])
        .and_then(Value::as_str)
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    #[test]
    fn digest_equality_is_not_verified_and_unknown_blocks_baseline() {
        let a = parse(
            r#"{"receipt_kind":"one-shot","coordinate":{"harness":"codex"},"manifest":{"digest":"aaa"},"facets":{"installed":{"truth_state":"present"},"discoverable":{"truth_state":"present"},"eligible":{"truth_state":"present"},"model-visible":{"truth_state":"indeterminate"},"use-evidence":{"truth_state":"indeterminate"},"outcome-affecting":{"truth_state":"indeterminate"}}}"#,
        )
        .unwrap();
        let b = a.clone();
        let report = diff(&a, &b, EquivalenceProfile::Strict);
        assert_eq!(report.get("files_equal").and_then(Value::as_bool), Some(true));
        assert_eq!(
            report
                .get("byte_equality_is_verified")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(report.get("verified").and_then(Value::as_bool), Some(false));
        assert_eq!(
            report.get("baseline_allowed").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            report.get("reconciliation").and_then(Value::as_str),
            Some("indeterminate")
        );
    }

    #[test]
    fn different_kinds_are_not_the_same_domain() {
        let a = parse(r#"{"receipt_kind":"one-shot","coordinate":{"harness":"codex"},"manifest":{"digest":"a"},"facets":{}}"#).unwrap();
        let b = parse(r#"{"receipt_kind":"ci","coordinate":{"harness":"codex"},"manifest":{"digest":"a"},"facets":{}}"#).unwrap();
        let report = diff(&a, &b, EquivalenceProfile::Strict);
        assert_eq!(report.get("same_kind").and_then(Value::as_bool), Some(false));
        assert_eq!(report.get("same_domain").and_then(Value::as_bool), Some(false));
    }
}
