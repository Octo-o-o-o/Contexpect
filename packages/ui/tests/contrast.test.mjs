import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const tokensCss = readFileSync(join(root, "../../ui-tokens/tokens.css"), "utf8");
const appCss = readFileSync(join(root, "../src/app.css"), "utf8");

/** Token values read from the stylesheet, so the test cannot drift from it. */
function tokens() {
  const out = {};
  for (const match of tokensCss.matchAll(/--([a-z-]+):\s*(#[0-9A-Fa-f]{6})/g)) {
    out[match[1]] = match[2];
  }
  return out;
}

function channelToLinear(value) {
  const c = value / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function relativeLuminance(hex) {
  const h = hex.replace("#", "");
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
  return (
    0.2126 * channelToLinear(r) + 0.7152 * channelToLinear(g) + 0.0722 * channelToLinear(b)
  );
}

/** WCAG 2.x contrast ratio. */
export function contrast(fg, bg) {
  const a = relativeLuminance(fg);
  const b = relativeLuminance(bg);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

test("the contrast helper matches the WCAG reference values", () => {
  // Black on white is exactly 21:1; any colour against itself is 1:1.
  assert.equal(Math.round(contrast("#000000", "#FFFFFF") * 100) / 100, 21);
  assert.equal(contrast("#123456", "#123456"), 1);
});

test("text meets WCAG 2.2 AA (SC 1.4.3) against the ground it is drawn on", () => {
  const T = tokens();
  const literals = { navLink: "#c9c6bb", navActiveBg: "#1c2533" };
  // Only combinations that actually occur in the rendered UI.
  const pairs = [
    ["body text on canvas", T["text-primary"], T["bg-canvas"]],
    ["body text on surface", T["text-primary"], T["bg-surface"]],
    ["muted on canvas", T["text-muted"], T["bg-canvas"]],
    ["muted on surface", T["text-muted"], T["bg-surface"]],
    ["shell text", T["text-on-shell"], T["bg-shell"]],
    ["nav link", literals.navLink, T["bg-shell"]],
    ["active nav link", T["text-on-shell"], literals.navActiveBg],
    ["verified badge", T.verified, T["bg-surface"]],
    ["suspected badge", T.suspected, T["bg-surface"]],
    ["confirmed badge", T.confirmed, T["bg-surface"]],
    ["unknown badge", T.unknown, T["bg-surface"]],
    ["info text", T.info, T["bg-surface"]],
  ];
  for (const [name, fg, bg] of pairs) {
    assert.ok(fg && bg, `${name}: missing token`);
    const ratio = contrast(fg, bg);
    assert.ok(ratio >= 4.5, `${name}: ${ratio.toFixed(2)}:1 is below 4.5:1 (${fg} on ${bg})`);
  }
});

test("interactive borders and focus rings meet SC 1.4.11 (3:1)", () => {
  const T = tokens();
  // A control's boundary has to be perceivable; a decorative panel edge does
  // not, which is why `--border` and `--border-strong` are separate tokens.
  assert.ok(
    contrast(T["border-strong"], T["bg-surface"]) >= 3,
    `control border ${contrast(T["border-strong"], T["bg-surface"]).toFixed(2)}:1`,
  );
  assert.ok(
    contrast(T.info, T["bg-surface"]) >= 3,
    "focus ring on surface must be perceivable",
  );
  assert.ok(contrast(T.info, T["bg-shell"]) >= 3, "focus ring on the shell must be perceivable");
});

test("controls use the strong border, not the decorative one", () => {
  // Getting this wrong reintroduces the 1.38:1 control outline.
  const controls = appCss.slice(
    appCss.indexOf("input, textarea, select {"),
    appCss.indexOf("input, textarea, select {") + 200,
  );
  assert.match(controls, /var\(--border-strong\)/, "form controls need the strong border");
  assert.match(
    appCss,
    /button\.secondary \{[^}]*var\(--border-strong\)/,
    "secondary buttons need the strong border",
  );
});

test("severity is never carried by colour alone", () => {
  // The severity hues are close in luminance, so they would be hard to tell
  // apart if colour were the only signal. It is not: each badge carries text.
  const app = readFileSync(join(root, "../src/App.tsx"), "utf8");
  assert.match(app, /t\(locale, "confirmed"\)/);
  assert.match(app, /t\(locale, "suspected"\)/);
  assert.match(app, /t\(locale, "unknown"\)/);
});
