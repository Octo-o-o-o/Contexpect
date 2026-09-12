import { test, expect } from "@playwright/test";
import { startTestDaemon } from "./test-daemon";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { zh } from "../../src/i18n-tables.js";

test("periodic monitor shows a real new Receipt and its static scope", async ({ page }, info) => {
  const runtime = await startTestDaemon([], true);
  try {
    const read = async () => (await (await page.request.get(`${runtime.base}/api/v1/monitor`)).json()).continuous;
    await expect.poll(async () => (await read()).last_receipt_id).toBeTruthy();
    const first = (await read()).last_receipt_id;
    writeFileSync(join(runtime.project, "AGENTS.md"), "changed synthetic instructions\n");
    await expect.poll(async () => (await read()).last_receipt_id).not.toBe(first);
    const latest = (await read()).last_receipt_id;
    await page.goto(`${runtime.base}/monitor`);
    await expect(page.getByTestId("monitor-summary")).toContainText(zh.monitorStaticScope);
    await expect(page.getByRole("link", { name: latest, exact: true })).toBeVisible();
    await page.screenshot({ path: info.outputPath("periodic-monitor.png"), fullPage: true });
    await page.getByRole("link", { name: latest, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/receipts/${latest}$`));
  } finally { await runtime.stop(); }
});

test("sync discloses the CLI encryption boundary and disables stale preview", async ({ page }, info) => {
  const runtime = await startTestDaemon();
  try {
    await page.goto(`${runtime.base}/sync`);
    await expect(page.getByTestId("sync-encrypted-notice")).toHaveText(zh.syncEncryptedCli);
    await expect(page.getByRole("button", { name: zh.syncApply, exact: true })).toBeDisabled();
    await page.getByRole("button", { name: zh.syncPreview, exact: true }).click();
    await expect(page.getByTestId("sync-outcome")).toBeVisible();
    await expect(page.getByRole("button", { name: zh.syncApply, exact: true })).toBeEnabled();
    await page.getByLabel("bundle_id", { exact: true }).fill("different-bundle");
    await expect(page.getByRole("button", { name: zh.syncApply, exact: true })).toBeDisabled();
    await expect(page.getByTestId("sync-outcome")).toHaveCount(0);
    await page.screenshot({ path: info.outputPath("sync-encryption-boundary.png"), fullPage: true });
  } finally { await runtime.stop(); }
});
