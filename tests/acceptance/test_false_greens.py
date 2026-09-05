#!/usr/bin/env python3
"""Negative tests that reproduce prior foundation false greens.

Mutates in-memory snapshots only. Does not write the worktree or TMPDIR.
"""

from __future__ import annotations

import csv
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from contexpect_memoryfs import memory_root, snapshot_paths  # noqa: E402

from check_acceptance import check_corpus, check_structure, check_traceability  # noqa: E402
from check_docs import check_markdown_links, check_security_channel  # noqa: E402
from contexpect_contract import (  # noqa: E402
    ARTIFACT_DIGEST_MANIFEST,
    CAPABILITIES,
    NA_RULES_FAMILIES,
    NO_SKILLS_FAMILIES,
    ORACLE_NATIVE_SHAPES,
    SCHEMA_VERSION,
    assign_trace_metadata,
    canonical_json,
    dump_json_yaml,
    extract_normative_statements,
    family_ids,
    load_json_yaml,
    load_prd_text,
    required_loss_cells,
    sha256_text,
    write_traceability_csv,
)
from contexpect_fixtures import (  # noqa: E402
    build_doctor_files,
    build_oracle_files,
    build_static_files,
    canonical_loss_native_paths,
    dumps,
    interpret_oracle,
    interpret_static,
    mutate_native_trigger,
    validate_doctor_structure,
    validate_oracle_structure,
    validate_static_structure,
)


_EIGHT_NAMES = [
    "compatibility-matrix.yaml",
    "claim-validity-matrix.yaml",
    "context-capability-matrix.yaml",
    "projection-matrix.yaml",
    "integration-contracts.yaml",
    "corpus-manifest.json",
    "traceability.csv",
    "reference-hardware.md",
    "artifact-digest-manifest.json",
]
_ACCEPTANCE_CACHE: dict[str, str] | None = None


def _copy_eight_artifacts():
    rels = [f"acceptance/{name}" for name in _EIGHT_NAMES]
    extra = "acceptance/semantic-team-contract.yaml"
    if (ROOT / extra).is_file():
        rels.append(extra)
    return memory_root(snapshot_paths(ROOT, rels))


def _copy_acceptance():
    global _ACCEPTANCE_CACHE
    if _ACCEPTANCE_CACHE is None:
        _ACCEPTANCE_CACHE = snapshot_paths(ROOT, ["acceptance"])
    return memory_root(dict(_ACCEPTANCE_CACHE))


def _refresh_digests(dst_root) -> None:
    artifacts = []
    for item in load_json_yaml(dst_root / ARTIFACT_DIGEST_MANIFEST)["artifacts"]:
        path = dst_root / item["path"]
        artifacts.append(
            {
                "path": item["path"],
                "digest_sha256": sha256_text(path.read_text(encoding="utf-8")),
                "schema_id": item.get("schema_id"),
            }
        )
    manifest = load_json_yaml(dst_root / ARTIFACT_DIGEST_MANIFEST)
    manifest["artifacts"] = artifacts
    (dst_root / ARTIFACT_DIGEST_MANIFEST).write_text(dump_json_yaml(manifest), encoding="utf-8")


def _copy_field_maps(dst_root) -> None:
    dst_root._store.update(snapshot_paths(ROOT, ["acceptance/field-to-claim"]))


def _copy_prd(dst_root) -> None:
    rel = "docs/requirements/2026-09-04-contexpect-complete-product-requirements.md"
    dst_root._store[rel] = (ROOT / rel).read_text(encoding="utf-8")


class StructureNegatives(unittest.TestCase):
    def test_floor_cell_demoted_fails(self) -> None:
        root = _copy_eight_artifacts()
        proj_path = root / "acceptance/projection-matrix.yaml"
        proj = load_json_yaml(proj_path)
        cell = next(item for item in proj["cells"] if item["id"] == "codex/packaged-skills/project-or-user")
        cell["required_write"] = False
        proj_path.write_text(dump_json_yaml(proj), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("demoted" in item or "packaged-skills" in item for item in errors))

    def test_capability_cartesian_cell_removed_fails(self) -> None:
        root = _copy_eight_artifacts()
        caps_path = root / "acceptance/context-capability-matrix.yaml"
        caps = load_json_yaml(caps_path)
        caps["cells"] = caps["cells"][1:]
        caps_path.write_text(dump_json_yaml(caps), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("Cartesian" in item or "missing Cartesian" in item for item in errors))

    def test_composite_placeholder_fails(self) -> None:
        root = _copy_eight_artifacts()
        compat_path = root / "acceptance/compatibility-matrix.yaml"
        compat = load_json_yaml(compat_path)
        compat["coordinates"][0]["version_source"] = "official-same-version-not-published-or-not-captured"
        compat_path.write_text(dump_json_yaml(compat), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("placeholder" in item for item in errors))

    def test_required_write_field_removed_fails(self) -> None:
        root = _copy_eight_artifacts()
        proj_path = root / "acceptance/projection-matrix.yaml"
        proj = load_json_yaml(proj_path)
        cell = next(item for item in proj["cells"] if item.get("required_write"))
        del cell["native_target"]
        proj_path.write_text(dump_json_yaml(proj), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(errors, "removing native_target must fail structure")
        self.assertTrue(any("native_target" in item for item in errors))

    def test_placeholder_pin_fails(self) -> None:
        root = _copy_eight_artifacts()
        integ_path = root / "acceptance/integration-contracts.yaml"
        integ = load_json_yaml(integ_path)
        integ["contracts"][0]["version_pin"] = "cutoff-unverified"
        integ["contracts"][0]["license"] = "inspect-at-pin-time"
        integ_path.write_text(dump_json_yaml(integ), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("forbidden placeholder" in item or "cutoff-unverified" in item for item in errors))

    def test_hermetic_cutoff_version_fails(self) -> None:
        root = _copy_eight_artifacts()
        compat_path = root / "acceptance/compatibility-matrix.yaml"
        compat = load_json_yaml(compat_path)
        compat["coordinates"][0]["version"] = "hermetic-cutoff-2026-09-04"
        compat_path.write_text(dump_json_yaml(compat), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("placeholder" in item or "hermetic-cutoff" in item for item in errors))

    def test_required_write_authority_export_only_fails(self) -> None:
        root = _copy_eight_artifacts()
        proj_path = root / "acceptance/projection-matrix.yaml"
        proj = load_json_yaml(proj_path)
        cell = next(item for item in proj["cells"] if item["id"] == "codex/packaged-skills/project-or-user")
        cell["authority"] = "export-only"
        proj_path.write_text(dump_json_yaml(proj), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("export-only" in item for item in errors))

    def test_missing_or_empty_field_mapping_fails(self) -> None:
        root = _copy_eight_artifacts()
        caps_path = root / "acceptance/context-capability-matrix.yaml"
        caps = load_json_yaml(caps_path)
        caps["cells"][0]["field_mapping"] = ""
        caps_path.write_text(dump_json_yaml(caps), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("field_mapping" in item or "empty field" in item for item in errors))
        root = _copy_eight_artifacts()
        caps_path = root / "acceptance/context-capability-matrix.yaml"
        caps = load_json_yaml(caps_path)
        del caps["cells"][0]["field_mapping"]
        caps_path.write_text(dump_json_yaml(caps), encoding="utf-8")
        _refresh_digests(root)
        errors = []
        check_structure(errors, root)
        self.assertTrue(any("field_mapping" in item for item in errors))

    def test_swapped_family_cross_link_fails(self) -> None:
        root = _copy_eight_artifacts()
        caps_path = root / "acceptance/context-capability-matrix.yaml"
        caps = load_json_yaml(caps_path)
        cell = caps["cells"][0]
        cell["family_id"] = "cursor" if cell["family_id"] != "cursor" else "codex"
        caps_path.write_text(dump_json_yaml(caps), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(
            any("does not match" in item or "canonical Cartesian" in item or "compatibility" in item for item in errors)
        )

    def test_schema_version_999_fails(self) -> None:
        root = _copy_eight_artifacts()
        compat_path = root / "acceptance/compatibility-matrix.yaml"
        compat = load_json_yaml(compat_path)
        compat["schema_version"] = 999
        compat_path.write_text(dump_json_yaml(compat), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("schema_version" in item and "999" in item for item in errors))

    def test_empty_or_unknown_sensitivity_fails(self) -> None:
        root = _copy_eight_artifacts()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["fixtures"][0]["sensitivity"] = ""
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("sensitivity" in item for item in errors))
        root = _copy_eight_artifacts()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["fixtures"][0]["sensitivity"] = "classified"
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors = []
        check_structure(errors, root)
        self.assertTrue(any("unknown sensitivity" in item or "classified" in item for item in errors))

    def test_empty_claim_axes_fails(self) -> None:
        root = _copy_eight_artifacts()
        claim_path = root / "acceptance/claim-validity-matrix.yaml"
        claim = load_json_yaml(claim_path)
        claim["axes"] = {}
        claim_path.write_text(dump_json_yaml(claim), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("axes" in item for item in errors))

    def test_empty_claim_invariants_fails(self) -> None:
        root = _copy_eight_artifacts()
        claim_path = root / "acceptance/claim-validity-matrix.yaml"
        claim = load_json_yaml(claim_path)
        claim["invariants"] = []
        claim_path.write_text(dump_json_yaml(claim), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("invariants" in item for item in errors))

    def test_wrong_artifact_identity_fails(self) -> None:
        root = _copy_eight_artifacts()
        compat_path = root / "acceptance/compatibility-matrix.yaml"
        compat = load_json_yaml(compat_path)
        compat["artifact"] = "not-the-compatibility-matrix"
        compat_path.write_text(dump_json_yaml(compat), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("artifact identity" in item for item in errors))

    def test_digest_schema_id_mismatch_fails(self) -> None:
        root = _copy_eight_artifacts()
        digest_path = root / "acceptance/artifact-digest-manifest.json"
        manifest = load_json_yaml(digest_path)
        manifest["artifacts"][0]["schema_id"] = "wrong-schema"
        digest_path.write_text(dump_json_yaml(manifest), encoding="utf-8")
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("schema_id" in item for item in errors))

    def test_digest_schema_id_duplicate_fails(self) -> None:
        root = _copy_eight_artifacts()
        digest_path = root / "acceptance/artifact-digest-manifest.json"
        manifest = load_json_yaml(digest_path)
        manifest["artifacts"][1]["schema_id"] = manifest["artifacts"][0]["schema_id"]
        digest_path.write_text(dump_json_yaml(manifest), encoding="utf-8")
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("duplicate" in item and "schema_id" in item for item in errors))

    def test_capability_duplicate_id_fails(self) -> None:
        root = _copy_eight_artifacts()
        caps_path = root / "acceptance/context-capability-matrix.yaml"
        caps = load_json_yaml(caps_path)
        caps["cells"].append(dict(caps["cells"][0]))
        caps_path.write_text(dump_json_yaml(caps), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("duplicate" in item for item in errors))

    def test_compatibility_duplicate_id_fails(self) -> None:
        root = _copy_eight_artifacts()
        compat_path = root / "acceptance/compatibility-matrix.yaml"
        compat = load_json_yaml(compat_path)
        compat["coordinates"].append(dict(compat["coordinates"][0]))
        compat_path.write_text(dump_json_yaml(compat), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("duplicate" in item for item in errors))

    def test_coordinate_version_field_mismatch_fails(self) -> None:
        root = _copy_eight_artifacts()
        compat_path = root / "acceptance/compatibility-matrix.yaml"
        compat = load_json_yaml(compat_path)
        compat["coordinates"][0]["version"] = "9.9.9-not-frozen"
        compat_path.write_text(dump_json_yaml(compat), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(
            any("does not match" in item or "version" in item for item in errors)
        )

    def test_missing_projection_cell_fails(self) -> None:
        root = _copy_eight_artifacts()
        proj_path = root / "acceptance/projection-matrix.yaml"
        proj = load_json_yaml(proj_path)
        proj["cells"] = proj["cells"][:-1]
        proj_path.write_text(dump_json_yaml(proj), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("missing canonical cells" in item or "projection-matrix missing" in item for item in errors))

    def test_duplicate_integration_id_fails(self) -> None:
        root = _copy_eight_artifacts()
        integ_path = root / "acceptance/integration-contracts.yaml"
        integ = load_json_yaml(integ_path)
        integ["contracts"].append(dict(integ["contracts"][0]))
        integ_path.write_text(dump_json_yaml(integ), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("duplicate" in item for item in errors))

    def test_self_lowered_floors_fail(self) -> None:
        root = _copy_eight_artifacts()
        proj_path = root / "acceptance/projection-matrix.yaml"
        proj = load_json_yaml(proj_path)
        proj["required_write_floor"] = "self-described-none"
        for cell in proj["cells"]:
            cell["required_write"] = False
        proj_path.write_text(dump_json_yaml(proj), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("demoted" in item or "required-write" in item for item in errors))


class TraceabilityNegatives(unittest.TestCase):
    def test_content_ids_stable_when_unrelated_sentence_inserted(self) -> None:
        original = load_prd_text(ROOT)
        old = [assign_trace_metadata(item) for item in extract_normative_statements(original)]
        old_ids = {item["requirement_id"] for item in old}
        old_digests = {item["source_digest"] for item in old}
        mutated = original.replace(
            "## 3. 产品原则",
            "## 3. 产品原则\n\n本句必须被追踪为插入稳定性探针。\n",
            1,
        )
        new = [assign_trace_metadata(item) for item in extract_normative_statements(mutated)]
        new_ids = {item["requirement_id"] for item in new}
        new_digests = {item["source_digest"] for item in new}
        self.assertTrue(old_ids <= new_ids)
        self.assertTrue(old_digests <= new_digests)
        self.assertGreater(len(new_ids), len(old_ids))

    def test_duplicate_requirement_id_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_prd(root)
        csv_path = root / "acceptance/traceability.csv"
        text = csv_path.read_text(encoding="utf-8")
        lines = text.splitlines()
        self.assertGreater(len(lines), 2)
        csv_path.write_text("\n".join(lines + [lines[1]]) + "\n", encoding="utf-8")
        errors: list[str] = []
        check_traceability(errors, root)
        self.assertTrue(any("duplicate" in item for item in errors))

    def test_semantic_swap_valid_enum_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_prd(root)
        csv_path = root / "acceptance/traceability.csv"
        with csv_path.open(encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle))
        target = next(row for row in rows if row["feature"] == "F-01")
        target["feature"] = "F-02"
        write_traceability_csv(csv_path, rows)
        errors: list[str] = []
        check_traceability(errors, root)
        self.assertTrue(any("canonical mapping" in item and "feature" in item for item in errors))

    def test_bogus_feature_metadata_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_prd(root)
        csv_path = root / "acceptance/traceability.csv"
        with csv_path.open(encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle))
        rows[0]["feature"] = "F-99"
        write_traceability_csv(csv_path, rows)
        errors: list[str] = []
        check_traceability(errors, root)
        self.assertTrue(
            any("bogus feature" in item or "canonical mapping" in item for item in errors)
        )

    def test_duplicate_sentence_across_sections_keeps_both(self) -> None:
        original = load_prd_text(ROOT)
        probe = "本句必须被追踪为跨节重复探针。"
        mutated = original.replace("## 3. 产品原则", "## 3. 产品原则\n\n" + probe + "\n", 1)
        mutated = mutated.replace("## 4. ", "## 4. \n\n" + probe + "\n", 1)
        rows = [assign_trace_metadata(item) for item in extract_normative_statements(mutated)]
        matches = [item for item in rows if item["statement"] == probe]
        self.assertEqual(len(matches), 2)
        self.assertEqual(len({item["requirement_id"] for item in matches}), 2)
        self.assertEqual(len({item["source_digest"] for item in matches}), 1)
        self.assertNotEqual(matches[0]["section"], matches[1]["section"])

    def test_dropped_duplicate_occurrence_fails(self) -> None:
        root = _copy_eight_artifacts()
        original = load_prd_text(ROOT)
        probe = "本句必须被追踪为跨节重复探针。"
        mutated = original.replace("## 3. 产品原则", "## 3. 产品原则\n\n" + probe + "\n", 1)
        mutated = mutated.replace("## 4. ", "## 4. \n\n" + probe + "\n", 1)
        prd_path = root / "docs/requirements/2026-09-04-contexpect-complete-product-requirements.md"
        prd_path.parent.mkdir(parents=True, exist_ok=True)
        prd_path.write_text(mutated, encoding="utf-8")
        rows = [assign_trace_metadata(item) for item in extract_normative_statements(mutated)]
        matches = [item for item in rows if item["statement"] == probe]
        self.assertEqual(len(matches), 2)
        kept = [item for item in rows if item["requirement_id"] != matches[1]["requirement_id"]]
        write_traceability_csv(root / "acceptance/traceability.csv", kept)
        errors: list[str] = []
        check_traceability(errors, root)
        self.assertTrue(any("occurrence identity" in item or "untracked" in item or "dropped" in item for item in errors))

    def test_feature_scope_closes_at_headings(self) -> None:
        statements = [assign_trace_metadata(item) for item in extract_normative_statements(load_prd_text(ROOT))]
        f01 = [item for item in statements if item["heading"].startswith("F-01")]
        self.assertTrue(f01)
        self.assertTrue(all(item["feature"] == "F-01" for item in f01))
        leaked = [
            item
            for item in statements
            if item["feature"] == "F-18" and (item["section"].startswith("9") or item["heading"].startswith("9."))
        ]
        self.assertFalse(leaked)


class CorpusNegatives(unittest.TestCase):
    def test_manifest_jsonl_digest_mismatch_fails(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = next(item for item in manifest["fixtures"] if item.get("kind") == "static")
        target["digest_sha256"] = "0" * 64
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(any("manifest digest != JSONL" in item for item in errors))

    def test_missing_physical_input_fails(self) -> None:
        root = _copy_acceptance()
        static_dir = root / "acceptance/corpus/development/static/inputs"
        victims = (
            list(static_dir.rglob("AGENTS.md"))
            + list(static_dir.rglob("envelope.json"))
            + list(static_dir.rglob(".clinerules"))
        )
        self.assertTrue(victims)
        victims[0].unlink()
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(any("physical input missing" in item or "input digest mismatch" in item for item in errors))

    def test_record_path_digest_mismatch_after_refresh_fails(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = next(
            item
            for item in manifest["fixtures"]
            if item.get("kind") == "static" and item.get("input_path")
        )
        jsonl_rel = target["path"]
        jsonl_path = root / jsonl_rel
        rows = [json.loads(line) for line in jsonl_path.read_text(encoding="utf-8").splitlines() if line.strip()]
        row = next(item for item in rows if item["id"] == target["id"])
        row["digest"] = "0" * 64
        row["input_path"] = "acceptance/corpus/development/static/inputs/does-not-exist"
        jsonl_path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")
        new_file_digest = sha256_text(jsonl_path.read_text(encoding="utf-8"))
        for item in manifest["files"]:
            if item["path"] == jsonl_rel:
                item["digest_sha256"] = new_file_digest
        target["digest_sha256"] = "0" * 64
        target["input_path"] = row["input_path"]
        target["input_digest_sha256"] = row.get("input_digest_sha256")
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(
            any(
                "digest mismatch" in item
                or "manifest digest" in item
                or "input_path" in item
                or "physical input" in item
                for item in errors
            )
        )

    def test_label_only_static_fixture_fails(self) -> None:
        errors: list[str] = []
        files = {
            "envelope.json": dumps(
                {
                    "schema_version": SCHEMA_VERSION,
                    "kind": "static-fixture",
                    "family_id": "codex",
                    "capability_id": "instructions",
                    "polarity": "positive",
                    "parse": {"included": True},
                }
            ),
            "AGENTS.md": "# family=codex capability=instructions\n",
        }
        validate_static_structure(
            {
                "id": "dev:static:label",
                "family_id": "codex",
                "capability_id": "instructions",
                "polarity": "positive",
            },
            files,
            errors,
        )
        self.assertTrue(any("label-only" in item for item in errors))

    def test_native_trigger_mutation_stale_expected_output_fails(self) -> None:
        root = _copy_acceptance()
        jsonl_rel = "acceptance/corpus/development/static/codex__cli.jsonl"
        jsonl_path = root / jsonl_rel
        rows = [json.loads(line) for line in jsonl_path.read_text(encoding="utf-8").splitlines() if line.strip()]
        target = next(
            item
            for item in rows
            if item.get("capability_id") == "instructions" and item.get("polarity") == "positive"
        )
        input_dir = root / target["input_path"]
        agents = input_dir / "AGENTS.md"
        self.assertTrue(agents.is_file())
        agents.write_text("## ignored\nThis instruction is excluded from discovery.\n", encoding="utf-8")
        (input_dir / ".ctxpect-ignore").write_text("AGENTS.md\n", encoding="utf-8")
        listed = []
        for child in sorted(input_dir.rglob("*")):
            if child.is_file():
                rel = child.relative_to(root).as_posix()
                listed.append({"path": rel, "digest_sha256": sha256_text(child.read_text(encoding="utf-8"))})
        new_input_digest = sha256_text(canonical_json(listed))
        target["input_files"] = listed
        target["input_digest_sha256"] = new_input_digest
        stale = dict(target.get("expected_output") or {})
        target["expected_output"] = stale
        body = dict(target)
        body.pop("digest", None)
        target["digest"] = sha256_text(canonical_json(body))
        jsonl_path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")
        new_file_digest = sha256_text(jsonl_path.read_text(encoding="utf-8"))
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        for item in manifest.get("files") or []:
            if item.get("path") == jsonl_rel:
                item["digest_sha256"] = new_file_digest
        for item in manifest.get("fixtures") or []:
            if item.get("id") == target["id"]:
                item["digest_sha256"] = target["digest"]
                item["input_digest_sha256"] = new_input_digest
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(
            any("expected_output" in item or "native parser" in item for item in errors),
            msg=errors[:8],
        )

    def test_interpreter_trigger_sensitivity_for_all_families(self) -> None:
        for family_id in family_ids():
            for cap in CAPABILITIES:
                files = build_static_files(family_id, cap["id"], "positive", "include")
                parsed = interpret_static(family_id, cap["id"], files)
                mutated = mutate_native_trigger(family_id, cap["id"], files)
                parsed2 = interpret_static(family_id, cap["id"], mutated)
                unsupported = (
                    cap["id"] == "other-unknown"
                    or (family_id in NA_RULES_FAMILIES and cap["id"] == "rules")
                    or (family_id in NO_SKILLS_FAMILIES and cap["id"] in {"skills", "plugins"})
                    or parsed.get("truth_state") == "indeterminate"
                    or parsed.get("loss_report_required")
                )
                if unsupported:
                    self.assertEqual(
                        parsed.get("truth_state"),
                        parsed2.get("truth_state"),
                        msg=f"{family_id}/{cap['id']} unsupported primitive must stay {parsed.get('truth_state')}",
                    )
                else:
                    self.assertNotEqual(
                        parsed,
                        parsed2,
                        msg=f"{family_id}/{cap['id']} native trigger mutation must change parser output",
                    )

    def test_trigger_label_doctor_fails(self) -> None:
        errors: list[str] = []
        files = {
            "envelope.json": dumps(
                {
                    "schema_version": SCHEMA_VERSION,
                    "kind": "doctor-positive",
                    "rule_id": "duplicate",
                    "polarity": "positive",
                }
            ),
            "AGENTS.md": "trigger: duplicate\n",
        }
        validate_doctor_structure(
            {"id": "dev:doctor:duplicate:positive:00", "rule_id": "duplicate", "polarity": "positive"},
            files,
            errors,
        )
        self.assertTrue(any("label-only" in item or "identical markdown" in item for item in errors))


class FieldMapIdentity(unittest.TestCase):
    def test_field_map_identity_mismatch(self) -> None:
        root = _copy_eight_artifacts()
        _copy_field_maps(root)
        foreign = load_json_yaml(root / "acceptance/field-to-claim/aider-cli.yaml")
        dest = root / "acceptance/field-to-claim/codex-cli.yaml"
        dest.write_text(dump_json_yaml(foreign), encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(
            any(
                "path/content identity" in item
                or "field-map family_id" in item
                or "duplicate field-map identity" in item
                for item in errors
            ),
            msg=errors[:12],
        )

    def test_field_map_removed_required_field_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_field_maps(root)
        path = root / "acceptance/field-to-claim/codex-cli.yaml"
        doc = load_json_yaml(path)
        del doc["version"]
        path.write_text(dump_json_yaml(doc), encoding="utf-8")
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("missing required fields" in item and "version" in item for item in errors), msg=errors[:8])

    def test_field_map_unexpected_field_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_field_maps(root)
        path = root / "acceptance/field-to-claim/codex-cli.yaml"
        doc = load_json_yaml(path)
        doc["unexpected_field"] = True
        path.write_text(dump_json_yaml(doc), encoding="utf-8")
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(any("unknown fields" in item and "unexpected_field" in item for item in errors), msg=errors[:8])

    def test_field_map_duplicate_identity_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_field_maps(root)
        path = root / "acceptance/field-to-claim/grok-build-cli.yaml"
        doc = load_json_yaml(path)
        doc["family_id"] = "codex"
        doc["surface"] = "cli"
        doc["version"] = load_json_yaml(root / "acceptance/field-to-claim/codex-cli.yaml")["version"]
        path.write_text(dump_json_yaml(doc), encoding="utf-8")
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(
            any("duplicate field-map identity" in item or "path/content identity" in item for item in errors),
            msg=errors[:8],
        )

    def test_field_map_directory_removed_fails(self) -> None:
        root = _copy_eight_artifacts()
        self.assertFalse((root / "acceptance/field-to-claim").exists())
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(
            any("missing acceptance/field-to-claim directory" in item for item in errors),
            msg=errors[:8],
        )

    def test_field_map_extra_foreign_file_fails(self) -> None:
        root = _copy_eight_artifacts()
        _copy_field_maps(root)
        foreign = {
            "schema_version": SCHEMA_VERSION,
            "cutoff": load_json_yaml(root / "acceptance/field-to-claim/codex-cli.yaml")["cutoff"],
            "artifact": "field-to-claim",
            "family_id": "codex",
            "surface": "cli",
            "version": load_json_yaml(root / "acceptance/field-to-claim/codex-cli.yaml")["version"],
            "live_tested": False,
            "fields": [
                {
                    "native_field": "foreign",
                    "lifecycle_stage": "installed",
                    "claim_kind": "resolved",
                    "coverage": "partial-declared-surface",
                    "minimum_evidence": "official-spec",
                }
            ],
        }
        extra = root / "acceptance/field-to-claim/foreign-extra.yaml"
        extra.write_text(dump_json_yaml(foreign), encoding="utf-8")
        errors: list[str] = []
        check_structure(errors, root)
        self.assertTrue(
            any("extra foreign" in item or "count" in item for item in errors),
            msg=errors[:8],
        )


class CorpusCrossLink(unittest.TestCase):
    def test_manifest_record_coordinate_mismatch(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        statics = [
            item
            for item in manifest["fixtures"]
            if item.get("corpus") == "development"
            and item.get("kind") == "static"
            and item.get("family_id") in {"codex", "cursor"}
            and item.get("coordinate_id")
        ]
        first = next(item for item in statics if item["family_id"] == "codex")
        second = next(item for item in statics if item["family_id"] == "cursor")
        identity_keys = ["coordinate_id", "family_id", "version", "surface", "os_lane"]
        first_identity = {key: first.get(key) for key in identity_keys}
        second_identity = {key: second.get(key) for key in identity_keys}

        def load_rows(path: Path) -> list[dict]:
            return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]

        def write_rows(path: Path, rows: list[dict]) -> None:
            path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")

        def recompute(row: dict) -> None:
            body = dict(row)
            body.pop("digest", None)
            row["digest"] = sha256_text(canonical_json(body))

        jsonl_first = root / first["path"]
        jsonl_second = root / second["path"]
        rows_first = load_rows(jsonl_first)
        rows_second = load_rows(jsonl_second)
        row_first = next(item for item in rows_first if item["id"] == first["id"])
        row_second = next(item for item in rows_second if item["id"] == second["id"])
        for key, value in second_identity.items():
            row_first[key] = value
            first[key] = value
        for key, value in first_identity.items():
            row_second[key] = value
            second[key] = value
        recompute(row_first)
        recompute(row_second)
        first["digest_sha256"] = row_first["digest"]
        second["digest_sha256"] = row_second["digest"]
        write_rows(jsonl_first, rows_first)
        write_rows(jsonl_second, rows_second)
        for item in manifest.get("files") or []:
            if item.get("path") == first["path"]:
                item["digest_sha256"] = sha256_text(jsonl_first.read_text(encoding="utf-8"))
            if item.get("path") == second["path"]:
                item["digest_sha256"] = sha256_text(jsonl_second.read_text(encoding="utf-8"))
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(
            any("coordinate" in item or "id-encoded" in item or "field-derived" in item for item in errors),
            msg=errors[:12],
        )

    def test_manifest_removed_identity_fields_fail(self) -> None:
        fields = ["family_id", "surface", "version", "os_lane", "input_path", "input_digest_sha256"]
        for field in fields:
            with self.subTest(field=field):
                root = _copy_acceptance()
                manifest_path = root / "acceptance/corpus-manifest.json"
                manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                target = next(
                    item
                    for item in manifest["fixtures"]
                    if item.get("corpus") == "development"
                    and item.get("kind") == "static"
                    and field in item
                )
                del target[field]
                manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
                _refresh_digests(root)
                structure_errors: list[str] = []
                check_structure(structure_errors, root)
                corpus_errors: list[str] = []
                check_corpus(corpus_errors, root)
                self.assertTrue(
                    any(field in item for item in structure_errors),
                    msg=f"structure did not reject missing {field}: {structure_errors[:8]}",
                )
                self.assertTrue(
                    any(field in item for item in corpus_errors),
                    msg=f"corpus did not reject missing {field}: {corpus_errors[:8]}",
                )

    def test_manifest_record_kind_mismatch(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = next(
            item
            for item in manifest["fixtures"]
            if item.get("corpus") == "development" and item.get("kind") == "static"
        )
        jsonl_path = root / target["path"]
        rows = [json.loads(line) for line in jsonl_path.read_text(encoding="utf-8").splitlines() if line.strip()]
        row = next(item for item in rows if item["id"] == target["id"])
        target["kind"] = "oracle"
        row["kind"] = "oracle"
        body = dict(row)
        body.pop("digest", None)
        row["digest"] = sha256_text(canonical_json(body))
        target["digest_sha256"] = row["digest"]
        jsonl_path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")
        for item in manifest.get("files") or []:
            if item.get("path") == target["path"]:
                item["digest_sha256"] = sha256_text(jsonl_path.read_text(encoding="utf-8"))
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(
            any("kind" in item for item in errors),
            msg=errors[:12],
        )

    def test_manifest_record_native_shape_mismatch(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        first = next(
            item
            for item in manifest["fixtures"]
            if item.get("kind") == "oracle" and item.get("oracle_id") == "debug-prompt-input"
        )
        second = next(
            item
            for item in manifest["fixtures"]
            if item.get("kind") == "oracle" and item.get("oracle_id") == "inspect-json"
        )

        def load_rows(path: Path) -> list[dict]:
            return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]

        def write_rows(path: Path, rows: list[dict]) -> None:
            path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")

        def recompute(row: dict) -> None:
            body = dict(row)
            body.pop("digest", None)
            row["digest"] = sha256_text(canonical_json(body))

        jsonl_first = root / first["path"]
        jsonl_second = root / second["path"]
        rows_first = load_rows(jsonl_first)
        rows_second = load_rows(jsonl_second)
        row_first = next(item for item in rows_first if item["id"] == first["id"])
        row_second = next(item for item in rows_second if item["id"] == second["id"])
        first_shape = first["expected_native_shape"]
        second_shape = second["expected_native_shape"]
        first["expected_native_shape"] = second_shape
        second["expected_native_shape"] = first_shape
        row_first["expected_native_shape"] = second_shape
        row_second["expected_native_shape"] = first_shape
        recompute(row_first)
        recompute(row_second)
        first["digest_sha256"] = row_first["digest"]
        second["digest_sha256"] = row_second["digest"]
        write_rows(jsonl_first, rows_first)
        write_rows(jsonl_second, rows_second)
        for item in manifest.get("files") or []:
            if item.get("path") == first["path"]:
                item["digest_sha256"] = sha256_text(jsonl_first.read_text(encoding="utf-8"))
            if item.get("path") == second["path"]:
                item["digest_sha256"] = sha256_text(jsonl_second.read_text(encoding="utf-8"))
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(
            any("native-shape" in item or "required_keys" in item or "canonical" in item for item in errors),
            msg=errors[:12],
        )

    def test_jsonl_duplicate_id_fails(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = next(item for item in manifest["fixtures"] if item.get("kind") == "static")
        jsonl_path = root / target["path"]
        text = jsonl_path.read_text(encoding="utf-8")
        first = next(line for line in text.splitlines() if line.strip())
        jsonl_path.write_text(text + first + "\n", encoding="utf-8")
        new_digest = sha256_text(jsonl_path.read_text(encoding="utf-8"))
        for item in manifest.get("files") or []:
            if item.get("path") == target["path"]:
                item["digest_sha256"] = new_digest
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(any("duplicate JSONL id" in item for item in errors), msg=errors[:8])

    def test_unmanifested_jsonl_record_fails(self) -> None:
        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = next(item for item in manifest["fixtures"] if item.get("kind") == "static")
        jsonl_path = root / target["path"]
        extra = {
            "id": "dev:static:unmanifested-record",
            "schema_version": SCHEMA_VERSION,
            "corpus": "development",
            "kind": "static",
            "coordinate_id": "unmanifested/coord",
            "license": "Apache-2.0",
            "sensitivity": "public-synthetic",
            "live_tested": False,
            "digest": "0" * 64,
        }
        jsonl_path.write_text(jsonl_path.read_text(encoding="utf-8") + canonical_json(extra) + "\n", encoding="utf-8")
        new_digest = sha256_text(jsonl_path.read_text(encoding="utf-8"))
        for item in manifest.get("files") or []:
            if item.get("path") == target["path"]:
                item["digest_sha256"] = new_digest
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(any("unmanifested JSONL record" in item for item in errors), msg=errors[:8])

    def test_unmanifested_jsonl_file_fails(self) -> None:
        root = _copy_acceptance()
        extra_path = root / "acceptance/corpus/development/static/unmanifested.jsonl"
        extra = {
            "id": "dev:static:unmanifested-file",
            "schema_version": SCHEMA_VERSION,
            "corpus": "development",
            "kind": "static",
            "coordinate_id": "unmanifested/file",
            "license": "Apache-2.0",
            "sensitivity": "public-synthetic",
            "live_tested": False,
            "digest": "0" * 64,
        }
        extra_path.write_text(canonical_json(extra) + "\n", encoding="utf-8")
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(any("unmanifested JSONL file" in item for item in errors), msg=errors[:8])

    def test_jsonl_manifest_bijection_holds(self) -> None:
        manifest = json.loads((ROOT / "acceptance/corpus-manifest.json").read_text(encoding="utf-8"))
        jsonl_ids: set[str] = set()
        for item in manifest.get("files") or []:
            rel = item.get("path") or ""
            if not rel.endswith(".jsonl"):
                continue
            path = ROOT / rel
            for line in path.read_text(encoding="utf-8").splitlines():
                if line.strip():
                    jsonl_ids.add(json.loads(line)["id"])
        manifest_ids = {item["id"] for item in manifest["fixtures"]}
        self.assertEqual(manifest_ids, jsonl_ids)


class LossNative(unittest.TestCase):
    def test_loss_native_evidence_required(self) -> None:
        for cell in required_loss_cells():
            files = build_static_files(cell["family_id"], cell["capability_id"], "loss", "loss")
            parsed = interpret_static(
                cell["family_id"],
                cell["capability_id"],
                files,
                surface=cell["surface"],
                os_lane=cell["os_lane"],
            )
            self.assertTrue(parsed.get("loss_report_required"), msg=cell["id"])
            row = {
                "id": cell["id"],
                "family_id": cell["family_id"],
                "capability_id": cell["capability_id"],
                "polarity": "loss",
                "surface": cell["surface"],
                "os_lane": cell["os_lane"],
            }
            ok: list[str] = []
            validate_static_structure(row, files, ok)
            self.assertFalse(ok, msg=ok)

            removed = dict(files)
            removed.pop("loss-report.json", None)
            parsed_removed = interpret_static(
                cell["family_id"], cell["capability_id"], removed, surface=cell["surface"], os_lane=cell["os_lane"]
            )
            self.assertFalse(parsed_removed.get("loss_report_required"), msg=cell["id"])
            err: list[str] = []
            validate_static_structure(row, removed, err)
            self.assertTrue(any("loss" in item for item in err), msg=err)

            emptied = dict(files)
            emptied["loss-report.json"] = "\n"
            err = []
            validate_static_structure(row, emptied, err)
            self.assertTrue(err, msg=cell["id"])
            self.assertFalse(
                interpret_static(
                    cell["family_id"], cell["capability_id"], emptied, surface=cell["surface"], os_lane=cell["os_lane"]
                ).get("loss_report_required")
            )

            invalid = dict(files)
            invalid["loss-report.json"] = dumps(
                {
                    "schema": "contexpect-loss-v1",
                    "missing_primitive": cell["capability_id"],
                    "projects_to": "instructions",
                    "family_id": cell["family_id"],
                    "native_paths_inspected": ["../outside/rules.md"],
                }
            )
            err = []
            validate_static_structure(row, invalid, err)
            self.assertTrue(any("invalid native path" in item for item in err), msg=err)

            unrelated = dict(files)
            unrelated["README.md"] = "# unrelated safe path\n"
            unrelated["loss-report.json"] = dumps(
                {
                    "schema": "contexpect-loss-v1",
                    "missing_primitive": cell["capability_id"],
                    "projects_to": "instructions",
                    "family_id": cell["family_id"],
                    "native_paths_inspected": ["README.md"],
                }
            )
            err = []
            validate_static_structure(row, unrelated, err)
            canonical = canonical_loss_native_paths(cell["family_id"], cell["capability_id"])
            self.assertNotEqual(canonical, ["README.md"], msg=cell["id"])
            self.assertTrue(any("canonical" in item for item in err), msg=err)
            self.assertFalse(
                interpret_static(
                    cell["family_id"],
                    cell["capability_id"],
                    unrelated,
                    surface=cell["surface"],
                    os_lane=cell["os_lane"],
                ).get("loss_report_required"),
                msg=cell["id"],
            )

            corrupt = dict(files)
            corrupt["loss-report.json"] = dumps({"schema": "not-a-loss-schema", "missing_primitive": cell["capability_id"]})
            err = []
            validate_static_structure(row, corrupt, err)
            self.assertTrue(err, msg=cell["id"])
            self.assertFalse(
                interpret_static(
                    cell["family_id"], cell["capability_id"], corrupt, surface=cell["surface"], os_lane=cell["os_lane"]
                ).get("loss_report_required")
            )

            mutated = mutate_native_trigger(cell["family_id"], cell["capability_id"], files)
            parsed_mut = interpret_static(
                cell["family_id"], cell["capability_id"], mutated, surface=cell["surface"], os_lane=cell["os_lane"]
            )
            self.assertNotEqual(parsed.get("loss_report_required"), parsed_mut.get("loss_report_required"), msg=cell["id"])

        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = next(
            item
            for item in manifest["fixtures"]
            if item.get("kind") == "static"
            and (item.get("class") == "loss" or str(item.get("id") or "").endswith(":loss"))
            and item.get("input_path")
        )
        jsonl_path = root / target["path"]
        rows = [json.loads(line) for line in jsonl_path.read_text(encoding="utf-8").splitlines() if line.strip()]
        row = next(item for item in rows if item["id"] == target["id"])
        report = root / row["input_path"] / "loss-report.json"
        self.assertTrue(report.is_file(), msg=row["input_path"])
        report.unlink()
        listed = []
        input_dir = root / row["input_path"]
        for child in sorted(input_dir.rglob("*")):
            if child.is_file():
                rel = child.relative_to(root).as_posix()
                listed.append({"path": rel, "digest_sha256": sha256_text(child.read_text(encoding="utf-8"))})
        row["input_files"] = listed
        row["input_digest_sha256"] = sha256_text(canonical_json(listed))
        body = dict(row)
        body.pop("digest", None)
        row["digest"] = sha256_text(canonical_json(body))
        jsonl_path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")
        new_file_digest = sha256_text(jsonl_path.read_text(encoding="utf-8"))
        for item in manifest.get("files") or []:
            if item.get("path") == target["path"]:
                item["digest_sha256"] = new_file_digest
        for item in manifest.get("fixtures") or []:
            if item.get("id") == target["id"]:
                item["digest_sha256"] = row["digest"]
                item["input_digest_sha256"] = row["input_digest_sha256"]
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors: list[str] = []
        check_corpus(errors, root)
        self.assertTrue(
            any("loss" in item or "expected_output" in item or "native" in item for item in errors),
            msg=errors[:12],
        )

        root = _copy_acceptance()
        manifest_path = root / "acceptance/corpus-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        mutated_files: set[str] = set()
        for cell in required_loss_cells():
            fid = f"dev:static:{cell['coordinate_id']}:{cell['capability_id']}:loss"
            target = next(item for item in manifest["fixtures"] if item["id"] == fid)
            jsonl_path = root / target["path"]
            rows = [json.loads(line) for line in jsonl_path.read_text(encoding="utf-8").splitlines() if line.strip()]
            row = next(item for item in rows if item["id"] == fid)
            report = root / row["input_path"] / "loss-report.json"
            payload = json.loads(report.read_text(encoding="utf-8"))
            self.assertEqual(
                payload.get("native_paths_inspected"),
                canonical_loss_native_paths(cell["family_id"], cell["capability_id"]),
                msg=fid,
            )
            payload["native_paths_inspected"] = ["README.md"]
            report.write_text(dumps(payload), encoding="utf-8")
            readme = root / row["input_path"] / "README.md"
            readme.write_text("# unrelated safe path\n", encoding="utf-8")
            listed = []
            input_dir = root / row["input_path"]
            for child in sorted(input_dir.rglob("*")):
                if child.is_file():
                    rel = child.relative_to(root).as_posix()
                    listed.append({"path": rel, "digest_sha256": sha256_text(child.read_text(encoding="utf-8"))})
            row["input_files"] = listed
            row["input_digest_sha256"] = sha256_text(canonical_json(listed))
            body = dict(row)
            body.pop("digest", None)
            row["digest"] = sha256_text(canonical_json(body))
            jsonl_path.write_text("\n".join(canonical_json(item) for item in rows) + "\n", encoding="utf-8")
            mutated_files.add(target["path"])
            for item in manifest.get("fixtures") or []:
                if item.get("id") == fid:
                    item["digest_sha256"] = row["digest"]
                    item["input_digest_sha256"] = row["input_digest_sha256"]
        for rel in mutated_files:
            path = root / rel
            for item in manifest.get("files") or []:
                if item.get("path") == rel:
                    item["digest_sha256"] = sha256_text(path.read_text(encoding="utf-8"))
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        _refresh_digests(root)
        errors = []
        check_corpus(errors, root)
        self.assertGreaterEqual(sum(1 for item in errors if "canonical" in item or "native_paths_inspected" in item), 8)
        self.assertTrue(
            any("canonical" in item or "native_paths_inspected" in item for item in errors),
            msg=errors[:12],
        )


class OracleNative(unittest.TestCase):
    def test_oracle_native_schema_strict(self) -> None:
        cases = (
            ("codex", "debug-prompt-input", ["codex", "debug", "prompt-input"], "task-message", "prompt-probe.json", "prompt-result.json"),
            ("grok-build", "inspect-json", ["grok", "inspect", "--json"], "instructions", "inspect-probe.json", "inspect-result.json"),
        )
        for family_id, oracle_id, command, capability_id, probe_name, result_name in cases:
            shape = dict(ORACLE_NATIVE_SHAPES[oracle_id])
            files = build_oracle_files(family_id, capability_id, "positive", command, shape)
            row = {
                "id": f"dev:oracle:{family_id}",
                "family_id": family_id,
                "capability_id": capability_id,
                "oracle_id": oracle_id,
                "command": command,
                "expected_native_shape": shape,
            }
            errors: list[str] = []
            validate_oracle_structure(row, files, errors)
            self.assertFalse(errors, msg=errors)
            parsed = interpret_oracle(family_id, capability_id, files)
            self.assertEqual(parsed.get("truth_state"), "present")
            self.assertEqual(parsed.get("command"), command)

            malformed = dict(files)
            malformed[result_name] = "{not-json"
            err: list[str] = []
            validate_oracle_structure(row, malformed, err)
            self.assertTrue(any("JSON" in item for item in err), msg=err)
            self.assertNotEqual(interpret_oracle(family_id, capability_id, malformed).get("truth_state"), "present")

            missing_cmd = dict(files)
            probe = json.loads(missing_cmd[probe_name])
            del probe["command"]
            missing_cmd[probe_name] = dumps(probe)
            err = []
            validate_oracle_structure(row, missing_cmd, err)
            self.assertTrue(any("missing command" in item for item in err), msg=err)
            parsed_missing = interpret_oracle(family_id, capability_id, missing_cmd)
            self.assertIsNone(parsed_missing.get("command"))

            missing_result = dict(files)
            missing_result.pop(result_name)
            err = []
            validate_oracle_structure(row, missing_result, err)
            self.assertTrue(any("missing" in item for item in err), msg=err)

            bogus = dict(row)
            bogus["expected_native_shape"] = {"format": "json", "required_keys": ["bogus"]}
            err = []
            validate_oracle_structure(bogus, files, err)
            self.assertTrue(any("bogus required keys" in item for item in err), msg=err)

            mismatch = dict(files)
            probe = json.loads(mismatch[probe_name])
            probe["result_path"] = "AGENTS.md"
            mismatch[probe_name] = dumps(probe)
            err = []
            validate_oracle_structure(row, mismatch, err)
            self.assertTrue(any("path mismatch" in item for item in err), msg=err)

            labels_only = {
                "AGENTS.md": "## always\nOracle workspace instructions.\n",
                "envelope.json": files["envelope.json"],
                probe_name: dumps({"capability_id": capability_id, "live_tested": False}),
            }
            parsed_labels = interpret_oracle(family_id, capability_id, labels_only)
            self.assertNotEqual(parsed_labels.get("truth_state"), "present")
            self.assertIsNone(parsed_labels.get("command"))


class CandidateMembership(unittest.TestCase):
    def test_manifest_input_files_are_candidate_visible(self) -> None:
        proc = subprocess.run(
            ["git", "-C", str(ROOT), "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
            capture_output=True,
            check=False,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr.decode("utf-8"))
        visible = {item for item in proc.stdout.decode("utf-8").split("\0") if item}
        manifest = json.loads((ROOT / "acceptance/corpus-manifest.json").read_text(encoding="utf-8"))
        missing: list[str] = []
        for item in manifest["fixtures"]:
            for entry in item.get("input_files") or []:
                path = entry.get("path") if isinstance(entry, dict) else None
                if path and path not in visible:
                    missing.append(path)
        jsonl_files = [item["path"] for item in manifest.get("files") or [] if str(item.get("path") or "").endswith(".jsonl")]
        for rel in jsonl_files:
            path = ROOT / rel
            for line in path.read_text(encoding="utf-8").splitlines():
                if not line.strip():
                    continue
                row = json.loads(line)
                for entry in row.get("input_files") or []:
                    input_path = entry.get("path") if isinstance(entry, dict) else None
                    if input_path and input_path not in visible:
                        missing.append(input_path)
        self.assertEqual(missing, [], msg=missing[:12])

    def test_ignore_fixtures_do_not_emit_git_gitignore(self) -> None:
        files = build_static_files("grok-build", "instructions", "negative", "ignore")
        self.assertNotIn(".gitignore", files)
        self.assertIn(".ctxpect-ignore", files)
        doctor_files, findings = build_doctor_files("gitignore_mismatch", "positive", "reachable-issue", 0)
        self.assertNotIn(".gitignore", doctor_files)
        self.assertIn(".ctxpect-gitignore", doctor_files)
        self.assertIn("tracked-secret.md", doctor_files)
        self.assertTrue(any(item.get("rule_id") == "gitignore_mismatch" for item in findings))


class DocsNegatives(unittest.TestCase):
    def test_missing_reference_definition_fails(self) -> None:
        root = memory_root({})
        md = root / "CODE_OF_CONDUCT.md"
        md.write_text(
            "# t\n\n> 状态：规范（尚未实施产品运行时）\n\n"
            "This Code of Conduct is adapted from the [Contributor Covenant][homepage].\n",
            encoding="utf-8",
        )
        errors: list[str] = []
        check_markdown_links(errors, root)
        self.assertTrue(any("missing reference-style definition" in item for item in errors))

    def test_missing_reference_relative_target_fails(self) -> None:
        root = memory_root({})
        md = root / "CODE_OF_CONDUCT.md"
        md.write_text(
            "# t\n\n> 状态：规范（尚未实施产品运行时）\n\n"
            "This Code of Conduct is adapted from the [Contributor Covenant][homepage],\n"
            "version 2.1.\n\n"
            "[homepage]: ./no-such-covenant.md\n",
            encoding="utf-8",
        )
        errors: list[str] = []
        check_markdown_links(errors, root)
        self.assertTrue(any("broken relative" in item for item in errors))

    def test_broken_relative_link_fails(self) -> None:
        root = memory_root({})
        md = root / "docs" / "broken.md"
        md.parent.mkdir(parents=True)
        md.write_text("# t\n\n> 状态：规范（尚未实施产品运行时）\n\n[missing](./no-such-file.md)\n", encoding="utf-8")
        errors: list[str] = []
        check_markdown_links(errors, root)
        self.assertTrue(any("broken relative link" in item for item in errors))

    def test_missing_private_reporting_url_fails(self) -> None:
        root = memory_root({})
        (root / "SECURITY.md").write_text("# Security\n\n> 状态：规范（尚未实施产品运行时）\n\nNo channel.\n", encoding="utf-8")
        (root / "GOVERNANCE.md").write_text("# G\n\n> 状态：规范（尚未实施产品运行时）\n", encoding="utf-8")
        (root / "CODE_OF_CONDUCT.md").write_text("# C\n\n> 状态：规范（尚未实施产品运行时）\n", encoding="utf-8")
        errors: list[str] = []
        check_security_channel(errors, root)
        self.assertTrue(any("private vulnerability reporting URL" in item for item in errors))


if __name__ == "__main__":
    unittest.main()
