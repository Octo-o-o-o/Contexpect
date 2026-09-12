// Synthetic project files, real inspect/Receipt/diff APIs and real browser UI.
import { expect, test } from "./isolated-test";
import type { APIRequestContext, Page } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { zh } from "../../src/i18n-tables.js";

async function inspect(request: APIRequestContext, base: string, harness = "codex", version = "0.147.0") {
  const response = await request.post(`${base}/api/v1/inspect`, {
    headers: { "X-Ctxpect-Client": "desktop" },
    data: { harness, version, surface: "cli", os_lane: "macos-27-arm64" },
  });
  expect(response.ok()).toBe(true);
  return (await response.json()).receipt;
}

function facet(page: Page, name: string) {
  return page.locator(".facet-grid .facet").filter({ has: page.locator(".f-lab", { hasText: new RegExp(`^${name}$`) }) });
}

test("DEMO-02 static eligibility does not become model-visible or runtime evidence", async ({ page, daemon }, testInfo) => {
  const receipt = await inspect(page.request, daemon.base);
  expect(receipt.facets.eligible.truth_state).toBe("present");
  expect(receipt.facets["model-visible"].truth_state).toBe("indeterminate");
  await page.goto(`${daemon.base}/inspector`);
  await expect(facet(page, "eligible").locator(".f-val")).toHaveText("present");
  for (const name of ["model-visible", "use-evidence", "outcome-affecting"]) {
    await expect(facet(page, name).locator(".f-val")).toHaveText("indeterminate");
  }
  await page.screenshot({ path: testInfo.outputPath("inspector-static-runtime-unknown.png"), fullPage: true });
});

test("DEMO-03 product exclusion leaves native loading unknown and budget unmeasured", async ({ page, daemon }, testInfo) => {
  writeFileSync(join(daemon.project, ".ctxpect-ignore"), "AGENTS.md\n");
  const receipt = await inspect(page.request, daemon.base);
  expect(receipt.facets.eligible.truth_state).toBe("indeterminate");
  expect(receipt.facets.eligible.unknown_reason_code).toBe("observation_scope_excluded");
  for (const [key, cell] of Object.entries(receipt.budget)) {
    if (key === "note") continue;
    expect(cell).toMatchObject({ status: "unknown", value: null });
  }
  await page.goto(`${daemon.base}/inspector`);
  await expect(facet(page, "eligible").locator(".f-val")).toHaveText("indeterminate");
  await expect(page.locator("main")).toContainText("observation_scope_excluded");
  await expect(page.locator("main")).toContainText(zh.budgetUnknownNote);
  await page.screenshot({ path: testInfo.outputPath("inspector-observation-excluded.png"), fullPage: true });
});

test("DEMO-10 different coordinates cannot become a baseline and changing selection invalidates the diff", async ({ page, daemon }, testInfo) => {
  const codex = await inspect(page.request, daemon.base);
  writeFileSync(join(daemon.project, "CLAUDE.md"), "synthetic Claude instructions\n");
  const claude = await inspect(page.request, daemon.base, "claude-code", "2.1.259");
  expect(codex.receipt_id).not.toBe(claude.receipt_id);
  await page.goto(`${daemon.base}/compare`);
  await page.getByLabel("diff-a").selectOption(codex.receipt_id);
  await page.getByLabel("diff-b").selectOption(claude.receipt_id);
  await page.getByRole("button", { name: zh.runDiff, exact: true }).click();
  const result = page.locator("main pre.mono");
  await expect(page.getByTestId("compare-incomparable")).toContainText(zh.compareIncomparable);
  let value = JSON.parse(await result.innerText());
  expect(value.same_domain).toBe(false);
  expect(value.baseline_allowed).toBe(false);
  expect(value.verified).toBe(false);
  expect(value.reconciliation).toBe("indeterminate");
  await page.screenshot({ path: testInfo.outputPath("compare-cross-coordinate.png"), fullPage: true });

  await page.getByLabel("diff-b").selectOption(codex.receipt_id);
  await expect(result).toHaveCount(0);
  await expect(page.getByTestId("compare-incomparable")).toHaveCount(0);
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let entered!: () => void;
  const pending = new Promise<void>((resolve) => { entered = resolve; });
  await page.route("**/api/v1/diff?*", async (route) => {
    entered();
    await held;
    await route.continue().catch(() => {}); // Selection cancels this request.
  });
  await page.getByRole("button", { name: zh.runDiff, exact: true }).click();
  await pending;
  const cancelled = page.waitForEvent("requestfailed", { predicate: (request) => request.url().includes("/api/v1/diff?") });
  await page.getByLabel("diff-b").selectOption(claude.receipt_id);
  release();
  await cancelled;
  await expect(result).toHaveCount(0);
  await page.unroute("**/api/v1/diff?*");
  await page.getByRole("button", { name: zh.runDiff, exact: true }).click();
  await expect(page.getByTestId("compare-incomparable")).toBeVisible();
  value = JSON.parse(await result.innerText());
  expect(value.same_domain).toBe(false);
});

test("compare retries the Receipt list after a transport failure", async ({ page, daemon }) => {
  const receipt = await inspect(page.request, daemon.base);
  await page.route("**/api/v1/receipts", (route) => route.abort("connectionrefused"));
  await page.goto(`${daemon.base}/compare`);
  await expect(page.locator('[data-state="offline"]')).toBeVisible();
  await page.unroute("**/api/v1/receipts");
  await page.getByRole("button", { name: zh.retry, exact: true }).click();
  await expect(page.getByLabel("diff-a").locator("option", { hasText: receipt.receipt_id })).toHaveCount(1);
  await expect(page.locator('[data-state="offline"]')).toHaveCount(0);
});
