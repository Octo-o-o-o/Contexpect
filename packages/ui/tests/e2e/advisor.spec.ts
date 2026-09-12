// DEMO-14 Advisor: the double-confirmation gate and the candidate boundary,
// against a real daemon (see global-setup.ts).
//
// What this proves: without both confirmations the page issues no request and
// says so, the daemon itself refuses an unapproved send (consent/ack are
// re-checked server-side), an approved send renders candidates that carry the
// six "not a fact / not into policy/CI/baseline/reconciliation / no treatment
// unlock" flags as false, and nothing is persisted — the store is byte-identical
// afterwards.
//
// What this does not prove: any real LLM analysis (the adapter is the local
// heuristic by contract), or any downstream care-plan unlock (none exists in
// this slice).
import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { zh } from "../../src/i18n-tables.js";

const base = () => process.env.E2E_BASE_URL ?? "";
const store = () => process.env.E2E_STORE ?? "";

/** Relative path → content digest for every file under the store. */
function storeSnapshot(): Record<string, string> {
  const out: Record<string, string> = {};
  const walk = (dir: string, rel: string) => {
    for (const name of readdirSync(dir).sort()) {
      const path = join(dir, name);
      const relPath = rel ? `${rel}/${name}` : name;
      if (statSync(path).isDirectory()) walk(path, relPath);
      else out[relPath] = createHash("sha256").update(readFileSync(path)).digest("hex");
    }
  };
  walk(store(), "");
  return out;
}

test.beforeEach(({}, testInfo) => {
  testInfo.setTimeout(30_000);
  expect(base(), "global setup exported E2E_BASE_URL").not.toBe("");
  expect(store(), "global setup exported E2E_STORE").not.toBe("");
});

test("DEMO-14 advisor: no send without both confirmations; an approved send yields unpersisted candidates only", async ({
  page,
}) => {
  const advisorRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/v1/advisor")) advisorRequests.push(request.method());
  });

  await page.goto(`${base()}/advisor`);

  // The preview of what would be sent is shown up front.
  const preview = page.getByTestId("advisor-preview");
  await expect(preview).toContainText("e1");
  await expect(preview).toContainText("redacted");
  await expect(preview).toContainText("local-heuristic-not-llm");

  // Untouched: the send button is disabled and the page names the required
  // consent state instead of sending anything.
  const send = page.getByRole("button", { name: zh.advisorSend });
  await expect(send).toBeDisabled();
  await expect(page.locator("main")).toContainText(zh.advisorBlocked);

  // One confirmation alone is still not authorization.
  await page.getByRole("checkbox", { name: zh.advisorConsent }).check();
  await expect(send).toBeDisabled();
  await page.getByRole("checkbox", { name: zh.advisorConsent }).uncheck();
  await page.getByRole("checkbox", { name: zh.advisorAck }).check();
  await expect(send).toBeDisabled();
  await page.getByRole("checkbox", { name: zh.advisorAck }).uncheck();

  // No request left the page at any point above.
  expect(advisorRequests).toEqual([]);

  // The daemon re-checks independently: a direct unapproved POST is refused.
  for (const body of [
    { consent: false, preview_ack: false },
    { consent: true, preview_ack: false },
    { consent: false, preview_ack: true },
  ]) {
    const refused = await page.request.post(`${base()}/api/v1/advisor`, {
      headers: { "X-Ctxpect-Client": "desktop" },
      data: body,
    });
    expect(refused.status()).toBe(400);
    const code = (await refused.json()).error.code;
    expect(["advisor.consent_required", "advisor.preview_required"]).toContain(code);
  }
  expect(advisorRequests).toEqual([]);

  // Both confirmations, then send.
  const before = storeSnapshot();
  await page.getByRole("checkbox", { name: zh.advisorConsent }).check();
  await page.getByRole("checkbox", { name: zh.advisorAck }).check();
  await expect(send).toBeEnabled();
  await send.click();
  expect(advisorRequests).toEqual(["POST"]);

  // Candidates render as candidates: the boundary note plus the six flags.
  await expect(page.getByTestId("advisor-not-claim")).toContainText(zh.advisorNotClaim);
  const candidates = page.getByTestId("advisor-candidates");
  await expect(candidates.locator("li")).toHaveCount(1);
  await expect(candidates).toContainText("Collect native evidence");
  await expect(candidates).toContainText("care-plan");
  const flagValues = await page.getByTestId("advisor-flags").locator("dd code").allTextContents();
  expect(flagValues).toEqual([
    "false", // is_claim
    "false", // enters_policy
    "false", // enters_ci
    "false", // enters_baseline
    "false", // enters_reconciliation
    "false", // unlocks_treatment
    "advisor-suggestion", // kind
  ]);

  // No candidate is persisted: the store is byte-identical, and the Receipt
  // ledger is still empty.
  expect(storeSnapshot()).toEqual(before);
  const receipts = await page.request.get(`${base()}/api/v1/receipts`);
  expect(((await receipts.json()) as { receipts: unknown[] }).receipts).toEqual([]);
});
