// C04 per-page state contract.
//
// C04 requires every V01–V16 entry to declare which states apply and to give
// a reason for each one that does not. A page that renders all nine states
// regardless would be manufacturing states the backend cannot produce, which
// C04 names explicitly as the thing not to do.
//
// The rule used throughout: a state is applicable when this page's real data
// can actually reach it. Everything else carries a reason.

/** States every page reaches: each one fetches, so each can load or fail. */
const ALWAYS = ["loading", "ok", "empty", "error", "offline"];

// Reasons reused across pages, so the same judgement reads the same way.
const R = {
  readOnlyGet:
    "本页只发 GET。只读端点不经 mutation 授权，策略拒绝会出现在 /policy 的内容里，不是本页的错误状态。",
  noReceipt: "本页不读 Receipt 快照，没有可过期的观测。",
  noHarness: "本页不展示 harness family，版本与 connector 状态不在本页数据里。",
  noUnknownCells: "本页数据是 store 记录，没有 Unknown 真值单元。",
};

/**
 * @typedef {{id: string, route: string, applicable: string[], notApplicable: Record<string,string>}} PageContract
 */

/** @type {PageContract[]} */
export const PAGE_CONTRACTS = [
  {
    id: "V01",
    route: "/checkup",
    entry: "顶栏选择项目并点检查；也可从 Doctor 的下钻返回",
    back: "/doctor",
    selection: "无：本页不把选择写进 URL，观测对象是顶栏当前项目",
    query: [
      { method: "POST", path: "/api/v1/inspect", purpose: "产生当前 Receipt" },
      { method: "GET", path: "/api/v1/doctor", purpose: "取该 Receipt 的诊断" },
    ],
    actions: [],
    persistence: "Receipt 由 daemon 写入 store；本页自身不持久化任何编辑",
    sensitive: "项目路径经顶栏遮罩策略；本页只展示 policy_result，不展示指令正文",
    applicable: [...ALWAYS, "partial", "stale", "unsupported-version", "connector-missing"],
    notApplicable: { "permission-denied": R.readOnlyGet },
  },
  {
    id: "V02",
    route: "/inspector",
    entry: "从 Checkup 或 Doctor 下钻",
    back: "/doctor",
    selection: "无：facet 展开状态是会话内的，不进 URL",
    query: [
      { method: "POST", path: "/api/v1/inspect", purpose: "产生当前 Receipt" },
      { method: "GET", path: "/api/v1/doctor", purpose: "取该 Receipt 的诊断" },
    ],
    actions: [],
    persistence: "不持久化；六 facet 与 budget 直接来自当前 Receipt",
    sensitive: "证据正文默认遮罩，按住显示，截图模式下不可揭示",
    applicable: [...ALWAYS, "partial", "stale", "unsupported-version", "connector-missing"],
    notApplicable: { "permission-denied": R.readOnlyGet },
  },
  {
    id: "V03",
    route: "/compare",
    entry: "从导航或 Receipts 列表进入",
    back: "/receipts",
    selection: "两个 Receipt id 存在组件状态里；本切片未写进 URL，因此比较结果不可分享链接",
    query: [
      { method: "GET", path: "/api/v1/receipts", purpose: "列出可比较的 Receipt" },
      { method: "GET", path: "/api/v1/diff", purpose: "按 profile 比较两份" },
    ],
    actions: [
      { id: "diff", label: "比较", effect: "只读比较，不写 store", lands: "就地显示 same_domain / files_equal / baseline_allowed 与被忽略字段" },
    ],
    persistence: "不持久化；比较结果不落盘",
    sensitive: "只比较 Receipt 的结构与摘要，不展开证据正文",
    applicable: [...ALWAYS, "partial", "stale", "unsupported-version", "connector-missing"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
    },
  },
  {
    id: "V04",
    // Verify is read-only, but delete is a store mutation and can be refused.
    route: "/receipts",
    entry: "从导航进入列表，点条目进明细",
    back: "/receipts",
    selection: "明细页的 Receipt id 在 URL 路径里，可分享可刷新",
    query: [
      { method: "GET", path: "/api/v1/receipts", purpose: "列表" },
      { method: "GET", path: "/api/v1/receipts/:id", purpose: "明细" },
      { method: "POST", path: "/api/v1/receipts/:id/verify", purpose: "重算本地连续性 MAC" },
      { method: "POST", path: "/api/v1/receipts/:id/delete", purpose: "删除并留 tombstone" },
    ],
    actions: [
      { id: "verify", label: "验签", effect: "只读；重算 MAC 与存储记录比对", lands: "就地显示结果并标注 org_identity: false" },
      { id: "delete", label: "删除", effect: "破坏性；经唯一 authority，写 tombstone", lands: "显示 tombstone 并禁用本页两个动作" },
    ],
    persistence: "删除不可撤销：原记录被 tombstone 取代，同 id 不能重建",
    sensitive: "删除对话明确声明已导出的外部副本不可召回；本页不提供正文导出",
    applicable: [
      ...ALWAYS,
      "partial",
      "stale",
      "permission-denied",
      "unsupported-version",
      "connector-missing",
    ],
    notApplicable: {},
  },
  {
    id: "V05",
    route: "/assets",
    entry: "从导航或 Integrations 进入",
    back: "/integrations",
    selection: "asset_id 在组件状态里；来源与落点都由仓内登记决定，界面不选择路径",
    query: [
      { method: "GET", path: "/api/v1/assets", purpose: "catalog、lock 与 SBOM" },
      { method: "GET", path: "/api/v1/assets/:id", purpose: "单个 family" },
      { method: "POST", path: "/api/v1/assets/:id/preview", purpose: "供应链核验并给出复制计划，只读" },
      { method: "POST", path: "/api/v1/assets/:id/copy", purpose: "执行复制，经唯一 authority" },
      { method: "POST", path: "/api/v1/assets/:id/rollback", purpose: "回滚一次复制事务" },
    ],
    actions: [
      { id: "preview", label: "核验并预览", effect: "只读；核验登记、许可证与内容摘要", lands: "显示许可证、来源、落点与损失，未通过则给出拒绝的原因码" },
      { id: "copy", label: "复制到项目", effect: "写入项目内登记的落点，需已批准例外", lands: "显示事务 id、刷新 lock 与 SBOM，并出现回滚按钮" },
      { id: "rollback", label: "回滚这次复制", effect: "恢复先前字节，或删除本次新建的文件", lands: "清空计划与结果并重新读取 lock" },
    ],
    persistence: "复制写入项目内文件、store 内的 lock 条目与事务备份；备份保留因此可回滚",
    sensitive: "只展示登记里的来源标识与许可证，不展示资产正文；来源与落点均不可由界面指定",
    applicable: [...ALWAYS, "partial", "permission-denied", "unsupported-version", "connector-missing"],
    notApplicable: {
      stale: "catalog 来自冻结的 family 清单，不是 Receipt 观测。",
    },
  },
  {
    id: "V06",
    route: "/sessions",
    entry: "从导航进入列表，点条目进明细",
    back: "/sessions",
    selection: "会话 id 在 URL 路径里",
    query: [
      { method: "GET", path: "/api/v1/sessions", purpose: "列表" },
      { method: "GET", path: "/api/v1/sessions/:id", purpose: "明细：timeline（类型/seq/长度/digest）与 tail" },
      { method: "GET", path: "/api/v1/sessions/:id/requests", purpose: "请求证据：每次请求的 header digest、消息数、来源区间、替换区间、派发证据" },
    ],
    actions: [],
    persistence: "导入的会话以 metadata-only 形式存在 store（正文不落盘）；本页只读，导入与删除走 API 或 CLI；删除后 requests 与 insights 一并不可读",
    sensitive: "会话正文永不进入 store，本页只显示类型、seq、长度、digest 与区间；「人类可见历史」与「该次请求派生 surface」两个视图都不显示正文",
    applicable: [...ALWAYS, "partial"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      stale: R.noReceipt,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V06",
    route: "/monitor",
    entry: "从导航进入；也常从 stale 横幅的下一步指引跳来",
    back: "/checkup",
    selection: "无可选项：本页始终针对当前会话的那一份 Receipt",
    query: [
      { method: "GET", path: "/api/v1/monitor", purpose: "对当前 Receipt 的 evidence 重算摘要判定新鲜度" },
    ],
    actions: [],
    persistence: "不持久化；判定每次请求重算",
    sensitive: "只报告 evidence 的相对路径与是否变化，不读取也不显示其内容",
    // Monitor exists to report staleness; that is its main non-ok state.
    applicable: [...ALWAYS, "partial", "stale"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V07",
    route: "/lab",
    entry: "从导航进入列表，点条目进明细",
    back: "/lab",
    selection: "实验 id 在 URL 路径里",
    query: [
      { method: "GET", path: "/api/v1/lab", purpose: "实验列表" },
      { method: "GET", path: "/api/v1/lab/:id", purpose: "单次实验结果" },
    ],
    actions: [],
    persistence: "实验结果写入 store 且样本量锁定；本页只读，发起实验走 API 或 CLI（需 runs 文档）；无 runs 的实验显示 executed: false 与原因，不写入",
    sensitive: "只展示合同、逐 run 摘要（id/arm/outcome）与判定，不展示被测提示正文",
    applicable: [...ALWAYS, "partial"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      stale: R.noReceipt,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V08",
    // Apply writes a bundle into the store, so a refusal reaches this page.
    route: "/sync",
    entry: "从导航进入；本页是文件夹传输的唯一界面入口",
    back: "/checkup",
    selection: "bundle_id 在组件状态里；目标路径固定，不可由界面指定",
    query: [
      { method: "GET", path: "/api/v1/sync", purpose: "同步状态" },
      { method: "POST", path: "/api/v1/sync/preview", purpose: "预览，只读" },
      { method: "POST", path: "/api/v1/sync/apply", purpose: "应用，经唯一 authority" },
    ],
    actions: [
      { id: "preview", label: "预览", effect: "只读；检查冲突与重放", lands: "分列显示 transport 与 semantic" },
      { id: "apply", label: "应用", effect: "写入 <store>/sync/folder，需已批准例外", lands: "显示 transport success 与 semantic structural-only，并禁用重复应用" },
    ],
    persistence: "bundle 写入 store 内固定目录并记入 append-only 日志，重放被拒",
    sensitive: "bundle 只带 secret 的引用名，不带取值；带 secret_value 的 Receipt 会被拒绝打包",
    applicable: [...ALWAYS, "permission-denied"],
    notApplicable: {
      stale: R.noReceipt,
      partial: R.noUnknownCells,
      "unsupported-version": R.noHarness,
      "connector-missing":
        "E2EE 与远端传输都未实现，缺的是实现而不是 connector；如实报 sync.e2ee_unimplemented / sync.no_remote_transport。",
    },
  },
  {
    id: "V09",
    route: "/doctor",
    entry: "应用的默认落地页；从任何页面点导航第一项回到这里",
    back: "/doctor",
    selection: "选中的 finding 在组件状态里；本切片未写进 URL",
    query: [
      { method: "POST", path: "/api/v1/inspect", purpose: "产生当前 Receipt" },
      { method: "GET", path: "/api/v1/doctor", purpose: "诊断" },
      { method: "POST", path: "/api/v1/collect", purpose: "收集下一条证据" },
    ],
    actions: [
      { id: "diagnose", label: "诊断", effect: "产生新 Receipt 并诊断", lands: "刷新发现列表与抽屉" },
      { id: "collect", label: "收集证据", effect: "按 finding 收集其声明的下一条证据", lands: "抽屉内就地显示结果；期间冻结选中" },
    ],
    persistence: "每次诊断写入一份新 Receipt；抽屉内无待保存的编辑",
    sensitive: "证据正文默认遮罩；抽屉不展示未经授权的 home 或会话内容",
    // Doctor runs inspect and recheck, so a refusal can reach this page.
    applicable: [
      ...ALWAYS,
      "partial",
      "stale",
      "permission-denied",
      "unsupported-version",
      "connector-missing",
    ],
    notApplicable: {},
  },
  {
    id: "V10",
    route: "/policy",
    entry: "从导航进入，或从被拒动作的下一步指引进入",
    back: "/doctor",
    selection: "无可选项：本页展示当前生效的五层叠加结果",
    query: [
      { method: "GET", path: "/api/v1/policy", purpose: "五层有效 policy 与 mutation 判定" },
    ],
    actions: [],
    persistence: "不持久化；判定每次读取重算",
    sensitive: "只展示规则与判定，不展示规则来源文件的路径",
    applicable: [...ALWAYS, "partial"],
    notApplicable: {
      "permission-denied":
        "策略拒绝是本页要展示的内容，不是获取本页的失败；deny 以 verdict 呈现，不折叠成错误状态。",
      stale: R.noReceipt,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V11",
    route: "/standards",
    entry: "从导航进入列表，点条目进明细",
    back: "/standards",
    selection: "standard id 在 URL 路径里",
    query: [
      { method: "GET", path: "/api/v1/standards", purpose: "列表" },
      { method: "GET", path: "/api/v1/standards/:id", purpose: "明细" },
    ],
    actions: [],
    persistence: "采纳状态写在 store；本页当前只读，发布与采纳走 CLI",
    sensitive: "只展示签名状态与摘要，不展示 standard 正文",
    applicable: [...ALWAYS, "partial"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      stale: R.noReceipt,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V12",
    route: "/settings",
    entry: "从导航进入；也从隐私或保留期相关提示跳来",
    back: "/doctor",
    selection: "无：草稿是会话内状态，刷新即丢弃",
    query: [
      { method: "GET", path: "/api/v1/settings/schema", purpose: "字段规格，编辑器据此生成" },
      { method: "GET", path: "/api/v1/settings", purpose: "当前值" },
      { method: "POST", path: "/api/v1/settings", purpose: "整档替换，经唯一 authority" },
    ],
    actions: [
      { id: "save", label: "保存", effect: "整档替换，store 侧再验证一次", lands: "刷新已保存值并提示已保存" },
      { id: "revert", label: "撤销改动", effect: "仅回退本地草稿，不发请求", lands: "草稿回到已保存值" },
    ],
    persistence: "保存后写入 store 的 settings.json 并进审计链；未保存的草稿不落盘，离开页面即丢失",
    sensitive: "本页不显示密钥材料；vault 只显示模式，不显示任何密钥",
    // Settings writes, so a refused save lands here.
    applicable: [...ALWAYS, "permission-denied"],
    notApplicable: {
      stale: R.noReceipt,
      partial: R.noUnknownCells,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V13",
    route: "/exceptions",
    entry: "从导航进入，或从 permission-denied 的下一步指引进入",
    back: "/policy",
    selection: "无可选项：本页列出该 store 内的全部例外",
    query: [
      { method: "GET", path: "/api/v1/exceptions", purpose: "例外列表" },
      { method: "GET", path: "/api/v1/exceptions/:id", purpose: "单条例外的生命周期状态与是否放行" },
    ],
    actions: [],
    persistence: "例外记录在 store；申请与批准需要已登记 principal 的密钥，只能走 CLI",
    sensitive: "不显示任何密钥材料；决策记录只含 principal id 与角色",
    applicable: [...ALWAYS, "permission-denied"],
    notApplicable: {
      stale: R.noReceipt,
      partial: R.noUnknownCells,
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V14",
    route: "/team/compliance",
    entry: "从导航进入；面向需要看合规汇总的人",
    back: "/standards",
    selection: "无可选项：统计范围固定为本地 store",
    query: [
      { method: "GET", path: "/api/v1/team/compliance", purpose: "本地 store 的合规统计" },
    ],
    actions: [],
    persistence: "不持久化；每次读取重新验签与统计",
    sensitive: "只出元数据统计，不含任何成员的 Receipt 正文",
    applicable: [...ALWAYS, "partial"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      stale:
        "freshness 只报 current-session-receipt 或 unknown；没有当前 Receipt 时是 partial，不是 stale。",
      "unsupported-version": R.noHarness,
      "connector-missing": R.noHarness,
    },
  },
  {
    id: "V15",
    route: "/care-plan/:findingId",
    entry: "从 Doctor 的 finding 下钻",
    back: "/doctor",
    selection: "finding id 在 URL 路径里，可分享可刷新",
    query: [
      { method: "GET", path: "/api/v1/care-plan/:id", purpose: "该 finding 自身的处置与落点" },
    ],
    actions: [],
    persistence: "不持久化；计划是预览，应用走 apply/rollback",
    sensitive: "只展示该 finding 已声明的目标与 authority，不展开目标文件正文",
    // A care plan is applied against policy, so a refusal reaches this page.
    applicable: [...ALWAYS, "partial", "stale", "permission-denied"],
    notApplicable: {
      "unsupported-version":
        "版本不受支持会表现为该 finding 的 lock_reason，由计划正文说明，不折叠成页面状态。",
      "connector-missing": "同上：connector 缺失是该 finding 的锁定理由，不是本页状态。",
    },
  },
  {
    id: "V16",
    route: "/integrations",
    entry: "从导航进入列表，点条目进明细",
    back: "/integrations",
    selection: "family id 在 URL 路径里",
    query: [
      { method: "GET", path: "/api/v1/integrations", purpose: "family 覆盖" },
      { method: "GET", path: "/api/v1/integrations/:id", purpose: "单个 family" },
    ],
    actions: [],
    persistence: "不持久化；catalog 来自冻结的 family 清单",
    sensitive: "只展示 family 与 capability 元数据，不做安装探测",
    applicable: [...ALWAYS, "partial", "unsupported-version", "connector-missing"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      stale: "catalog 来自冻结的 family 清单，不是 Receipt 观测。",
    },
  },
];

/** Look up a contract by route path. Detail routes inherit their list route. */
export function contractFor(route) {
  const exact = PAGE_CONTRACTS.find((item) => item.route === route);
  if (exact) return exact;
  const base = `/${String(route).split("/").filter(Boolean)[0] ?? ""}`;
  return PAGE_CONTRACTS.find((item) => item.route === base) ?? null;
}

/** Whether `state` is declared applicable on `route`. */
export function stateApplies(route, state) {
  const contract = contractFor(route);
  if (!contract) return true;
  return contract.applicable.includes(state);
}
