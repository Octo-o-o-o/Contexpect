export const ROUTES = [
  { path: "/checkup", id: "V01", nav: "checkup" },
  { path: "/inspector", id: "V02", nav: "inspector" },
  { path: "/compare", id: "V03", nav: "compare" },
  { path: "/receipts", id: "V04", nav: "receipts" },
  { path: "/receipts/:id", id: "V04", nav: "receipts" },
  { path: "/assets", id: "V05", nav: "assets" },
  { path: "/assets/:id", id: "V05", nav: "assets" },
  { path: "/sessions", id: "V06", nav: "sessions" },
  { path: "/sessions/:id", id: "V06", nav: "sessions" },
  { path: "/monitor", id: "V06", nav: "monitor" },
  { path: "/lab", id: "V07", nav: "lab" },
  { path: "/lab/:id", id: "V07", nav: "lab" },
  { path: "/sync", id: "V08", nav: "sync" },
  { path: "/doctor", id: "V09", nav: "doctor" },
  { path: "/policy", id: "V10", nav: "policy" },
  { path: "/standards", id: "V11", nav: "standards" },
  { path: "/standards/:id", id: "V11", nav: "standards" },
  { path: "/settings", id: "V12", nav: "settings" },
  { path: "/exceptions", id: "V13", nav: "exceptions" },
  { path: "/team/compliance", id: "V14", nav: "team" },
  { path: "/care-plan/:findingId", id: "V15", nav: "care" },
  { path: "/integrations", id: "V16", nav: "integrations" },
  { path: "/integrations/:id", id: "V16", nav: "integrations" },
  { path: "/advisor", id: "V17", nav: "advisor" },
] as const;

export const NAV = [
  { to: "/doctor", key: "doctor" },
  { to: "/checkup", key: "checkup" },
  { to: "/inspector", key: "inspector" },
  { to: "/compare", key: "compare" },
  { to: "/receipts", key: "receipts" },
  { to: "/sessions", key: "sessions" },
  { to: "/monitor", key: "monitor" },
  { to: "/lab", key: "lab" },
  { to: "/advisor", key: "advisor" },
  { to: "/assets", key: "assets" },
  { to: "/integrations", key: "integrations" },
  { to: "/sync", key: "sync" },
  { to: "/policy", key: "policy" },
  { to: "/standards", key: "standards" },
  { to: "/exceptions", key: "exceptions" },
  { to: "/team/compliance", key: "team" },
  { to: "/settings", key: "settings" },
] as const;

/**
 * The nav key for a pathname, matching `:param` segments.
 *
 * Used to title the document: a single-page app that never updates its title
 * leaves every route reading as the same page (WCAG 2.2 SC 2.4.2).
 */
export function navKeyForPath(pathname: string): string | null {
  const path = pathname.replace(/\/+$/, "") || "/";
  // Longest pattern first, so `/receipts/:id` wins over `/receipts`.
  const ordered = [...ROUTES].sort((a, b) => b.path.length - a.path.length);
  for (const route of ordered) {
    const pattern = new RegExp(
      `^${route.path.replace(/:[A-Za-z]+/g, "[^/]+").replace(/\//g, "\\/")}$`,
    );
    if (pattern.test(path)) return route.nav;
  }
  return null;
}
