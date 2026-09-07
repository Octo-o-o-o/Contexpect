// Masking policy. Kept as plain JS with a `.d.ts` sibling — the same shape as
// `i18n-tables.js` — so `node --test` can exercise the real functions the UI
// runs, not a re-implementation of them.

/**
 * Whether held-to-reveal currently reveals body text.
 *
 * Screenshot privacy is a hard mask: holding does not lift it. That is the
 * whole point of the mode, so it is decided here and nowhere else.
 *
 * @param {boolean} hold
 * @param {"default" | "screenshot"} privacy
 * @returns {boolean}
 */
export function revealed(hold, privacy) {
  return hold === true && privacy !== "screenshot";
}

/**
 * Whether the project path may be rendered into the DOM.
 *
 * Focus is deliberately not an input. A focused field that unmasks would put
 * the real path back into the DOM and into the accessibility tree in exactly
 * the mode that exists to keep it out of both.
 *
 * @param {"default" | "screenshot"} privacy
 * @param {boolean} hold
 * @returns {boolean}
 */
export function projectVisible(privacy, hold) {
  return privacy === "default" || revealed(hold, privacy);
}

/**
 * What the project field shows. While masked the field is read-only, so the
 * mask characters can never be edited back into the real value.
 *
 * @param {string} project
 * @param {boolean} visible
 * @returns {string}
 */
export function projectFieldValue(project, visible) {
  if (visible) return project;
  return project ? "••••" : "";
}
