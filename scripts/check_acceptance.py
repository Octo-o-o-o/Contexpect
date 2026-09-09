#!/usr/bin/env python3
"""Foundation stage gates for acceptance artifacts, traceability, and corpora.

Usage:
  python3 scripts/check_acceptance.py --structure
  python3 scripts/check_acceptance.py --traceability
  python3 scripts/check_acceptance.py --corpus

Offline. No network. No third-party packages.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

from contexpect_contract import (  # noqa: E402
    ACCEPTANCE_ARTIFACTS,
    ALLOWED_CAPABILITY_STATUS,
    ALLOWED_COLLECTORS,
    ALLOWED_COVERAGE,
    ALLOWED_IMPORTERS,
    ALLOWED_LIVE_STATUS,
    ALLOWED_OWNERS,
    ALLOWED_PROVENANCE,
    ARTIFACT_DIGEST_MANIFEST,
    CORPUS_CROSS_LINK_FIELDS,
    CUTOFF,
    DOCTOR_BLOCKING_RULES,
    FEATURES,
    FROZEN_FORBIDDEN_REQUIRED_WRITE_CELL_IDS,
    FROZEN_REQUIRED_WRITE_CELL_IDS,
    NA_RULES_FAMILIES,
    NO_SKILLS_FAMILIES,
    ORACLE_NATIVE_SHAPES,
    PROJECTION_REQUIRED_WRITE_FIELDS,
    TRACE_FIELDS,
    UNKNOWN_REASON_CODES,
    assign_trace_metadata,
    canonical_capability_cells,
    canonical_json,
    capability_semantics,
    declared_oracle_coordinates,
    derived_fixture_coordinate_id,
    encoded_coordinate_from_fixture_id,
    encoded_kind_from_fixture_id,
    extract_normative_statements,
    family_ids,
    field_map_relpath,
    iter_surfaces,
    jsonl_required_fields,
    load_json_yaml,
    load_prd_text,
    manifest_required_fields,
    missing_fields,
    occurrence_identity,
    occurrence_requirement_id,
    read_traceability_csv,
    implementation_cell_valid,
    implementation_paths,
    require_fields,
    require_schema_version,
    require_sensitivity,
    required_loss_cells,
    required_supported_static_coordinates,
    scan_forbidden_placeholders,
    sha256_text,
    uses_export_only,
)
from contexpect_fixtures import (  # noqa: E402
    interpret_doctor,
    interpret_oracle,
    interpret_static,
    load_input_files,
    validate_doctor_structure,
    validate_oracle_structure,
    validate_static_structure,
)
from contexpect_schema import (  # noqa: E402
    validate_capability_matrix,
    validate_claim_validity,
    validate_compatibility,
    validate_corpus_manifest_header,
    validate_digest_manifest,
    validate_field_mapping,
    validate_integration_contracts,
    validate_projection_matrix,
)
from contexpect_freeze import (  # noqa: E402
    DIGEST_NOT_PUBLISHED,
    EVIDENCE_BACKED_UNAVAILABLE,
    UBUNTU_24_04,
    WINDOWS_11_24H2,
    is_forbidden_placeholder,
)


def fail(message: str, errors: list[str]) -> None:
    errors.append(message)


def _git_candidate_paths(root: Path) -> set[str] | None:
    git_root = root
    if not (git_root / ".git").exists():
        if root.resolve() != ROOT.resolve():
            return None
        git_root = ROOT
    proc = subprocess.run(
        [
            "git",
            "-C",
            str(git_root),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        return None
    return {item for item in proc.stdout.decode("utf-8").split("\0") if item}


def load_artifact(rel: str, root: Path) -> object:
    return load_json_yaml(root / rel)


def check_structure(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    for rel in ACCEPTANCE_ARTIFACTS + [ARTIFACT_DIGEST_MANIFEST]:
        path = root / rel
        if not path.is_file():
            fail(f"missing {rel}", errors)

    if not (root / "acceptance/compatibility-matrix.yaml").is_file():
        return

    compat = load_artifact("acceptance/compatibility-matrix.yaml", root)
    claim = load_artifact("acceptance/claim-validity-matrix.yaml", root)
    caps = load_artifact("acceptance/context-capability-matrix.yaml", root)
    proj = load_artifact("acceptance/projection-matrix.yaml", root)
    integ = load_artifact("acceptance/integration-contracts.yaml", root)
    manifest = json.loads((root / "acceptance/corpus-manifest.json").read_text(encoding="utf-8"))
    digest_manifest = json.loads((root / ARTIFACT_DIGEST_MANIFEST).read_text(encoding="utf-8"))

    named = {
        "compatibility-matrix": compat,
        "claim-validity-matrix": claim,
        "context-capability-matrix": caps,
        "projection-matrix": proj,
        "integration-contracts": integ,
        "corpus-manifest": manifest,
        "artifact-digest-manifest": digest_manifest,
    }
    for name, obj in named.items():
        if not isinstance(obj, dict):
            fail(f"{name} is not an object", errors)
            continue
        scan_forbidden_placeholders(obj, name, errors)

    validate_compatibility(compat, errors)
    validate_claim_validity(claim, errors)
    validate_capability_matrix(caps, errors)
    validate_projection_matrix(proj, errors)
    validate_integration_contracts(integ, errors)
    validate_digest_manifest(digest_manifest, errors)
    validate_corpus_manifest_header(manifest, errors)

    families = set(family_ids())
    for row in compat.get("coordinates") or []:
        if not isinstance(row, dict):
            continue
        if row.get("fabricated_native_evidence"):
            fail(f"{row.get('id')} fabricates native evidence", errors)
        live = (row.get("live_install") or {}).get("live_status")
        if live not in ALLOWED_LIVE_STATUS:
            fail(f"{row.get('id')} has unexpected live_status {live}", errors)
        for key in ("version", "version_source"):
            if is_forbidden_placeholder(row.get(key)):
                fail(f"{row.get('id')} {key} is a forbidden placeholder", errors)
        if row.get("os_lane") == "macos-27-arm64" and row.get("family_id") == "deepseek-harness":
            if row["live_install"]["executable_present"] is not False:
                fail("DeepSeek must not treat config residue as executable-present", errors)
            if row["live_install"]["live_status"] != "not-installed":
                fail("DeepSeek live_status must be not-installed", errors)
        if row.get("os_lane") == "macos-27-arm64" and row.get("family_id") == "coze":
            if row["live_install"]["live_status"] != "connector-required":
                fail("Coze live_status must be connector-required", errors)
            if row["live_install"]["executable_present"] is not False:
                fail("Coze recent-item residue must not become executable-present", errors)

    ubuntu_lane = next((lane for lane in compat.get("os_lanes") or [] if lane.get("id") == "ubuntu-24.04-x86_64"), {})
    windows_lane = next((lane for lane in compat.get("os_lanes") or [] if lane.get("id") == "windows-11-24h2-x86_64"), {})
    if ubuntu_lane.get("image_digest") != UBUNTU_24_04["image_digest"]:
        fail("Ubuntu lane is not pinned to the official 24.04.4 live-server SHA-256", errors)
    if windows_lane.get("build") != WINDOWS_11_24H2["build"]:
        fail("Windows lane is not pinned to official 24H2 build 26100.9278", errors)
    if windows_lane.get("image_digest") != DIGEST_NOT_PUBLISHED:
        fail("Windows ISO digest must remain digest-not-published-by-source unless Microsoft publishes a stable catalog", errors)

    other = [c for c in caps.get("cells") or [] if c.get("capability_id") == "other-unknown"]
    if any(c.get("status") == "required-supported" for c in other):
        fail("other-unknown cannot be required-supported", errors)

    expected_cap_cells = {cell["id"]: cell for cell in canonical_capability_cells()}
    actual_cap_cells = {cell["id"]: cell for cell in caps.get("cells") or []}
    compat_ids = {row["id"] for row in compat.get("coordinates") or []}
    mapping_root = root / "acceptance/field-to-claim"
    mapping_docs: dict[str, dict] = {}
    canonical_paths = {
        field_map_relpath(family["id"], surface["id"]): (family["id"], surface["id"], surface["version"])
        for family, surface in iter_surfaces()
    }
    if not mapping_root.is_dir():
        fail("missing acceptance/field-to-claim directory", errors)
    else:
        seen_identities: dict[tuple[object, object], str] = {}
        present_paths: set[str] = set()
        for path in sorted(mapping_root.glob("*.yaml")):
            rel = path.relative_to(root).as_posix()
            present_paths.add(rel)
            doc = load_json_yaml(path)
            mapping_docs[rel] = doc if isinstance(doc, dict) else {}
            validate_field_mapping(doc, rel, errors)
            if not isinstance(doc, dict):
                continue
            identity = (doc.get("family_id"), doc.get("surface"))
            prior = seen_identities.get(identity)
            if prior:
                fail(f"{rel} duplicate field-map identity {identity} also claimed by {prior}", errors)
            else:
                seen_identities[identity] = rel
            if rel not in canonical_paths:
                fail(f"{rel} extra foreign field-map {rel}", errors)
        missing_maps = sorted(set(canonical_paths) - present_paths)
        extra_maps = sorted(present_paths - set(canonical_paths))
        if missing_maps:
            fail(f"field-to-claim missing canonical maps {missing_maps[:8]}", errors)
        if extra_maps:
            fail(f"field-to-claim extra foreign mapping {extra_maps[:8]}", errors)
        if len(present_paths) != len(canonical_paths):
            fail(
                f"field-to-claim count {len(present_paths)} != canonical {len(canonical_paths)}",
                errors,
            )
    for cell_id, expected in expected_cap_cells.items():
        actual = actual_cap_cells.get(cell_id)
        if not actual:
            continue
        if actual.get("status") not in ALLOWED_CAPABILITY_STATUS:
            fail(f"{cell_id} status {actual.get('status')!r} is not allowed", errors)
        coord_id = actual.get("compatibility_coordinate_id")
        expected_coord = (
            f"{actual.get('family_id')}/{actual.get('version')}/"
            f"{actual.get('surface')}/{actual.get('os_lane')}"
        )
        if coord_id != expected_coord:
            fail(f"{cell_id} compatibility_coordinate_id {coord_id!r} != {expected_coord!r}", errors)
        if coord_id != expected["coordinate_id"]:
            fail(f"{cell_id} compatibility coordinate != canonical {expected['coordinate_id']}", errors)
        if coord_id not in compat_ids:
            fail(f"{cell_id} compatibility_coordinate_id {coord_id!r} is not in the compatibility matrix", errors)
        semantics = capability_semantics(
            expected["status"],
            expected["capability_id"],
            expected["family_id"],
            expected["surface"],
        )
        for key, value in semantics.items():
            if actual.get(key) != value:
                fail(f"{cell_id} {key} {actual.get(key)!r} != canonical {value!r}", errors)
        if actual.get("collector") not in ALLOWED_COLLECTORS:
            fail(f"{cell_id} collector {actual.get('collector')!r} is not allowed", errors)
        if actual.get("importer") not in ALLOWED_IMPORTERS:
            fail(f"{cell_id} importer {actual.get('importer')!r} is not allowed", errors)
        if actual.get("provenance") not in ALLOWED_PROVENANCE:
            fail(f"{cell_id} provenance {actual.get('provenance')!r} is not allowed", errors)
        if actual.get("coverage") not in ALLOWED_COVERAGE:
            fail(f"{cell_id} coverage {actual.get('coverage')!r} is not allowed", errors)
        mapping = actual.get("field_mapping")
        if not mapping:
            fail(f"{cell_id} missing field_mapping", errors)
            continue
        canonical_mapping = field_map_relpath(str(actual.get("family_id") or ""), str(actual.get("surface") or ""))
        if mapping != canonical_mapping:
            fail(f"{cell_id} field_mapping {mapping!r} != canonical {canonical_mapping!r}", errors)
        mapping_path = root / mapping
        if not mapping_path.is_file():
            fail(f"{cell_id} field_mapping file missing {mapping}", errors)
            continue
        doc = mapping_docs.get(mapping)
        if doc is None:
            if mapping_path.is_file():
                loaded = load_json_yaml(mapping_path)
                doc = loaded if isinstance(loaded, dict) else {}
                mapping_docs[mapping] = doc
            else:
                continue
        if not doc.get("fields"):
            fail(f"{mapping} has empty field mapping", errors)
        if doc.get("family_id") != actual.get("family_id"):
            fail(
                f"{cell_id} field-map family_id {doc.get('family_id')!r} != cell {actual.get('family_id')!r}",
                errors,
            )
        if doc.get("surface") != actual.get("surface"):
            fail(
                f"{cell_id} field-map surface {doc.get('surface')!r} != cell {actual.get('surface')!r}",
                errors,
            )
        if doc.get("version") != actual.get("version"):
            fail(
                f"{cell_id} field-map version {doc.get('version')!r} != cell {actual.get('version')!r}",
                errors,
            )
        if doc.get("cutoff") != CUTOFF:
            fail(f"{cell_id} field-map cutoff {doc.get('cutoff')!r} != {CUTOFF}", errors)
        if doc.get("live_tested") is not False:
            fail(f"{cell_id} field-map live_tested {doc.get('live_tested')!r} must be false", errors)

    by_proj = {cell["id"]: cell for cell in proj.get("cells") or []}
    for cell_id in FROZEN_REQUIRED_WRITE_CELL_IDS:
        cell = by_proj.get(cell_id)
        if not cell:
            fail(f"projection floor cell missing {cell_id}", errors)
            continue
        if not cell.get("required_write"):
            fail(f"projection floor cell {cell_id} demoted away from required-write", errors)
        require_fields(cell, PROJECTION_REQUIRED_WRITE_FIELDS, f"required-write {cell_id}", errors)
        if uses_export_only(cell):
            fail(f"required-write cell {cell_id} uses export-only as mode, authority, or executor", errors)
        for gate in ("preview", "apply", "rollback", "post_receipt", "concurrency_guard", "loss_report"):
            if not cell.get(gate):
                fail(f"required-write cell {cell_id} missing {gate}", errors)
        native = cell.get("native_target")
        if not isinstance(native, dict) or not native.get("path_glob") or not native.get("family_id"):
            fail(f"required-write cell {cell_id} native_target is incomplete", errors)
        if not cell.get("authority") or not cell.get("authority_version"):
            fail(f"required-write cell {cell_id} missing authority/version", errors)
        if not cell.get("loss_contract"):
            fail(f"required-write cell {cell_id} missing loss_contract", errors)
    for cell_id in FROZEN_FORBIDDEN_REQUIRED_WRITE_CELL_IDS:
        cell = by_proj.get(cell_id)
        if cell and cell.get("required_write"):
            fail(f"{cell_id} must not be required-write", errors)
    for cell in proj["cells"]:
        if cell.get("required_write") and uses_export_only(cell):
            fail(f"{cell.get('id')} required_write=true may not use export-only as mode, authority, or executor", errors)
    if "cursor/user-instructions/user" in by_proj and by_proj["cursor/user-instructions/user"].get("required_write"):
        fail("Cursor user instructions must remain export-only", errors)

    for contract in integ.get("contracts") or []:
        pin = contract.get("version_pin")
        honesty = contract.get("version_pin_honesty")
        evidence = contract.get("pin_evidence") or {}
        if is_forbidden_placeholder(pin) or is_forbidden_placeholder(contract.get("license")):
            fail(f"{contract.get('name')} uses a forbidden placeholder pin or license", errors)
        if pin != EVIDENCE_BACKED_UNAVAILABLE and not (isinstance(pin, str) and pin and pin != EVIDENCE_BACKED_UNAVAILABLE):
            fail(f"{contract.get('name')} version_pin is empty", errors)
        if pin == EVIDENCE_BACKED_UNAVAILABLE:
            if not evidence.get("source_url") or not evidence.get("reason"):
                fail(f"{contract.get('name')} unavailable pin lacks source-backed evidence", errors)
            if honesty != "research-ledger-records-github-url-without-cutoff-release-tag":
                fail(f"{contract.get('name')} unavailable honesty marker mismatch", errors)

    hashed = {item["path"]: item["digest_sha256"] for item in digest_manifest.get("artifacts") or []}
    for rel in ACCEPTANCE_ARTIFACTS:
        if rel not in hashed:
            fail(f"artifact digest manifest missing {rel}", errors)
            continue
        actual = sha256_text((root / rel).read_text(encoding="utf-8"))
        if actual != hashed[rel]:
            fail(f"artifact digest mismatch for {rel}", errors)

    if manifest.get("cutoff") != CUTOFF:
        fail("corpus-manifest cutoff mismatch", errors)
    for item in manifest.get("fixtures") or []:
        require_fields(item, manifest_required_fields(item), item.get("id", "fixture"), errors)
        require_schema_version(item, item.get("id", "fixture"), errors)
        require_sensitivity(item, item.get("id", "fixture"), errors)

    hardware = (root / "acceptance/reference-hardware.md").read_text(encoding="utf-8")
    for token in [
        "macOS 27.0",
        "26A5425a",
        "Ubuntu 24.04",
        "Windows 11",
        UBUNTU_24_04["image_digest"],
        WINDOWS_11_24H2["build"],
        CUTOFF,
        DIGEST_NOT_PUBLISHED,
    ]:
        if token not in hardware:
            fail(f"reference-hardware.md missing {token}", errors)
    if is_forbidden_placeholder(hardware):
        fail("reference-hardware.md contains a forbidden placeholder token", errors)

    for feature in FEATURES:
        _ = feature


def check_traceability(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    prd = load_prd_text(root)
    extracted = [assign_trace_metadata(item) for item in extract_normative_statements(prd)]
    csv_path = root / "acceptance/traceability.csv"
    if not csv_path.is_file():
        fail("missing acceptance/traceability.csv", errors)
        return
    rows = read_traceability_csv(csv_path)
    if not rows:
        fail("traceability.csv is empty", errors)
        return
    missing_cols = [field for field in TRACE_FIELDS if field not in rows[0]]
    if missing_cols:
        fail(f"traceability.csv missing columns {missing_cols}", errors)

    extracted_ids = [item["requirement_id"] for item in extracted]
    if len(extracted_ids) != len(set(extracted_ids)):
        fail("extractor produced duplicate requirement_id values", errors)
    extracted_by_id = {item["requirement_id"]: item for item in extracted}
    row_ids: dict[str, dict] = {}
    sibling_counts: dict[tuple[str, str, str], int] = {}
    for row in rows:
        digest = row.get("source_digest") or ""
        req_id = row.get("requirement_id") or ""
        statement = row.get("statement") or ""
        heading = row.get("heading") or ""
        section = row.get("section") or "0"
        if not digest:
            fail(f"traceability row {req_id} missing source_digest", errors)
            continue
        if req_id in row_ids:
            fail(f"duplicate requirement_id {req_id}", errors)
        row_ids[req_id] = row
        for field in TRACE_FIELDS:
            if not (row.get(field) or "").strip():
                fail(f"{req_id} empty field {field}", errors)
        sibling_key = (section, heading, statement)
        sibling_index = sibling_counts.get(sibling_key, 0)
        sibling_counts[sibling_key] = sibling_index + 1
        expected_id = occurrence_requirement_id(section, heading, statement, sibling_index)
        if req_id != expected_id or not req_id.startswith("REQ-"):
            fail(f"{req_id} is not a stable content-derived id for its statement", errors)
        canonical = extracted_by_id.get(req_id)
        if canonical:
            for field in TRACE_FIELDS:
                actual = (row.get(field) or "").strip()
                expect = str(canonical.get(field) or "").strip()
                if actual != expect:
                    fail(
                        f"{req_id} field {field} {actual!r} != canonical mapping {expect!r}",
                        errors,
                    )
        feature = row.get("feature") or ""
        implementation = row.get("implementation") or ""
        if not implementation_cell_valid(implementation):
            fail(
                f"{req_id} implementation must be `unimplemented`, `partial: <paths>` or `<paths>`, got {implementation!r}",
                errors,
            )
        for rel in implementation_paths(implementation):
            # Paths are checked against the real repository: an implementation
            # column that names a file that does not exist is a false claim.
            if not (ROOT / rel).is_file():
                fail(f"{req_id} implementation path does not exist: {rel}", errors)
        if not (row.get("evidence_test") or "").strip():
            fail(f"{req_id} evidence_test is empty; use `none` when there is none", errors)
        if heading.startswith("F-") and feature != heading.split()[0]:
            fail(f"{req_id} feature {feature} does not close at heading {heading}", errors)
        if heading.startswith("WP-") and feature != "NA":
            fail(f"{req_id} WP heading leaked feature {feature}", errors)

    extracted_occ = Counter(occurrence_identity(item) for item in extracted)
    row_occ = Counter(occurrence_identity(row) for row in rows)
    if extracted_occ != row_occ:
        dropped = sum((extracted_occ - row_occ).values())
        extra_n = sum((row_occ - extracted_occ).values())
        fail(
            f"traceability occurrence identity mismatch: dropped {dropped}, extra {extra_n}",
            errors,
        )
    missing_ids = sorted(set(extracted_by_id) - set(row_ids))
    extra_ids = sorted(set(row_ids) - set(extracted_by_id))
    if missing_ids:
        sample = extracted_by_id[missing_ids[0]]["statement"][:120]
        fail(f"{len(missing_ids)} untracked normative statements; first: {sample}", errors)
    if extra_ids:
        fail(f"{len(extra_ids)} traceability rows are not current PRD statements", errors)
    if len(extracted) < 100:
        fail(f"extractor found only {len(extracted)} statements; likely too coarse or PRD unread", errors)

    feature_leak = [
        row
        for row in extracted
        if row["feature"] == "F-18" and (row["section"].startswith("9") or row["heading"].startswith("9."))
    ]
    if feature_leak:
        fail("F-18 feature scope leaked into section 9 headings", errors)

    print(f"traceability statements: {len(extracted)}")
    print(f"traceability rows: {len(rows)}")


def _iter_jsonl(path: Path):
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            yield line_no, json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path} line {line_no}: {exc}") from exc


def check_corpus(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    manifest_path = root / "acceptance/corpus-manifest.json"
    if not manifest_path.is_file():
        fail("missing acceptance/corpus-manifest.json", errors)
        return
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    fixtures = manifest.get("fixtures") or []
    files = {item["path"]: item["digest_sha256"] for item in manifest.get("files") or []}

    for rel, expected in files.items():
        path = root / rel
        if not path.is_file():
            fail(f"corpus file missing {rel}", errors)
            continue
        actual = sha256_text(path.read_text(encoding="utf-8"))
        if actual != expected:
            fail(f"digest mismatch for {rel}", errors)

    ids = [item["id"] for item in fixtures]
    if len(ids) != len(set(ids)):
        fail("corpus fixture ids are not unique", errors)

    # Native-synthetic sessions: written by a harness's own API, held fixed
    # by digest, synthetic and never live-tested. They are not JSONL golden
    # rows (no `id` per line), so they are checked here, not below.
    native = manifest.get("native_synthetic")
    if not isinstance(native, list) or not native:
        fail("corpus-manifest lacks native_synthetic sessions", errors)
    else:
        if (manifest.get("corpora") or {}).get("native-synthetic", {}).get("live_tested") is not False:
            fail("native-synthetic corpus must declare live_tested false", errors)
        for item in native:
            if item.get("live_tested") is not False:
                fail(f"{item.get('id')} native-synthetic must not claim live_tested", errors)
            if item.get("license") != "Apache-2.0":
                fail(f"{item.get('id')} native-synthetic missing Apache-2.0 license", errors)
            sha = item.get("harness_sha")
            if not isinstance(sha, str) or len(sha) != 40:
                fail(f"{item.get('id')} native-synthetic lacks a 40-hex harness_sha", errors)
            names = {Path(f["path"]).name for f in item.get("files") or []}
            if names != {"session.jsonl", "expected.json", "meta.json"}:
                fail(f"{item.get('id')} native-synthetic must list session.jsonl, expected.json, meta.json", errors)
            for entry in item.get("files") or []:
                path = root / entry["path"]
                if not path.is_file():
                    fail(f"native-synthetic file missing {entry['path']}", errors)
                    continue
                text = path.read_text(encoding="utf-8")
                if sha256_text(text) != entry.get("digest_sha256"):
                    fail(f"digest mismatch for {entry['path']}", errors)
                if "/Users/" in text or "/home/" in text:
                    fail(f"{entry['path']} carries a host home path", errors)
                if path.name == "meta.json":
                    meta = json.loads(text)
                    if meta.get("live_tested") is not False or meta.get("synthetic") is not True:
                        fail(f"{entry['path']} must declare live_tested false and synthetic true", errors)
                    if meta.get("dsh_sha") != sha:
                        fail(f"{entry['path']} dsh_sha != manifest harness_sha", errors)
                if path.name == "expected.json":
                    expected = json.loads(text)
                    if expected.get("schema") != "ctxpect-dsh-fixture-expected-v1":
                        fail(f"{entry['path']} unexpected schema {expected.get('schema')!r}", errors)

    by_corpus = Counter(item["corpus"] for item in fixtures)
    for name in ("development", "sealed", "live"):
        if by_corpus[name] == 0:
            fail(f"corpus {name} has no fixtures", errors)

    sealed_with_answers = [
        item for item in fixtures if item.get("corpus") == "sealed" and item.get("answers_status") != "withheld-until-rc"
    ]
    if sealed_with_answers:
        fail("sealed fixtures must withhold answers", errors)
    live_claimed = [item for item in fixtures if item.get("corpus") == "live" and item.get("live_tested")]
    if live_claimed:
        fail("live corpus must not claim live_tested in this stage", errors)
    generated_live = [item for item in fixtures if item.get("live_tested") is True]
    if generated_live:
        fail("no fixture may claim live_tested=true in foundation stage", errors)

    jsonl_by_id: dict[str, tuple[str, dict]] = {}
    jsonl_paths: set[str] = {rel for rel in files if rel.endswith(".jsonl")}
    corpus_root = root / "acceptance/corpus"
    record_dirs = [
        corpus_root / "development" / "static",
        corpus_root / "development" / "oracle",
        corpus_root / "development" / "doctor",
        corpus_root / "sealed",
        corpus_root / "live",
    ]
    for folder in record_dirs:
        if folder.is_dir():
            for path in folder.glob("*.jsonl"):
                jsonl_paths.add(path.relative_to(root).as_posix())
    for rel in sorted(jsonl_paths):
        path = root / rel
        if not path.is_file():
            fail(f"corpus file missing {rel}", errors)
            continue
        if rel not in files:
            fail(f"unmanifested JSONL file {rel}", errors)
        for _, row in _iter_jsonl(path):
            rid = row.get("id")
            if not rid:
                fail(f"{rel} JSONL record missing id", errors)
                continue
            if rid in jsonl_by_id:
                prior_path, _prior = jsonl_by_id[rid]
                fail(f"duplicate JSONL id {rid} in {rel} and {prior_path}", errors)
                continue
            jsonl_by_id[rid] = (rel, row)

    manifest_ids = [item.get("id") for item in fixtures]
    jsonl_ids = set(jsonl_by_id)
    extra_ids = sorted(jsonl_ids - set(manifest_ids))
    missing_ids = sorted(set(manifest_ids) - jsonl_ids)
    if extra_ids:
        fail(f"{len(extra_ids)} unmanifested JSONL records; first: {extra_ids[0]}", errors)
    if missing_ids:
        fail(f"{len(missing_ids)} manifest-only fixtures without JSONL; first: {missing_ids[0]}", errors)

    for item in fixtures:
        required_manifest = manifest_required_fields(item)
        missing = missing_fields(item, required_manifest)
        if missing:
            fail(f"{item.get('id')} missing {missing}", errors)
        require_fields(item, required_manifest, item.get("id", "fixture"), errors)
        require_schema_version(item, item.get("id", "fixture"), errors)
        require_sensitivity(item, item.get("id", "fixture"), errors)
        rec = jsonl_by_id.get(item["id"])
        if not rec:
            fail(f"{item['id']} missing JSONL record for manifest fixture", errors)
            continue
        jsonl_path, row = rec
        required_jsonl = jsonl_required_fields(row)
        require_fields(row, required_jsonl, row.get("id", "record"), errors)
        require_schema_version(row, row.get("id", "record"), errors)
        require_sensitivity(row, row.get("id", "record"), errors)
        if item.get("id") != row.get("id"):
            fail(f"{item['id']} manifest id != JSONL id {row.get('id')!r}", errors)
        if item.get("path") != jsonl_path:
            fail(f"{item['id']} manifest path {item.get('path')!r} != JSONL {jsonl_path!r}", errors)
        if item.get("digest_sha256") != row.get("digest"):
            fail(f"{item['id']} manifest digest != JSONL record digest", errors)
        compare_keys: list[str] = []
        seen_keys: set[str] = set()
        for key in CORPUS_CROSS_LINK_FIELDS:
            required = key in required_manifest or key in required_jsonl
            present = key in item or key in row
            if required or present:
                if key not in seen_keys:
                    seen_keys.add(key)
                    compare_keys.append(key)
        for key in compare_keys:
            if key not in item:
                fail(f"{item['id']} manifest missing {key}", errors)
                continue
            if key not in row:
                fail(f"{item['id']} JSONL missing {key}", errors)
                continue
            if item[key] != row[key]:
                fail(f"{item['id']} manifest {key} {item[key]!r} != JSONL {key} {row[key]!r}", errors)
        encoded_kind = encoded_kind_from_fixture_id(str(item.get("id") or ""))
        if encoded_kind and item.get("kind") != encoded_kind:
            fail(f"{item['id']} kind {item.get('kind')!r} != id-encoded {encoded_kind!r}", errors)
        if encoded_kind and row.get("kind") != encoded_kind:
            fail(f"{item['id']} JSONL kind {row.get('kind')!r} != id-encoded {encoded_kind!r}", errors)
        encoded = encoded_coordinate_from_fixture_id(str(item.get("id") or ""))
        if encoded and encoded != item.get("coordinate_id"):
            fail(
                f"{item['id']} coordinate_id {item.get('coordinate_id')!r} != id-encoded {encoded!r}",
                errors,
            )
        if encoded and encoded != row.get("coordinate_id"):
            fail(
                f"{item['id']} JSONL coordinate_id {row.get('coordinate_id')!r} != id-encoded {encoded!r}",
                errors,
            )
        derived = derived_fixture_coordinate_id(row)
        if derived and row.get("coordinate_id") != derived:
            fail(
                f"{item['id']} coordinate_id {row.get('coordinate_id')!r} != field-derived {derived!r}",
                errors,
            )
        if item.get("corpus") == "development" and (encoded_kind == "oracle" or item.get("kind") == "oracle"):
            if "expected_native_shape" not in item:
                fail(f"{item['id']} manifest missing expected_native_shape", errors)
            elif "expected_native_shape" not in row:
                fail(f"{item['id']} JSONL missing expected_native_shape", errors)
            elif item.get("expected_native_shape") != row.get("expected_native_shape"):
                fail(f"{item['id']} native-shape identity mismatch between manifest and JSONL", errors)
            oracle_id = row.get("oracle_id") if "oracle_id" in row else item.get("oracle_id")
            canonical_shape = ORACLE_NATIVE_SHAPES.get(oracle_id) if oracle_id else None
            shape = row["expected_native_shape"] if "expected_native_shape" in row else None
            if canonical_shape and isinstance(shape, dict):
                if list(shape.get("required_keys") or []) != list(canonical_shape["required_keys"]):
                    fail(
                        f"{item['id']} expected_native_shape.required_keys {shape.get('required_keys')!r} "
                        f"!= canonical {canonical_shape['required_keys']!r}",
                        errors,
                    )
        if item.get("corpus") == "development" and (
            encoded_kind in {"static", "oracle", "doctor"} or item.get("kind") in {"static", "oracle", "doctor"}
        ):
            if "input_path" not in item:
                fail(f"{item['id']} manifest missing input_path", errors)
                continue
            if "input_digest_sha256" not in item:
                fail(f"{item['id']} manifest missing input_digest_sha256", errors)
                continue
            if "input_path" not in row or "input_digest_sha256" not in row:
                fail(f"{item['id']} JSONL missing input_path/input_digest_sha256", errors)
                continue
            if item["input_path"] != row["input_path"] or item["input_digest_sha256"] != row["input_digest_sha256"]:
                fail(f"{item['id']} manifest input identity != JSONL input identity", errors)
                continue
            input_path = item["input_path"]
            input_digest = item["input_digest_sha256"]
            if not input_path or not input_digest:
                fail(f"{item['id']} missing physical input path/digest", errors)
                continue
            path = root / input_path
            if path.is_file():
                actual = sha256_text(path.read_text(encoding="utf-8"))
                if actual != input_digest:
                    fail(f"{item['id']} input digest mismatch", errors)
            elif path.is_dir():
                listed = []
                for child in sorted(path.rglob("*")):
                    if child.is_file():
                        rel = child.relative_to(root).as_posix()
                        listed.append({"path": rel, "digest_sha256": sha256_text(child.read_text(encoding="utf-8"))})
                actual = sha256_text(canonical_json(listed))
                if actual != input_digest:
                    fail(f"{item['id']} directory input digest mismatch", errors)
            else:
                fail(f"{item['id']} physical input missing {input_path}", errors)
                continue
            on_disk = load_input_files(root, input_path)
            if item.get("kind") == "static":
                validate_static_structure(row, on_disk, errors)
                parsed = interpret_static(
                    row.get("family_id") or "",
                    row.get("capability_id") or "",
                    on_disk,
                    surface=row.get("surface"),
                    os_lane=row.get("os_lane"),
                )
                if row.get("expected_output") != parsed:
                    fail(f"{item['id']} expected_output != native parser output", errors)
                claim = row.get("expected_claim") or {}
                if claim.get("truth_state") != parsed.get("truth_state"):
                    fail(f"{item['id']} expected_claim.truth_state != native parser truth_state", errors)
            elif item.get("kind") == "oracle":
                validate_oracle_structure(row, on_disk, errors)
                parsed = interpret_oracle(
                    row.get("family_id") or "",
                    row.get("capability_id") or "",
                    on_disk,
                )
                expected_out = row.get("expected_output") or {}
                comparable = {key: expected_out.get(key) for key in parsed}
                if comparable != parsed:
                    fail(f"{item['id']} expected_output != native oracle parser output", errors)
            elif item.get("kind") == "doctor":
                validate_doctor_structure(row, on_disk, errors)
                findings = interpret_doctor(on_disk)
                if findings != (row.get("expected_findings") or []):
                    fail(f"{item['id']} expected_findings != doctor parser output", errors)

    static_coords = {c["id"]: c for c in required_supported_static_coordinates()}
    static_counts = Counter()
    static_classes: dict[str, set[str]] = defaultdict(set)
    coverage: dict[tuple[str, str], set[str]] = defaultdict(set)
    for item in fixtures:
        if item.get("corpus") == "development" and item.get("kind") == "static":
            static_counts[item["coordinate_id"].split("/oracle:")[0]] += 1
            if item.get("license") != "Apache-2.0":
                fail(f"{item['id']} missing Apache-2.0 license", errors)
    for jsonl_path, row in jsonl_by_id.values():
        if row.get("corpus") != "development" or row.get("kind") not in {"static", "oracle"}:
            continue
        coord = (row.get("coordinate_id") or "").split("/oracle:")[0]
        cap = row.get("capability_id") or ""
        polarity = row.get("polarity") or ""
        if not polarity:
            klass = row.get("class") or ""
            if klass == "unknown_honesty":
                polarity = "indeterminate"
            elif klass in {"exclude", "ignore", "negative"}:
                polarity = "negative"
            elif klass == "loss":
                polarity = "loss"
            else:
                polarity = "positive"
        coverage[(coord, cap)].add(polarity)
        if row.get("kind") == "static":
            static_classes[coord].add(row.get("class") or "")
        if not row.get("expected_claim"):
            fail(f"{row.get('id')} missing expected claim", errors)
        if not row.get("input_path"):
            fail(f"{row.get('id')} missing executable input", errors)
    for coord_id in static_coords:
        if static_counts[coord_id] < 60:
            fail(f"{coord_id} has {static_counts[coord_id]} static cases, need >= 60", errors)
        needed = {"include", "unknown_honesty", "negative"}
        if needed - static_classes[coord_id]:
            fail(f"{coord_id} missing static classes {sorted(needed - static_classes[coord_id])}", errors)

    for cell in canonical_capability_cells():
        pols = coverage[(cell["coordinate_id"], cell["capability_id"])]
        if cell["status"] == "required-supported":
            if "positive" not in pols or "negative" not in pols:
                fail(
                    f"{cell['id']} required-supported missing positive/negative fixtures (have {sorted(pols)})",
                    errors,
                )
        elif cell["status"] == "required-unknown-honesty":
            if "indeterminate" not in pols:
                fail(f"{cell['id']} required-unknown-honesty missing indeterminate fixture", errors)
        elif cell["status"] == "not-applicable":
            needs_loss = (
                cell["family_id"] in NA_RULES_FAMILIES and cell["capability_id"] == "rules"
            ) or (
                cell["family_id"] in NO_SKILLS_FAMILIES and cell["capability_id"] in {"skills", "plugins"}
            )
            if needs_loss and cell["os_lane"] == "macos-27-arm64" and "loss" not in pols:
                fail(f"{cell['id']} missing loss fixture", errors)

    loss_cells = required_loss_cells()
    if len(loss_cells) != 8:
        fail(f"canonical required loss cells are {len(loss_cells)}, need exactly 8", errors)
    for cell in loss_cells:
        pols = coverage[(cell["coordinate_id"], cell["capability_id"])]
        if "loss" not in pols:
            fail(f"{cell['id']} missing required loss fixture", errors)

    oracle_coords = declared_oracle_coordinates()
    oracle_counts = Counter()
    for item in fixtures:
        if item.get("corpus") == "development" and item.get("kind") == "oracle":
            oracle_counts[item["coordinate_id"]] += 1
            if item.get("native_result_captured"):
                fail(f"{item['id']} claims a captured native result", errors)
    if len(oracle_coords) != 2:
        fail("exactly two declared repeatable oracles are allowed in this freeze", errors)
    for coord in oracle_coords:
        if oracle_counts[coord["id"]] < 12:
            fail(f"{coord['id']} has {oracle_counts[coord['id']]} oracle cases, need >= 12", errors)

    extra_oracles = set(oracle_counts) - {c["id"] for c in oracle_coords}
    if extra_oracles:
        fail(f"oracle cases exist for undeclared coordinates {sorted(extra_oracles)[:5]}", errors)

    oracle_dir = root / "acceptance/corpus/development/oracle"
    if oracle_dir.is_dir():
        for path in oracle_dir.glob("*.jsonl"):
            for _, row in _iter_jsonl(path):
                shape = row.get("expected_native_shape") or {}
                if not shape.get("format") or not shape.get("required_keys"):
                    fail(f"{row.get('id')} missing expected native shape", errors)
                if not row.get("input_path"):
                    fail(f"{row.get('id')} missing repeatable oracle input", errors)

    doctor_path = root / "acceptance/corpus/development/doctor/doctor-corpus.jsonl"
    if not doctor_path.is_file():
        fail("missing doctor corpus", errors)
        return
    doctor_rows = [row for _, row in _iter_jsonl(doctor_path)]
    if len(doctor_rows) < 250:
        fail(f"doctor corpus has {len(doctor_rows)} cases, need >= 250", errors)
    dedicated_clean = sum(1 for row in doctor_rows if row.get("class") == "clean")
    lookalike = sum(1 for row in doctor_rows if row.get("class") == "clean-lookalike")
    issue = sum(1 for row in doctor_rows if row.get("class") == "reachable-issue")
    if dedicated_clean < 125:
        fail(f"doctor dedicated clean={dedicated_clean}, need >= 125", errors)
    if issue < 125:
        fail(f"doctor reachable-issue={issue}, need >= 125", errors)
    if dedicated_clean + lookalike + issue != len(doctor_rows):
        fail("doctor classes must be clean, clean-lookalike, or reachable-issue", errors)
    for row in doctor_rows:
        if not row.get("input_path") or not row.get("input_digest_sha256"):
            fail(f"{row.get('id')} missing detector input", errors)
        if "expected_findings" not in row:
            fail(f"{row.get('id')} missing expected findings", errors)
        if row.get("class") == "reachable-issue" and row.get("polarity") == "positive":
            if not row.get("expected_findings"):
                fail(f"{row.get('id')} positive issue has empty expected findings", errors)

    blocking = defaultdict(lambda: Counter())
    for row in doctor_rows:
        if row.get("blocking") and row.get("rule_id"):
            blocking[row["rule_id"]][row["polarity"]] += 1
    for rule in DOCTOR_BLOCKING_RULES:
        pos = blocking[rule]["positive"]
        neg = blocking[rule]["negative"]
        if pos < 20 or neg < 20:
            fail(f"blocking rule {rule} pos={pos} neg={neg}, need 20/20", errors)

    for rel in files:
        path = root / rel
        if not rel.endswith(".jsonl") or not path.is_file():
            continue
        for line_no, row in _iter_jsonl(path):
            digest = row.get("digest")
            if not digest:
                fail(f"{rel}:{line_no} missing digest", errors)
                continue
            body = dict(row)
            body.pop("digest", None)
            actual = sha256_text(canonical_json(body))
            if actual != digest:
                fail(f"{rel}:{line_no} digest mismatch", errors)
                break

    live_coords = [item.get("coordinate_id") for item in fixtures if item.get("corpus") == "live"]
    if len(live_coords) != len(set(live_coords)):
        fail("duplicate live coordinates", errors)

    visible = _git_candidate_paths(root)
    if root.resolve() == ROOT.resolve() and visible is None:
        fail("unable to read git candidate set for corpus input membership", errors)
    elif visible is not None:
        reported: set[str] = set()
        for item in fixtures:
            entries: list[object] = []
            if isinstance(item.get("input_files"), list):
                entries.extend(item["input_files"])
            rec = jsonl_by_id.get(item.get("id"))
            if rec and isinstance(rec[1].get("input_files"), list):
                entries.extend(rec[1]["input_files"])
            for entry in entries:
                path = entry.get("path") if isinstance(entry, dict) else None
                if not path or path in reported:
                    continue
                if path not in visible:
                    reported.add(path)
                    fail(f"{item.get('id')} input {path} is not in the git candidate set", errors)

    print(f"static coordinates meeting >=60: {sum(1 for c in static_coords if static_counts[c] >= 60)}/{len(static_coords)}")
    print(f"declared oracles: {len(oracle_coords)}")
    print(f"doctor cases: {len(doctor_rows)} clean={dedicated_clean} lookalike={lookalike} issue={issue}")


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--structure", action="store_true")
    group.add_argument("--traceability", action="store_true")
    group.add_argument("--corpus", action="store_true")
    args = parser.parse_args()
    errors: list[str] = []
    if args.structure:
        check_structure(errors)
        label = "structure"
    elif args.traceability:
        check_traceability(errors)
        label = "traceability"
    else:
        check_corpus(errors)
        label = "corpus"
    if errors:
        print(f"check_acceptance.py --{label} FAIL")
        for item in errors:
            print(f"- {item}")
        return 1
    print(f"check_acceptance.py --{label} PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
