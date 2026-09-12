#!/usr/bin/env python3
"""Semantic alignment and Team Context Standard contract (docs stage).

Generated fixtures are Apache-2.0 synthetic data. live_tested is always false.
This module does not run harnesses, does not claim native oracle results, and
requires no network. Projection outcomes are orthogonal to reconciliation
states; text/hash equality is never semantic equivalence.
"""

from __future__ import annotations

import hashlib
import hmac
import re
import shutil
from datetime import datetime
from typing import Any

from contexpect_contract import (
    ANCHOR_IDS,
    ARTIFACT_DIGEST_MANIFEST,
    CAPABILITIES,
    CLAIM_RECONCILIATION_STATES,
    CONNECTOR_FAMILIES,
    CUTOFF,
    DECLARED_NATIVE_ORACLES,
    FAMILIES,
    LICENSE_ID,
    OS_LANES,
    SCHEMA_VERSION,
    canonical_json,
    capability_status,
    dump_json_yaml,
    family_by_id,
    family_ids,
    sha256_text,
)
from contexpect_freeze import FAMILY_INPUT_SPECS, NATIVE_TARGETS
from contexpect_schema import require_closed_object

SEMANTIC_TEAM_CONTRACT = "acceptance/semantic-team-contract.yaml"
SEMANTIC_TEAM_DIR = "acceptance/semantic-team"
SEMANTIC_TEAM_ARTIFACT = "semantic-team-contract"

PROJECTION_OUTCOMES = [
    "exact",
    "native-equivalent",
    "transformed",
    "lossless-native-overlay",
    "lossy",
    "unsupported",
    "unknown",
]
LOSS_CLASSES = ["omitted", "weakened", "approximated", "duplicated", "harness-only"]
DRIFT_CLASSES = [
    "benign-native-representation",
    "unapproved-semantic-drift",
    "approved-exception",
    "unknown-evidence",
]
ENFORCEMENT_MODES = ["recommended", "required", "prohibited"]
ENFORCEMENT_HONESTY = ["enforceable", "detect-only", "non-enforceable"]
LAYER_ORDER = ["organization", "team", "project", "role", "personal"]
EXCEPTION_STATES = ["request", "approval", "rejection", "revocation", "expiry"]
STANDARD_LIFECYCLES = [
    "validate",
    "publish",
    "preview",
    "adopt",
    "pin",
    "update",
    "status",
    "leave",
    "rollback",
    "revoke",
]
PIPELINE_STEPS = [
    "canonical-intent",
    "target-coordinate",
    "capability-negotiation",
    "authority-selection",
    "harness-native-projection",
    "preview-apply-rollback",
    "target-resolver-oracle",
    "reconciliation-receipt",
]
EQUIVALENCE_BASES = ["canonical-intent-semantics", "documented-native-mapping"]
FORBIDDEN_EQUIVALENCE_BASES = ["byte-equality", "hash-equality", "identical-file-copy"]
CHANNELS = ["git-file-local", "file-local", "encrypted-cloud-optional-transport"]
ROUND_TRIP_DIGEST_KEYS = [
    "intent",
    "plan",
    "authority",
    "transaction",
    "native_artifact",
    "resolver_oracle",
    "policy",
    "exception",
]
REQUIRED_FOUR = ["codex", "claude-code", "cursor", "grok-build"]
REQUIRED_SCENARIOS = [
    "ST1-pos",
    "ST1-neg",
    "ST2-pos",
    "ST2-neg",
    "ST2-same-bytes-pos",
    "ST3-pos",
    "ST3-neg",
    "ST4-pos",
    "ST4-neg",
    "ST5-pos",
    "ST5-neg",
    "ST6-pos",
    "ST6-neg",
    "ST7-pos",
    "ST7-neg",
    "ST8-pos",
    "ST8-neg",
]
REQUIRED_MALFORMED = [
    "malformed-schema",
    "missing-signature",
    "swapped-digest",
    "truncated-bundle",
    "forged-publisher",
]
CANONICAL_INTENT_FIELDS = [
    "id",
    "meaning",
    "scope",
    "precedence",
    "activation",
    "required_capability",
    "dependencies",
    "permission_security_boundary",
    "lifecycle",
    "owner",
    "desired_outcome",
    "authority",
    "provenance",
]
MEANING_FIELDS = ["goal", "constraints", "non_goals"]
SCOPE_FIELDS = ["kind", "target", "paths"]
PRECEDENCE_FIELDS = ["rank", "policy", "shadows_lower"]
ACTIVATION_FIELDS = ["trigger", "mode"]
AUTHORITY_FIELDS = ["id", "kind"]
PROVENANCE_FIELDS = ["source", "recorded_at", "digest_sha256"]
INTENT_ID_PREFIX = "intent."
INTENT_LIFECYCLES = ["draft", "active", "deprecated", "revoked"]
PRECEDENCE_POLICIES = [
    "team-required-over-personal-recommended",
    "organization-over-team",
    "exception-over-required",
]
ACTIVATION_MODES = ["required-instruction", "recommended-instruction", "advisory"]
AUTHORITY_KINDS = ["contexpect-native"]
SCOPE_KINDS = LAYER_ORDER + ["member-device"]
CAPABILITY_IDS = [item["id"] for item in CAPABILITIES]
FAMILY_NATIVE_TARGET_FIELDS = [
    "family_id",
    "native_path",
    "syntax",
    "scope",
    "precedence",
    "lifecycle",
    "primitives",
    "managed_channel",
    "alignment",
    "byte_copy_forbidden",
]
FAMILY_PRECEDENCE = "harness-native-document-order"
FAMILY_LIFECYCLE = "preview-apply-rollback"
PROJECTION_REQUIRED_FIELDS = [
    "family_id",
    "coordinate",
    "surface",
    "os_lane",
    "native_path",
    "syntax",
    "native_files",
    "native_digest",
    "projection_outcome",
    "authority",
    "pipeline",
    "capability_negotiation",
    "oracle",
    "equivalence_basis",
    "scope",
    "precedence",
    "lifecycle",
    "primitives",
    "managed_channel",
]
NEGOTIATION_FIELDS = ["required_capability", "status", "unsupported", "unknown", "dropped"]
# Closed enum for a negotiation status, plus the honest tokens that may appear in
# the unknown list alongside capability ids.
NEGOTIATION_STATUSES = [
    "required-supported",
    "required-unknown-honesty",
    "not-applicable",
    "unknown",
    "unsupported",
]
NEGOTIATION_SUPPORTED = "required-supported"
NEGOTIATION_UNKNOWN_STATES = frozenset({"unknown", "required-unknown-honesty"})
NEGOTIATION_UNSUPPORTED_STATES = frozenset({"not-applicable", "unsupported"})
UNKNOWN_CAPABILITY_TOKENS = ["model-visible-complete-prompt"]
OVERLAY_REQUIRED_FIELDS = [
    "id",
    "owner",
    "reason",
    "scope",
    "lifecycle",
    "authority",
    "intent_id",
    "policy_id",
    "weakens_required",
    "projection_outcome",
    "native_files",
    "digest",
]
OVERLAY_SCOPE_FIELDS = ["family_id", "surface", "path"]
PUBLISHER_FIELDS = [
    "id",
    "display_name",
    "signing_identity",
    "role",
    "authorization",
    "key_id",
    "trust_state",
]
PUBLISHER_ROLES = ["team-lead", "publisher"]
PUBLISHER_AUTHORIZATIONS = ["publish-team-standard"]
TRUST_STATES = ["trusted", "revoked", "expired", "unknown"]
TEAM_STANDARD_FIELDS = [
    "stable_id",
    "semantic_version",
    "revision",
    "publisher",
    "members",
    "trust",
    "lineage",
    "compatibility_floors",
    "target_harness_coordinates",
    "canonical_intent_set",
    "policy_rules",
    "release_notes",
    "migration",
    "rollback",
    "content_digest_manifest",
    "signatures",
    "expiry",
    "channel",
    "transport",
    "lifecycle_metadata",
]
COMPAT_FLOOR_FIELDS = ["ctxpect_schema", "harness"]
MIGRATION_FIELDS = ["from_revision", "strategy"]
ROLLBACK_FIELDS = [
    "safe",
    "preserves_unrelated_personal_files",
    "last_known_good_revision",
    "transaction",
]
ROLLBACK_TX_FIELDS = [
    "authorized",
    "target_revision",
    "target_digest",
    "target_is_last_known_good",
    "audit_digest",
    "receipt_digest",
]
SIGNATURE_FIELDS = ["key_id", "alg", "signature", "live_tested", "note"]
MEMBER_RECORD_FIELDS = ["id", "role", "key_id", "authorization", "trust_state"]
TRUST_RECORD_FIELDS = ["publisher_key_id", "state", "verified_key_id"]
LINEAGE_FIELDS = ["current_floor", "previous_revision", "previous_digest", "history"]
LINEAGE_HISTORY_FIELDS = ["revision", "digest_sha256"]
TRANSPORT_FIELDS = ["kind", "local_first", "encrypted_cloud_is_optional"]
LIFECYCLE_META_FIELDS = ["state", "published_at", "expires_at"]
MANIFEST_ENTRY_FIELDS = ["path", "digest_sha256"]
POLICY_RULE_FIELDS = ["id", "layer", "mode", "honesty", "intent_ids", "text"]
EXCEPTION_REQUIRED_FIELDS = [
    "id",
    "state",
    "requester",
    "approver",
    "reason",
    "intent_ids",
    "policy_ids",
    "scope",
    "use_limit",
    "time_limit",
    "timestamps",
    "freshness",
    "signing_identity",
    "signature",
    "audit_chain",
    "overlay_digest",
    "overlay_lifecycle",
]
EXCEPTION_SCOPE_FIELDS = ["member", "device", "project", "harness"]
EXCEPTION_TIME_FIELDS = ["from", "until"]
EXCEPTION_TIMESTAMP_FIELDS = ["requested_at", "issued_at", "approved_at", "expires_at"]
EXCEPTION_TIMESTAMP_ALLOWED = EXCEPTION_TIMESTAMP_FIELDS + ["revoked_at"]
AUDIT_ENTRY_FIELDS = ["sequence", "event", "at", "previous_digest", "actor", "digest"]
# Each audit event is bound to the exception timestamp it records.
AUDIT_EVENT_TIMESTAMP = {
    "request": "requested_at",
    "approval": "approved_at",
    "revocation": "revoked_at",
    "expiry": "expires_at",
}
POLICY_EVAL_FIELDS = [
    "policy_id",
    "mode",
    "result",
    "covered_by_exception",
    "inspection_export_allowed",
]
POLICY_EVAL_RESULTS = ["pass", "deny", "indeterminate"]
AUDIT_EVENTS_BY_STATE = {
    "request": ["request"],
    "approval": ["request", "approval"],
    "rejection": ["request", "rejection"],
    "revocation": ["request", "approval", "revocation"],
    "expiry": ["request", "approval", "expiry"],
}
DISCLOSURE_REQUIRED_FIELDS = [
    "shown_before_report",
    "previewed_at",
    "previewed_fields",
    "preview",
    "upload_consented",
    "consent_status",
    "disclosed_digest",
    "acknowledgement",
    "report_or_upload_at",
    "report_payload_digest",
]
PREVIEW_OBJECT_FIELDS = [
    "standard_version",
    "per_harness_projection",
    "loss_unknown",
    "exception_metadata",
]
CONSENT_STATUSES = ["acknowledged", "denied"]
ACK_FIELDS = ["member_id", "digest", "at"]
LAYER_ASSIGNMENT_FIELDS = ["layer", "mode", "rule_id"]
EFFECTIVE_POLICY_FIELDS = [
    "family_id",
    "computed_via",
    "mode",
    "honesty",
    "shadows_higher_required",
    "note",
    "source_layer",
    "source_rule_id",
    "algorithm",
]
EFFECTIVE_ALGORITHM = "layer-fold-v1"
EFFECTIVE_COMPUTED_VIA = "layer-fold-v1/native-projection-pipeline"
LIFECYCLE_REQUIRED_GROUPS = [
    "steps",
    "staged_rollout",
    "conflict_resolution",
    "last_known_good",
    "key_rotation",
    "rollback",
    "update",
]
LIFECYCLE_ALLOWED = LIFECYCLE_REQUIRED_GROUPS + ["traceable_artifacts"]
STAGED_ROLLOUT_FIELDS = ["cohort", "percent", "order", "stages", "approval", "health_rollback"]
STAGE_FIELDS = ["name", "order", "percent", "approval_required", "health_rollback"]
CONFLICT_FIELDS = ["mode", "silent_merge", "inputs", "winner", "reason"]
LKG_FIELDS = ["revision", "recoverable", "identity", "digest"]
LIFECYCLE_ROLLBACK_FIELDS = [
    "target_revision",
    "target_digest",
    "authorized",
    "preserves_unrelated_personal_files",
    "authorization",
]
ROTATION_FIELDS = [
    "old_key_id",
    "new_key_id",
    "old_trust_state",
    "new_trust_state",
    "rotated_at",
    "applied",
    "activated_at",
    "revoked_at",
    "trust_store_result",
    "dual_authorization",
    "old_signature",
    "new_signature",
]
UPDATE_FIELDS = ["digest", "preview", "disclosure", "apply_at", "preview_digest"]
RECEIPT_REQUIRED_FIELDS = [
    "id",
    "reconciliation_state",
    "native_result_captured",
    "bound_digests",
    "bound_digest_manifest",
]
PAYLOAD_FIELDS = [
    "scenario",
    "canonical_intents",
    "projections",
    "overlays",
    "loss_reports",
    "equivalence_bindings",
    "team_standard",
    "layer_assignments",
    "exceptions",
    "member_disclosure",
    "leader_view",
    "receipts",
    "policy_evals",
    "lifecycle",
    "claimed_alignment",
    "documentation_only",
    "variants",
    "rollback_plan",
    "trust_state",
    "effective_policies",
    "expected_violations",
]
FIXTURE_REQUIRED = [
    "id",
    "schema_version",
    "cutoff",
    "kind",
    "scenario",
    "polarity",
    "expected_gate_result",
    "expected_violations",
    "license",
    "live_tested",
    "digest",
    "payload",
]
FIXTURE_ALLOWED = FIXTURE_REQUIRED + ["notes", "covers"]
CONTRACT_REQUIRED = [
    "schema_version",
    "cutoff",
    "artifact",
    "license",
    "live_tested",
    "projection_outcomes",
    "loss_classes",
    "drift_classes",
    "enforcement_modes",
    "enforcement_honesty",
    "layer_order",
    "reconciliation_states",
    "pipeline_steps",
    "exception_states",
    "standard_lifecycles",
    "required_scenarios",
    "required_malformed_fixtures",
    "fixture_id_mapping",
    "canonical_intent_fields",
    "team_standard_fields",
    "family_native_target_fields",
    "family_native_targets",
    "round_trip_digest_keys",
    "publisher_fields",
    "synthetic_signing",
    "content_digest_manifest",
    "notes",
]
CONTRACT_ALLOWED = list(CONTRACT_REQUIRED)
SECRET_KEY_NAMES = frozenset(
    {
        "token",
        "api_token",
        "api_key",
        "password",
        "secret",
        "secret_value",
        "private_key",
        "private_key_material",
        "authorization",
        "session_token",
        "prompt_text",
        "private_prompt_text",
        "full_session_history",
        "session_body",
    }
)
FORBIDDEN_LEADER_FIELDS = frozenset(
    {
        "private_prompt_text",
        "prompt_text",
        "secrets",
        "secret_value",
        "unrelated_personal_context",
        "full_session_history",
        "session_history",
        "session_body",
    }
)
ALLOWED_LEADER_FIELDS = [
    "standard_id",
    "standard_version",
    "compatibility",
    "compliance_state",
    "semantic_drift_class",
    "loss_unknown",
    "exception_metadata",
    "device_freshness",
    "redacted_evidence_refs",
    "member_id_redacted",
    "harness_id",
]
SYNTHETIC_KEYS = {
    "syn-publisher-team-lead": "contexpect-synthetic-hmac-publisher-v1-offline-fixture",
    "syn-publisher-team-lead-rotated": "contexpect-synthetic-hmac-publisher-rotated-v1-offline-fixture",
    "syn-approver-lead": "contexpect-synthetic-hmac-approver-v1-offline-fixture",
    "syn-member": "contexpect-synthetic-hmac-member-v1-offline-fixture",
}
KEY_REGISTRY = {
    "syn-publisher-team-lead": {
        "role": "team-lead",
        "authorization": "publish-team-standard",
        "trust_state": "trusted",
        "identities": ["syn-publisher-team-lead"],
        "may_publish": True,
        "may_approve_exception": True,
    },
    "syn-publisher-team-lead-rotated": {
        "role": "team-lead",
        "authorization": "publish-team-standard",
        "trust_state": "trusted",
        "identities": ["syn-publisher-team-lead-rotated"],
        "may_publish": True,
        "may_approve_exception": True,
    },
    "syn-approver-lead": {
        "role": "team-lead",
        "authorization": "approve-exception",
        "trust_state": "trusted",
        "identities": ["syn-approver-lead"],
        "may_publish": False,
        "may_approve_exception": True,
    },
    "syn-member": {
        "role": "member",
        "authorization": "request-exception",
        "trust_state": "trusted",
        "identities": ["syn-member"],
        "may_publish": False,
        "may_approve_exception": False,
    },
}
MEMBER_REGISTRY = {
    key: {
        "id": key,
        "key_id": key,
        "role": meta["role"],
        "authorization": meta["authorization"],
        "trust_state": meta["trust_state"],
        "allowed_member": True,
        "may_publish": bool(meta.get("may_publish")),
        "may_approve_exception": bool(meta.get("may_approve_exception")),
    }
    for key, meta in KEY_REGISTRY.items()
}
FROZEN_TS = "2026-09-04T12:00:00+08:00"
EVALUATION_TIME = FROZEN_TS
FROZEN_REQUESTED = "2026-09-04T10:00:00+08:00"
FROZEN_ISSUED = "2026-09-04T11:00:00+08:00"
FROZEN_APPROVED = FROZEN_TS
FROZEN_EXPIRY = "2026-09-04T23:59:59+08:00"
FROZEN_PAST = "2026-08-01T00:00:00+08:00"
FROZEN_FUTURE = "2026-12-31T23:59:59+08:00"
PREVIEW_AT = "2026-09-04T11:00:00+08:00"
ACK_AT = FROZEN_TS
REPORT_AT = "2026-09-04T12:30:00+08:00"
INTENT_ID = "intent.tests-must-use-vitest"
POLICY_ID = "policy.vitest-required"
STANDARD_ID = "tcs.vitest-standard"
AUTHORITY = "contexpect-native"

VIOLATION_CODES = [
    "byte-copy-as-alignment",
    "hash-equality-as-equivalence",
    "projection-outcome-is-reconciliation-state",
    "silent-capability-loss",
    "unsupported-marked-verified",
    "unknown-marked-verified",
    "lossy-without-exception-marked-equivalent",
    "overlay-weakens-required-without-exception",
    "personal-shadows-required",
    "unsigned-standard",
    "secret-in-standard",
    "rollback-unsafe-standard",
    "unbounded-exception",
    "expired-exception-pass",
    "stale-offline-exception-pass",
    "revoked-exception-pass",
    "leader-private-prompt",
    "leader-secrets",
    "leader-personal-context",
    "leader-session-history",
    "documentation-only-claim",
    "missing-signature",
    "swapped-digest",
    "truncated-bundle",
    "forged-publisher",
    "malformed-schema",
    "fork-conflict-silent-merge",
    "rollback-deletes-personal",
    "revoked-key-still-trusted",
    "silent-drop-unsupported",
    "silent-drop-unknown",
    "missing-round-trip-digest",
    "pipeline-order",
    "pipeline-missing",
    "canonical-intent-type",
    "family-native-incomplete",
    "verified-without-oracle",
    "overlay-metadata-missing",
    "overlay-digest-mismatch",
    "exception-scope-mismatch",
    "empty-round-trip-bindings",
    "bound-digest-mismatch",
    "member-key-as-publisher",
    "incomplete-publisher",
    "publisher-role-mismatch",
    "unknown-signing-key",
    "revoked-signing-key",
    "expired-signing-key",
    "rollback-version",
    "required-downgraded-to-recommended",
    "false-enforcement-claim",
    "unmanaged-marked-enforceable",
    "incomplete-exception",
    "forged-exception",
    "broken-audit-chain",
    "invalid-exception-signer",
    "disclosure-missing",
    "disclosure-digest-mismatch",
    "nested-private-content",
    "update-without-preview",
    "update-without-disclosure",
    "invalid-key-rotation",
    "last-known-good-missing",
    "canonical-intent-empty-collection",
    "family-native-mismatch",
    "exception-intent-mismatch",
    "exception-policy-mismatch",
    "exception-overlay-digest-mismatch",
    "publisher-identity-tuple-mismatch",
    "nested-standard-type",
    "signed-rollback-without-authorization",
    "effective-policy-mismatch",
    "computed-via-mismatch",
    "unregistered-identity",
    "approver-signer-mismatch",
    "current-labelled-expired",
    "impossible-time-order",
    "audit-event-state-mismatch",
    "empty-preview",
    "disclosure-payload-mismatch",
    "disclosure-order",
    "missing-lifecycle-group",
    "incomplete-rollout",
    "unsigned-rotation",
    "unrelated-rollback-target",
    "update-preview-digest-mismatch",
    "conflict-nondeterministic",
    "required-collection-cardinality",
    "required-scenario-coverage",
    "undeclared-repeatable-oracle",
    "requester-role-mismatch",
    "invalid-freshness",
    "invalid-signature-alg",
    "lineage-skip",
    "invalid-semver",
    "incomplete-manifest",
    "lifecycle-step-order",
    "untrusted-rotation-key",
    "oracle-grammar",
]

SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
SUPPORTED_SIGNATURE_ALG = "hmac-sha256-synthetic"
FIXTURE_OS_LANE = "macos-27-arm64"
FRESHNESS_STATES = ["current", "stale-offline", "expired", "revoked"]
REQUESTER_ROLES = frozenset({"member"})
APPROVER_ROLES = frozenset({"team-lead", "publisher"})
ORACLE_FIELDS = ["kind", "command", "native_result_captured", "live_tested", "honesty"]
ORACLE_KINDS = ["declared-repeatable", "static-resolver"]
DECLARED_ORACLE_COMMANDS = {
    item["family_id"]: list(item["command"]) for item in DECLARED_NATIVE_ORACLES.values()
}
PIPELINE_STAGE_FIELDS = ["step", "order", "status"]
PIPELINE_STAGE_STATUS = "recorded"
PROJECTION_ALLOWED_FIELDS = PROJECTION_REQUIRED_FIELDS + ["reconciliation_state"]
NATIVE_FILE_FIELDS = ["path", "content", "digest_sha256"]
LIFECYCLE_STEP_REQUIRED = ["command", "actor", "preview", "disclosure"]
LIFECYCLE_STEP_ALLOWED = LIFECYCLE_STEP_REQUIRED + ["state", "pin", "per_harness", "drift_class"]
LIFECYCLE_STEP_ACTORS = ["leader", "member"]
LIFECYCLE_STEP_STATES = ["published", "previewed", "adopted", "projected", "reported"]
LIFECYCLE_STEP_COMMANDS = [
    "standard publish",
    "standard preview",
    "standard adopt",
    "intent project",
    "align status",
]
CONFLICT_MODES = ["explicit-member-choice"]
FAMILY_PATH_EXTRAS = {
    "claude-code": (".claude/rules/testing.md",),
    "cursor": (".cursor/rules/testing.mdc",),
}
SCENARIO_GRAPH = {
    "ST1-pos": {"lists": {"canonical_intents": 1, "projections": 4}},
    "ST2-pos": {"lists": {"canonical_intents": 1, "projections": 18}},
    "ST3-pos": {
        "lists": {
            "canonical_intents": 1,
            "projections": 4,
            "overlays": 1,
            "exceptions": 1,
            "receipts": 1,
        }
    },
    "ST4-pos": {
        "lists": {"canonical_intents": 1, "projections": 4},
        "objects": {"team_standard": True},
    },
    "ST5-pos": {
        "lists": {
            "canonical_intents": 1,
            "projections": 4,
            "layer_assignments": 5,
            "effective_policies": 4,
        },
        "require_policy_required": True,
    },
    "ST6-pos": {
        "lists": {
            "canonical_intents": 1,
            "projections": 4,
            "exceptions": 1,
            "policy_evals": 1,
            "receipts": 1,
        }
    },
    "ST7-pos": {
        "lists": {"canonical_intents": 1},
        "objects": {"member_disclosure": True, "leader_view": True, "team_standard": True},
    },
    "ST8-pos": {
        "lists": {"canonical_intents": 1, "projections": 4, "receipts": 1},
        "objects": {
            "lifecycle": True,
            "team_standard": True,
            "member_disclosure": True,
            "leader_view": True,
        },
    },
}


def _digest_obj(obj: Any) -> str:
    return sha256_text(canonical_json(obj))


def _case_digest(payload: dict[str, Any]) -> str:
    body = dict(payload)
    body.pop("digest", None)
    return _digest_obj(body)


def _is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(ch in "0123456789abcdef" for ch in value)


def _is_timestamp(value: Any) -> bool:
    return isinstance(value, str) and "T" in value and value.endswith("+08:00") and len(value) >= 25


def _is_nonempty_str(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _parse_ts(value: Any) -> datetime | None:
    if not _is_timestamp(value):
        return None
    try:
        return datetime.fromisoformat(value)
    except ValueError:
        return None


def _flag(found: list[str], code: str) -> None:
    if code not in found:
        found.append(code)


def _closed(obj: Any, required: list[str], allowed: list[str] | None = None) -> tuple[bool, str]:
    if not isinstance(obj, dict):
        return False, "type"
    allowed = allowed or required
    extra = set(obj) - set(allowed)
    missing = set(required) - set(obj)
    if extra or missing:
        return False, "keys"
    return True, "ok"


def _require_closed(
    obj: Any,
    required: list[str],
    found: list[str],
    type_code: str,
    extra_code: str | None = None,
    allowed: list[str] | None = None,
) -> bool:
    ok, _reason = _closed(obj, required, allowed)
    if not ok:
        _flag(found, type_code)
        if extra_code:
            _flag(found, extra_code)
        return False
    return True


def _is_semver(value: Any) -> bool:
    return isinstance(value, str) and bool(SEMVER_RE.fullmatch(value))


def _signature_alg_ok(signature: Any) -> bool:
    return isinstance(signature, dict) and signature.get("alg") == SUPPORTED_SIGNATURE_ALG


def _require_nonempty_str_list(value: Any, found: list[str], code: str) -> bool:
    if not isinstance(value, list) or not value or not all(_is_nonempty_str(item) for item in value):
        _flag(found, code)
        return False
    return True


def family_native_syntax(family_id: str) -> str:
    documented = {
        "codex": "markdown-nested-agents-chain",
        "claude-code": "markdown-plus-path-scoped-rules",
        "cursor": "cursor-mdc-project-rules",
        "grok-build": "grok-project-instructions",
    }
    if family_id in documented:
        return documented[family_id]
    fmt = FAMILY_INPUT_SPECS[family_id][1]
    return f"{family_id}-{fmt}"


def family_allowed_paths(family_id: str) -> frozenset[str]:
    spec = FAMILY_INPUT_SPECS.get(family_id)
    paths = {spec[0]} if spec else set()
    paths.update(FAMILY_PATH_EXTRAS.get(family_id, ()))
    return frozenset(paths)


def path_allowed_for_family(path: Any, family_id: str) -> bool:
    return _is_nonempty_str(path) and str(path) in family_allowed_paths(family_id)


def lkg_digest() -> str:
    return sha256_text(f"{STANDARD_ID}@0")


def effective_note(honesty: str) -> str:
    if honesty == "enforceable":
        return "Managed native instruction channel is enforceable for this required rule."
    if honesty == "detect-only":
        return "Target has no stable managed write surface; report detect-only instead of claiming compliance."
    return "Connector or unmanaged target cannot be reported as enforceable."


def _fold_layers(assignments: list[Any]) -> dict[str, dict[str, str]]:
    folded: dict[str, dict[str, str]] = {}
    for layer in LAYER_ORDER:
        for row in assignments:
            if not isinstance(row, dict) or row.get("layer") != layer:
                continue
            rule = row.get("rule_id")
            mode = row.get("mode")
            if not _is_nonempty_str(rule) or mode not in ENFORCEMENT_MODES:
                continue
            prev = folded.get(str(rule))
            if prev and _mode_rank(mode) < _mode_rank(prev["mode"]) and prev["mode"] in {"required", "prohibited"}:
                continue
            folded[str(rule)] = {"mode": str(mode), "layer": str(layer)}
    return folded


def _effective_exception_applies(payload: dict[str, Any], family_id: str) -> bool:
    """A currently valid exception scoped to this harness suspends the enforceable claim."""
    for exc in payload.get("exceptions") or []:
        if not isinstance(exc, dict) or not _exception_currently_valid(exc):
            continue
        scope = exc.get("scope") if isinstance(exc.get("scope"), dict) else {}
        if scope.get("harness") in {family_id, "*"}:
            return True
    return False


def recompute_effective_row(payload: dict[str, Any], family_id: str) -> dict[str, Any]:
    folded = _fold_layers(payload.get("layer_assignments") or [])
    if POLICY_ID in folded:
        rule = POLICY_ID
        chosen = folded[POLICY_ID]
    elif folded:
        rule, chosen = next(iter(folded.items()))
    else:
        rule = POLICY_ID
        chosen = {"mode": "recommended", "layer": "personal"}
    projections = payload.get("projections") or []
    proj = next((item for item in projections if isinstance(item, dict) and item.get("family_id") == family_id), None)
    registry_managed = family_managed_channel(family_id)
    if proj is None:
        # Without projection evidence for this harness there is no managed channel to claim.
        honesty = derived_enforcement_honesty(family_id, False)
    else:
        claimed = proj.get("managed_channel")
        managed = bool(registry_managed and claimed) if isinstance(claimed, bool) else False
        honesty = derived_enforcement_honesty(family_id, managed)
        negotiation = proj.get("capability_negotiation") if isinstance(proj.get("capability_negotiation"), dict) else {}
        status = negotiation.get("status")
        if status in NEGOTIATION_UNSUPPORTED_STATES or negotiation.get("unsupported"):
            honesty = "non-enforceable"
        elif status in NEGOTIATION_UNKNOWN_STATES or negotiation.get("unknown") or negotiation.get("dropped"):
            if honesty == "enforceable":
                honesty = "detect-only"
    if honesty == "enforceable" and _effective_exception_applies(payload, family_id):
        honesty = "detect-only"
    return {
        "family_id": family_id,
        "computed_via": EFFECTIVE_COMPUTED_VIA,
        "mode": chosen["mode"],
        "honesty": honesty,
        "shadows_higher_required": False,
        "note": effective_note(honesty),
        "source_layer": chosen["layer"],
        "source_rule_id": rule,
        "algorithm": EFFECTIVE_ALGORITHM,
    }


def registry_capability_status(family_id: str, capability: str) -> str:
    """Authoritative capability status for a family's fixture surface."""
    meta = _surface_for(family_id)
    family = family_by_id(family_id)
    surface = next(item for item in family["surfaces"] if item["id"] == meta["surface"])
    return capability_status(family_id, meta["surface"], capability, surface["static_support"])


def negotiation_states_allowed(registry_status: str) -> frozenset[str]:
    """Negotiation may degrade honestly below the registry, never claim more than it."""
    if registry_status == "required-supported":
        return frozenset({NEGOTIATION_SUPPORTED}) | NEGOTIATION_UNKNOWN_STATES
    if registry_status == "required-unknown-honesty":
        return NEGOTIATION_UNKNOWN_STATES
    return NEGOTIATION_UNSUPPORTED_STATES


def family_primitives(family_id: str) -> list[str]:
    family = family_by_id(family_id)
    surface = family["surfaces"][0]
    prims = []
    for cap in CAPABILITIES:
        status = capability_status(family_id, surface["id"], cap["id"], surface["static_support"])
        if status == "required-supported":
            prims.append(cap["id"])
    if not prims:
        if family_id in CONNECTOR_FAMILIES:
            prims = ["mcp-declarations"]
        else:
            prims = ["instructions"]
    return prims


def family_managed_channel(family_id: str) -> bool:
    if family_id in CONNECTOR_FAMILIES:
        return False
    project = (NATIVE_TARGETS.get("project-instructions") or {}).get(family_id)
    if project and project.get("kind") in {"ui-handoff", "export-only"}:
        return False
    return True


def derived_enforcement_honesty(family_id: str, managed_channel: Any) -> str:
    if family_id in CONNECTOR_FAMILIES:
        return "non-enforceable"
    if managed_channel is False:
        return "detect-only"
    return "enforceable"


def intent_provenance_digest(intent: dict[str, Any]) -> str:
    return _digest_obj({"id": intent.get("id"), "meaning": intent.get("meaning")})


def _mode_rank(mode: Any) -> int:
    return {"prohibited": 3, "required": 2, "recommended": 1}.get(mode, 0)


def synthetic_sign(payload: Any, key_id: str) -> dict[str, Any]:
    material = SYNTHETIC_KEYS[key_id]
    mac = hmac.new(material.encode("utf-8"), canonical_json(payload).encode("utf-8"), hashlib.sha256).hexdigest()
    return {
        "key_id": key_id,
        "alg": "hmac-sha256-synthetic",
        "signature": mac,
        "live_tested": False,
        "note": "offline synthetic fixture key; not a user token",
    }


def verify_synthetic(payload: Any, signature: dict[str, Any]) -> bool:
    key_id = signature.get("key_id")
    if key_id not in SYNTHETIC_KEYS:
        return False
    expected = synthetic_sign(payload, key_id)
    return hmac.compare_digest(expected["signature"], str(signature.get("signature") or ""))


def family_native_targets() -> list[dict[str, Any]]:
    rows = []
    for family in FAMILIES:
        spec = FAMILY_INPUT_SPECS[family["id"]]
        project = (NATIVE_TARGETS.get("project-instructions") or {}).get(family["id"])
        rows.append(
            {
                "family_id": family["id"],
                "native_path": spec[0] if not project else project["path_glob"],
                "syntax": family_native_syntax(family["id"]),
                "scope": project["scope"] if project else "project",
                "precedence": FAMILY_PRECEDENCE,
                "lifecycle": FAMILY_LIFECYCLE,
                "primitives": family_primitives(family["id"]),
                "managed_channel": family_managed_channel(family["id"]),
                "alignment": "harness-native-projection",
                "byte_copy_forbidden": True,
            }
        )
    return rows


def family_target_map() -> dict[str, dict[str, Any]]:
    return {row["family_id"]: row for row in family_native_targets()}


def _surface_for(family_id: str) -> dict[str, str]:
    family = next(item for item in FAMILIES if item["id"] == family_id)
    if family_id == "cursor":
        surface = next(item for item in family["surfaces"] if item["id"] == "ide")
    else:
        surface = family["surfaces"][0]
    return {
        "family_id": family_id,
        "surface": surface["id"],
        "version": surface["version"],
        "os_lane": FIXTURE_OS_LANE,
        "coordinate": f"{family_id}/{surface['version']}/{surface['id']}/{FIXTURE_OS_LANE}",
    }


def empty_payload() -> dict[str, Any]:
    null_keys = {
        "team_standard",
        "member_disclosure",
        "leader_view",
        "lifecycle",
        "claimed_alignment",
        "rollback_plan",
        "trust_state",
    }
    payload: dict[str, Any] = {}
    for key in PAYLOAD_FIELDS:
        if key == "documentation_only":
            payload[key] = False
        elif key == "scenario":
            payload[key] = ""
        elif key in null_keys:
            payload[key] = None
        else:
            payload[key] = []
    return payload


def pipeline_record() -> list[dict[str, Any]]:
    return [{"step": name, "order": index + 1, "status": "recorded"} for index, name in enumerate(PIPELINE_STEPS)]


def oracle_block(family_id: str) -> dict[str, Any]:
    if family_id == "codex":
        return {
            "kind": "declared-repeatable",
            "command": ["codex", "debug", "prompt-input"],
            "native_result_captured": False,
            "live_tested": False,
            "honesty": "recipe-only-not-a-native-observation",
        }
    if family_id == "grok-build":
        return {
            "kind": "declared-repeatable",
            "command": ["grok", "inspect", "--json"],
            "native_result_captured": False,
            "live_tested": False,
            "honesty": "discovered-configuration-does-not-prove-model-visible",
        }
    return {
        "kind": "static-resolver",
        "command": None,
        "native_result_captured": False,
        "live_tested": False,
        "honesty": "no-declared-repeatable-oracle-unknown-or-static-only",
    }


def vitest_intent() -> dict[str, Any]:
    meaning = {
        "goal": "JavaScript unit and component tests must use Vitest",
        "constraints": ["Do not introduce Jest as the project test runner"],
        "non_goals": ["Do not rewrite existing non-JavaScript tests"],
    }
    intent = {
        "id": INTENT_ID,
        "meaning": meaning,
        "scope": {
            "kind": "project",
            "target": "javascript-tests",
            "paths": ["packages/**", "apps/**"],
        },
        "precedence": {
            "rank": "team",
            "policy": "team-required-over-personal-recommended",
            "shadows_lower": True,
        },
        "activation": {"trigger": "javascript-test-paths", "mode": "required-instruction"},
        "required_capability": "instructions",
        "dependencies": [],
        "permission_security_boundary": "instructions-are-not-sandbox-policy",
        "lifecycle": "active",
        "owner": "syn-publisher-team-lead",
        "desired_outcome": "Each target harness encodes the Vitest requirement in its native instruction/rule primitive",
        "authority": {"id": AUTHORITY, "kind": "contexpect-native"},
        "provenance": {
            "source": "synthetic-acceptance-fixture",
            "recorded_at": FROZEN_TS,
            "digest_sha256": "",
        },
    }
    intent["provenance"]["digest_sha256"] = intent_provenance_digest(intent)
    return intent


def native_file(path: str, content: str) -> dict[str, str]:
    return {"path": path, "content": content, "digest_sha256": sha256_text(content)}


def native_bundle_digest(files: list[dict[str, str]]) -> str:
    return _digest_obj([{"path": item["path"], "digest_sha256": item["digest_sha256"]} for item in files])


# Authoritative native projection bodies. Single source for the builder and the
# validator: a projection must carry exactly these paths and exactly this content,
# so a deleted path rule or a cross-family content swap cannot pass.
FAMILY_NATIVE_BODIES: dict[str, tuple[tuple[str, str], ...]] = {
    "codex": (
        (
            "AGENTS.md",
            "# AGENTS.md\n\n"
            "<!-- contexpect-native: Codex nested instruction chain; not a byte copy -->\n\n"
            "## Testing\n\n"
            "This repository's JavaScript tests must use Vitest.\n"
            "Do not add Jest as the project test runner.\n",
        ),
    ),
    "claude-code": (
        (
            "CLAUDE.md",
            "# CLAUDE.md\n\n"
            "## Testing policy\n\n"
            "Use Vitest for JavaScript unit and component tests. Jest is not the project test runner.\n",
        ),
        (
            ".claude/rules/testing.md",
            "---\npaths:\n  - \"packages/**\"\n  - \"apps/**\"\n---\n"
            "JavaScript tests in these paths must run on Vitest.\n",
        ),
    ),
    "cursor": (
        (
            ".cursor/rules/testing.mdc",
            "---\ndescription: Project test runner\n"
            "globs: **/*.{test,spec}.{ts,tsx,js,jsx}\nalwaysApply: false\n---\n"
            "This project requires Vitest. Do not introduce Jest as the test runner.\n",
        ),
    ),
    "grok-build": (
        (
            "AGENTS.md",
            "# Project instructions (Grok Build)\n\n"
            "[testing]\nrequired_runner = Vitest\nprohibited_runner = Jest\n",
        ),
    ),
    "deepseek-harness": (
        (
            "AGENTS.md",
            "# DeepSeek Harness\n\nVitest is required for JavaScript tests. Native model-visible prompt is unknown.\n",
        ),
    ),
    # C-F02 (2026-09-12): kimi-code and zcode both read AGENTS.md markdown project
    # instructions, and no native mechanism is known to require different bodies, so
    # their authoritative bodies are byte-identical on purpose. Byte identity is
    # neither proof of semantic equivalence nor evidence against it.
    "kimi-code": (
        (
            "AGENTS.md",
            "# AGENTS.md\n\n"
            "## Testing\n\n"
            "This repository's JavaScript tests must use Vitest.\n"
            "Do not add Jest as the project test runner.\n",
        ),
    ),
    "zcode": (
        (
            "AGENTS.md",
            "# AGENTS.md\n\n"
            "## Testing\n\n"
            "This repository's JavaScript tests must use Vitest.\n"
            "Do not add Jest as the project test runner.\n",
        ),
    ),
}


def family_native_content(family_id: str) -> list[dict[str, str]]:
    entries = FAMILY_NATIVE_BODIES.get(family_id)
    if entries is None:
        path = FAMILY_INPUT_SPECS[family_id][0]
        entries = ((path, f"# {family_id}\nJavaScript tests must use Vitest. Jest is not the project runner.\n"),)
    return [native_file(path, body) for path, body in entries]


def family_required_paths(family_id: str) -> tuple[str, ...]:
    return tuple(item["path"] for item in family_native_content(family_id))


def four_native_files() -> dict[str, list[dict[str, str]]]:
    return {family_id: family_native_content(family_id) for family_id in REQUIRED_FOUR}


def four_syntax() -> dict[str, str]:
    return {
        "codex": "markdown-nested-agents-chain",
        "claude-code": "markdown-plus-path-scoped-rules",
        "cursor": "cursor-mdc-project-rules",
        "grok-build": "grok-project-instructions",
    }


def four_outcomes() -> dict[str, str]:
    return {
        "codex": "transformed",
        "claude-code": "transformed",
        "cursor": "native-equivalent",
        "grok-build": "native-equivalent",
    }


def make_projection(family_id: str, files: list[dict[str, str]], outcome: str, extra: dict[str, Any] | None = None) -> dict[str, Any]:
    meta = _surface_for(family_id)
    target = family_target_map()[family_id]
    negotiation = {
        "required_capability": "instructions",
        "status": "required-supported",
        "unsupported": [],
        "unknown": [],
        "dropped": [],
    }
    row = {
        "family_id": family_id,
        "coordinate": meta["coordinate"],
        "surface": meta["surface"],
        "os_lane": meta["os_lane"],
        "native_path": files[0]["path"],
        "syntax": target["syntax"],
        "native_files": files,
        "native_digest": native_bundle_digest(files),
        "projection_outcome": outcome,
        "authority": AUTHORITY,
        "pipeline": pipeline_record(),
        "capability_negotiation": negotiation,
        "oracle": oracle_block(family_id),
        "equivalence_basis": "canonical-intent-semantics",
        "scope": target["scope"],
        "precedence": target["precedence"],
        "lifecycle": target["lifecycle"],
        "primitives": list(target["primitives"]),
        "managed_channel": target["managed_channel"],
    }
    if extra:
        row.update(extra)
    return row


def bound_receipt(
    projections: list[dict[str, Any]],
    exception_id: str | None = None,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    stub = dict(payload or {})
    stub["projections"] = projections
    if exception_id and not stub.get("exceptions"):
        stub["exceptions"] = [{"id": exception_id}]
    bound = _expected_bound_digests(stub, projections)
    return {
        "id": "receipt.semantic-team-roundtrip",
        "reconciliation_state": "structural-only",
        "native_result_captured": False,
        "bound_digests": bound,
        "bound_digest_manifest": _digest_obj(bound),
    }


def policy_rule(mode: str = "required") -> dict[str, Any]:
    return {
        "id": POLICY_ID,
        "layer": "team",
        "mode": mode,
        "honesty": "enforceable",
        "intent_ids": [INTENT_ID],
        "text": "JavaScript tests must use Vitest",
    }


def standard_unsigned(extra: dict[str, Any] | None = None) -> dict[str, Any]:
    intent = vitest_intent()
    rule = policy_rule()
    manifest = [
        {"path": f"canonical_intent_set/{INTENT_ID}", "digest_sha256": _digest_obj(intent)},
        {"path": f"policy_rules/{POLICY_ID}", "digest_sha256": _digest_obj(rule)},
    ]
    body = {
        "stable_id": STANDARD_ID,
        "semantic_version": "1.0.0",
        "revision": 1,
        "publisher": {
            "id": "syn-publisher-team-lead",
            "display_name": "Synthetic Team Lead",
            "signing_identity": "syn-publisher-team-lead",
            "role": "team-lead",
            "authorization": "publish-team-standard",
            "key_id": "syn-publisher-team-lead",
            "trust_state": "trusted",
        },
        "members": [
            {
                "id": key,
                "role": meta["role"],
                "key_id": key,
                "authorization": meta["authorization"],
                "trust_state": meta["trust_state"],
            }
            for key, meta in KEY_REGISTRY.items()
        ],
        "trust": {
            "publisher_key_id": "syn-publisher-team-lead",
            "state": "trusted",
            "verified_key_id": "syn-publisher-team-lead",
        },
        "lineage": {
            "current_floor": 1,
            "previous_revision": 0,
            "previous_digest": lkg_digest(),
            "history": [{"revision": 0, "digest_sha256": lkg_digest()}],
        },
        "compatibility_floors": {
            "ctxpect_schema": SCHEMA_VERSION,
            "harness": {
                "codex": "0.147.0",
                "claude-code": "2.1.259",
                "cursor": "3.19.7",
                "grok-build": "1.0.13",
            },
        },
        "target_harness_coordinates": [_surface_for(fid)["coordinate"] for fid in REQUIRED_FOUR],
        "canonical_intent_set": [INTENT_ID],
        "policy_rules": [rule],
        "release_notes": "Synthetic Team Context Standard for the Vitest instruction. Git/file local-first; encrypted cloud is optional transport only.",
        "migration": {"from_revision": 0, "strategy": "adopt-then-project"},
        "rollback": {
            "safe": True,
            "preserves_unrelated_personal_files": True,
            "last_known_good_revision": 0,
            "transaction": {
                "authorized": False,
                "target_revision": 0,
                "target_digest": lkg_digest(),
                "target_is_last_known_good": True,
                "audit_digest": sha256_text("none"),
                "receipt_digest": sha256_text("none"),
            },
        },
        "content_digest_manifest": manifest,
        "signatures": [],
        "expiry": FROZEN_FUTURE,
        "channel": "git-file-local",
        "transport": {
            "kind": "git-file-local",
            "local_first": True,
            "encrypted_cloud_is_optional": True,
        },
        "lifecycle_metadata": {
            "state": "publish",
            "published_at": FROZEN_TS,
            "expires_at": FROZEN_FUTURE,
        },
    }
    if extra:
        body.update(extra)
    return body


def sign_standard(body: dict[str, Any], key_id: str = "syn-publisher-team-lead") -> dict[str, Any]:
    unsigned = dict(body)
    unsigned["signatures"] = []
    signed = dict(body)
    signed["signatures"] = [synthetic_sign(unsigned, key_id)]
    return signed


def valid_standard() -> dict[str, Any]:
    return sign_standard(standard_unsigned())


def audit_chain(
    events: list[str],
    actor: str = "syn-approver-lead",
    actors: list[str] | None = None,
    times: list[str] | None = None,
) -> list[dict[str, Any]]:
    rows = []
    previous = "genesis"
    for index, event in enumerate(events, start=1):
        payload = {
            "sequence": index,
            "event": event,
            "at": (times[index - 1] if times else FROZEN_TS),
            "previous_digest": previous,
            "actor": (actors[index - 1] if actors else actor),
        }
        digest = _digest_obj(payload)
        rows.append({**payload, "digest": digest})
        previous = digest
    return rows


def resign_exception(exc: dict[str, Any], key_id: str | None = None) -> dict[str, Any]:
    key_id = key_id or str(exc.get("signing_identity") or "syn-approver-lead")
    body = {key: value for key, value in exc.items() if key != "signature"}
    exc["signature"] = synthetic_sign(body, key_id)
    return exc


def valid_exception(state: str = "approval") -> dict[str, Any]:
    events = list(AUDIT_EVENTS_BY_STATE[state])
    actor_for = {
        "request": "syn-member",
        "approval": "syn-approver-lead",
        "rejection": "syn-approver-lead",
        "revocation": "syn-approver-lead",
        "expiry": "syn-approver-lead",
    }
    time_for = {
        "request": FROZEN_REQUESTED,
        "approval": FROZEN_APPROVED,
        "rejection": FROZEN_APPROVED,
        "revocation": FROZEN_APPROVED,
        "expiry": FROZEN_EXPIRY,
    }
    stamps = {
        "requested_at": FROZEN_REQUESTED,
        "issued_at": FROZEN_ISSUED,
        "approved_at": FROZEN_APPROVED,
        "expires_at": FROZEN_EXPIRY,
    }
    if state == "revocation":
        stamps["revoked_at"] = FROZEN_APPROVED
    body = {
        "id": "exc.scoped-vitest-jest-migration",
        "state": state,
        "requester": "syn-member",
        "approver": "syn-approver-lead",
        "reason": "legacy package still on Jest during a bounded migration window",
        "intent_ids": [INTENT_ID],
        "policy_ids": [POLICY_ID],
        "scope": {
            "member": "syn-member",
            "device": "device-a",
            "project": "packages/legacy",
            "harness": "cursor",
        },
        "use_limit": 3,
        "time_limit": {"from": FROZEN_REQUESTED, "until": FROZEN_EXPIRY},
        "timestamps": stamps,
        "freshness": "current",
        "signing_identity": "syn-approver-lead",
        "audit_chain": audit_chain(
            events,
            actors=[actor_for[event] for event in events],
            times=[time_for[event] for event in events],
        ),
        "overlay_digest": sha256_text("none"),
        "overlay_lifecycle": FAMILY_LIFECYCLE,
    }
    return resign_exception(body, "syn-approver-lead")


def disclosure_binding_digest(disclosure: dict[str, Any], payload: dict[str, Any] | None = None) -> str:
    payload = payload or {}
    preview = disclosure.get("preview") if isinstance(disclosure.get("preview"), dict) else {}
    ack = disclosure.get("acknowledgement") if isinstance(disclosure.get("acknowledgement"), dict) else {}
    standard = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
    lifecycle = payload.get("lifecycle") if isinstance(payload.get("lifecycle"), dict) else {}
    update = lifecycle.get("update") if isinstance(lifecycle.get("update"), dict) else {}
    return _digest_obj(
        {
            "previewed_fields": list(PREVIEW_OBJECT_FIELDS),
            "preview": preview,
            "member_id": ack.get("member_id"),
            "team_id": standard.get("stable_id") or STANDARD_ID,
            "version": preview.get("standard_version"),
            "update_id": update.get("digest") or sha256_text("none"),
            "payload_id": disclosure.get("report_payload_digest") or sha256_text("none"),
            "previewed_at": disclosure.get("previewed_at"),
            "acknowledged_at": ack.get("at"),
            # The member acknowledges a decision, not only a preview: the upload
            # consent bit must move this digest so it cannot be flipped afterwards.
            "consent_status": disclosure.get("consent_status"),
            "upload_consented": disclosure.get("upload_consented"),
        }
    )


def expected_update_digest(payload: dict[str, Any] | None = None) -> str:
    """Derive the update digest from the content actually delivered by the update.

    Identity plus revision is not enough: a changed policy set, content manifest,
    receipt binding or disclosure must move this digest.
    """
    payload = payload or {}
    standard = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
    disclosure = payload.get("member_disclosure") if isinstance(payload.get("member_disclosure"), dict) else {}
    receipts = [item for item in (payload.get("receipts") or []) if isinstance(item, dict)]
    return _digest_obj(
        {
            "stable_id": standard.get("stable_id") or STANDARD_ID,
            "revision": standard.get("revision") if isinstance(standard.get("revision"), int) else 1,
            "apply": True,
            "semantic_version": standard.get("semantic_version"),
            "policy_rules": [
                item for item in (standard.get("policy_rules") or []) if isinstance(item, dict)
            ],
            "content_digest_manifest": [
                item for item in (standard.get("content_digest_manifest") or []) if isinstance(item, dict)
            ],
            "receipt_manifests": [item.get("bound_digest_manifest") for item in receipts],
            "disclosure_preview": disclosure.get("preview"),
            "disclosure_payload_digest": disclosure.get("report_payload_digest"),
        }
    )


def bind_report_payload(payload: dict[str, Any]) -> None:
    disclosure = payload.get("member_disclosure")
    if not isinstance(disclosure, dict) or not isinstance(disclosure.get("preview"), dict):
        return
    view = payload.get("leader_view")
    if isinstance(view, dict):
        disclosure["report_payload_digest"] = _digest_obj(view)
    else:
        disclosure["report_payload_digest"] = _digest_obj(disclosure.get("preview") or {})


def bind_update(payload: dict[str, Any]) -> None:
    lifecycle = payload.get("lifecycle")
    if not isinstance(lifecycle, dict):
        return
    update = lifecycle.get("update")
    if not isinstance(update, dict):
        return
    digest = expected_update_digest(payload)
    if "digest" in update:
        update["digest"] = digest
    if "preview_digest" in update:
        update["preview_digest"] = digest


def bind_disclosure(payload: dict[str, Any]) -> None:
    disclosure = payload.get("member_disclosure")
    if not isinstance(disclosure, dict) or not isinstance(disclosure.get("preview"), dict):
        return
    bind_report_payload(payload)
    digest = disclosure_binding_digest(disclosure, payload)
    disclosure["disclosed_digest"] = digest
    ack = disclosure.get("acknowledgement")
    if isinstance(ack, dict):
        ack["digest"] = digest


def bind_receipts(payload: dict[str, Any]) -> None:
    receipts = payload.get("receipts")
    if not isinstance(receipts, list):
        return
    projections = [item for item in (payload.get("projections") or []) if isinstance(item, dict)]
    expected = _expected_bound_digests(payload, projections)
    for receipt in receipts:
        if not isinstance(receipt, dict):
            continue
        bound = receipt.get("bound_digests")
        if isinstance(bound, dict) and set(ROUND_TRIP_DIGEST_KEYS) <= set(bound):
            receipt["bound_digests"] = expected
            receipt["bound_digest_manifest"] = _digest_obj(expected)


def member_disclosure(view: dict[str, Any] | None = None) -> dict[str, Any]:
    preview = {
        "standard_version": "1.0.0",
        "per_harness_projection": True,
        "loss_unknown": True,
        "exception_metadata": True,
    }
    payload_digest = _digest_obj(view) if view is not None else _digest_obj(preview)
    report_at = REPORT_AT if view is not None else ACK_AT
    row = {
        "shown_before_report": True,
        "previewed_at": PREVIEW_AT,
        "previewed_fields": list(PREVIEW_OBJECT_FIELDS),
        "preview": preview,
        "upload_consented": False,
        "consent_status": "acknowledged",
        "disclosed_digest": "",
        "acknowledgement": {
            "member_id": "syn-member",
            "digest": "",
            "at": ACK_AT,
        },
        "report_or_upload_at": report_at,
        "report_payload_digest": payload_digest,
    }
    digest = disclosure_binding_digest(row, {})
    row["disclosed_digest"] = digest
    row["acknowledgement"]["digest"] = digest
    return row


def leader_view_redacted() -> dict[str, Any]:
    return {
        "standard_id": STANDARD_ID,
        "standard_version": "1.0.0",
        "compatibility": {"floors": ["codex", "claude-code", "cursor", "grok-build"]},
        "compliance_state": "structural-only",
        "semantic_drift_class": "benign-native-representation",
        "loss_unknown": {"loss_classes": ["harness-only"], "unknown": True},
        "exception_metadata": {
            "id": "exc.scoped-vitest-jest-migration",
            "state": "approval",
            "scope": "packages/legacy",
            "expires_at": FROZEN_EXPIRY,
        },
        "device_freshness": {"device-a": "current"},
        "redacted_evidence_refs": ["evidence:redacted:intent.tests-must-use-vitest"],
        "member_id_redacted": "member-redacted",
        "harness_id": "cursor",
    }


def layer_assignments_ok() -> list[dict[str, Any]]:
    return [
        {"layer": "organization", "mode": "required", "rule_id": POLICY_ID, "honesty": "enforceable"},
        {"layer": "team", "mode": "required", "rule_id": POLICY_ID, "honesty": "enforceable"},
        {"layer": "project", "mode": "required", "rule_id": POLICY_ID, "honesty": "enforceable"},
        {"layer": "role", "mode": "recommended", "rule_id": "policy.format-on-save", "honesty": "detect-only"},
        {"layer": "personal", "mode": "recommended", "rule_id": "policy.personal-theme", "honesty": "non-enforceable"},
    ]


def effective_policies_ok(
    projections: list[dict[str, Any]] | None = None,
    assignments: list[dict[str, Any]] | None = None,
) -> list[dict[str, Any]]:
    payload = {
        "layer_assignments": assignments or layer_assignments_ok(),
        "projections": projections or four_projections(),
    }
    return [recompute_effective_row(payload, family_id) for family_id in REQUIRED_FOUR]


def four_projections() -> list[dict[str, Any]]:
    files = four_native_files()
    outcomes = four_outcomes()
    return [make_projection(fid, files[fid], outcomes[fid]) for fid in REQUIRED_FOUR]


def equivalence_ok(projections: list[dict[str, Any]]) -> list[dict[str, Any]]:
    digests = [item["native_digest"] for item in projections]
    return [
        {
            "intent_id": INTENT_ID,
            "family_ids": [item["family_id"] for item in projections],
            "equivalent": True,
            "byte_identical": len(set(digests)) == 1,
            "equivalence_basis": "canonical-intent-semantics",
            "text_hash_equality_is_not_semantic_equivalence": True,
            "native_digests": digests,
        }
    ]


def loss_harness_only() -> list[dict[str, Any]]:
    return [
        {
            "intent_id": INTENT_ID,
            "family_id": "codex",
            "entries": [
                {
                    "class": "harness-only",
                    "detail": "Codex nested AGENTS chain comment is harness-specific and is not required of other families",
                }
            ],
        }
    ]


def overlay_ok() -> dict[str, Any]:
    files = four_native_files()["claude-code"]
    body = {
        "id": "overlay.claude-ui-packages-glob",
        "owner": "syn-publisher-team-lead",
        "reason": "Claude path-scoped rules need an extra glob for packages/ui",
        "scope": {
            "family_id": "claude-code",
            "surface": "cli",
            "path": ".claude/rules/testing.md",
        },
        "lifecycle": FAMILY_LIFECYCLE,
        "authority": AUTHORITY,
        "intent_id": INTENT_ID,
        "policy_id": POLICY_ID,
        "weakens_required": False,
        "projection_outcome": "lossless-native-overlay",
        "native_files": files,
    }
    body["digest"] = _digest_obj({k: v for k, v in body.items() if k != "digest"})
    return body


def covering_exception_for(overlay: dict[str, Any], **overrides: Any) -> dict[str, Any]:
    exc = valid_exception()
    exc["intent_ids"] = [overlay.get("intent_id") or INTENT_ID]
    exc["policy_ids"] = [overlay.get("policy_id") or POLICY_ID]
    scope = dict(exc["scope"])
    ov_scope = overlay.get("scope") if isinstance(overlay.get("scope"), dict) else {}
    scope["harness"] = ov_scope.get("family_id") or scope["harness"]
    scope["project"] = ov_scope.get("path") or scope["project"]
    exc["scope"] = scope
    exc["overlay_digest"] = overlay.get("digest") or sha256_text("none")
    exc["overlay_lifecycle"] = overlay.get("lifecycle") or FAMILY_LIFECYCLE
    exc.update(overrides)
    return resign_exception(exc)


def with_payload(**kwargs: Any) -> dict[str, Any]:
    payload = empty_payload()
    payload.update(kwargs)
    return payload


def finalize_fixture(
    scenario: str,
    polarity: str,
    payload: dict[str, Any],
    expected_violations: list[str],
    notes: str,
    covers: list[str] | None = None,
) -> dict[str, Any]:
    payload["scenario"] = scenario
    bind_report_payload(payload)
    bind_receipts(payload)
    bind_update(payload)
    bind_disclosure(payload)
    row = {
        "id": scenario,
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "kind": "semantic-team-fixture",
        "scenario": scenario,
        "polarity": polarity,
        "expected_gate_result": "pass" if polarity == "positive" else "fail",
        "expected_violations": expected_violations,
        "license": LICENSE_ID,
        "live_tested": False,
        "payload": payload,
        "notes": notes,
        "covers": covers or [scenario],
    }
    row["digest"] = _case_digest(row)
    return row


def identical_copy_files() -> list[dict[str, str]]:
    return [native_file("AGENTS.md", "# Identical copied instruction\nTests must use Vitest.\n")]


def build_st1_pos() -> dict[str, Any]:
    projections = four_projections()
    return finalize_fixture(
        "ST1-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            equivalence_bindings=equivalence_ok(projections),
            loss_reports=loss_harness_only(),
        ),
        [],
        "One CanonicalIntent is projected to harness-native artifacts; files are not identical.",
    )


def build_st1_neg() -> dict[str, Any]:
    copied = identical_copy_files()
    digest = native_bundle_digest(copied)
    projections = []
    for family_id in REQUIRED_FOUR:
        projections.append(
            make_projection(
                family_id,
                copied,
                "native-equivalent",
                extra={"equivalence_basis": "hash-equality", "native_path": "AGENTS.md"},
            )
        )
    claimed = {
        "method": "copy-identical-md",
        "path": "AGENTS.md",
        "hash": digest,
        "treated_as_semantic_equivalence": True,
        "targets": REQUIRED_FOUR,
    }
    binding = {
        "intent_id": INTENT_ID,
        "family_ids": REQUIRED_FOUR,
        "equivalent": True,
        "byte_identical": True,
        "equivalence_basis": "hash-equality",
        "text_hash_equality_is_not_semantic_equivalence": False,
        "native_digests": [digest] * 4,
    }
    return finalize_fixture(
        "ST1-neg",
        "negative",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            equivalence_bindings=[binding],
            claimed_alignment=claimed,
        ),
        ["byte-copy-as-alignment", "hash-equality-as-equivalence"],
        "Treating byte/hash equality of one copied MD as alignment must fail.",
    )


def honest_unknown_projection() -> dict[str, Any]:
    files = family_native_content("deepseek-harness")
    return make_projection(
        "deepseek-harness",
        files,
        "unknown",
        extra={
            "reconciliation_state": "indeterminate",
            "capability_negotiation": {
                "required_capability": "instructions",
                "status": "unknown",
                "unsupported": [],
                "unknown": ["model-visible-complete-prompt"],
                "dropped": [],
            },
        },
    )


def _projection_for_declared_family(family_id: str) -> dict[str, Any]:
    files = family_native_content(family_id)
    meta = _surface_for(family_id)
    family = family_by_id(family_id)
    surface = next(item for item in family["surfaces"] if item["id"] == meta["surface"])
    status = capability_status(family_id, meta["surface"], "instructions", surface["static_support"])
    extra: dict[str, Any] = {}
    outcome = "native-equivalent"
    if family_id in CONNECTOR_FAMILIES or status != "required-supported":
        outcome = "unsupported" if family_id in CONNECTOR_FAMILIES else "unknown"
        extra["reconciliation_state"] = "structural-only" if family_id in CONNECTOR_FAMILIES else "indeterminate"
        extra["capability_negotiation"] = {
            "required_capability": "instructions",
            "status": status if status != "required-supported" else "not-applicable",
            "unsupported": ["instructions"] if family_id in CONNECTOR_FAMILIES else [],
            "unknown": [] if family_id in CONNECTOR_FAMILIES else ["model-visible-complete-prompt"],
            "dropped": [],
        }
    return make_projection(family_id, files, outcome, extra=extra or None)


def all_declared_projections() -> list[dict[str, Any]]:
    rows = four_projections()
    seen = {item["family_id"] for item in rows}
    rows.append(honest_unknown_projection())
    seen.add("deepseek-harness")
    for family_id in family_ids():
        if family_id in seen:
            continue
        rows.append(_projection_for_declared_family(family_id))
    return rows


def build_st2_pos() -> dict[str, Any]:
    projections = all_declared_projections()
    return finalize_fixture(
        "ST2-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            equivalence_bindings=equivalence_ok(projections[:4]),
            loss_reports=loss_harness_only()
            + [
                {
                    "intent_id": INTENT_ID,
                    "family_id": "deepseek-harness",
                    "entries": [
                        {
                            "class": "omitted",
                            "detail": "No declared repeatable oracle; projection remains unknown/indeterminate and is not verified",
                        }
                    ],
                }
            ],
            receipts=[bound_receipt([item for item in projections if item["family_id"] in REQUIRED_FOUR])],
        ),
        [],
        "Intent projected differently but equivalently to Codex, Claude Code, Cursor, and grok-build. Honest unknown+indeterminate is accepted and not verified.",
    )


def build_st2_neg() -> dict[str, Any]:
    projections = four_projections()
    dropped = make_projection(
        "codex",
        four_native_files()["codex"],
        "verified",
        extra={
            "capability_negotiation": {
                "required_capability": "rules",
                "status": "not-applicable",
                "unsupported": ["scoped-conditional-rules"],
                "unknown": ["model-visible-complete-prompt"],
                "dropped": ["scoped-conditional-rules", "model-visible-complete-prompt"],
            },
            "reconciliation_state": "verified",
        },
    )
    projections[0] = dropped
    binding = {
        "intent_id": INTENT_ID,
        "family_ids": REQUIRED_FOUR,
        "equivalent": True,
        "byte_identical": False,
        "equivalence_basis": "canonical-intent-semantics",
        "text_hash_equality_is_not_semantic_equivalence": True,
        "native_digests": [item["native_digest"] for item in projections],
    }
    return finalize_fixture(
        "ST2-neg",
        "negative",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            equivalence_bindings=[binding],
            receipts=[{"id": "receipt.false-verified", "reconciliation_state": "verified", "native_result_captured": False, "bound_digests": {}}],
        ),
        [
            "projection-outcome-is-reconciliation-state",
            "silent-capability-loss",
            "unsupported-marked-verified",
            "unknown-marked-verified",
            "silent-drop-unsupported",
            "silent-drop-unknown",
        ],
        "Silent capability loss / dropping unsupported or unknown and reporting verified must fail.",
    )


def build_st2_same_bytes_pos() -> dict[str, Any]:
    projections = [
        make_projection("kimi-code", family_native_content("kimi-code"), "native-equivalent"),
        make_projection("zcode", family_native_content("zcode"), "native-equivalent"),
    ]
    binding = {
        "intent_id": INTENT_ID,
        "family_ids": ["kimi-code", "zcode"],
        "equivalent": True,
        "byte_identical": False,
        "equivalence_basis": "canonical-intent-semantics",
        "text_hash_equality_is_not_semantic_equivalence": True,
        "native_digests": [item["native_digest"] for item in projections],
    }
    return finalize_fixture(
        "ST2-same-bytes-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            equivalence_bindings=[binding],
            receipts=[bound_receipt(projections)],
        ),
        [],
        "C-F02 minimal counterexample: kimi-code and zcode share the AGENTS.md path glob and "
        "project byte-identical bodies. Byte identity is not evidence for or against semantic "
        "equivalence; the binding rests on canonical-intent-semantics, and byte_identical=False "
        "records that byte identity is not asserted as the equivalence evidence.",
    )


def build_st3_pos() -> dict[str, Any]:
    projections = four_projections()
    overlay = overlay_ok()
    exc = covering_exception_for(overlay)
    return finalize_fixture(
        "ST3-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            overlays=[overlay],
            loss_reports=loss_harness_only(),
            equivalence_bindings=equivalence_ok(projections),
            receipts=[bound_receipt(projections, exc["id"])],
            exceptions=[exc],
        ),
        [],
        "Justified native overlay with owner/reason/scope/separate digest; round-trip digests bound.",
    )


def build_st3_neg() -> dict[str, Any]:
    projections = four_projections()
    overlay = overlay_ok()
    overlay["weakens_required"] = True
    overlay["reason"] = "Drop the Vitest requirement on Cursor without an exception"
    overlay["scope"] = {"family_id": "cursor", "surface": "ide", "path": ".cursor/rules/testing.mdc"}
    overlay["projection_outcome"] = "lossy"
    overlay["digest"] = _digest_obj({k: v for k, v in overlay.items() if k != "digest"})
    return finalize_fixture(
        "ST3-neg",
        "negative",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            overlays=[overlay],
            loss_reports=[
                {
                    "intent_id": INTENT_ID,
                    "family_id": "cursor",
                    "entries": [{"class": "weakened", "detail": "Required Vitest rule removed by unapproved overlay"}],
                }
            ],
            equivalence_bindings=equivalence_ok(projections),
        ),
        ["overlay-weakens-required-without-exception"],
        "Unapproved overlay that weakens a required team rule must fail.",
    )


def build_st4_pos() -> dict[str, Any]:
    return finalize_fixture(
        "ST4-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            team_standard=valid_standard(),
            projections=four_projections(),
        ),
        [],
        "Signed versioned TeamContextStandard with required fields, no secrets, git/file local-first.",
    )


def _st4_unsigned() -> dict[str, Any]:
    return with_payload(canonical_intents=[vitest_intent()], team_standard=standard_unsigned())


def _st4_secret() -> dict[str, Any]:
    body = valid_standard()
    body["api_token"] = "synthetic-forbidden-secret-field"
    return with_payload(canonical_intents=[vitest_intent()], team_standard=body)


def _st4_rollback_unsafe() -> dict[str, Any]:
    body = valid_standard()
    body["rollback"] = {
        "safe": False,
        "preserves_unrelated_personal_files": False,
        "last_known_good_revision": 0,
        "transaction": {
            "authorized": False,
            "target_revision": 0,
            "target_digest": lkg_digest(),
            "target_is_last_known_good": True,
            "audit_digest": sha256_text("none"),
            "receipt_digest": sha256_text("none"),
        },
    }
    return with_payload(canonical_intents=[vitest_intent()], team_standard=body)


def _st4_member_publisher() -> dict[str, Any]:
    return with_payload(canonical_intents=[vitest_intent()], team_standard=sign_standard(standard_unsigned(), "syn-member"))


def _st4_incomplete_publisher() -> dict[str, Any]:
    body = valid_standard()
    body["publisher"] = {"id": "syn-publisher-team-lead", "display_name": "Synthetic Team Lead"}
    return with_payload(canonical_intents=[vitest_intent()], team_standard=body)


def build_st4_neg() -> dict[str, Any]:
    unsigned = _st4_unsigned()
    secret = _st4_secret()
    unsafe = _st4_rollback_unsafe()
    member = _st4_member_publisher()
    incomplete = _st4_incomplete_publisher()
    unsigned["expected_violations"] = ["unsigned-standard", "missing-signature"]
    secret["expected_violations"] = ["secret-in-standard"]
    unsafe["expected_violations"] = ["rollback-unsafe-standard"]
    member["expected_violations"] = ["member-key-as-publisher"]
    incomplete["expected_violations"] = ["incomplete-publisher"]
    payload = with_payload(variants=[unsigned, secret, unsafe, member, incomplete])
    return finalize_fixture(
        "ST4-neg",
        "negative",
        payload,
        [
            "unsigned-standard",
            "missing-signature",
            "secret-in-standard",
            "rollback-unsafe-standard",
            "member-key-as-publisher",
            "incomplete-publisher",
        ],
        "Unsigned, secret-bearing, rollback-unsafe, member-signed, or incomplete-publisher standard must fail.",
    )


def build_st5_pos() -> dict[str, Any]:
    projections = four_projections()
    return finalize_fixture(
        "ST5-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            team_standard=valid_standard(),
            layer_assignments=layer_assignments_ok(),
            effective_policies=effective_policies_ok(),
            member_disclosure=member_disclosure(),
        ),
        [],
        "Layered enforcement; member effective state per harness via the same pipeline; detect-only when unmanaged.",
    )


def build_st5_neg() -> dict[str, Any]:
    layers = layer_assignments_ok()
    layers[-1] = {
        "layer": "personal",
        "mode": "recommended",
        "rule_id": POLICY_ID,
        "honesty": "non-enforceable",
    }
    shadow = with_payload(
        canonical_intents=[vitest_intent()],
        projections=four_projections(),
        layer_assignments=layers,
        effective_policies=effective_policies_ok(),
    )
    shadow["expected_violations"] = ["personal-shadows-required", "required-downgraded-to-recommended"]
    downgrade_layers = layer_assignments_ok()
    downgrade_layers[2] = {
        "layer": "project",
        "mode": "recommended",
        "rule_id": POLICY_ID,
        "honesty": "enforceable",
    }
    downgrade = with_payload(
        canonical_intents=[vitest_intent()],
        projections=four_projections(),
        layer_assignments=downgrade_layers,
        effective_policies=effective_policies_ok(),
    )
    downgrade["expected_violations"] = ["required-downgraded-to-recommended"]
    unmanaged = four_projections()
    for proj in unmanaged:
        if proj["family_id"] == "cursor":
            proj["managed_channel"] = False
    false_enf = effective_policies_ok()
    for row in false_enf:
        if row["family_id"] == "cursor":
            row["honesty"] = "enforceable"
    false_claim = with_payload(
        canonical_intents=[vitest_intent()],
        projections=unmanaged,
        layer_assignments=layer_assignments_ok(),
        effective_policies=false_enf,
    )
    false_claim["expected_violations"] = ["unmanaged-marked-enforceable", "false-enforcement-claim"]
    return finalize_fixture(
        "ST5-neg",
        "negative",
        with_payload(variants=[shadow, downgrade, false_claim]),
        [
            "personal-shadows-required",
            "required-downgraded-to-recommended",
            "unmanaged-marked-enforceable",
            "false-enforcement-claim",
        ],
        "Personal/project weakening of required rules, and unmanaged channels claimed enforceable, must fail.",
    )


def build_st6_pos() -> dict[str, Any]:
    exc = valid_exception()
    return finalize_fixture(
        "ST6-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=four_projections(),
            exceptions=[exc],
            policy_evals=[
                {
                    "policy_id": POLICY_ID,
                    "mode": "required",
                    "result": "pass",
                    "covered_by_exception": exc["id"],
                    "inspection_export_allowed": True,
                }
            ],
            receipts=[bound_receipt(four_projections(), exc["id"])],
        ),
        [],
        "Scoped expiring exception with requester/approver/signature/audit.",
    )


def _exception_variant(exc: dict[str, Any], eval_row: dict[str, Any], expected: list[str]) -> dict[str, Any]:
    payload = with_payload(canonical_intents=[vitest_intent()], exceptions=[exc], policy_evals=[eval_row])
    payload["expected_violations"] = expected
    return payload


def build_st6_neg() -> dict[str, Any]:
    unbounded = valid_exception()
    unbounded["use_limit"] = None
    unbounded["time_limit"] = {"from": None, "until": None}
    unbounded["scope"] = {"member": "*", "device": "*", "project": "*", "harness": "*"}
    expired = valid_exception("expiry")
    expired["time_limit"] = {"from": FROZEN_PAST, "until": FROZEN_PAST}
    expired["freshness"] = "expired"
    revoked = valid_exception("revocation")
    revoked["freshness"] = "revoked"
    stale = valid_exception("approval")
    stale["freshness"] = "stale-offline"
    stale_pass = {
        "policy_id": POLICY_ID,
        "mode": "required",
        "result": "pass",
        "covered_by_exception": stale["id"],
        "inspection_export_allowed": True,
    }
    variants = [
        _exception_variant(
            unbounded,
            {"policy_id": POLICY_ID, "mode": "required", "result": "pass", "covered_by_exception": unbounded["id"], "inspection_export_allowed": True},
            ["unbounded-exception"],
        ),
        _exception_variant(
            expired,
            {"policy_id": POLICY_ID, "mode": "required", "result": "pass", "covered_by_exception": expired["id"], "inspection_export_allowed": True},
            ["expired-exception-pass"],
        ),
        _exception_variant(
            revoked,
            {"policy_id": POLICY_ID, "mode": "required", "result": "pass", "covered_by_exception": revoked["id"], "inspection_export_allowed": True},
            ["revoked-exception-pass"],
        ),
        _exception_variant(stale, stale_pass, ["stale-offline-exception-pass"]),
    ]
    return finalize_fixture(
        "ST6-neg",
        "negative",
        with_payload(variants=variants),
        [
            "unbounded-exception",
            "expired-exception-pass",
            "revoked-exception-pass",
            "stale-offline-exception-pass",
        ],
        "Unbounded or expired/stale/revoked exceptions cannot pass required policy. Offline stale fails closed while inspection/export remains allowed.",
    )


def build_st7_pos() -> dict[str, Any]:
    view = leader_view_redacted()
    return finalize_fixture(
        "ST7-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            team_standard=valid_standard(),
            member_disclosure=member_disclosure(view),
            leader_view=view,
            loss_reports=loss_harness_only(),
        ),
        [],
        "Privacy-redacted leader report with allowed metadata only.",
    )


def build_st7_neg() -> dict[str, Any]:
    view = leader_view_redacted()
    view["private_prompt_text"] = "user private prompt body"
    view["secrets"] = {"api_token": "synthetic-forbidden-secret-field"}
    view["unrelated_personal_context"] = "personal notes unrelated to the standard"
    view["full_session_history"] = [{"turn": 1, "text": "full session"}]
    nested = dict(view.get("exception_metadata") or {})
    nested["private_prompt_text"] = "nested private prompt in exception metadata"
    view["exception_metadata"] = nested
    return finalize_fixture(
        "ST7-neg",
        "negative",
        with_payload(
            canonical_intents=[vitest_intent()],
            member_disclosure={
                "shown_before_report": False,
                "previewed_at": None,
                "previewed_fields": [],
                "upload_consented": True,
                "disclosed_digest": "",
                "acknowledgement": {},
            },
            leader_view=view,
        ),
        [
            "leader-private-prompt",
            "leader-secrets",
            "leader-personal-context",
            "leader-session-history",
            "disclosure-missing",
            "nested-private-content",
        ],
        "Leader report containing private prompt, secrets, unrelated personal context, or full session history must fail.",
    )


def complete_key_rotation(**overrides: Any) -> dict[str, Any]:
    core = {
        "old_key_id": "syn-publisher-team-lead",
        "new_key_id": "syn-publisher-team-lead-rotated",
        "old_trust_state": "revoked",
        "new_trust_state": "trusted",
        "rotated_at": FROZEN_TS,
        "applied": True,
        "activated_at": FROZEN_TS,
        "revoked_at": FROZEN_TS,
        "trust_store_result": "new-key-trusted-old-key-revoked",
        "dual_authorization": ["syn-publisher-team-lead", "syn-publisher-team-lead-rotated"],
    }
    core.update(overrides)
    unsigned = {key: value for key, value in core.items() if key not in {"old_signature", "new_signature"}}
    core["old_signature"] = synthetic_sign(unsigned, core["old_key_id"])
    core["new_signature"] = synthetic_sign(unsigned, core["new_key_id"])
    return core


def lifecycle_ok(projections: list[dict[str, Any]]) -> dict[str, Any]:
    update_digest = expected_update_digest()
    return {
        "steps": [
            {"command": "standard publish", "actor": "leader", "state": "published", "preview": True, "disclosure": True},
            {"command": "standard preview", "actor": "member", "disclosure": True, "preview": True},
            {"command": "standard adopt", "actor": "member", "pin": True, "preview": True, "disclosure": True},
            {"command": "intent project", "actor": "member", "per_harness": True, "preview": True, "disclosure": True},
            {"command": "align status", "actor": "leader", "drift_class": "benign-native-representation", "preview": True, "disclosure": True},
        ],
        "staged_rollout": {
            "cohort": "canary",
            "percent": 10,
            "order": ["canary", "ga"],
            "stages": [
                {"name": "canary", "order": 1, "percent": 10, "approval_required": True, "health_rollback": True},
                {"name": "ga", "order": 2, "percent": 100, "approval_required": True, "health_rollback": True},
            ],
            "approval": "syn-approver-lead",
            "health_rollback": True,
        },
        "conflict_resolution": {
            "mode": "explicit-member-choice",
            "silent_merge": False,
            "inputs": ["rev-1", "rev-1-fork"],
            "winner": "rev-1",
            "reason": "explicit-member-choice-on-deterministic-heads",
        },
        "last_known_good": {
            "revision": 0,
            "recoverable": True,
            "identity": STANDARD_ID,
            "digest": lkg_digest(),
        },
        "key_rotation": complete_key_rotation(),
        "rollback": {
            "target_revision": 0,
            "target_digest": lkg_digest(),
            "authorized": True,
            "preserves_unrelated_personal_files": True,
            "authorization": "publish-team-standard",
        },
        "update": {
            "digest": update_digest,
            "preview": True,
            "disclosure": True,
            "apply_at": REPORT_AT,
            "preview_digest": update_digest,
        },
        "traceable_artifacts": [
            _digest_obj(vitest_intent()),
            _digest_obj(valid_standard()),
            native_bundle_digest(projections[0]["native_files"]),
        ],
    }


def build_st8_pos() -> dict[str, Any]:
    projections = four_projections()
    view = leader_view_redacted()
    rollback = {
        "preserves_unrelated_personal_files": True,
        "deletes": ["managed/.ctxpect/standard-binding.json"],
        "retains": ["personal/unrelated-notes.md"],
    }
    return finalize_fixture(
        "ST8-pos",
        "positive",
        with_payload(
            canonical_intents=[vitest_intent()],
            projections=projections,
            team_standard=valid_standard(),
            member_disclosure=member_disclosure(view),
            leader_view=view,
            lifecycle=lifecycle_ok(projections),
            rollback_plan=rollback,
            receipts=[bound_receipt(projections)],
            equivalence_bindings=equivalence_ok(projections),
        ),
        [],
        "Leader publish → member preview/disclosure → adopt/pin → per-harness projection → compliance/drift.",
    )


def build_st8_neg() -> dict[str, Any]:
    docs_only = with_payload(documentation_only=True, claimed_alignment={"method": "readme-prose", "treated_as_semantic_equivalence": True})
    docs_only["expected_violations"] = ["documentation-only-claim"]
    fork = with_payload(
        team_standard=valid_standard(),
        trust_state={"heads": ["rev-1", "rev-1-fork"], "silent_merge": True, "equivalent": True},
        lifecycle={"conflict_resolution": {"mode": "silent-merge", "silent_merge": True}, "steps": [], "traceable_artifacts": []},
    )
    fork["expected_violations"] = ["fork-conflict-silent-merge"]
    rollback_bad = with_payload(
        rollback_plan={
            "preserves_unrelated_personal_files": False,
            "deletes": ["personal/unrelated-notes.md"],
            "retains": [],
        }
    )
    rollback_bad["expected_violations"] = ["rollback-deletes-personal"]
    revoked_key = with_payload(
        team_standard=valid_standard(),
        trust_state={"publisher_key_revoked": True, "still_trusted": True},
    )
    revoked_key["expected_violations"] = ["revoked-key-still-trusted"]
    upgrade = with_payload(
        canonical_intents=[vitest_intent()],
        team_standard=valid_standard(),
        projections=four_projections(),
        documentation_only=False,
        lifecycle={
            "steps": [{"command": "standard update", "preview": False, "pin": True, "disclosure": False}],
            "traceable_artifacts": [_digest_obj(vitest_intent())],
            "last_known_good": {"revision": None, "recoverable": False},
        },
        member_disclosure={
            "shown_before_report": False,
            "previewed_at": None,
            "previewed_fields": [],
            "upload_consented": True,
            "disclosed_digest": "",
            "acknowledgement": {},
        },
    )
    upgrade["expected_violations"] = ["update-without-preview", "update-without-disclosure", "disclosure-missing"]
    rotation = with_payload(
        canonical_intents=[vitest_intent()],
        team_standard=valid_standard(),
        lifecycle={
            "steps": [{"command": "standard rotate-key", "preview": True, "disclosure": True}],
            "traceable_artifacts": [_digest_obj(vitest_intent())],
            "key_rotation": {
                "old_key_id": "syn-publisher-team-lead",
                "new_key_id": "syn-member",
                "old_trust_state": "trusted",
                "new_trust_state": "trusted",
                "rotated_at": FROZEN_TS,
                "applied": True,
                "signature": {"key_id": "syn-member", "signature": "0" * 64},
            },
        },
        member_disclosure=member_disclosure(),
    )
    rotation["expected_violations"] = ["invalid-key-rotation", "member-key-as-publisher"]
    return finalize_fixture(
        "ST8-neg",
        "negative",
        with_payload(variants=[docs_only, upgrade, fork, rollback_bad, revoked_key, rotation]),
        [
            "documentation-only-claim",
            "fork-conflict-silent-merge",
            "rollback-deletes-personal",
            "revoked-key-still-trusted",
            "update-without-preview",
            "update-without-disclosure",
            "invalid-key-rotation",
        ],
        "Documentation-only claim, update without preview/disclosure, silent fork merge, rollback that deletes personal files, revoked key still trusted, or invalid key rotation must fail.",
    )


def build_malformed_schema() -> dict[str, Any]:
    intent = vitest_intent()
    intent["unexpected_field"] = "not-in-schema"
    payload = with_payload(canonical_intents=[intent])
    return finalize_fixture(
        "malformed-schema",
        "negative",
        payload,
        ["malformed-schema"],
        "CanonicalIntent with undeclared keys must fail.",
    )


def build_missing_signature() -> dict[str, Any]:
    return finalize_fixture(
        "missing-signature",
        "negative",
        with_payload(canonical_intents=[vitest_intent()], team_standard=standard_unsigned()),
        ["unsigned-standard", "missing-signature"],
        "TeamContextStandard without signatures must fail.",
    )


def build_swapped_digest() -> dict[str, Any]:
    body = valid_standard()
    body["content_digest_manifest"] = [
        {"path": f"canonical_intent_set/{INTENT_ID}", "digest_sha256": "0" * 64},
        {"path": f"policy_rules/{POLICY_ID}", "digest_sha256": "0" * 64},
    ]
    return finalize_fixture(
        "swapped-digest",
        "negative",
        with_payload(canonical_intents=[vitest_intent()], team_standard=body),
        ["swapped-digest"],
        "Swapped content digest in a signed standard must fail.",
    )


def build_truncated_bundle() -> dict[str, Any]:
    body = valid_standard()
    del body["signatures"]
    del body["expiry"]
    del body["channel"]
    return finalize_fixture(
        "truncated-bundle",
        "negative",
        with_payload(canonical_intents=[vitest_intent()], team_standard=body),
        ["truncated-bundle"],
        "Truncated TeamContextStandard bundle must fail.",
    )


def build_forged_publisher() -> dict[str, Any]:
    body = valid_standard()
    body["signatures"] = [
        {
            "key_id": "syn-publisher-team-lead",
            "alg": "hmac-sha256-synthetic",
            "signature": "0" * 64,
            "live_tested": False,
            "note": "offline synthetic fixture key; not a user token",
        }
    ]
    return finalize_fixture(
        "forged-publisher",
        "negative",
        with_payload(canonical_intents=[vitest_intent()], team_standard=body),
        ["forged-publisher"],
        "Forged publisher signature must fail.",
    )


def all_fixtures() -> list[dict[str, Any]]:
    return [
        build_st1_pos(),
        build_st1_neg(),
        build_st2_pos(),
        build_st2_neg(),
        build_st2_same_bytes_pos(),
        build_st3_pos(),
        build_st3_neg(),
        build_st4_pos(),
        build_st4_neg(),
        build_st5_pos(),
        build_st5_neg(),
        build_st6_pos(),
        build_st6_neg(),
        build_st7_pos(),
        build_st7_neg(),
        build_st8_pos(),
        build_st8_neg(),
        build_malformed_schema(),
        build_missing_signature(),
        build_swapped_digest(),
        build_truncated_bundle(),
        build_forged_publisher(),
    ]


ROLE_AUTHORIZATION_VALUES = {
    "publish-team-standard",
    "approve-exception",
    "request-exception",
}


def _walk_secret_keys(obj: Any, found: list[str]) -> None:
    if isinstance(obj, dict):
        for key, value in obj.items():
            if key in SECRET_KEY_NAMES:
                if key == "authorization" and value in ROLE_AUTHORIZATION_VALUES:
                    continue
                found.append(key)
            _walk_secret_keys(value, found)
    elif isinstance(obj, list):
        for item in obj:
            _walk_secret_keys(item, found)


def _native_fingerprint(projection: dict[str, Any]) -> str:
    files = projection.get("native_files") or []
    return _digest_obj([{"path": item.get("path"), "content": item.get("content")} for item in files])


def _empty_collection(value: Any) -> bool:
    if value is None:
        return True
    if isinstance(value, (list, dict, tuple, set)) and len(value) == 0:
        return True
    return False


def _cardinality_codes(field: str) -> list[str]:
    extra = {
        "canonical_intents": ["canonical-intent-empty-collection", "canonical-intent-type"],
        "overlays": ["overlay-metadata-missing"],
        "exceptions": ["incomplete-exception"],
        "receipts": ["empty-round-trip-bindings", "missing-round-trip-digest"],
        "layer_assignments": ["required-scenario-coverage"],
        "effective_policies": ["required-scenario-coverage"],
        "lifecycle": ["missing-lifecycle-group"],
        "team_standard": ["truncated-bundle"],
        "member_disclosure": ["disclosure-missing"],
        "leader_view": ["disclosure-missing"],
    }
    return extra.get(field, [])


def _validate_required_graph(payload: dict[str, Any], scenario: str | None, found: list[str]) -> None:
    spec = SCENARIO_GRAPH.get(scenario or "") or {}
    for field, count in (spec.get("lists") or {}).items():
        value = payload.get(field)
        if not isinstance(value, list) or len(value) != count:
            _flag(found, "required-collection-cardinality")
            for code in _cardinality_codes(field):
                _flag(found, code)
    for field, required in (spec.get("objects") or {}).items():
        if not required:
            continue
        value = payload.get(field)
        if not isinstance(value, dict) or _empty_collection(value):
            _flag(found, "required-collection-cardinality")
            for code in _cardinality_codes(field):
                _flag(found, code)


def _validate_intent(intent: Any, found: list[str], intent_ids: set[str]) -> None:
    if not isinstance(intent, dict):
        _flag(found, "canonical-intent-type")
        _flag(found, "malformed-schema")
        return
    extra = set(intent) - set(CANONICAL_INTENT_FIELDS)
    missing = set(CANONICAL_INTENT_FIELDS) - set(intent)
    if extra or missing:
        _flag(found, "malformed-schema")
    ident = intent.get("id")
    if not _is_nonempty_str(ident) or not str(ident).startswith(INTENT_ID_PREFIX):
        _flag(found, "canonical-intent-type")
    else:
        intent_ids.add(str(ident))
    nested_objects = {
        "meaning": MEANING_FIELDS,
        "scope": SCOPE_FIELDS,
        "precedence": PRECEDENCE_FIELDS,
        "activation": ACTIVATION_FIELDS,
        "authority": AUTHORITY_FIELDS,
        "provenance": PROVENANCE_FIELDS,
    }
    for key, fields in nested_objects.items():
        value = intent.get(key)
        if not isinstance(value, dict):
            _flag(found, "canonical-intent-type")
            continue
        if set(value) != set(fields):
            _flag(found, "malformed-schema")
        if _empty_collection(value):
            _flag(found, "canonical-intent-type")
            _flag(found, "canonical-intent-empty-collection")
    meaning = intent.get("meaning") if isinstance(intent.get("meaning"), dict) else {}
    if not _is_nonempty_str(meaning.get("goal")):
        _flag(found, "canonical-intent-type")
    for list_key in ("constraints", "non_goals"):
        value = meaning.get(list_key)
        if not isinstance(value, list) or _empty_collection(value) or not all(_is_nonempty_str(item) for item in value):
            _flag(found, "canonical-intent-type")
            if isinstance(value, list) and _empty_collection(value):
                _flag(found, "canonical-intent-empty-collection")
    scope = intent.get("scope") if isinstance(intent.get("scope"), dict) else {}
    if scope.get("kind") not in SCOPE_KINDS or not _is_nonempty_str(scope.get("target")):
        _flag(found, "canonical-intent-type")
    paths = scope.get("paths")
    if not isinstance(paths, list) or _empty_collection(paths) or not all(_is_nonempty_str(item) for item in paths):
        _flag(found, "canonical-intent-type")
        if isinstance(paths, list) and _empty_collection(paths):
            _flag(found, "canonical-intent-empty-collection")
    precedence = intent.get("precedence") if isinstance(intent.get("precedence"), dict) else {}
    if precedence.get("rank") not in LAYER_ORDER or precedence.get("policy") not in PRECEDENCE_POLICIES:
        _flag(found, "canonical-intent-type")
    if not isinstance(precedence.get("shadows_lower"), bool):
        _flag(found, "canonical-intent-type")
    activation = intent.get("activation") if isinstance(intent.get("activation"), dict) else {}
    if not _is_nonempty_str(activation.get("trigger")) or activation.get("mode") not in ACTIVATION_MODES:
        _flag(found, "canonical-intent-type")
    if intent.get("required_capability") not in CAPABILITY_IDS:
        _flag(found, "canonical-intent-type")
    dependencies = intent.get("dependencies")
    if not isinstance(dependencies, list) or any(not _is_nonempty_str(item) for item in dependencies):
        _flag(found, "canonical-intent-type")
    if not _is_nonempty_str(intent.get("permission_security_boundary")):
        _flag(found, "canonical-intent-type")
    if intent.get("lifecycle") not in INTENT_LIFECYCLES:
        _flag(found, "canonical-intent-type")
    if not _is_nonempty_str(intent.get("owner")):
        _flag(found, "canonical-intent-type")
    if not _is_nonempty_str(intent.get("desired_outcome")):
        _flag(found, "canonical-intent-type")
    authority = intent.get("authority") if isinstance(intent.get("authority"), dict) else {}
    if not _is_nonempty_str(authority.get("id")) or authority.get("kind") not in AUTHORITY_KINDS:
        _flag(found, "canonical-intent-type")
    provenance = intent.get("provenance") if isinstance(intent.get("provenance"), dict) else {}
    if not _is_nonempty_str(provenance.get("source")) or not _is_timestamp(provenance.get("recorded_at")):
        _flag(found, "canonical-intent-type")
    if provenance.get("digest_sha256") != intent_provenance_digest(intent):
        _flag(found, "canonical-intent-type")
    for key in CANONICAL_INTENT_FIELDS:
        if key in intent and intent[key] is None:
            _flag(found, "canonical-intent-type")


def _validate_pipeline(pipeline: Any, found: list[str]) -> None:
    if not isinstance(pipeline, list) or not pipeline:
        _flag(found, "pipeline-missing")
        return
    steps: list[str] = []
    orders: list[Any] = []
    for index, item in enumerate(pipeline, start=1):
        if not _require_closed(item, PIPELINE_STAGE_FIELDS, found, "pipeline-missing", "pipeline-order"):
            continue
        steps.append(str(item.get("step")))
        orders.append(item.get("order"))
        if item.get("order") != index or item.get("status") != PIPELINE_STAGE_STATUS:
            _flag(found, "pipeline-order")
    if steps != PIPELINE_STEPS or len(steps) != len(set(steps)):
        _flag(found, "pipeline-order")
    if orders != list(range(1, len(PIPELINE_STEPS) + 1)):
        _flag(found, "pipeline-order")


def _declared_oracle(family_id: Any) -> bool:
    return family_id in DECLARED_ORACLE_COMMANDS


def _validate_oracle(oracle: Any, family_id: Any, found: list[str]) -> None:
    if not isinstance(oracle, dict):
        _flag(found, "malformed-schema")
        _flag(found, "oracle-grammar")
        return
    if not _require_closed(oracle, ORACLE_FIELDS, found, "oracle-grammar", "malformed-schema"):
        return
    kind = oracle.get("kind")
    if kind not in ORACLE_KINDS:
        _flag(found, "oracle-grammar")
        _flag(found, "malformed-schema")
    command = oracle.get("command")
    expected_cmd = DECLARED_ORACLE_COMMANDS.get(str(family_id))
    if kind == "declared-repeatable":
        if expected_cmd is None:
            _flag(found, "undeclared-repeatable-oracle")
            _flag(found, "oracle-grammar")
        elif command != expected_cmd:
            _flag(found, "oracle-grammar")
            _flag(found, "undeclared-repeatable-oracle")
        if oracle.get("native_result_captured") is True:
            _flag(found, "oracle-grammar")
        if oracle.get("live_tested") is not False:
            _flag(found, "oracle-grammar")
    elif expected_cmd is not None and command not in (None, expected_cmd):
        _flag(found, "oracle-grammar")


def _validate_projection(
    proj: Any,
    found: list[str],
    targets: dict[str, dict[str, Any]],
    declared_capabilities: set[str] | None = None,
) -> None:
    if not isinstance(proj, dict):
        _flag(found, "malformed-schema")
        return
    extra = set(proj) - set(PROJECTION_ALLOWED_FIELDS)
    missing = set(PROJECTION_REQUIRED_FIELDS) - set(proj)
    if extra or missing:
        if missing & {"scope", "precedence", "lifecycle", "primitives"}:
            _flag(found, "family-native-incomplete")
        else:
            _flag(found, "malformed-schema")
    family_id = proj.get("family_id")
    target = targets.get(family_id)
    if not _is_nonempty_str(family_id) or target is None:
        _flag(found, "family-native-incomplete")
        _validate_pipeline(proj.get("pipeline"), found)
        return
    meta = _surface_for(str(family_id))
    if (
        proj.get("coordinate") != meta["coordinate"]
        or proj.get("surface") != meta["surface"]
        or proj.get("os_lane") != meta["os_lane"]
        or proj.get("os_lane") not in {lane["id"] for lane in OS_LANES}
    ):
        _flag(found, "family-native-incomplete")
        _flag(found, "family-native-mismatch")
    if proj.get("authority") != AUTHORITY:
        _flag(found, "family-native-incomplete")
        _flag(found, "family-native-mismatch")
    for key in ("scope", "precedence", "lifecycle"):
        if not _is_nonempty_str(proj.get(key)):
            _flag(found, "family-native-incomplete")
        elif proj.get(key) != target.get(key):
            _flag(found, "family-native-incomplete")
            _flag(found, "family-native-mismatch")
    primitives = proj.get("primitives")
    if not isinstance(primitives, list) or not primitives or not all(_is_nonempty_str(item) for item in primitives):
        _flag(found, "family-native-incomplete")
    elif list(primitives) != list(target.get("primitives") or []):
        _flag(found, "family-native-incomplete")
        _flag(found, "family-native-mismatch")
    if not isinstance(proj.get("managed_channel"), bool):
        _flag(found, "family-native-incomplete")
    elif proj.get("managed_channel") != target.get("managed_channel"):
        _flag(found, "family-native-incomplete")
        _flag(found, "family-native-mismatch")
    if not _is_nonempty_str(proj.get("native_path")) or not _is_nonempty_str(proj.get("syntax")):
        _flag(found, "family-native-incomplete")
    else:
        if proj.get("syntax") != target.get("syntax"):
            _flag(found, "family-native-incomplete")
            _flag(found, "family-native-mismatch")
        if not path_allowed_for_family(proj.get("native_path"), family_id):
            _flag(found, "family-native-incomplete")
            _flag(found, "family-native-mismatch")
        elif proj.get("native_path") not in family_required_paths(family_id):
            _flag(found, "family-native-incomplete")
            _flag(found, "family-native-mismatch")
    files = proj.get("native_files")
    if not isinstance(files, list) or not files:
        _flag(found, "malformed-schema")
        _flag(found, "family-native-incomplete")
    else:
        authoritative = {item["path"]: item for item in family_native_content(family_id)}
        ready = []
        seen_paths: list[str] = []
        for item in files:
            if not _require_closed(item, NATIVE_FILE_FIELDS, found, "malformed-schema"):
                _flag(found, "family-native-incomplete")
                continue
            path = item.get("path")
            seen_paths.append(str(path))
            if not path_allowed_for_family(path, family_id):
                _flag(found, "family-native-incomplete")
                _flag(found, "family-native-mismatch")
            if item.get("digest_sha256") != sha256_text(str(item.get("content") or "")):
                _flag(found, "family-native-incomplete")
            expected_file = authoritative.get(str(path))
            if expected_file is None or item.get("content") != expected_file.get("content"):
                # A deleted rule file, a foreign body, or a cross-family swap.
                _flag(found, "family-native-incomplete")
                _flag(found, "family-native-mismatch")
            ready.append(item)
        if set(seen_paths) != set(authoritative) or len(seen_paths) != len(set(seen_paths)):
            _flag(found, "family-native-incomplete")
            _flag(found, "family-native-mismatch")
        if ready and native_bundle_digest(ready) != proj.get("native_digest"):
            _flag(found, "family-native-incomplete")
        if native_bundle_digest(list(authoritative.values())) != proj.get("native_digest"):
            _flag(found, "family-native-incomplete")
            _flag(found, "family-native-mismatch")
    outcome = proj.get("projection_outcome")
    if outcome in CLAIM_RECONCILIATION_STATES or outcome == "verified":
        _flag(found, "projection-outcome-is-reconciliation-state")
    if outcome not in PROJECTION_OUTCOMES and outcome not in CLAIM_RECONCILIATION_STATES:
        _flag(found, "malformed-schema")
    basis = proj.get("equivalence_basis")
    if basis in FORBIDDEN_EQUIVALENCE_BASES:
        _flag(found, "hash-equality-as-equivalence")
        _flag(found, "byte-copy-as-alignment")
    elif basis not in EQUIVALENCE_BASES:
        # An undeclared basis cannot support an equivalence claim, whatever it is spelled.
        _flag(found, "malformed-schema")
        _flag(found, "byte-copy-as-alignment")
    negotiation = proj.get("capability_negotiation")
    if not isinstance(negotiation, dict):
        _flag(found, "malformed-schema")
        _flag(found, "silent-capability-loss")
        negotiation = {}
    else:
        extra_n = set(negotiation) - set(NEGOTIATION_FIELDS)
        missing_n = set(NEGOTIATION_FIELDS) - set(negotiation)
        if extra_n or missing_n:
            _flag(found, "malformed-schema")
        for key in ("unsupported", "unknown", "dropped"):
            value = negotiation.get(key)
            if not isinstance(value, list):
                _flag(found, "malformed-schema")
            else:
                allowed_tokens = set(CAPABILITY_IDS)
                if key == "unknown":
                    allowed_tokens |= set(UNKNOWN_CAPABILITY_TOKENS)
                if any(item not in allowed_tokens for item in value):
                    _flag(found, "malformed-schema")
                    _flag(found, "silent-capability-loss")
        required_capability = negotiation.get("required_capability")
        if required_capability not in CAPABILITY_IDS:
            _flag(found, "malformed-schema")
            _flag(found, "silent-capability-loss")
        elif declared_capabilities and required_capability not in declared_capabilities:
            # The harness does not get to pick which capability the intent needs:
            # negotiating a capability it happens to support would turn an honest
            # "cannot express this" into a false "required-supported".
            _flag(found, "silent-capability-loss")
            _flag(found, "family-native-mismatch")
        status = negotiation.get("status")
        if status not in NEGOTIATION_STATUSES:
            _flag(found, "malformed-schema")
            _flag(found, "silent-capability-loss")
        elif required_capability in CAPABILITY_IDS:
            registry_status = registry_capability_status(str(family_id), str(required_capability))
            if status not in negotiation_states_allowed(registry_status):
                # Claiming more (or a different kind of) support than the registry grants.
                _flag(found, "family-native-mismatch")
                _flag(found, "silent-capability-loss")
    dropped = negotiation.get("dropped") or []
    unsupported = negotiation.get("unsupported") or []
    unknown = negotiation.get("unknown") or []
    status = negotiation.get("status")
    if status in NEGOTIATION_STATUSES:
        if status == NEGOTIATION_SUPPORTED and (unsupported or unknown or dropped):
            _flag(found, "silent-capability-loss")
        if status in NEGOTIATION_UNKNOWN_STATES and (not unknown or unsupported):
            _flag(found, "silent-capability-loss")
            _flag(found, "unknown-marked-verified")
        if status in NEGOTIATION_UNSUPPORTED_STATES and not unsupported:
            _flag(found, "silent-capability-loss")
            _flag(found, "unsupported-marked-verified")
        if (
            status in NEGOTIATION_UNSUPPORTED_STATES
            and unsupported
            and negotiation.get("required_capability") not in unsupported
        ):
            # Declaring some other capability unsupported does not answer for the
            # one the intent actually requires.
            _flag(found, "silent-capability-loss")
            _flag(found, "unsupported-marked-verified")
    if dropped:
        _flag(found, "silent-capability-loss")
    if dropped and unsupported:
        _flag(found, "silent-drop-unsupported")
    if dropped and unknown:
        _flag(found, "silent-drop-unknown")
    recon = proj.get("reconciliation_state")
    if recon is not None and recon not in CLAIM_RECONCILIATION_STATES:
        _flag(found, "malformed-schema")
        _flag(found, "projection-outcome-is-reconciliation-state")
    # The negotiated status, the projection outcome and the reconciliation state
    # must tell the same story; an honest Unknown may not be dressed as support.
    if status in NEGOTIATION_STATUSES and outcome in PROJECTION_OUTCOMES:
        if status == NEGOTIATION_SUPPORTED and outcome in {"lossy", "unsupported", "unknown"}:
            _flag(found, "silent-capability-loss")
            _flag(found, "family-native-mismatch")
        if status in NEGOTIATION_UNKNOWN_STATES and (outcome != "unknown" or recon != "indeterminate"):
            _flag(found, "unknown-marked-verified")
            _flag(found, "silent-capability-loss")
        if status in NEGOTIATION_UNSUPPORTED_STATES and outcome != "unsupported":
            _flag(found, "unsupported-marked-verified")
            _flag(found, "silent-capability-loss")
    oracle = proj.get("oracle") if isinstance(proj.get("oracle"), dict) else {}
    captured = bool(oracle.get("native_result_captured"))
    if recon == "verified" or outcome == "verified":
        if not _declared_oracle(family_id) or not captured:
            _flag(found, "verified-without-oracle")
        if unsupported or dropped:
            _flag(found, "unsupported-marked-verified")
        if unknown or dropped or not captured:
            _flag(found, "unknown-marked-verified")
    if outcome in {"lossy", "unsupported", "unknown"} and recon == "verified":
        _flag(found, "unknown-marked-verified")
    _validate_pipeline(proj.get("pipeline"), found)
    _validate_oracle(proj.get("oracle"), family_id, found)


def _exception_currently_valid(exc: dict[str, Any], eval_time: datetime | None = None) -> bool:
    eval_time = eval_time or _parse_ts(EVALUATION_TIME)
    if exc.get("state") != "approval":
        return False
    stamps = exc.get("timestamps") if isinstance(exc.get("timestamps"), dict) else {}
    expires = _parse_ts(stamps.get("expires_at"))
    until = _parse_ts((exc.get("time_limit") or {}).get("until") if isinstance(exc.get("time_limit"), dict) else None)
    if eval_time is None or expires is None or until is None:
        return False
    if expires <= eval_time or until <= eval_time:
        return False
    if exc.get("freshness") != "current":
        return False
    signer = exc.get("signing_identity")
    if signer not in KEY_REGISTRY or not KEY_REGISTRY[signer].get("may_approve_exception"):
        return False
    return True


def _overlay_coverage_mismatches(exc: dict[str, Any], overlay: dict[str, Any]) -> list[str]:
    codes: list[str] = []
    intent_ids = exc.get("intent_ids") if isinstance(exc.get("intent_ids"), list) else []
    policy_ids = exc.get("policy_ids") if isinstance(exc.get("policy_ids"), list) else []
    overlay_intent = overlay.get("intent_id") or INTENT_ID
    overlay_policy = overlay.get("policy_id") or POLICY_ID
    if overlay_intent not in intent_ids:
        codes.append("exception-intent-mismatch")
    if overlay_policy not in policy_ids:
        codes.append("exception-policy-mismatch")
    scope = exc.get("scope") if isinstance(exc.get("scope"), dict) else {}
    ov = overlay.get("scope") if isinstance(overlay.get("scope"), dict) else {}
    if scope.get("harness") != ov.get("family_id"):
        codes.append("exception-scope-mismatch")
    if scope.get("project") != ov.get("path"):
        codes.append("exception-scope-mismatch")
    if exc.get("overlay_digest") != overlay.get("digest"):
        codes.append("exception-overlay-digest-mismatch")
    if exc.get("overlay_lifecycle") != overlay.get("lifecycle"):
        codes.append("exception-overlay-digest-mismatch")
    return codes


def _exception_covers_overlay(exc: dict[str, Any], overlay: dict[str, Any]) -> bool:
    if not _exception_currently_valid(exc):
        return False
    return not _overlay_coverage_mismatches(exc, overlay)


def _validate_overlay(overlay: Any, payload: dict[str, Any], found: list[str]) -> None:
    if not isinstance(overlay, dict):
        _flag(found, "overlay-metadata-missing")
        return
    missing = [key for key in OVERLAY_REQUIRED_FIELDS if key not in overlay]
    if missing:
        _flag(found, "overlay-metadata-missing")
    extra = [key for key in overlay if key not in OVERLAY_REQUIRED_FIELDS]
    if extra:
        _flag(found, "malformed-schema")
    for key in ("owner", "reason", "lifecycle", "authority", "intent_id", "policy_id"):
        if not _is_nonempty_str(overlay.get(key)):
            _flag(found, "overlay-metadata-missing")
    scope = overlay.get("scope")
    if not isinstance(scope, dict) or any(not _is_nonempty_str(scope.get(key)) for key in OVERLAY_SCOPE_FIELDS):
        _flag(found, "overlay-metadata-missing")
    elif set(scope) != set(OVERLAY_SCOPE_FIELDS):
        _flag(found, "malformed-schema")
        _flag(found, "overlay-metadata-missing")
    else:
        # An overlay must point at a real harness surface and a real native path.
        overlay_family = scope.get("family_id")
        if overlay_family not in set(family_ids()):
            _flag(found, "overlay-metadata-missing")
            _flag(found, "family-native-mismatch")
        else:
            if scope.get("surface") != _surface_for(str(overlay_family))["surface"]:
                _flag(found, "overlay-metadata-missing")
                _flag(found, "family-native-mismatch")
            if not path_allowed_for_family(scope.get("path"), str(overlay_family)):
                _flag(found, "overlay-metadata-missing")
                _flag(found, "family-native-mismatch")
    # Overlay identity references must resolve against the authoritative objects.
    declared_intents = {
        item.get("id")
        for item in (payload.get("canonical_intents") or [])
        if isinstance(item, dict)
    }
    if declared_intents and overlay.get("intent_id") not in declared_intents:
        _flag(found, "overlay-metadata-missing")
        _flag(found, "exception-intent-mismatch")
    standard = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
    declared_policies = {
        item.get("id")
        for item in (standard.get("policy_rules") or [])
        if isinstance(item, dict)
    } or {POLICY_ID}
    if overlay.get("policy_id") not in declared_policies:
        _flag(found, "overlay-metadata-missing")
        _flag(found, "exception-policy-mismatch")
    if overlay.get("authority") != AUTHORITY:
        _flag(found, "overlay-metadata-missing")
    if overlay.get("projection_outcome") not in PROJECTION_OUTCOMES:
        _flag(found, "overlay-metadata-missing")
        _flag(found, "malformed-schema")
    if not isinstance(overlay.get("weakens_required"), bool):
        _flag(found, "overlay-metadata-missing")
        _flag(found, "malformed-schema")
    overlay_files = overlay.get("native_files")
    if not isinstance(overlay_files, list) or not overlay_files:
        _flag(found, "overlay-metadata-missing")
    else:
        overlay_paths: list[str] = []
        for item in overlay_files:
            if not isinstance(item, dict) or not _require_closed(item, NATIVE_FILE_FIELDS, found, "malformed-schema"):
                _flag(found, "overlay-metadata-missing")
                continue
            overlay_paths.append(str(item.get("path")))
            if item.get("digest_sha256") != sha256_text(str(item.get("content") or "")):
                _flag(found, "overlay-digest-mismatch")
            if isinstance(scope, dict) and not path_allowed_for_family(item.get("path"), str(scope.get("family_id"))):
                _flag(found, "overlay-metadata-missing")
                _flag(found, "family-native-mismatch")
        if len(overlay_paths) != len(set(overlay_paths)):
            _flag(found, "overlay-metadata-missing")
            _flag(found, "overlay-digest-mismatch")
        if isinstance(scope, dict) and str(scope.get("path")) not in overlay_paths:
            # The declared overlay surface must actually be among the overlay files.
            _flag(found, "overlay-metadata-missing")
    body = {key: value for key, value in overlay.items() if key != "digest"}
    if not _is_sha256(overlay.get("digest")) or overlay.get("digest") != _digest_obj(body):
        _flag(found, "overlay-digest-mismatch")
    exceptions = [item for item in (payload.get("exceptions") or []) if isinstance(item, dict)]
    if overlay.get("weakens_required"):
        covering = [item for item in exceptions if _exception_covers_overlay(item, overlay)]
        if not covering:
            _flag(found, "overlay-weakens-required-without-exception")
            if exceptions:
                _flag(found, "exception-scope-mismatch")
            for item in exceptions:
                for code in _overlay_coverage_mismatches(item, overlay):
                    _flag(found, code)
    elif exceptions:
        if not any(item.get("overlay_digest") == overlay.get("digest") for item in exceptions):
            _flag(found, "exception-overlay-digest-mismatch")
            _flag(found, "bound-digest-mismatch")


def _validate_publisher(publisher: Any, found: list[str]) -> dict[str, Any]:
    if not isinstance(publisher, dict):
        _flag(found, "incomplete-publisher")
        _flag(found, "nested-standard-type")
        return {}
    missing = [key for key in PUBLISHER_FIELDS if key not in publisher]
    extra = [key for key in publisher if key not in PUBLISHER_FIELDS]
    if missing:
        _flag(found, "incomplete-publisher")
    if extra:
        _flag(found, "malformed-schema")
        _flag(found, "nested-standard-type")
    for key in ("id", "display_name", "signing_identity", "key_id"):
        if not _is_nonempty_str(publisher.get(key)):
            _flag(found, "incomplete-publisher")
    role = publisher.get("role")
    authorization = publisher.get("authorization")
    key_id = publisher.get("key_id")
    identity = publisher.get("signing_identity")
    trust = publisher.get("trust_state")
    if role not in PUBLISHER_ROLES:
        _flag(found, "publisher-role-mismatch")
    if authorization not in PUBLISHER_AUTHORIZATIONS:
        _flag(found, "publisher-role-mismatch")
    if key_id not in KEY_REGISTRY:
        _flag(found, "unknown-signing-key")
        return publisher
    meta = KEY_REGISTRY[key_id]
    if identity != publisher.get("id") or identity != key_id:
        _flag(found, "publisher-role-mismatch")
        _flag(found, "publisher-identity-tuple-mismatch")
    if not meta.get("may_publish"):
        _flag(found, "member-key-as-publisher")
        _flag(found, "publisher-role-mismatch")
    if key_id == "syn-member":
        _flag(found, "member-key-as-publisher")
    if trust == "revoked" or meta.get("trust_state") == "revoked":
        _flag(found, "revoked-signing-key")
    if trust == "expired":
        _flag(found, "expired-signing-key")
    if trust == "unknown":
        _flag(found, "unknown-signing-key")
    if trust not in TRUST_STATES:
        _flag(found, "incomplete-publisher")
    if meta.get("role") != role or meta.get("authorization") != authorization:
        _flag(found, "publisher-role-mismatch")
        _flag(found, "publisher-identity-tuple-mismatch")
    return publisher


def _validate_signature_object(sig: Any, found: list[str], fail_code: str) -> None:
    if not isinstance(sig, dict):
        _flag(found, fail_code)
        _flag(found, "nested-standard-type")
        return
    if not _require_closed(sig, SIGNATURE_FIELDS, found, "nested-standard-type"):
        _flag(found, fail_code)
    if sig.get("alg") != SUPPORTED_SIGNATURE_ALG or sig.get("alg") == "none":
        _flag(found, "invalid-signature-alg")
        _flag(found, fail_code)


def _validate_identity_tuple(standard: dict[str, Any], found: list[str]) -> None:
    publisher = standard.get("publisher") if isinstance(standard.get("publisher"), dict) else {}
    trust = standard.get("trust") if isinstance(standard.get("trust"), dict) else {}
    signatures = standard.get("signatures") if isinstance(standard.get("signatures"), list) else []
    publisher_key = publisher.get("key_id")
    identities = [
        publisher.get("id"),
        publisher.get("signing_identity"),
        publisher.get("key_id"),
        trust.get("publisher_key_id"),
        trust.get("verified_key_id"),
    ]
    for sig in signatures:
        if isinstance(sig, dict):
            identities.append(sig.get("key_id"))
    nonempty = [item for item in identities if item]
    if nonempty and len(set(nonempty)) != 1:
        _flag(found, "publisher-identity-tuple-mismatch")
    unsigned = dict(standard)
    unsigned["signatures"] = []
    for sig in signatures:
        _validate_signature_object(sig, found, "forged-publisher")
        if not isinstance(sig, dict):
            continue
        if sig.get("key_id") != publisher_key:
            _flag(found, "publisher-identity-tuple-mismatch")
        if publisher_key in SYNTHETIC_KEYS:
            expected = synthetic_sign(unsigned, publisher_key)
            if not hmac.compare_digest(expected["signature"], str(sig.get("signature") or "")):
                _flag(found, "forged-publisher")
        elif not verify_synthetic(unsigned, sig):
            _flag(found, "forged-publisher")


def _validate_audit_chain(chain: Any, found: list[str], exc: dict[str, Any] | None = None) -> None:
    if not isinstance(chain, list) or not chain:
        _flag(found, "broken-audit-chain")
        return
    previous = "genesis"
    events: list[str] = []
    actor_for = {
        "request": (exc or {}).get("requester"),
        "approval": (exc or {}).get("approver"),
        "rejection": (exc or {}).get("approver"),
        "revocation": (exc or {}).get("approver"),
        "expiry": (exc or {}).get("approver"),
    }
    previous_ts = None
    for index, entry in enumerate(chain, start=1):
        if not isinstance(entry, dict):
            _flag(found, "broken-audit-chain")
            continue
        missing = [key for key in AUDIT_ENTRY_FIELDS if key not in entry]
        extra = [key for key in entry if key not in AUDIT_ENTRY_FIELDS]
        if missing or extra:
            _flag(found, "broken-audit-chain")
        if entry.get("sequence") != index or entry.get("previous_digest") != previous:
            _flag(found, "broken-audit-chain")
        body = {key: value for key, value in entry.items() if key != "digest"}
        if entry.get("digest") != _digest_obj(body):
            _flag(found, "broken-audit-chain")
        if not _is_nonempty_str(entry.get("event")) or not _is_timestamp(entry.get("at")):
            _flag(found, "broken-audit-chain")
        event = entry.get("event")
        events.append(str(event))
        expected_actor = actor_for.get(str(event))
        if expected_actor and entry.get("actor") != expected_actor:
            _flag(found, "audit-event-state-mismatch")
        if exc is not None:
            stamp_key = AUDIT_EVENT_TIMESTAMP.get(str(event))
            stamps = exc.get("timestamps") if isinstance(exc.get("timestamps"), dict) else {}
            if stamp_key and stamps.get(stamp_key) != entry.get("at"):
                # The audit entry must record the same instant as the exception state it claims.
                _flag(found, "audit-event-state-mismatch")
                _flag(found, "impossible-time-order")
        ts = _parse_ts(entry.get("at"))
        if previous_ts and ts and ts < previous_ts:
            _flag(found, "impossible-time-order")
            _flag(found, "broken-audit-chain")
        previous_ts = ts or previous_ts
        previous = entry.get("digest") or previous
    if exc is not None:
        expected_events = AUDIT_EVENTS_BY_STATE.get(str(exc.get("state")), [])
        if events != expected_events:
            _flag(found, "audit-event-state-mismatch")
            _flag(found, "broken-audit-chain")


def _validate_exception_times(exc: dict[str, Any], found: list[str]) -> None:
    eval_time = _parse_ts(EVALUATION_TIME)
    stamps = exc.get("timestamps") if isinstance(exc.get("timestamps"), dict) else {}
    extra = set(stamps) - set(EXCEPTION_TIMESTAMP_ALLOWED)
    missing = set(EXCEPTION_TIMESTAMP_FIELDS) - set(stamps)
    if extra or missing:
        _flag(found, "incomplete-exception")
    parsed = {key: _parse_ts(stamps.get(key)) for key in EXCEPTION_TIMESTAMP_ALLOWED if key in stamps}
    for key in EXCEPTION_TIMESTAMP_FIELDS:
        if parsed.get(key) is None:
            _flag(found, "incomplete-exception")
    requested = parsed.get("requested_at")
    issued = parsed.get("issued_at")
    approved = parsed.get("approved_at")
    expires = parsed.get("expires_at")
    revoked = parsed.get("revoked_at")
    if requested and issued and approved and expires:
        if not (requested < issued < approved < expires):
            _flag(found, "impossible-time-order")
        if revoked is not None and not (approved <= revoked):
            _flag(found, "impossible-time-order")
    time_limit = exc.get("time_limit") if isinstance(exc.get("time_limit"), dict) else {}
    start = _parse_ts(time_limit.get("from"))
    until = _parse_ts(time_limit.get("until"))
    if start and until and not (start < until):
        _flag(found, "impossible-time-order")
    if eval_time and expires and expires <= eval_time and exc.get("freshness") == "current":
        _flag(found, "current-labelled-expired")
    if eval_time and until and until <= eval_time and exc.get("freshness") == "current":
        _flag(found, "current-labelled-expired")


def _validate_exception(exc: Any, found: list[str]) -> None:
    if not isinstance(exc, dict):
        _flag(found, "incomplete-exception")
        return
    missing = [key for key in EXCEPTION_REQUIRED_FIELDS if key not in exc]
    extra = [key for key in exc if key not in EXCEPTION_REQUIRED_FIELDS]
    if missing:
        _flag(found, "incomplete-exception")
    if extra:
        _flag(found, "malformed-schema")
    for key in ("id", "requester", "approver", "reason", "signing_identity"):
        if not _is_nonempty_str(exc.get(key)):
            _flag(found, "incomplete-exception")
    if exc.get("state") not in EXCEPTION_STATES:
        _flag(found, "incomplete-exception")
    if exc.get("freshness") not in FRESHNESS_STATES:
        _flag(found, "invalid-freshness")
        _flag(found, "incomplete-exception")
    intent_ids = exc.get("intent_ids")
    policy_ids = exc.get("policy_ids")
    if not isinstance(intent_ids, list) or not intent_ids or not all(_is_nonempty_str(item) for item in intent_ids):
        _flag(found, "incomplete-exception")
    if not isinstance(policy_ids, list) or not policy_ids or not all(_is_nonempty_str(item) for item in policy_ids):
        _flag(found, "incomplete-exception")
    scope = exc.get("scope") if isinstance(exc.get("scope"), dict) else {}
    if not _require_closed(scope, EXCEPTION_SCOPE_FIELDS, found, "incomplete-exception"):
        pass
    if any(not _is_nonempty_str(scope.get(key)) for key in EXCEPTION_SCOPE_FIELDS):
        _flag(found, "incomplete-exception")
    if any(scope.get(key) == "*" for key in EXCEPTION_SCOPE_FIELDS):
        _flag(found, "unbounded-exception")
    use_limit = exc.get("use_limit")
    if not isinstance(use_limit, int) or isinstance(use_limit, bool) or use_limit <= 0:
        _flag(found, "unbounded-exception")
    time_limit = exc.get("time_limit") if isinstance(exc.get("time_limit"), dict) else {}
    if not _require_closed(time_limit, EXCEPTION_TIME_FIELDS, found, "unbounded-exception"):
        pass
    if not _is_timestamp(time_limit.get("from")) or not _is_timestamp(time_limit.get("until")):
        _flag(found, "unbounded-exception")
    _validate_exception_times(exc, found)
    requester = exc.get("requester")
    approver = exc.get("approver")
    signer = exc.get("signing_identity")
    if requester not in MEMBER_REGISTRY:
        _flag(found, "unregistered-identity")
        _flag(found, "incomplete-exception")
    else:
        if MEMBER_REGISTRY[requester].get("role") not in REQUESTER_ROLES:
            _flag(found, "requester-role-mismatch")
            _flag(found, "unregistered-identity")
        if not MEMBER_REGISTRY[requester].get("allowed_member"):
            _flag(found, "unregistered-identity")
    if approver not in MEMBER_REGISTRY or signer not in MEMBER_REGISTRY:
        _flag(found, "unregistered-identity")
        _flag(found, "invalid-exception-signer")
    if signer not in KEY_REGISTRY:
        _flag(found, "unknown-signing-key")
        _flag(found, "invalid-exception-signer")
    else:
        meta = KEY_REGISTRY[signer]
        if exc.get("state") in {"approval", "rejection", "revocation"} and not meta.get("may_approve_exception"):
            _flag(found, "invalid-exception-signer")
        if signer == "syn-member" and exc.get("state") == "approval":
            _flag(found, "invalid-exception-signer")
        if approver != signer:
            _flag(found, "approver-signer-mismatch")
            _flag(found, "invalid-exception-signer")
        if approver in KEY_REGISTRY and (
            not KEY_REGISTRY[approver].get("may_approve_exception")
            or KEY_REGISTRY[approver].get("role") not in APPROVER_ROLES
        ):
            _flag(found, "invalid-exception-signer")
    signature = exc.get("signature")
    unsigned = {key: value for key, value in exc.items() if key != "signature"}
    _validate_signature_object(signature, found, "forged-exception")
    if isinstance(signature, dict):
        if signature.get("key_id") != signer:
            _flag(found, "approver-signer-mismatch")
            _flag(found, "invalid-exception-signer")
        if not verify_synthetic(unsigned, signature):
            _flag(found, "forged-exception")
    if not _is_nonempty_str(exc.get("overlay_digest")) or not _is_nonempty_str(exc.get("overlay_lifecycle")):
        _flag(found, "incomplete-exception")
    _validate_audit_chain(exc.get("audit_chain"), found, exc)


def _compute_layer_violations(assignments: list[Any], found: list[str]) -> dict[str, str]:
    by_layer: dict[str, dict[str, Any]] = {}
    for row in assignments:
        if isinstance(row, dict) and row.get("layer") in LAYER_ORDER:
            by_layer[row["layer"]] = row
            extra = set(row) - set(LAYER_ASSIGNMENT_FIELDS) - {"honesty", "shadows_higher_required", "relaxes"}
            missing = set(LAYER_ASSIGNMENT_FIELDS) - set(row)
            if extra or missing:
                _flag(found, "malformed-schema")
    strict: dict[str, str] = {}
    for layer in LAYER_ORDER:
        row = by_layer.get(layer)
        if not row:
            continue
        rule_id = row.get("rule_id")
        mode = row.get("mode")
        if mode not in ENFORCEMENT_MODES or not _is_nonempty_str(rule_id):
            _flag(found, "malformed-schema")
            continue
        if rule_id in strict:
            prev = strict[rule_id]
            if _mode_rank(mode) < _mode_rank(prev) and prev in {"required", "prohibited"}:
                _flag(found, "required-downgraded-to-recommended")
                if layer in {"project", "personal", "role"}:
                    _flag(found, "personal-shadows-required")
        if mode in {"required", "prohibited"} and rule_id not in strict:
            strict[rule_id] = mode
        if row.get("shadows_higher_required") or row.get("relaxes") in {"required", "prohibited"}:
            _flag(found, "personal-shadows-required")
    return strict


def _transaction_body(payload: dict[str, Any]) -> dict[str, Any]:
    """Bind the transaction to the content it actually changes, not to three booleans."""
    lifecycle = payload.get("lifecycle") if isinstance(payload.get("lifecycle"), dict) else {}
    update = lifecycle.get("update") if isinstance(lifecycle.get("update"), dict) else {}
    rollback = lifecycle.get("rollback") if isinstance(lifecycle.get("rollback"), dict) else {}
    overlays = [item for item in (payload.get("overlays") or []) if isinstance(item, dict)]
    overlay_digests = [item.get("digest") for item in overlays]
    standard = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
    if update or rollback:
        return {
            "preview": update.get("preview") is True,
            "apply": bool(update.get("apply_at")),
            "rollback": rollback.get("authorized") is True,
            "apply_at": update.get("apply_at"),
            "rollback_target_revision": rollback.get("target_revision"),
            "rollback_target_digest": rollback.get("target_digest"),
            "standard_revision": standard.get("revision"),
            "overlay_digests": overlay_digests,
        }
    return {
        "preview": True,
        "apply": False,
        "rollback": True,
        "apply_at": None,
        "rollback_target_revision": None,
        "rollback_target_digest": None,
        "standard_revision": standard.get("revision"),
        "overlay_digests": overlay_digests,
    }


def _expected_bound_digests(payload: dict[str, Any], projections: list[dict[str, Any]]) -> dict[str, str]:
    intents = [item for item in (payload.get("canonical_intents") or []) if isinstance(item, dict)]
    if intents:
        intent = intents[0]
        intent_digest = _digest_obj(intent)
        authority = intent.get("authority")
        intent_id = intent.get("id")
    else:
        intent_digest = sha256_text("missing-intent")
        authority = None
        intent_id = None
    if isinstance(authority, dict):
        authority_digest = _digest_obj(authority)
    else:
        authority_digest = sha256_text(AUTHORITY)
    exception_digest = sha256_text("none")
    for exc in payload.get("exceptions") or []:
        if isinstance(exc, dict):
            exception_digest = _digest_obj(exc)
            break
    used = [item for item in projections if isinstance(item, dict) and item.get("family_id") in REQUIRED_FOUR]
    if len(used) < 4:
        used = [item for item in projections if isinstance(item, dict)]
    native = _digest_obj([item.get("native_digest") for item in used])
    oracle = _digest_obj([item.get("oracle") for item in used])
    rules = []
    standard = payload.get("team_standard")
    if isinstance(standard, dict):
        rules = [item for item in (standard.get("policy_rules") or []) if isinstance(item, dict)]
    policy_digest = _digest_obj(rules[0]) if rules else _digest_obj(policy_rule())
    return {
        "intent": intent_digest,
        "plan": _digest_obj({"authority": AUTHORITY, "intent_id": intent_id, "pipeline": list(PIPELINE_STEPS)}),
        "authority": authority_digest,
        "transaction": _digest_obj(_transaction_body(payload)),
        "native_artifact": native,
        "resolver_oracle": oracle,
        "policy": policy_digest,
        "exception": exception_digest,
    }


def _walk_leader_privacy(obj: Any, found: list[str], top: bool = True) -> None:
    if isinstance(obj, dict):
        keys = set(obj)
        if "private_prompt_text" in keys or "prompt_text" in keys:
            _flag(found, "leader-private-prompt")
            if not top:
                _flag(found, "nested-private-content")
        if "secrets" in keys or "secret_value" in keys:
            _flag(found, "leader-secrets")
            if not top:
                _flag(found, "nested-private-content")
        if "unrelated_personal_context" in keys:
            _flag(found, "leader-personal-context")
            if not top:
                _flag(found, "nested-private-content")
        if keys & {"full_session_history", "session_history", "session_body"}:
            _flag(found, "leader-session-history")
            if not top:
                _flag(found, "nested-private-content")
        if keys & SECRET_KEY_NAMES:
            _flag(found, "leader-secrets")
            if not top:
                _flag(found, "nested-private-content")
        for value in obj.values():
            _walk_leader_privacy(value, found, top=False)
    elif isinstance(obj, list):
        for item in obj:
            _walk_leader_privacy(item, found, top=False)


def _validate_disclosure(payload: dict[str, Any], found: list[str]) -> None:
    view = payload.get("leader_view")
    lifecycle = payload.get("lifecycle") if isinstance(payload.get("lifecycle"), dict) else {}
    steps = lifecycle.get("steps") or []
    needs = isinstance(view, dict) or any(
        "update" in str(step.get("command") or "") or "report" in str(step.get("command") or "")
        for step in steps
        if isinstance(step, dict)
    )
    disclosure = payload.get("member_disclosure")
    if not needs and disclosure is None:
        return
    if needs and not isinstance(disclosure, dict):
        _flag(found, "disclosure-missing")
        return
    if not isinstance(disclosure, dict):
        return
    missing = [key for key in DISCLOSURE_REQUIRED_FIELDS if key not in disclosure]
    extra = [key for key in disclosure if key not in DISCLOSURE_REQUIRED_FIELDS]
    if missing:
        _flag(found, "disclosure-missing")
    if extra:
        _flag(found, "malformed-schema")
    if disclosure.get("shown_before_report") is not True:
        _flag(found, "disclosure-missing")
    if disclosure.get("consent_status") not in CONSENT_STATUSES:
        _flag(found, "disclosure-missing")
    if not isinstance(disclosure.get("upload_consented"), bool):
        _flag(found, "disclosure-missing")
        _flag(found, "malformed-schema")
    if disclosure.get("upload_consented") is True and disclosure.get("consent_status") != "acknowledged":
        _flag(found, "disclosure-missing")
    preview = disclosure.get("preview")
    previewed_fields = disclosure.get("previewed_fields")
    if not isinstance(preview, dict) or _empty_collection(preview):
        _flag(found, "empty-preview")
        _flag(found, "disclosure-missing")
        preview = {}
    else:
        if set(preview) != set(PREVIEW_OBJECT_FIELDS):
            _flag(found, "malformed-schema")
            _flag(found, "empty-preview")
        if any(preview.get(key) in (None, "", [], {}) for key in PREVIEW_OBJECT_FIELDS):
            _flag(found, "empty-preview")
        # A preview that says "no per-harness projection / no loss / no exception
        # metadata" is not a preview of this disclosure.
        for key in ("per_harness_projection", "loss_unknown", "exception_metadata"):
            if preview.get(key) is not True:
                _flag(found, "empty-preview")
                _flag(found, "disclosure-missing")
        standard = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
        expected_version = standard.get("semantic_version")
        if expected_version is not None and preview.get("standard_version") != expected_version:
            _flag(found, "disclosure-payload-mismatch")
            _flag(found, "disclosure-digest-mismatch")
        elif expected_version is None and not _is_semver(preview.get("standard_version")):
            _flag(found, "disclosure-payload-mismatch")
        if isinstance(view, dict):
            if view.get("standard_version") != preview.get("standard_version"):
                _flag(found, "disclosure-payload-mismatch")
            if standard.get("stable_id") is not None and view.get("standard_id") != standard.get("stable_id"):
                _flag(found, "disclosure-payload-mismatch")
    if list(previewed_fields or []) != list(PREVIEW_OBJECT_FIELDS):
        _flag(found, "empty-preview")
        _flag(found, "disclosure-missing")
    ack = disclosure.get("acknowledgement") if isinstance(disclosure.get("acknowledgement"), dict) else {}
    if not _require_closed(ack, ACK_FIELDS, found, "disclosure-missing"):
        pass
    expected_digest = disclosure_binding_digest(disclosure, payload) if preview else ""
    digest = disclosure.get("disclosed_digest")
    if not _is_sha256(digest) or digest != expected_digest or ack.get("digest") != expected_digest:
        _flag(found, "disclosure-digest-mismatch")
        if needs:
            _flag(found, "disclosure-missing")
    if ack.get("member_id") not in MEMBER_REGISTRY:
        _flag(found, "unregistered-identity")
        _flag(found, "disclosure-missing")
    elif MEMBER_REGISTRY[ack.get("member_id")].get("role") not in REQUESTER_ROLES:
        _flag(found, "requester-role-mismatch")
        _flag(found, "disclosure-digest-mismatch")
    preview_at = _parse_ts(disclosure.get("previewed_at"))
    ack_at = _parse_ts(ack.get("at"))
    report_at = _parse_ts(disclosure.get("report_or_upload_at"))
    if not (preview_at and ack_at and report_at) or not (preview_at < ack_at <= report_at):
        _flag(found, "disclosure-order")
    payload_digest = disclosure.get("report_payload_digest")
    if isinstance(view, dict):
        expected_payload = _digest_obj(view)
    else:
        expected_payload = _digest_obj(preview) if preview else ""
    if payload_digest != expected_payload:
        _flag(found, "disclosure-payload-mismatch")
    if disclosure.get("upload_consented") and disclosure.get("shown_before_report") is not True:
        _flag(found, "disclosure-missing")
    if isinstance(view, dict) and disclosure.get("consent_status") != "acknowledged":
        _flag(found, "disclosure-missing")


def _validate_rotation(rotation: Any, found: list[str]) -> None:
    if not isinstance(rotation, dict):
        _flag(found, "invalid-key-rotation")
        _flag(found, "unsigned-rotation")
        return
    if not _require_closed(rotation, ROTATION_FIELDS, found, "invalid-key-rotation"):
        _flag(found, "unsigned-rotation")
    old_key = rotation.get("old_key_id")
    new_key = rotation.get("new_key_id")
    if not isinstance(rotation.get("applied"), bool):
        _flag(found, "invalid-key-rotation")
        _flag(found, "malformed-schema")
    if old_key == new_key:
        # Rotating a key onto itself is not a rotation.
        _flag(found, "invalid-key-rotation")
        _flag(found, "untrusted-rotation-key")
    if rotation.get("old_trust_state") not in TRUST_STATES:
        _flag(found, "invalid-key-rotation")
    if old_key not in KEY_REGISTRY or not KEY_REGISTRY[old_key].get("may_publish"):
        _flag(found, "invalid-key-rotation")
    if new_key not in KEY_REGISTRY or not KEY_REGISTRY[new_key].get("may_publish"):
        _flag(found, "invalid-key-rotation")
        _flag(found, "untrusted-rotation-key")
        if new_key == "syn-member":
            _flag(found, "member-key-as-publisher")
    if rotation.get("new_trust_state") != "trusted":
        _flag(found, "invalid-key-rotation")
        _flag(found, "untrusted-rotation-key")
    dual = rotation.get("dual_authorization")
    if not isinstance(dual, list) or set(dual) != {old_key, new_key}:
        _flag(found, "invalid-key-rotation")
        _flag(found, "unsigned-rotation")
    unsigned = {key: value for key, value in rotation.items() if key not in {"old_signature", "new_signature"}}
    for sig_key, expected_id in (("old_signature", old_key), ("new_signature", new_key)):
        sig = rotation.get(sig_key)
        _validate_signature_object(sig, found, "unsigned-rotation")
        if not isinstance(sig, dict) or sig.get("key_id") != expected_id or not verify_synthetic(unsigned, sig):
            _flag(found, "invalid-key-rotation")
            _flag(found, "unsigned-rotation")
    activated = _parse_ts(rotation.get("activated_at"))
    revoked = _parse_ts(rotation.get("revoked_at"))
    rotated = _parse_ts(rotation.get("rotated_at"))
    if not (activated and revoked and rotated) or not (rotated <= activated <= revoked):
        _flag(found, "invalid-key-rotation")
    if rotation.get("applied") and rotation.get("trust_store_result") != "new-key-trusted-old-key-revoked":
        _flag(found, "invalid-key-rotation")
    if rotation.get("applied") and rotation.get("old_trust_state") != "revoked":
        _flag(found, "invalid-key-rotation")
        _flag(found, "revoked-key-still-trusted")


def _validate_lifecycle(payload: dict[str, Any], found: list[str], required: bool) -> None:
    lifecycle = payload.get("lifecycle")
    if not isinstance(lifecycle, dict):
        if required:
            _flag(found, "missing-lifecycle-group")
            _flag(found, "required-collection-cardinality")
        return
    extra = [key for key in lifecycle if key not in LIFECYCLE_ALLOWED]
    if extra:
        _flag(found, "malformed-schema")
    missing_groups = [key for key in LIFECYCLE_REQUIRED_GROUPS if key not in lifecycle]
    if missing_groups:
        _flag(found, "missing-lifecycle-group")
        if "last_known_good" in missing_groups:
            _flag(found, "last-known-good-missing")
        if "update" in missing_groups:
            _flag(found, "update-without-preview")
            _flag(found, "update-without-disclosure")
        if "key_rotation" in missing_groups:
            _flag(found, "invalid-key-rotation")
            _flag(found, "unsigned-rotation")
        if "steps" in missing_groups:
            _flag(found, "lifecycle-step-order")
    steps = lifecycle.get("steps")
    if not isinstance(steps, list) or not steps:
        _flag(found, "missing-lifecycle-group")
        _flag(found, "lifecycle-step-order")
        steps = []
    commands = []
    has_update = False
    for step in steps:
        if not isinstance(step, dict):
            _flag(found, "lifecycle-step-order")
            continue
        if not _require_closed(
            step,
            LIFECYCLE_STEP_REQUIRED,
            found,
            "lifecycle-step-order",
            allowed=LIFECYCLE_STEP_ALLOWED,
        ):
            pass
        commands.append(str(step.get("command") or ""))
        command = str(step.get("command") or "")
        # A step must name a real actor and carry real preview/disclosure booleans;
        # null or an unrelated value is not an answer.
        if step.get("actor") not in LIFECYCLE_STEP_ACTORS:
            _flag(found, "lifecycle-step-order")
            _flag(found, "unregistered-identity")
        if step.get("preview") is not True:
            _flag(found, "update-without-preview")
            _flag(found, "lifecycle-step-order")
        if step.get("disclosure") is not True:
            _flag(found, "update-without-disclosure")
            _flag(found, "lifecycle-step-order")
        if "state" in step and step.get("state") not in LIFECYCLE_STEP_STATES:
            _flag(found, "lifecycle-step-order")
            _flag(found, "malformed-schema")
        for key in ("pin", "per_harness"):
            if key in step and not isinstance(step.get(key), bool):
                _flag(found, "lifecycle-step-order")
                _flag(found, "malformed-schema")
        if "drift_class" in step and step.get("drift_class") not in DRIFT_CLASSES:
            _flag(found, "lifecycle-step-order")
            _flag(found, "malformed-schema")
        if "update" in command:
            has_update = True
            if step.get("preview") is False:
                _flag(found, "update-without-preview")
            if step.get("disclosure") is False:
                _flag(found, "update-without-disclosure")
    if commands != LIFECYCLE_STEP_COMMANDS:
        _flag(found, "lifecycle-step-order")
    rollout = lifecycle.get("staged_rollout")
    if not isinstance(rollout, dict):
        _flag(found, "incomplete-rollout")
        _flag(found, "missing-lifecycle-group")
    else:
        if not _require_closed(rollout, STAGED_ROLLOUT_FIELDS, found, "incomplete-rollout"):
            pass
        percent = rollout.get("percent")
        if not isinstance(percent, int) or isinstance(percent, bool) or not 1 <= percent <= 100:
            _flag(found, "incomplete-rollout")
        stages = rollout.get("stages")
        if not isinstance(stages, list) or not stages:
            _flag(found, "incomplete-rollout")
        else:
            percents: list[int] = []
            for index, stage in enumerate(stages, start=1):
                if not isinstance(stage, dict) or not _require_closed(stage, STAGE_FIELDS, found, "incomplete-rollout"):
                    _flag(found, "incomplete-rollout")
                    continue
                if stage.get("order") != index:
                    _flag(found, "incomplete-rollout")
                if stage.get("approval_required") is not True or stage.get("health_rollback") is not True:
                    _flag(found, "incomplete-rollout")
                stage_percent = stage.get("percent")
                if not isinstance(stage_percent, int) or isinstance(stage_percent, bool) or not 1 <= stage_percent <= 100:
                    _flag(found, "incomplete-rollout")
                else:
                    percents.append(stage_percent)
            if percents != sorted(percents):
                _flag(found, "incomplete-rollout")
        if rollout.get("approval") not in MEMBER_REGISTRY:
            _flag(found, "incomplete-rollout")
            _flag(found, "unregistered-identity")
        if rollout.get("health_rollback") is not True:
            _flag(found, "incomplete-rollout")
        order = rollout.get("order")
        if not isinstance(order, list) or not order or not all(_is_nonempty_str(item) for item in order):
            _flag(found, "incomplete-rollout")
        else:
            stage_names = [
                stage.get("name")
                for stage in (rollout.get("stages") or [])
                if isinstance(stage, dict)
            ]
            if stage_names and list(order) != stage_names:
                # The declared order must be the stage sequence itself.
                _flag(found, "incomplete-rollout")
            if len(order) != len(set(map(str, order))):
                _flag(found, "incomplete-rollout")
            if stage_names and rollout.get("cohort") not in stage_names:
                _flag(found, "incomplete-rollout")
        stages_list = rollout.get("stages") or []
        stage_name_list = [
            stage.get("name") for stage in stages_list if isinstance(stage, dict)
        ]
        if len(stage_name_list) != len(set(map(str, stage_name_list))):
            _flag(found, "incomplete-rollout")
        cohort_stage = next(
            (
                stage
                for stage in stages_list
                if isinstance(stage, dict) and stage.get("name") == rollout.get("cohort")
            ),
            None,
        )
        if cohort_stage is not None and rollout.get("percent") != cohort_stage.get("percent"):
            # The reported percent must be the percent of the cohort the rollout declares it is in.
            _flag(found, "incomplete-rollout")
    conflict = lifecycle.get("conflict_resolution")
    if isinstance(conflict, dict) and conflict.get("silent_merge"):
        _flag(found, "fork-conflict-silent-merge")
    if not isinstance(conflict, dict):
        _flag(found, "conflict-nondeterministic")
        _flag(found, "missing-lifecycle-group")
    else:
        if not _require_closed(conflict, CONFLICT_FIELDS, found, "conflict-nondeterministic"):
            pass
        inputs = conflict.get("inputs")
        winner = conflict.get("winner")
        if not isinstance(inputs, list) or not inputs or winner not in inputs or len(set(inputs)) != len(inputs):
            _flag(found, "conflict-nondeterministic")
        if not _is_nonempty_str(conflict.get("reason")) or conflict.get("mode") not in CONFLICT_MODES:
            _flag(found, "conflict-nondeterministic")
        if not isinstance(conflict.get("silent_merge"), bool):
            _flag(found, "conflict-nondeterministic")
            _flag(found, "malformed-schema")
        if isinstance(inputs, list) and (
            not all(_is_nonempty_str(item) for item in inputs) or len(inputs) < 2
        ):
            # A conflict needs at least two distinct, named heads to choose between.
            _flag(found, "conflict-nondeterministic")
        if conflict.get("mode") == "silent-merge":
            _flag(found, "conflict-nondeterministic")
    last_known = lifecycle.get("last_known_good")
    if not isinstance(last_known, dict):
        _flag(found, "last-known-good-missing")
    else:
        if not _require_closed(last_known, LKG_FIELDS, found, "last-known-good-missing"):
            pass
        if last_known.get("recoverable") is not True or last_known.get("revision") is None:
            _flag(found, "last-known-good-missing")
        if last_known.get("identity") != STANDARD_ID or last_known.get("digest") != lkg_digest():
            _flag(found, "last-known-good-missing")
        lkg_revision = last_known.get("revision")
        if not isinstance(lkg_revision, int) or isinstance(lkg_revision, bool) or lkg_revision < 0:
            _flag(found, "last-known-good-missing")
        standard = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
        if standard:
            lineage = standard.get("lineage") if isinstance(standard.get("lineage"), dict) else {}
            if lineage and lkg_revision != lineage.get("previous_revision"):
                # The recoverable good state must be the standard's actual predecessor.
                _flag(found, "last-known-good-missing")
                _flag(found, "unrelated-rollback-target")
            if lineage and last_known.get("digest") != lineage.get("previous_digest"):
                _flag(found, "last-known-good-missing")
                _flag(found, "unrelated-rollback-target")
            if standard.get("stable_id") is not None and last_known.get("identity") != standard.get("stable_id"):
                _flag(found, "last-known-good-missing")
    rollback = lifecycle.get("rollback")
    if not isinstance(rollback, dict):
        _flag(found, "unrelated-rollback-target")
        _flag(found, "missing-lifecycle-group")
    else:
        if not _require_closed(rollback, LIFECYCLE_ROLLBACK_FIELDS, found, "unrelated-rollback-target"):
            pass
        if rollback.get("target_revision") != (last_known.get("revision") if isinstance(last_known, dict) else None):
            _flag(found, "unrelated-rollback-target")
        if rollback.get("target_digest") != (last_known.get("digest") if isinstance(last_known, dict) else None):
            _flag(found, "unrelated-rollback-target")
        if rollback.get("authorized") is not True or rollback.get("authorization") not in PUBLISHER_AUTHORIZATIONS:
            _flag(found, "unrelated-rollback-target")
        if rollback.get("preserves_unrelated_personal_files") is not True:
            _flag(found, "rollback-deletes-personal")
    update = lifecycle.get("update")
    if not isinstance(update, dict):
        if has_update or required:
            _flag(found, "update-without-preview")
            _flag(found, "update-without-disclosure")
    else:
        if not _require_closed(update, UPDATE_FIELDS, found, "update-without-preview"):
            pass
        if update.get("preview") is not True:
            _flag(found, "update-without-preview")
        if update.get("disclosure") is not True:
            _flag(found, "update-without-disclosure")
        expected_digest = expected_update_digest(payload)
        if update.get("digest") != expected_digest or update.get("preview_digest") != expected_digest:
            _flag(found, "update-preview-digest-mismatch")
        if not _is_timestamp(update.get("apply_at")):
            _flag(found, "update-without-preview")
        disclosure = payload.get("member_disclosure") if isinstance(payload.get("member_disclosure"), dict) else {}
        preview_at = _parse_ts(disclosure.get("previewed_at"))
        apply_at = _parse_ts(update.get("apply_at"))
        if preview_at and apply_at and not (preview_at < apply_at):
            _flag(found, "disclosure-order")
    rotation = lifecycle.get("key_rotation")
    if rotation is None and required:
        _flag(found, "invalid-key-rotation")
        _flag(found, "unsigned-rotation")
    elif rotation is not None:
        _validate_rotation(rotation, found)


def _authorized_lkg_rollback(standard: dict[str, Any], lineage: dict[str, Any]) -> bool:
    rollback = standard.get("rollback") if isinstance(standard.get("rollback"), dict) else {}
    tx = rollback.get("transaction") if isinstance(rollback.get("transaction"), dict) else {}
    return (
        tx.get("authorized") is True
        and tx.get("target_is_last_known_good") is True
        and tx.get("target_revision") == lineage.get("previous_revision")
        and tx.get("target_digest") == lineage.get("previous_digest")
        and tx.get("target_digest") == lkg_digest()
        and _is_sha256(tx.get("audit_digest"))
        and _is_sha256(tx.get("receipt_digest"))
        and tx.get("audit_digest") != sha256_text("none")
        and tx.get("receipt_digest") != sha256_text("none")
    )


def _validate_team_standard(standard: Any, payload: dict[str, Any], intent_ids: set[str], found: list[str]) -> None:
    if not isinstance(standard, dict):
        return
    missing = [key for key in TEAM_STANDARD_FIELDS if key not in standard]
    extra = [key for key in standard if key not in TEAM_STANDARD_FIELDS and key not in SECRET_KEY_NAMES]
    if missing:
        found.append("truncated-bundle")
    if extra:
        found.append("malformed-schema")
        found.append("nested-standard-type")
    secrets: list[str] = []
    _walk_secret_keys(standard, secrets)
    if secrets:
        found.append("secret-in-standard")
    _validate_publisher(standard.get("publisher"), found)
    _validate_identity_tuple(standard, found)
    if not _is_semver(standard.get("semantic_version")):
        _flag(found, "invalid-semver")
        _flag(found, "nested-standard-type")
    revision = standard.get("revision")
    if not isinstance(revision, int) or isinstance(revision, bool) or revision < 0:
        _flag(found, "nested-standard-type")
        _flag(found, "rollback-version")
    members = standard.get("members")
    if not isinstance(members, list) or not members:
        _flag(found, "truncated-bundle")
        _flag(found, "nested-standard-type")
    else:
        member_ids = set()
        for member in members:
            if not isinstance(member, dict) or not _require_closed(member, MEMBER_RECORD_FIELDS, found, "nested-standard-type"):
                _flag(found, "nested-standard-type")
                continue
            if member.get("id") not in KEY_REGISTRY or member.get("key_id") != member.get("id"):
                _flag(found, "nested-standard-type")
                _flag(found, "publisher-identity-tuple-mismatch")
            member_ids.add(member.get("id"))
        publisher = standard.get("publisher") if isinstance(standard.get("publisher"), dict) else {}
        if publisher.get("id") not in member_ids:
            _flag(found, "incomplete-publisher")
    trust = standard.get("trust")
    if not isinstance(trust, dict) or not _require_closed(trust, TRUST_RECORD_FIELDS, found, "nested-standard-type"):
        _flag(found, "nested-standard-type")
    elif trust.get("state") not in TRUST_STATES:
        _flag(found, "nested-standard-type")
    else:
        state = trust.get("state")
        if state == "revoked":
            _flag(found, "revoked-signing-key")
            _flag(found, "revoked-key-still-trusted")
        elif state == "expired":
            _flag(found, "expired-signing-key")
        elif state == "unknown":
            _flag(found, "unknown-signing-key")
        if trust.get("publisher_key_id") not in KEY_REGISTRY:
            _flag(found, "unknown-signing-key")
        elif KEY_REGISTRY[trust["publisher_key_id"]].get("trust_state") == "revoked" and state == "trusted":
            _flag(found, "revoked-key-still-trusted")
    lineage = standard.get("lineage")
    if not isinstance(lineage, dict) or not _require_closed(lineage, LINEAGE_FIELDS, found, "nested-standard-type"):
        _flag(found, "nested-standard-type")
        lineage = {}
    else:
        history = lineage.get("history")
        if not isinstance(history, list) or not history:
            _flag(found, "nested-standard-type")
        else:
            seen_revisions: list[Any] = []
            digest_by_revision: dict[Any, Any] = {}
            for item in history:
                if not isinstance(item, dict) or not _require_closed(item, LINEAGE_HISTORY_FIELDS, found, "nested-standard-type"):
                    _flag(found, "nested-standard-type")
                    continue
                item_revision = item.get("revision")
                if not isinstance(item_revision, int) or isinstance(item_revision, bool) or item_revision < 0:
                    _flag(found, "nested-standard-type")
                    _flag(found, "rollback-version")
                if not _is_sha256(item.get("digest_sha256")):
                    _flag(found, "nested-standard-type")
                    _flag(found, "swapped-digest")
                seen_revisions.append(item_revision)
                digest_by_revision[item_revision] = item.get("digest_sha256")
            # Replay: a revision may appear at most once, and the recorded order must ascend.
            if len(seen_revisions) != len(set(map(str, seen_revisions))):
                _flag(found, "lineage-skip")
                _flag(found, "rollback-version")
            ordered = [item for item in seen_revisions if isinstance(item, int) and not isinstance(item, bool)]
            if ordered != sorted(ordered):
                _flag(found, "lineage-skip")
            # The claimed predecessor must be the last recorded history entry.
            prev_revision = lineage.get("previous_revision")
            if ordered and ordered[-1] != prev_revision:
                _flag(found, "lineage-skip")
                _flag(found, "rollback-version")
            if prev_revision in digest_by_revision and digest_by_revision[prev_revision] != lineage.get("previous_digest"):
                _flag(found, "swapped-digest")
                _flag(found, "lineage-skip")
            if isinstance(revision, int) and not isinstance(revision, bool) and revision in digest_by_revision:
                # The current revision cannot already be a historical one.
                _flag(found, "lineage-skip")
                _flag(found, "rollback-version")
        floor = lineage.get("current_floor")
        prev_rev = lineage.get("previous_revision")
        if not isinstance(floor, int) or isinstance(floor, bool) or not isinstance(prev_rev, int) or isinstance(prev_rev, bool):
            _flag(found, "nested-standard-type")
        elif isinstance(revision, int) and not isinstance(revision, bool):
            authorized = _authorized_lkg_rollback(standard, lineage)
            if authorized:
                if revision != prev_rev or lineage.get("previous_digest") != lkg_digest():
                    _flag(found, "rollback-version")
                    _flag(found, "signed-rollback-without-authorization")
            else:
                if revision != prev_rev + 1:
                    _flag(found, "lineage-skip")
                    _flag(found, "rollback-version")
                    if revision < floor or revision <= prev_rev:
                        _flag(found, "signed-rollback-without-authorization")
                if revision < floor:
                    _flag(found, "rollback-version")
                    _flag(found, "signed-rollback-without-authorization")
    if not isinstance(standard.get("compatibility_floors"), dict):
        _flag(found, "truncated-bundle")
        _flag(found, "nested-standard-type")
    else:
        floors = standard.get("compatibility_floors")
        if not _require_closed(floors, COMPAT_FLOOR_FIELDS, found, "nested-standard-type"):
            pass
        if not isinstance(floors.get("harness"), dict) or not floors.get("harness"):
            _flag(found, "nested-standard-type")
    coordinates = standard.get("target_harness_coordinates")
    if not isinstance(coordinates, list) or not coordinates:
        _flag(found, "truncated-bundle")
    else:
        known = {_surface_for(fid)["coordinate"] for fid in family_ids()}
        if any(item not in known for item in coordinates):
            _flag(found, "family-native-mismatch")
            _flag(found, "nested-standard-type")
        if len(coordinates) != len(set(map(str, coordinates))):
            _flag(found, "nested-standard-type")
            _flag(found, "incomplete-manifest")
    rules = standard.get("policy_rules")
    if not isinstance(rules, list) or not rules:
        _flag(found, "truncated-bundle")
        _flag(found, "nested-standard-type")
    else:
        rule_ids: list[Any] = []
        for rule in rules:
            if not isinstance(rule, dict) or not _require_closed(rule, POLICY_RULE_FIELDS, found, "nested-standard-type"):
                _flag(found, "nested-standard-type")
                continue
            rule_ids.append(rule.get("id"))
            if not _is_nonempty_str(rule.get("id")) or not _is_nonempty_str(rule.get("text")):
                _flag(found, "nested-standard-type")
            if rule.get("layer") not in LAYER_ORDER:
                _flag(found, "nested-standard-type")
            if rule.get("mode") not in ENFORCEMENT_MODES:
                _flag(found, "nested-standard-type")
                _flag(found, "required-downgraded-to-recommended")
            if rule.get("honesty") not in ENFORCEMENT_HONESTY:
                _flag(found, "nested-standard-type")
                _flag(found, "false-enforcement-claim")
            rule_intents = rule.get("intent_ids")
            if not isinstance(rule_intents, list) or not rule_intents or not all(_is_nonempty_str(item) for item in rule_intents):
                _flag(found, "nested-standard-type")
            elif intent_ids and any(item not in intent_ids for item in rule_intents):
                _flag(found, "exception-intent-mismatch")
                _flag(found, "nested-standard-type")
        if len(rule_ids) != len(set(map(str, rule_ids))):
            _flag(found, "incomplete-manifest")
            _flag(found, "nested-standard-type")
    if not isinstance(standard.get("canonical_intent_set"), list) or not standard.get("canonical_intent_set"):
        _flag(found, "truncated-bundle")
    elif intent_ids and any(item not in intent_ids for item in standard.get("canonical_intent_set") or []):
        _flag(found, "malformed-schema")
    signatures = standard.get("signatures")
    if "signatures" in standard and not signatures:
        found.append("unsigned-standard")
        found.append("missing-signature")
    elif "signatures" in standard and not isinstance(signatures, list):
        # A signature block that is not an array is not a set of signatures.
        found.append("unsigned-standard")
        found.append("missing-signature")
        _flag(found, "nested-standard-type")
    elif signatures:
        signing_keys: list[Any] = []
        for sig in signatures:
            if not isinstance(sig, dict):
                _flag(found, "nested-standard-type")
                found.append("missing-signature")
                continue
            key_id = sig.get("key_id")
            signing_keys.append(key_id)
            if key_id not in KEY_REGISTRY:
                _flag(found, "unknown-signing-key")
            elif not KEY_REGISTRY[key_id].get("may_publish"):
                _flag(found, "member-key-as-publisher")
        if len(signing_keys) != len(set(map(str, signing_keys))):
            _flag(found, "publisher-identity-tuple-mismatch")
            _flag(found, "forged-publisher")
    rollback = standard.get("rollback") or {}
    if not isinstance(rollback, dict) or not _require_closed(rollback, ROLLBACK_FIELDS, found, "nested-standard-type", "truncated-bundle"):
        pass
    else:
        tx = rollback.get("transaction")
        if not isinstance(tx, dict) or not _require_closed(tx, ROLLBACK_TX_FIELDS, found, "nested-standard-type"):
            _flag(found, "nested-standard-type")
    if rollback.get("preserves_unrelated_personal_files") is False or rollback.get("safe") is False:
        found.append("rollback-unsafe-standard")
    migration = standard.get("migration") if isinstance(standard.get("migration"), dict) else {}
    if not _require_closed(migration, MIGRATION_FIELDS, found, "nested-standard-type"):
        pass
    if isinstance(revision, int) and isinstance(migration.get("from_revision"), int):
        if revision < migration["from_revision"] and rollback.get("safe") is not True:
            _flag(found, "rollback-version")
    if standard.get("channel") not in CHANNELS:
        _flag(found, "malformed-schema")
    transport = standard.get("transport")
    if not isinstance(transport, dict) or not _require_closed(transport, TRANSPORT_FIELDS, found, "nested-standard-type"):
        _flag(found, "nested-standard-type")
    elif transport.get("kind") != standard.get("channel") or transport.get("local_first") is not True:
        _flag(found, "nested-standard-type")
    lifecycle_meta = standard.get("lifecycle_metadata")
    if not isinstance(lifecycle_meta, dict) or not _require_closed(lifecycle_meta, LIFECYCLE_META_FIELDS, found, "nested-standard-type"):
        _flag(found, "nested-standard-type")
    elif lifecycle_meta.get("state") not in STANDARD_LIFECYCLES or not _is_timestamp(lifecycle_meta.get("published_at")):
        _flag(found, "nested-standard-type")
    if not _is_timestamp(standard.get("expiry")):
        _flag(found, "truncated-bundle")
    manifest = standard.get("content_digest_manifest") or []
    if not isinstance(manifest, list) or not manifest:
        _flag(found, "truncated-bundle")
        _flag(found, "incomplete-manifest")
        return
    bundled: dict[str, str] = {}
    for intent in payload.get("canonical_intents") or []:
        if isinstance(intent, dict) and intent.get("id"):
            bundled[f"canonical_intent_set/{intent['id']}"] = _digest_obj(intent)
    for rule in standard.get("policy_rules") or []:
        if isinstance(rule, dict) and rule.get("id"):
            bundled[f"policy_rules/{rule['id']}"] = _digest_obj(rule)
    recorded: dict[str, str] = {}
    recorded_paths: list[str] = []
    for entry in manifest:
        if not isinstance(entry, dict):
            _flag(found, "nested-standard-type")
            continue
        if not _require_closed(entry, MANIFEST_ENTRY_FIELDS, found, "nested-standard-type"):
            continue
        path = entry.get("path") or ""
        digest = entry.get("digest_sha256")
        if not _is_sha256(digest):
            _flag(found, "swapped-digest")
            _flag(found, "incomplete-manifest")
        recorded_paths.append(str(path))
        recorded[path] = digest
    # The manifest must be a bijection onto the bundled content: no duplicate paths.
    if len(recorded_paths) != len(set(recorded_paths)):
        _flag(found, "incomplete-manifest")
        _flag(found, "swapped-digest")
    if set(bundled) != set(recorded):
        _flag(found, "incomplete-manifest")
        _flag(found, "swapped-digest")
    for path, digest in bundled.items():
        if recorded.get(path) != digest:
            found.append("swapped-digest")
            _flag(found, "incomplete-manifest")


def collect_violations(payload: dict[str, Any], scenario: str | None = None) -> list[str]:
    found: list[str] = []
    scenario = scenario or (payload.get("scenario") if isinstance(payload.get("scenario"), str) else None)
    variants = payload.get("variants") or []
    if variants:
        for variant in variants:
            found.extend(collect_violations(variant, scenario=scenario))
        return sorted(set(found))

    if payload.get("documentation_only"):
        found.append("documentation-only-claim")

    claimed = payload.get("claimed_alignment") or {}
    if claimed.get("method") in {"copy-identical-md", "readme-prose"} and claimed.get("treated_as_semantic_equivalence"):
        found.append("byte-copy-as-alignment")
        if claimed.get("method") == "copy-identical-md":
            found.append("hash-equality-as-equivalence")

    _validate_required_graph(payload, scenario, found)
    graph = SCENARIO_GRAPH.get(scenario or "") or {}

    intent_ids: set[str] = set()
    intents = payload.get("canonical_intents")
    if intents is None or (isinstance(intents, list) and not intents and graph.get("lists", {}).get("canonical_intents")):
        _flag(found, "canonical-intent-empty-collection")
        _flag(found, "canonical-intent-type")
    for intent in intents or []:
        _validate_intent(intent, found, intent_ids)

    targets = family_target_map()
    projections = payload.get("projections") or []
    declared_capabilities = {
        item.get("required_capability")
        for item in (payload.get("canonical_intents") or [])
        if isinstance(item, dict) and _is_nonempty_str(item.get("required_capability"))
    }
    declared_oracles: list[str] = []
    for proj in projections:
        _validate_projection(proj, found, targets, declared_capabilities)
        if isinstance(proj, dict) and isinstance(proj.get("oracle"), dict) and proj["oracle"].get("kind") == "declared-repeatable":
            declared_oracles.append(str(proj.get("family_id")))
    extra_oracles = [fid for fid in declared_oracles if fid not in DECLARED_ORACLE_COMMANDS]
    if extra_oracles or len(set(declared_oracles)) > len(DECLARED_ORACLE_COMMANDS):
        _flag(found, "undeclared-repeatable-oracle")
        _flag(found, "oracle-grammar")

    for binding in payload.get("equivalence_bindings") or []:
        if not isinstance(binding, dict):
            continue
        basis = binding.get("equivalence_basis")
        if basis in FORBIDDEN_EQUIVALENCE_BASES:
            found.append("hash-equality-as-equivalence")
            found.append("byte-copy-as-alignment")
        elif basis not in EQUIVALENCE_BASES:
            found.append("malformed-schema")
            found.append("byte-copy-as-alignment")
        if binding.get("equivalent") and binding.get("byte_identical"):
            found.append("byte-copy-as-alignment")
            found.append("hash-equality-as-equivalence")
        if binding.get("equivalent") and not binding.get("text_hash_equality_is_not_semantic_equivalence", True):
            found.append("hash-equality-as-equivalence")
        families = binding.get("family_ids") or []
        # C-F02 (2026-09-12): byte-identical projections under an equivalent binding
        # are not a violation. Byte identity is not evidence of non-equivalence; the
        # prohibitions above (byte_identical claim, byte/hash equivalence_basis)
        # already forbid using byte identity *as* the equivalence basis.
        bound_families = set(families) if families else set()
        for proj in projections:
            if not isinstance(proj, dict):
                continue
            if bound_families and proj.get("family_id") not in bound_families:
                continue
            outcome = proj.get("projection_outcome")
            negotiation = proj.get("capability_negotiation") or {}
            if not isinstance(negotiation, dict):
                continue
            covering = any(
                _exception_currently_valid(exc)
                for exc in (payload.get("exceptions") or [])
                if isinstance(exc, dict)
            )
            if binding.get("equivalent") and outcome in {"lossy", "unsupported", "unknown"} and not covering:
                found.append("lossy-without-exception-marked-equivalent")
            if binding.get("equivalent") and negotiation.get("dropped") and outcome in {"exact", "native-equivalent", "transformed"}:
                found.append("lossy-without-exception-marked-equivalent")

    # C-F02 (2026-09-12): the former "all four anchor projections byte-identical =>
    # byte-copy-as-alignment" assertion was removed. Byte identity across harnesses is
    # permitted; only claiming byte/hash identity *as* the equivalence basis fails.

    overlays = payload.get("overlays") or []
    for overlay in overlays:
        _validate_overlay(overlay, payload, found)

    assignments = payload.get("layer_assignments") or []
    _compute_layer_violations(assignments, found)
    if graph.get("require_policy_required"):
        folded = _fold_layers(assignments if isinstance(assignments, list) else [])
        if folded.get(POLICY_ID, {}).get("mode") != "required":
            _flag(found, "required-downgraded-to-recommended")
            _flag(found, "required-scenario-coverage")
    covered_families: list[str] = []
    for row in payload.get("effective_policies") or []:
        if not isinstance(row, dict):
            _flag(found, "malformed-schema")
            continue
        extra = set(row) - set(EFFECTIVE_POLICY_FIELDS)
        missing = set(EFFECTIVE_POLICY_FIELDS) - set(row)
        if extra or missing:
            _flag(found, "malformed-schema")
        if row.get("shadows_higher_required"):
            found.append("personal-shadows-required")
        family_id = row.get("family_id")
        if not _is_nonempty_str(family_id):
            _flag(found, "malformed-schema")
            continue
        covered_families.append(str(family_id))
        expected = recompute_effective_row(payload, str(family_id))
        if row.get("honesty") == "enforceable" and expected.get("honesty") != "enforceable":
            _flag(found, "unmanaged-marked-enforceable")
            _flag(found, "false-enforcement-claim")
        if row.get("computed_via") != expected.get("computed_via") or row.get("algorithm") != expected.get("algorithm"):
            _flag(found, "computed-via-mismatch")
            _flag(found, "effective-policy-mismatch")
        for key in EFFECTIVE_POLICY_FIELDS:
            if row.get(key) != expected.get(key):
                _flag(found, "effective-policy-mismatch")
                if key in {"mode", "honesty"}:
                    _flag(found, "false-enforcement-claim")
                break
        if row.get("honesty") not in ENFORCEMENT_HONESTY:
            _flag(found, "malformed-schema")
        if row.get("honesty") == "enforceable" and family_id in CONNECTOR_FAMILIES:
            _flag(found, "unmanaged-marked-enforceable")
            _flag(found, "false-enforcement-claim")
    if graph.get("lists", {}).get("effective_policies") and set(covered_families) != set(REQUIRED_FOUR):
        _flag(found, "required-scenario-coverage")

    _validate_team_standard(payload.get("team_standard"), payload, intent_ids, found)

    # Policy evaluations are a closed schema bound to declared rules and exceptions.
    standard_obj = payload.get("team_standard") if isinstance(payload.get("team_standard"), dict) else {}
    known_policy_ids = {
        item.get("id")
        for item in (standard_obj.get("policy_rules") or [])
        if isinstance(item, dict)
    } or {POLICY_ID}
    known_exception_ids = {
        item.get("id")
        for item in (payload.get("exceptions") or [])
        if isinstance(item, dict)
    }
    for eval_row in payload.get("policy_evals") or []:
        if not isinstance(eval_row, dict) or not _require_closed(eval_row, POLICY_EVAL_FIELDS, found, "malformed-schema"):
            _flag(found, "malformed-schema")
            _flag(found, "required-scenario-coverage")
            continue
        if eval_row.get("policy_id") not in known_policy_ids:
            _flag(found, "exception-policy-mismatch")
            _flag(found, "malformed-schema")
        if eval_row.get("mode") not in ENFORCEMENT_MODES:
            _flag(found, "malformed-schema")
            _flag(found, "required-downgraded-to-recommended")
        if eval_row.get("result") not in POLICY_EVAL_RESULTS:
            _flag(found, "malformed-schema")
            _flag(found, "false-enforcement-claim")
        if not isinstance(eval_row.get("inspection_export_allowed"), bool):
            _flag(found, "malformed-schema")
        covered = eval_row.get("covered_by_exception")
        if covered is not None:
            if covered not in known_exception_ids:
                _flag(found, "exception-scope-mismatch")
                _flag(found, "malformed-schema")
        elif eval_row.get("mode") == "required" and eval_row.get("result") == "pass":
            # A required rule cannot report pass without a named covering exception.
            _flag(found, "false-enforcement-claim")
            _flag(found, "required-scenario-coverage")

    eval_time = _parse_ts(EVALUATION_TIME)
    overlay_digests = {
        item.get("digest")
        for item in overlays
        if isinstance(item, dict) and item.get("digest")
    }
    for exc in payload.get("exceptions") or []:
        _validate_exception(exc, found)
        if not isinstance(exc, dict):
            continue
        if overlay_digests and exc.get("overlay_digest") not in overlay_digests and exc.get("overlay_digest") != sha256_text("none"):
            _flag(found, "exception-overlay-digest-mismatch")
        freshness = exc.get("freshness")
        state = exc.get("state")
        stamps = exc.get("timestamps") if isinstance(exc.get("timestamps"), dict) else {}
        expires = _parse_ts(stamps.get("expires_at"))
        for eval_row in payload.get("policy_evals") or []:
            if eval_row.get("covered_by_exception") != exc.get("id"):
                continue
            if eval_row.get("mode") == "required" and eval_row.get("result") == "pass":
                if freshness == "expired" or state == "expiry" or (eval_time and expires and expires <= eval_time):
                    found.append("expired-exception-pass")
                if freshness == "stale-offline":
                    found.append("stale-offline-exception-pass")
                if freshness == "revoked" or state == "revocation":
                    found.append("revoked-exception-pass")
                if freshness not in FRESHNESS_STATES:
                    _flag(found, "invalid-freshness")
                    found.append("expired-exception-pass")
                if state in {"request", "rejection"}:
                    # A request that was never approved, or an outright rejection,
                    # cannot license a required rule to report pass.
                    found.append("unbounded-exception")
                    found.append("false-enforcement-claim")
                if not _exception_currently_valid(exc):
                    if state == "approval" and freshness == "current":
                        found.append("expired-exception-pass")
                    elif freshness not in {"expired", "stale-offline", "revoked"} and state == "approval":
                        found.append("expired-exception-pass")
                    elif state not in {"request", "rejection"}:
                        found.append("expired-exception-pass")

    view = payload.get("leader_view")
    if isinstance(view, dict):
        extra_leader = sorted(set(view) - set(ALLOWED_LEADER_FIELDS) - FORBIDDEN_LEADER_FIELDS)
        if extra_leader:
            found.append("malformed-schema")
        _walk_leader_privacy(view, found, top=True)

    _validate_disclosure(payload, found)
    _validate_lifecycle(payload, found, required=bool((graph.get("objects") or {}).get("lifecycle")))

    expected_bound = _expected_bound_digests(payload, [item for item in projections if isinstance(item, dict)])
    receipt_ids: list[str] = []
    for receipt in payload.get("receipts") or []:
        if not isinstance(receipt, dict):
            continue
        receipt_ids.append(str(receipt.get("id")))
        extra_r = set(receipt) - set(RECEIPT_REQUIRED_FIELDS)
        missing_r = set(RECEIPT_REQUIRED_FIELDS) - set(receipt)
        if extra_r or missing_r:
            _flag(found, "malformed-schema")
        if receipt.get("reconciliation_state") not in CLAIM_RECONCILIATION_STATES:
            _flag(found, "malformed-schema")
            _flag(found, "projection-outcome-is-reconciliation-state")
        if not isinstance(receipt.get("native_result_captured"), bool):
            _flag(found, "malformed-schema")
        if receipt.get("reconciliation_state") == "verified" and not receipt.get("native_result_captured"):
            found.append("unknown-marked-verified")
            found.append("verified-without-oracle")
        if receipt.get("native_result_captured") is True:
            # A captured native result requires a declared repeatable oracle behind it.
            captured_families = [
                item.get("family_id")
                for item in projections
                if isinstance(item, dict)
                and isinstance(item.get("oracle"), dict)
                and item["oracle"].get("native_result_captured") is True
            ]
            if not any(_declared_oracle(fid) for fid in captured_families):
                found.append("verified-without-oracle")
        bound = receipt.get("bound_digests")
        if not isinstance(bound, dict) or not bound:
            found.append("empty-round-trip-bindings")
            found.append("missing-round-trip-digest")
            continue
        missing_keys = [key for key in ROUND_TRIP_DIGEST_KEYS if key not in bound]
        empty_keys = [key for key in ROUND_TRIP_DIGEST_KEYS if not _is_sha256(bound.get(key))]
        extra_keys = sorted(set(bound) - set(ROUND_TRIP_DIGEST_KEYS))
        if extra_keys:
            found.append("malformed-schema")
        if missing_keys or empty_keys:
            found.append("missing-round-trip-digest")
            if not bound or empty_keys == ROUND_TRIP_DIGEST_KEYS:
                found.append("empty-round-trip-bindings")
        for key in ROUND_TRIP_DIGEST_KEYS:
            if key in bound and bound.get(key) != expected_bound.get(key):
                found.append("bound-digest-mismatch")
                break
        if receipt.get("bound_digest_manifest") != _digest_obj(bound):
            found.append("bound-digest-mismatch")
    if receipt_ids and len(receipt_ids) != len(set(receipt_ids)):
        found.append("bound-digest-mismatch")

    rollback_plan = payload.get("rollback_plan") or {}
    if rollback_plan.get("preserves_unrelated_personal_files") is False:
        found.append("rollback-deletes-personal")
    deletes = rollback_plan.get("deletes") or []
    if any(str(item).startswith("personal/") for item in deletes):
        found.append("rollback-deletes-personal")

    trust = payload.get("trust_state") or {}
    if trust.get("silent_merge") and trust.get("equivalent"):
        found.append("fork-conflict-silent-merge")
    if trust.get("publisher_key_revoked") and trust.get("still_trusted"):
        found.append("revoked-key-still-trusted")

    return sorted(set(found))


def iter_payloads(fixture: dict[str, Any]) -> list[dict[str, Any]]:
    payload = fixture.get("payload") or {}
    variants = payload.get("variants") or []
    if variants:
        return list(variants)
    return [payload]


def build_contract(content_digest_manifest: list[dict[str, str]]) -> dict[str, Any]:
    mapping = {item: item for item in REQUIRED_SCENARIOS + REQUIRED_MALFORMED}
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": SEMANTIC_TEAM_ARTIFACT,
        "license": LICENSE_ID,
        "live_tested": False,
        "projection_outcomes": list(PROJECTION_OUTCOMES),
        "loss_classes": list(LOSS_CLASSES),
        "drift_classes": list(DRIFT_CLASSES),
        "enforcement_modes": list(ENFORCEMENT_MODES),
        "enforcement_honesty": list(ENFORCEMENT_HONESTY),
        "layer_order": list(LAYER_ORDER),
        "reconciliation_states": list(CLAIM_RECONCILIATION_STATES),
        "pipeline_steps": list(PIPELINE_STEPS),
        "exception_states": list(EXCEPTION_STATES),
        "standard_lifecycles": list(STANDARD_LIFECYCLES),
        "required_scenarios": list(REQUIRED_SCENARIOS),
        "required_malformed_fixtures": list(REQUIRED_MALFORMED),
        "fixture_id_mapping": mapping,
        "canonical_intent_fields": list(CANONICAL_INTENT_FIELDS),
        "team_standard_fields": list(TEAM_STANDARD_FIELDS),
        "family_native_target_fields": list(FAMILY_NATIVE_TARGET_FIELDS),
        "family_native_targets": family_native_targets(),
        "round_trip_digest_keys": list(ROUND_TRIP_DIGEST_KEYS),
        "publisher_fields": list(PUBLISHER_FIELDS),
        "synthetic_signing": {
            "alg": "hmac-sha256-synthetic",
            "key_ids": sorted(SYNTHETIC_KEYS),
            "live_tested": False,
            "note": "offline synthetic fixture keys; not user tokens",
        },
        "content_digest_manifest": content_digest_manifest,
        "notes": [
            "Keep the original eight PRD §17.0 artifacts. This contract is an additional generated artifact.",
            "Fixture files under acceptance/semantic-team/ are hashed here (bijection). The contract itself is hashed in acceptance/artifact-digest-manifest.json.",
            "Projection outcomes are orthogonal to reconciliation states verified|structural-only|indeterminate|failed.",
            "Text/hash equality is evidence of identical files only, never semantic equivalence.",
            "Encrypted cloud is optional transport for TeamContextStandard; git/file distribution is local-first.",
            "Declared repeatable oracles remain Codex debug prompt-input and Grok inspect --json. This stage does not capture native results.",
        ],
    }


def build_semantic_team_files() -> dict[str, str]:
    files: dict[str, str] = {}
    for fixture in all_fixtures():
        rel = f"{SEMANTIC_TEAM_DIR}/{fixture['id']}.json"
        files[rel] = dump_json_yaml(fixture)
    manifest = [
        {"path": path, "digest_sha256": sha256_text(text)}
        for path, text in sorted(files.items())
    ]
    contract = build_contract(manifest)
    files[SEMANTIC_TEAM_CONTRACT] = dump_json_yaml(contract)
    return files


def write_semantic_team_artifacts(root: Any) -> dict[str, str]:
    files = build_semantic_team_files()
    team_dir = root / SEMANTIC_TEAM_DIR
    if team_dir.exists():
        shutil.rmtree(team_dir)
    for rel, text in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return files


def validate_contract_object(contract: dict[str, Any], errors: list[str]) -> None:
    require_closed_object(
        contract,
        CONTRACT_ALLOWED,
        CONTRACT_REQUIRED,
        "semantic-team-contract",
        errors,
        nonempty=["artifact", "cutoff", "required_scenarios", "content_digest_manifest"],
    )
    if contract.get("artifact") != SEMANTIC_TEAM_ARTIFACT:
        errors.append(
            f"semantic-team-contract artifact identity {contract.get('artifact')!r} != {SEMANTIC_TEAM_ARTIFACT!r}"
        )
    if contract.get("cutoff") != CUTOFF:
        errors.append("semantic-team-contract cutoff mismatch")
    if contract.get("live_tested") is not False:
        errors.append("semantic-team-contract live_tested must be false")
    if contract.get("license") != LICENSE_ID:
        errors.append("semantic-team-contract license must be Apache-2.0")
    if contract.get("projection_outcomes") != PROJECTION_OUTCOMES:
        errors.append("semantic-team-contract projection_outcomes does not match the locked enum")
    if contract.get("reconciliation_states") != list(CLAIM_RECONCILIATION_STATES):
        errors.append("semantic-team-contract must keep CLAIM_RECONCILIATION_STATES exactly")
    if contract.get("layer_order") != LAYER_ORDER:
        errors.append("semantic-team-contract layer_order must be organization>team>project>role>personal")
    if contract.get("pipeline_steps") != PIPELINE_STEPS:
        errors.append("semantic-team-contract pipeline_steps order mismatch")
    if "verified" in (contract.get("projection_outcomes") or []):
        errors.append("projection outcome enum must not include reconciliation state verified")
    targets = contract.get("family_native_targets") or []
    got = [row.get("family_id") for row in targets if isinstance(row, dict)]
    missing = sorted(set(family_ids()) - set(got))
    extra = sorted(set(got) - set(family_ids()))
    if missing:
        errors.append(f"family_native_targets missing families {missing}")
    if extra:
        errors.append(f"family_native_targets extra families {extra}")
    if set(got) != set(ANCHOR_IDS) and not missing:
        # all families required, including expansion
        pass
    if contract.get("required_scenarios") != REQUIRED_SCENARIOS:
        errors.append("required_scenarios must match the locked order (ST1-pos..ST8-neg plus ST2-same-bytes-pos)")
    if contract.get("required_malformed_fixtures") != REQUIRED_MALFORMED:
        errors.append("required_malformed_fixtures missing dedicated forged/partial/malformed cases")
    if contract.get("family_native_target_fields") != FAMILY_NATIVE_TARGET_FIELDS:
        errors.append("family_native_target_fields does not match the locked grammar")
    if contract.get("round_trip_digest_keys") != ROUND_TRIP_DIGEST_KEYS:
        errors.append("round_trip_digest_keys does not match the locked eight-way binding")
    if contract.get("publisher_fields") != PUBLISHER_FIELDS:
        errors.append("publisher_fields does not match the locked publisher grammar")
    for row in contract.get("family_native_targets") or []:
        if not isinstance(row, dict):
            errors.append("family_native_targets entry is not an object")
            continue
        missing = [key for key in FAMILY_NATIVE_TARGET_FIELDS if key not in row]
        if missing:
            errors.append(f"{row.get('family_id')} native target missing {missing}")
        if not isinstance(row.get("primitives"), list) or not row.get("primitives"):
            errors.append(f"{row.get('family_id')} native primitives must be a non-empty list")
        if not isinstance(row.get("managed_channel"), bool):
            errors.append(f"{row.get('family_id')} managed_channel must be a bool")
