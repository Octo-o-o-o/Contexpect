// Server-side render entry for the presentation-layer tests.
//
// `renderRoute` renders the whole application at a route with a MemoryRouter,
// which is what a first paint looks like before any request resolves (every
// self-fetching page starts in `loading`). `renderState` renders the C04
// state banner a page shows for one state. Neither performs a request: SSR
// runs no effects, so this covers presentation, not data flow or interaction.
import { renderToString } from "react-dom/server";
import { MemoryRouter } from "react-router-dom";
import { App, LabResultView, SessionRequestsView, StateBanner } from "./App";
import type { Locale } from "./i18n";
import type { Json } from "./api";

export function renderRoute(route: string, locale: Locale = "zh-CN"): string {
  void locale;
  return renderToString(
    <MemoryRouter initialEntries={[route]}>
      <App />
    </MemoryRouter>,
  );
}

/** The request-evidence view for a session record and its requests document. */
export function renderSessionRequests(session: Json, requests: Json, selected: number | null = null, locale: Locale = "zh-CN"): string {
  return renderToString(
    <SessionRequestsView session={session} requests={requests} selected={selected} onSelect={() => undefined} locale={locale} />,
  );
}

/** The Effect Lab result view for one experiment document. */
export function renderLabResult(data: Json, locale: Locale = "zh-CN"): string {
  return renderToString(<LabResultView data={data} locale={locale} />);
}

export function renderState(route: string, state: string, reasonCode: string, locale: Locale = "zh-CN"): string {
  return renderToString(
    <StateBanner
      route={route}
      status={state}
      reasonCode={reasonCode}
      retryable={state === "error" || state === "offline" || state === "empty"}
      locale={locale}
      onRetry={() => undefined}
    />,
  );
}
