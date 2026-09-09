// Presentation-layer rendering (C6). The app is compiled once with
// `vite build --ssr` (no new dependency: vite, react-dom/server and
// react-router-dom are already here) and rendered with `react-dom/server`.
//
// What this covers: every route renders at first paint with the shell
// landmarks, and every page shows the C04 banner for each state its contract
// declares applicable — and flags a state the contract declares not
// applicable. SSR runs no effects, so data flow, cancellation and interaction
// are not covered here; they remain a separate, unfinished E2E item.
import assert from "node:assert/strict";
import test from "node:test";
import { execFileSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join } from "node:path";

import { PAGE_CONTRACTS, contractFor } from "../src/page-contract.js";
import { ROUTE_PATHS } from "./route-paths.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = join(here, "..");
const outDir = join(here, ".ssr-out");

/** States that render a banner. `loading` and `ok` have no banner label. */
const BANNER_STATES = [
  "empty",
  "error",
  "partial",
  "stale",
  "offline",
  "permission-denied",
  "unsupported-version",
  "connector-missing",
];

/** A reason code the product actually emits for each state. */
const REASON = {
  empty: "api.no_content",
  error: "store.missing",
  partial: "runtime_snapshot_missing",
  stale: "monitor.evidence_changed",
  offline: "api.unreachable",
  "permission-denied": "policy.approval_required",
  "unsupported-version": "unsupported_harness_version",
  "connector-missing": "connector_required",
};

let ssr;
test.before(async () => {
  rmSync(outDir, { recursive: true, force: true });
  execFileSync(
    "pnpm",
    ["exec", "vite", "build", "--ssr", "src/ssr-entry.tsx", "--outDir", "tests/.ssr-out", "--logLevel", "error"],
    { cwd: uiRoot, stdio: "inherit" },
  );
  const entry = join(outDir, "ssr-entry.js");
  assert.ok(existsSync(entry), "vite --ssr did not emit ssr-entry.js");
  ssr = await import(pathToFileURL(entry).href);
});

test("every route renders at first paint with the shell landmarks", () => {
  for (const route of ROUTE_PATHS) {
    const concrete = route.replace(/:[A-Za-z]+/g, "x");
    const html = ssr.renderRoute(concrete);
    assert.match(html, /<main[^>]*id="main-content"/, `${route}: main landmark`);
    assert.match(html, /<nav[^>]*aria-label="primary"/, `${route}: primary nav`);
    assert.match(html, /class="skip-link"/, `${route}: skip link`);
    assert.doesNotMatch(html, /not found|notFound/i, `${route}: rendered the not-found fallback`);
  }
  // The root redirects to Doctor; an unknown route says so.
  assert.match(ssr.renderRoute("/no-such-route"), /\/no-such-route/);
});

test("self-fetching pages start in loading with a cancel control", () => {
  for (const route of ["/receipts", "/sessions", "/monitor", "/lab", "/policy", "/standards", "/exceptions", "/team/compliance"]) {
    const html = ssr.renderRoute(route);
    assert.match(html, /role="status"/, `${route}: loading status`);
    assert.match(html, /<button[^>]*>[^<]*<\/button>/, `${route}: a control for cancel`);
  }
});

test("each page renders the banner for every state its contract declares applicable", () => {
  for (const contract of PAGE_CONTRACTS) {
    for (const state of contract.applicable) {
      if (!BANNER_STATES.includes(state)) continue;
      const html = ssr.renderState(contract.route, state, REASON[state]);
      assert.match(html, new RegExp(`data-state="${state}"`), `${contract.route}/${state}: banner state`);
      assert.match(html, new RegExp(`<code>${REASON[state]}</code>`), `${contract.route}/${state}: reason code shown`);
      assert.doesNotMatch(html, /data-undeclared/, `${contract.route}/${state}: declared applicable, must not be flagged`);
      const failure = state === "error" || state === "offline" || state === "permission-denied";
      assert.match(html, new RegExp(`role="${failure ? "alert" : "status"}"`), `${contract.route}/${state}: role`);
      if (state === "permission-denied") {
        // Retry cannot widen a refused authorization, so it is not offered.
        assert.doesNotMatch(html, /<button/, `${contract.route}/${state}: no retry on a refusal`);
      }
    }
  }
});

test("a state the contract declares not applicable is surfaced as a contradiction, not hidden", () => {
  let checked = 0;
  for (const contract of PAGE_CONTRACTS) {
    for (const state of Object.keys(contract.notApplicable)) {
      if (!BANNER_STATES.includes(state)) continue;
      const html = ssr.renderState(contract.route, state, REASON[state]);
      assert.match(html, /data-undeclared="true"/, `${contract.route}/${state}: undeclared state must be flagged`);
      checked += 1;
    }
  }
  assert.ok(checked > 0, "at least one not-applicable state exists to check");
});

test("detail routes render their list route's contract states", () => {
  const detail = contractFor("/receipts/:id");
  assert.ok(detail, "detail route resolves to a contract");
  const html = ssr.renderState("/receipts/:id", "permission-denied", REASON["permission-denied"]);
  assert.match(html, /data-state="permission-denied"/);
  assert.doesNotMatch(html, /data-undeclared/);
});

// ---- T4(e): the session request-evidence page and T2: the Effect Lab result ----

const SESSION_DOC = {
  schema: "ctxpect-session-v1",
  session_id: "s_abc",
  mapping_id: "deepseek-harness-cli",
  bodies_stored: false,
  timeline: [
    { seq: 0, type: "turn/start", len: 40, digest: "d0".padEnd(64, "0"), surface_op: null },
    { seq: 1, type: "user/message", len: 120, digest: "d1".padEnd(64, "1"), surface_op: "append" },
    { seq: 2, type: "request/header", len: 300, digest: "d2".padEnd(64, "2"), surface_op: null },
    { seq: 3, type: "assistant/message", len: 200, digest: "d3".padEnd(64, "3"), surface_op: "append" },
    { seq: 4, type: "user/message", len: 90, digest: "d4".padEnd(64, "4"), surface_op: "replace" },
  ],
};

const REQUESTS_DOC = {
  session_id: "s_abc",
  partial: true,
  requests: [
    {
      seq: 2,
      reason: "initial",
      header_digest: "abcdef0123456789".padEnd(64, "f"),
      message_count: 1,
      surface_nodes: [1],
      source_seq_ranges: [[1, 1]],
      replaced_ranges: [],
      dispatch_evidence: 3,
      unknown_reasons: [],
    },
    {
      seq: 5,
      reason: "series",
      header_digest: "9999999999999999".padEnd(64, "9"),
      message_count: 2,
      surface_nodes: [4, 3],
      source_seq_ranges: [[3, 4]],
      replaced_ranges: [{ seq: 4, start: 1, end: 1, shadowed: [1] }],
      dispatch_evidence: null,
      unknown_reasons: ["runtime_snapshot_missing: no provider output in this turn/step"],
    },
  ],
  tail: { interrupted: true, closers: [{ type: "tool/result", error_code: "TOOL_OUTCOME_UNKNOWN" }, { type: "turn/end", reason: "interrupted" }] },
};

/** React separates adjacent text nodes with comment markers; drop them for matching. */
const strip = (html) => html.replace(/<!-- -->/g, "");

test("the session page lists request evidence and renders only metadata", () => {
  const html = strip(ssr.renderSessionRequests(SESSION_DOC, REQUESTS_DOC, 1));
  // Every request: header digest (prefix), message count, ranges, dispatch evidence, limits.
  assert.match(html, /abcdef0123456789/);
  assert.match(html, /9999999999999999/);
  assert.match(html, /data-dispatch="seq"[^>]*>seq 3/);
  assert.match(html, /data-dispatch="none"/);
  assert.match(html, /仅已准备/);
  assert.match(html, /runtime_snapshot_missing/);
  assert.match(html, /3–4/);
  assert.match(html, /4: 1–1/);
  // Tail: listed, not appended.
  assert.match(html, /TOOL_OUTCOME_UNKNOWN/);
  assert.match(html, /只列出、未追加/);
  // Two metadata-only views: human-visible history (append-origin only) and
  // the selected request's derived surface (seqs 4 and 3, in surface order).
  const tables = html.match(/data-metadata-only="true"/g) ?? [];
  assert.equal(tables.length, 2);
  assert.match(html, /人类可见历史/);
  assert.match(html, /该次请求派生 surface[^<]*· seq 5/);
  assert.doesNotMatch(html, /<td>2<\/td><td>request\/header/, "a log-only event is not part of the human transcript");
  // No body, no path, nothing beyond type/seq/len/digest.
  assert.match(html, /bodies_stored: false/);
  assert.doesNotMatch(html, /"content"|List the files|\/tmp\//);
  // The reconstruction is flagged as partial.
  assert.match(html, /部分重建/);
});

test("the session page without a selected request still shows the human-visible history", () => {
  const html = ssr.renderSessionRequests(SESSION_DOC, REQUESTS_DOC, null);
  const tables = html.match(/data-metadata-only="true"/g) ?? [];
  assert.equal(tables.length, 1);
});

test("an experiment that was not executed says so and shows no decision", () => {
  const html = strip(ssr.renderLabResult({
    schema: "ctxpect-effect-result-v1",
    experiment_id: "e1",
    executed: false,
    decision: null,
    reason_code: "effect.runs_required",
    estimator: { available: false, reason_code: "effect.estimator_unavailable", note: "no frozen statistical implementation" },
  }));
  assert.match(html, /data-executed="false"/);
  assert.match(html, /未执行/);
  assert.match(html, /<code>effect\.runs_required<\/code>/);
  assert.match(html, /判定: —/);
  assert.doesNotMatch(html, /supported-/);
  const executed = strip(ssr.renderLabResult({ executed: true, decision: "inconclusive", reason_code: "effect.estimator_unavailable", estimator: { available: false } }));
  assert.match(executed, /data-executed="true"/);
  assert.match(executed, /判定: inconclusive/);
});

