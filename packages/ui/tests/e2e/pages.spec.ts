// Interaction E2E against a real daemon (see global-setup.ts). These cover
// what the SSR render tests cannot: data flow, navigation between ids, the
// generation guard's visible effect, and a real inspect round trip.
import { expect, test } from "@playwright/test";

const base = () => process.env.E2E_BASE_URL ?? "";
const project = () => process.env.E2E_PROJECT ?? "";

test.beforeEach(({}, testInfo) => {
  testInfo.setTimeout(30_000);
  expect(base(), "global setup exported E2E_BASE_URL").not.toBe("");
});

test("the session page fetches request evidence and reacts to selecting a request", async ({ page }) => {
  await page.goto(`${base()}/sessions/s-alpha`);
  const rows = page.locator('[data-testid="request-table"] tbody tr');
  await expect(rows).toHaveCount(4);
  await expect(rows.nth(0).locator("[data-dispatch]")).toHaveText(/seq 6/);
  await expect(page.locator('[data-metadata-only="true"]')).toHaveCount(1);
  await rows.nth(1).getByRole("button").click();
  await expect(page.locator('[data-metadata-only="true"]')).toHaveCount(2);
  await expect(page.locator('[data-metadata-only="true"] caption').nth(1)).toContainText("seq 13");
  await expect(rows.nth(1)).toHaveAttribute("data-selected", "true");
  // Nothing of the session bodies reaches the page.
  const text = await page.locator("main").innerText();
  expect(text).not.toContain("List the files");
  expect(text).not.toContain("/tmp/ctxpect-fixture");
  await expect(page.locator('[data-testid="session-tail"]')).toContainText("TOOL_OUTCOME_UNKNOWN");
});

test("navigating between sessions replaces the answer; an older error does not linger", async ({ page }) => {
  await page.goto(`${base()}/sessions/s-missing`);
  await expect(page.locator('[data-state="error"]')).toContainText("store.missing");
  await page.goto(`${base()}/sessions/s-beta`);
  await expect(page.locator("h1")).toContainText("s-beta");
  await expect(page.locator('[data-testid="request-table"] tbody tr')).toHaveCount(4);
  await expect(page.locator('[data-state="error"]')).toHaveCount(0);
  await page.goto(`${base()}/sessions/s-alpha`);
  await expect(page.locator("h1")).toContainText("s-alpha");
  await expect(page.locator('[data-testid="request-table"] tbody tr')).toHaveCount(4);
});

test("a late answer for the session the user left is dropped, not applied", async ({ page }) => {
  // s-alpha's request evidence is held back; the user moves on to a session
  // that does not exist before it arrives. The error for s-missing must
  // stay, and s-alpha's table must never appear over it.
  await page.route("**/api/v1/sessions/s-alpha/requests", async (route) => {
    await new Promise((r) => setTimeout(r, 1500));
    await route.continue();
  });
  await page.goto(`${base()}/sessions/s-alpha`);
  await expect(page.locator("h1")).toContainText("s-alpha");
  // Client-side navigation (the router listens to popstate), so the s-alpha
  // request stays in flight in the same document.
  await page.evaluate(() => {
    window.history.pushState({}, "", "/sessions/s-missing");
    window.dispatchEvent(new PopStateEvent("popstate"));
  });
  await expect(page.locator('[data-state="error"]')).toContainText("store.missing");
  await page.waitForTimeout(2500);
  await expect(page.locator('[data-state="error"]')).toContainText("store.missing");
  await expect(page.locator('[data-testid="request-table"]')).toHaveCount(0);
});

test("the lab list and detail show execution state and the frozen estimator's verdict", async ({ page }) => {
  await page.goto(`${base()}/lab`);
  const row = page.locator('tr[data-executed="true"]');
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("e-lab");
  await expect(row).toContainText("inconclusive");
  await row.getByRole("link").click();
  await expect(page).toHaveURL(/\/lab\/e-lab$/);
  const result = page.locator('[data-testid="lab-result"]');
  await expect(result).toHaveAttribute("data-executed", "true");
  await expect(result).toContainText("inconclusive");
  await expect(result).toContainText("paired-exact-binomial-v2");
});

test("checkup runs a real inspect through the daemon and renders its policy result", async ({ page }) => {
  await page.goto(`${base()}/checkup`);
  const input = page.getByPlaceholder("<project>");
  await input.fill(project());
  await page.getByRole("button", { name: "检查" }).click();
  await expect(page.locator("main")).toContainText("verdict", { timeout: 15_000 });
  await expect(page.locator('[data-state="error"]')).toHaveCount(0);
});
