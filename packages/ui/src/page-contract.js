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
    applicable: [...ALWAYS, "partial", "stale", "unsupported-version", "connector-missing"],
    notApplicable: { "permission-denied": R.readOnlyGet },
  },
  {
    id: "V02",
    route: "/inspector",
    applicable: [...ALWAYS, "partial", "stale", "unsupported-version", "connector-missing"],
    notApplicable: { "permission-denied": R.readOnlyGet },
  },
  {
    id: "V03",
    route: "/compare",
    applicable: [...ALWAYS, "partial", "stale", "unsupported-version", "connector-missing"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
    },
  },
  {
    id: "V04",
    // Verify is read-only, but delete is a store mutation and can be refused.
    route: "/receipts",
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
    applicable: [...ALWAYS, "partial", "unsupported-version", "connector-missing"],
    notApplicable: {
      "permission-denied": R.readOnlyGet,
      stale: "catalog 来自冻结的 family 清单，不是 Receipt 观测。",
    },
  },
  {
    id: "V06",
    route: "/sessions",
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
