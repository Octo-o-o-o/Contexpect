/** Presentation of existing evidence only; none of these helpers grants authority. */
export function findingScope(finding, receipt) {
  if (finding.facet !== "project-content") return "receipt";
  const path = finding.path;
  // These rules compare multiple files; a match for just one path cannot
  // establish that all participating files belong to the observed chain.
  if (["duplicate", "conflict", "gitignore_mismatch"].includes(finding.rule_id)) return "project";
  const layers = Array.isArray(receipt.layers) ? receipt.layers : [];
  const linked = layers.some((layer) => {
    if (typeof layer.rel !== "string" || !layer.rel.startsWith("<project>/") ||
        typeof layer.adopted !== "string") return false;
    return layer.adopted === path;
  });
  return linked ? "linked-file" : "project";
}

export function findingSource(finding) {
  const path = typeof finding.path === "string" ? finding.path : "";
  if (path.startsWith("acceptance/corpus/")) return "corpus";
  if (path.startsWith(".octoworkflow/")) return "history";
  if (/(^|\/)(tests?|fixtures?)(\/|\.)/.test(path)) return "test";
  return "file";
}

/** Accept both the store clock and RFC3339; preserve an unrecognized value. */
export function displayTime(value, locale) {
  if (typeof value !== "string" || !value) return "—";
  const storeClock = /^(\d+)\.(\d+)Z$/.exec(value);
  const millis = storeClock ? Number(`${storeClock[1]}.${storeClock[2]}`) * 1000 : Date.parse(value);
  const date = new Date(millis);
  return Number.isFinite(millis) && !Number.isNaN(date.getTime())
    ? new Intl.DateTimeFormat(locale === "zh-CN" ? "zh-CN" : "en", {
      year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit",
      second: "2-digit", timeZoneName: "short", hour12: false,
    }).format(date)
    : value;
}
