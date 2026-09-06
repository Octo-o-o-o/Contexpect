/** Typed design tokens. Unknown is not a severity color. */
export const evidence = {
  native: { label: "Native evidence", color: "var(--verified)" },
  static: { label: "Static resolution", color: "var(--info)" },
  attested: { label: "User-attested", color: "var(--verified)" },
  unknown: { label: "Unknown", color: "var(--unknown)" },
};

export const severity = {
  confirmed: { label: "Confirmed", color: "var(--confirmed)" },
  suspected: { label: "Suspected", color: "var(--suspected)" },
};

export const truthState = {
  present: "present",
  absent: "absent",
  indeterminate: "indeterminate",
  "not-applicable": "not-applicable",
};

/** Expected is a user-facing description of resolved context, not a truth_state. */
export const expectedIsNotTruthState = true;
export const unknownIsNotSeverity = true;
