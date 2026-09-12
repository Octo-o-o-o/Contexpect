#!/usr/bin/env python3
"""Negative tests for the semantic-alignment / Team Context Standard gate.

Mutates in-memory builder output only. Does not write the worktree or TMPDIR.
"""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from contexpect_contract import family_ids  # noqa: E402
from contexpect_semantic_team import (  # noqa: E402
    AUDIT_EVENTS_BY_STATE,
    EQUIVALENCE_BASES,
    DECLARED_ORACLE_COMMANDS,
    EVALUATION_TIME,
    FAMILY_PATH_EXTRAS,
    FROZEN_PAST,
    FROZEN_REQUESTED,
    INTENT_ID,
    LIFECYCLE_REQUIRED_GROUPS,
    PIPELINE_STEPS,
    POLICY_ID,
    PREVIEW_AT,
    REPORT_AT,
    REQUIRED_FOUR,
    SCENARIO_GRAPH,
    SEMANTIC_TEAM_CONTRACT,
    SEMANTIC_TEAM_DIR,
    _case_digest,
    _digest_obj,
    all_declared_projections,
    audit_chain,
    bound_receipt,
    build_semantic_team_files,
    build_st1_pos,
    build_st2_pos,
    build_st2_same_bytes_pos,
    build_st3_pos,
    build_st4_pos,
    build_st5_pos,
    build_st6_pos,
    build_st7_pos,
    build_st8_pos,
    bind_disclosure,
    bind_receipts,
    bind_report_payload,
    bind_update,
    collect_violations,
    covering_exception_for,
    disclosure_binding_digest,
    family_allowed_paths,
    family_native_content,
    family_required_paths,
    four_native_files,
    native_bundle_digest,
    intent_provenance_digest,
    overlay_ok,
    recompute_effective_row,
    registry_capability_status,
    resign_exception,
    sha256_text,
    sign_standard,
    synthetic_sign,
    valid_exception,
    validate_contract_object,
)


def _mutate_fixture(builder, mutator):
    fixture = copy.deepcopy(builder())
    mutator(fixture)
    fixture["digest"] = _case_digest(fixture)
    return fixture


def _payload_violations(fixture) -> list[str]:
    return collect_violations(fixture.get("payload") or {}, scenario=fixture.get("id"))


class SemanticTeamNegatives(unittest.TestCase):
    def test_malformed_contract_fails(self) -> None:
        import json

        files = build_semantic_team_files()
        obj = json.loads(files[SEMANTIC_TEAM_CONTRACT])
        obj["unexpected_field"] = "not-allowed"
        errors: list[str] = []
        validate_contract_object(obj, errors)
        self.assertTrue(any("unknown fields" in item or "unexpected_field" in item for item in errors))

    def test_partial_bundle_fails(self) -> None:
        fixture = _mutate_fixture(
            build_st4_pos,
            lambda obj: (
                obj["payload"]["team_standard"].pop("signatures", None),
                obj["payload"]["team_standard"].pop("expiry", None),
                obj["payload"]["team_standard"].pop("channel", None),
            ),
        )
        found = _payload_violations(fixture)
        self.assertTrue(any(code in found for code in ("truncated-bundle", "unsigned-standard", "missing-signature")))

    def test_forged_signature_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["team_standard"]["signatures"][0]["signature"] = "0" * 64

        found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
        self.assertIn("forged-publisher", found)

    def test_swapped_digest_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["team_standard"]["content_digest_manifest"][0]["digest_sha256"] = "0" * 64

        found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
        self.assertIn("swapped-digest", found)

    def test_byte_copy_as_alignment_fails(self) -> None:
        copied = four_native_files()["codex"]

        def mutate(obj: dict) -> None:
            for proj in obj["payload"]["projections"]:
                if proj["family_id"] in {"codex", "claude-code", "cursor", "grok-build"}:
                    proj["native_files"] = copied
                    proj["native_path"] = copied[0]["path"]
                    proj["equivalence_basis"] = "hash-equality"
            for binding in obj["payload"]["equivalence_bindings"]:
                binding["byte_identical"] = True
                binding["equivalence_basis"] = "hash-equality"

        found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
        self.assertTrue(any(code in found for code in ("byte-copy-as-alignment", "hash-equality-as-equivalence")))

    def test_silent_loss_marked_verified_fails(self) -> None:
        def mutate(obj: dict) -> None:
            proj = obj["payload"]["projections"][0]
            proj["projection_outcome"] = "verified"
            proj["reconciliation_state"] = "verified"
            proj["capability_negotiation"]["dropped"] = ["scoped-conditional-rules"]
            proj["capability_negotiation"]["unsupported"] = ["scoped-conditional-rules"]
            proj["capability_negotiation"]["unknown"] = ["model-visible-complete-prompt"]

        found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
        self.assertTrue(
            any(
                code in found
                for code in (
                    "projection-outcome-is-reconciliation-state",
                    "silent-capability-loss",
                    "unsupported-marked-verified",
                    "unknown-marked-verified",
                )
            )
        )

    def test_required_rule_shadow_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["layer_assignments"][-1]["shadows_higher_required"] = True
            obj["payload"]["layer_assignments"][-1]["relaxes"] = "required"
            obj["payload"]["layer_assignments"][-1]["rule_id"] = obj["payload"]["layer_assignments"][1]["rule_id"]
            obj["payload"]["layer_assignments"][-1]["mode"] = "recommended"

        found = _payload_violations(_mutate_fixture(build_st5_pos, mutate))
        self.assertTrue(any(code in found for code in ("personal-shadows-required", "required-downgraded-to-recommended")))

    def test_expired_offline_stale_exception_fail_closed(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["exceptions"][0]["freshness"] = "stale-offline"
            obj["payload"]["exceptions"][0]["state"] = "expiry"
            obj["payload"]["policy_evals"][0]["result"] = "pass"

        found = _payload_violations(_mutate_fixture(build_st6_pos, mutate))
        self.assertTrue(
            any(code in found for code in ("stale-offline-exception-pass", "expired-exception-pass", "forged-exception"))
        )

    def test_private_content_leader_disclosure_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["leader_view"]["private_prompt_text"] = "secret prompt"
            obj["payload"]["leader_view"]["full_session_history"] = [{"text": "session"}]

        found = _payload_violations(_mutate_fixture(build_st7_pos, mutate))
        self.assertTrue(any(code in found for code in ("leader-private-prompt", "leader-session-history")))

    def test_wrong_canonical_intent_types_fail(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["canonical_intents"][0]["meaning"] = "not-an-object"
            obj["payload"]["canonical_intents"][0]["scope"] = []
            obj["payload"]["canonical_intents"][0]["id"] = 12

        fixture = _mutate_fixture(build_st1_pos, mutate)
        found = _payload_violations(fixture)
        self.assertIn("canonical-intent-type", found)
        self.assertNotEqual(fixture["digest"], build_st1_pos()["digest"])

    def test_missing_reordered_pipeline_fails(self) -> None:
        def drop(obj: dict) -> None:
            obj["payload"]["projections"][0]["pipeline"] = []

        def reorder(obj: dict) -> None:
            obj["payload"]["projections"][0]["pipeline"] = list(
                reversed(obj["payload"]["projections"][0]["pipeline"])
            )

        dropped = _payload_violations(_mutate_fixture(build_st2_pos, drop))
        reordered = _payload_violations(_mutate_fixture(build_st2_pos, reorder))
        self.assertIn("pipeline-missing", dropped)
        self.assertIn("pipeline-order", reordered)
        self.assertEqual(len(PIPELINE_STEPS), 8)

    def test_missing_family_scope_precedence_lifecycle_primitives_fails(self) -> None:
        def mutate(obj: dict) -> None:
            proj = obj["payload"]["projections"][0]
            del proj["scope"]
            del proj["precedence"]
            del proj["lifecycle"]
            del proj["primitives"]

        found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
        self.assertIn("family-native-incomplete", found)

    def test_overlay_metadata_omissions_fail(self) -> None:
        def mutate(obj: dict) -> None:
            overlay = obj["payload"]["overlays"][0]
            del overlay["owner"]
            del overlay["reason"]
            del overlay["lifecycle"]
            del overlay["authority"]

        found = _payload_violations(_mutate_fixture(build_st3_pos, mutate))
        self.assertIn("overlay-metadata-missing", found)

    def test_out_of_scope_exception_fails(self) -> None:
        def mutate(obj: dict) -> None:
            overlay = overlay_ok()
            overlay["weakens_required"] = True
            overlay["digest"] = _digest_obj({k: v for k, v in overlay.items() if k != "digest"})
            obj["payload"]["overlays"] = [overlay]
            exc = valid_exception()
            exc["scope"]["harness"] = "cursor"
            obj["payload"]["exceptions"] = [exc]

        found = _payload_violations(_mutate_fixture(build_st3_pos, mutate))
        self.assertTrue(any(code in found for code in ("exception-scope-mismatch", "overlay-weakens-required-without-exception")))

    def test_empty_eight_way_bindings_fail(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["receipts"][0]["bound_digests"] = {}

        found = _payload_violations(_mutate_fixture(build_st3_pos, mutate))
        self.assertTrue(any(code in found for code in ("empty-round-trip-bindings", "missing-round-trip-digest")))

    def test_member_key_signing_publisher_fails(self) -> None:
        from contexpect_semantic_team import sign_standard, standard_unsigned

        def mutate(obj: dict) -> None:
            obj["payload"]["team_standard"] = sign_standard(standard_unsigned(), "syn-member")

        found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
        self.assertIn("member-key-as-publisher", found)

    def test_incomplete_publisher_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["team_standard"]["publisher"] = {"id": "syn-publisher-team-lead"}

        found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
        self.assertIn("incomplete-publisher", found)

    def test_required_downgraded_to_recommended_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["layer_assignments"][2]["mode"] = "recommended"

        found = _payload_violations(_mutate_fixture(build_st5_pos, mutate))
        self.assertIn("required-downgraded-to-recommended", found)

    def test_unmanaged_target_falsely_enforceable_fails(self) -> None:
        def mutate(obj: dict) -> None:
            for proj in obj["payload"]["projections"]:
                if proj["family_id"] == "cursor":
                    proj["managed_channel"] = False
            for row in obj["payload"]["effective_policies"]:
                if row["family_id"] == "cursor":
                    row["honesty"] = "enforceable"

        found = _payload_violations(_mutate_fixture(build_st5_pos, mutate))
        self.assertTrue(any(code in found for code in ("unmanaged-marked-enforceable", "false-enforcement-claim")))

    def test_incomplete_forged_exception_and_broken_audit_chain_fail(self) -> None:
        def incomplete(obj: dict) -> None:
            del obj["payload"]["exceptions"][0]["approver"]
            del obj["payload"]["exceptions"][0]["reason"]

        def forged(obj: dict) -> None:
            obj["payload"]["exceptions"][0]["signature"]["signature"] = "0" * 64

        def broken(obj: dict) -> None:
            chain = obj["payload"]["exceptions"][0]["audit_chain"]
            chain[0], chain[1] = chain[1], chain[0]

        self.assertIn("incomplete-exception", _payload_violations(_mutate_fixture(build_st6_pos, incomplete)))
        self.assertIn("forged-exception", _payload_violations(_mutate_fixture(build_st6_pos, forged)))
        self.assertIn("broken-audit-chain", _payload_violations(_mutate_fixture(build_st6_pos, broken)))

    def test_disclosure_false_missing_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["shown_before_report"] = False
            obj["payload"]["member_disclosure"]["acknowledgement"] = {}

        found = _payload_violations(_mutate_fixture(build_st7_pos, mutate))
        self.assertTrue(any(code in found for code in ("disclosure-missing", "disclosure-digest-mismatch")))

    def test_nested_private_prompt_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["leader_view"]["exception_metadata"]["private_prompt_text"] = "nested secret prompt"

        found = _payload_violations(_mutate_fixture(build_st7_pos, mutate))
        self.assertTrue(any(code in found for code in ("nested-private-content", "leader-private-prompt")))

    def test_update_without_preview_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["lifecycle"]["steps"] = [
                {"command": "standard update", "preview": False, "disclosure": False, "pin": True}
            ]

        found = _payload_violations(_mutate_fixture(build_st8_pos, mutate))
        self.assertTrue(any(code in found for code in ("update-without-preview", "update-without-disclosure")))

    def test_forged_invalid_key_rotation_fails(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["lifecycle"]["key_rotation"] = {
                "old_key_id": "syn-publisher-team-lead",
                "new_key_id": "syn-member",
                "old_trust_state": "trusted",
                "new_trust_state": "trusted",
                "rotated_at": "2026-09-04T12:00:00+08:00",
                "applied": True,
                "signature": {"key_id": "syn-member", "signature": "0" * 64},
            }

        found = _payload_violations(_mutate_fixture(build_st8_pos, mutate))
        self.assertTrue(any(code in found for code in ("invalid-key-rotation", "member-key-as-publisher")))

    def test_honest_unknown_indeterminate_is_accepted_and_not_verified(self) -> None:
        fixture = build_st2_pos()
        found = _payload_violations(fixture)
        self.assertEqual(found, [])
        unknown = [
            item
            for item in fixture["payload"]["projections"]
            if item.get("projection_outcome") == "unknown"
        ]
        self.assertTrue(unknown)
        for item in unknown:
            self.assertEqual(item.get("reconciliation_state"), "indeterminate")
            self.assertNotEqual(item.get("projection_outcome"), "verified")
            self.assertNotEqual(item.get("reconciliation_state"), "verified")

    def test_generator_twice_semantic_team_byte_identical(self) -> None:
        first = build_semantic_team_files()
        second = build_semantic_team_files()
        self.assertEqual(first, second)
        self.assertIn(SEMANTIC_TEAM_CONTRACT, first)
        self.assertTrue(any(path.startswith(SEMANTIC_TEAM_DIR + "/") for path in first))


def _refresh_intent(intent: dict) -> None:
    provenance = intent.setdefault("provenance", {})
    provenance["digest_sha256"] = intent_provenance_digest(intent)


def _resign_standard(payload_standard: dict) -> None:
    unsigned = dict(payload_standard)
    unsigned["signatures"] = []
    payload_standard.clear()
    payload_standard.update(sign_standard(unsigned))


class SemanticTeamRepair2(unittest.TestCase):
    def test_empty_required_nested_collections_fail(self) -> None:
        cases = [
            ("meaning.constraints", lambda intent: intent["meaning"].__setitem__("constraints", [])),
            ("meaning.non_goals", lambda intent: intent["meaning"].__setitem__("non_goals", [])),
            ("scope.paths", lambda intent: intent["scope"].__setitem__("paths", [])),
            ("meaning", lambda intent: intent.__setitem__("meaning", {})),
            ("scope", lambda intent: intent.__setitem__("scope", {})),
            ("precedence", lambda intent: intent.__setitem__("precedence", {})),
            ("activation", lambda intent: intent.__setitem__("activation", {})),
            ("authority", lambda intent: intent.__setitem__("authority", {})),
            ("provenance", lambda intent: intent.__setitem__("provenance", {})),
        ]
        for name, mutator in cases:
            with self.subTest(collection=name):
                def mutate(obj: dict, fn=mutator) -> None:
                    intent = obj["payload"]["canonical_intents"][0]
                    fn(intent)
                    if isinstance(intent.get("provenance"), dict) and intent["provenance"]:
                        _refresh_intent(intent)

                fixture = _mutate_fixture(build_st1_pos, mutate)
                found = _payload_violations(fixture)
                self.assertTrue(
                    any(code in found for code in ("canonical-intent-type", "canonical-intent-empty-collection", "malformed-schema")),
                    msg=f"{name} empty collection accepted: {found}",
                )
                self.assertNotEqual(fixture["digest"], build_st1_pos()["digest"])

    def test_wrong_nonempty_native_path_and_syntax_fail(self) -> None:
        def wrong_path(obj: dict) -> None:
            proj = obj["payload"]["projections"][0]
            proj["native_path"] = ".cursor/rules/testing.mdc"

        def wrong_syntax(obj: dict) -> None:
            obj["payload"]["projections"][0]["syntax"] = "grok-project-instructions"

        path_found = _payload_violations(_mutate_fixture(build_st2_pos, wrong_path))
        syntax_found = _payload_violations(_mutate_fixture(build_st2_pos, wrong_syntax))
        self.assertTrue(any(code in path_found for code in ("family-native-incomplete", "family-native-mismatch")))
        self.assertTrue(any(code in syntax_found for code in ("family-native-incomplete", "family-native-mismatch")))

    def test_cross_family_swapped_target_metadata_fails(self) -> None:
        def mutate(obj: dict) -> None:
            rows = obj["payload"]["projections"]
            codex = next(item for item in rows if item["family_id"] == "codex")
            cursor = next(item for item in rows if item["family_id"] == "cursor")
            for key in ("native_path", "syntax", "scope", "precedence", "lifecycle", "primitives"):
                codex[key] = copy.deepcopy(cursor[key])

        found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
        self.assertTrue(any(code in found for code in ("family-native-incomplete", "family-native-mismatch")))

    def test_exception_coverage_dimension_probes_fail(self) -> None:
        def weakening(obj: dict) -> dict:
            overlay = overlay_ok()
            overlay["weakens_required"] = True
            overlay["digest"] = _digest_obj({k: v for k, v in overlay.items() if k != "digest"})
            obj["payload"]["overlays"] = [overlay]
            return overlay

        def wrong_intent(obj: dict) -> None:
            overlay = weakening(obj)
            exc = covering_exception_for(overlay, intent_ids=["intent.other-rule"])
            obj["payload"]["exceptions"] = [exc]

        def wrong_policy(obj: dict) -> None:
            overlay = weakening(obj)
            exc = covering_exception_for(overlay, policy_ids=["policy.other"])
            obj["payload"]["exceptions"] = [exc]

        def wrong_digest(obj: dict) -> None:
            overlay = weakening(obj)
            exc = covering_exception_for(overlay, overlay_digest=sha256_text("unrelated-overlay"))
            obj["payload"]["exceptions"] = [exc]

        def partial_scope(obj: dict) -> None:
            overlay = weakening(obj)
            exc = covering_exception_for(overlay)
            exc["scope"]["project"] = "apps/unrelated"
            obj["payload"]["exceptions"] = [resign_exception(exc)]

        intent_found = _payload_violations(_mutate_fixture(build_st3_pos, wrong_intent))
        policy_found = _payload_violations(_mutate_fixture(build_st3_pos, wrong_policy))
        digest_found = _payload_violations(_mutate_fixture(build_st3_pos, wrong_digest))
        scope_found = _payload_violations(_mutate_fixture(build_st3_pos, partial_scope))
        self.assertIn("exception-intent-mismatch", intent_found)
        self.assertIn("overlay-weakens-required-without-exception", intent_found)
        self.assertIn("exception-policy-mismatch", policy_found)
        self.assertIn("exception-overlay-digest-mismatch", digest_found)
        self.assertTrue(any(code in scope_found for code in ("exception-scope-mismatch", "overlay-weakens-required-without-exception")))

    def test_signature_by_different_known_key_fails(self) -> None:
        def mutate(obj: dict) -> None:
            standard = obj["payload"]["team_standard"]
            unsigned = dict(standard)
            unsigned["signatures"] = []
            standard["signatures"] = [synthetic_sign(unsigned, "syn-publisher-team-lead-rotated")]

        found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
        self.assertTrue(any(code in found for code in ("publisher-identity-tuple-mismatch", "forged-publisher")))

    def test_signed_rollback_without_authorized_transaction_fails(self) -> None:
        def mutate(obj: dict) -> None:
            standard = obj["payload"]["team_standard"]
            standard["revision"] = 0
            _resign_standard(standard)

        found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
        self.assertTrue(any(code in found for code in ("rollback-version", "signed-rollback-without-authorization")))

    def test_nested_standard_empty_or_wrong_type_fails(self) -> None:
        def empty_members(obj: dict) -> None:
            obj["payload"]["team_standard"]["members"] = []
            _resign_standard(obj["payload"]["team_standard"])

        def wrong_trust(obj: dict) -> None:
            obj["payload"]["team_standard"]["trust"] = []
            _resign_standard(obj["payload"]["team_standard"])

        self.assertTrue(
            any(
                code in _payload_violations(_mutate_fixture(build_st4_pos, empty_members))
                for code in ("nested-standard-type", "truncated-bundle")
            )
        )
        self.assertIn("nested-standard-type", _payload_violations(_mutate_fixture(build_st4_pos, wrong_trust)))

    def test_self_reported_effective_fields_fail(self) -> None:
        def mode_only(obj: dict) -> None:
            obj["payload"]["effective_policies"][0]["mode"] = "recommended"

        def source_only(obj: dict) -> None:
            obj["payload"]["effective_policies"][0]["source_layer"] = "personal"
            obj["payload"]["effective_policies"][0]["source_rule_id"] = "policy.personal-theme"

        def computed_via_only(obj: dict) -> None:
            obj["payload"]["effective_policies"][0]["computed_via"] = "native-projection-pipeline"
            obj["payload"]["effective_policies"][0]["algorithm"] = "unofficial"

        def honesty_only(obj: dict) -> None:
            obj["payload"]["effective_policies"][0]["honesty"] = "detect-only"

        for mutator, expected in (
            (mode_only, "effective-policy-mismatch"),
            (source_only, "effective-policy-mismatch"),
            (computed_via_only, "computed-via-mismatch"),
            (honesty_only, "effective-policy-mismatch"),
        ):
            found = _payload_violations(_mutate_fixture(build_st5_pos, mutator))
            self.assertIn(expected, found)

    def test_exception_identity_time_and_audit_probes_fail(self) -> None:
        def unregistered(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["requester"] = "syn-unknown"
            resign_exception(exc)

        def approver_mismatch(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["approver"] = "syn-member"
            resign_exception(exc, "syn-approver-lead")

        def current_expired(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["freshness"] = "current"
            exc["timestamps"]["expires_at"] = FROZEN_PAST
            exc["time_limit"]["until"] = FROZEN_PAST
            resign_exception(exc)

        def impossible_order(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["timestamps"]["requested_at"] = REPORT_AT
            exc["timestamps"]["issued_at"] = PREVIEW_AT
            exc["timestamps"]["approved_at"] = FROZEN_PAST
            resign_exception(exc)

        def audit_mismatch(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["audit_chain"][0]["event"] = "revocation"
            body = {k: v for k, v in exc["audit_chain"][0].items() if k != "digest"}
            exc["audit_chain"][0]["digest"] = _digest_obj(body)
            resign_exception(exc)

        self.assertIn("unregistered-identity", _payload_violations(_mutate_fixture(build_st6_pos, unregistered)))
        self.assertIn("approver-signer-mismatch", _payload_violations(_mutate_fixture(build_st6_pos, approver_mismatch)))
        current_found = _payload_violations(_mutate_fixture(build_st6_pos, current_expired))
        self.assertTrue(any(code in current_found for code in ("current-labelled-expired", "expired-exception-pass")))
        self.assertIn("impossible-time-order", _payload_violations(_mutate_fixture(build_st6_pos, impossible_order)))
        self.assertIn("audit-event-state-mismatch", _payload_violations(_mutate_fixture(build_st6_pos, audit_mismatch)))
        self.assertEqual(EVALUATION_TIME.endswith("+08:00"), True)

    def test_disclosure_schema_digest_payload_and_order_fail(self) -> None:
        def arbitrary_digest(obj: dict) -> None:
            fake = sha256_text("not-disclosed-metadata")
            obj["payload"]["member_disclosure"]["disclosed_digest"] = fake
            obj["payload"]["member_disclosure"]["acknowledgement"]["digest"] = fake

        def empty_preview(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["preview"] = {}
            obj["payload"]["member_disclosure"]["previewed_fields"] = []

        def mismatched_payload(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["report_payload_digest"] = sha256_text("other-payload")

        def invalid_order(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["previewed_at"] = REPORT_AT
            obj["payload"]["member_disclosure"]["acknowledgement"]["at"] = PREVIEW_AT

        self.assertIn("disclosure-digest-mismatch", _payload_violations(_mutate_fixture(build_st7_pos, arbitrary_digest)))
        self.assertIn("empty-preview", _payload_violations(_mutate_fixture(build_st7_pos, empty_preview)))
        self.assertIn("disclosure-payload-mismatch", _payload_violations(_mutate_fixture(build_st7_pos, mismatched_payload)))
        self.assertIn("disclosure-order", _payload_violations(_mutate_fixture(build_st7_pos, invalid_order)))

    def test_lifecycle_group_deletions_and_mutations_fail(self) -> None:
        for group in LIFECYCLE_REQUIRED_GROUPS:
            with self.subTest(group=group):
                def mutate(obj: dict, name=group) -> None:
                    obj["payload"]["lifecycle"].pop(name, None)

                found = _payload_violations(_mutate_fixture(build_st8_pos, mutate))
                self.assertTrue(
                    any(
                        code in found
                        for code in (
                            "missing-lifecycle-group",
                            "update-without-preview",
                            "update-without-disclosure",
                            "last-known-good-missing",
                            "invalid-key-rotation",
                            "unsigned-rotation",
                            "incomplete-rollout",
                            "unrelated-rollback-target",
                            "conflict-nondeterministic",
                        )
                    ),
                    msg=f"deleting {group} accepted: {found}",
                )

        def missing_preview(obj: dict) -> None:
            obj["payload"]["lifecycle"]["update"]["preview"] = False
            obj["payload"]["lifecycle"]["steps"] = [
                {"command": "standard update", "preview": False, "disclosure": True}
            ]

        def unsigned_rotation(obj: dict) -> None:
            rotation = obj["payload"]["lifecycle"]["key_rotation"]
            rotation.pop("new_signature", None)
            rotation.pop("old_signature", None)

        def wrong_order(obj: dict) -> None:
            stages = obj["payload"]["lifecycle"]["staged_rollout"]["stages"]
            stages[0]["order"] = 2
            stages[1]["order"] = 1
            stages[0]["percent"] = 100
            stages[1]["percent"] = 10

        def unrelated_target(obj: dict) -> None:
            obj["payload"]["lifecycle"]["rollback"]["target_revision"] = 99
            obj["payload"]["lifecycle"]["rollback"]["target_digest"] = sha256_text("unrelated")

        self.assertIn("update-without-preview", _payload_violations(_mutate_fixture(build_st8_pos, missing_preview)))
        self.assertIn("unsigned-rotation", _payload_violations(_mutate_fixture(build_st8_pos, unsigned_rotation)))
        self.assertIn("incomplete-rollout", _payload_violations(_mutate_fixture(build_st8_pos, wrong_order)))
        self.assertIn("unrelated-rollback-target", _payload_violations(_mutate_fixture(build_st8_pos, unrelated_target)))


POSITIVE_BUILDERS = {
    "ST1-pos": build_st1_pos,
    "ST2-pos": build_st2_pos,
    "ST3-pos": build_st3_pos,
    "ST4-pos": build_st4_pos,
    "ST5-pos": build_st5_pos,
    "ST6-pos": build_st6_pos,
    "ST7-pos": build_st7_pos,
    "ST8-pos": build_st8_pos,
}

FAMILY_METADATA_MUTATIONS = (
    ("native_path", "not-a-declared-path.md"),
    ("syntax", "not-a-declared-syntax"),
    ("scope", "user"),
    ("precedence", "not-harness-native-document-order"),
    ("lifecycle", "not-preview-apply-rollback"),
    ("primitives", ["not-a-primitive"]),
    ("coordinate", "missing-family/0.0.0/none/none"),
    ("surface", "not-a-surface"),
    ("os_lane", "windows-11-24h2-x86_64"),
    ("authority", "export-only"),
)


def _assert_codes(found, *codes, label=""):
    matched = [code for code in codes if code in found]
    if not matched:
        raise AssertionError(f"{label} expected one of {codes}, got {found}")


class SemanticTeamMutationTable(unittest.TestCase):
    def test_eight_positive_scenarios_pass(self) -> None:
        for name, builder in POSITIVE_BUILDERS.items():
            with self.subTest(scenario=name):
                fixture = builder()
                found = _payload_violations(fixture)
                self.assertEqual(found, [], msg=f"{name} violations {found}")
                self.assertEqual(fixture["digest"], _case_digest(fixture))

    def test_honest_unknown_indeterminate_not_verified(self) -> None:
        fixture = build_st2_pos()
        self.assertEqual(_payload_violations(fixture), [])
        unknown = [item for item in fixture["payload"]["projections"] if item.get("projection_outcome") == "unknown"]
        self.assertTrue(unknown)
        for item in unknown:
            self.assertEqual(item.get("reconciliation_state"), "indeterminate")
            self.assertNotEqual(item.get("projection_outcome"), "verified")
            self.assertNotEqual(item.get("reconciliation_state"), "verified")

    def test_only_two_declared_repeatable_oracles_pass(self) -> None:
        fixture = build_st2_pos()
        declared = [
            item["family_id"]
            for item in fixture["payload"]["projections"]
            if item.get("oracle", {}).get("kind") == "declared-repeatable"
        ]
        self.assertEqual(set(declared), set(DECLARED_ORACLE_COMMANDS))
        self.assertEqual(len(DECLARED_ORACLE_COMMANDS), 2)

        def add_third(obj: dict) -> None:
            for proj in obj["payload"]["projections"]:
                if proj["family_id"] == "opencode":
                    proj["oracle"] = {
                        "kind": "declared-repeatable",
                        "command": ["opencode", "inspect", "--json"],
                        "native_result_captured": False,
                        "live_tested": False,
                        "honesty": "not-a-declared-oracle",
                    }

        found = _payload_violations(_mutate_fixture(build_st2_pos, add_third))
        _assert_codes(found, "undeclared-repeatable-oracle", "oracle-grammar", label="third oracle")

    def test_in_memory_generated_maps_match_disk_twice(self) -> None:
        first = build_semantic_team_files()
        second = build_semantic_team_files()
        self.assertEqual(first, second)
        disk = {rel: (ROOT / rel).read_text(encoding="utf-8") for rel in first}
        self.assertEqual(first, disk)
        self.assertEqual(second, disk)
        self.assertIn(SEMANTIC_TEAM_CONTRACT, first)
        self.assertTrue(any(path.startswith(SEMANTIC_TEAM_DIR + "/") for path in first))

    def test_top_level_empty_and_null_required_collections_fail(self) -> None:
        for scenario, spec in SCENARIO_GRAPH.items():
            builder = POSITIVE_BUILDERS[scenario]
            for field, count in (spec.get("lists") or {}).items():
                for value in ([], None):
                    with self.subTest(scenario=scenario, field=field, value=value):
                        def mutate(obj: dict, name=field, assigned=value) -> None:
                            obj["payload"][name] = assigned

                        found = _payload_violations(_mutate_fixture(builder, mutate))
                        _assert_codes(
                            found,
                            "required-collection-cardinality",
                            "canonical-intent-empty-collection",
                            "overlay-metadata-missing",
                            "empty-round-trip-bindings",
                            "required-scenario-coverage",
                            label=f"{scenario}.{field}={value!r}",
                        )
            for field in spec.get("objects") or {}:
                for value in (None, {}):
                    with self.subTest(scenario=scenario, field=field, value=value):
                        def mutate(obj: dict, name=field, assigned=value) -> None:
                            obj["payload"][name] = assigned

                        found = _payload_violations(_mutate_fixture(builder, mutate))
                        _assert_codes(
                            found,
                            "required-collection-cardinality",
                            "missing-lifecycle-group",
                            "truncated-bundle",
                            "disclosure-missing",
                            label=f"{scenario}.{field}={value!r}",
                        )

    def test_all_eighteen_family_authoritative_metadata_mutations_fail(self) -> None:
        families = family_ids()
        self.assertEqual(len(families), 18)
        present = {item["family_id"] for item in all_declared_projections()}
        self.assertEqual(present, set(families))
        for family_id in families:
            for key, wrong in FAMILY_METADATA_MUTATIONS:
                with self.subTest(family_id=family_id, field=key):
                    def mutate(obj: dict, fid=family_id, field=key, value=wrong) -> None:
                        proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                        if field == "native_path":
                            proj["native_path"] = value
                            proj["native_files"][0]["path"] = value
                        elif field == "managed_channel":
                            proj["managed_channel"] = not bool(proj.get("managed_channel"))
                        else:
                            proj[field] = copy.deepcopy(value)

                    found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
                    _assert_codes(
                        found,
                        "family-native-incomplete",
                        "family-native-mismatch",
                        label=f"{family_id}.{key}",
                    )
            with self.subTest(family_id=family_id, field="managed_channel"):
                def invert(obj: dict, fid=family_id) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                    proj["managed_channel"] = not bool(proj.get("managed_channel"))

                found = _payload_violations(_mutate_fixture(build_st2_pos, invert))
                _assert_codes(found, "family-native-incomplete", "family-native-mismatch", label=f"{family_id}.managed_channel")
            allowed = family_allowed_paths(family_id)
            self.assertTrue(allowed)
            extras = FAMILY_PATH_EXTRAS.get(family_id, ())
            for extra in extras:
                self.assertIn(extra, allowed)

    def test_b1_canonical_intent_empty_null_unknown_and_wrong_types_fail(self) -> None:
        cases = [
            ("empty-list", lambda obj: obj["payload"].__setitem__("canonical_intents", [])),
            ("null", lambda obj: obj["payload"].__setitem__("canonical_intents", None)),
            ("null-intent", lambda obj: obj["payload"]["canonical_intents"].__setitem__(0, None)),
            ("unknown-key", lambda obj: obj["payload"]["canonical_intents"][0].__setitem__("unexpected", "x")),
            ("wrong-id-type", lambda obj: obj["payload"]["canonical_intents"][0].__setitem__("id", 12)),
        ]
        for name, mutator in cases:
            with self.subTest(case=name):
                fixture = _mutate_fixture(build_st1_pos, mutator)
                found = _payload_violations(fixture)
                _assert_codes(
                    found,
                    "canonical-intent-type",
                    "canonical-intent-empty-collection",
                    "malformed-schema",
                    "required-collection-cardinality",
                    label=name,
                )
                self.assertNotEqual(fixture["digest"], build_st1_pos()["digest"])

    def test_b3_overlay_exception_receipt_graph_mutations_fail(self) -> None:
        def overlay_only(obj: dict) -> None:
            overlay = obj["payload"]["overlays"][0]
            overlay["reason"] = overlay["reason"] + " changed"
            overlay["digest"] = _digest_obj({k: v for k, v in overlay.items() if k != "digest"})

        def substring_scope(obj: dict) -> None:
            overlay = overlay_ok()
            overlay["weakens_required"] = True
            overlay["digest"] = _digest_obj({k: v for k, v in overlay.items() if k != "digest"})
            obj["payload"]["overlays"] = [overlay]
            exc = covering_exception_for(overlay)
            exc["scope"]["project"] = "testing"
            obj["payload"]["exceptions"] = [resign_exception(exc)]
            obj["payload"]["receipts"] = [bound_receipt(obj["payload"]["projections"], payload=obj["payload"])]

        overlay_found = _payload_violations(_mutate_fixture(build_st3_pos, overlay_only))
        _assert_codes(overlay_found, "exception-overlay-digest-mismatch", "bound-digest-mismatch", label="overlay graph")
        scope_found = _payload_violations(_mutate_fixture(build_st3_pos, substring_scope))
        _assert_codes(scope_found, "exception-scope-mismatch", "overlay-weakens-required-without-exception", label="substring")

    def test_b4_semver_lineage_alg_none_and_incomplete_manifest_fail(self) -> None:
        def skip_revision(obj: dict) -> None:
            standard = obj["payload"]["team_standard"]
            standard["revision"] = 3
            _resign_standard(standard)

        def alg_none(obj: dict) -> None:
            obj["payload"]["team_standard"]["signatures"][0]["alg"] = "none"

        def bad_semver(obj: dict) -> None:
            standard = obj["payload"]["team_standard"]
            standard["semantic_version"] = 1
            _resign_standard(standard)

        def incomplete(obj: dict) -> None:
            standard = obj["payload"]["team_standard"]
            standard["content_digest_manifest"] = [standard["content_digest_manifest"][0]]
            _resign_standard(standard)

        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, skip_revision)), "lineage-skip", "rollback-version")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, alg_none)), "invalid-signature-alg", "forged-publisher")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, bad_semver)), "invalid-semver", "nested-standard-type")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, incomplete)), "incomplete-manifest", "swapped-digest")

    def test_b5_empty_and_consistent_false_effective_result_fail(self) -> None:
        def clear_both(obj: dict) -> None:
            obj["payload"]["layer_assignments"] = []
            obj["payload"]["effective_policies"] = []

        def consistent_false(obj: dict) -> None:
            for row in obj["payload"]["layer_assignments"]:
                row["mode"] = "recommended"
                row["rule_id"] = "policy.personal-theme"
            obj["payload"]["effective_policies"] = [
                recompute_effective_row(obj["payload"], family_id) for family_id in REQUIRED_FOUR
            ]

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st5_pos, clear_both)),
            "required-collection-cardinality",
            "required-scenario-coverage",
        )
        _assert_codes(
            _payload_violations(_mutate_fixture(build_st5_pos, consistent_false)),
            "required-scenario-coverage",
            "required-downgraded-to-recommended",
        )

    def test_b6_requester_role_freshness_time_and_alg_none_fail(self) -> None:
        def requester_lead(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["requester"] = "syn-publisher-team-lead"
            events = [entry["event"] for entry in exc["audit_chain"]]
            actors = ["syn-publisher-team-lead" if event == "request" else "syn-approver-lead" for event in events]
            times = [entry["at"] for entry in exc["audit_chain"]]
            exc["audit_chain"] = audit_chain(events, actors=actors, times=times)
            resign_exception(exc)

        def arbitrary_freshness(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["freshness"] = "fresh-enough"
            resign_exception(exc)

        def reverse_time(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            exc["timestamps"]["requested_at"] = REPORT_AT
            exc["timestamps"]["issued_at"] = PREVIEW_AT
            exc["timestamps"]["approved_at"] = FROZEN_PAST
            resign_exception(exc)

        def alg_none(obj: dict) -> None:
            obj["payload"]["exceptions"][0]["signature"]["alg"] = "none"

        _assert_codes(_payload_violations(_mutate_fixture(build_st6_pos, requester_lead)), "requester-role-mismatch")
        _assert_codes(_payload_violations(_mutate_fixture(build_st6_pos, arbitrary_freshness)), "invalid-freshness")
        _assert_codes(_payload_violations(_mutate_fixture(build_st6_pos, reverse_time)), "impossible-time-order")
        _assert_codes(_payload_violations(_mutate_fixture(build_st6_pos, alg_none)), "invalid-signature-alg", "forged-exception")

    def test_b7_ack_identity_time_fields_and_payload_fail(self) -> None:
        def ack_identity(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["acknowledgement"]["member_id"] = "syn-approver-lead"

        def ack_time(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["acknowledgement"]["at"] = REPORT_AT

        def fields(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["previewed_fields"] = ["standard_version"]

        def payload(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["report_payload_digest"] = sha256_text("other-payload")

        _assert_codes(_payload_violations(_mutate_fixture(build_st7_pos, ack_identity)), "disclosure-digest-mismatch", "requester-role-mismatch")
        _assert_codes(_payload_violations(_mutate_fixture(build_st7_pos, ack_time)), "disclosure-digest-mismatch")
        _assert_codes(_payload_violations(_mutate_fixture(build_st7_pos, fields)), "empty-preview", "disclosure-missing")
        _assert_codes(_payload_violations(_mutate_fixture(build_st7_pos, payload)), "disclosure-payload-mismatch")

    def test_b8_lifecycle_null_steps_arbitrary_digest_and_untrusted_key_fail(self) -> None:
        def null_lifecycle(obj: dict) -> None:
            obj["payload"]["lifecycle"] = None

        def empty_steps(obj: dict) -> None:
            obj["payload"]["lifecycle"]["steps"] = []

        def arbitrary_digest(obj: dict) -> None:
            fake = sha256_text("arbitrary")
            obj["payload"]["lifecycle"]["update"]["digest"] = fake
            obj["payload"]["lifecycle"]["update"]["preview_digest"] = fake

        def untrusted(obj: dict) -> None:
            rotation = obj["payload"]["lifecycle"]["key_rotation"]
            rotation["new_trust_state"] = "unknown"
            unsigned = {key: value for key, value in rotation.items() if key not in {"old_signature", "new_signature"}}
            rotation["old_signature"] = synthetic_sign(unsigned, rotation["old_key_id"])
            rotation["new_signature"] = synthetic_sign(unsigned, rotation["new_key_id"])

        def percent_string(obj: dict) -> None:
            obj["payload"]["lifecycle"]["staged_rollout"]["percent"] = "10"
            obj["payload"]["lifecycle"]["staged_rollout"]["stages"][0]["percent"] = "10"

        _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, null_lifecycle)), "missing-lifecycle-group", "required-collection-cardinality")
        _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, empty_steps)), "lifecycle-step-order", "missing-lifecycle-group")
        _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, arbitrary_digest)), "update-preview-digest-mismatch")
        _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, untrusted)), "untrusted-rotation-key", "invalid-key-rotation")
        _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, percent_string)), "incomplete-rollout")
        for group in LIFECYCLE_REQUIRED_GROUPS:
            with self.subTest(group=group):
                def mutate(obj: dict, name=group) -> None:
                    obj["payload"]["lifecycle"].pop(name, None)

                found = _payload_violations(_mutate_fixture(build_st8_pos, mutate))
                self.assertTrue(found, msg=f"deleting {group} accepted")


class SemanticTeamAuthoritativeBinding(unittest.TestCase):
    """Review-4 B2-B8: false accepts reachable from the official builder output."""

    def _rebuild_exception(self, obj, state, times, actors):
        exc = obj["payload"]["exceptions"][0]
        exc["state"] = state
        exc["audit_chain"] = audit_chain(AUDIT_EVENTS_BY_STATE[state], actors=actors, times=times)
        obj["payload"]["exceptions"][0] = resign_exception(exc)

    # --- B2: capability negotiation and native target set ---------------------
    def test_b2_arbitrary_negotiation_status_fails_for_every_family(self) -> None:
        for family_id in family_ids():
            with self.subTest(family_id=family_id):
                def mutate(obj: dict, fid=family_id) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                    proj["capability_negotiation"]["status"] = "totally-made-up"

                found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
                _assert_codes(found, "malformed-schema", "silent-capability-loss", label=f"{family_id}.status")

    def test_b2_negotiation_may_not_claim_more_than_the_registry(self) -> None:
        for family_id in family_ids():
            registry = registry_capability_status(family_id, "instructions")
            if registry == "required-supported":
                continue
            with self.subTest(family_id=family_id, registry=registry):
                def mutate(obj: dict, fid=family_id) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                    proj["capability_negotiation"]["status"] = "required-supported"

                found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
                _assert_codes(found, "family-native-mismatch", "silent-capability-loss", label=family_id)

    def test_b2_honest_unknown_may_not_be_hidden_as_supported(self) -> None:
        def mutate(obj: dict) -> None:
            proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "deepseek-harness")
            proj["capability_negotiation"]["status"] = "required-supported"
            proj["capability_negotiation"]["unknown"] = []

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
            "silent-capability-loss",
            "family-native-mismatch",
        )

    def test_b2_unknown_required_capability_fails(self) -> None:
        def mutate(obj: dict) -> None:
            proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "codex")
            proj["capability_negotiation"]["required_capability"] = "not-a-capability"

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
            "malformed-schema",
            "silent-capability-loss",
        )

    def test_b2_dropping_an_authoritative_native_path_fails(self) -> None:
        for family_id in family_ids():
            paths = family_required_paths(family_id)
            if len(paths) < 2:
                continue
            with self.subTest(family_id=family_id):
                def mutate(obj: dict, fid=family_id, drop=paths[-1]) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                    proj["native_files"] = [item for item in proj["native_files"] if item["path"] != drop]
                    proj["native_digest"] = native_bundle_digest(proj["native_files"])

                found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
                _assert_codes(found, "family-native-incomplete", "family-native-mismatch", label=family_id)

    def test_b2_cross_family_native_content_swap_fails(self) -> None:
        def mutate(obj: dict) -> None:
            codex = next(item for item in obj["payload"]["projections"] if item["family_id"] == "codex")
            codex["native_files"] = copy.deepcopy(family_native_content("grok-build"))
            codex["native_digest"] = native_bundle_digest(codex["native_files"])

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
            "family-native-incomplete",
            "family-native-mismatch",
        )

    def test_b2_foreign_native_body_fails_for_every_family(self) -> None:
        for family_id in family_ids():
            with self.subTest(family_id=family_id):
                def mutate(obj: dict, fid=family_id) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                    body = "# unrelated body\nJest is the runner.\n"
                    proj["native_files"][0]["content"] = body
                    proj["native_files"][0]["digest_sha256"] = sha256_text(body)
                    proj["native_digest"] = native_bundle_digest(proj["native_files"])

                found = _payload_violations(_mutate_fixture(build_st2_pos, mutate))
                _assert_codes(found, "family-native-incomplete", "family-native-mismatch", label=family_id)

    # --- B3: overlay / receipt references and transaction binding -------------
    def test_b3_receipt_state_is_a_closed_enum(self) -> None:
        for bad in ("whatever", None, 1, "Verified"):
            with self.subTest(value=bad):
                def mutate(obj: dict, value=bad) -> None:
                    obj["payload"]["receipts"][0]["reconciliation_state"] = value

                found = _payload_violations(_mutate_fixture(build_st3_pos, mutate))
                _assert_codes(found, "malformed-schema", "projection-outcome-is-reconciliation-state", label=str(bad))

    def test_b3_overlay_references_must_resolve(self) -> None:
        cases = (
            ("intent", lambda o: o["payload"]["overlays"][0].__setitem__("intent_id", "intent.does-not-exist"),
             ("exception-intent-mismatch", "overlay-metadata-missing")),
            ("policy", lambda o: o["payload"]["overlays"][0].__setitem__("policy_id", "policy.does-not-exist"),
             ("exception-policy-mismatch", "overlay-metadata-missing")),
            ("family", lambda o: o["payload"]["overlays"][0]["scope"].__setitem__("family_id", "not-a-harness"),
             ("family-native-mismatch", "overlay-metadata-missing")),
            ("path", lambda o: o["payload"]["overlays"][0]["scope"].__setitem__("path", "random/file.md"),
             ("family-native-mismatch", "overlay-metadata-missing")),
            ("surface", lambda o: o["payload"]["overlays"][0]["scope"].__setitem__("surface", "not-a-surface"),
             ("family-native-mismatch", "overlay-metadata-missing")),
            ("outcome", lambda o: o["payload"]["overlays"][0].__setitem__("projection_outcome", "whatever"),
             ("malformed-schema", "overlay-metadata-missing")),
        )
        for label, mutator, codes in cases:
            with self.subTest(case=label):
                _assert_codes(_payload_violations(_mutate_fixture(build_st3_pos, mutator)), *codes, label=label)

    def test_b3_overlay_native_files_must_be_a_bijection(self) -> None:
        def mutate(obj: dict) -> None:
            files = obj["payload"]["overlays"][0]["native_files"]
            files.append(copy.deepcopy(files[0]))

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st3_pos, mutate)),
            "overlay-metadata-missing",
            "overlay-digest-mismatch",
        )

    def test_b3_transaction_digest_binds_changed_lifecycle_content(self) -> None:
        cases = (
            ("apply_at", lambda o: o["payload"]["lifecycle"]["update"].__setitem__("apply_at", "2026-09-04T23:00:00+08:00")),
            ("rollback_revision", lambda o: o["payload"]["lifecycle"]["rollback"].__setitem__("target_revision", 99)),
            ("rollback_authorized", lambda o: o["payload"]["lifecycle"]["rollback"].__setitem__("authorized", False)),
        )
        for label, mutator in cases:
            with self.subTest(case=label):
                _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, mutator)), "bound-digest-mismatch", label=label)

    # --- B4: Team Standard typing, uniqueness, lineage, trust -----------------
    def test_b4_signature_block_must_be_an_array(self) -> None:
        for bad in ({"key_id": "syn-publisher-team-lead"}, "signed", 1):
            with self.subTest(value=type(bad).__name__):
                def mutate(obj: dict, value=bad) -> None:
                    obj["payload"]["team_standard"]["signatures"] = value

                found = _payload_violations(_mutate_fixture(build_st4_pos, mutate))
                _assert_codes(found, "unsigned-standard", "missing-signature", label=str(bad))

    def test_b4_duplicate_manifest_paths_and_coordinates_fail(self) -> None:
        def dup_manifest(obj: dict) -> None:
            manifest = obj["payload"]["team_standard"]["content_digest_manifest"]
            manifest.append(copy.deepcopy(manifest[0]))

        def dup_coordinate(obj: dict) -> None:
            coords = obj["payload"]["team_standard"]["target_harness_coordinates"]
            coords.append(coords[0])

        def unknown_coordinate(obj: dict) -> None:
            obj["payload"]["team_standard"]["target_harness_coordinates"].append("ghost/9.9.9/cli/macos-27-arm64")

        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, dup_manifest)), "incomplete-manifest", "swapped-digest")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, dup_coordinate)), "incomplete-manifest", "nested-standard-type")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, unknown_coordinate)), "family-native-mismatch", "nested-standard-type")

    def test_b4_lineage_replay_fails(self) -> None:
        def repeat_revision(obj: dict) -> None:
            history = obj["payload"]["team_standard"]["lineage"]["history"]
            history.append(copy.deepcopy(history[0]))

        def current_in_history(obj: dict) -> None:
            lineage = obj["payload"]["team_standard"]["lineage"]
            lineage["history"].append({"revision": 1, "digest_sha256": lineage["previous_digest"]})

        def digest_swap(obj: dict) -> None:
            obj["payload"]["team_standard"]["lineage"]["history"][0]["digest_sha256"] = "0" * 64

        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, repeat_revision)), "lineage-skip", "rollback-version")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, current_in_history)), "lineage-skip", "rollback-version")
        _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, digest_swap)), "swapped-digest", "lineage-skip")

    def test_b4_revoked_or_expired_trust_fails(self) -> None:
        for state, codes in (
            ("revoked", ("revoked-signing-key", "revoked-key-still-trusted")),
            ("expired", ("expired-signing-key",)),
            ("unknown", ("unknown-signing-key",)),
        ):
            with self.subTest(state=state):
                def mutate(obj: dict, value=state) -> None:
                    obj["payload"]["team_standard"]["trust"]["state"] = value

                _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, mutate)), *codes, label=state)

    def test_b4_policy_rule_typing_and_uniqueness(self) -> None:
        cases = (
            ("mode", lambda o: o["payload"]["team_standard"]["policy_rules"][0].__setitem__("mode", "whatever"),
             ("nested-standard-type", "required-downgraded-to-recommended")),
            ("honesty", lambda o: o["payload"]["team_standard"]["policy_rules"][0].__setitem__("honesty", "whatever"),
             ("nested-standard-type", "false-enforcement-claim")),
            ("layer", lambda o: o["payload"]["team_standard"]["policy_rules"][0].__setitem__("layer", "whatever"),
             ("nested-standard-type",)),
            ("intents-type", lambda o: o["payload"]["team_standard"]["policy_rules"][0].__setitem__("intent_ids", "intent.x"),
             ("nested-standard-type",)),
            ("foreign-intent", lambda o: o["payload"]["team_standard"]["policy_rules"][0].__setitem__("intent_ids", ["intent.not-declared"]),
             ("exception-intent-mismatch", "nested-standard-type")),
            ("duplicate-id", lambda o: o["payload"]["team_standard"]["policy_rules"].append(
                copy.deepcopy(o["payload"]["team_standard"]["policy_rules"][0])),
             ("incomplete-manifest", "nested-standard-type")),
        )
        for label, mutator, codes in cases:
            with self.subTest(case=label):
                _assert_codes(_payload_violations(_mutate_fixture(build_st4_pos, mutator)), *codes, label=label)

    # --- B5: effective enforcement must consume negotiation and exceptions ----
    def test_b5_unknown_capability_cannot_stay_enforceable(self) -> None:
        def mutate(obj: dict) -> None:
            proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "codex")
            proj["capability_negotiation"]["status"] = "unknown"
            proj["capability_negotiation"]["unknown"] = ["model-visible-complete-prompt"]

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st5_pos, mutate)),
            "false-enforcement-claim",
            "effective-policy-mismatch",
        )

    def test_b5_dropped_capability_cannot_stay_enforceable(self) -> None:
        def mutate(obj: dict) -> None:
            proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "cursor")
            proj["capability_negotiation"]["dropped"] = ["rules"]

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st5_pos, mutate)),
            "false-enforcement-claim",
            "effective-policy-mismatch",
        )

    def test_b5_missing_projection_cannot_stay_enforceable(self) -> None:
        def mutate(obj: dict) -> None:
            obj["payload"]["projections"] = [
                item for item in obj["payload"]["projections"] if item["family_id"] != "codex"
            ]

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st5_pos, mutate)),
            "unmanaged-marked-enforceable",
            "false-enforcement-claim",
        )

    def test_b5_covering_exception_suspends_the_enforceable_claim(self) -> None:
        def mutate(obj: dict) -> None:
            exc = valid_exception()
            exc["scope"]["harness"] = "codex"
            obj["payload"]["exceptions"] = [resign_exception(exc)]

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st5_pos, mutate)),
            "false-enforcement-claim",
            "effective-policy-mismatch",
        )

    # --- B6: policy evaluation schema and exception state binding -------------
    def test_b6_policy_eval_schema_is_closed(self) -> None:
        cases = (
            ("result", lambda o: o["payload"]["policy_evals"][0].__setitem__("result", "whatever"),
             ("malformed-schema", "false-enforcement-claim")),
            ("mode", lambda o: o["payload"]["policy_evals"][0].__setitem__("mode", "whatever"),
             ("malformed-schema", "required-downgraded-to-recommended")),
            ("policy", lambda o: o["payload"]["policy_evals"][0].__setitem__("policy_id", "policy.nope"),
             ("exception-policy-mismatch", "malformed-schema")),
            ("exception", lambda o: o["payload"]["policy_evals"][0].__setitem__("covered_by_exception", "exc.nope"),
             ("exception-scope-mismatch", "malformed-schema")),
            ("uncovered", lambda o: o["payload"]["policy_evals"][0].__setitem__("covered_by_exception", None),
             ("false-enforcement-claim", "required-scenario-coverage")),
            ("export-flag", lambda o: o["payload"]["policy_evals"][0].__setitem__("inspection_export_allowed", "yes"),
             ("malformed-schema",)),
            ("extra-field", lambda o: o["payload"]["policy_evals"][0].__setitem__("unexpected", 1),
             ("malformed-schema",)),
        )
        for label, mutator, codes in cases:
            with self.subTest(case=label):
                _assert_codes(_payload_violations(_mutate_fixture(build_st6_pos, mutator)), *codes, label=label)

    def test_b6_unapproved_exception_cannot_license_a_required_pass(self) -> None:
        for state, times, actors in (
            ("request", [FROZEN_REQUESTED], ["syn-member"]),
            ("rejection", [FROZEN_REQUESTED, EVALUATION_TIME], ["syn-member", "syn-approver-lead"]),
        ):
            with self.subTest(state=state):
                def mutate(obj: dict, s=state, t=times, a=actors) -> None:
                    self._rebuild_exception(obj, s, t, a)

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st6_pos, mutate)),
                    "unbounded-exception",
                    "false-enforcement-claim",
                    label=state,
                )

    def test_b6_audit_entries_bind_to_exception_timestamps(self) -> None:
        def mutate(obj: dict) -> None:
            exc = obj["payload"]["exceptions"][0]
            chain = copy.deepcopy(exc["audit_chain"])
            chain[1]["at"] = "2026-09-04T12:30:00+08:00"
            chain[1]["digest"] = _digest_obj({k: v for k, v in chain[1].items() if k != "digest"})
            exc["audit_chain"] = chain
            obj["payload"]["exceptions"][0] = resign_exception(exc)

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st6_pos, mutate)),
            "audit-event-state-mismatch",
            "impossible-time-order",
        )

    # --- B7: consent and preview typing bound to the standard ----------------
    def test_b7_upload_consent_must_be_boolean(self) -> None:
        for bad in (None, "", "yes", 0, []):
            with self.subTest(value=repr(bad)):
                def mutate(obj: dict, value=bad) -> None:
                    obj["payload"]["member_disclosure"]["upload_consented"] = value

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st7_pos, mutate)),
                    "disclosure-missing",
                    "malformed-schema",
                    label=repr(bad),
                )

    def test_b7_preview_flags_must_actually_be_previewed(self) -> None:
        for key in ("per_harness_projection", "loss_unknown", "exception_metadata"):
            with self.subTest(field=key):
                def mutate(obj: dict, field=key) -> None:
                    obj["payload"]["member_disclosure"]["preview"][field] = False

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st7_pos, mutate)),
                    "empty-preview",
                    "disclosure-missing",
                    label=key,
                )

    def test_b7_preview_version_binds_to_standard_and_leader_view(self) -> None:
        def leader_drift(obj: dict) -> None:
            obj["payload"]["leader_view"]["standard_version"] = "9.9.9"

        def standard_drift(obj: dict) -> None:
            obj["payload"]["member_disclosure"]["preview"]["standard_version"] = "2.0.0"

        _assert_codes(_payload_violations(_mutate_fixture(build_st7_pos, leader_drift)), "disclosure-payload-mismatch")
        _assert_codes(
            _payload_violations(_mutate_fixture(build_st8_pos, standard_drift)),
            "disclosure-payload-mismatch",
            "disclosure-digest-mismatch",
        )

    # --- B8: lifecycle typing and content-derived update digest --------------
    def test_b8_lifecycle_steps_reject_null_and_unrelated_values(self) -> None:
        def null_all(obj: dict) -> None:
            for step in obj["payload"]["lifecycle"]["steps"]:
                step["actor"] = None
                step["preview"] = None
                step["disclosure"] = None

        cases = (
            ("all-null", null_all, ("lifecycle-step-order", "unregistered-identity")),
            ("actor", lambda o: o["payload"]["lifecycle"]["steps"][0].__setitem__("actor", "nobody"),
             ("lifecycle-step-order", "unregistered-identity")),
            ("preview", lambda o: o["payload"]["lifecycle"]["steps"][2].__setitem__("preview", None),
             ("update-without-preview", "lifecycle-step-order")),
            ("disclosure", lambda o: o["payload"]["lifecycle"]["steps"][3].__setitem__("disclosure", "yes"),
             ("update-without-disclosure", "lifecycle-step-order")),
            ("state", lambda o: o["payload"]["lifecycle"]["steps"][0].__setitem__("state", "made-up"),
             ("lifecycle-step-order", "malformed-schema")),
            ("drift", lambda o: o["payload"]["lifecycle"]["steps"][4].__setitem__("drift_class", "made-up"),
             ("lifecycle-step-order", "malformed-schema")),
        )
        for label, mutator, codes in cases:
            with self.subTest(case=label):
                _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, mutator)), *codes, label=label)

    def test_b8_update_digest_is_derived_from_delivered_content(self) -> None:
        cases = (
            ("policy-text", lambda o: o["payload"]["team_standard"]["policy_rules"][0].__setitem__("text", "Jest is fine now")),
            ("manifest", lambda o: o["payload"]["team_standard"]["content_digest_manifest"][0].__setitem__("digest_sha256", "0" * 64)),
            ("preview", lambda o: o["payload"]["member_disclosure"]["preview"].__setitem__("standard_version", "1.0.1")),
            ("receipt", lambda o: o["payload"]["receipts"][0].__setitem__("bound_digest_manifest", "0" * 64)),
        )
        for label, mutator in cases:
            with self.subTest(case=label):
                found = _payload_violations(_mutate_fixture(build_st8_pos, mutator))
                self.assertTrue(found, msg=f"{label} accepted")

    def test_b8_rollout_conflict_lkg_and_rotation_relations(self) -> None:
        cases = (
            ("cohort", lambda o: o["payload"]["lifecycle"]["staged_rollout"].__setitem__("cohort", "ghost"),
             ("incomplete-rollout",)),
            ("order", lambda o: o["payload"]["lifecycle"]["staged_rollout"].__setitem__("order", ["ga", "canary"]),
             ("incomplete-rollout",)),
            ("percent", lambda o: o["payload"]["lifecycle"]["staged_rollout"].__setitem__("percent", 55),
             ("incomplete-rollout",)),
            ("silent-merge-type", lambda o: o["payload"]["lifecycle"]["conflict_resolution"].__setitem__("silent_merge", "no"),
             ("conflict-nondeterministic", "malformed-schema")),
            ("single-head", lambda o: o["payload"]["lifecycle"]["conflict_resolution"].update(
                {"inputs": ["rev-1"], "winner": "rev-1"}),
             ("conflict-nondeterministic",)),
            ("lkg-revision", lambda o: o["payload"]["lifecycle"]["last_known_good"].__setitem__("revision", 7),
             ("last-known-good-missing", "unrelated-rollback-target")),
            ("rotation-same-key", lambda o: o["payload"]["lifecycle"]["key_rotation"].__setitem__(
                "new_key_id", o["payload"]["lifecycle"]["key_rotation"]["old_key_id"]),
             ("invalid-key-rotation", "untrusted-rotation-key")),
            ("rotation-applied-type", lambda o: o["payload"]["lifecycle"]["key_rotation"].__setitem__("applied", "yes"),
             ("invalid-key-rotation", "malformed-schema")),
        )
        for label, mutator, codes in cases:
            with self.subTest(case=label):
                _assert_codes(_payload_violations(_mutate_fixture(build_st8_pos, mutator)), *codes, label=label)


class SemanticTeamC2Review1(unittest.TestCase):
    """c2 review-1 B1-B3: gaps found by the fresh reviewer after repair-1."""

    @staticmethod
    def _rebind(payload) -> None:
        bind_report_payload(payload)
        bind_receipts(payload)
        bind_update(payload)
        bind_disclosure(payload)

    # --- B1: the declared equivalence-basis allowlist must actually be enforced ---
    def test_c2b1_undeclared_equivalence_basis_fails_on_bindings(self) -> None:
        for bad in ("text-hash-equality", "sha256-digest-match", "byte-identical-copy", "", None, 123):
            with self.subTest(value=repr(bad)):
                def mutate(obj: dict, value=bad) -> None:
                    for binding in obj["payload"]["equivalence_bindings"]:
                        binding["equivalence_basis"] = value

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st1_pos, mutate)),
                    "byte-copy-as-alignment",
                    "malformed-schema",
                    label=repr(bad),
                )

    def test_c2b1_undeclared_equivalence_basis_fails_on_projections(self) -> None:
        for bad in ("text-hash-equality", None, 123):
            with self.subTest(value=repr(bad)):
                def mutate(obj: dict, value=bad) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "codex")
                    proj["equivalence_basis"] = value

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
                    "byte-copy-as-alignment",
                    "malformed-schema",
                    label=repr(bad),
                )

    def test_c2b1_declared_bases_still_pass(self) -> None:
        for good in EQUIVALENCE_BASES:
            with self.subTest(value=good):
                def mutate(obj: dict, value=good) -> None:
                    for binding in obj["payload"]["equivalence_bindings"]:
                        binding["equivalence_basis"] = value

                found = _payload_violations(_mutate_fixture(build_st1_pos, mutate))
                self.assertEqual(found, [], msg=f"declared basis {good} rejected: {found}")

    # --- B2: the acknowledgement must cover the upload-consent decision ----------
    def test_c2b2_consent_decision_moves_the_acknowledged_digest(self) -> None:
        fixture = build_st7_pos()
        disclosure = copy.deepcopy(fixture["payload"]["member_disclosure"])
        baseline = disclosure_binding_digest(disclosure, fixture["payload"])
        flipped = copy.deepcopy(disclosure)
        flipped["upload_consented"] = not flipped["upload_consented"]
        self.assertNotEqual(baseline, disclosure_binding_digest(flipped, fixture["payload"]))
        denied = copy.deepcopy(disclosure)
        denied["consent_status"] = "denied"
        self.assertNotEqual(baseline, disclosure_binding_digest(denied, fixture["payload"]))

    def test_c2b2_consent_cannot_be_flipped_after_acknowledgement(self) -> None:
        for scenario, builder in (("ST7-pos", build_st7_pos), ("ST8-pos", build_st8_pos)):
            for consented in (False, True):
                with self.subTest(scenario=scenario, baseline=consented):
                    fixture = copy.deepcopy(builder())
                    fixture["payload"]["member_disclosure"]["upload_consented"] = consented
                    self._rebind(fixture["payload"])
                    self.assertEqual(
                        collect_violations(fixture["payload"], scenario=scenario),
                        [],
                        msg=f"{scenario} consented={consented} baseline rejected",
                    )
                    for label, mutator in (
                        ("flip-consent", lambda d: d.__setitem__("upload_consented", not consented)),
                        ("deny-after-ack", lambda d: d.__setitem__("consent_status", "denied")),
                    ):
                        tampered = copy.deepcopy(fixture)
                        mutator(tampered["payload"]["member_disclosure"])
                        found = collect_violations(tampered["payload"], scenario=scenario)
                        _assert_codes(
                            found,
                            "disclosure-digest-mismatch",
                            "disclosure-missing",
                            label=f"{scenario}.{consented}.{label}",
                        )

    # --- B3: the rollout percent belongs to the declared cohort ------------------
    def test_c2b3_rollout_percent_binds_to_the_declared_cohort(self) -> None:
        mismatches = (("ga", 10), ("canary", 100), ("ga", 1), ("canary", 55))
        for cohort, percent in mismatches:
            with self.subTest(cohort=cohort, percent=percent):
                def mutate(obj: dict, c=cohort, pct=percent) -> None:
                    obj["payload"]["lifecycle"]["staged_rollout"].update({"cohort": c, "percent": pct})

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st8_pos, mutate)),
                    "incomplete-rollout",
                    label=f"{cohort}/{percent}",
                )

    def test_c2b3_declared_cohort_percent_pairs_pass(self) -> None:
        for cohort, percent in (("canary", 10), ("ga", 100)):
            with self.subTest(cohort=cohort, percent=percent):
                def mutate(obj: dict, c=cohort, pct=percent) -> None:
                    obj["payload"]["lifecycle"]["staged_rollout"].update({"cohort": c, "percent": pct})

                found = _payload_violations(_mutate_fixture(build_st8_pos, mutate))
                self.assertEqual(found, [], msg=f"{cohort}/{percent} rejected: {found}")


class SemanticTeamC2Review2(unittest.TestCase):
    """c2 review-2 C2-N1: the negotiated capability must be the one the intent requires."""

    def test_c2n1_negotiated_capability_binds_to_the_canonical_intent(self) -> None:
        for family_id in family_ids():
            with self.subTest(family_id=family_id):
                def mutate(obj: dict, fid=family_id) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == fid)
                    proj["capability_negotiation"]["required_capability"] = "mcp-declarations"

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
                    "silent-capability-loss",
                    "family-native-mismatch",
                    label=family_id,
                )

    def test_c2n1_connector_cannot_reframe_unsupported_as_supported(self) -> None:
        def mutate(obj: dict) -> None:
            proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "coze")
            proj["capability_negotiation"] = {
                "required_capability": "mcp-declarations",
                "status": "required-supported",
                "unsupported": [],
                "unknown": [],
                "dropped": [],
            }
            proj["projection_outcome"] = "transformed"
            proj.pop("reconciliation_state", None)

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
            "silent-capability-loss",
            "family-native-mismatch",
        )

    def test_c2n1_unsupported_list_must_name_the_required_capability(self) -> None:
        for wrong in (["tool-invocation"], ["memory"], ["skills"]):
            with self.subTest(unsupported=wrong):
                def mutate(obj: dict, value=wrong) -> None:
                    proj = next(item for item in obj["payload"]["projections"] if item["family_id"] == "coze")
                    proj["capability_negotiation"]["unsupported"] = list(value)

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st2_pos, mutate)),
                    "silent-capability-loss",
                    "unsupported-marked-verified",
                    label=str(wrong),
                )

    def test_c2n1_honest_degradations_remain_expressible(self) -> None:
        """The binding must not kill the honest Unknown / unsupported narratives."""
        fixture = build_st2_pos()
        self.assertEqual(_payload_violations(fixture), [])
        deepseek = next(
            item for item in fixture["payload"]["projections"] if item["family_id"] == "deepseek-harness"
        )
        self.assertEqual(deepseek["capability_negotiation"]["status"], "unknown")
        self.assertEqual(deepseek["projection_outcome"], "unknown")
        self.assertEqual(deepseek["reconciliation_state"], "indeterminate")
        coze = next(item for item in fixture["payload"]["projections"] if item["family_id"] == "coze")
        self.assertEqual(coze["capability_negotiation"]["status"], "not-applicable")
        self.assertEqual(coze["projection_outcome"], "unsupported")
        self.assertIn("instructions", coze["capability_negotiation"]["unsupported"])


class SemanticTeamCF02Revision(unittest.TestCase):
    """C-F02 (2026-09-12): byte identity is neither proof of semantic equivalence
    nor evidence of non-equivalence. ST2-same-bytes-pos is the minimal
    counterexample to the former 'bodies must differ' clause."""

    def test_same_bytes_projection_passes(self) -> None:
        fixture = build_st2_same_bytes_pos()
        projections = fixture["payload"]["projections"]
        self.assertEqual(
            {item["family_id"] for item in projections},
            {"kimi-code", "zcode"},
        )
        self.assertEqual(
            len({item["native_digest"] for item in projections}),
            1,
            "fixture must project byte-identical bodies",
        )
        self.assertEqual(_payload_violations(fixture), [])
        self.assertEqual(fixture["digest"], _case_digest(fixture))

    def test_same_bytes_binding_byte_identical_true_fails(self) -> None:
        def mutate(obj: dict) -> None:
            for binding in obj["payload"]["equivalence_bindings"]:
                binding["byte_identical"] = True

        _assert_codes(
            _payload_violations(_mutate_fixture(build_st2_same_bytes_pos, mutate)),
            "byte-copy-as-alignment",
            "hash-equality-as-equivalence",
        )

    def test_same_bytes_binding_byte_basis_fails(self) -> None:
        for bad in ("byte-equality", "hash-equality"):
            with self.subTest(basis=bad):
                def mutate(obj: dict, value=bad) -> None:
                    for binding in obj["payload"]["equivalence_bindings"]:
                        binding["equivalence_basis"] = value

                _assert_codes(
                    _payload_violations(_mutate_fixture(build_st2_same_bytes_pos, mutate)),
                    "byte-copy-as-alignment",
                    "hash-equality-as-equivalence",
                    label=bad,
                )


if __name__ == "__main__":
    unittest.main()
