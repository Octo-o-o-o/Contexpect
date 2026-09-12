// Synthetic inputs, real API and disk writes. Every test owns a fresh daemon/store.
import { expect, test } from "./isolated-test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { zh } from "../../src/i18n-tables.js";


const ASSET_ID = "skill-e2e";
const SOURCE_REL = "vendor/skill-e2e.md";
const TARGET_REL = ".ctxpect/skills/skill-e2e.md";
const SEEDED_BYTES = "synthetic e2e skill body (not a real skill)\n";

async function previewAsset(page: import("@playwright/test").Page, base: string) {
  await page.goto(`${base}/assets`);
  await page.getByPlaceholder(zh.assetsIdPlaceholder).fill(ASSET_ID);
  await page.getByRole("button", { name: zh.assetsPreview }).click();
  return page.getByTestId("assets-plan");
}

test("DEMO-06 static repair: vetting preview, approved copy, then static re-verification", async ({
  page, daemon,
}) => {
  // Proves: the UI drives the real preview → authorized copy → re-check loop
  // for a static fix; the copy lands only after a vetted preview, and the
  // post-write state is re-observed rather than assumed.
  // Does not prove: runtime/model-visible effects of the copied asset (no
  // harness run exists in this slice). A new static Receipt is linked separately.
  const plan = await previewAsset(page, daemon.base);
  await expect(plan).toContainText("MIT");
  await expect(plan).toContainText(`project:${SOURCE_REL}`);
  await expect(plan).toContainText(TARGET_REL);
  // Fresh target: nothing is lost.
  await expect(plan).toContainText("none");

  // Copy is gated on the preview: it is the same button row, enabled only now.
  await page.getByRole("button", { name: zh.assetsCopy }).click();
  await expect(page.getByTestId("assets-copied")).toContainText("tx_");

  const verification = page.getByTestId("assets-verification");
  await expect(verification).toContainText(zh.assetsRuntimeUnknown);
  const receiptLink = verification.getByRole("link");
  const href = await receiptLink.getAttribute("href");
  expect(href).toMatch(/^\/receipts\//);
  const receiptResponse = await page.request.get(`${daemon.base}/api/v1${href}`);
  expect(receiptResponse.ok()).toBe(true);
  expect((await receiptResponse.json()).coordinate.project_digest).toBeTruthy();

  // Static re-verification: the declared bytes actually landed…
  expect(readFileSync(join(daemon.project, TARGET_REL), "utf8")).toBe(SEEDED_BYTES);
  // …and a fresh preview now reports an overwrite plan with a loss statement
  // instead of "none" — the state changed in a way the UI can explain.
  const replan = await previewAsset(page, daemon.base);
  await expect(replan).toContainText("existing target content is replaced");
});

test("DEMO-12 a write without trusted authorization is refused while read-only use stays available", async ({
  page, daemon,
}) => {
  // Proves: a mutation the policy layer has no grant for (assets.rollback is
  // deliberately not granted in this seed) is refused by the daemon with the
  // policy reason code intact, the refusal changes nothing on disk, and the
  // safe read-only path (preview) keeps working afterwards. The UI identity
  // is not consulted: the refusal comes from the daemon, not the page.
  // Does not prove: the full principal/exception lifecycle (enrollment,
  // expiry), which lives in the policy crate's own tests.
  const plan = await previewAsset(page, daemon.base);
  await expect(plan).toBeVisible();

  // Copy is granted (assets.copy), so this lands; rollback is not.
  await page.getByRole("button", { name: zh.assetsCopy }).click();
  await expect(page.getByTestId("assets-copied")).toContainText("tx_");
  const before = readFileSync(join(daemon.project, TARGET_REL), "utf8");

  await page.getByRole("button", { name: zh.assetsRollback }).click();
  const refusal = page.getByTestId("assets-error");
  await expect(refusal).toContainText("policy.approval_required");
  // The refused write changed nothing.
  expect(readFileSync(join(daemon.project, TARGET_REL), "utf8")).toBe(before);

  // Read-only vetting is unaffected by the refusal.
  const again = await previewAsset(page, daemon.base);
  await expect(again).toContainText(TARGET_REL);
  await expect(page.getByTestId("assets-error")).toHaveCount(0);
});

test("DEMO-07 an edit after the preview makes the write refuse rather than silently copy different bytes", async ({
  page, daemon,
}) => {
  // Proves: after the UI shows a vetted preview, an out-of-band edit to the
  // asset source makes the daemon refuse the copy (assets.digest_mismatch) —
  // the bytes that were vetted are not the bytes on disk, and the UI shows
  // the refusal instead of reporting a write. Re-previewing fails the same
  // way, because the registry still pins the original digest.
  // The companion target-edit case also checks the persisted API preview.
  const plan = await previewAsset(page, daemon.base);
  await expect(plan).toContainText(TARGET_REL);

  // The external edit: the source no longer hashes to the registered digest.
  writeFileSync(join(daemon.project, SOURCE_REL), "edited outside the frozen preview\n");

  await page.getByRole("button", { name: zh.assetsCopy }).click();
  const refusal = page.getByTestId("assets-error");
  await expect(refusal).toContainText("assets.digest_mismatch");
  await expect(page.getByTestId("assets-copied")).toHaveCount(0);

  // Re-preview is refused identically — the fix is to re-register the asset,
  // not to retry the copy.
  await page.getByRole("button", { name: zh.assetsPreview }).click();
  await expect(page.getByTestId("assets-error")).toContainText("assets.digest_mismatch");
  await expect(page.getByTestId("assets-plan")).toHaveCount(0);
});


test("DEMO-07 target changes invalidate the visible plan; changing asset id clears it", async ({ page, daemon }, testInfo) => {
  const plan = await previewAsset(page, daemon.base);
  await expect(plan).toContainText(TARGET_REL);
  mkdirSync(join(daemon.project, ".ctxpect/skills"), { recursive: true });
  writeFileSync(join(daemon.project, TARGET_REL), "later user edit\n");
  await page.getByRole("button", { name: zh.assetsCopy }).click();
  await expect(page.getByTestId("assets-error")).toContainText("assets.concurrent_hash");
  expect(readFileSync(join(daemon.project, TARGET_REL), "utf8")).toBe("later user edit\n");
  await expect(page.getByRole("button", { name: zh.assetsCopy })).toBeDisabled();
  await page.screenshot({ path: testInfo.outputPath("assets-target-conflict.png"), fullPage: true });

  await page.getByRole("button", { name: zh.assetsPreview }).click();
  await expect(plan).toContainText("existing target content is replaced");
  await page.getByPlaceholder(zh.assetsIdPlaceholder).fill("another-asset");
  await expect(plan).toHaveCount(0);
  await expect(page.getByRole("button", { name: zh.assetsCopy })).toBeDisabled();
});

test.describe("authorized rollback", () => {
  test.use({ extraGrants: ["assets.rollback"] });
  test("DEMO-09 rollback preserves later edits and then links a new static Receipt", async ({ page, daemon }, testInfo) => {
    await expect(await previewAsset(page, daemon.base)).toContainText(TARGET_REL);
    await page.getByRole("button", { name: zh.assetsCopy }).click();
    await expect(page.getByTestId("assets-copied")).toBeVisible();
    const oldLink = await page.getByTestId("assets-verification").getByRole("link").getAttribute("href");
    const oldReceipt = await (await page.request.get(`${daemon.base}/api/v1${oldLink}`)).text();
    writeFileSync(join(daemon.project, TARGET_REL), "later edit\n");
    await page.getByRole("button", { name: zh.assetsRollback }).click();
    await expect(page.getByTestId("assets-error")).toContainText("assets.rollback_conflict");
    expect(readFileSync(join(daemon.project, TARGET_REL), "utf8")).toBe("later edit\n");
    await page.screenshot({ path: testInfo.outputPath("assets-rollback-conflict.png"), fullPage: true });
    writeFileSync(join(daemon.project, TARGET_REL), SEEDED_BYTES);
    await page.getByRole("button", { name: zh.assetsRollback }).click();
    await expect(page.getByTestId("assets-verification")).toContainText(zh.assetsRolledBack);
    await expect(page.getByTestId("assets-verification")).toContainText(zh.assetsRuntimeUnknown);
    expect(existsSync(join(daemon.project, TARGET_REL))).toBe(false);
    const newLink = await page.getByTestId("assets-verification").getByRole("link").getAttribute("href");
    expect(newLink).not.toBe(oldLink);
    expect(await (await page.request.get(`${daemon.base}/api/v1${oldLink}`)).text()).toBe(oldReceipt);
    const assets = await page.request.get(`${daemon.base}/api/v1/assets`);
  expect((await assets.json()).sbom.component_count).toBe(0);
  await page.screenshot({ path: testInfo.outputPath("assets-rollback-post-receipt.png"), fullPage: true });
  });
});
