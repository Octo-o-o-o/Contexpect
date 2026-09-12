#!/usr/bin/env python3
"""Deterministic gate for semantic alignment and Team Context Standard fixtures.

Offline. No network. No third-party packages.
Exit 0 on pass, 1 on fail. Prints check_semantic_team.py PASS or FAIL.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

from contexpect_contract import (  # noqa: E402
    ARTIFACT_DIGEST_MANIFEST,
    CLAIM_RECONCILIATION_STATES,
    CUTOFF,
    LICENSE_ID,
    canonical_json,
    load_json_yaml,
    sha256_text,
)
from contexpect_schema import require_closed_object  # noqa: E402
from contexpect_semantic_team import (  # noqa: E402
    FIXTURE_ALLOWED,
    FIXTURE_REQUIRED,
    PAYLOAD_FIELDS,
    PROJECTION_OUTCOMES,
    REQUIRED_FOUR,
    REQUIRED_MALFORMED,
    REQUIRED_SCENARIOS,
    SEMANTIC_TEAM_ARTIFACT,
    SEMANTIC_TEAM_CONTRACT,
    SEMANTIC_TEAM_DIR,
    _case_digest,
    collect_violations,
    family_native_targets,
    iter_payloads,
    validate_contract_object,
)

FOUR_PATHS = {
    "codex": "AGENTS.md",
    "claude-code": "CLAUDE.md",
    "cursor": ".cursor/rules/testing.mdc",
    "grok-build": "AGENTS.md",
}


def fail(message: str, errors: list[str]) -> None:
    errors.append(message)


def _load_contract(root: Path, errors: list[str]) -> dict | None:
    path = root / SEMANTIC_TEAM_CONTRACT
    if not path.is_file():
        fail(f"missing {SEMANTIC_TEAM_CONTRACT}", errors)
        return None
    try:
        obj = load_json_yaml(path)
    except Exception as exc:  # noqa: BLE001 — gate must fail closed on malformed JSON
        fail(f"{SEMANTIC_TEAM_CONTRACT} is not JSON-compatible YAML: {exc}", errors)
        return None
    if not isinstance(obj, dict):
        fail("semantic-team-contract is not an object", errors)
        return None
    return obj


def _fixture_files(root: Path) -> list[Path]:
    directory = root / SEMANTIC_TEAM_DIR
    if not directory.is_dir():
        return []
    return sorted(path for path in directory.rglob("*") if path.is_file())


def check_digest_manifest(root: Path, errors: list[str]) -> None:
    digest_path = root / ARTIFACT_DIGEST_MANIFEST
    if not digest_path.is_file():
        fail(f"missing {ARTIFACT_DIGEST_MANIFEST}", errors)
        return
    try:
        manifest = load_json_yaml(digest_path)
    except Exception as exc:  # noqa: BLE001
        fail(f"{ARTIFACT_DIGEST_MANIFEST} is not JSON: {exc}", errors)
        return
    artifacts = manifest.get("artifacts") if isinstance(manifest, dict) else None
    if not isinstance(artifacts, list):
        fail("artifact-digest-manifest.artifacts is not a list", errors)
        return
    hashed = {item.get("path"): item for item in artifacts if isinstance(item, dict)}
    if SEMANTIC_TEAM_CONTRACT not in hashed:
        fail(f"{ARTIFACT_DIGEST_MANIFEST} missing {SEMANTIC_TEAM_CONTRACT}", errors)
        return
    contract_path = root / SEMANTIC_TEAM_CONTRACT
    if contract_path.is_file():
        actual = sha256_text(contract_path.read_text(encoding="utf-8"))
        recorded = hashed[SEMANTIC_TEAM_CONTRACT].get("digest_sha256")
        if recorded != actual:
            fail(f"artifact digest mismatch for {SEMANTIC_TEAM_CONTRACT}", errors)
    schema_id = hashed[SEMANTIC_TEAM_CONTRACT].get("schema_id")
    if schema_id != SEMANTIC_TEAM_ARTIFACT:
        fail(f"{SEMANTIC_TEAM_CONTRACT} digest schema_id {schema_id!r} != {SEMANTIC_TEAM_ARTIFACT!r}", errors)


def check_bijection(contract: dict, root: Path, errors: list[str]) -> dict[str, str]:
    manifest = contract.get("content_digest_manifest") or []
    if not isinstance(manifest, list) or not manifest:
        fail("semantic-team-contract content_digest_manifest is empty", errors)
        return {}
    recorded = {}
    for item in manifest:
        if not isinstance(item, dict):
            fail("content_digest_manifest entry is not an object", errors)
            continue
        path = item.get("path")
        digest = item.get("digest_sha256")
        extra = sorted(set(item) - {"path", "digest_sha256"})
        if extra:
            fail(f"content_digest_manifest unknown fields {extra}", errors)
        if not path or not digest:
            fail("content_digest_manifest entry missing path or digest", errors)
            continue
        recorded[path] = digest
    actual_files = _fixture_files(root)
    actual = {}
    for path in actual_files:
        rel = path.relative_to(root).as_posix()
        actual[rel] = sha256_text(path.read_text(encoding="utf-8"))
    extra = sorted(set(actual) - set(recorded))
    missing = sorted(set(recorded) - set(actual))
    if extra:
        fail(f"semantic-team fixture files not in contract manifest {extra[:8]}", errors)
    if missing:
        fail(f"contract manifest paths missing on disk {missing[:8]}", errors)
    for rel, digest in sorted(actual.items()):
        if rel in recorded and recorded[rel] != digest:
            fail(f"content_digest_manifest mismatch for {rel}", errors)
    if SEMANTIC_TEAM_CONTRACT in recorded:
        fail("contract must not hash itself inside content_digest_manifest", errors)
    return actual


def check_fixture_record(rel: str, fixture: dict, errors: list[str]) -> None:
    require_closed_object(
        fixture,
        FIXTURE_ALLOWED,
        FIXTURE_REQUIRED,
        rel,
        errors,
        nonempty=["id", "scenario", "payload", "digest"],
    )
    if fixture.get("kind") != "semantic-team-fixture":
        fail(f"{rel} kind must be semantic-team-fixture", errors)
    if fixture.get("live_tested") is not False:
        fail(f"{rel} live_tested must be false", errors)
    if fixture.get("license") != LICENSE_ID:
        fail(f"{rel} license must be Apache-2.0", errors)
    if fixture.get("cutoff") != CUTOFF:
        fail(f"{rel} cutoff mismatch", errors)
    if fixture.get("id") != fixture.get("scenario"):
        fail(f"{rel} id/scenario mismatch", errors)
    polarity = fixture.get("polarity")
    expected_result = fixture.get("expected_gate_result")
    if polarity == "positive" and expected_result != "pass":
        fail(f"{rel} positive fixture expected_gate_result must be pass", errors)
    if polarity == "negative" and expected_result != "fail":
        fail(f"{rel} negative fixture expected_gate_result must be fail", errors)
    digest = fixture.get("digest")
    actual_digest = _case_digest(fixture)
    if digest != actual_digest:
        fail(f"{rel} digest does not match canonical bytes", errors)
    payload = fixture.get("payload")
    if not isinstance(payload, dict):
        fail(f"{rel} payload is not an object", errors)
        return
    extra = sorted(set(payload) - set(PAYLOAD_FIELDS))
    missing = sorted(set(PAYLOAD_FIELDS) - set(payload))
    if extra:
        fail(f"{rel} payload unknown fields {extra}", errors)
    if missing:
        fail(f"{rel} payload missing fields {missing}", errors)


def check_fixture_semantics(fixture: dict, errors: list[str]) -> None:
    scenario = fixture.get("id") or "fixture"
    polarity = fixture.get("polarity")
    expected = set(fixture.get("expected_violations") or [])
    actual: set[str] = set()
    for payload in iter_payloads(fixture):
        got = collect_violations(payload, scenario=fixture.get("id"))
        actual.update(got)
        variant_expected = set(payload.get("expected_violations") or [])
        if variant_expected and not variant_expected <= set(got):
            fail(
                f"{scenario} variant missing violations {sorted(variant_expected - set(got))}; actual={sorted(got)}",
                errors,
            )
        if polarity == "positive":
            for proj in payload.get("projections") or []:
                if not isinstance(proj, dict):
                    continue
                outcome = proj.get("projection_outcome")
                recon = proj.get("reconciliation_state")
                if outcome in CLAIM_RECONCILIATION_STATES or outcome == "verified":
                    fail(f"{scenario} projection outcome {outcome!r} is a reconciliation state", errors)
                if outcome in {"unsupported", "unknown", "lossy"}:
                    if recon not in {"structural-only", "indeterminate"}:
                        fail(
                            f"{scenario} honest {outcome!r} projection requires structural-only or indeterminate reconciliation",
                            errors,
                        )
                    if recon == "verified":
                        fail(f"{scenario} {outcome!r} projection must not be reported verified", errors)
    if polarity == "positive":
        if actual:
            fail(f"{scenario} positive fixture has violations {sorted(actual)}", errors)
        if expected:
            fail(f"{scenario} positive fixture must not declare expected_violations", errors)
    else:
        if not actual:
            fail(f"{scenario} negative fixture passed", errors)
        if expected and not expected <= actual:
            fail(
                f"{scenario} missing expected violations {sorted(expected - actual)}; actual={sorted(actual)}",
                errors,
            )


def check_st2_projection_equivalence(fixtures: dict[str, dict], errors: list[str]) -> None:
    # C-F02 (2026-09-12): byte identity is neither proof of semantic equivalence nor
    # evidence of non-equivalence, so the former "four fingerprints must differ" and
    # per-pair "path/syntax must differ" assertions were removed. Byte identity as an
    # equivalence *basis* stays forbidden via the binding assertions below and via
    # collect_violations (byte_identical=True or a byte/hash equivalence_basis fails);
    # byte-identical projections across families are permitted, as ST2-same-bytes-pos
    # demonstrates.
    fixture = fixtures.get("ST2-pos")
    if not fixture:
        fail("ST2-pos missing; cannot check native projection equivalence semantics", errors)
        return
    projections = [
        item
        for item in (fixture.get("payload") or {}).get("projections") or []
        if isinstance(item, dict) and item.get("family_id") in REQUIRED_FOUR
    ]
    present = [item.get("family_id") for item in projections]
    missing = [fid for fid in REQUIRED_FOUR if fid not in present]
    if missing:
        fail(f"ST2-pos missing required harness projections {missing}", errors)
        return
    paths = {item["family_id"]: item.get("native_path") for item in projections if item["family_id"] in REQUIRED_FOUR}
    for family_id, expected_path in FOUR_PATHS.items():
        if paths.get(family_id) != expected_path:
            fail(f"ST2-pos {family_id} native path {paths.get(family_id)!r} != declared {expected_path!r}", errors)
    bindings = (fixture.get("payload") or {}).get("equivalence_bindings") or []
    if not any(item.get("equivalent") for item in bindings if isinstance(item, dict)):
        fail("ST2-pos must still mark the four native projections semantically equivalent", errors)
    if any(item.get("byte_identical") for item in bindings if isinstance(item, dict)):
        fail("ST2-pos must not treat byte identity as equivalence", errors)
    if any(item.get("equivalence_basis") in {"byte-equality", "hash-equality"} for item in bindings if isinstance(item, dict)):
        fail("ST2-pos equivalence_basis must not be byte/hash equality", errors)
    outcomes = {item.get("projection_outcome") for item in projections}
    if "verified" in outcomes:
        fail("ST2-pos projection outcome must not be verified", errors)
    allowed = {"exact", "native-equivalent", "transformed", "lossless-native-overlay"}
    if not outcomes <= allowed:
        fail(f"ST2-pos projection outcomes {sorted(outcomes)} are not exact/native-equivalent/transformed", errors)


def check_semantic_team(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    contract = _load_contract(root, errors)
    if contract is None:
        return
    validate_contract_object(contract, errors)
    check_digest_manifest(root, errors)
    check_bijection(contract, root, errors)

    mapping = contract.get("fixture_id_mapping") or {}
    required = list(contract.get("required_scenarios") or REQUIRED_SCENARIOS)
    malformed = list(contract.get("required_malformed_fixtures") or REQUIRED_MALFORMED)
    fixtures: dict[str, dict] = {}
    for path in _fixture_files(root):
        rel = path.relative_to(root).as_posix()
        try:
            obj = load_json_yaml(path)
        except Exception as exc:  # noqa: BLE001
            fail(f"{rel} is not JSON-compatible: {exc}", errors)
            continue
        if not isinstance(obj, dict):
            fail(f"{rel} is not an object", errors)
            continue
        check_fixture_record(rel, obj, errors)
        fid = obj.get("id")
        if fid in fixtures:
            fail(f"duplicate semantic-team fixture id {fid}", errors)
        if fid:
            fixtures[fid] = obj
        check_fixture_semantics(obj, errors)

    for scenario in required + malformed:
        mapped = mapping.get(scenario, scenario)
        if mapped not in fixtures:
            fail(f"required semantic-team scenario missing: {scenario}", errors)

    expected_targets = {row["family_id"]: row for row in family_native_targets()}
    actual_targets = {
        row.get("family_id"): row
        for row in (contract.get("family_native_targets") or [])
        if isinstance(row, dict)
    }
    if set(expected_targets) != set(actual_targets):
        fail("family_native_targets must cover every declared FAMILIES id", errors)
    for family_id, expected in expected_targets.items():
        actual = actual_targets.get(family_id) or {}
        for key in ("native_path", "syntax", "scope", "precedence", "lifecycle", "primitives", "managed_channel"):
            if actual.get(key) != expected.get(key):
                fail(f"{family_id} native target {key} does not match the locked family grammar", errors)

    if "verified" in PROJECTION_OUTCOMES:
        fail("locked projection outcome enum must not contain verified", errors)
    if list(CLAIM_RECONCILIATION_STATES) != ["verified", "structural-only", "indeterminate", "failed"]:
        fail("CLAIM_RECONCILIATION_STATES mutated", errors)

    check_st2_projection_equivalence(fixtures, errors)


def main() -> int:
    errors: list[str] = []
    check_semantic_team(errors)
    if errors:
        print("check_semantic_team.py FAIL")
        for item in errors:
            print(f"- {item}")
        return 1
    print("check_semantic_team.py PASS")
    print(f"scenarios: {len(REQUIRED_SCENARIOS)}")
    print(f"malformed fixtures: {len(REQUIRED_MALFORMED)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
