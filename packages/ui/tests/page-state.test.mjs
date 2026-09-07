import assert from "node:assert/strict";
import test from "node:test";

import { STATES, classifyFailure, classifyPayload } from "../src/page-state.js";
import { PAGE_CONTRACTS, contractFor, stateApplies } from "../src/page-contract.js";
import { ROUTE_PATHS } from "./route-paths.mjs";

/** Every state C04 names, excluding the internal `ok`. */
const C04_STATES = STATES.filter((state) => state !== "ok");

test("a transport failure is offline, not a generic error", () => {
  const verdict = classifyFailure("transport", "");
  assert.equal(verdict.state, "offline");
  assert.equal(verdict.retryable, true);
});

test("refusals are permission-denied and faults are error", () => {
  for (const code of [
    "policy.approval_required",
    "policy.denied",
    "principal.secret_mismatch",
    "exception.approver_role_required",
    "api.identity_required",
    "advisor.consent_required",
  ]) {
    assert.equal(classifyFailure("envelope", code).state, "permission-denied", code);
  }
  for (const code of ["api.not_found", "store.missing", "usage.invalid", "io.unresolvable"]) {
    assert.equal(classifyFailure("envelope", code).state, "error", code);
  }
});

test("retry is not offered for a refusal, because it cannot widen authorization", () => {
  assert.equal(classifyFailure("envelope", "policy.denied").retryable, false);
  assert.equal(classifyFailure("envelope", "store.missing").retryable, true);
});

test("staleness in the payload beats every other content state", () => {
  const monitorStale = {
    staleness: { status: "stale", reason_code: "monitor.evidence_changed" },
    unknown: 5,
  };
  const verdict = classifyPayload(monitorStale);
  assert.equal(verdict.state, "stale");
  assert.equal(verdict.reasonCode, "monitor.evidence_changed");

  assert.equal(classifyPayload({ stale: true, receipt: { a: 1 } }).state, "stale");
});

test("unsupported version and missing connector come from real reason codes", () => {
  // These are payload reason codes the catalog emits, not HTTP error codes.
  const catalog = {
    families: [{ family_id: "x", installation: { reason_code: "unsupported_harness_version" } }],
  };
  assert.equal(classifyPayload(catalog).state, "unsupported-version");

  const connector = { families: [{ family_id: "y", connector: "connector_required" }] };
  assert.equal(classifyPayload(connector).state, "connector-missing");
});

test("empty is empty, and envelope bookkeeping is not content", () => {
  assert.equal(classifyPayload(null).state, "empty");
  assert.equal(classifyPayload({}).state, "empty");
  assert.equal(classifyPayload({ receipts: [] }).state, "empty");
  // Envelope fields alone do not make a page non-empty.
  assert.equal(
    classifyPayload({ schema: "x", schema_version: 1, command: "c", exit_code: 0 }).state,
    "empty",
  );
  assert.equal(classifyPayload({ receipts: ["r_1"] }).state, "ok");
});

test("unknown cells make an answer partial rather than ok", () => {
  assert.equal(classifyPayload({ counts: { unknown: 5, confirmed: 0 } }).state, "partial");
  assert.equal(classifyPayload({ unknown: 3, standard_status: { total: 1 } }).state, "partial");
  assert.equal(
    classifyPayload({ staleness: { status: "unknown", reason_code: "api.no_current_receipt" } })
      .state,
    "partial",
  );
  assert.equal(
    classifyPayload({ facets: [{ reason_code: "runtime_snapshot_missing" }] }).state,
    "partial",
  );
  // A clean answer stays ok.
  assert.equal(classifyPayload({ counts: { unknown: 0, confirmed: 1 } }).state, "ok");
});

test("every V01-V16 route has a contract covering all C04 states", () => {
  for (const route of ROUTE_PATHS) {
    const contract = contractFor(route);
    assert.ok(contract, `no contract for ${route}`);
    for (const state of C04_STATES) {
      const declared =
        contract.applicable.includes(state) ||
        Object.prototype.hasOwnProperty.call(contract.notApplicable, state);
      assert.ok(declared, `${route} does not declare ${state}`);
    }
  }
});

test("every not-applicable state carries a substantive reason", () => {
  for (const contract of PAGE_CONTRACTS) {
    for (const [state, reason] of Object.entries(contract.notApplicable)) {
      assert.ok(C04_STATES.includes(state), `${contract.route}: ${state} is not a C04 state`);
      assert.ok(
        !contract.applicable.includes(state),
        `${contract.route}: ${state} is both applicable and not`,
      );
      // A reason must explain, not restate. Reject "不适用"-style placeholders.
      assert.ok(reason.length >= 12, `${contract.route}/${state}: reason too thin`);
      assert.match(reason, /[一-鿿]/, `${contract.route}/${state}`);
      assert.ok(
        !/^不适用/.test(reason),
        `${contract.route}/${state}: "不适用" restates the verdict instead of giving a reason`,
      );
    }
  }
});

test("loading, ok, empty, error and offline are applicable everywhere", () => {
  // Every page fetches, so none of these can be declared away.
  for (const contract of PAGE_CONTRACTS) {
    for (const state of ["loading", "ok", "empty", "error", "offline"]) {
      assert.ok(
        contract.applicable.includes(state),
        `${contract.route} must be able to reach ${state}`,
      );
    }
  }
});

test("stateApplies resolves detail routes through their list route", () => {
  assert.equal(stateApplies("/assets/:id", "connector-missing"), true);
  assert.equal(stateApplies("/sessions/:id", "connector-missing"), false);
});
