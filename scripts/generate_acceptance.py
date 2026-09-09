#!/usr/bin/env python3
"""Deterministically generate PRD §17.0 acceptance artifacts and corpora.

This command is a documentation-stage formatter. It does not run harnesses,
does not claim live tests, and requires no network.
"""

from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

from contexpect_contract import (  # noqa: E402
    ACCEPTANCE_ARTIFACTS,
    ANCHOR_IDS,
    ARTIFACT_DIGEST_MANIFEST,
    ARTIFACT_SCHEMA_IDS,
    CAPABILITIES,
    CLAIM_AXES,
    CLAIM_INVARIANTS,
    CLAIM_PRECEDENCE,
    CLAIM_RECONCILIATION_INVARIANTS,
    CLAIM_RECONCILIATION_STATES,
    CLAIM_USE_EVIDENCE_KINDS,
    CUTOFF,
    DOCTOR_BLOCKING_RULES,
    DOCTOR_NONBLOCKING_RULES,
    LICENSE_ID,
    ORACLE_CASE_CLASSES,
    SCHEMA_VERSION,
    STATIC_CAPABILITY_IDS,
    STATIC_CASE_CLASSES,
    UNKNOWN_REASON_CODES,
    assign_trace_metadata,
    NA_RULES_FAMILIES,
    NO_SKILLS_FAMILIES,
    ORACLE_CAPABILITY_IDS,
    ORACLE_NATIVE_SHAPES,
    canonical_compatibility_matrix,
    canonical_json,
    canonical_capability_cells,
    capability_semantics,
    capability_status,
    declared_oracle_coordinates,
    dump_json_yaml,
    extract_normative_statements,
    family_ids,
    iter_surfaces,
    load_prd_text,
    projection_cells,
    required_loss_cells,
    required_supported_static_coordinates,
    sha256_text,
    write_traceability_csv,
)
from contexpect_fixtures import (  # noqa: E402
    build_doctor_files,
    build_oracle_files,
    build_static_files,
    expected_claim_from_parse,
    interpret_oracle,
    interpret_static,
)
from contexpect_freeze import (  # noqa: E402
    ACCESS_DATE,
    DIGEST_NOT_PUBLISHED,
    EVIDENCE_BACKED_UNAVAILABLE,
    INTEGRATION_SOURCES,
    INTEGRATION_UNAVAILABLE,
    UBUNTU_24_04,
    WINDOWS_11_24H2,
)
from contexpect_semantic_team import write_semantic_team_artifacts  # noqa: E402


def _case_digest(payload: dict) -> str:
    body = dict(payload)
    body.pop("digest", None)
    return sha256_text(canonical_json(body))


def write_json(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(dump_json_yaml(obj), encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict]) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [canonical_json(row) for row in rows]
    text = "\n".join(lines) + "\n"
    path.write_text(text, encoding="utf-8")
    return sha256_text(text)


def write_bytes(path: Path, text: str) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return sha256_text(text)


def compatibility_matrix() -> dict:
    return canonical_compatibility_matrix()


def claim_validity_matrix() -> dict:
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "claim-validity-matrix",
        "axes": CLAIM_AXES,
        "use_evidence_kinds": CLAIM_USE_EVIDENCE_KINDS,
        "precedence": CLAIM_PRECEDENCE,
        "llm_suggestion_in_provenance": False,
        "unknown_reason_codes": UNKNOWN_REASON_CODES,
        "invariants": CLAIM_INVARIANTS,
        "reconciliation_states": CLAIM_RECONCILIATION_STATES,
        "reconciliation_invariants": CLAIM_RECONCILIATION_INVARIANTS,
        "legal_examples": [
            {
                "id": "codex-prompt-input-partial",
                "claim_kind": "observed",
                "lifecycle_stage": "model-visible",
                "truth_state": "present",
                "provenance": "native-runtime",
                "coverage": "partial-declared-surface",
                "precision": "exact",
                "knowledge_status": "current",
                "notes": "Exact only for returned message/character counts.",
            }
        ],
        "illegal_examples": [
            {
                "id": "static-as-observed-present",
                "claim_kind": "resolved",
                "lifecycle_stage": "model-visible",
                "truth_state": "present",
                "why": "Static resolver cannot prove model-visible present.",
            }
        ],
    }


def context_capability_matrix() -> dict:
    index_by_id = {cap["id"]: cap["index"] for cap in CAPABILITIES}
    cells = []
    for cell in canonical_capability_cells():
        semantics = capability_semantics(
            cell["status"], cell["capability_id"], cell["family_id"], cell["surface"]
        )
        cells.append(
            {
                "id": cell["id"],
                "family_id": cell["family_id"],
                "version": cell["version"],
                "surface": cell["surface"],
                "os_lane": cell["os_lane"],
                "capability_id": cell["capability_id"],
                "capability_index": index_by_id[cell["capability_id"]],
                "status": cell["status"],
                "compatibility_coordinate_id": cell["coordinate_id"],
                **semantics,
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "context-capability-matrix",
        "capabilities": CAPABILITIES,
        "cells": cells,
        "notes": [
            "other-unknown remains required-unknown-honesty and blocks a complete-coverage conclusion.",
            "Aider skills/plugins are not-applicable; do not wrap Aider as SKILL.md.",
            "Codex and Grok independent scoped-rule primitives are not-applicable; loss must be shown.",
            "Coze is a connector; most prompt-parity capabilities are not-applicable.",
            "Cartesian cells and required floors come from immutable canonical constants, not artifact self-description.",
        ],
    }


def projection_matrix() -> dict:
    cells = projection_cells()
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "projection-matrix",
        "required_write_floor": "PRD F-09 table; cells cannot be demoted to export-only",
        "authorities": [
            "contexpect-native",
            "APM",
            "Agentpack",
            "agentsync",
            "export-only",
            "export-only-or-frozen-external",
        ],
        "gates_per_required_write_cell": [
            "native_target",
            "authority",
            "authority_version",
            "loss_contract",
            "loss_report",
            "concurrency_guard",
            "preview",
            "apply",
            "rollback",
            "post_receipt",
        ],
        "cells": cells,
        "unique_authority_rule": "One ProjectionAuthority per target profile/transaction; two projectors on the same cell are forbidden.",
    }


def integration_contracts() -> dict:
    degrade = "read-only display / export plan / Unknown; never silently enable a built-in substitute"
    specs = [
        (
            "Microsoft APM",
            "CLI + file-import; unique PolicyAuthority for package/source/install/license/digest/SBOM",
            "package lifecycle, lock, SBOM, apm-policy.yml evaluation import",
            degrade,
            "Contexpect must not re-evaluate APM domain checks.",
        ),
        (
            "Agentpack",
            "CLI or frozen external ProjectionAuthority",
            "preview/apply/snapshot/rollback for assigned projection cells",
            degrade,
            "Cannot share a required-write cell with contexpect-native.",
        ),
        (
            "agentsync",
            "CLI; secret-bearing path gated",
            "desired-state projection, loss report, secret refs",
            "read-only/import/export plan until secret artifact gate passes",
            "Backup/local Git artifacts may leak parsed secrets.",
        ),
        (
            "CtxWise",
            "AdapterProvider for Codex JSON/fixture surface at a frozen commit",
            "Codex resolver fixtures and prompt-input xray",
            "bundled static resolver with Observed gap labeled; still run independent golden/native-oracle tests",
            "NOTICE attribution required if code is borrowed.",
        ),
        (
            "Scopeon",
            "file import or OTLP",
            "runtime token/cost/timeline",
            degrade,
            "Not a static intent resolver.",
        ),
        (
            "ctxray",
            "file import",
            "session history analysis",
            degrade,
            "Observed-window wording still applies.",
        ),
        (
            "ContextSpy",
            "advanced opt-in adapter",
            "request-level runtime profiler",
            "disabled by default; product is complete without it",
            "Must not become default MITM/CA.",
        ),
        (
            "age or SOPS",
            "library/format via ADR-0003",
            "E2EE bundle encryption and recipient wrapping",
            "sync disabled if no mature format can be used",
            "No homemade crypto, recovery-code protocol, or device PKI.",
        ),
        (
            "SignerAdapter (SSH/GPG/minisign/Sigstore/existing trust store)",
            "adapter",
            "Receipt and sync envelope authenticity",
            "unsigned local continuity signature is not organizational identity",
            "RFC 8785 JCS + DSSE or equivalent public envelope.",
        ),
    ]
    contracts = []
    for name, mode, capability, degrade_text, notes in specs:
        source = INTEGRATION_SOURCES[name]
        license_id = source["license_from_research"]
        contracts.append(
            {
                "name": name,
                "integration_mode": mode,
                "version_pin": EVIDENCE_BACKED_UNAVAILABLE,
                "version_pin_honesty": "research-ledger-records-github-url-without-cutoff-release-tag",
                "pin_evidence": {
                    "source_url": source["source_url"],
                    "access_date": ACCESS_DATE,
                    "status": EVIDENCE_BACKED_UNAVAILABLE,
                    "reason": INTEGRATION_UNAVAILABLE["reason"],
                    "digest_status": DIGEST_NOT_PUBLISHED,
                },
                "capability": capability,
                "unavailable_degrade": degrade_text,
                "license": license_id,
                "notes": notes,
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "integration-contracts",
        "contracts": contracts,
        "borrow_only_forbidden": [
            "unlicensed code",
            "GPL-3.0 embedded into Apache-2.0 core",
            "README metrics treated as Contexpect truth",
        ],
    }


def field_mapping_docs() -> dict[str, dict]:
    docs = {}
    for family, surface in iter_surfaces():
        key = f"{family['id']}-{surface['id']}"
        if family["id"] == "codex" and surface["id"] == "cli":
            fields = [
                {
                    "native_field": "prompt-input.messages[].role",
                    "lifecycle_stage": "model-visible",
                    "claim_kind": "observed",
                    "coverage": "partial-declared-surface",
                    "minimum_evidence": "native-runtime",
                    "notes": "Does not prove complete core prompt.",
                },
                {
                    "native_field": "prompt-input character counts",
                    "lifecycle_stage": "model-visible",
                    "claim_kind": "observed",
                    "coverage": "partial-declared-surface",
                    "precision": "exact",
                    "minimum_evidence": "native-runtime",
                },
            ]
        elif family["id"] == "deepseek-harness" and surface["id"] == "cli":
            # Native session-log fields the `deepseek-harness-cli` importer reads
            # (crates/ctxpect-importer/src/deepseek_harness.rs). A request/header
            # proves preparation only; provider output in the same turn/step
            # (agent-loop/src/agent.ts:364-368 appends assistant/chunk per
            # streamed chunk after opening the stream) proves dispatch. Usage is
            # provider-reported and is not occupancy. Stages the log does not
            # cover are listed as Unknown.
            fields = [
                {
                    "native_field": "declared-static-path",
                    "lifecycle_stage": "installed",
                    "claim_kind": "resolved",
                    "coverage": "partial-declared-surface",
                    "minimum_evidence": "official-spec",
                },
                {
                    "native_field": "session.jsonl request/header (foldRequestHeader over the log prefix)",
                    "lifecycle_stage": "eligible",
                    "claim_kind": "observed",
                    "coverage": "full-declared-surface",
                    "minimum_evidence": "native-log",
                    "precision": "exact",
                    "notes": "prepared: the header is appended before dispatch (agent.ts:508-517); it does not prove the request reached the model.",
                },
                {
                    "native_field": "session.jsonl assistant/chunk | assistant/message in the header's turn/step (dispatch evidence)",
                    "lifecycle_stage": "model-visible",
                    "claim_kind": "observed",
                    "coverage": "full-declared-surface",
                    "minimum_evidence": "native-log",
                    "precision": "exact",
                    "notes": "The derived surface at the header seq (foldSurface + deriveEventMessage) is what was dispatched; provider output proves dispatch (agent.ts:364-368). Without it the claim is indeterminate (runtime_snapshot_missing).",
                },
                {
                    "native_field": "session.jsonl assistant/message.usage",
                    "lifecycle_stage": "model-visible",
                    "claim_kind": "observed",
                    "coverage": "partial-declared-surface",
                    "minimum_evidence": "native-log",
                    "precision": "exact",
                    "notes": "provider-reported token accounting shown as reported; cumulative input is not occupancy and request/context.contextWindow is capacity, not use.",
                },
                {
                    "native_field": "not-covered:use-evidence",
                    "lifecycle_stage": "use-evidence",
                    "claim_kind": "observed",
                    "coverage": "unknown",
                    "minimum_evidence": "native-runtime",
                    "notes": "Unknown: the session log carries no attribution of which context shaped the answer; the importer answers indeterminate (runtime_snapshot_missing).",
                },
                {
                    "native_field": "not-covered:outcome-affecting",
                    "lifecycle_stage": "outcome-affecting",
                    "claim_kind": "effect",
                    "coverage": "unknown",
                    "minimum_evidence": "native-runtime",
                    "notes": "Unknown: outcome effect needs an Effect Lab contract, never a log; the importer answers indeterminate (runtime_snapshot_missing).",
                },
            ]
        elif family["id"] == "grok-build" and surface["id"] == "cli":
            fields = [
                {
                    "native_field": "inspect --json discovered configuration",
                    "lifecycle_stage": "discoverable",
                    "claim_kind": "observed",
                    "coverage": "partial-declared-surface",
                    "minimum_evidence": "native-runtime",
                    "notes": "Discovered configuration does not automatically mean model-visible.",
                }
            ]
        else:
            fields = [
                {
                    "native_field": "declared-static-path",
                    "lifecycle_stage": "installed",
                    "claim_kind": "resolved",
                    "coverage": "partial-declared-surface",
                    "minimum_evidence": "official-spec",
                }
            ]
        docs[key] = {
            "schema_version": SCHEMA_VERSION,
            "cutoff": CUTOFF,
            "artifact": "field-to-claim",
            "family_id": family["id"],
            "surface": surface["id"],
            "version": surface["version"],
            "live_tested": False,
            "fields": fields,
        }
    return docs


def reference_hardware_md() -> str:
    return f"""# Reference hardware and dataset freeze

> 状态：规范（尚未实施产品运行时）
> cutoff: `{CUTOFF}`

本文件冻结性能测试对照条件。它不是一份已跑分报告。Ubuntu 官方 live-server ISO SHA-256 与 Windows 11 24H2 官方 build 已按 cutoff 前第一方目录固定。Windows ISO 内容 SHA-256 未被 Microsoft 放入稳定 checksum catalog，因此记录 `{DIGEST_NOT_PUBLISHED}`，禁止编造。

## Research capture machine (not the performance minimum)

| Field | Value | Honesty |
| --- | --- | --- |
| OS lane | macOS 27.0 build 26A5425a arm64 | Captured in `docs/research/2026-09-04-local-environment-receipt.md` |
| CPU | Apple M5 Pro | Captured |
| Memory | 51539607552 bytes | Captured |
| Logical CPUs | 18 | Captured |
| Filesystem | APFS | implied by macOS lane; not independently hashed |
| Role | research / documentation capture | Must not replace the three performance gate lanes |

Do not copy absolute home directories into other publishable files.

## Performance gate lanes (PRD §14.2)

These lanes are required. Results are not recorded in this stage.

| Lane id | Hardware / OS | Filesystem | Frozen image / build | Status |
| --- | --- | --- | --- | --- |
| perf-macos-m2 | Apple M2 / 16GB / NVMe / macOS | APFS | research receipt is not this lane | not-run |
| perf-ubuntu-24.04 | Ubuntu 24.04 VM 4 vCPU / 8GB | ext4 | `{UBUNTU_24_04["image_name"]}` `{UBUNTU_24_04["image_digest"]}` | not-run; official image frozen |
| perf-windows-11-24h2 | Windows 11 4 core / 16GB | NTFS | Windows 11 24H2 build `{WINDOWS_11_24H2["build"]}` / `{WINDOWS_11_24H2["kb"]}`; ISO digest `{DIGEST_NOT_PUBLISHED}` | not-run; official build frozen |

Source facts: `docs/research/2026-09-04-source-backed-coordinates.md`.

Gate method: record exact hardware, dataset digest, cold cache and warm cache five times each; use the median; any single run above 2× target fails.

## Dataset generator freeze

| Dataset | Generator contract | Digest |
| --- | --- | --- |
| medium-repo | 100000 ordinary files, 1000 candidate context assets, 100 symlinks, 20 nested roots | `generator:medium-repo:v1`; content digest awaits generator implementation in WP-02+ |
| doctor-corpus | see `acceptance/corpus/development/doctor/` | file digests recorded in `acceptance/corpus-manifest.json` |
| static-golden | 60 generated cases per required-supported coordinate | file digests recorded in `acceptance/corpus-manifest.json` |

Generated fixtures are Apache-2.0 synthetic data. They are not live native observations.

## Cache conditions

- Cold: drop filesystem cache according to OS procedure before each of five runs.
- Warm: immediately repeat the same scan without deleting the local snapshot DB.
- Daemon idle window: 30 minutes, 1 second sample, importer idle.

## Explicit non-claims

- This stage did not run the 5s cold / 500ms incremental scan.
- Research-machine specs are not the pass bar.
- Windows ISO bytes were not hashed locally; the frozen identity is the official 24H2 build number.
"""


def _capability_cycle(family_id: str, surface_id: str, static_support: str) -> list[str]:
    ids = []
    for cap in CAPABILITIES:
        status = capability_status(family_id, surface_id, cap["id"], static_support)
        if status == "required-supported" and cap["id"] in STATIC_CAPABILITY_IDS:
            ids.append(cap["id"])
    if not ids:
        ids = ["instructions"]
    return ids


def _write_input_dir(root: Path, input_dir: str, files: dict[str, str]) -> tuple[str, list[dict]]:
    listed = []
    for name, body in sorted(files.items()):
        rel = f"{input_dir}/{name}"
        digest = write_bytes(root / rel, body)
        listed.append({"path": rel, "digest_sha256": digest})
    return sha256_text(canonical_json(listed)), listed


def _static_payload(
    coord: dict,
    case_id: str,
    class_name: str,
    polarity: str,
    capability_id: str,
    input_dir: str,
    input_digest: str,
    listed: list[dict],
    files: dict[str, str] | None = None,
) -> dict:
    parsed = interpret_static(
        coord["family_id"],
        capability_id,
        files or {},
        surface=coord.get("surface"),
        os_lane=coord.get("os_lane"),
    )
    payload = {
        "id": case_id,
        "schema_version": SCHEMA_VERSION,
        "corpus": "development",
        "kind": "static",
        "class": class_name,
        "polarity": polarity,
        "coordinate_id": coord["id"],
        "family_id": coord["family_id"],
        "version": coord["version"],
        "surface": coord["surface"],
        "os_lane": coord["os_lane"],
        "capability_id": capability_id,
        "license": LICENSE_ID,
        "origin": "generated-fixture",
        "sensitivity": "public-synthetic",
        "live_tested": False,
        "input_path": input_dir,
        "input_files": listed,
        "input_digest_sha256": input_digest,
        "expected_claim": expected_claim_from_parse(parsed, polarity),
        "expected_output": parsed,
    }
    payload["digest"] = _case_digest(payload)
    return payload


def static_cases_for(coord: dict, root: Path) -> list[dict]:
    rows = []
    seen_ids: set[str] = set()
    family_id = coord["family_id"]
    surface = coord["surface"]
    required_caps: list[str] = []
    unknown_caps: list[str] = []
    loss_caps: list[str] = []
    for cap in CAPABILITIES:
        status = capability_status(family_id, surface, cap["id"], "required-supported")
        if status == "required-supported":
            required_caps.append(cap["id"])
        elif status == "required-unknown-honesty":
            unknown_caps.append(cap["id"])
        elif status == "not-applicable":
            if (family_id in NA_RULES_FAMILIES and cap["id"] == "rules") or (
                family_id in NO_SKILLS_FAMILIES and cap["id"] in {"skills", "plugins"}
            ):
                loss_caps.append(cap["id"])

    def emit(case_id: str, class_name: str, polarity: str, capability_id: str, idx: int) -> None:
        if case_id in seen_ids:
            return
        slug = case_id.replace(":", "__").replace("/", "__")
        input_dir = f"acceptance/corpus/development/static/inputs/{slug}"
        files = build_static_files(family_id, capability_id, polarity, class_name)
        input_digest, listed = _write_input_dir(root, input_dir, files)
        rows.append(
            _static_payload(
                coord, case_id, class_name, polarity, capability_id, input_dir, input_digest, listed, files
            )
        )
        seen_ids.add(case_id)

    for cap_id in required_caps:
        emit(f"dev:static:{coord['id']}:{cap_id}:positive", "include", "positive", cap_id, 0)
        emit(f"dev:static:{coord['id']}:{cap_id}:negative", "negative", "negative", cap_id, 0)
    for cap_id in unknown_caps:
        emit(
            f"dev:static:{coord['id']}:{cap_id}:indeterminate",
            "unknown_honesty",
            "indeterminate",
            cap_id,
            0,
        )
    for cap_id in loss_caps:
        emit(f"dev:static:{coord['id']}:{cap_id}:loss", "loss", "loss", cap_id, 0)

    pad = 0
    while len(rows) < 60:
        class_name = STATIC_CASE_CLASSES[pad % len(STATIC_CASE_CLASSES)]
        cap_id = required_caps[pad % len(required_caps)] if required_caps else "instructions"
        if class_name in {"exclude", "ignore", "negative"}:
            polarity = "negative"
        elif class_name == "unknown_honesty":
            polarity = "indeterminate"
            cap_id = unknown_caps[pad % len(unknown_caps)] if unknown_caps else "other-unknown"
        elif class_name == "loss":
            if loss_caps:
                polarity = "loss"
                cap_id = loss_caps[0]
            else:
                class_name = "include"
                polarity = "positive"
        else:
            polarity = "positive"
        case_id = f"dev:static:{coord['id']}:{class_name}:{pad:02d}"
        emit(case_id, class_name, polarity, cap_id, pad)
        pad += 1
        if pad > 200:
            break
    return rows


def honesty_cases(root: Path, covered_coord_ids: set[str]) -> list[dict]:
    """One indeterminate case per required-unknown-honesty cell not already covered."""
    rows = []
    for cell in canonical_capability_cells():
        if cell["status"] != "required-unknown-honesty":
            continue
        coord_id = cell["coordinate_id"]
        if coord_id in covered_coord_ids:
            continue
        family_id = cell["family_id"]
        cap_id = cell["capability_id"]
        slug = f"{family_id}__{cell['surface']}__{cell['os_lane']}__{cap_id}__unknown-honesty"
        input_dir = f"acceptance/corpus/development/static/inputs/{slug}"
        files = build_static_files(family_id, cap_id, "indeterminate", "unknown_honesty")
        input_digest, listed = _write_input_dir(root, input_dir, files)
        coord = {
            "id": coord_id,
            "family_id": family_id,
            "version": cell["version"],
            "surface": cell["surface"],
            "os_lane": cell["os_lane"],
        }
        case_id = f"dev:static:{coord_id}:{cap_id}:indeterminate"
        payload = _static_payload(
            coord,
            case_id,
            "unknown_honesty",
            "indeterminate",
            cap_id,
            input_dir,
            input_digest,
            listed,
            files,
        )
        rows.append(payload)
    return rows


def loss_cases(root: Path, covered_coord_ids: set[str]) -> list[dict]:
    """Emit the required loss cells that are not already in required-supported static JSONL."""
    rows = []
    seen: set[str] = set()
    for cell in required_loss_cells():
        coord_id = cell["coordinate_id"]
        case_id = f"dev:static:{coord_id}:{cell['capability_id']}:loss"
        if case_id in seen:
            continue
        seen.add(case_id)
        if coord_id in covered_coord_ids:
            continue
        family_id = cell["family_id"]
        cap_id = cell["capability_id"]
        slug = f"{family_id}__{cell['surface']}__{cell['os_lane']}__{cap_id}__loss"
        input_dir = f"acceptance/corpus/development/static/inputs/{slug}"
        files = build_static_files(family_id, cap_id, "loss", "loss")
        input_digest, listed = _write_input_dir(root, input_dir, files)
        coord = {
            "id": coord_id,
            "family_id": family_id,
            "version": cell["version"],
            "surface": cell["surface"],
            "os_lane": cell["os_lane"],
        }
        rows.append(
            _static_payload(
                coord,
                case_id,
                "loss",
                "loss",
                cap_id,
                input_dir,
                input_digest,
                listed,
                files,
            )
        )
    return rows


def _oracle_workspace(
    coord: dict, class_name: str, capability_id: str, polarity: str, command: list[str], shape: dict, root: Path
) -> tuple[str, str, list[dict], dict[str, str]]:
    family = coord["family_id"]
    input_dir = (
        f"acceptance/corpus/development/oracle/inputs/"
        f"{family}/{coord['oracle_id']}/{class_name}-{capability_id}-{polarity}"
    )
    files = build_oracle_files(family, capability_id, polarity, command, shape)
    digest, listed = _write_input_dir(root, input_dir, files)
    return input_dir, digest, listed, files


def oracle_cases_for(coord: dict, root: Path) -> list[dict]:
    rows = []
    key = f"{coord['family_id']}:{coord['surface']}"
    oracle_caps = list(ORACLE_CAPABILITY_IDS.get(key, ()))
    base_shape = dict(ORACLE_NATIVE_SHAPES[coord["oracle_id"]])
    command = list(coord["command"])
    default_cap = "task-message" if coord["oracle_id"] == "debug-prompt-input" else "instructions"
    class_map = {
        "include_observed": ("positive", oracle_caps[0] if oracle_caps else default_cap),
        "exclude_not_in_export": ("negative", oracle_caps[0] if oracle_caps else default_cap),
        "partial_coverage": ("positive", oracle_caps[1] if len(oracle_caps) > 1 else default_cap),
        "unknown_core_prompt": ("indeterminate", oracle_caps[1] if len(oracle_caps) > 1 else default_cap),
        "cap_truncation": ("positive", oracle_caps[-1] if oracle_caps else default_cap),
        "skill_catalog_vs_body": ("negative", oracle_caps[-1] if oracle_caps else default_cap),
        "mcp_schema_partial": ("positive", oracle_caps[-1] if oracle_caps else default_cap),
        "negative_missing_item": ("negative", oracle_caps[1] if len(oracle_caps) > 1 else default_cap),
        "cwd_change": ("positive", default_cap),
        "version_pin": ("positive", oracle_caps[0] if oracle_caps else default_cap),
        "redaction": ("negative", oracle_caps[0] if oracle_caps else default_cap),
        "repeat_digest_stability": ("positive", oracle_caps[1] if len(oracle_caps) > 1 else default_cap),
    }

    def emit(case_id: str, class_name: str, polarity: str, capability_id: str, shape: dict) -> None:
        input_dir, input_digest, listed, files = _oracle_workspace(
            coord, class_name, capability_id, polarity, command, shape, root
        )
        parsed = interpret_oracle(coord["family_id"], capability_id, files)
        claim = expected_claim_from_parse(parsed, polarity)
        claim["claim_kind"] = "observed"
        claim["lifecycle_stage"] = "model-visible" if coord["oracle_id"] == "debug-prompt-input" else "discoverable"
        claim["provenance"] = "native-runtime"
        payload = {
            "id": case_id,
            "schema_version": SCHEMA_VERSION,
            "corpus": "development",
            "kind": "oracle",
            "class": class_name,
            "polarity": polarity,
            "capability_id": capability_id,
            "coordinate_id": coord["id"],
            "family_id": coord["family_id"],
            "version": coord["version"],
            "surface": coord["surface"],
            "os_lane": coord["os_lane"],
            "oracle_id": coord["oracle_id"],
            "command": command,
            "license": LICENSE_ID,
            "origin": "generated-fixture-recipe",
            "sensitivity": "public-synthetic",
            "live_tested": False,
            "native_result_captured": False,
            "input_path": input_dir,
            "input_files": listed,
            "input_digest_sha256": input_digest,
            "expected_native_shape": shape,
            "expected_output": {**parsed, "expected_native_shape": shape},
            "expected_claim": claim,
            "recipe": {
                "class": class_name,
                "repeatable": True,
                "capability_id": capability_id,
                "notes": "Recipe plus synthetic workspace. This foundation stage did not execute the harness.",
            },
        }
        payload["digest"] = _case_digest(payload)
        rows.append(payload)

    for class_name in ORACLE_CASE_CLASSES:
        polarity, cap_id = class_map.get(class_name, ("positive", default_cap))
        shape = dict(base_shape)
        if polarity == "negative":
            shape["absent_keys"] = ["undeclared-item"]
        emit(f"dev:oracle:{coord['id']}:{class_name}", class_name, polarity, cap_id, shape)
    for cap_id in oracle_caps:
        for polarity in ("positive", "negative"):
            case_id = f"dev:oracle:{coord['id']}:{cap_id}:{polarity}"
            if any(row["id"] == case_id for row in rows):
                continue
            emit(case_id, f"capability-{polarity}", polarity, cap_id, dict(base_shape))
    return rows


def doctor_cases(root: Path) -> list[dict]:
    rows = []
    doctor_coord = "doctor/development/macos-27-arm64"

    def emit(payload: dict, files: dict[str, str], findings: list[dict]) -> None:
        input_dir = f"acceptance/corpus/development/doctor/inputs/{payload['id']}"
        input_files = []
        for name, body in sorted(files.items()):
            rel = f"{input_dir}/{name}"
            digest = write_bytes(root / rel, body)
            input_files.append({"path": rel, "digest_sha256": digest})
        payload.setdefault("schema_version", SCHEMA_VERSION)
        payload["coordinate_id"] = doctor_coord
        payload["input_path"] = input_dir
        payload["input_files"] = input_files
        payload["input_digest_sha256"] = sha256_text(canonical_json(input_files))
        payload["expected_findings"] = findings
        payload["expected_output"] = {
            "resolver": "doctor",
            "findings": findings,
            "native_paths_used": sorted(name for name in files if name != "envelope.json"),
        }
        payload["digest"] = _case_digest(payload)
        rows.append(payload)

    for idx in range(125):
        files, findings = build_doctor_files(None, "negative", "clean", idx)
        emit(
            {
                "id": f"dev:doctor:clean:{idx:03d}",
                "corpus": "development",
                "kind": "doctor",
                "class": "clean",
                "rule_id": None,
                "blocking": False,
                "polarity": "negative",
                "expected_exit": 0,
                "license": LICENSE_ID,
                "origin": "generated-fixture",
                "sensitivity": "public-synthetic",
                "live_tested": False,
            },
            files,
            findings,
        )

    for rule in DOCTOR_BLOCKING_RULES:
        for polarity, expected_exit, klass in (
            ("positive", 2, "reachable-issue"),
            ("negative", 0, "clean-lookalike"),
        ):
            for idx in range(20):
                files, findings = build_doctor_files(rule, polarity, klass, idx)
                emit(
                    {
                        "id": f"dev:doctor:{rule}:{polarity}:{idx:02d}",
                        "corpus": "development",
                        "kind": "doctor",
                        "class": klass,
                        "rule_id": rule,
                        "blocking": True,
                        "polarity": polarity,
                        "expected_exit": expected_exit,
                        "license": LICENSE_ID,
                        "origin": "generated-fixture",
                        "sensitivity": "public-synthetic",
                        "live_tested": False,
                    },
                    files,
                    findings,
                )

    for rule in DOCTOR_NONBLOCKING_RULES:
        for polarity, expected_exit, klass in (
            ("positive", 0, "reachable-issue"),
            ("negative", 0, "clean-lookalike"),
        ):
            count = 8 if polarity == "positive" else 4
            for idx in range(count):
                files, findings = build_doctor_files(rule, polarity, klass, idx)
                emit(
                    {
                        "id": f"dev:doctor:{rule}:{polarity}:{idx:02d}",
                        "corpus": "development",
                        "kind": "doctor",
                        "class": klass,
                        "rule_id": rule,
                        "blocking": False,
                        "polarity": polarity,
                        "expected_exit": expected_exit,
                        "warning_only": True,
                        "license": LICENSE_ID,
                        "origin": "generated-fixture",
                        "sensitivity": "public-synthetic",
                        "live_tested": False,
                    },
                    files,
                    findings,
                )
    return rows


def sealed_stubs(static_coords, oracle_coords) -> list[dict]:
    rows = []
    for coord in static_coords:
        for idx in range(2):
            payload = {
                "id": f"sealed:static:{coord['id']}:withheld:{idx:02d}",
                "schema_version": SCHEMA_VERSION,
                "corpus": "sealed",
                "kind": "static",
                "coordinate_id": coord["id"],
                "answers_status": "withheld-until-rc",
                "license": LICENSE_ID,
                "origin": "independent-maintainer-slot",
                "sensitivity": "withheld",
                "live_tested": False,
                "recipe": {"withheld": True},
            }
            payload["digest"] = _case_digest(payload)
            rows.append(payload)
    for coord in oracle_coords:
        payload = {
            "id": f"sealed:oracle:{coord['id']}:withheld",
            "schema_version": SCHEMA_VERSION,
            "corpus": "sealed",
            "kind": "oracle",
            "coordinate_id": coord["id"],
            "answers_status": "withheld-until-rc",
            "license": LICENSE_ID,
            "origin": "independent-maintainer-slot",
            "sensitivity": "withheld",
            "live_tested": False,
            "recipe": {"withheld": True},
        }
        payload["digest"] = _case_digest(payload)
        rows.append(payload)
    for idx in range(20):
        payload = {
            "id": f"sealed:doctor:withheld:{idx:02d}",
            "schema_version": SCHEMA_VERSION,
            "corpus": "sealed",
            "kind": "doctor",
            "coordinate_id": "doctor/sealed/withheld",
            "answers_status": "withheld-until-rc",
            "license": LICENSE_ID,
            "origin": "independent-maintainer-slot",
            "sensitivity": "withheld",
            "live_tested": False,
            "recipe": {"withheld": True},
        }
        payload["digest"] = _case_digest(payload)
        rows.append(payload)
    return rows


def live_recipes(oracle_coords) -> list[dict]:
    rows = []
    for coord in oracle_coords:
        payload = {
            "id": f"live:oracle:{coord['id']}:recipe",
            "schema_version": SCHEMA_VERSION,
            "corpus": "live",
            "kind": "oracle-recipe",
            "coordinate_id": coord["id"],
            "family_id": coord["family_id"],
            "command": coord["command"],
            "executed": False,
            "result": "recipe-only-not-a-native-observation",
            "license": LICENSE_ID,
            "origin": "live-recipe-only",
            "sensitivity": "redacted-recipe",
            "live_tested": False,
            "notes": "Run only on a matching installed harness during WP-02+. Do not treat this row as a native observation.",
        }
        payload["digest"] = _case_digest(payload)
        rows.append(payload)
    return rows


NATIVE_SYNTHETIC_FILES = ("session.jsonl", "expected.json", "meta.json")


def native_synthetic_fixtures(root: Path) -> list[dict]:
    """Register the native-synthetic corpus: session logs produced by a
    harness's own public API (`scripts/fixtures/<family>/generate.ts` run
    inside the pinned checkout), with expected values the harness computed.
    The generator does not produce these files; it records their digests so
    `check_acceptance.py --corpus` can hold them fixed."""
    base = root / "acceptance/corpus/development/native"
    out: list[dict] = []
    if not base.is_dir():
        return out
    for folder in sorted(p for p in base.iterdir() if p.is_dir()):
        files = []
        for name in NATIVE_SYNTHETIC_FILES:
            path = folder / name
            if not path.is_file():
                raise SystemExit(f"native-synthetic fixture {folder.name} lacks {name}")
            files.append(
                {
                    "path": path.relative_to(root).as_posix(),
                    "digest_sha256": sha256_text(path.read_text(encoding="utf-8")),
                    "license": LICENSE_ID,
                }
            )
        meta = json.loads((folder / "meta.json").read_text(encoding="utf-8"))
        out.append(
            {
                "id": f"dev:native:{folder.name}",
                "corpus": "native-synthetic",
                "kind": "native-session",
                "path": files[0]["path"],
                "files": files,
                "generator": meta.get("script"),
                "harness_sha": meta.get("dsh_sha"),
                "harness_version": meta.get("dsh_session_version"),
                "harness_license": meta.get("dsh_license"),
                "license": LICENSE_ID,
                "origin": "generated-by-harness-public-api",
                "sensitivity": "public-synthetic",
                "live_tested": False,
            }
        )
    return out


def corpus_manifest(entries: list[dict], file_digests: dict[str, str], counts: dict, native: list[dict]) -> dict:
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "corpus-manifest",
        "license": LICENSE_ID,
        "corpora": {
            "development": {
                "role": "everyday implementation fixtures; answers visible",
                "live_tested": False,
            },
            "sealed": {
                "role": "independent maintainer conformance; answers withheld until release candidate freeze",
                "answers_status": "withheld-until-rc",
                "live_tested": False,
            },
            "live": {
                "role": "temporary native-oracle recipes from real anchor harnesses; this stage records recipes only",
                "executed": False,
                "live_tested": False,
            },
            "native-synthetic": {
                "role": "synthetic session logs written by a harness's own public API at a pinned SHA, with expected values the harness computed; the Rust importer is checked against them (native_conformance gate)",
                "executed_harness": False,
                "live_tested": False,
                "widens_oracles": False,
            },
        },
        "counts": counts,
        "native_synthetic": native,
        "files": [
            {"path": path, "digest_sha256": digest, "license": LICENSE_ID}
            for path, digest in sorted(file_digests.items())
        ],
        "fixtures": entries,
        "non_claims": [
            "Generated fixtures are not live tests.",
            "Native-synthetic sessions are synthetic; no harness was executed and no oracle was widened.",
            "Sealed answers are withheld from implementers in this repository.",
            "Live recipes were not executed in the foundation stage.",
            "Security-critical fixtures are listed separately from the 60/12 floors.",
        ],
    }


def _fixture_entry(row: dict, rel: str, extra: dict | None = None) -> dict:
    entry = {
        "id": row["id"],
        "schema_version": row.get("schema_version", SCHEMA_VERSION),
        "corpus": row["corpus"],
        "kind": row["kind"],
        "coordinate_id": row.get("coordinate_id") or "unspecified",
        "path": rel,
        "digest_sha256": row["digest"],
        "license": LICENSE_ID,
        "origin": row.get("origin", "generated"),
        "sensitivity": row.get("sensitivity") or "public-synthetic",
        "live_tested": False,
    }
    for key in (
        "class",
        "family_id",
        "version",
        "surface",
        "os_lane",
        "capability_id",
        "oracle_id",
        "command",
        "expected_native_shape",
        "input_files",
    ):
        if row.get(key) is not None and key not in entry:
            entry[key] = row[key]
    if extra:
        entry.update(extra)
    return entry


def artifact_digest_manifest(root: Path) -> dict:
    artifacts = []
    for rel, schema_id in ARTIFACT_SCHEMA_IDS.items():
        path = root / rel
        artifacts.append(
            {
                "path": rel,
                "digest_sha256": sha256_text(path.read_text(encoding="utf-8")),
                "schema_id": schema_id,
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "cutoff": CUTOFF,
        "artifact": "artifact-digest-manifest",
        "artifacts": artifacts,
        "notes": (
            "Digests of the eight PRD §17.0 artifacts plus "
            "acceptance/semantic-team-contract.yaml after deterministic generation. "
            "Fixture files under acceptance/semantic-team/ are hashed inside the "
            "semantic-team contract content_digest_manifest."
        ),
    }


def main() -> int:
    root = ROOT
    acceptance = root / "acceptance"
    acceptance.mkdir(parents=True, exist_ok=True)

    write_json(acceptance / "compatibility-matrix.yaml", compatibility_matrix())
    write_json(acceptance / "claim-validity-matrix.yaml", claim_validity_matrix())
    write_json(acceptance / "context-capability-matrix.yaml", context_capability_matrix())
    write_json(acceptance / "projection-matrix.yaml", projection_matrix())
    write_json(acceptance / "integration-contracts.yaml", integration_contracts())
    (acceptance / "reference-hardware.md").write_text(reference_hardware_md(), encoding="utf-8")

    prd = load_prd_text(root)
    statements = [assign_trace_metadata(item) for item in extract_normative_statements(prd)]
    write_traceability_csv(acceptance / "traceability.csv", statements)

    mappings = field_mapping_docs()
    mapping_dir = acceptance / "field-to-claim"
    mapping_dir.mkdir(parents=True, exist_ok=True)
    for key, doc in mappings.items():
        write_json(mapping_dir / f"{key}.yaml", doc)

    static_coords = required_supported_static_coordinates()
    oracle_coords = declared_oracle_coordinates()
    file_digests: dict[str, str] = {}
    fixture_entries: list[dict] = []
    for stale in (
        "acceptance/corpus/development/static/inputs",
        "acceptance/corpus/development/oracle/inputs",
        "acceptance/corpus/development/doctor/inputs",
        "acceptance/semantic-team",
    ):
        stale_path = root / stale
        if stale_path.exists():
            shutil.rmtree(stale_path)

    static_dir = acceptance / "corpus" / "development" / "static"
    static_dir.mkdir(parents=True, exist_ok=True)
    grouped_static: dict[str, list[dict]] = {}
    for coord in static_coords:
        rel = f"acceptance/corpus/development/static/{coord['family_id']}__{coord['surface']}.jsonl"
        grouped_static.setdefault(rel, []).extend(static_cases_for(coord, root))
    for rel, rows in grouped_static.items():
        digest = write_jsonl(root / rel, rows)
        file_digests[rel] = digest
        for row in rows:
            fixture_entries.append(
                _fixture_entry(
                    row,
                    rel,
                    {
                        "kind": "static",
                        "coordinate_id": row["coordinate_id"],
                        "input_path": row["input_path"],
                        "input_digest_sha256": row["input_digest_sha256"],
                        "golden_claim": row["expected_claim"],
                    },
                )
            )

    honesty_rel = "acceptance/corpus/development/static/unknown-honesty-cells.jsonl"
    honesty_rows = honesty_cases(root, {coord["id"] for coord in static_coords})
    file_digests[honesty_rel] = write_jsonl(root / honesty_rel, honesty_rows)
    for row in honesty_rows:
        fixture_entries.append(
            _fixture_entry(
                row,
                honesty_rel,
                {
                    "kind": "static",
                    "coordinate_id": row["coordinate_id"],
                    "input_path": row["input_path"],
                    "input_digest_sha256": row["input_digest_sha256"],
                    "golden_claim": row["expected_claim"],
                },
            )
        )

    loss_rel = "acceptance/corpus/development/static/loss-cells.jsonl"
    loss_rows = loss_cases(root, {coord["id"] for coord in static_coords})
    file_digests[loss_rel] = write_jsonl(root / loss_rel, loss_rows)
    for row in loss_rows:
        fixture_entries.append(
            _fixture_entry(
                row,
                loss_rel,
                {
                    "kind": "static",
                    "coordinate_id": row["coordinate_id"],
                    "input_path": row["input_path"],
                    "input_digest_sha256": row["input_digest_sha256"],
                    "golden_claim": row["expected_claim"],
                },
            )
        )

    oracle_dir = acceptance / "corpus" / "development" / "oracle"
    oracle_dir.mkdir(parents=True, exist_ok=True)
    for coord in oracle_coords:
        rows = oracle_cases_for(coord, root)
        rel = f"acceptance/corpus/development/oracle/{coord['family_id']}__{coord['oracle_id']}.jsonl"
        digest = write_jsonl(root / rel, rows)
        file_digests[rel] = digest
        for row in rows:
            fixture_entries.append(
                _fixture_entry(
                    row,
                    rel,
                    {
                        "kind": "oracle",
                        "coordinate_id": coord["id"],
                        "input_path": row["input_path"],
                        "input_digest_sha256": row["input_digest_sha256"],
                        "golden_claim": row["expected_claim"],
                        "native_result_captured": False,
                    },
                )
            )

    doctor_rows = doctor_cases(root)
    rel = "acceptance/corpus/development/doctor/doctor-corpus.jsonl"
    file_digests[rel] = write_jsonl(root / rel, doctor_rows)
    for row in doctor_rows:
        fixture_entries.append(
            _fixture_entry(
                row,
                rel,
                {
                    "kind": "doctor",
                    "class": row["class"],
                    "rule_id": row.get("rule_id"),
                    "coordinate_id": row["coordinate_id"],
                    "input_path": row["input_path"],
                    "input_digest_sha256": row["input_digest_sha256"],
                    "blocking": row.get("blocking"),
                    "polarity": row.get("polarity"),
                },
            )
        )

    sealed_rows = sealed_stubs(static_coords, oracle_coords)
    rel = "acceptance/corpus/sealed/stubs.jsonl"
    file_digests[rel] = write_jsonl(root / rel, sealed_rows)
    (root / "acceptance/corpus/sealed/README.md").write_text(
        """# Sealed conformance corpus

> 状态：规范（尚未实施产品运行时）

Sealed golden answers are withheld until a release-candidate freeze.
This repository only stores case identifiers, licenses, and recipe stubs.
Implementers must not special-case sealed or live answers.
""",
        encoding="utf-8",
    )
    for row in sealed_rows:
        fixture_entries.append(
            _fixture_entry(
                row,
                rel,
                {
                    "answers_status": "withheld-until-rc",
                    "coordinate_id": row["coordinate_id"],
                    "sensitivity": row["sensitivity"],
                },
            )
        )

    live_rows = live_recipes(oracle_coords)
    rel = "acceptance/corpus/live/oracle-recipes.jsonl"
    file_digests[rel] = write_jsonl(root / rel, live_rows)
    for row in live_rows:
        fixture_entries.append(
            _fixture_entry(
                row,
                rel,
                {
                    "kind": "oracle-recipe",
                    "executed": False,
                    "coordinate_id": row["coordinate_id"],
                },
            )
        )

    native = native_synthetic_fixtures(root)
    clean = sum(1 for r in doctor_rows if r["class"] in {"clean", "clean-lookalike"})
    issue = sum(1 for r in doctor_rows if r["class"] == "reachable-issue")
    counts = {
        "required_supported_static_coordinates": len(static_coords),
        "static_cases_per_required_supported_coordinate": 60,
        "declared_oracle_coordinates": len(oracle_coords),
        "oracle_cases_per_declared_oracle": 12,
        "doctor_total": len(doctor_rows),
        "doctor_clean_or_lookalike": clean,
        "doctor_reachable_issue": issue,
        "doctor_dedicated_clean": 125,
        "families": len(family_ids()),
        "anchor_families": ANCHOR_IDS,
        "native_synthetic_sessions": len(native),
    }
    write_json(acceptance / "corpus-manifest.json", corpus_manifest(fixture_entries, file_digests, counts, native))
    write_semantic_team_artifacts(root)
    write_json(root / ARTIFACT_DIGEST_MANIFEST, artifact_digest_manifest(root))

    missing = [rel for rel in ACCEPTANCE_ARTIFACTS if not (root / rel).exists()]
    if missing:
        raise SystemExit("missing artifacts: " + ", ".join(missing))
    print(f"generated {len(statements)} traceability rows")
    print(f"static coordinates: {len(static_coords)}")
    print(f"oracle coordinates: {len(oracle_coords)}")
    print(f"doctor cases: {len(doctor_rows)} clean+lookalike={clean} issue={issue}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
