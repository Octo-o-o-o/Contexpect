// C04 page states, derived from what the API actually returns.
//
// Plain JS with a `.d.ts` sibling so `node --test` exercises the real
// classifier the UI runs. The rules below map to reason codes the product
// actually emits; nothing here invents a state the backend cannot produce.

/** Every state C04 names, plus `ok` for "content, nothing special". */
export const STATES = [
  "loading",
  "ok",
  "empty",
  "error",
  "partial",
  "stale",
  "offline",
  "permission-denied",
  "unsupported-version",
  "connector-missing",
];

/**
 * Error codes that mean "you are not allowed", as opposed to "it broke".
 * Prefixes match the crate that owns the decision.
 */
const DENIED_PREFIXES = ["policy.", "principal.", "exception."];
const DENIED_CODES = new Set([
  "api.identity_required",
  "advisor.consent_required",
  "advisor.preview_required",
  "io.permission_denied",
  "permission_not_granted",
]);

/** Reason codes carried *inside* a successful payload. */
const UNSUPPORTED_VERSION = "unsupported_harness_version";
const CONNECTOR_REQUIRED = "connector_required";

/** Reason codes that mean the answer is incomplete rather than wrong. */
const PARTIAL_REASONS = new Set([
  "observation_scope_excluded",
  "runtime_snapshot_missing",
  "config_residue_only",
  "surface_not_exposed",
  "tool_schema_not_exported",
  "official_distribution_not_captured",
  "current_occupancy_not_reported",
  "attachment_unavailable",
  "authentication_unavailable",
  "cloud_setting_unavailable",
  "sandbox_unavailable",
]);

const STALE_REASONS = new Set(["evidence_stale"]);

/**
 * Classify a failed request.
 *
 * @param {"transport" | "envelope"} kind — `transport` when fetch itself
 *   failed, so the daemon could not be reached at all.
 * @param {string} code — the envelope's `error.code`, empty for transport.
 * @returns {{state: string, reasonCode: string, retryable: boolean}}
 */
export function classifyFailure(kind, code) {
  if (kind === "transport") {
    return { state: "offline", reasonCode: code || "api.unreachable", retryable: true };
  }
  const denied =
    DENIED_CODES.has(code) || DENIED_PREFIXES.some((prefix) => code.startsWith(prefix));
  return {
    state: denied ? "permission-denied" : "error",
    reasonCode: code || "api.error",
    // Retrying a refusal re-sends the same request; it cannot widen the
    // authorization that was refused, so offering retry is safe but useless
    // until something else changes. Retry is offered for faults only.
    retryable: !denied,
  };
}

/** Collect every `reason_code` anywhere in a payload. */
function reasonCodes(value, out = new Set(), depth = 0) {
  if (depth > 8 || value == null || typeof value !== "object") return out;
  if (Array.isArray(value)) {
    for (const item of value) reasonCodes(item, out, depth + 1);
    return out;
  }
  for (const [key, child] of Object.entries(value)) {
    // A catalog is a list of independent coordinates. An unsupported sibling
    // does not invalidate the selected harness or an asset action.
    if (key === "families" && Array.isArray(child)) {
      for (const family of child) {
        if (family?.active_coordinate === true) reasonCodes(family, out, depth + 1);
      }
      continue;
    }
    if ((key === "reason_code" || key === "connector") && typeof child === "string") {
      out.add(child);
    }
    reasonCodes(child, out, depth + 1);
  }
  return out;
}

/** Whether a payload carries anything worth rendering. */
function isEmpty(value) {
  if (value == null) return true;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value !== "object") return false;
  // Envelope bookkeeping is not content.
  const skip = new Set(["schema", "schema_version", "command", "exit_code", "snapshot_digest"]);
  const entries = Object.entries(value).filter(([key]) => !skip.has(key));
  if (entries.length === 0) return true;
  return entries.every(([, child]) => Array.isArray(child) && child.length === 0);
}

/**
 * Classify a successful payload by what it says about itself.
 *
 * Order matters: staleness invalidates the whole answer, an unsupported
 * version or missing connector explains why content is thin, and `partial`
 * is the residual "some cells are Unknown".
 *
 * @param {unknown} data
 * @returns {{state: string, reasonCode: string, retryable: boolean}}
 */
export function classifyPayload(data) {
  const codes = reasonCodes(data);
  const obj = data && typeof data === "object" && !Array.isArray(data) ? data : {};
  const staleness = obj.staleness && typeof obj.staleness === "object" ? obj.staleness : null;

  if (obj.stale === true || staleness?.status === "stale") {
    return { state: "stale", reasonCode: staleness?.reason_code ?? "monitor.evidence_changed", retryable: true };
  }
  for (const code of codes) {
    if (STALE_REASONS.has(code)) {
      return { state: "stale", reasonCode: code, retryable: true };
    }
  }
  if (codes.has(UNSUPPORTED_VERSION)) {
    return { state: "unsupported-version", reasonCode: UNSUPPORTED_VERSION, retryable: false };
  }
  if (codes.has(CONNECTOR_REQUIRED)) {
    return { state: "connector-missing", reasonCode: CONNECTOR_REQUIRED, retryable: false };
  }
  if (isEmpty(data)) {
    return { state: "empty", reasonCode: "api.no_content", retryable: true };
  }

  const unknownCount = typeof obj.unknown === "number" ? obj.unknown : 0;
  const countsUnknown =
    obj.counts && typeof obj.counts === "object" && typeof obj.counts.unknown === "number"
      ? obj.counts.unknown
      : 0;
  const partialReason = [...codes].find((code) => PARTIAL_REASONS.has(code));
  if (partialReason || unknownCount > 0 || countsUnknown > 0 || staleness?.status === "unknown") {
    return {
      state: "partial",
      reasonCode: partialReason ?? staleness?.reason_code ?? "claim.unknown_cells",
      retryable: true,
    };
  }
  return { state: "ok", reasonCode: "", retryable: false };
}
