//! Enrolled principals: the identity source that exception approval reads.
//!
//! The registry is a version-controlled file inside the project, so who may
//! request and who may approve is reviewable in the same place as the code.
//! Enrollment records a digest, never the secret itself.
//!
//! This is a *local* identity root, not an organization one. It proves that
//! the caller holds the secret enrolled for a listed principal. It does not
//! prove a corporate identity, and every document it produces says so.

use ctxpect_schema::{array, hmac_sha256_hex, object, string, Value};

use crate::PolicyError;

pub const REGISTRY_SCHEMA: &str = "ctxpect-principals-v1";

/// Domain separator for enrollment digests. It is bound to the principal id
/// so an enrollment cannot be replayed for a different principal.
const ENROLLMENT_DOMAIN: &str = "ctxpect-principal-enrollment-v1";

/// Roles the registry understands. A role that is not listed here is a typo
/// or a forgery, and enrollment refuses it rather than silently ignoring it.
pub const ROLES: &[&str] = &["requester", "approver"];

/// A caller whose held secret matched an enrolled principal.
///
/// It can only be constructed by [`verify_principal`], so a value of this
/// type is evidence rather than an assertion. Nothing here comes from
/// `--role` or `--actor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProof {
    principal_id: String,
    roles: Vec<String>,
    self_approval: bool,
}

impl PrincipalProof {
    #[must_use]
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    #[must_use]
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|item| item == role)
    }

    /// Whether this principal is enrolled to approve its own requests. It is
    /// off unless the registry says otherwise, in writing.
    #[must_use]
    pub const fn self_approval(&self) -> bool {
        self.self_approval
    }

    /// A record of the identity used, safe to persist: it names the
    /// principal and the enrollment, and carries no secret.
    #[must_use]
    pub fn to_value(&self) -> Value {
        object([
            ("principal_id", string(&self.principal_id)),
            (
                "roles",
                array(self.roles.iter().map(string).collect::<Vec<_>>()),
            ),
            ("identity_kind", string("enrolled-local-principal")),
            // A held local secret is not a corporate identity, and this
            // document must never be read as one.
            ("org_identity", Value::Bool(false)),
        ])
    }
}

/// The digest enrolled for `principal_id` when the holder has `secret`.
///
/// The registry is public, so this digest is public too. The secret must be
/// high-entropy random material: a guessable secret can be recovered offline
/// from the enrolled digest.
#[must_use]
pub fn enrollment_digest(principal_id: &str, secret: &[u8]) -> String {
    hmac_sha256_hex(
        secret,
        format!("{ENROLLMENT_DOMAIN}:{principal_id}").as_bytes(),
    )
}

fn entries(registry: &Value) -> Result<&[Value], PolicyError> {
    let schema = registry.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != REGISTRY_SCHEMA {
        return Err(PolicyError::new(
            "principal.registry_schema",
            format!("principal registry schema must be `{REGISTRY_SCHEMA}`, found `{schema}`"),
        ));
    }
    registry
        .get("principals")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            PolicyError::new(
                "principal.registry_malformed",
                "principal registry must carry a `principals` array",
            )
        })
}

/// Verify that the caller holds the secret enrolled for `principal_id`.
///
/// Every failure is fail-closed and distinguishable: an absent registry, an
/// unlisted principal, and a wrong secret do not collapse into one code.
pub fn verify_principal(
    registry: Option<&Value>,
    principal_id: &str,
    secret: &[u8],
) -> Result<PrincipalProof, PolicyError> {
    let Some(registry) = registry else {
        return Err(PolicyError::new(
            "principal.registry_absent",
            "no enrolled principal registry is present; identity is fail-closed",
        ));
    };
    let items = entries(registry)?;
    if secret.is_empty() {
        return Err(PolicyError::new(
            "principal.secret_absent",
            "no principal secret was supplied; a named principal is not by itself identity",
        ));
    }
    let Some(record) = items
        .iter()
        .find(|item| item.get("principal_id").and_then(Value::as_str) == Some(principal_id))
    else {
        return Err(PolicyError::new(
            "principal.not_enrolled",
            format!("`{principal_id}` is not in the enrolled principal registry"),
        ));
    };

    let enrolled = record
        .get("key_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if enrolled.is_empty() {
        return Err(PolicyError::new(
            "principal.enrollment_incomplete",
            format!("`{principal_id}` has no enrolled key digest"),
        ));
    }
    if enrollment_digest(principal_id, secret) != enrolled {
        return Err(PolicyError::new(
            "principal.secret_mismatch",
            "the supplied secret does not match the enrolled digest",
        ));
    }

    let mut roles = Vec::new();
    for role in record.get("roles").and_then(Value::as_array).unwrap_or(&[]) {
        let Some(name) = role.as_str() else {
            return Err(PolicyError::new(
                "principal.role_unknown",
                "principal roles must be strings",
            ));
        };
        if !ROLES.contains(&name) {
            return Err(PolicyError::new(
                "principal.role_unknown",
                format!("role `{name}` is not one of the enrolled roles"),
            ));
        }
        roles.push(name.to_string());
    }

    Ok(PrincipalProof {
        principal_id: principal_id.to_string(),
        roles,
        self_approval: record
            .get("self_approval")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    fn registry(digest: &str) -> Value {
        parse(&format!(
            r#"{{"schema":"ctxpect-principals-v1","principals":[{{"principal_id":"alice","roles":["requester","approver"],"key_id":"k1","key_digest":"{digest}"}}]}}"#
        ))
        .unwrap()
    }

    #[test]
    fn holding_the_enrolled_secret_is_identity_and_naming_a_principal_is_not() {
        let secret = b"high-entropy-secret";
        let reg = registry(&enrollment_digest("alice", secret));

        let proof = verify_principal(Some(&reg), "alice", secret).unwrap();
        assert_eq!(proof.principal_id(), "alice");
        assert!(proof.has_role("approver"));
        assert!(!proof.self_approval());
        assert_eq!(
            proof.to_value().get("org_identity").and_then(Value::as_bool),
            Some(false)
        );

        // Naming the principal without the secret is not identity.
        let err = verify_principal(Some(&reg), "alice", b"").expect_err("no secret");
        assert_eq!(err.code, "principal.secret_absent");
        let err = verify_principal(Some(&reg), "alice", b"guess").expect_err("wrong secret");
        assert_eq!(err.code, "principal.secret_mismatch");

        // An unlisted principal cannot enroll itself by asking.
        let err = verify_principal(Some(&reg), "mallory", secret).expect_err("unlisted");
        assert_eq!(err.code, "principal.not_enrolled");

        // No registry at all is fail-closed, not an open door.
        let err = verify_principal(None, "alice", secret).expect_err("absent");
        assert_eq!(err.code, "principal.registry_absent");
    }

    #[test]
    fn an_enrollment_cannot_be_replayed_for_another_principal() {
        let secret = b"high-entropy-secret";
        // The digest is bound to the id, so alice's enrollment does not
        // authenticate bob even with the same secret.
        assert_ne!(
            enrollment_digest("alice", secret),
            enrollment_digest("bob", secret)
        );
        let reg = parse(&format!(
            r#"{{"schema":"ctxpect-principals-v1","principals":[{{"principal_id":"bob","roles":["approver"],"key_id":"k1","key_digest":"{}"}}]}}"#,
            enrollment_digest("alice", secret)
        ))
        .unwrap();
        let err = verify_principal(Some(&reg), "bob", secret).expect_err("replay");
        assert_eq!(err.code, "principal.secret_mismatch");
    }

    #[test]
    fn a_malformed_registry_is_refused_rather_than_partially_trusted() {
        let secret = b"s";
        let wrong_schema = parse(r#"{"schema":"other","principals":[]}"#).unwrap();
        let err = verify_principal(Some(&wrong_schema), "alice", secret).expect_err("schema");
        assert_eq!(err.code, "principal.registry_schema");

        let no_array = parse(r#"{"schema":"ctxpect-principals-v1"}"#).unwrap();
        let err = verify_principal(Some(&no_array), "alice", secret).expect_err("malformed");
        assert_eq!(err.code, "principal.registry_malformed");

        let bad_role = parse(&format!(
            r#"{{"schema":"ctxpect-principals-v1","principals":[{{"principal_id":"alice","roles":["admin"],"key_digest":"{}"}}]}}"#,
            enrollment_digest("alice", secret)
        ))
        .unwrap();
        let err = verify_principal(Some(&bad_role), "alice", secret).expect_err("role");
        assert_eq!(err.code, "principal.role_unknown");
    }
}
