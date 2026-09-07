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

test("collapsed navigation keeps an accessible name", () => {
  // Below 1024px the nav labels collapse. Removing them from the
  // accessibility tree leaves every link unnamed for screen readers
  // (WCAG 2.2 SC 2.4.4 and 4.1.2), which is what `display: none` did.
  const css = readFileSync(join(root, "../src/app.css"), "utf8");
  const collapsed = css.slice(css.indexOf("max-width: 1023px"), css.indexOf("max-width: 767px"));
  assert.ok(
    !/\.nav a span\s*\{[^}]*display:\s*none/.test(collapsed),
    "nav labels must not be display:none — that also hides them from assistive tech",
  );
  assert.match(collapsed, /clip:\s*rect/, "labels should be visually hidden, not removed");

  // The explicit label does not depend on how a given AT treats
  // visually-hidden text.
  assert.match(
    app,
    /aria-label=\{t\(locale, item\.key\)\}/,
    "nav links need an explicit accessible name",
  );
});

test("long content scrolls inside its block instead of widening the page", () => {
  // A longer translation once widened the whole layout: `white-space: pre`
  // with visible overflow let one long JSON line push the page out, and the
  // grid item grew to fit it so nothing ever scrolled.
  const css = readFileSync(join(root, "../src/app.css"), "utf8");
  const mono = css.slice(css.indexOf(".mono {"), css.indexOf(".mono {") + 200);
  assert.match(mono, /overflow-x:\s*auto/, "long lines must scroll inside .mono");

  // `min-width: 0` / `minmax(0, 1fr)` is what stops a grid item from growing
  // to its content; without it the overflow rule above never takes effect.
  const panel = css.slice(css.indexOf(".panel {"), css.indexOf(".panel {") + 260);
  assert.match(panel, /min-width:\s*0/, "the panel must not grow to fit its content");
  assert.match(css, /\.shell \{[^}]*minmax\(0, 1fr\)/, "shell column must be able to shrink");
  assert.match(css, /\.workspace \{[^}]*minmax\(0, 1fr\)/, "workspace column must be able to shrink");
});

test("landmarks and a bypass mechanism exist (WCAG 2.2 SC 2.4.1, 1.3.1)", () => {
  // Sixteen nav links precede the content on every page. Without a skip
  // link, reaching the content by keyboard means tabbing past all of them.
  assert.match(app, /className="skip-link" href="#main-content"/);
  assert.match(app, /<main className="main" id="main-content" tabIndex=\{-1\}>/);
  // `tabIndex={-1}` is what lets the skip link move focus to the landmark
  // without adding it to the tab order.
  assert.match(app, /<main className="narrow" id="main-content"/, "the narrow view needs it too");
  assert.match(app, /<nav className="nav" aria-label="primary">/);

  const css = readFileSync(join(root, "../src/app.css"), "utf8");
  const skip = css.slice(css.indexOf(".skip-link"), css.indexOf(".skip-link") + 400);
  // Hiding it with `display: none` would remove it from the tab order and
  // defeat the point.
  assert.ok(!/\.skip-link \{[^}]*display:\s*none/.test(skip), "skip link must stay focusable");
  assert.match(skip, /\.skip-link:focus/, "it must become visible when focused");
});

test("the findings table carries its own accessible name", () => {
  // A nearby heading is not attached to the table.
  assert.match(app, /<caption className="sr-only">\{t\(locale, "findingsTableCaption"\)\}<\/caption>/);
});
