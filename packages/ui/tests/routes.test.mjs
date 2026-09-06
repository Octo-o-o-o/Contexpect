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
];

test("all V01-V16 routes are declared", () => {
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
