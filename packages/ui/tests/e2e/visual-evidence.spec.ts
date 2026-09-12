// U06 visual evidence: real screenshots of the white-monochrome theme against
// the real daemon + seed (see global-setup.ts), following the capture contract
// in docs/handoff/contexpect-2026-09-11/07_ROUTES_STATES_AND_JOURNEYS.md §6 —
// files are named `route__state__viewport__locale.png`, and only states that
// were actually rendered are reported as captured.
//
// Shared-daemon constraints (workers=1, files run alphabetically, so this file
// runs after pages.spec.ts):
//   - The cold-start assertions in pages.spec.ts have already run; the Receipts
//     they created are still in the store and are what /doctor restores here.
//   - This file is the only one that mutates Receipt state: it tombstones one
//     non-selected Receipt in its last test, after compare/care-plan are done.
//   - The receipt.delete exception is planted into the scratch store mid-run
//     (same shape as global-setup's plantGrants) so both halves of the delete
//     gate — refusal without a grant, then a granted tombstone — are real
//     daemon decisions, not page fakes.
//   - settings__offline uses page.route aborts: the fetch genuinely fails
//     (kind "transport"), which is the same condition the app cannot
//     distinguish from a dead daemon. The daemon itself stays up.
//
// Screenshots land in tests/.e2e-out/evidence/ (gitignored, like the rest of
// .e2e-out); the run also writes manifest.json there with a sha256 per file.
// docs/process/2026-09-12-visual-evidence.md is rendered from that manifest.
import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { zh } from "../../src/i18n-tables.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../../..");
const evidenceDir = join(here, "../.e2e-out/evidence");

const base = () => process.env.E2E_BASE_URL ?? "";
const store = () => process.env.E2E_STORE ?? "";
const project = () => process.env.E2E_PROJECT ?? "";

const DESKTOP = { width: 1440, height: 900 };
const NARROW = { width: 390, height: 844 };

type Entry = {
  file: string;
  route: string;
  state: string;
  viewport: string;
  locale: string;
  sha256: string;
  proves: string;
  doesNotProve: string;
};

const entries: Entry[] = [];
let browserVersion = "";

/** Full-page capture under the contract name, hashed into the manifest. */
async function snap(
  page: import("@playwright/test").Page,
  route: string,
  state: string,
  viewport: "desktop" | "narrow",
  locale: "zh" | "en",
  proves: string,
  doesNotProve: string,
): Promise<void> {
  const file = `${route}__${state}__${viewport}__${locale}.png`;
  const path = join(evidenceDir, file);
  await page.screenshot({ path, fullPage: true });
  const sha256 = createHash("sha256").update(readFileSync(path)).digest("hex");
  entries.push({ file, route, state, viewport, locale, sha256, proves, doesNotProve });
}

const NOT_WEBVIEW =
  "不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为";

test.beforeAll(() => {
  mkdirSync(evidenceDir, { recursive: true });
});

test.beforeEach(async ({ page }, testInfo) => {
  testInfo.setTimeout(60_000);
  expect(base(), "global setup exported E2E_BASE_URL").not.toBe("");
  await page.setViewportSize(DESKTOP);
});

test.afterAll(() => {
  const commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], {
    cwd: repoRoot,
    encoding: "utf8",
  }).trim();
  const dirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot, encoding: "utf8" })
    .trim().length > 0;
  writeFileSync(
    join(evidenceDir, "manifest.json"),
    JSON.stringify(
      {
        captured_at: new Date().toISOString(),
        build_commit: `${commit}${dirty ? " (工作区含未提交改动)" : ""}`,
        browser: `Chromium ${browserVersion} (headless, Playwright)`,
        fixture:
          "global-setup 临时目录种子：合成 project（AGENTS.md、登记的 skill-e2e 资产）、s-alpha/s-beta 两条 deepseek-harness-cli native 会话（acceptance/corpus/development/native，live_tested: false）、e-lab 已执行实验；Receipt 为本跑次真实 inspect 产物（pages.spec.ts 一次；本 spec 改写 AGENTS.md 后再次 inspect 取得第二份内容寻址 Receipt）",
        screenshots_dir: "packages/ui/tests/.e2e-out/evidence/（gitignored）",
        entries,
      },
      null,
      2,
    ),
  );
});

test("doctor restores the run's Receipt as a partial diagnosis, in zh and en, and degrades to the narrow read-only surface", async ({
  page,
  browser,
}) => {
  browserVersion = browser.version();
  await page.goto(`${base()}/doctor`);
  // The restored Receipt's counts replace the cold "—" placeholders.
  await expect(page.locator(".stat-num").first()).not.toHaveText("—");
  // Indeterminate facet cells on the synthetic fixture keep the verdict partial.
  await expect(page.locator('[data-state="partial"]')).toBeVisible();
  await expect(page.getByTestId("evidence-facts")).toBeVisible();
  await snap(
    page,
    "doctor",
    "partial",
    "desktop",
    "zh",
    "白底黑字主题在 /doctor 实机渲染：统计带、findings 表、证据抽屉与 partial 状态横幅同时可读，中文界面",
    NOT_WEBVIEW,
  );

  await page.selectOption('select[aria-label="locale"]', "en");
  await expect(page.locator(".sb-title")).toContainText("Partly unknown");
  await snap(
    page,
    "doctor",
    "partial",
    "desktop",
    "en",
    "同一状态在英文 locale 下渲染，i18n 切换无布局破坏",
    NOT_WEBVIEW,
  );
  await page.selectOption('select[aria-label="locale"]', "zh-CN");

  await page.setViewportSize(NARROW);
  await expect(page.locator(".narrow")).toBeVisible();
  await snap(
    page,
    "doctor",
    "narrow-readonly",
    "narrow",
    "zh",
    "窄视口（390px）按 C06 渲染只读窄屏面而不是残缺全壳，白底黑字主题生效",
    `${NOT_WEBVIEW}；窄屏面是路由无关的，此图代表全部路由的窄屏行为`,
  );
  await page.setViewportSize(DESKTOP);
});

test("inspector expands the six facets of the restored Receipt", async ({ page }) => {
  await page.goto(`${base()}/inspector`);
  await expect(page.locator(".facet-grid").first().locator(".facet")).toHaveCount(6);
  await snap(
    page,
    "inspector",
    "six-facets",
    "desktop",
    "zh",
    "/inspector 六面（installed/discoverable/eligible/model-visible/use-evidence/outcome-affecting）与 provenance/coverage/precision/knowledge_status 四轴实机渲染",
    `${NOT_WEBVIEW}； facet 真值多为 unknown 是种子 fixture 的事实，不是主题缺陷`,
  );
});

test("compare diffs two real Receipts from this run", async ({ page }) => {
  // Receipt ids are content-addressed over the inspect snapshot, and the
  // snapshot does not cover every file: adding a marker file leaves the id
  // unchanged. Changing the seeded AGENTS.md — a file the Doctor content
  // rules actually read — produces a genuinely different observation.
  writeFileSync(join(project(), "AGENTS.md"), "hello from e2e\nu06 second-snapshot marker\n");
  const inspected = await page.request.post(`${base()}/api/v1/inspect`, {
    headers: { "X-Ctxpect-Client": "desktop" },
    data: {},
  });
  expect(inspected.ok()).toBe(true);
  const listed = await page.request.get(`${base()}/api/v1/receipts`);
  const ids = ((await listed.json()) as { receipts: { receipt_id: string }[] }).receipts
    .map((row) => row.receipt_id)
    .filter((id) => id && !id.includes("tombstone"));
  expect(ids.length).toBeGreaterThanOrEqual(2);
  await page.goto(`${base()}/compare`);
  await page.locator('select[aria-label="diff-a"]').selectOption(ids[0]);
  await page.locator('select[aria-label="diff-b"]').selectOption(ids[1]);
  await page.getByRole("button", { name: zh.runDiff }).click();
  await expect(page.locator("pre.mono")).toContainText("same_domain");
  await snap(
    page,
    "compare",
    "diff",
    "desktop",
    "zh",
    "/compare 对两份真实 Receipt 跑出结构化 diff 并渲染 same_domain 等字段",
    `${NOT_WEBVIEW}；「不可比较」（same_domain=false）需要不同 kind/坐标的两份 Receipt，本 fixture 只有同项目同类 inspect Receipt，未拍到`,
  );
});

test("receipts list shows the run's history", async ({ page }) => {
  await page.goto(`${base()}/receipts`);
  await expect(page.locator("main")).toContainText("receipt", { ignoreCase: true });
  await page.waitForLoadState("networkidle");
  await snap(
    page,
    "receipts",
    "list",
    "desktop",
    "zh",
    "/receipts 历史列表实机渲染本跑次 inspect 产生的 Receipt 行",
    NOT_WEBVIEW,
  );
});

test("care plan for a real finding renders as a locked preview", async ({ page }) => {
  const status = await (await page.request.get(`${base()}/api/v1/status`)).json();
  const receiptId = (status as { selected_receipt: { receipt_id: string } }).selected_receipt
    .receipt_id;
  const doctor = await (
    await page.request.get(`${base()}/api/v1/doctor?receipt_id=${encodeURIComponent(receiptId)}`)
  ).json();
  const findingId = (doctor as { findings: { finding_id: string }[] }).findings[0].finding_id;
  await page.goto(`${base()}/care-plan/${encodeURIComponent(findingId)}`);
  await expect(page.locator("main")).toContainText("preview_required");
  await expect(page.locator("main")).toContainText(zh.carePlanLocked);
  await snap(
    page,
    "care-plan",
    "preview",
    "desktop",
    "zh",
    "/care-plan/:findingId 对真实 finding 渲染冻结预览（treatment_locked、preview_required: true）",
    `${NOT_WEBVIEW}；摘要冲突与写入后待补证在本切片没有可达的写入路径（页面声明 carePlanLocked），未拍到`,
  );
});

test("advisor without both confirmations sends nothing and shows the blocked state", async ({
  page,
}) => {
  await page.goto(`${base()}/advisor`);
  await expect(page.getByTestId("advisor-preview")).toContainText("local-heuristic-not-llm");
  await expect(page.getByRole("button", { name: zh.advisorSend })).toBeDisabled();
  await expect(page.locator("main")).toContainText(zh.advisorBlocked);
  await snap(
    page,
    "advisor",
    "blocked",
    "desktop",
    "zh",
    "/advisor 未授权态：发送按钮禁用、预览与阻止说明实机渲染（advisor.spec.ts 另证明此态零请求）",
    NOT_WEBVIEW,
  );
});

test("lab list and the executed experiment detail", async ({ page }) => {
  await page.goto(`${base()}/lab`);
  await expect(page.locator('tr[data-executed="true"]')).toHaveCount(1);
  await snap(
    page,
    "lab",
    "list",
    "desktop",
    "zh",
    "/lab 列表实机渲染",
    NOT_WEBVIEW,
  );
  await page.goto(`${base()}/lab/e-lab`);
  await expect(page.getByTestId("lab-result")).toHaveAttribute("data-executed", "true");
  await snap(
    page,
    "lab",
    "executed",
    "desktop",
    "zh",
    "/lab/e-lab 已执行实验的判定（inconclusive / paired-exact-binomial-v2）实机渲染",
    `${NOT_WEBVIEW}；「未执行」态的 POST 应答不落盘，/lab/:id 只服务已持久化（即已执行）实验，未拍到`,
  );
});

test("sync preview renders, and apply without a grant is refused by the daemon", async ({
  page,
}) => {
  await page.goto(`${base()}/sync`);
  await page.getByRole("button", { name: zh.syncPreview }).click();
  await expect(page.getByTestId("sync-outcome")).toBeVisible();
  await snap(
    page,
    "sync",
    "preview",
    "desktop",
    "zh",
    "/sync 对空目标文件夹的只读预览分列 transport 与 semantic，并标注 transport 未核验",
    `${NOT_WEBVIEW}；分叉（conflict）需要 <store>/sync/folder 里已有另一 bundle，种子fixture无法经 API 制造，未拍到`,
  );
  await page.getByRole("button", { name: zh.syncApply }).click();
  await expect(page.getByTestId("sync-error")).toContainText("policy.approval_required");
  await snap(
    page,
    "sync",
    "apply-refused",
    "desktop",
    "zh",
    "无 sync.apply 例外时应用被 daemon 拒绝，页面如实显示 policy.approval_required",
    NOT_WEBVIEW,
  );
});

test("settings renders its offline state when requests fail, and recovers on retry", async ({
  page,
}) => {
  await page.route("**/api/v1/settings**", (route) => void route.abort());
  await page.goto(`${base()}/settings`);
  await expect(page.locator('[data-state="offline"]')).toBeVisible();
  await snap(
    page,
    "settings",
    "offline",
    "desktop",
    "zh",
    "/settings 在 transport 失败时渲染 offline 态横幅与重试入口",
    `${NOT_WEBVIEW}；失败由 page.route abort 注入——对页面而言与 daemon 宕机同为 transport 失败，但 daemon 实际未停`,
  );
  await page.unroute("**/api/v1/settings**");
  await page.getByRole("button", { name: zh.retry }).click();
  await expect(page.locator(".settings-form")).toBeVisible();
  await snap(
    page,
    "settings",
    "ok",
    "desktop",
    "zh",
    "重试后 /settings 表单从 schema 生成并渲染（离线恢复回路）",
    NOT_WEBVIEW,
  );
});

test("the remaining read routes render under the theme", async ({ page }) => {
  const shots: [string, string][] = [
    ["/checkup", "checkup"],
    ["/assets", "assets"],
    ["/sessions", "sessions"],
    ["/monitor", "monitor"],
    ["/integrations", "integrations"],
    ["/policy", "policy"],
    ["/standards", "standards"],
    ["/exceptions", "exceptions"],
    ["/team/compliance", "team-compliance"],
  ];
  for (const [path, name] of shots) {
    await page.goto(`${base()}${path}`);
    await page.waitForLoadState("networkidle");
    await expect(page.locator("h1").first()).toBeVisible();
    await expect(page.locator('[data-state="loading"]')).toHaveCount(0);
    await expect(page.locator('[data-state="error"]')).toHaveCount(0);
    await snap(
      page,
      name,
      "ok",
      "desktop",
      "zh",
      `白底黑字主题在 ${path} 实机渲染，内容加载完成且无错误态`,
      NOT_WEBVIEW,
    );
  }
  await page.goto(`${base()}/sessions/s-alpha`);
  await expect(page.locator('[data-testid="request-table"] tbody tr')).toHaveCount(4);
  await snap(
    page,
    "sessions",
    "detail",
    "desktop",
    "zh",
    "/sessions/s-alpha 请求证据表（metadata-only，正文不进页面）实机渲染",
    NOT_WEBVIEW,
  );
});

test("receipt delete: refused without a grant, tombstoned once the exception exists", async ({
  page,
}) => {
  // Pick a Receipt that is not the session-selected one, so the tombstone
  // cannot change what /doctor restores.
  const status = await (await page.request.get(`${base()}/api/v1/status`)).json();
  const selectedId = (status as { selected_receipt: { receipt_id: string } }).selected_receipt
    .receipt_id;
  const listed = await page.request.get(`${base()}/api/v1/receipts`);
  const ids = ((await listed.json()) as { receipts: { receipt_id: string }[] }).receipts.map(
    (row) => row.receipt_id,
  );
  const target = ids.find((id) => id !== selectedId);
  expect(target, "a non-selected Receipt exists (pages.spec.ts ran before this file)").toBeTruthy();

  await page.goto(`${base()}/receipts/${target}`);
  await expect(page.locator(".docpage .kv")).toBeVisible();
  await page.getByRole("button", { name: zh.receiptDelete }).first().click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText(zh.receiptDeleteExternal);
  await snap(
    page,
    "receipts",
    "delete-confirm",
    "desktop",
    "zh",
    "删除前的后果对话（tombstone/派生失效/外部副本不可召回）实机渲染",
    NOT_WEBVIEW,
  );
  await dialog.getByRole("button", { name: zh.receiptDelete }).click();
  await expect(page.getByTestId("delete-error")).toContainText("policy.approval_required");
  await snap(
    page,
    "receipts",
    "delete-refused",
    "desktop",
    "zh",
    "无 receipt.delete 例外时删除被 daemon 拒绝，页面如实显示 policy.approval_required 且不产生 tombstone",
    NOT_WEBVIEW,
  );

  // Plant the grant the way global-setup plants its exceptions; the policy
  // decision is re-computed per request, so it takes effect immediately.
  const digest = createHash("sha256")
    .update(`project:${realpathSync(project())}`, "utf8")
    .digest("hex");
  writeFileSync(
    join(store(), "exceptions/ex-e2e-receipt-delete.json"),
    JSON.stringify({
      exception_id: "ex-e2e-receipt-delete",
      requester: "operator",
      action: "receipt.delete",
      project_digest: digest,
      target: "*",
      state: "approved",
      created_at: 1,
      expires_at: 4102444800,
      reason: "e2e grant",
      approver: "enrolled-out-of-band",
    }),
  );
  await page.getByRole("button", { name: zh.receiptDelete }).first().click();
  await page.getByRole("dialog").getByRole("button", { name: zh.receiptDelete }).click();
  await expect(page.getByTestId("delete-result")).toContainText(zh.receiptDeleted);
  await snap(
    page,
    "receipts",
    "deleted",
    "desktop",
    "zh",
    "有已批准例外时删除落 tombstone，页面显示删除回执并禁用后续动作",
    NOT_WEBVIEW,
  );
  const after = await (
    await page.request.get(`${base()}/api/v1/receipts`)
  ).json() as { receipts: { receipt_id: string; tombstone?: boolean }[] };
  const tombstoned = after.receipts.find((row) => row.receipt_id === target);
  expect(tombstoned?.tombstone).toBe(true);
});
