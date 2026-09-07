import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { projectFieldValue, projectVisible, revealed } from "../src/mask.js";

const root = dirname(fileURLToPath(import.meta.url));
const app = readFileSync(join(root, "../src/App.tsx"), "utf8");

test("screenshot privacy is a hard mask that hold cannot lift", () => {
  assert.equal(revealed(true, "default"), true);
  assert.equal(revealed(false, "default"), false);
  assert.equal(revealed(true, "screenshot"), false);
  assert.equal(revealed(false, "screenshot"), false);
});

test("the project path is never visible under screenshot privacy", () => {
  assert.equal(projectVisible("default", false), true);
  assert.equal(projectVisible("default", true), true);
  // No combination of inputs reveals it in screenshot mode.
  assert.equal(projectVisible("screenshot", false), false);
  assert.equal(projectVisible("screenshot", true), false);
});

test("a masked project field renders mask characters, never the path", () => {
  const real = "/Users/someone/secret-project";
  assert.equal(projectFieldValue(real, true), real);
  assert.equal(projectFieldValue(real, false), "••••");
  // An empty project stays empty so the placeholder shows.
  assert.equal(projectFieldValue("", false), "");
});

test("focus is not a reveal path in App.tsx", () => {
  // The earlier bypass was `showProject = ... || projectFocused`, which put
  // the real path back into the DOM and the aria-label on focus.
  assert.ok(!app.includes("projectFocused"), "focus must not gate the mask");
  assert.ok(
    app.includes("projectVisible(privacy, hold)"),
    "App must use the shared masking policy",
  );
});

test("the masked project field is read-only and not aria-hidden", () => {
  // aria-hidden on a focusable element hides it from assistive tech while
  // leaving it in the tab order.
  const start = app.indexOf("<input");
  const input = app.slice(start, app.indexOf('placeholder="<project>"', start));
  // Match the attribute, not the word: the source comment names it too.
  assert.ok(!/aria-hidden[=\s]/.test(input), "no aria-hidden on the input");
  assert.ok(input.includes("readOnly={!showProject}"), "masked field is read-only");
  assert.ok(input.includes("aria-label"), "masked state is announced by label");
});
