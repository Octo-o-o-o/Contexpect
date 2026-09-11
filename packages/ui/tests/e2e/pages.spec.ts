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

test("cold Doctor shows unmeasured counts and the actual daemon address", async ({ page }) => {
  await page.goto(`${base()}/doctor`);
  await expect(page.locator('[data-testid="startup-coordinate"]')).toContainText("none");
  await expect(page.locator(".stat-num")).toHaveText(["—", "—", "—"]);
  await expect(page.locator(".shell-foot .daemon")).toHaveText(new URL(base()).host);
  await expect(page.locator('[data-testid="startup-coordinate"]')).not.toContainText(project());
});

test("command palette contains focus and Escape returns it to the opener", async ({ page }) => {
  await page.goto(`${base()}/doctor`);
  const opener = page.getByRole("button", { name: "命令面板" });
  await opener.click();
  const modal = page.getByRole("dialog", { name: "命令面板" });
  await expect(modal).toBeVisible();
  await page.keyboard.press("Shift+Tab");
  expect(await modal.evaluate((el) => el.contains(document.activeElement))).toBe(true);
  await page.keyboard.press("Escape");
  await expect(modal).toHaveCount(0);
  await expect(opener).toBeFocused();
  await page.keyboard.press("Control+k");
  await modal.getByRole("textbox").fill("会话");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/sessions$/);
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

test("cancel stops a self-fetching page's request and says so", async ({ page }) => {
  // The receipts list is held back; the user cancels while it loads.
  await page.route("**/api/v1/receipts", async (route) => {
    await new Promise((r) => setTimeout(r, 5000));
    await route.continue();
  });
  await page.goto(`${base()}/receipts`);
  await expect(page.locator('[data-state="loading"]')).toHaveCount(1);
  await page.getByRole("button", { name: "取消" }).first().click();
  await expect(page.locator('[data-state="cancelled"]')).toHaveCount(1);
  await expect(page.locator('[data-state="cancelled"]')).toContainText("api.cancelled");
  // The cancelled request never lands on the page.
  await page.waitForTimeout(1000);
  await expect(page.locator('[data-state="cancelled"]')).toHaveCount(1);
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


test("session summaries show metadata while keeping the detail links", async ({ page }) => {
  await page.goto(`${base()}/sessions`);
  const table = page.getByTestId("session-list");
  await expect(table).toContainText("deepseek-harness-cli");
  await expect(table.getByRole("link", { name: "s-alpha", exact: true })).toHaveAttribute("href", "/sessions/s-alpha");
  await expect(table.locator("tbody tr").first().locator("td").nth(4)).toHaveText("false");
  await expect(table).not.toContainText("List the files");
});

test("reloading all diagnostic routes restores one explicit Receipt", async ({ page, request }) => {
  const response = await request.post(`${base()}/api/v1/inspect`, {
    headers: { "X-Ctxpect-Client": "desktop" }, data: {},
  });
  expect(response.ok()).toBe(true);
  const { receipt } = await response.json();
  for (const path of ["/doctor", "/checkup", "/inspector"]) {
    await page.goto(`${base()}${path}`);
    await expect(page.getByTestId("startup-coordinate")).toContainText(receipt.receipt_id);
    await expect(page.locator('[data-state="error"]')).toHaveCount(0);
  }
  await page.goto(`${base()}/doctor`);
  await expect(page.locator(".stat-num").first()).not.toHaveText("—");
  await expect(page.locator(".chain-glyph")).toHaveCount(0);
  await expect(page.getByTestId("evidence-facts")).toBeVisible();
});

test("a delayed bootstrap cannot overwrite a newer manual inspect", async ({ page }) => {
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let captured!: () => void;
  const ready = new Promise<void>((resolve) => { captured = resolve; });
  await page.route("**/api/v1/status", async (route) => {
    const response = await route.fetch();
    captured();
    await held;
    await route.fulfill({ response }).catch(() => undefined);
  });
  await page.goto(`${base()}/checkup`);
  await ready;
  await page.getByPlaceholder("<project>").fill(project());
  const inspecting = page.waitForResponse((r) => r.url().endsWith("/api/v1/inspect"));
  await page.getByRole("button", { name: "检查", exact: true }).click();
  const receipt = (await (await inspecting).json()).receipt;
  await expect(page.getByTestId("startup-coordinate")).toContainText(receipt.receipt_id);
  release();
  await page.waitForTimeout(150);
  await expect(page.getByTestId("startup-coordinate")).toContainText(receipt.receipt_id);
  await expect(page.locator('[data-state="error"]')).toHaveCount(0);
});


test("diagnostic layouts stay reachable at desktop, medium and narrow widths", async ({ page }, testInfo) => {
  for (const width of [1440, 1100, 820, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`${base()}/doctor`);
    if (width >= 768) {
      await expect(page.locator(".stat-num").first()).not.toHaveText("—");
      await expect(page.getByRole("button", { name: "检查", exact: true })).toBeVisible();
      const contained = await page.locator(".topbar").evaluate((bar) => {
        const outer = bar.getBoundingClientRect();
        return Array.from(bar.querySelectorAll("button, input, select")).every((el) => {
          const box = el.getBoundingClientRect();
          return box.top >= outer.top && box.bottom <= outer.bottom && box.right <= outer.right;
        });
      });
      expect(contained, `topbar controls at ${width}`).toBe(true);
    } else {
      await expect(page.locator(".narrow")).toBeVisible();
      await expect(page.getByRole("button", { name: "检查", exact: true })).toHaveCount(0);
    }
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
    expect(overflow, `horizontal overflow at ${width}`).toBe(false);
    await page.screenshot({ path: testInfo.outputPath(`doctor-${width}.png`), fullPage: true });
  }
});
