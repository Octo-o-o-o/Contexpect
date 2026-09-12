import { test, expect } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { startTestDaemon } from "./test-daemon";
import { zh } from "../../src/i18n-tables.js";

test("real project audit preserves fixture blocking but never diagnoses PNG bytes as instructions", async ({ page }) => {
  const runtime = await startTestDaemon();
  try {
    const corpus = join(runtime.project, "acceptance/corpus/isolated-test");
    mkdirSync(corpus, { recursive: true });
    writeFileSync(join(corpus, "AGENTS.md"), "ghp_fixture_not_a_real_secret_00\n");
    writeFileSync(join(runtime.project, "evidence.png"), Buffer.concat([
      Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), Buffer.from("over\u200Bride\n"),
    ]));
    await page.request.post(`${runtime.base}/api/v1/inspect`, {
      headers: { "X-Ctxpect-Client": "desktop" }, data: {},
    });
    await page.goto(`${runtime.base}/doctor`);
    await expect(page.getByTestId("audit-scope-note")).toContainText(zh.auditScopeNote);
    const findings = page.getByRole("table", { name: zh.findingsTableCaption });
    await expect(findings.getByText("acceptance/corpus/isolated-test/AGENTS.md", { exact: true })).toBeVisible();
    await expect(findings).toContainText("blocking");
    await expect(findings).not.toContainText("evidence.png");
    const counts = await page.getByTestId("active-counts").innerText();
    await page.getByLabel(zh.findingScope, { exact: true }).selectOption("receipt");
    await expect(findings).not.toContainText("acceptance/corpus");
    await expect(page.getByTestId("active-counts")).toHaveText(counts);
    await page.getByRole("link", { name: zh.integrations, exact: true }).click();
    await expect(page.getByTestId("integration-catalog")).toContainText("0.147.0");
    await expect(page.locator('.statebanner[data-state="unsupported-version"]')).toHaveCount(0);
    await page.getByRole("link", { name: zh.assets, exact: true }).click();
    await expect(page.getByRole("heading", { name: zh.assets, exact: true })).toBeVisible();
    await expect(page.locator('.statebanner[data-state="unsupported-version"]')).toHaveCount(0);
    await page.getByRole("link", { name: zh.checkup, exact: true }).click();
    await expect(page.getByTestId("static-check-result")).toHaveText("pass");
    await expect(page.getByText(zh.staticCheckLimit, { exact: true })).toBeVisible();
  } finally { await runtime.stop(); }
});

test("stalled bootstrap becomes an explicit timeout instead of evaluated Unknown facets", async ({ page }) => {
  test.setTimeout(45_000);
  await page.route("**/api/v1/status", () => {});
  await page.goto(`${process.env.E2E_BASE_URL}/inspector`);
  await expect(page.getByText(zh.loadingEvidence, { exact: true })).toBeVisible();
  await expect(page.locator(".facet-grid")).toHaveCount(0);
  await expect(page.getByRole("alert").filter({ hasText: "api.timeout" }).first()).toBeVisible({ timeout: 35_000 });
  await expect(page.getByText(zh.timeoutNext, { exact: false })).toBeVisible();
});

test("inspection stays loading until its diagnosis finishes", async ({ page }) => {
  await page.goto(`${process.env.E2E_BASE_URL}/doctor`);
  await expect(page.getByTestId("active-counts")).toBeVisible();
  let release!: () => void;
  let entered!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  const requested = new Promise<void>((resolve) => { entered = resolve; });
  await page.route("**/api/v1/doctor?**", async (route) => {
    entered();
    await held;
    await route.continue();
  });
  await page.getByRole("button", { name: zh.inspect, exact: true }).click();
  await requested;
  try {
    await expect(page.locator(".topbar").getByText(zh.loading, { exact: true })).toBeVisible();
    await expect(page.getByTestId("active-counts")).toHaveCount(0);
  } finally { release(); }
  await expect(page.getByTestId("active-counts")).toBeVisible();
});
