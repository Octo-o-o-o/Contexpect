// No fabricated response bodies: route.abort injects only transport failure.
import { expect, test } from "./isolated-test";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { zh } from "../../src/i18n-tables.js";
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

test("DEMO-17 refreshing offline retains the same snapshot, and recovery replaces it", async ({ page, daemon }, testInfo) => {
  await page.goto(`${daemon.base}/sessions`);
  await expect(page.getByRole("link", { name: "s-alpha", exact: true })).toBeVisible();
  const before = await page.locator("main table").innerText();
  await page.route("**/api/v1/sessions", (route) => route.abort("connectionrefused"));
  await page.getByRole("button", { name: zh.refresh, exact: true }).click();
  await expect(page.locator('[data-state="offline"]')).toContainText("api.unreachable");
  await expect(page.getByTestId("retained-snapshot")).toContainText(zh.retainedSnapshot);
  expect(await page.locator("main table").innerText()).toBe(before);
  await page.screenshot({ path: testInfo.outputPath("sessions-offline-retained.png"), fullPage: true });
  await page.unroute("**/api/v1/sessions");
  await page.getByRole("button", { name: zh.retry, exact: true }).click();
  await expect(page.getByTestId("retained-snapshot")).toHaveCount(0);
  await expect(page.locator('[data-state="offline"]')).toHaveCount(0);
  expect(await page.locator("main table").innerText()).toBe(before);
});

test("DEMO-13 deleting an import invalidates insights, refresh removes it, and the source stays intact", async ({ page, daemon }) => {
  const source = join(repoRoot, "acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1/session.jsonl");
  const original = readFileSync(source);
  await page.goto(`${daemon.base}/sessions`);
  await expect(page.getByRole("link", { name: "s-alpha", exact: true })).toBeVisible();
  const bin = process.env.CTXPECT_BIN ?? join(repoRoot, "target/debug/ctxpect");
  const result = JSON.parse(execFileSync(bin, ["sessions", "--reason", "delete", "--session", "s-alpha", "--project", daemon.project, "--store", daemon.store, "--json"], { encoding: "utf8", timeout: 10_000 }));
  expect(result.insights_invalidated).toBe(true);
  expect(existsSync(join(daemon.store, "insights/s-alpha.json"))).toBe(false);
  expect(readFileSync(source)).toEqual(original);
  await page.getByRole("button", { name: zh.refresh, exact: true }).click();
  await expect(page.getByRole("link", { name: "s-alpha", exact: true })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "s-beta", exact: true })).toBeVisible();
  await page.goto(`${daemon.base}/sessions/s-alpha`);
  await expect(page.locator('[data-state="error"]')).toContainText("store.missing");
  await expect(page.getByTestId("request-table")).toHaveCount(0);
});

test("Q03 refreshing a different resource cannot inherit the previous resource's snapshot", async ({ page, daemon }) => {
  await page.goto(`${daemon.base}/lab/e-lab`);
  await expect(page.getByTestId("lab-result")).toHaveAttribute("data-executed", "true");
  await page.route("**/api/v1/lab/missing", (route) => route.abort("connectionrefused"));
  await page.evaluate(() => {
    history.pushState({}, "", "/lab/missing");
    dispatchEvent(new PopStateEvent("popstate"));
  });
  await expect(page.locator('[data-state="offline"]')).toBeVisible();
  await expect(page.getByTestId("lab-result")).toHaveCount(0);
  await expect(page.getByTestId("retained-snapshot")).toHaveCount(0);
});
