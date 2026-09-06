//! Formal Context Receipts and the explicit `dev-inspect-v0` migration.
//!
//! A development snapshot (`schema=dev-inspect-v0`, `receipt_kind=development-snapshot`)
//! is not a signed Context Receipt. Promotion must go through
//! [`migrate_dev_inspect_v0`], which records the source schema, snapshot digest,
//! and migration identifier. Renaming fields is not a migration.

use ctxpect_schema::{
    array, canonical_json, digest_value, hmac_sha256_hex, object, opt_string, string,
    strip_time_fields, Value,
};

/// Wire schema of the inspect development snapshot.
pub const DEV_INSPECT_SCHEMA: &str = "dev-inspect-v0";
/// Wire kind of the inspect development snapshot.
pub const DEV_INSPECT_KIND: &str = "development-snapshot";
/// Formal Receipt schema. Distinct from the development snapshot schema.
pub const RECEIPT_SCHEMA: &str = "ctxpect-receipt-v1";
/// Migration identifier stored on every promoted snapshot.
pub const MIGRATION_ID: &str = "dev-inspect-v0-to-ctxpect-receipt-v1";

/// The six Receipt kinds in PRD §4.3 / F-05.
pub const RECEIPT_KINDS: &[&str] = &[
    "one-shot",
    "preflight",
    "runtime",
    "post-session",
    "device-baseline",
    "ci",
];

/// Signature kinds this crate will emit. Organizational identity is not one of them.
pub const LOCAL_CONTINUITY: &str = "local-continuity";

/// Why a snapshot cannot become a formal Receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptError {
    pub code: &'static str,
    pub message: String,
}

impl ReceiptError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Local-continuity key material. Not an org/PKI identity.
#[derive(Debug, Clone)]
pub struct ContinuityKey {
    pub key_id: String,
    pub secret: Vec<u8>,
}

impl ContinuityKey {
    /// Derive a store-local key from raw bytes. The key id is a digest prefix.
    #[must_use]
    pub fn from_secret(secret: Vec<u8>) -> Self {
        let key_id = format!("lc_{}", &ctxpect_schema::sha256_hex(&secret)[..16]);
        Self { key_id, secret }
    }
}

/// Promote a `dev-inspect-v0` envelope into an unsigned-or-locally-signed Receipt.
///
/// The original snapshot is referenced, not rewritten in place. `schema` stays
/// `ctxpect-receipt-v1`; the source snapshot keeps `dev-inspect-v0`.
pub fn migrate_dev_inspect_v0(
    snapshot: &Value,
    receipt_kind: &str,
    continuity: Option<&ContinuityKey>,
    created_at: &str,
    receipt_id: &str,
) -> Result<Value, ReceiptError> {
    if !RECEIPT_KINDS.contains(&receipt_kind) {
        return Err(ReceiptError::new(
            "receipt.kind_invalid",
            format!("receipt_kind `{receipt_kind}` is not one of the six formal kinds"),
        ));
    }
    let schema = snapshot.get("schema").and_then(Value::as_str);
    let kind = snapshot.get("receipt_kind").and_then(Value::as_str);
    if schema != Some(DEV_INSPECT_SCHEMA) || kind != Some(DEV_INSPECT_KIND) {
        return Err(ReceiptError::new(
            "receipt.migration_required",
            "formal Receipts are produced only by migrate_dev_inspect_v0 from a dev-inspect-v0 development-snapshot; renaming schema is not a migration",
        ));
    }
    let snapshot_digest = snapshot
        .get("snapshot_digest")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| digest_value(&strip_time_fields(snapshot)));

    let scope = snapshot
        .get("scope")
        .cloned()
        .unwrap_or_else(|| object::<String>([]));
    let results = snapshot
        .get("results")
        .and_then(Value::as_array)
        .unwrap_or(&[]);
    let claims = collect_claims(results);
    let facets = first_facets(results);
    let evidence = first_array(results, "evidence");
    let edges = first_array(results, "edges");
    let layers = first_array(results, "layers");

    let mut body = object([
        ("schema", string(RECEIPT_SCHEMA)),
        ("schema_version", Value::Int(1)),
        ("receipt_kind", string(receipt_kind)),
        ("receipt_id", string(receipt_id)),
        ("created_at", string(created_at)),
        (
            "coordinate",
            object([
                ("device", unknown_or(scope.get("device"))),
                ("environment", unknown_or(scope.get("environment"))),
                ("account_alias", unknown_or(scope.get("account_alias"))),
                ("organization", unknown_or(scope.get("organization"))),
                (
                    "policy_snapshot",
                    unknown_or(scope.get("policy_snapshot")),
                ),
                (
                    "harness",
                    string(scope.get("harness").and_then(Value::as_str).unwrap_or("unknown")),
                ),
                (
                    "version",
                    string(scope.get("version").and_then(Value::as_str).unwrap_or("unknown")),
                ),
                (
                    "version_provenance",
                    string(
                        scope
                            .get("version_provenance")
                            .and_then(Value::as_str)
                            .unwrap_or("user-attested"),
                    ),
                ),
                (
                    "surface",
                    string(scope.get("surface").and_then(Value::as_str).unwrap_or("unknown")),
                ),
                (
                    "os_lane",
                    string(scope.get("os_lane").and_then(Value::as_str).unwrap_or("unknown")),
                ),
                (
                    "project",
                    string(
                        scope
                            .pointer(&["roots", "project"])
                            .and_then(Value::as_str)
                            .unwrap_or("<project>"),
                    ),
                ),
                (
                    "cwd",
                    string(scope.get("cwd").and_then(Value::as_str).unwrap_or("<project>/")),
                ),
                ("task", Value::Null),
                ("snapshot", string(receipt_id)),
            ]),
        ),
        (
            "source_snapshot",
            object([
                ("schema", string(DEV_INSPECT_SCHEMA)),
                ("receipt_kind", string(DEV_INSPECT_KIND)),
                ("snapshot_digest", string(snapshot_digest)),
                ("migration", string(MIGRATION_ID)),
                ("migrated", Value::Bool(true)),
            ]),
        ),
        ("claims", claims),
        ("facets", facets),
        ("evidence", evidence),
        ("edges", edges),
        ("layers", layers),
        (
            "unknown",
            snapshot
                .get("unknown")
                .cloned()
                .unwrap_or_else(|| array([])),
        ),
        (
            "findings",
            snapshot
                .get("findings")
                .cloned()
                .unwrap_or_else(|| array([])),
        ),
        (
            "warnings",
            snapshot
                .get("warnings")
                .cloned()
                .unwrap_or_else(|| array([])),
        ),
        (
            "assumptions",
            snapshot
                .get("assumptions")
                .cloned()
                .unwrap_or_else(|| array([])),
        ),
        (
            "explanation",
            snapshot
                .get("explanation")
                .cloned()
                .unwrap_or_else(|| array([])),
        ),
        (
            "policy_result",
            snapshot
                .get("policy_result")
                .cloned()
                .unwrap_or_else(|| object([("verdict", string("indeterminate"))])),
        ),
        (
            "required",
            snapshot
                .get("required")
                .cloned()
                .unwrap_or_else(|| array([])),
        ),
        (
            "budget",
            unknown_budget(),
        ),
        (
            "redaction",
            object([
                ("policy", string("default")),
                ("bodies_included", Value::Bool(false)),
            ]),
        ),
        ("tombstone", Value::Null),
        (
            "signature",
            object([
                ("kind", string(LOCAL_CONTINUITY)),
                ("algorithm", string("hmac-sha256")),
                ("key_id", opt_string(continuity.map(|k| k.key_id.as_str()))),
                ("org_identity", Value::Bool(false)),
                ("value", Value::Null),
            ]),
        ),
    ]);

    let manifest_digest = digest_body(&body);
    if let Value::Object(map) = &mut body {
        map.insert(
            "manifest".to_string(),
            object([("digest", string(&manifest_digest))]),
        );
    }
    if let Some(key) = continuity {
        let mac = hmac_sha256_hex(&key.secret, manifest_digest.as_bytes());
        if let Some(Value::Object(sig)) = match &mut body {
            Value::Object(map) => map.get_mut("signature"),
            _ => None,
        } {
            sig.insert("value".to_string(), string(mac));
            sig.insert("key_id".to_string(), string(&key.key_id));
        }
    }
    Ok(body)
}

/// Verify a formal Receipt's local-continuity MAC. Does not assert org identity.
pub fn verify_local_continuity(receipt: &Value, key: &ContinuityKey) -> Result<(), ReceiptError> {
    let schema = receipt.get("schema").and_then(Value::as_str);
    if schema != Some(RECEIPT_SCHEMA) {
        return Err(ReceiptError::new(
            "receipt.not_formal",
            "verify_local_continuity only accepts ctxpect-receipt-v1",
        ));
    }
    let kind = receipt
        .pointer(&["signature", "kind"])
        .and_then(Value::as_str);
    if kind != Some(LOCAL_CONTINUITY) {
        return Err(ReceiptError::new(
            "receipt.signature_kind",
            "this Receipt is not a local-continuity signature",
        ));
    }
    let org = receipt
        .pointer(&["signature", "org_identity"])
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if org {
        return Err(ReceiptError::new(
            "receipt.org_identity_forbidden",
            "local-continuity signatures must not claim organizational identity",
        ));
    }
    let expected_digest = digest_body(receipt);
    let stored = receipt
        .pointer(&["manifest", "digest"])
        .and_then(Value::as_str)
        .unwrap_or("");
    if stored != expected_digest {
        return Err(ReceiptError::new(
            "receipt.manifest_mismatch",
            "manifest.digest does not match the canonical body",
        ));
    }
    let mac = hmac_sha256_hex(&key.secret, expected_digest.as_bytes());
    let got = receipt
        .pointer(&["signature", "value"])
        .and_then(Value::as_str)
        .unwrap_or("");
    if got != mac {
        return Err(ReceiptError::new(
            "receipt.signature_mismatch",
            "local-continuity MAC does not match",
        ));
    }
    Ok(())
}

/// Tombstone a Receipt: keep id, digest, kind; drop evidence bodies.
pub fn tombstone(receipt: &Value, deleted_at: &str, reason: &str) -> Result<Value, ReceiptError> {
    let id = receipt
        .get("receipt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ReceiptError::new("receipt.missing_id", "receipt_id is required"))?;
    let digest = receipt
        .pointer(&["manifest", "digest"])
        .and_then(Value::as_str)
        .unwrap_or("");
    let kind = receipt
        .get("receipt_kind")
        .and_then(Value::as_str)
        .unwrap_or("one-shot");
    Ok(object([
        ("schema", string(RECEIPT_SCHEMA)),
        ("schema_version", Value::Int(1)),
        ("receipt_id", string(id)),
        ("receipt_kind", string(kind)),
        (
            "tombstone",
            object([
                ("deleted_at", string(deleted_at)),
                ("reason", string(reason)),
                ("original_digest", string(digest)),
                ("evidence_bodies", string("purged")),
                ("derived_invalidated", Value::Bool(true)),
                ("external_copies", string("not-recallable")),
            ]),
        ),
        ("claims", array([])),
        ("evidence", array([])),
        ("findings", array([])),
        ("unknown", array([])),
        ("explanation", array([])),
        ("facets", object::<String>([])),
        (
            "manifest",
            object([("digest", string(digest))]),
        ),
        (
            "signature",
            receipt
                .get("signature")
                .cloned()
                .unwrap_or(Value::Null),
        ),
    ]))
}

/// Reject a development snapshot that was merely relabeled as a formal Receipt.
pub fn reject_relabeled_snapshot(value: &Value) -> Result<(), ReceiptError> {
    let schema = value.get("schema").and_then(Value::as_str);
    let kind = value.get("receipt_kind").and_then(Value::as_str);
    if schema == Some(RECEIPT_SCHEMA) && kind == Some(DEV_INSPECT_KIND) {
        return Err(ReceiptError::new(
            "receipt.relabel_forbidden",
            "development-snapshot cannot be a ctxpect-receipt-v1 kind",
        ));
    }
    if schema == Some(RECEIPT_SCHEMA) {
        let migrated = value
            .pointer(&["source_snapshot", "migrated"])
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let migration = value
            .pointer(&["source_snapshot", "migration"])
            .and_then(Value::as_str);
        if !migrated || migration != Some(MIGRATION_ID) {
            return Err(ReceiptError::new(
                "receipt.migration_missing",
                "ctxpect-receipt-v1 that originated from inspect must carry source_snapshot.migration=dev-inspect-v0-to-ctxpect-receipt-v1",
            ));
        }
    }
    Ok(())
}

fn digest_body(receipt: &Value) -> String {
    let stripped = strip_time_fields(receipt);
    let body = match stripped {
        Value::Object(mut map) => {
            map.remove("manifest");
            if let Some(Value::Object(sig)) = map.get_mut("signature") {
                sig.insert("value".to_string(), Value::Null);
            }
            Value::Object(map)
        }
        other => other,
    };
    digest_value(&body)
}

fn unknown_or(value: Option<&Value>) -> Value {
    match value.and_then(Value::as_str) {
        Some(text) if !text.is_empty() => string(text),
        _ => string("unknown"),
    }
}

fn unknown_budget() -> Value {
    let cell = object([
        ("status", string("unknown")),
        ("reason_code", string("current_occupancy_not_reported")),
        ("unit", Value::Null),
        ("value", Value::Null),
        ("precision", string("not-applicable")),
    ]);
    object([
        ("constant", cell.clone()),
        ("conditional", cell.clone()),
        ("skill_catalog", cell.clone()),
        ("skill_body", cell.clone()),
        ("tool_schema", cell.clone()),
        ("tool_result", cell.clone()),
        ("history", cell.clone()),
        ("memory", cell.clone()),
        ("current_user", cell.clone()),
        ("unknown", cell),
        (
            "note",
            string("Unknown budget cells are not drawn as zero and are not summed into a fake total"),
        ),
    ])
}

fn collect_claims(results: &[Value]) -> Value {
    let mut out = Vec::new();
    for result in results {
        if let Some(claim) = result.get("claim") {
            out.push(claim.clone());
        }
        if let Some(Value::Object(facets)) = result.get("facets") {
            for (name, claim) in facets {
                if let Value::Object(map) = claim {
                    let mut copy = map.clone();
                    copy.insert("facet".to_string(), string(name));
                    copy.insert(
                        "capability_id".to_string(),
                        string(
                            result
                                .get("capability_id")
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                        ),
                    );
                    out.push(Value::Object(copy));
                }
            }
        }
    }
    array(out)
}

fn first_facets(results: &[Value]) -> Value {
    results
        .first()
        .and_then(|item| item.get("facets"))
        .cloned()
        .unwrap_or_else(|| object::<String>([]))
}

fn first_array(results: &[Value], key: &str) -> Value {
    results
        .first()
        .and_then(|item| item.get(key))
        .cloned()
        .unwrap_or_else(|| array([]))
}

/// Stable id from label + payload bytes.
#[must_use]
pub fn receipt_id_for(label: &str, payload: &str) -> String {
    let digest = ctxpect_schema::sha256_text(&format!("{label}\n{payload}"));
    format!("r_{}", &digest[..16])
}

/// Canonical JSON of a Receipt, for export.
#[must_use]
pub fn encode(receipt: &Value) -> String {
    canonical_json(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    fn snapshot() -> Value {
        parse(
            r#"{"schema":"dev-inspect-v0","receipt_kind":"development-snapshot","snapshot_digest":"abc","scope":{"harness":"codex","version":"0.147.0","surface":"cli","os_lane":"macos-27-arm64","cwd":"<project>/","roots":{"project":"<project>"}},"results":[{"capability_id":"instructions","claim":{"truth_state":"present","claim_kind":"resolved","lifecycle_stage":"eligible","provenance":"harness-source","coverage":"full-declared-surface","precision":"exact","knowledge_status":"current"},"facets":{"installed":{"truth_state":"present","claim_kind":"resolved","lifecycle_stage":"installed","provenance":"harness-source","coverage":"full-declared-surface","precision":"exact","knowledge_status":"current"},"model-visible":{"truth_state":"indeterminate","claim_kind":"resolved","lifecycle_stage":"model-visible","provenance":"heuristic","coverage":"unknown","precision":"not-applicable","knowledge_status":"unknown","unknown_reason_code":"runtime_snapshot_missing"}},"evidence":[],"edges":[],"layers":[]}],"unknown":[],"findings":[],"policy_result":{"verdict":"pass","exit_code":0},"required":["instructions"]}"#,
        )
        .expect("snapshot")
    }

    #[test]
    fn migration_records_source_schema() {
        let key = ContinuityKey::from_secret(b"test-key".to_vec());
        let receipt = migrate_dev_inspect_v0(
            &snapshot(),
            "one-shot",
            Some(&key),
            "2026-09-06T00:00:00+08:00",
            "r_test",
        )
        .expect("migrate");
        assert_eq!(
            receipt.get("schema").and_then(Value::as_str),
            Some(RECEIPT_SCHEMA)
        );
        assert_eq!(
            receipt.get("receipt_kind").and_then(Value::as_str),
            Some("one-shot")
        );
        assert_eq!(
            receipt
                .pointer(&["source_snapshot", "schema"])
                .and_then(Value::as_str),
            Some(DEV_INSPECT_SCHEMA)
        );
        assert_eq!(
            receipt
                .pointer(&["source_snapshot", "migration"])
                .and_then(Value::as_str),
            Some(MIGRATION_ID)
        );
        verify_local_continuity(&receipt, &key).expect("mac");
        reject_relabeled_snapshot(&receipt).expect("ok");
    }

    #[test]
    fn relabel_without_migration_is_rejected() {
        let fake = parse(r#"{"schema":"ctxpect-receipt-v1","receipt_kind":"one-shot"}"#).unwrap();
        let err = reject_relabeled_snapshot(&fake).expect_err("relabel");
        assert_eq!(err.code, "receipt.migration_missing");
    }

    #[test]
    fn cannot_migrate_already_formal() {
        let formal = parse(r#"{"schema":"ctxpect-receipt-v1","receipt_kind":"one-shot"}"#).unwrap();
        let err = migrate_dev_inspect_v0(&formal, "one-shot", None, "t", "id").expect_err("no");
        assert_eq!(err.code, "receipt.migration_required");
    }

    #[test]
    fn unknown_budget_is_not_zero() {
        let receipt =
            migrate_dev_inspect_v0(&snapshot(), "one-shot", None, "t", "id").expect("migrate");
        let constant = receipt.pointer(&["budget", "constant"]).expect("budget");
        assert_eq!(
            constant.get("status").and_then(Value::as_str),
            Some("unknown")
        );
        assert_ne!(constant.get("value"), Some(&Value::Int(0)));
    }
}
