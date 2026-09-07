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

/// Coordinate axes that pin the native target. No profile may ignore them:
/// two Receipts taken against different harnesses, surfaces, OS lanes, or
/// tool versions are not the same subject.
pub const DOMAIN_AXES: &[&str] = &["harness", "surface", "os_lane", "version"];

/// Coordinate axes that describe the machine a Receipt was taken on rather
/// than the subject it describes. `ignore-device-local` ignores exactly these.
pub const DEVICE_LOCAL_AXES: &[&str] =
    &["device", "account_alias", "environment", "cwd", "project"];

/// Governance axes. They are neither device-local nor a native target, and
/// no profile ignores them: `policy_snapshot` is policy identity and
/// `organization` is tenant identity.
pub const GOVERNANCE_AXES: &[&str] = &["organization", "policy_snapshot", "task"];

impl EquivalenceProfile {
    /// Whether this profile ignores `axis` when deciding comparability.
    #[must_use]
    pub fn ignores_axis(self, axis: &str) -> bool {
        match self {
            Self::Strict => false,
            Self::IgnoreDeviceLocal => DEVICE_LOCAL_AXES.contains(&axis),
        }
    }

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
    // `snapshot` is the per-run Receipt id. It differs between any two
    // distinct Receipts, so it identifies the instance, not the domain.
    let mut axis_rows = Vec::new();
    let mut ignored_fields = Vec::new();
    let mut domain_equal = true;
    for axis in DOMAIN_AXES
        .iter()
        .chain(GOVERNANCE_AXES)
        .chain(DEVICE_LOCAL_AXES)
    {
        let l = left
            .pointer(&["coordinate", axis])
            .and_then(Value::as_str)
            .unwrap_or("");
        let r = right
            .pointer(&["coordinate", axis])
            .and_then(Value::as_str)
            .unwrap_or("");
        let equal = l == r;
        let ignored = !equal && profile.ignores_axis(axis);
        if ignored {
            ignored_fields.push(string(format!("coordinate.{axis}")));
        }
        if !equal && !ignored {
            domain_equal = false;
        }
        axis_rows.push(object([
            ("axis", string(*axis)),
            ("left", string(l)),
            ("right", string(r)),
            ("equal", Value::Bool(equal)),
            ("ignored_by_profile", Value::Bool(ignored)),
        ]));
    }

    let left_digest = left
        .pointer(&["manifest", "digest"])
        .and_then(Value::as_str)
        .unwrap_or("");
    let right_digest = right
        .pointer(&["manifest", "digest"])
        .and_then(Value::as_str)
        .unwrap_or("");
    let digest_equal = !left_digest.is_empty() && left_digest == right_digest;
    // The manifest digest covers device-local paths, so under
    // `ignore-device-local` it may differ for identical assets. Asset
    // identity itself is never ignored: fall back to comparing the declared
    // (root, path, content_digest) triples, which carry no device-local text.
    let assets_equal = !digest_equal
        && profile == EquivalenceProfile::IgnoreDeviceLocal
        && asset_identity(left) == asset_identity(right)
        && !asset_identity(left).is_empty();
    if assets_equal {
        ignored_fields.push(string("manifest.digest"));
    }
    let files_equal = digest_equal || assets_equal;

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
        ("same_domain", Value::Bool(same_kind && domain_equal)),
        ("coordinate_axes", array(axis_rows)),
        // Exactly which fields this profile ignored, and only where they
        // actually differed. An empty list means the profile changed nothing.
        ("ignored_fields", array(ignored_fields)),
        ("digest_equal", Value::Bool(digest_equal)),
        ("files_equal_via_asset_identity", Value::Bool(assets_equal)),
        ("left_kind", string(left_kind)),
        ("right_kind", string(right_kind)),
        ("files_equal", Value::Bool(files_equal)),
        (
            "byte_equality_is_verified",
            Value::Bool(false),
        ),
        (
            "reconciliation",
            string(if !same_kind || !domain_equal || unknown_cells > 0 {
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
            // Receipts from a different native target are not a baseline for
            // each other, whatever the profile decided to ignore.
            "baseline_allowed",
            Value::Bool(unknown_cells == 0 && mismatches == 0 && same_kind && domain_equal),
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

/// Declared (root, path, content_digest) triples, sorted. This is asset
/// identity: no profile ignores it.
fn asset_identity(receipt: &Value) -> Vec<String> {
    let mut out: Vec<String> = receipt
        .get("evidence")
        .and_then(Value::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|item| {
            let field = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or("");
            format!(
                "{}\u{1f}{}\u{1f}{}",
                field("root"),
                field("path"),
                field("content_digest")
            )
        })
        .collect();
    out.sort();
    out
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

    /// Same subject, two machines: device-local axes and the manifest digest
    /// differ, the native target and asset identity do not.
    fn cross_device_pair() -> (Value, Value) {
        let facets = r#""facets":{"installed":{"truth_state":"present"},"discoverable":{"truth_state":"present"},"eligible":{"truth_state":"present"},"model-visible":{"truth_state":"present"},"use-evidence":{"truth_state":"present"},"outcome-affecting":{"truth_state":"present"}}"#;
        let evidence = r#""evidence":[{"root":"project","path":"AGENTS.md","content_digest":"c49f"}]"#;
        let a = parse(&format!(
            r#"{{"receipt_kind":"one-shot","coordinate":{{"harness":"codex","surface":"cli","os_lane":"macos-27-arm64","version":"0.147.0","device":"host-a","account_alias":"alice","environment":"laptop","cwd":"/Users/a/p/","project":"/Users/a/p"}},"manifest":{{"digest":"aaa"}},{evidence},{facets}}}"#
        ))
        .unwrap();
        let b = parse(&format!(
            r#"{{"receipt_kind":"one-shot","coordinate":{{"harness":"codex","surface":"cli","os_lane":"macos-27-arm64","version":"0.147.0","device":"host-b","account_alias":"bob","environment":"desktop","cwd":"/Users/b/q/","project":"/Users/b/q"}},"manifest":{{"digest":"bbb"}},{evidence},{facets}}}"#
        ))
        .unwrap();
        (a, b)
    }

    #[test]
    fn profile_changes_the_outcome_for_device_local_differences() {
        let (a, b) = cross_device_pair();

        let strict = diff(&a, &b, EquivalenceProfile::Strict);
        assert_eq!(strict.get("same_domain").and_then(Value::as_bool), Some(false));
        assert_eq!(strict.get("files_equal").and_then(Value::as_bool), Some(false));
        assert_eq!(
            strict.get("ignored_fields").and_then(Value::as_array).map(<[Value]>::len),
            Some(0)
        );
        assert_eq!(
            strict.get("baseline_allowed").and_then(Value::as_bool),
            Some(false)
        );

        let relaxed = diff(&a, &b, EquivalenceProfile::IgnoreDeviceLocal);
        assert_eq!(relaxed.get("same_domain").and_then(Value::as_bool), Some(true));
        // Digest differs; equality came from asset identity, and that is stated.
        assert_eq!(relaxed.get("digest_equal").and_then(Value::as_bool), Some(false));
        assert_eq!(relaxed.get("files_equal").and_then(Value::as_bool), Some(true));
        assert_eq!(
            relaxed
                .get("files_equal_via_asset_identity")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            relaxed.get("baseline_allowed").and_then(Value::as_bool),
            Some(true)
        );
        // Byte identity is still not semantic verification.
        assert_eq!(relaxed.get("verified").and_then(Value::as_bool), Some(false));

        let ignored: Vec<&str> = relaxed
            .get("ignored_fields")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(ignored.contains(&"coordinate.device"), "{ignored:?}");
        assert!(ignored.contains(&"coordinate.cwd"), "{ignored:?}");
        assert!(ignored.contains(&"manifest.digest"), "{ignored:?}");
        assert!(!ignored.contains(&"coordinate.harness"), "{ignored:?}");
    }

    #[test]
    fn no_profile_ignores_the_native_target_or_asset_identity() {
        for axis in DOMAIN_AXES.iter().chain(GOVERNANCE_AXES) {
            assert!(
                !EquivalenceProfile::IgnoreDeviceLocal.ignores_axis(axis),
                "{axis} must never be ignored"
            );
        }

        // A different OS lane is a different subject even under the relaxed
        // profile.
        let (a, mut b) = cross_device_pair();
        if let Value::Object(map) = &mut b
            && let Some(Value::Object(coord)) = map.get_mut("coordinate")
        {
            coord.insert("os_lane".into(), string("linux-x86_64"));
        }
        let relaxed = diff(&a, &b, EquivalenceProfile::IgnoreDeviceLocal);
        assert_eq!(relaxed.get("same_domain").and_then(Value::as_bool), Some(false));

        // Differing asset content is not equal under any profile.
        let (a, mut b) = cross_device_pair();
        if let Value::Object(map) = &mut b
            && let Some(Value::Array(items)) = map.get_mut("evidence")
            && let Some(Value::Object(first)) = items.first_mut()
        {
            first.insert("content_digest".into(), string("deadbeef"));
        }
        let relaxed = diff(&a, &b, EquivalenceProfile::IgnoreDeviceLocal);
        assert_eq!(relaxed.get("files_equal").and_then(Value::as_bool), Some(false));
    }

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
