// Three-way consistency: the CLI, the daemon API and the UI must answer the
// same question identically (R04 extended to the UI leg).
//
// Fact choice is deliberate. The two facts below are read-only from every
// leg, because this daemon+store is shared with the whole e2e run and the
// cold-start assertions elsewhere depend on no Receipt existing yet:
//   - `ctxpect doctor --store …` *persists* a one-shot Receipt (dispatch.rs
//     `doctor_cmd` → `persist_inspect_in`), and the daemon's /api/v1/doctor
//     needs a selected Receipt to answer at all — so a doctor/receipt-id
//     three-way cannot run here without poisoning the shared cold start.
//   - Doctor's time semantics also differ by entry: after T02d the CLI
//     defaults to the wall clock (pin with `--as-of`), while the API has no
//     as_of entry. The facts chosen here carry no as_of-sensitive rule
//     output, so all three legs read the same answer under equal input.
// Timestamp-class fields are stripped following the R04 precedent in
// crates/ctxpect-cli/tests/product_loops.rs.
import { expect, test } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../../..");
const base = () => process.env.E2E_BASE_URL ?? "";
const store = () => process.env.E2E_STORE ?? "";
const project = () => process.env.E2E_PROJECT ?? "";
const bin = () => process.env.CTXPECT_BIN ?? join(repoRoot, "target/debug/ctxpect");

type Json = Record<string, unknown>;

function cliJson(args: string[]): Json {
  const out = execFileSync(bin(), args, { encoding: "utf8" });
  return JSON.parse(out) as Json;
}

/** Drop fields that legitimately differ between invocations (R04's list). */
function strip(value: unknown): unknown {
  const volatile = new Set([
    "created_at",
    "at",
    "snapshot_digest",
    "command",
    "exit_code",
    "schema_version",
  ]);
  if (Array.isArray(value)) return value.map(strip);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Json)
        .filter(([key]) => !volatile.has(key))
        .map(([key, item]) => [key, strip(item)]),
    );
  }
  return value;
}

test.beforeEach(({}, testInfo) => {
  testInfo.setTimeout(30_000);
  expect(base(), "global setup exported E2E_BASE_URL").not.toBe("");
});

test("an exception's status document is identical from the CLI and the API, and the UI lists the same id", async ({
  page,
}) => {
  const exceptionId = "ex-e2e-sessions-import";
  const cli = cliJson([
    "exception", "status", "--json",
    "--store", store(), "--project", project(), "--id", exceptionId,
  ]);
  const response = await page.request.get(`${base()}/api/v1/exceptions/${exceptionId}`);
  expect(response.ok()).toBe(true);
  const api = (await response.json()) as Json;
  expect(strip(cli)).toEqual(strip(api));
  // The fact under comparison, not just any JSON: this is the planted grant.
  expect(api.exception_id).toBe(exceptionId);
  expect(api.state).toBe("approved");

  // The UI leg: /exceptions renders ids (there is no detail route), so the
  // rendered set must contain exactly the ids the API lists.
  const listApi = (await (await page.request.get(`${base()}/api/v1/exceptions`)).json()) as Json;
  const apiIds = (listApi.exceptions as string[]).slice().sort();
  expect(apiIds).toContain(exceptionId);
  await page.goto(`${base()}/exceptions`);
  const uiIds = (await page.locator("tbody td.mono").allTextContents()).map((s) => s.trim()).sort();
  expect(uiIds).toEqual(apiIds);
});

test("the session id set and one session document agree across CLI, API and the rendered list", async ({
  page,
}) => {
  // (b) CLI leg.
  const cliList = cliJson(["sessions", "--json", "--store", store()]);
  const cliIds = (cliList.sessions as string[]).slice().sort();

  // (a) API leg.
  const listResponse = await page.request.get(`${base()}/api/v1/sessions`);
  expect(listResponse.ok()).toBe(true);
  const apiList = (await listResponse.json()) as Json;
  const apiIds = (apiList.sessions as string[]).slice().sort();

  // (c) UI leg: the rendered link texts on /sessions.
  await page.goto(`${base()}/sessions`);
  const uiIds = (
    await page.getByTestId("session-list").locator("tbody tr td:first-child").allTextContents()
  )
    .map((s) => s.trim())
    .sort();

  expect(cliIds).toEqual(apiIds);
  expect(uiIds).toEqual(apiIds);
  expect(apiIds).toEqual(["s-alpha", "s-beta"]);

  // One full document, CLI vs API, volatile fields stripped as in R04.
  const cliDoc = cliJson(["sessions", "--json", "--store", store(), "--id", "s-alpha"]);
  const docResponse = await page.request.get(`${base()}/api/v1/sessions/s-alpha`);
  expect(docResponse.ok()).toBe(true);
  expect(strip(cliDoc)).toEqual(strip(await docResponse.json()));
});
