#!/usr/bin/env python3
"""Closed, exact schemas for the eight foundation acceptance artifacts.

Canonical keysets and Cartesian products come from immutable contract
constants, never from the candidate artifact under validation.
"""

from __future__ import annotations

from collections import Counter
from typing import Any

from contexpect_contract import (
    ARTIFACT_ALLOWED_TOP_LEVEL,
    ARTIFACT_REQUIRED_TOP_LEVEL,
    ARTIFACT_SCHEMA_IDS,
    CAPABILITY_CELL_ALLOWED_FIELDS,
    CAPABILITY_CELL_REQUIRED_FIELDS,
    CLAIM_AXES,
    CLAIM_INVARIANT_IDS,
    CLAIM_INVARIANTS,
    CLAIM_PRECEDENCE,
    CLAIM_RECONCILIATION_INVARIANTS,
    CLAIM_RECONCILIATION_STATES,
    CLAIM_USE_EVIDENCE_KINDS,
    COORDINATE_ALLOWED_FIELDS,
    COORDINATE_REQUIRED_FIELDS,
    CUTOFF,
    DIGEST_ENTRY_REQUIRED_FIELDS,
    FIELD_MAP_ALLOWED_TOP_LEVEL,
    FIELD_MAP_ARTIFACT,
    FIELD_MAP_FIELD_ALLOWED,
    FIELD_MAP_FIELD_REQUIRED,
    FIELD_MAP_REQUIRED_TOP_LEVEL,
    INTEGRATION_CANONICAL_IDS,
    INTEGRATION_CONTRACT_REQUIRED_FIELDS,
    LIVE_INSTALL_ALLOWED_FIELDS,
    LIVE_INSTALL_REQUIRED_FIELDS,
    NATIVE_TARGET_ALLOWED_FIELDS,
    NATIVE_TARGET_REQUIRED_FIELDS,
    OS_LANES,
    PROJECTION_CELL_ALLOWED_FIELDS,
    PROJECTION_REQUIRED_WRITE_FIELDS,
    SCHEMA_VERSION,
    UNKNOWN_REASON_CODES,
    canonical_capability_cells,
    canonical_compatibility_coordinates,
    canonical_compatibility_matrix,
    capability_cell_id,
    coordinate_id,
    family_ids,
    field_map_relpath,
    frozen_surface,
    projection_cells,
    require_schema_version,
)

# Imported for validators that compare identity strings.
ARTIFACT_IDENTITY = {
    "compatibility-matrix": "compatibility-matrix",
    "claim-validity-matrix": "claim-validity-matrix",
    "context-capability-matrix": "context-capability-matrix",
    "projection-matrix": "projection-matrix",
    "integration-contracts": "integration-contracts",
    "corpus-manifest": "corpus-manifest",
    "artifact-digest-manifest": "artifact-digest-manifest",
}


def reject_unknown_fields(obj: dict[str, Any], allowed: list[str], label: str, errors: list[str]) -> None:
    extra = sorted(set(obj) - set(allowed))
    if extra:
        errors.append(f"{label} unknown fields {extra}")


def reject_missing_fields(obj: dict[str, Any], required: list[str], label: str, errors: list[str]) -> None:
    missing = sorted(set(required) - set(obj))
    if missing:
        errors.append(f"{label} missing required fields {missing}")


def reject_empty(value: Any, label: str, errors: list[str]) -> None:
    if value is None:
        errors.append(f"{label} is empty")
        return
    if isinstance(value, str) and not value.strip():
        errors.append(f"{label} is empty")
    if isinstance(value, (list, dict, tuple)) and len(value) == 0:
        errors.append(f"{label} is empty")


def require_closed_object(
    obj: Any,
    allowed: list[str],
    required: list[str],
    label: str,
    errors: list[str],
    nonempty: list[str] | None = None,
) -> bool:
    if not isinstance(obj, dict):
        errors.append(f"{label} is not an object")
        return False
    reject_unknown_fields(obj, allowed, label, errors)
    reject_missing_fields(obj, required, label, errors)
    for key in nonempty or []:
        reject_empty(obj.get(key), f"{label}.{key}", errors)
    return True


def reject_duplicate_values(values: list[Any], label: str, errors: list[str]) -> None:
    counts = Counter(values)
    dupes = sorted(key for key, count in counts.items() if count > 1 and key not in (None, ""))
    if dupes:
        errors.append(f"{label} duplicate values {dupes[:8]}")


def require_exact_sequence(actual: Any, expected: Any, label: str, errors: list[str]) -> None:
    if actual != expected:
        errors.append(f"{label} does not match the canonical constant sequence")


def require_exact_mapping(actual: Any, expected: dict[str, Any], label: str, errors: list[str]) -> None:
    if not isinstance(actual, dict):
        errors.append(f"{label} is not an object")
        return
    reject_empty(actual, label, errors)
    extra = sorted(set(actual) - set(expected))
    missing = sorted(set(expected) - set(actual))
    if extra:
        errors.append(f"{label} unknown keys {extra}")
    if missing:
        errors.append(f"{label} missing keys {missing}")
    for key, want in expected.items():
        if key in actual and actual[key] != want:
            errors.append(f"{label}.{key} does not match the canonical constant")
            reject_empty(actual[key], f"{label}.{key}", errors)


def validate_artifact_header(obj: dict[str, Any], identity: str, errors: list[str]) -> None:
    require_closed_object(
        obj,
        ARTIFACT_ALLOWED_TOP_LEVEL[identity],
        ARTIFACT_REQUIRED_TOP_LEVEL[identity],
        identity,
        errors,
        nonempty=["artifact", "cutoff"],
    )
    if obj.get("artifact") != ARTIFACT_IDENTITY[identity]:
        errors.append(
            f"{identity} artifact identity {obj.get('artifact')!r} != {ARTIFACT_IDENTITY[identity]!r}"
        )
    if obj.get("cutoff") != CUTOFF:
        errors.append(f"{identity} cutoff {obj.get('cutoff')!r} != {CUTOFF}")
    require_schema_version(obj, identity, errors)


def validate_compatibility(compat: dict[str, Any], errors: list[str]) -> None:
    validate_artifact_header(compat, "compatibility-matrix", errors)
    expected = canonical_compatibility_matrix()
    require_exact_sequence(compat.get("os_lanes"), expected["os_lanes"], "compatibility-matrix.os_lanes", errors)
    require_exact_sequence(compat.get("families"), expected["families"], "compatibility-matrix.families", errors)
    if compat.get("declared_native_oracles") != expected["declared_native_oracles"]:
        errors.append("declared native oracles must equal the frozen Codex/Grok pair")
    if compat.get("honesty") != expected["honesty"]:
        errors.append("compatibility-matrix.honesty does not match canonical constants")
    if compat.get("source_backed_os") != expected["source_backed_os"]:
        errors.append("compatibility-matrix.source_backed_os does not match canonical constants")
    if family_ids() != [family["id"] for family in expected["families"]]:
        errors.append("canonical family list drifted from FAMILIES")
    if len(expected["families"]) != 18:
        errors.append("canonical family list is not 18")

    rows = compat.get("coordinates")
    if not isinstance(rows, list):
        errors.append("compatibility-matrix.coordinates is not a list")
        return
    reject_empty(rows, "compatibility-matrix.coordinates", errors)
    ids = [row.get("id") for row in rows if isinstance(row, dict)]
    reject_duplicate_values(ids, "compatibility-matrix.coordinates.id", errors)
    expected_rows = {row["id"]: row for row in canonical_compatibility_coordinates()}
    actual_rows = {row.get("id"): row for row in rows if isinstance(row, dict)}
    missing = sorted(set(expected_rows) - set(actual_rows))
    extra = sorted(set(actual_rows) - set(expected_rows))
    if missing:
        errors.append(f"compatibility-matrix missing Cartesian coordinates {missing[:8]}")
    if extra:
        errors.append(f"compatibility-matrix unexpected coordinates {extra[:8]}")
    for coord_id, expected_row in expected_rows.items():
        actual = actual_rows.get(coord_id)
        if not isinstance(actual, dict):
            continue
        require_closed_object(
            actual,
            COORDINATE_ALLOWED_FIELDS,
            COORDINATE_REQUIRED_FIELDS,
            coord_id,
            errors,
            nonempty=["id", "family_id", "version", "surface", "os_lane"],
        )
        require_closed_object(
            actual.get("live_install") or {},
            LIVE_INSTALL_ALLOWED_FIELDS,
            LIVE_INSTALL_REQUIRED_FIELDS,
            f"{coord_id}.live_install",
            errors,
            nonempty=["live_status"],
        )
        derived = coordinate_id(
            actual.get("family_id") or "",
            actual.get("version") or "",
            actual.get("surface") or "",
            actual.get("os_lane") or "",
        )
        if actual.get("id") != derived:
            errors.append(f"{coord_id} id does not match family/version/surface/os fields")
        try:
            surface = frozen_surface(expected_row["family_id"], expected_row["surface"])
        except KeyError:
            errors.append(f"{coord_id} is not a frozen family/surface")
            continue
        if actual.get("family_id") != expected_row["family_id"]:
            errors.append(f"{coord_id} family_id {actual.get('family_id')!r} != canonical {expected_row['family_id']!r}")
        if actual.get("surface") != expected_row["surface"]:
            errors.append(f"{coord_id} surface {actual.get('surface')!r} != canonical {expected_row['surface']!r}")
        if actual.get("os_lane") != expected_row["os_lane"]:
            errors.append(f"{coord_id} os_lane {actual.get('os_lane')!r} != canonical {expected_row['os_lane']!r}")
        if actual.get("version") != surface["version"]:
            errors.append(
                f"{coord_id} version {actual.get('version')!r} != frozen surface version {surface['version']!r}"
            )
        if actual.get("version") != expected_row["version"]:
            errors.append(f"{coord_id} version does not match canonical Cartesian version")
        for key in (
            "family_name",
            "cohort",
            "version_source",
            "static_support",
            "native_support",
            "native_oracle_ids",
            "fabricated_native_evidence",
        ):
            if actual.get(key) != expected_row[key]:
                errors.append(f"{coord_id} {key} {actual.get(key)!r} != canonical {expected_row[key]!r}")


def validate_claim_validity(claim: dict[str, Any], errors: list[str]) -> None:
    validate_artifact_header(claim, "claim-validity-matrix", errors)
    require_exact_mapping(claim.get("axes"), CLAIM_AXES, "claim-validity-matrix.axes", errors)
    reject_empty(claim.get("invariants"), "claim-validity-matrix.invariants", errors)
    invariants = claim.get("invariants") or []
    if not isinstance(invariants, list):
        errors.append("claim-validity-matrix.invariants is not a list")
    else:
        ids = [item.get("id") for item in invariants if isinstance(item, dict)]
        reject_duplicate_values(ids, "claim-validity-matrix.invariants.id", errors)
        if ids != CLAIM_INVARIANT_IDS:
            errors.append("claim-validity-matrix.invariants ids != canonical CLAIM_INVARIANT_IDS")
        if invariants != CLAIM_INVARIANTS:
            errors.append("claim-validity-matrix.invariants do not match canonical CLAIM_INVARIANTS")
    require_exact_sequence(
        claim.get("unknown_reason_codes"),
        UNKNOWN_REASON_CODES,
        "claim-validity-matrix.unknown_reason_codes",
        errors,
    )
    require_exact_sequence(
        claim.get("precedence"), CLAIM_PRECEDENCE, "claim-validity-matrix.precedence", errors
    )
    require_exact_sequence(
        claim.get("use_evidence_kinds"),
        CLAIM_USE_EVIDENCE_KINDS,
        "claim-validity-matrix.use_evidence_kinds",
        errors,
    )
    require_exact_sequence(
        claim.get("reconciliation_states"),
        CLAIM_RECONCILIATION_STATES,
        "claim-validity-matrix.reconciliation_states",
        errors,
    )
    require_exact_sequence(
        claim.get("reconciliation_invariants"),
        CLAIM_RECONCILIATION_INVARIANTS,
        "claim-validity-matrix.reconciliation_invariants",
        errors,
    )
    if claim.get("llm_suggestion_in_provenance") is not False:
        errors.append("LLM suggestion must be excluded from provenance")
    reject_empty(claim.get("legal_examples"), "claim-validity-matrix.legal_examples", errors)
    reject_empty(claim.get("illegal_examples"), "claim-validity-matrix.illegal_examples", errors)


def validate_capability_matrix(caps: dict[str, Any], errors: list[str]) -> None:
    from contexpect_contract import CAPABILITIES

    validate_artifact_header(caps, "context-capability-matrix", errors)
    require_exact_sequence(
        caps.get("capabilities"), CAPABILITIES, "context-capability-matrix.capabilities", errors
    )
    cells = caps.get("cells")
    if not isinstance(cells, list):
        errors.append("context-capability-matrix.cells is not a list")
        return
    reject_empty(cells, "context-capability-matrix.cells", errors)
    ids = [cell.get("id") for cell in cells if isinstance(cell, dict)]
    reject_duplicate_values(ids, "context-capability-matrix.cells.id", errors)
    expected_cells = {cell["id"]: cell for cell in canonical_capability_cells()}
    actual_cells = {cell.get("id"): cell for cell in cells if isinstance(cell, dict)}
    missing = sorted(set(expected_cells) - set(actual_cells))
    extra = sorted(set(actual_cells) - set(expected_cells))
    if missing:
        errors.append(f"capability matrix missing Cartesian cells {missing[:8]}")
    if extra:
        errors.append(f"capability matrix has unexpected cells {extra[:8]}")
    for cell_id, expected in expected_cells.items():
        actual = actual_cells.get(cell_id)
        if not isinstance(actual, dict):
            continue
        require_closed_object(
            actual,
            CAPABILITY_CELL_ALLOWED_FIELDS,
            CAPABILITY_CELL_REQUIRED_FIELDS,
            cell_id,
            errors,
            nonempty=["id", "family_id", "version", "surface", "os_lane", "capability_id", "status", "field_mapping"],
        )
        derived = capability_cell_id(
            actual.get("family_id") or "",
            actual.get("version") or "",
            actual.get("surface") or "",
            actual.get("os_lane") or "",
            actual.get("capability_id") or "",
        )
        if actual.get("id") != derived:
            errors.append(f"{cell_id} id does not match family/version/surface/os/capability fields")
        if derived != expected["id"]:
            errors.append(f"{cell_id} fields do not match canonical Cartesian id {expected['id']}")
        for key in ("family_id", "version", "surface", "os_lane", "capability_id", "status"):
            if actual.get(key) != expected[key]:
                errors.append(f"{cell_id} {key} {actual.get(key)!r} != canonical {expected[key]!r}")
        try:
            surface = frozen_surface(expected["family_id"], expected["surface"])
        except KeyError:
            errors.append(f"{cell_id} is not a frozen family/surface")
            continue
        if actual.get("version") != surface["version"]:
            errors.append(
                f"{cell_id} version {actual.get('version')!r} != frozen surface version {surface['version']!r}"
            )
        if actual.get("os_lane") not in {lane["id"] for lane in OS_LANES}:
            errors.append(f"{cell_id} os_lane {actual.get('os_lane')!r} is not a frozen OS lane")


def validate_projection_matrix(proj: dict[str, Any], errors: list[str]) -> None:
    validate_artifact_header(proj, "projection-matrix", errors)
    cells = proj.get("cells")
    if not isinstance(cells, list):
        errors.append("projection-matrix.cells is not a list")
        return
    reject_empty(cells, "projection-matrix.cells", errors)
    ids = [cell.get("id") for cell in cells if isinstance(cell, dict)]
    reject_duplicate_values(ids, "projection-matrix.cells.id", errors)
    expected_cells = {cell["id"]: cell for cell in projection_cells()}
    actual_cells = {cell.get("id"): cell for cell in cells if isinstance(cell, dict)}
    missing = sorted(set(expected_cells) - set(actual_cells))
    extra = sorted(set(actual_cells) - set(expected_cells))
    if missing:
        errors.append(f"projection-matrix missing canonical cells {missing[:8]}")
    if extra:
        errors.append(f"projection-matrix unexpected cells {extra[:8]}")
    for cell_id, expected in expected_cells.items():
        actual = actual_cells.get(cell_id)
        if not isinstance(actual, dict):
            continue
        require_closed_object(
            actual,
            PROJECTION_CELL_ALLOWED_FIELDS,
            ["id", "family_id", "asset", "scope", "mode", "authority", "required_write"],
            cell_id,
            errors,
            nonempty=["id", "family_id", "asset", "scope", "mode", "authority"],
        )
        if actual.get("id") != expected["id"]:
            errors.append(f"{cell_id} id does not match canonical {expected['id']}")
        for key in ("family_id", "asset", "scope", "required_write", "mode", "authority"):
            if actual.get(key) != expected[key]:
                errors.append(f"{cell_id} {key} {actual.get(key)!r} != canonical {expected[key]!r}")
        native = actual.get("native_target")
        if actual.get("required_write"):
            for field in PROJECTION_REQUIRED_WRITE_FIELDS:
                if field not in actual:
                    errors.append(f"{cell_id} missing required-write field {field}")
            if not isinstance(native, dict):
                errors.append(f"{cell_id} native_target is incomplete")
            else:
                require_closed_object(
                    native,
                    NATIVE_TARGET_ALLOWED_FIELDS,
                    NATIVE_TARGET_REQUIRED_FIELDS,
                    f"{cell_id}.native_target",
                    errors,
                    nonempty=["kind", "family_id", "path_glob"],
                )
                if native.get("family_id") != actual.get("family_id"):
                    errors.append(f"{cell_id} native_target.family_id does not match the cell")


def validate_integration_contracts(integ: dict[str, Any], errors: list[str]) -> None:
    validate_artifact_header(integ, "integration-contracts", errors)
    contracts = integ.get("contracts")
    if not isinstance(contracts, list):
        errors.append("integration-contracts.contracts is not a list")
        return
    reject_empty(contracts, "integration-contracts.contracts", errors)
    names = [row.get("name") for row in contracts if isinstance(row, dict)]
    reject_duplicate_values(names, "integration-contracts.contracts.name", errors)
    extra = sorted(set(names) - set(INTEGRATION_CANONICAL_IDS))
    missing = sorted(set(INTEGRATION_CANONICAL_IDS) - set(names))
    if extra:
        errors.append(f"integration-contracts unexpected ids {extra[:8]}")
    if missing:
        errors.append(f"integration-contracts missing ids {missing[:8]}")
    for contract in contracts:
        if not isinstance(contract, dict):
            errors.append("integration contract is not an object")
            continue
        label = contract.get("name") or "contract"
        require_closed_object(
            contract,
            INTEGRATION_CONTRACT_REQUIRED_FIELDS,
            INTEGRATION_CONTRACT_REQUIRED_FIELDS,
            label,
            errors,
            nonempty=["name", "integration_mode", "version_pin", "capability", "license"],
        )


def validate_digest_manifest(manifest: dict[str, Any], errors: list[str]) -> None:
    validate_artifact_header(manifest, "artifact-digest-manifest", errors)
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list):
        errors.append("artifact-digest-manifest.artifacts is not a list")
        return
    reject_empty(artifacts, "artifact-digest-manifest.artifacts", errors)
    paths = [item.get("path") for item in artifacts if isinstance(item, dict)]
    schema_ids = [item.get("schema_id") for item in artifacts if isinstance(item, dict)]
    reject_duplicate_values(paths, "artifact-digest-manifest.path", errors)
    reject_duplicate_values(schema_ids, "artifact-digest-manifest.schema_id", errors)
    expected_paths = list(ARTIFACT_SCHEMA_IDS)
    extra = sorted(set(paths) - set(expected_paths))
    missing = sorted(set(expected_paths) - set(paths))
    if extra:
        errors.append(f"artifact-digest-manifest unexpected paths {extra[:8]}")
    if missing:
        errors.append(f"artifact-digest-manifest missing paths {missing[:8]}")
    for item in artifacts:
        if not isinstance(item, dict):
            continue
        label = item.get("path") or "digest-entry"
        require_closed_object(
            item,
            DIGEST_ENTRY_REQUIRED_FIELDS,
            DIGEST_ENTRY_REQUIRED_FIELDS,
            label,
            errors,
            nonempty=DIGEST_ENTRY_REQUIRED_FIELDS,
        )
        path = item.get("path")
        expected_schema = ARTIFACT_SCHEMA_IDS.get(path)
        if expected_schema and item.get("schema_id") != expected_schema:
            errors.append(
                f"{path} digest schema_id {item.get('schema_id')!r} != canonical {expected_schema!r}"
            )


def validate_corpus_manifest_header(manifest: dict[str, Any], errors: list[str]) -> None:
    validate_artifact_header(manifest, "corpus-manifest", errors)
    reject_empty(manifest.get("fixtures"), "corpus-manifest.fixtures", errors)
    reject_empty(manifest.get("files"), "corpus-manifest.files", errors)
    reject_empty(manifest.get("corpora"), "corpus-manifest.corpora", errors)
    reject_empty(manifest.get("counts"), "corpus-manifest.counts", errors)


def validate_field_mapping(doc: Any, relpath: str, errors: list[str]) -> None:
    if not require_closed_object(
        doc,
        FIELD_MAP_ALLOWED_TOP_LEVEL,
        FIELD_MAP_REQUIRED_TOP_LEVEL,
        relpath,
        errors,
        nonempty=["artifact", "family_id", "surface", "version", "cutoff", "fields"],
    ):
        return
    if doc.get("artifact") != FIELD_MAP_ARTIFACT:
        errors.append(f"{relpath} artifact identity {doc.get('artifact')!r} != {FIELD_MAP_ARTIFACT!r}")
    if doc.get("cutoff") != CUTOFF:
        errors.append(f"{relpath} cutoff {doc.get('cutoff')!r} != {CUTOFF}")
    require_schema_version(doc, relpath, errors)
    if doc.get("live_tested") is not False:
        errors.append(f"{relpath} live_tested {doc.get('live_tested')!r} must be false")
    expected_path = field_map_relpath(str(doc.get("family_id") or ""), str(doc.get("surface") or ""))
    if relpath != expected_path:
        errors.append(f"{relpath} path/content identity != {expected_path}")
    fields = doc.get("fields")
    if not isinstance(fields, list):
        errors.append(f"{relpath}.fields is not a list")
        return
    reject_empty(fields, f"{relpath}.fields", errors)
    native_fields = []
    for idx, field in enumerate(fields):
        label = f"{relpath}.fields[{idx}]"
        if not require_closed_object(
            field,
            FIELD_MAP_FIELD_ALLOWED,
            FIELD_MAP_FIELD_REQUIRED,
            label,
            errors,
            nonempty=FIELD_MAP_FIELD_REQUIRED,
        ):
            continue
        native_fields.append(field.get("native_field"))
        if field.get("claim_kind") not in CLAIM_AXES["claim_kind"]:
            errors.append(f"{label} claim_kind {field.get('claim_kind')!r} is not a canonical claim kind")
        if field.get("lifecycle_stage") not in CLAIM_AXES["lifecycle_stage"]:
            errors.append(f"{label} lifecycle_stage {field.get('lifecycle_stage')!r} is not a canonical stage")
        if field.get("coverage") not in CLAIM_AXES["coverage"]:
            errors.append(f"{label} coverage {field.get('coverage')!r} is not a canonical coverage")
        if field.get("minimum_evidence") not in CLAIM_AXES["provenance"]:
            errors.append(f"{label} minimum_evidence {field.get('minimum_evidence')!r} is not a canonical provenance")
        precision = field.get("precision")
        if precision is not None and precision not in CLAIM_AXES["precision"]:
            errors.append(f"{label} precision {precision!r} is not a canonical precision")
    reject_duplicate_values(native_fields, f"{relpath}.fields.native_field", errors)
