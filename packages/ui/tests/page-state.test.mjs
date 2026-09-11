import assert from "node:assert/strict";
import test from "node:test";

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { STATES, classifyFailure, classifyPayload } from "../src/page-state.js";
import { PAGE_CONTRACTS, contractFor, stateApplies } from "../src/page-contract.js";
import { ROUTE_PATHS } from "./route-paths.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const uiSource = readFileSync(join(root, "../src/App.tsx"), "utf8");
const httpSource = readFileSync(join(root, "../../../crates/ctxpect-cli/src/http.rs"), "utf8");

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

// ---- C04 requires more than states: entry/back, selection, DTO, actions,
// ---- landing points, persistence and sensitive-data boundaries. These
// ---- assertions keep those declarations honest against the real code.


/**
 * The daemon's routing table, parsed from `ROUTE_TABLE` in http.rs: one
 * `(method, path pattern)` per routed endpoint. Both dimensions are exact, so
 * a contract cannot declare a method the daemon does not route for a path.
 */
function backendRoutes() {
  const start = httpSource.indexOf("pub const ROUTE_TABLE");
  const end = httpSource.indexOf("];", start);
  const block = httpSource.slice(start, end);
  return [...block.matchAll(/\(\s*"(GET|POST|PUT|DELETE)"\s*,\s*"(\/api\/v1[^"]*)"\s*\)/g)].map((m) => ({
    method: m[1],
    path: m[2].replace(/:[A-Za-z]+/g, ":x"),
  }));
}

function routedByBackend(declared) {
  const pattern = declared.path.replace(/:[A-Za-z]+/g, ":x");
  return backendRoutes().some((route) => route.path === pattern && route.method === declared.method);
}

test("the routing table is parsed and routes by method as well as path", () => {
  const routes = backendRoutes();
  // Exact: a route rustfmt wrapped onto two lines, or one added without
  // updating this count, would otherwise disappear from the contract check.
  assert.equal(routes.length, 45, `parsed ${routes.length} routes`);
  assert.ok(routedByBackend({ method: "POST", path: "/api/v1/receipts/:id/verify" }));
  assert.ok(routedByBackend({ method: "GET", path: "/api/v1/care-plan/:findingId" }));
  // A path the daemon knows under another method is not routed: the method
  // dimension is checked, not defaulted to `*`.
  assert.equal(routedByBackend({ method: "DELETE", path: "/api/v1/receipts/:id/verify" }), false);
  assert.equal(routedByBackend({ method: "POST", path: "/api/v1/care-plan/:id" }), false);
});

test("every declared query names an endpoint the daemon actually routes", () => {
  for (const contract of PAGE_CONTRACTS) {
    assert.ok(Array.isArray(contract.query) && contract.query.length > 0, contract.route);
    for (const declared of contract.query) {
      assert.ok(
        routedByBackend(declared),
        `${contract.route} declares ${declared.method} ${declared.path}, which the daemon does not route`,
      );
    }
  }
});

test("every page declares entry, back, selection, persistence and sensitivity", () => {
  const validRoutes = new Set(ROUTE_PATHS);
  for (const contract of PAGE_CONTRACTS) {
    for (const field of ["entry", "selection", "persistence", "sensitive"]) {
      const text = contract[field];
      assert.equal(typeof text, "string", `${contract.route}.${field}`);
      // Substantive, not a placeholder.
      assert.ok(text.length >= 10, `${contract.route}.${field} is too thin: ${text}`);
      assert.match(text, /[一-鿿]/, `${contract.route}.${field}`);
    }
    assert.ok(
      validRoutes.has(contract.back),
      `${contract.route} returns to ${contract.back}, which is not a route`,
    );
  }
});

test("every action states its effect and where success lands", () => {
  for (const contract of PAGE_CONTRACTS) {
    assert.ok(Array.isArray(contract.actions), contract.route);
    for (const action of contract.actions) {
      for (const field of ["id", "label", "effect", "lands"]) {
        assert.ok(action[field], `${contract.route}/${action.id ?? "?"}.${field} missing`);
      }
      // A landing point must say what changes, not merely that it succeeded.
      assert.ok(
        action.lands.length >= 8,
        `${contract.route}/${action.id}: landing point is too thin`,
      );
    }
  }
});

test("a page that writes can be refused, and one that cannot write says so", () => {
  for (const contract of PAGE_CONTRACTS) {
    const writes = contract.query.some((q) => q.method !== "GET" && !/inspect|doctor/.test(q.path));
    if (writes) {
      assert.ok(
        contract.applicable.includes("permission-denied"),
        `${contract.route} writes, so a refusal must be a declared state`,
      );
    }
  }
});

test("declared actions correspond to controls that exist in the UI", () => {
  // Guards against a contract promising an action the page never renders.
  const withActions = PAGE_CONTRACTS.filter((c) => c.actions.length > 0).map((c) => c.route);
  assert.deepEqual(withActions.sort(), [
    "/assets",
    "/compare",
    "/doctor",
    "/receipts",
    "/settings",
    "/sync",
  ]);
  for (const marker of [
    "receiptVerify",
    "receiptDelete",
    "syncPreview",
    "syncApply",
    "settingsSave",
    "settingsRevert",
    "assetsPreview",
    "assetsCopy",
    "assetsRollback",
  ]) {
    assert.ok(uiSource.includes(marker), `${marker} is declared but not rendered`);
  }
});

test("the narrow viewport renders a read-only surface, not the full shell", () => {
  // C06: below 768px the product is a read-only Receipt/notification view.
  // Hiding the navigation while still rendering the whole app would leave no
  // way to move between pages.
  assert.match(uiSource, /const NARROW_QUERY = "\(max-width: 767px\)"/);
  assert.match(uiSource, /if \(narrow\) \{/, "App must branch on the narrow viewport");
  assert.match(uiSource, /<NarrowReadOnly/, "the narrow branch renders the read-only view");

  const view = uiSource.slice(
    uiSource.indexOf("function NarrowReadOnly("),
    uiSource.indexOf("/** i18n key for each C04 state's label"),
  );
  assert.ok(view.length > 0, "NarrowReadOnly must exist");
  assert.ok(!view.includes("NAV.map"), "the read-only view does not render the full navigation");
  // Notifications have no endpoint in this slice; the view must not present
  // that as "none yet".
  assert.match(view, /notifications\.unimplemented/);
});

test("narrow detection does not rely on the change event alone", () => {
  // A viewport change that delivers no `change` event would otherwise strand
  // the user in the wrong layout until a reload.
  const hook = uiSource.slice(
    uiSource.indexOf("function useNarrowViewport"),
    uiSource.indexOf("function NarrowReadOnly("),
  );
  assert.match(hook, /addEventListener\("change", sync\)/);
  assert.match(hook, /addEventListener\("resize", sync\)/);
  assert.match(hook, /removeEventListener\("resize", sync\)/, "the resize listener is cleaned up");
});
