#!/usr/bin/env python3
"""Shared Contexpect foundation-stage contract helpers.

No third-party dependencies. YAML artifacts are JSON-compatible YAML
(RFC 8259 JSON subset) so the Python standard library can validate them.
"""

from __future__ import annotations

import csv
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Iterable

from contexpect_freeze import (  # noqa: E402
    ACCESS_DATE,
    AIDER,
    CLINE,
    COZE,
    DEEPSEEK_HARNESS,
    DIGEST_NOT_PUBLISHED,
    INTEGRATION_SOURCES,
    NATIVE_TARGETS,
    OPENHANDS,
    UBUNTU_24_04,
    WINDOWS_11_24H2,
    WINDSURF,
)

CUTOFF = "2026-09-04T23:59:59+08:00"
SCHEMA_VERSION = 1
SUPPORTED_SCHEMA_VERSIONS = frozenset({SCHEMA_VERSION})
LICENSE_ID = "Apache-2.0"
ALLOWED_SENSITIVITY = frozenset({"public-synthetic", "withheld", "redacted-recipe"})
ALLOWED_CAPABILITY_STATUS = frozenset(
    {"required-supported", "required-unknown-honesty", "not-applicable"}
)
ALLOWED_COVERAGE = frozenset(
    {"full-declared-surface", "partial-declared-surface", "unknown", "not-applicable"}
)
ALLOWED_PROVENANCE = frozenset(
    {"native-runtime", "native-log", "harness-source", "official-spec"}
)
ALLOWED_COLLECTORS = frozenset({"static-resolver", None})
ALLOWED_IMPORTERS = frozenset(
    {"codex-debug-prompt-input", "grok-inspect-json", "unknown-honesty", None}
)
CAPABILITY_CELL_REQUIRED_FIELDS = [
    "id",
    "family_id",
    "version",
    "surface",
    "os_lane",
    "capability_id",
    "status",
    "collector",
    "importer",
    "field_mapping",
    "provenance",
    "coverage",
    "compatibility_coordinate_id",
]
JSONL_RECORD_REQUIRED_FIELDS = [
    "id",
    "schema_version",
    "corpus",
    "kind",
    "coordinate_id",
    "license",
    "sensitivity",
    "live_tested",
    "digest",
]
EXPORT_ONLY_MARKERS = ("export-only",)
PRD_RELPATH = "docs/requirements/2026-09-04-contexpect-complete-product-requirements.md"
CANONICAL_STATUS = "规范（尚未实施产品运行时）"

ROOT_OSS_FILES = [
    "README.md",
    "LICENSE",
    "NOTICE",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "SECURITY.md",
    "GOVERNANCE.md",
    "SUPPORT.md",
    ".gitignore",
    "AGENTS.md",
]

CANONICAL_DOCS = [
    "docs/README.md",
    "docs/architecture/overview.md",
    "docs/architecture/data-and-truth-model.md",
    "docs/architecture/semantic-alignment-and-team-standard.md",
    "docs/security/privacy-and-threat-model.md",
    "docs/security/encrypted-sync-protocol.md",
    "docs/adapters/architecture.md",
    "docs/adapters/authoring-guide.md",
    "docs/guides/user-guide.md",
    "docs/guides/cli-reference.md",
    "docs/guides/desktop-ui.md",
    "docs/guides/configuration.md",
    "docs/guides/operations.md",
    "docs/guides/llm-advisor-and-effect-lab.md",
    "docs/process/release.md",
    "docs/process/test-strategy.md",
    "docs/process/implementation-plan.md",
    "docs/process/dependency-and-provenance.md",
    "docs/adr/0001-rust-tauri-react-sqlite.md",
    "docs/adr/0002-trust-boundaries.md",
    "docs/adr/0003-encrypted-sync-and-signing.md",
    "docs/adr/0004-adapter-isolation.md",
]

ACCEPTANCE_ARTIFACTS = [
    "acceptance/compatibility-matrix.yaml",
    "acceptance/corpus-manifest.json",
    "acceptance/claim-validity-matrix.yaml",
    "acceptance/context-capability-matrix.yaml",
    "acceptance/projection-matrix.yaml",
    "acceptance/traceability.csv",
    "acceptance/reference-hardware.md",
    "acceptance/integration-contracts.yaml",
]

ARTIFACT_DIGEST_MANIFEST = "acceptance/artifact-digest-manifest.json"

ARTIFACT_SCHEMA_IDS = {
    "acceptance/compatibility-matrix.yaml": "compatibility-matrix",
    "acceptance/corpus-manifest.json": "corpus-manifest",
    "acceptance/claim-validity-matrix.yaml": "claim-validity-matrix",
    "acceptance/context-capability-matrix.yaml": "context-capability-matrix",
    "acceptance/projection-matrix.yaml": "projection-matrix",
    "acceptance/traceability.csv": "traceability",
    "acceptance/reference-hardware.md": "reference-hardware",
    "acceptance/integration-contracts.yaml": "integration-contracts",
    "acceptance/semantic-team-contract.yaml": "semantic-team-contract",
}

ARTIFACT_REQUIRED_TOP_LEVEL = {
    "compatibility-matrix": [
        "schema_version",
        "cutoff",
        "artifact",
        "os_lanes",
        "families",
        "declared_native_oracles",
        "coordinates",
        "honesty",
        "source_backed_os",
    ],
    "claim-validity-matrix": [
        "schema_version",
        "cutoff",
        "artifact",
        "axes",
        "unknown_reason_codes",
        "llm_suggestion_in_provenance",
        "invariants",
        "precedence",
        "use_evidence_kinds",
        "reconciliation_states",
        "reconciliation_invariants",
        "legal_examples",
        "illegal_examples",
    ],
    "context-capability-matrix": [
        "schema_version",
        "cutoff",
        "artifact",
        "capabilities",
        "cells",
        "notes",
    ],
    "projection-matrix": [
        "schema_version",
        "cutoff",
        "artifact",
        "cells",
        "required_write_floor",
        "gates_per_required_write_cell",
        "authorities",
        "unique_authority_rule",
    ],
    "integration-contracts": [
        "schema_version",
        "cutoff",
        "artifact",
        "contracts",
        "borrow_only_forbidden",
    ],
    "corpus-manifest": [
        "schema_version",
        "cutoff",
        "artifact",
        "license",
        "corpora",
        "files",
        "fixtures",
        "counts",
        "non_claims",
    ],
    "artifact-digest-manifest": [
        "schema_version",
        "cutoff",
        "artifact",
        "artifacts",
        "notes",
    ],
}

# Closed schemas: allowed keysets equal required keysets plus nested optionals below.
ARTIFACT_ALLOWED_TOP_LEVEL = {
    name: list(fields) for name, fields in ARTIFACT_REQUIRED_TOP_LEVEL.items()
}

COORDINATE_ALLOWED_FIELDS = [
    "id",
    "family_id",
    "family_name",
    "cohort",
    "version",
    "version_source",
    "surface",
    "os_lane",
    "static_support",
    "native_support",
    "live_install",
    "native_oracle_ids",
    "fabricated_native_evidence",
    "notes",
    "source_url",
]

LIVE_INSTALL_ALLOWED_FIELDS = [
    "executable_present",
    "app_present",
    "config_residue_present",
    "authenticated",
    "connector_ready",
    "live_status",
    "unknown_reason",
]

LIVE_INSTALL_REQUIRED_FIELDS = [
    "executable_present",
    "app_present",
    "config_residue_present",
    "authenticated",
    "connector_ready",
    "live_status",
]

CAPABILITY_CELL_ALLOWED_FIELDS = [
    "id",
    "family_id",
    "version",
    "surface",
    "os_lane",
    "capability_id",
    "capability_index",
    "status",
    "collector",
    "importer",
    "field_mapping",
    "provenance",
    "coverage",
    "compatibility_coordinate_id",
]

PROJECTION_CELL_ALLOWED_FIELDS = [
    "id",
    "family_id",
    "family_name",
    "asset",
    "scope",
    "mode",
    "authority",
    "authority_version",
    "native_target",
    "loss_contract",
    "loss_report",
    "concurrency_guard",
    "preview",
    "apply",
    "rollback",
    "post_receipt",
    "required_write",
    "extra",
]

NATIVE_TARGET_ALLOWED_FIELDS = ["kind", "family_id", "surface", "path_glob", "scope"]
NATIVE_TARGET_REQUIRED_FIELDS = ["kind", "family_id", "path_glob"]

DIGEST_ENTRY_REQUIRED_FIELDS = ["path", "digest_sha256", "schema_id"]

CLAIM_AXES = {
    "claim_kind": ["resolved", "observed", "effect"],
    "lifecycle_stage": [
        "installed",
        "discoverable",
        "eligible",
        "model-visible",
        "use-evidence",
        "outcome-affecting",
    ],
    "truth_state": ["present", "absent", "indeterminate", "not-applicable"],
    "provenance": [
        "native-runtime",
        "native-log",
        "harness-source",
        "official-spec",
        "heuristic",
        "user-attested",
    ],
    "coverage": ["full-declared-surface", "partial-declared-surface", "unknown"],
    "precision": ["exact", "derived", "estimated", "not-applicable"],
    "knowledge_status": ["current", "stale", "conflicted", "unknown"],
}

CLAIM_PRECEDENCE = [
    "native-runtime",
    "native-log",
    "harness-source",
    "official-spec",
    "heuristic",
    "user-attested",
]

CLAIM_USE_EVIDENCE_KINDS = [
    "invocation-observed",
    "reference-observed",
    "behavior-consistent",
    "internal-attribution",
]

CLAIM_INVARIANTS = [
    {
        "id": "resolved-no-present-runtime-facets",
        "if": {
            "claim_kind": "resolved",
            "lifecycle_stage": ["model-visible", "use-evidence", "outcome-affecting"],
        },
        "forbidden_truth_state": ["present"],
        "allowed_truth_state": ["indeterminate", "not-applicable"],
    },
    {
        "id": "model-visible-present-requires-native",
        "if": {"lifecycle_stage": "model-visible", "truth_state": "present"},
        "required_provenance": ["native-runtime", "native-log"],
        "minimum_coverage": "partial-declared-surface",
    },
    {
        "id": "internal-attribution-never-present-without-native-semantics",
        "if": {"use_evidence_kind": "internal-attribution"},
        "forbidden_truth_state": ["present"],
        "default_truth_state": "indeterminate",
    },
    {
        "id": "outcome-affecting-decision-from-experiment-only",
        "if": {
            "lifecycle_stage": "outcome-affecting",
            "decision": [
                "supported-beneficial",
                "supported-harmful",
                "supported-equivalent-within-margin",
            ],
        },
        "required_claim_kind": "effect",
        "required_fields": ["experiment_id", "contract_digest"],
    },
    {
        "id": "inconclusive-is-not-absent",
        "if": {"decision": "inconclusive"},
        "forbidden_truth_state": ["absent"],
    },
    {
        "id": "unexposed-timeline-no-fake-events",
        "if": {"capability_unexposed": True},
        "required_truth_state": ["indeterminate"],
        "forbid_synthetic_events": True,
    },
    {
        "id": "conflict-keep-conflicted",
        "if": {"two_current_same_coverage": True, "contradictory": True},
        "required_knowledge_status": "conflicted",
        "silent_choice": False,
    },
    {
        "id": "higher-provenance-cannot-fill-uncovered-fields",
        "if": {"higher_provenance_partial": True},
        "cannot_override": "fields_outside_its_coverage",
    },
    {
        "id": "absent-requires-sufficient-coverage",
        "if": {"truth_state": "absent"},
        "forbidden_coverage": ["unknown"],
    },
]

CLAIM_INVARIANT_IDS = [item["id"] for item in CLAIM_INVARIANTS]

CLAIM_RECONCILIATION_STATES = ["verified", "structural-only", "indeterminate", "failed"]
CLAIM_RECONCILIATION_INVARIANTS = [
    "structural-only and indeterminate must never be displayed as verified",
    "algorithm stops-and-keeps-unrun-steps; no total score masking",
]

INTEGRATION_CANONICAL_IDS = list(INTEGRATION_SOURCES.keys())

COORDINATE_REQUIRED_FIELDS = [
    "id",
    "family_id",
    "family_name",
    "cohort",
    "version",
    "version_source",
    "surface",
    "os_lane",
    "static_support",
    "native_support",
    "live_install",
    "native_oracle_ids",
    "fabricated_native_evidence",
]

PROJECTION_REQUIRED_WRITE_FIELDS = [
    "id",
    "family_id",
    "asset",
    "scope",
    "mode",
    "authority",
    "authority_version",
    "native_target",
    "loss_contract",
    "loss_report",
    "concurrency_guard",
    "preview",
    "apply",
    "rollback",
    "post_receipt",
    "required_write",
]

CORPUS_FIXTURE_REQUIRED_FIELDS = [
    "id",
    "schema_version",
    "corpus",
    "kind",
    "path",
    "digest_sha256",
    "license",
    "sensitivity",
    "coordinate_id",
    "live_tested",
]

# Manifest fixture ↔ JSONL record identity. Compared exactly when required
# for the fixture class; missing on either side is a failure, not a skip.
CORPUS_CROSS_LINK_FIELDS = [
    "id",
    "coordinate_id",
    "kind",
    "class",
    "family_id",
    "version",
    "surface",
    "os_lane",
    "capability_id",
    "oracle_id",
    "command",
    "input_path",
    "input_digest_sha256",
    "input_files",
    "expected_native_shape",
]

CORPUS_DEVELOPMENT_INPUT_FIELDS = ["input_path", "input_digest_sha256", "input_files"]
CORPUS_DEVELOPMENT_COORDINATE_FIELDS = ["family_id", "version", "surface", "os_lane"]
CORPUS_DEVELOPMENT_STATIC_ORACLE_FIELDS = CORPUS_DEVELOPMENT_COORDINATE_FIELDS + [
    "capability_id",
    "class",
] + CORPUS_DEVELOPMENT_INPUT_FIELDS
CORPUS_DEVELOPMENT_ORACLE_EXTRA_FIELDS = ["oracle_id", "command", "expected_native_shape"]
CORPUS_DEVELOPMENT_DOCTOR_FIELDS = ["class"] + CORPUS_DEVELOPMENT_INPUT_FIELDS

FIELD_MAP_ARTIFACT = "field-to-claim"
FIELD_MAP_REQUIRED_TOP_LEVEL = [
    "schema_version",
    "cutoff",
    "artifact",
    "family_id",
    "surface",
    "version",
    "live_tested",
    "fields",
]
FIELD_MAP_ALLOWED_TOP_LEVEL = list(FIELD_MAP_REQUIRED_TOP_LEVEL)
FIELD_MAP_FIELD_REQUIRED = [
    "native_field",
    "lifecycle_stage",
    "claim_kind",
    "coverage",
    "minimum_evidence",
]
FIELD_MAP_FIELD_ALLOWED = FIELD_MAP_FIELD_REQUIRED + ["notes", "precision"]
FIELD_MAP_IDENTITY_FIELDS = ["family_id", "surface", "version", "cutoff", "live_tested"]

INTEGRATION_CONTRACT_REQUIRED_FIELDS = [
    "name",
    "integration_mode",
    "version_pin",
    "version_pin_honesty",
    "pin_evidence",
    "capability",
    "unavailable_degrade",
    "license",
    "notes",
]

ALLOWED_OWNERS = frozenset({"product-core", "security", "architecture", "acceptance", "governance"})
ALLOWED_LIVE_STATUS = frozenset(
    {
        "installed",
        "installed-auth-unhealthy",
        "not-installed",
        "connector-required",
        "unknown",
        "official-image-frozen",
        "official-build-frozen",
        "captured",
    }
)

SECTION_SEMANTICS = {
    "0": {
        "wps": ["WP-01"],
        "artifact": "docs/requirements/2026-09-04-contexpect-complete-product-requirements.md",
        "owner": "architecture",
        "gate": "docs-structure; acceptance-validation; traceability-validation",
    },
    "1": {
        "wps": ["WP-01"],
        "artifact": "docs/architecture/overview.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "2": {
        "wps": ["WP-01"],
        "artifact": "docs/architecture/overview.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "3": {
        "wps": ["WP-01"],
        "artifact": "docs/architecture/data-and-truth-model.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "4": {
        "wps": ["WP-01"],
        "artifact": "acceptance/claim-validity-matrix.yaml; acceptance/context-capability-matrix.yaml",
        "owner": "product-core",
        "gate": "claim tuple accuracy; native oracle reconciliation",
    },
    "5": {
        "wps": ["WP-01", "WP-04"],
        "artifact": "docs/guides/user-guide.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "6": {
        "wps": ["WP-02", "WP-04", "WP-05"],
        "artifact": "docs/guides/user-guide.md; docs/guides/operations.md",
        "owner": "product-core",
        "gate": "docs-structure; daemon/CI gates in WP-05/WP-11",
    },
    "7": {
        "wps": ["WP-02", "WP-04", "WP-05"],
        "artifact": "docs/guides/user-guide.md",
        "owner": "product-core",
        "gate": "docs-structure",
    },
    "8": {
        "wps": ["WP-01"],
        "artifact": "docs/process/implementation-plan.md; acceptance/",
        "owner": "product-core",
        "gate": "docs-structure; acceptance-validation",
    },
    "9": {
        "wps": ["WP-04"],
        "artifact": "docs/guides/desktop-ui.md",
        "owner": "product-core",
        "gate": "internal usability gate; WCAG audit; privacy screenshot mode",
    },
    "10": {
        "wps": ["WP-01", "WP-03"],
        "artifact": "docs/architecture/data-and-truth-model.md; acceptance/claim-validity-matrix.yaml",
        "owner": "product-core",
        "gate": "schema/API compatibility; identity corpus",
    },
    "11": {
        "wps": ["WP-01", "WP-02"],
        "artifact": "acceptance/compatibility-matrix.yaml; docs/adapters/architecture.md",
        "owner": "product-core",
        "gate": "compatibility matrix; environment fixtures; unknown-version exit 3",
    },
    "12": {
        "wps": ["WP-01", "WP-02"],
        "artifact": "docs/architecture/overview.md; docs/adr/0001-rust-tauri-react-sqlite.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "13": {
        "wps": ["WP-01", "WP-07", "WP-11"],
        "artifact": "docs/security/privacy-and-threat-model.md; docs/security/encrypted-sync-protocol.md",
        "owner": "security",
        "gate": "security/privacy gates in WP-07/WP-11/WP-12",
    },
    "14": {
        "wps": ["WP-12"],
        "artifact": "acceptance/reference-hardware.md; docs/process/test-strategy.md",
        "owner": "acceptance",
        "gate": "performance/reliability gates in WP-12",
    },
    "15": {
        "wps": ["WP-01", "WP-08"],
        "artifact": "acceptance/integration-contracts.yaml; docs/process/dependency-and-provenance.md",
        "owner": "product-core",
        "gate": "integration contract; license/provenance/SBOM",
    },
    "16": {
        "wps": ["WP-01"],
        "artifact": "docs/process/implementation-plan.md",
        "owner": "product-core",
        "gate": "docs-structure; WP coverage",
    },
    "17": {
        "wps": ["WP-01", "WP-12"],
        "artifact": "acceptance/",
        "owner": "acceptance",
        "gate": "docs-structure; acceptance-validation; traceability-validation; corpus-validation",
    },
    "18": {
        "wps": ["WP-12"],
        "artifact": "docs/process/test-strategy.md",
        "owner": "acceptance",
        "gate": "full-product acceptance in WP-12",
    },
    "19": {
        "wps": ["WP-01"],
        "artifact": "docs/architecture/overview.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "20": {
        "wps": ["WP-01"],
        "artifact": "docs/architecture/overview.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
    "21": {
        "wps": ["WP-01"],
        "artifact": "docs/process/release.md; GOVERNANCE.md",
        "owner": "governance",
        "gate": "docs-structure",
    },
    "22": {
        "wps": ["WP-01"],
        "artifact": "README.md; GOVERNANCE.md",
        "owner": "governance",
        "gate": "docs-structure",
    },
    "23": {
        "wps": ["WP-01"],
        "artifact": "docs/architecture/overview.md",
        "owner": "architecture",
        "gate": "docs-structure",
    },
}

FEATURES = [f"F-{i:02d}" for i in range(1, 19)]
WORK_PACKAGES = [f"WP-{i:02d}" for i in range(1, 13)]

F_TO_WP = {
    "F-01": ["WP-01", "WP-02"],
    "F-02": ["WP-01", "WP-02"],
    "F-03": ["WP-01", "WP-02"],
    "F-04": ["WP-02", "WP-04", "WP-05"],
    "F-05": ["WP-01", "WP-03"],
    "F-06": ["WP-04"],
    "F-07": ["WP-03", "WP-04"],
    "F-08": ["WP-02", "WP-08"],
    "F-09": ["WP-06"],
    "F-10": ["WP-07"],
    "F-11": ["WP-01", "WP-08"],
    "F-12": ["WP-05"],
    "F-13": ["WP-09"],
    "F-14": ["WP-09"],
    "F-15": ["WP-10"],
    "F-16": ["WP-05", "WP-11"],
    "F-17": ["WP-01", "WP-11"],
    "F-18": ["WP-01", "WP-11"],
}

F_TO_GATE = {
    "F-01": "compatibility matrix; environment fixtures; unknown-version exit 3",
    "F-02": "corpus discovery F1; filesystem/security negatives",
    "F-03": "claim tuple accuracy; native oracle reconciliation",
    "F-04": "task probe golden; shell/env injection negatives; preflight→runtime link",
    "F-05": "Receipt schema/signature verification; tombstone/delete tests",
    "F-06": "internal usability gate; WCAG audit; privacy screenshot mode",
    "F-07": "cross-coordinate golden diff; no numeric-confidence check",
    "F-08": "deterministic Doctor corpus; false-positive review; SARIF",
    "F-09": "executor contract; loss report; concurrency/rollback/unmanaged-file tests; semantic-team-validation; scripts/check_semantic_team.py",
    "F-10": "crypto vectors; replay/rollback/equivocation/device revoke tests; semantic-team-validation; scripts/check_semantic_team.py",
    "F-11": "integration contract; license/provenance/SBOM; malicious package corpus",
    "F-12": "importer coverage manifest; partial/unknown honesty; retention/delete",
    "F-13": "consent/payload/egress tests; evidence-linked suggestion checks",
    "F-14": "observed-window wording invariant; history deletion/derivation invalidation",
    "F-15": "frozen ExperimentContract; statistical decision fixtures; runner receipt",
    "F-16": "daemon resource/notification tests; CI exit contract; offline gate",
    "F-17": "JSON Schema/API compatibility; adapter sandbox/egress; import traversal",
    "F-18": "policy precedence/binding/signature/expiry/offline fixtures; Approval scope; audit chain; CI 0/2/3; semantic-team-validation; scripts/check_semantic_team.py",
}

SEMANTIC_TEAM_GATE = "semantic-team-validation; scripts/check_semantic_team.py"

# Single source of truth for the six required gates of this stage. Every canonical
# document that lists the gates must list all six, name and command.
REQUIRED_GATES = [
    ("docs-structure", "python3 scripts/check_docs.py"),
    ("acceptance-validation", "python3 scripts/check_acceptance.py --structure"),
    ("traceability-validation", "python3 scripts/check_acceptance.py --traceability"),
    ("corpus-validation", "python3 scripts/check_acceptance.py --corpus"),
    ("semantic-team-validation", "python3 scripts/check_semantic_team.py"),
    (
        "validator-negative-tests",
        "TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'",
    ),
]
GATE_TABLE_DOCS = [
    "README.md",
    "AGENTS.md",
    "docs/README.md",
    "docs/architecture/semantic-alignment-and-team-standard.md",
    "docs/process/test-strategy.md",
    "docs/process/implementation-plan.md",
]
SEMANTIC_TEAM_ARTIFACT_HINT = (
    "acceptance/semantic-team-contract.yaml; "
    "docs/architecture/semantic-alignment-and-team-standard.md; "
    "scripts/check_semantic_team.py"
)
SEMANTIC_TEAM_MARKERS = (
    "CanonicalIntent",
    "TeamContextStandard",
    "Team Context Standard",
    "native-equivalent",
    "lossless-native-overlay",
    "detect-only",
    "non-enforceable",
    "语义对齐",
    "团队上下文标准",
    "投影结果",
    "byte/hash",
    "字节或哈希",
    "organization > team > project > role > personal",
)

UNKNOWN_REASON_CODES = [
    "surface_not_exposed",
    "unsupported_harness_version",
    "permission_not_granted",
    "runtime_snapshot_missing",
    "cloud_setting_unavailable",
    "dynamic_agent_selection",
    "tool_schema_not_exported",
    "current_occupancy_not_reported",
    "content_redacted_by_policy",
    "import_parse_failed",
    "evidence_stale",
    "not_installed",
    "connector_required",
    "config_residue_only",
    "authentication_unavailable",
    "sandbox_unavailable",
    "configured_model_unsupported",
    "attachment_unavailable",
    "official_distribution_not_captured",
    "hermetic_fixture_only",
]

CAPABILITIES = [
    {
        "id": "instructions",
        "index": 1,
        "name": "system/developer/managed/team/user/project/nested/local instructions",
    },
    {
        "id": "rules",
        "index": 2,
        "name": "conditional/manual/agent-selected rules",
    },
    {
        "id": "memory",
        "index": 3,
        "name": "user/auto/subagent memory",
    },
    {
        "id": "skills",
        "index": 4,
        "name": "skills and on-demand references/assets/scripts",
    },
    {
        "id": "plugins",
        "index": 5,
        "name": "plugins/extensions",
    },
    {
        "id": "commands",
        "index": 6,
        "name": "custom commands and reusable prompts",
    },
    {
        "id": "agents",
        "index": 7,
        "name": "agent/subagent definitions, delegation and child-context boundary",
    },
    {
        "id": "mcp-declarations",
        "index": 8,
        "name": "MCP server instructions, tools, prompts, resources",
    },
    {
        "id": "tool-schema-catalog",
        "index": 9,
        "name": "built-in/MCP tool schema, tool search/catalog metadata",
    },
    {
        "id": "tool-invocation",
        "index": 10,
        "name": "tool/MCP invocation and result",
    },
    {
        "id": "task-message",
        "index": 11,
        "name": "current user/task message, @ files, attachments, retrieval/file reads",
    },
    {
        "id": "history",
        "index": 12,
        "name": "conversation history, assistant messages/prefill",
    },
    {
        "id": "compaction-steering",
        "index": 13,
        "name": "compaction summary, steering, handoff",
    },
    {
        "id": "environment-metadata",
        "index": 14,
        "name": "environment/repository/worktree/model metadata",
    },
    {
        "id": "permission-description",
        "index": 15,
        "name": "model-visible permission/policy/sandbox description",
    },
    {
        "id": "other-unknown",
        "index": 16,
        "name": "other-unknown",
    },
]

STATIC_CAPABILITY_IDS = {
    "instructions",
    "rules",
    "memory",
    "skills",
    "plugins",
    "commands",
    "agents",
    "mcp-declarations",
    "environment-metadata",
}

# Live native oracles declared as repeatable machine surfaces with local command evidence.
DECLARED_NATIVE_ORACLES = {
    "codex:cli:debug-prompt-input": {
        "family_id": "codex",
        "surface": "cli",
        "oracle_id": "debug-prompt-input",
        "command": ["codex", "debug", "prompt-input"],
        "coverage": "partial-declared-surface",
        "local_evidence": "local-environment-receipt: codex debug --help lists prompt-input; a structure-only probe was captured without storing prompt body",
        "notes": "Does not return complete core prompt or provider-wire payload.",
    },
    "grok-build:cli:inspect-json": {
        "family_id": "grok-build",
        "surface": "cli",
        "oracle_id": "inspect-json",
        "command": ["grok", "inspect", "--json"],
        "coverage": "partial-declared-surface",
        "local_evidence": "local-environment-receipt: grok inspect --help documents --json",
        "notes": "A later read-only simulation was blocked by sandbox-unavailable; the command is still a declared oracle, not a live session capture in this stage.",
    },
}

# Native result shapes for the two declared oracles. Kept out of
# DECLARED_NATIVE_ORACLES so the compatibility-matrix dump does not change.
ORACLE_NATIVE_SHAPES = {
    "debug-prompt-input": {
        "format": "json",
        "required_keys": ["messages"],
        "optional_keys": ["characters", "by_role"],
        "coverage": "partial-declared-surface",
        "prohibits": ["complete-core-prompt", "provider-wire-payload"],
    },
    "inspect-json": {
        "format": "json",
        "required_keys": ["cwd"],
        "optional_keys": ["rules", "skills", "mcp"],
        "coverage": "partial-declared-surface",
        "prohibits": ["model-visible-present-without-native-proof"],
    },
}

OS_LANES = [
    {
        "id": "macos-27-arm64",
        "os": "macOS",
        "version": "27.0",
        "build": "26A5425a",
        "arch": "arm64",
        "live_status": "captured",
        "image_digest": "os-build:26A5425a",
        "digest_status": "local-os-build",
        "evidence": "docs/research/2026-09-04-local-environment-receipt.md",
        "source_url": "docs/research/2026-09-04-local-environment-receipt.md",
        "access_date": "2026-09-04",
        "honesty": "Captured on the research machine at cutoff. Not a performance-gate minimum lane.",
    },
    {
        "id": UBUNTU_24_04["id"],
        "os": UBUNTU_24_04["os"],
        "version": UBUNTU_24_04["version"],
        "build": UBUNTU_24_04["build"],
        "arch": UBUNTU_24_04["arch"],
        "live_status": UBUNTU_24_04["live_status"],
        "image_name": UBUNTU_24_04["image_name"],
        "image_digest": UBUNTU_24_04["image_digest"],
        "digest_status": UBUNTU_24_04["digest_status"],
        "evidence": UBUNTU_24_04["source_url"],
        "source_url": UBUNTU_24_04["source_url"],
        "access_date": UBUNTU_24_04["access_date"],
        "honesty": UBUNTU_24_04["evidence_scope"],
    },
    {
        "id": WINDOWS_11_24H2["id"],
        "os": WINDOWS_11_24H2["os"],
        "version": WINDOWS_11_24H2["version"],
        "build": WINDOWS_11_24H2["build"],
        "arch": WINDOWS_11_24H2["arch"],
        "live_status": WINDOWS_11_24H2["live_status"],
        "image_name": WINDOWS_11_24H2["image_name"],
        "image_digest": WINDOWS_11_24H2["image_digest"],
        "digest_status": WINDOWS_11_24H2["digest_status"],
        "kb": WINDOWS_11_24H2["kb"],
        "os_build_family": WINDOWS_11_24H2["os_build_family"],
        "evidence": WINDOWS_11_24H2["source_url"],
        "source_url": WINDOWS_11_24H2["source_url"],
        "access_date": WINDOWS_11_24H2["access_date"],
        "honesty": WINDOWS_11_24H2["evidence_scope"],
    },
]

# Primary local coding surfaces that must appear in the 18-family matrix.
FAMILIES = [
    {
        "id": "codex",
        "name": "Codex",
        "cohort": "anchor",
        "surfaces": [
            {
                "id": "cli",
                "version": "0.147.0",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": ["debug-prompt-input"],
            },
            {
                "id": "desktop",
                "version": "unknown-honesty",
                "version_source": "unknown-honesty-official-same-version-absent",
                "macos_install": {
                    "executable_present": "unknown",
                    "app_present": "unknown",
                    "config_residue_present": "unknown",
                    "authenticated": "unknown",
                    "connector_ready": False,
                    "live_status": "unknown",
                },
                "static_support": "required-unknown-honesty",
                "native_oracle_ids": [],
            },
            {
                "id": "cloud",
                "version": "unknown-honesty",
                "version_source": "connector-surface-no-local-version",
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": "unknown",
                    "connector_ready": False,
                    "live_status": "connector-required",
                },
                "static_support": "required-unknown-honesty",
                "native_oracle_ids": [],
            },
        ],
    },
    {
        "id": "claude-code",
        "name": "Claude Code",
        "cohort": "anchor",
        "surfaces": [
            {
                "id": "cli",
                "version": "2.1.259",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "declared_but_not_repeatable_oracle": "/context is product-declared; this freeze does not claim a machine-readable repeatable oracle",
            },
            {
                "id": "cloud",
                "version": "unknown-honesty",
                "version_source": "connector-surface-no-local-version",
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": "unknown",
                    "connector_ready": False,
                    "live_status": "connector-required",
                },
                "static_support": "required-unknown-honesty",
                "native_oracle_ids": [],
            },
        ],
    },
    {
        "id": "cursor",
        "name": "Cursor",
        "cohort": "anchor",
        "surfaces": [
            {
                "id": "ide",
                "version": "3.19.7",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": True,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": "User/Team Rules may be opaque; model-visible is required-unknown-honesty.",
            },
            {
                "id": "agent-cli",
                "version": "2026.08.25-3e8eec8",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
            },
            {
                "id": "cloud",
                "version": "unknown-honesty",
                "version_source": "connector-surface-no-local-version",
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": "unknown",
                    "connector_ready": False,
                    "live_status": "connector-required",
                },
                "static_support": "required-unknown-honesty",
                "native_oracle_ids": [],
            },
        ],
    },
    {
        "id": "grok-build",
        "name": "Grok Build",
        "cohort": "anchor",
        "surfaces": [
            {
                "id": "cli",
                "version": "1.0.13",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": ["inspect-json"],
            },
            {
                "id": "tui",
                "version": "1.0.13",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-unknown-honesty",
                "native_oracle_ids": [],
                "notes": "TUI is a distinct surface from headless CLI.",
            },
        ],
    },
    {
        "id": "opencode",
        "name": "OpenCode",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli",
                "version": "1.18.21",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "installed-auth-unhealthy",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": "A later simulation returned HTTP 401 for a configured model; install ≠ auth.",
            }
        ],
    },
    {
        "id": "deepseek-harness",
        "name": "DeepSeek Harness",
        "cohort": "expansion-hermetic",
        "surfaces": [
            {
                "id": "cli",
                "version": DEEPSEEK_HARNESS["version"],
                "version_source": DEEPSEEK_HARNESS["version_source"],
                "release_tag": DEEPSEEK_HARNESS["release_tag"],
                "git_commit": DEEPSEEK_HARNESS["git_commit"],
                "source_url": DEEPSEEK_HARNESS["source_url"],
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": True,
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "not-installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": (
                    "Config residue must not be reported as a runnable install. "
                    "Official cutoff release is GitHub immutable tag "
                    f"{DEEPSEEK_HARNESS['release_tag']} ({DEEPSEEK_HARNESS['git_commit']}); "
                    "this machine did not have a runnable dsh executable."
                ),
            }
        ],
    },
    {
        "id": "kimi-code",
        "name": "Kimi Code",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli",
                "version": "0.40.1",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
            }
        ],
    },
    {
        "id": "zcode",
        "name": "ZCode",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "app",
                "version": "3.10.2",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": False,
                    "app_present": True,
                    "config_residue_present": True,
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": "No standalone CLI was found. Do not fabricate a CLI surface.",
            }
        ],
    },
    {
        "id": "qwen-code",
        "name": "Qwen Code",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli",
                "version": "0.18.0",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
            }
        ],
    },
    {
        "id": "goose",
        "name": "Goose",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli",
                "version": "1.37.0",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
            }
        ],
    },
    {
        "id": "gemini-cli",
        "name": "Gemini CLI",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli",
                "version": "0.55.1",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "installed-auth-unhealthy",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": "A later account/client combination was rejected; that does not retire the family.",
            }
        ],
    },
    {
        "id": "github-copilot-cli",
        "name": "GitHub Copilot CLI",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli",
                "version": "1.0.82",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": False,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": "plugins list is documented as a native inventory surface but was not captured as a repeatable oracle in the local receipt.",
            }
        ],
    },
    {
        "id": "kiro",
        "name": "Kiro",
        "cohort": "expansion-local",
        "surfaces": [
            {
                "id": "cli-app",
                "version": "2.9.0",
                "version_source": "local-environment-receipt",
                "macos_install": {
                    "executable_present": True,
                    "app_present": True,
                    "config_residue_present": "unknown-not-scanned",
                    "authenticated": "unknown-not-scanned",
                    "connector_ready": True,
                    "live_status": "installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": "App bundle contains kiro-cli executables; PATH command kiro was absent. executable-present via bundle, not PATH.",
            }
        ],
    },
    {
        "id": "cline",
        "name": "Cline",
        "cohort": "expansion-hermetic",
        "surfaces": [
            {
                "id": "cli",
                "version": CLINE["version"],
                "version_source": CLINE["version_source"],
                "release_tag": CLINE["release_tag"],
                "git_commit": CLINE["git_commit"],
                "source_url": CLINE["source_url"],
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "not-installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": (
                    f"Official CLI freeze is {CLINE['release_tag']} "
                    f"({CLINE['git_commit']}); not installed on the research machine."
                ),
            }
        ],
    },
    {
        "id": "aider",
        "name": "Aider",
        "cohort": "expansion-hermetic",
        "surfaces": [
            {
                "id": "cli",
                "version": AIDER["version"],
                "version_source": AIDER["version_source"],
                "release_tag": AIDER["release_tag"],
                "git_commit": AIDER["git_commit"],
                "source_url": AIDER["source_url"],
                "package_digest": AIDER["image_digest"],
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "not-installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": (
                    "Do not pretend Aider is a SKILL.md harness. Official cutoff release is "
                    f"aider-chat {AIDER['version']} ({AIDER['release_tag']}, {AIDER['git_commit']})."
                ),
            }
        ],
    },
    {
        "id": "openhands",
        "name": "OpenHands",
        "cohort": "expansion-hermetic",
        "surfaces": [
            {
                "id": "sdk",
                "version": OPENHANDS["version"],
                "version_source": OPENHANDS["version_source"],
                "release_tag": OPENHANDS["release_tag"],
                "git_commit": OPENHANDS["git_commit"],
                "source_url": OPENHANDS["source_url"],
                "image_name": OPENHANDS["image_name"],
                "image_digest": OPENHANDS["image_digest"],
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "not-installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": (
                    f"Official cutoff release is {OPENHANDS['release_tag']} "
                    f"({OPENHANDS['git_commit']}); image tag {OPENHANDS['image_name']} "
                    f"has {DIGEST_NOT_PUBLISHED}."
                ),
            }
        ],
    },
    {
        "id": "windsurf",
        "name": "Windsurf",
        "cohort": "expansion-hermetic",
        "surfaces": [
            {
                "id": "ide",
                "version": WINDSURF["version"],
                "version_source": WINDSURF["version_source"],
                "release_tag": WINDSURF["release_tag"],
                "git_commit": WINDSURF["git_commit"],
                "source_url": WINDSURF["source_url"],
                "image_digest": WINDSURF["image_digest"],
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "not-installed",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": (
                    f"Official Windsurf-branded editor freeze is {WINDSURF['version']} "
                    f"(build {WINDSURF['git_commit']}). Devin Desktop 3.x is a successor name, "
                    "not this family freeze. Not installed on the research machine."
                ),
            }
        ],
    },
    {
        "id": "coze",
        "name": "Coze",
        "cohort": "connector",
        "surfaces": [
            {
                "id": "connector",
                "version": COZE["version"],
                "version_source": COZE["version_source"],
                "release_tag": COZE["release_tag"],
                "package": COZE["package"],
                "source_url": COZE["source_url"],
                "image_digest": COZE["image_digest"],
                "macos_install": {
                    "executable_present": False,
                    "app_present": False,
                    "config_residue_present": False,
                    "authenticated": False,
                    "connector_ready": False,
                    "live_status": "connector-required",
                },
                "static_support": "required-supported",
                "native_oracle_ids": [],
                "notes": (
                    "Recent-document residue is not a runnable install. Coze is a "
                    f"connector/executor. Official connector freeze is {COZE['package']}@{COZE['version']}."
                ),
            }
        ],
    },
]

STATIC_CASE_CLASSES = [
    "include",
    "exclude",
    "override",
    "cap",
    "ignore",
    "conditional",
    "progressive",
    "legacy",
    "filesystem_edge",
    "negative",
    "unknown_honesty",
    "loss",
]

ORACLE_CASE_CLASSES = [
    "include_observed",
    "exclude_not_in_export",
    "partial_coverage",
    "unknown_core_prompt",
    "cap_truncation",
    "skill_catalog_vs_body",
    "mcp_schema_partial",
    "negative_missing_item",
    "cwd_change",
    "version_pin",
    "redaction",
    "repeat_digest_stability",
]

DOCTOR_BLOCKING_RULES = [
    "secret_literal",
    "symlink_escape",
    "hidden_unicode",
    "path_containment_escape",
    "required_asset_missing",
    "unapproved_lossy_projection",
    "archive_traversal",
    "passive_scan_exec",
]

DOCTOR_NONBLOCKING_RULES = [
    "duplicate",
    "conflict",
    "stale",
    "oversized_resident",
    "cap_truncation",
    "bad_frontmatter",
    "gitignore_mismatch",
    "single_device_only",
    "unknown_source",
    "version_incompatible",
    "undiscoverable_path",
    "placement_recommendation",
]

ANCHOR_IDS = ["codex", "claude-code", "cursor", "grok-build"]

NA_RULES_FAMILIES = {"codex", "grok-build", "aider"}
NO_SKILLS_FAMILIES = {"aider"}
CONNECTOR_FAMILIES = {"coze"}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8"))


def canonical_json(obj: Any) -> str:
    return json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def dump_json_yaml(obj: Any) -> str:
    return json.dumps(obj, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def load_json_yaml(path: Path) -> Any:
    text = path.read_text(encoding="utf-8")
    return json.loads(text)


def family_ids() -> list[str]:
    return [family["id"] for family in FAMILIES]


def family_by_id(family_id: str) -> dict[str, Any]:
    for family in FAMILIES:
        if family["id"] == family_id:
            return family
    raise KeyError(family_id)


def iter_surfaces() -> Iterable[tuple[dict[str, Any], dict[str, Any]]]:
    for family in FAMILIES:
        for surface in family["surfaces"]:
            yield family, surface


def coordinate_id(family_id: str, version: str, surface: str, os_lane: str) -> str:
    return f"{family_id}/{version}/{surface}/{os_lane}"


def field_map_relpath(family_id: str, surface: str) -> str:
    return f"acceptance/field-to-claim/{family_id}-{surface}.yaml"


def encoded_kind_from_fixture_id(fixture_id: str) -> str | None:
    if not isinstance(fixture_id, str):
        return None
    if fixture_id.startswith("dev:static:") or fixture_id.startswith("sealed:static:"):
        return "static"
    if fixture_id.startswith("dev:oracle:") or fixture_id.startswith("sealed:oracle:"):
        return "oracle"
    if fixture_id.startswith("dev:doctor:") or fixture_id.startswith("sealed:doctor:"):
        return "doctor"
    if fixture_id.startswith("live:oracle:"):
        return "oracle-recipe"
    return None


def _dedupe_fields(fields: list[str]) -> list[str]:
    out: list[str] = []
    seen: set[str] = set()
    for field in fields:
        if field not in seen:
            seen.add(field)
            out.append(field)
    return out


def fixture_class_kind(item: dict[str, Any]) -> str | None:
    encoded = encoded_kind_from_fixture_id(str(item.get("id") or ""))
    if encoded:
        return encoded
    kind = item.get("kind")
    return kind if isinstance(kind, str) and kind else None


def manifest_required_fields(item: dict[str, Any]) -> list[str]:
    fields = list(CORPUS_FIXTURE_REQUIRED_FIELDS)
    corpus = item.get("corpus")
    kind = fixture_class_kind(item)
    if corpus == "development" and kind in {"static", "oracle"}:
        fields.extend(CORPUS_DEVELOPMENT_STATIC_ORACLE_FIELDS)
        if kind == "oracle":
            fields.extend(CORPUS_DEVELOPMENT_ORACLE_EXTRA_FIELDS)
    elif corpus == "development" and kind == "doctor":
        fields.extend(CORPUS_DEVELOPMENT_DOCTOR_FIELDS)
    return _dedupe_fields(fields)


def jsonl_required_fields(row: dict[str, Any]) -> list[str]:
    fields = list(JSONL_RECORD_REQUIRED_FIELDS)
    corpus = row.get("corpus")
    kind = fixture_class_kind(row)
    if corpus == "development" and kind in {"static", "oracle"}:
        fields.extend(CORPUS_DEVELOPMENT_STATIC_ORACLE_FIELDS)
        if kind == "oracle":
            fields.extend(CORPUS_DEVELOPMENT_ORACLE_EXTRA_FIELDS)
    elif corpus == "development" and kind == "doctor":
        fields.extend(CORPUS_DEVELOPMENT_DOCTOR_FIELDS)
    return _dedupe_fields(fields)


def encoded_coordinate_from_fixture_id(fixture_id: str) -> str | None:
    """Stable coordinate encoded in a generated fixture id, if any."""
    if not isinstance(fixture_id, str):
        return None
    oracle_prefixes = ("dev:oracle:", "sealed:oracle:", "live:oracle:")
    static_prefixes = ("dev:static:", "sealed:static:")
    for prefix in oracle_prefixes:
        if fixture_id.startswith(prefix):
            rest = fixture_id[len(prefix) :]
            marker = "/oracle:"
            idx = rest.find(marker)
            if idx < 0:
                return None
            tail = rest[idx + len(marker) :]
            oracle_id, _sep, _suffix = tail.partition(":")
            if not oracle_id:
                return None
            return rest[: idx + len(marker) + len(oracle_id)]
    for prefix in static_prefixes:
        if fixture_id.startswith(prefix):
            rest = fixture_id[len(prefix) :]
            coord, _sep, _tail = rest.partition(":")
            return coord or None
    return None


def derived_fixture_coordinate_id(row: dict[str, Any]) -> str | None:
    family_id = row.get("family_id")
    version = row.get("version")
    surface = row.get("surface")
    os_lane = row.get("os_lane")
    if not all(isinstance(value, str) and value for value in (family_id, version, surface, os_lane)):
        return None
    base = coordinate_id(family_id, version, surface, os_lane)
    oracle_id = row.get("oracle_id")
    kind = row.get("kind")
    if oracle_id and kind in {"oracle", "oracle-recipe"}:
        return f"{base}/oracle:{oracle_id}"
    return base


def unknown_live_install() -> dict[str, Any]:
    return {
        "executable_present": "unknown",
        "app_present": "unknown",
        "config_residue_present": "unknown",
        "authenticated": "unknown",
        "connector_ready": False,
        "live_status": "unknown",
        "unknown_reason": "official_distribution_not_captured",
    }


def canonical_compatibility_coordinates() -> list[dict[str, Any]]:
    """Complete family × surface × OS-lane Cartesian set from frozen constants."""
    coordinates = []
    for family, surface in iter_surfaces():
        for lane in OS_LANES:
            install = dict(surface["macos_install"])
            if lane["id"] != "macos-27-arm64":
                install = unknown_live_install()
                support = "required-unknown-honesty"
                if surface["id"] in {"app", "ide", "cli-app"} and lane["os"] != "macOS":
                    if family["id"] in {"zcode", "kiro"}:
                        support = "not-applicable"
                native = "not-applicable" if support == "not-applicable" else "required-unknown-honesty"
            else:
                support = surface["static_support"]
                native = (
                    "required-supported"
                    if surface.get("native_oracle_ids")
                    else "required-unknown-honesty"
                )
                if support != "required-supported":
                    native = "required-unknown-honesty"
            source_url = surface.get("source_url")
            if not (
                isinstance(source_url, str)
                and (
                    source_url.startswith("http://")
                    or source_url.startswith("https://")
                    or source_url.startswith("docs/")
                )
            ):
                source_url = None
            coordinates.append(
                {
                    "id": coordinate_id(family["id"], surface["version"], surface["id"], lane["id"]),
                    "family_id": family["id"],
                    "family_name": family["name"],
                    "cohort": family["cohort"],
                    "version": surface["version"],
                    "version_source": surface["version_source"],
                    "surface": surface["id"],
                    "os_lane": lane["id"],
                    "static_support": support
                    if lane["id"] == "macos-27-arm64"
                    else ("not-applicable" if support == "not-applicable" else "required-unknown-honesty"),
                    "native_support": native,
                    "live_install": install,
                    "native_oracle_ids": list(surface.get("native_oracle_ids") or []),
                    "notes": surface.get("notes") or surface.get("declared_but_not_repeatable_oracle") or "",
                    "fabricated_native_evidence": False,
                    "source_url": source_url,
                }
            )
    return coordinates


def canonical_compatibility_matrix() -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "compatibility-matrix",
        "os_lanes": OS_LANES,
        "source_backed_os": {
            "ubuntu-24.04-x86_64": {
                "image_name": UBUNTU_24_04["image_name"],
                "image_digest": UBUNTU_24_04["image_digest"],
                "source_url": UBUNTU_24_04["source_url"],
                "access_date": ACCESS_DATE,
            },
            "windows-11-24h2-x86_64": {
                "build": WINDOWS_11_24H2["build"],
                "image_digest": WINDOWS_11_24H2["image_digest"],
                "source_url": WINDOWS_11_24H2["source_url"],
                "access_date": ACCESS_DATE,
            },
        },
        "families": [
            {
                "id": family["id"],
                "name": family["name"],
                "cohort": family["cohort"],
                "surfaces": [surface["id"] for surface in family["surfaces"]],
            }
            for family in FAMILIES
        ],
        "declared_native_oracles": DECLARED_NATIVE_ORACLES,
        "coordinates": canonical_compatibility_coordinates(),
        "honesty": {
            "no_fabricated_native_evidence": True,
            "config_residue_is_not_install": True,
            "recent_item_is_not_install": True,
            "unknown_version_fail_closed": True,
            "ubuntu_image_digest": UBUNTU_24_04["image_digest"],
            "windows_build": WINDOWS_11_24H2["build"],
            "windows_image_digest": WINDOWS_11_24H2["image_digest"],
        },
    }


def required_loss_cells() -> list[dict[str, Any]]:
    """The 8 required loss cells: NA rules for Codex/Grok/Aider surfaces plus Aider skills/plugins."""
    macos = "macos-27-arm64"
    cells = []
    for family, surface in iter_surfaces():
        cap_ids: list[str] = []
        if family["id"] in NA_RULES_FAMILIES:
            cap_ids.append("rules")
        if family["id"] in NO_SKILLS_FAMILIES:
            cap_ids.extend(["skills", "plugins"])
        for cap_id in cap_ids:
            cells.append(
                {
                    "id": f"{family['id']}/{surface['version']}/{surface['id']}/{macos}/{cap_id}",
                    "family_id": family["id"],
                    "version": surface["version"],
                    "surface": surface["id"],
                    "os_lane": macos,
                    "capability_id": cap_id,
                    "coordinate_id": coordinate_id(
                        family["id"], surface["version"], surface["id"], macos
                    ),
                }
            )
    return cells


def frozen_surface(family_id: str, surface_id: str) -> dict[str, Any]:
    family = family_by_id(family_id)
    for surface in family["surfaces"]:
        if surface["id"] == surface_id:
            return surface
    raise KeyError(f"{family_id}/{surface_id}")


def primary_surface_id(family_id: str) -> str:
    return family_by_id(family_id)["surfaces"][0]["id"]


def required_supported_static_coordinates() -> list[dict[str, Any]]:
    """Primary static coordinates that must each have >=60 generated static cases.

    Only macOS live-captured OS is required-supported for static grammar. Ubuntu
    and Windows remain required-unknown-honesty until official image digests
    and same-version distribution evidence are captured.
    """
    coords = []
    macos = "macos-27-arm64"
    for family, surface in iter_surfaces():
        if surface["static_support"] != "required-supported":
            continue
        coords.append(
            {
                "id": coordinate_id(family["id"], surface["version"], surface["id"], macos),
                "family_id": family["id"],
                "family_name": family["name"],
                "version": surface["version"],
                "surface": surface["id"],
                "os_lane": macos,
                "support": "required-supported",
                "kind": "static",
                "live_status": surface["macos_install"]["live_status"],
                "native_oracle_ids": list(surface.get("native_oracle_ids") or []),
            }
        )
    return coords


def declared_oracle_coordinates() -> list[dict[str, Any]]:
    coords = []
    macos = "macos-27-arm64"
    for oracle_key, oracle in DECLARED_NATIVE_ORACLES.items():
        family = family_by_id(oracle["family_id"])
        surface = next(s for s in family["surfaces"] if s["id"] == oracle["surface"])
        coords.append(
            {
                "id": coordinate_id(family["id"], surface["version"], surface["id"], macos)
                + f"/oracle:{oracle['oracle_id']}",
                "oracle_key": oracle_key,
                "family_id": family["id"],
                "family_name": family["name"],
                "version": surface["version"],
                "surface": surface["id"],
                "os_lane": macos,
                "oracle_id": oracle["oracle_id"],
                "command": oracle["command"],
                "coverage": oracle["coverage"],
                "live_tested": False,
            }
        )
    return coords


def capability_status(family_id: str, surface_id: str, cap_id: str, static_support: str) -> str:
    if cap_id == "other-unknown":
        return "required-unknown-honesty"
    if family_id in CONNECTOR_FAMILIES and cap_id not in {
        "environment-metadata",
        "mcp-declarations",
        "skills",
        "other-unknown",
    }:
        return "not-applicable"
    if family_id in NO_SKILLS_FAMILIES and cap_id in {"skills", "plugins"}:
        return "not-applicable"
    if family_id in NA_RULES_FAMILIES and cap_id == "rules":
        return "not-applicable"
    if cap_id in STATIC_CAPABILITY_IDS:
        if static_support == "required-supported":
            return "required-supported"
        return "required-unknown-honesty"
    # Dynamic categories: native present only for declared oracles on matching surfaces.
    oracle_caps = {
        "codex:cli": {"task-message", "history", "tool-schema-catalog"},
        "grok-build:cli": {"environment-metadata", "instructions"},
    }
    key = f"{family_id}:{surface_id}"
    if cap_id in oracle_caps.get(key, set()):
        return "required-supported"
    return "required-unknown-honesty"


# PRD F-09 required-write floor. Independent of generated projection-matrix cells.
FROZEN_REQUIRED_WRITE_CELL_IDS = frozenset(
    {
        "codex/project-instructions/project",
        "claude-code/project-instructions/project",
        "cursor/project-instructions/project",
        "grok-build/project-instructions/project",
        "codex/user-instructions/user",
        "claude-code/user-instructions/user",
        "grok-build/user-instructions/user",
        "codex/packaged-skills/project-or-user",
        "claude-code/packaged-skills/project-or-user",
        "cursor/packaged-skills/project-or-user",
        "grok-build/packaged-skills/project-or-user",
        "codex/unmanaged-skills/project-or-user",
        "claude-code/unmanaged-skills/project-or-user",
        "cursor/unmanaged-skills/project-or-user",
        "grok-build/unmanaged-skills/project-or-user",
        "claude-code/scoped-conditional-rules/project",
        "cursor/scoped-conditional-rules/project",
        "codex/native-mcp-config/public-writable-user-or-project",
        "claude-code/native-mcp-config/public-writable-user-or-project",
        "cursor/native-mcp-config/public-writable-user-or-project",
        "grok-build/native-mcp-config/public-writable-user-or-project",
    }
)

FROZEN_FORBIDDEN_REQUIRED_WRITE_CELL_IDS = frozenset(
    {
        "cursor/user-instructions/user",
        "codex/commands-plugins-hooks-managed-ui/various",
        "claude-code/commands-plugins-hooks-managed-ui/various",
        "cursor/commands-plugins-hooks-managed-ui/various",
        "grok-build/commands-plugins-hooks-managed-ui/various",
    }
)

ORACLE_CAPABILITY_IDS = {
    "codex:cli": ("task-message", "history", "tool-schema-catalog"),
    "grok-build:cli": ("environment-metadata", "instructions"),
}


def canonical_capability_cells() -> list[dict[str, Any]]:
    """Complete family × surface × OS lane × capability Cartesian set."""
    cells = []
    for family, surface in iter_surfaces():
        for lane in OS_LANES:
            for cap in CAPABILITIES:
                if lane["id"] == "macos-27-arm64":
                    status = capability_status(
                        family["id"], surface["id"], cap["id"], surface["static_support"]
                    )
                elif family["id"] in {"zcode", "kiro"} and surface["id"] in {"app", "cli-app"}:
                    status = "not-applicable"
                else:
                    status = "required-unknown-honesty"
                cells.append(
                    {
                        "id": f"{family['id']}/{surface['version']}/{surface['id']}/{lane['id']}/{cap['id']}",
                        "family_id": family["id"],
                        "version": surface["version"],
                        "surface": surface["id"],
                        "os_lane": lane["id"],
                        "capability_id": cap["id"],
                        "status": status,
                        "coordinate_id": coordinate_id(
                            family["id"], surface["version"], surface["id"], lane["id"]
                        ),
                    }
                )
    return cells


def projection_cells() -> list[dict[str, Any]]:
    cells = []
    anchors = {
        "codex": "Codex",
        "claude-code": "Claude Code",
        "cursor": "Cursor",
        "grok-build": "Grok Build",
    }
    specs = [
        ("project-instructions", "project", "required-write", "contexpect-native", None),
        ("user-instructions", "user", "mixed", "contexpect-native", "cursor-export-only"),
        ("packaged-skills", "project-or-user", "required-write", "APM", None),
        ("unmanaged-skills", "project-or-user", "required-write", "contexpect-native", None),
        ("scoped-conditional-rules", "project", "mixed-na", "contexpect-native", None),
        ("native-mcp-config", "public-writable-user-or-project", "required-write", "contexpect-native", "package-install-APM"),
        ("commands-plugins-hooks-managed-ui", "various", "observe-import-export", "export-only-or-frozen-external", None),
    ]
    for family_id, family_name in anchors.items():
        for asset, scope, mode, authority, extra in specs:
            if asset == "user-instructions" and family_id == "cursor":
                cell_mode = "observe-import-export"
                cell_authority = "export-only"
                write = False
                loss = "Anchor versions have no stable external write surface for User Rules; UI handoff required."
            elif asset == "scoped-conditional-rules" and family_id in {"codex", "grok-build"}:
                cell_mode = "not-applicable-primitive"
                cell_authority = "contexpect-native"
                write = False
                loss = "No independent native scoped-rule primitive; project/nested instruction projection must show loss rather than drop intent."
            elif asset == "commands-plugins-hooks-managed-ui":
                cell_mode = "observe-import-export"
                cell_authority = "export-only-or-frozen-external"
                write = False
                loss = "Required-write only if §17.0 later freezes a stable native executor for that exact cell."
            else:
                cell_mode = "required-write"
                cell_authority = authority
                write = True
                loss = "Unsupported native fields must skip or use native overlay; silent deletion is forbidden."
            native_target = dict(NATIVE_TARGETS[asset][family_id])
            native_target["family_id"] = family_id
            cells.append(
                {
                    "id": f"{family_id}/{asset}/{scope}",
                    "family_id": family_id,
                    "family_name": family_name,
                    "asset": asset,
                    "scope": scope,
                    "mode": cell_mode,
                    "authority": cell_authority,
                    "authority_version": "foundation-freeze-2026-09-04",
                    "native_target": native_target,
                    "required_write": write,
                    "preview": write,
                    "apply": write,
                    "rollback": write,
                    "post_receipt": True,
                    "concurrency_guard": write,
                    "loss_contract": loss,
                    "loss_report": True,
                    "extra": extra,
                }
            )
    for family, surface in iter_surfaces():
        if family["id"] in anchors:
            continue
        cells.append(
            {
                "id": f"{family['id']}/declared-assets/{surface['id']}",
                "family_id": family["id"],
                "family_name": family["name"],
                "asset": "declared-static-assets",
                "scope": "as-declared-by-adapter",
                "mode": "observe-import-export",
                "authority": "export-only",
                "authority_version": "foundation-freeze-2026-09-04",
                "native_target": {
                    "kind": "export-only",
                    "family_id": family["id"],
                    "surface": surface["id"],
                    "path_glob": "declared-static-assets",
                    "scope": "as-declared-by-adapter",
                },
                "required_write": False,
                "preview": False,
                "apply": False,
                "rollback": False,
                "post_receipt": True,
                "concurrency_guard": False,
                "loss_contract": "Expansion families are not in the F-09 required-write floor. Observe/import/export only unless a later acceptance-contract revision adds a required-write cell.",
                "loss_report": True,
                "extra": None,
            }
        )
    return cells


NORMATIVE_LINE_RE = re.compile(
    r"(必须|不得|不能|禁止|永不|不允许|只有.{0,80}才)"
)
HEADING_RE = re.compile(r"^(#{2,4})\s+(.*)$")
SECTION_RE = re.compile(r"^(\d+(?:\.\d+)*)")
FEATURE_HEADING_RE = re.compile(r"\b(F-\d{2})\b")
WP_HEADING_RE = re.compile(r"\b(WP-\d{2})\b")


def _normalize_statement(text: str) -> str:
    text = re.sub(r"\s+", " ", text).strip()
    text = text.strip("-* ")
    return text


def _is_normative(text: str) -> bool:
    return bool(NORMATIVE_LINE_RE.search(text))


def _modal_of(text: str) -> str:
    if re.search(r"禁止|不得|不能|永不|不允许", text):
        if "必须" in text:
            return "MUST-AND-MUST-NOT"
        return "MUST-NOT"
    if re.search(r"只有.{0,80}才", text):
        if "必须" in text:
            return "MUST-ONLY-IF"
        return "ONLY-IF"
    if "必须" in text:
        return "MUST"
    return "OTHER"


def _apply_heading_scope(title: str, level: int, current_section: str, current_feature: str, current_wp: str) -> tuple[str, str, str]:
    """Close feature/WP scope at headings. Feature F-xx only lives under that heading."""
    section = current_section
    feature = current_feature
    wp = current_wp
    section_match = SECTION_RE.match(title)
    if section_match:
        section = section_match.group(1)
    f_head = re.match(r"^(F-\d{2})\b", title)
    wp_head = re.match(r"^(WP-\d{2})\b", title)
    if f_head:
        feature = f_head.group(1)
    elif wp_head:
        wp = wp_head.group(1)
        feature = ""
    elif level <= 2:
        feature = ""
        if not wp_head:
            wp = ""
    elif level == 3 and not f_head:
        feature = ""
        if wp_head:
            wp = wp_head.group(1)
    return section, feature, wp


def content_requirement_id(section: str, digest: str) -> str:
    """Legacy helper. Prefer occurrence_requirement_id for heading-scoped identity."""
    return f"REQ-{section}-{digest[:16]}"


def occurrence_requirement_id(
    section: str, heading: str, text: str, sibling_index: int = 0
) -> str:
    """Content-derived id scoped to section and heading.

    Unrelated insertions in other sections do not renumber existing rows. The
    same sentence in two headings or sections yields two distinct ids.
    """
    section = section or "0"
    heading = heading or "document"
    heading_key = sha256_text(heading)[:8]
    content_key = sha256_text(text)[:16]
    if sibling_index:
        sib = sha256_text(f"{section}\n{heading}\n{text}\n{sibling_index}")[:8]
        return f"REQ-{section}-{heading_key}-{content_key}-{sib}"
    return f"REQ-{section}-{heading_key}-{content_key}"


def occurrence_identity(item: dict[str, Any]) -> tuple[str, str, str, str]:
    return (
        item.get("section") or "0",
        item.get("heading") or "",
        item.get("source_digest") or "",
        item.get("statement") or "",
    )


def section_semantics(section: str) -> dict[str, Any]:
    best_key = ""
    for key in SECTION_SEMANTICS:
        if section == key or section.startswith(key + "."):
            if len(key) > len(best_key):
                best_key = key
    if best_key:
        return SECTION_SEMANTICS[best_key]
    return SECTION_SEMANTICS["0"]


def extract_normative_statements(prd_text: str) -> list[dict[str, Any]]:
    """Extract PRD normative units in document order.

    Units are list items, table rows, or sentences that contain the markers
    required by PRD §17.0. Fenced code blocks are skipped. Feature scope is
    closed when a heading that is not F-xx begins.
    """
    lines = prd_text.splitlines()
    in_code = False
    current_section = "0"
    current_heading = "document"
    current_feature = ""
    current_wp = ""
    raw_units: list[tuple[str, str, str, str, str]] = []
    buffer: list[str] = []

    def flush() -> None:
        nonlocal buffer
        if not buffer:
            return
        text = _normalize_statement(" ".join(buffer))
        buffer = []
        if text and _is_normative(text):
            raw_units.append((current_section, current_heading, current_feature, current_wp, text))

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("```"):
            flush()
            in_code = not in_code
            continue
        if in_code:
            continue
        heading = HEADING_RE.match(line)
        if heading:
            flush()
            title = heading.group(2).strip()
            level = len(heading.group(1))
            current_heading = title
            current_section, current_feature, current_wp = _apply_heading_scope(
                title, level, current_section, current_feature, current_wp
            )
            continue

        if not stripped:
            flush()
            continue
        if stripped.startswith("|") and re.match(r"^\|?\s*-+", stripped.replace("|", " ")):
            continue
        if stripped.startswith("|"):
            flush()
            cell_text = _normalize_statement(stripped)
            if _is_normative(cell_text):
                raw_units.append(
                    (current_section, current_heading, current_feature, current_wp, cell_text)
                )
            continue
        if re.match(r"^([-*] |\d+\.\s)", stripped):
            flush()
            buffer = [stripped]
            continue
        if buffer:
            buffer.append(stripped)
            continue
        if _is_normative(stripped):
            parts = re.split(r"(?<=[。；;])", stripped)
            for part in parts:
                part_n = _normalize_statement(part)
                if part_n and _is_normative(part_n):
                    raw_units.append(
                        (current_section, current_heading, current_feature, current_wp, part_n)
                    )
        else:
            flush()

    flush()

    statements: list[dict[str, Any]] = []
    used_ids: set[str] = set()
    sibling_counts: dict[tuple[str, str, str], int] = {}
    for section, heading, feature, wp, text in raw_units:
        digest = sha256_text(text)
        sibling_key = (section, heading, text)
        sibling_index = sibling_counts.get(sibling_key, 0)
        sibling_counts[sibling_key] = sibling_index + 1
        req_id = occurrence_requirement_id(section, heading, text, sibling_index)
        if req_id in used_ids:
            req_id = occurrence_requirement_id(section, heading, text + "\n" + req_id, sibling_index)
        used_ids.add(req_id)
        statements.append(
            {
                "requirement_id": req_id,
                "section": section,
                "heading": heading,
                "feature": feature,
                "work_package_hint": wp,
                "modal": _modal_of(text),
                "source_digest": digest,
                "statement": text,
            }
        )
    return statements


def assign_trace_metadata(item: dict[str, Any]) -> dict[str, Any]:
    heading = item.get("heading") or ""
    section = item.get("section") or "0"
    heading_feature = re.match(r"^(F-\d{2})\b", heading)
    heading_wp = re.match(r"^(WP-\d{2})\b", heading)
    feature = heading_feature.group(1) if heading_feature else (item.get("feature") or "")
    if heading_wp:
        feature = ""
    if not heading_feature and not heading_wp:
        # Feature closed at non-F headings; do not steal identity from later mentions
        # except 17.6 summary rows that are explicitly about an F-id in the first cell.
        if section.startswith("17.6") or heading.startswith("17.6"):
            text_feature = FEATURE_HEADING_RE.search(item.get("statement") or "")
            if text_feature:
                feature = text_feature.group(1)
        elif not feature:
            feature = ""
    semantics = section_semantics(section)
    if feature in F_TO_WP:
        wps = list(F_TO_WP[feature])
        gate = F_TO_GATE[feature]
        artifact = f"acceptance/*; docs/process/implementation-plan.md; {feature}"
        owner = "product-core"
    elif heading_wp:
        wps = [heading_wp.group(1)]
        gate = "docs-structure; WP coverage"
        artifact = "docs/process/implementation-plan.md"
        owner = "product-core"
    elif item.get("work_package_hint") in WORK_PACKAGES:
        wps = [item["work_package_hint"]]
        gate = semantics["gate"]
        artifact = semantics["artifact"]
        owner = semantics["owner"]
    else:
        wps = list(semantics["wps"])
        gate = semantics["gate"]
        artifact = semantics["artifact"]
        owner = semantics["owner"]
    if not feature:
        feature = "NA"
    if feature != "NA" and feature not in FEATURES:
        feature = "NA"
    if owner not in ALLOWED_OWNERS:
        owner = "architecture"
    blob = f"{heading}\n{item.get('statement') or ''}"
    if any(marker in blob for marker in SEMANTIC_TEAM_MARKERS):
        gate = SEMANTIC_TEAM_GATE
        artifact = SEMANTIC_TEAM_ARTIFACT_HINT
    return {
        **item,
        "feature": feature,
        "work_packages": ";".join(wps),
        "test_or_gate": gate,
        "artifact": artifact,
        "owner": owner,
    }


TRACE_FIELDS = [
    "requirement_id",
    "section",
    "source_digest",
    "modal",
    "work_packages",
    "test_or_gate",
    "artifact",
    "owner",
    "feature",
    "heading",
    "statement",
]


def write_traceability_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=TRACE_FIELDS, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field, "") for field in TRACE_FIELDS})


def read_traceability_csv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def load_prd_text(root: Path | None = None) -> str:
    root = root or repo_root()
    return (root / PRD_RELPATH).read_text(encoding="utf-8")


PRIVATE_SECURITY_URL = "https://github.com/Octo-o-o-o/Contexpect/security/advisories/new"
PUBLIC_SECURITY_FALLBACK_URL = "https://github.com/Octo-o-o-o/Contexpect/issues/new"


def missing_fields(obj: dict[str, Any], required: list[str]) -> list[str]:
    return [field for field in required if field not in obj]


def require_fields(obj: dict[str, Any], required: list[str], label: str, errors: list[str]) -> None:
    missing = missing_fields(obj, required)
    if missing:
        errors.append(f"{label} missing required fields {missing}")
    for field in required:
        if field not in obj:
            continue
        value = obj.get(field)
        if value is None:
            errors.append(f"{label} empty field {field}")
            continue
        if isinstance(value, str) and not value.strip():
            errors.append(f"{label} empty field {field}")
        if isinstance(value, (list, dict)) and field in {
            "field_mapping",
            "expected_native_shape",
            "command",
            "input_path",
            "input_files",
        } and not value:
            errors.append(f"{label} empty field {field}")


def require_schema_version(obj: dict[str, Any], label: str, errors: list[str]) -> None:
    version = obj.get("schema_version")
    if version not in SUPPORTED_SCHEMA_VERSIONS:
        errors.append(f"{label} schema_version {version!r} is not supported (want {SCHEMA_VERSION})")


def require_sensitivity(obj: dict[str, Any], label: str, errors: list[str]) -> None:
    value = obj.get("sensitivity")
    if not value or not str(value).strip():
        errors.append(f"{label} empty sensitivity")
    elif value not in ALLOWED_SENSITIVITY:
        errors.append(f"{label} unknown sensitivity {value!r}")


def uses_export_only(cell: dict[str, Any]) -> bool:
    tokens = [
        cell.get("mode"),
        cell.get("authority"),
        cell.get("executor"),
        cell.get("executor_kind"),
        (cell.get("native_target") or {}).get("kind") if isinstance(cell.get("native_target"), dict) else None,
    ]
    return any(isinstance(token, str) and "export-only" in token for token in tokens)


def capability_cell_id(family_id: str, version: str, surface: str, os_lane: str, capability_id: str) -> str:
    return f"{family_id}/{version}/{surface}/{os_lane}/{capability_id}"


def capability_semantics(status: str, cap_id: str, family_id: str, surface: str) -> dict[str, Any]:
    mapping = f"acceptance/field-to-claim/{family_id}-{surface}.yaml"
    collector = None
    importer = None
    if cap_id in STATIC_CAPABILITY_IDS:
        collector = "static-resolver"
    if family_id == "codex" and surface == "cli" and cap_id in {
        "task-message",
        "history",
        "tool-schema-catalog",
    }:
        importer = "codex-debug-prompt-input"
    if family_id == "grok-build" and surface == "cli" and cap_id in {
        "instructions",
        "environment-metadata",
    }:
        importer = "grok-inspect-json"
    if status == "required-supported":
        return {
            "collector": collector,
            "importer": importer,
            "field_mapping": mapping,
            "provenance": "native-runtime" if importer and importer != "unknown-honesty" else "official-spec",
            "coverage": "partial-declared-surface",
        }
    if status == "required-unknown-honesty":
        return {
            "collector": collector,
            "importer": importer or "unknown-honesty",
            "field_mapping": mapping,
            "provenance": "official-spec",
            "coverage": "unknown",
        }
    return {
        "collector": None,
        "importer": None,
        "field_mapping": mapping,
        "provenance": "official-spec",
        "coverage": "not-applicable",
    }


def scan_forbidden_placeholders(obj: Any, path: str, errors: list[str], keys: set[str] | None = None) -> None:
    """Reject invented placeholder tokens in any string, including composites."""
    from contexpect_freeze import is_forbidden_placeholder

    del keys  # all strings are scanned; keys kept for call-site compatibility
    if isinstance(obj, dict):
        for key, value in obj.items():
            scan_forbidden_placeholders(value, f"{path}.{key}", errors)
    elif isinstance(obj, list):
        for idx, item in enumerate(obj):
            scan_forbidden_placeholders(item, f"{path}[{idx}]", errors)
    elif isinstance(obj, str) and is_forbidden_placeholder(obj):
        errors.append(f"{path} uses forbidden placeholder {obj!r}")
