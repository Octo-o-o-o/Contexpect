import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const routes = readFileSync(join(root, "../src/routes.ts"), "utf8");
const app = readFileSync(join(root, "../src/App.tsx"), "utf8");
const tokens = readFileSync(join(root, "../../ui-tokens/tokens.js"), "utf8");

const expected = [
  "/checkup",
  "/inspector",
  "/compare",
  "/receipts",
  "/assets",
  "/assets/:id",
  "/sessions",
  "/sessions/:id",
  "/monitor",
  "/lab",
  "/lab/:id",
  "/sync",
  "/doctor",
  "/policy",
  "/standards",
  "/standards/:id",
  "/settings",
  "/exceptions",
  "/team/compliance",
  "/care-plan/:findingId",
  "/integrations",
  "/integrations/:id",
  "/advisor",
];

test("all V01-V17 routes are declared", () => {
  for (const path of expected) {
    assert.match(routes, new RegExp(path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    const needle = path.replace(/:findingId|:id/g, "");
    assert.match(app, new RegExp(needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  }
});

test("C03 tokens keep Expected out of truth_state and Unknown out of severity", () => {
  assert.match(tokens, /expectedIsNotTruthState = true/);
  assert.match(tokens, /unknownIsNotSeverity = true/);
  assert.match(tokens, /present/);
  assert.doesNotMatch(tokens, /severity:[\s\S]*unknown:/);
});

test("routes.ts declares exactly the routes App.tsx renders", () => {
  // A route that exists only in App.tsx is invisible to everything driven by
  // routes.ts — including the C04 page contract, which then never checks it.
  const block = routes.slice(routes.indexOf("export const ROUTES"), routes.indexOf("export const NAV"));
  const declared = new Set([...block.matchAll(/path:\s*"([^"]+)"/g)].map((m) => m[1]));
  const rendered = new Set(
    [...app.matchAll(/<Route\s+path="([^"]+)"/g), ...app.matchAll(/<Route\s*\n\s*path="([^"]+)"/g)]
      .map((m) => m[1])
      // `/` is the redirect into the default page, not a page of its own.
      .filter((p) => p !== "/" && p !== "*"),
  );
  const missing = [...rendered].filter((p) => !declared.has(p)).sort();
  const extra = [...declared].filter((p) => !rendered.has(p)).sort();
  assert.deepEqual(missing, [], `rendered but not declared: ${missing.join(", ")}`);
  assert.deepEqual(extra, [], `declared but not rendered: ${extra.join(", ")}`);
});

test("every route's nav key resolves to a translated label", () => {
  // A missing key falls back to the key name, which then shows up in the
  // document title as e.g. "care · Contexpect".
  const block = routes.slice(routes.indexOf("export const ROUTES"), routes.indexOf("export const NAV"));
  const keys = new Set([...block.matchAll(/nav:\s*"([^"]+)"/g)].map((m) => m[1]));
  const tables = readFileSync(join(root, "../src/i18n-tables.js"), "utf8");
  for (const key of keys) {
    assert.match(tables, new RegExp(`\\n\\s*${key}:\\s*"`), `no i18n entry for nav key "${key}"`);
  }
});
