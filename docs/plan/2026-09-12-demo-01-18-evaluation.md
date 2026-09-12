# DEMO-01–18 场景落地评估（U04/U05 前端收口切片）

日期：2026-09-12。范围：对照 [DEMO 合同 §6 场景表](../handoff/contexpect-2026-09-11/08_DEMO_CONTRACT_AND_FIXTURES.md) 逐条评估 18 个场景在现有 e2e（`packages/ui/tests/e2e/`，真 daemon + synthetic 种子，无 mock、无平行 demo-data 层）下的覆盖状态，并记录本切片新增的用例。

**为什么不建平行 demo-data 层**：合同 §4 要求"交互必须真改可解释状态"。现有 e2e 架构（global-setup 起真实 daemon、只经 API 播种、浏览器无 mock）已满足该要求；再叠一层 DemoDataSource 只会引入第二套真值来源，与合同 §7"优先使用现有 DTO"一致地选择不建。

以下原评估保留当时边界；当前变化以文末“后续修正”及[本轮裁决](2026-09-12-handoff-value-implementation.md)为准。

**原切片共享 daemon 的冷启动约束**：全套 e2e 共用一个 daemon + store，`pages.spec.ts` 首条用例断言"无 Receipt 的冷启动"（坐标 none、计数 —）。因此任何会持久化 Receipt 的动作（API inspect、`ctxpect doctor --store`）都不能出现在字母序先于 `pages.spec.ts` 的 spec 里（本切片三个新文件均排在其前）。这是多个"需要一张 Receipt 才能展示"的场景本次不落地的直接原因，不是后端能力缺口。

## 原 U04/U05 切片逐条评估（历史快照）

| 场景 | 现有覆盖 | 本次新增 | 未覆盖原因与建议归属 |
| --- | --- | --- | --- |
| DEMO-01 首次接入 | `pages.spec.ts`「cold Doctor shows unmeasured counts and the actual daemon address」：冷启动无 Receipt、坐标显示 none、不自动读 home | — | "显式选演示范围"的 UI 流程不存在（daemon 项目根由启动参数固定）；归桌面壳/设置工作包 |
| DEMO-02 静态结果/运行未知 | 同上冷启动用例：未观测时计数显示"—"而非 0 或假通过 | — | 六面 facet 的"静态已知 / 运行未知"对照需要一张 Receipt，受冷启动约束；建议归后续独立 daemon 的 e2e 或组件级 SSR 测试 |
| DEMO-03 产品排除/预算 | — | 单测层：`page-state.js` 的 PARTIAL_REASONS 收录 `observation_scope_excluded`，`page-state.test.mjs` 断言归类 partial（信息不全而非错误） | 端到端需 `.ctxpect-ignore` 夹具 + inspect，同冷启动约束；建议与 DEMO-02 同批 |
| DEMO-04 缺权限/未知版本 | 单测：`page-state.test.mjs`「unsupported version and missing connector come from real reason codes」 | DEMO-12 覆盖"缺授权写入被拒"半边 | "未知 harness 版本"的 e2e 需 catalog 夹具；建议归 integrations/fixture 工作包 |
| DEMO-05 请求证据导入 | global-setup 经 API 导入 s-alpha/s-beta；`pages.spec.ts`「the session page fetches request evidence…」与「session summaries…」：已观察字段显示、正文不上页、其余仍未知 | — | — |
| DEMO-06 正常静态修复 | — | `demo.spec.ts` DEMO-06：/assets 核验预览 → 授权复制 → 字节落盘核对 + 再预览显示覆盖计划（静态重验） | API copy 端点不返回 post-Receipt（CLI `assets copy` 有），本用例的重验为 digest 级而非 Receipt 级；该差异是否收口归后端/API 工作包决策。运行效果验证按合同本就不要求（缺效果实验不阻止有限修复） |
| DEMO-07 预览后变化 | — | `demo.spec.ts` DEMO-07：预览后带外改源文件 → copy 拒 `assets.digest_mismatch` → 再预览同拒 | 持久预览失效路径（`/api/v1/intent/preview` + `/api/v1/apply`）在本切片无 UI 驱动；target 侧并发保护（`assets.concurrent_hash`）经此端点不可达（copy 时重算 plan）。用例注释已声明证明边界 |
| DEMO-08 部分写入/恢复 | — | — | 资产复制是单文件事务，多文件部分失败无 UI 入口；归 ctxpect-assets/投影多文件事务工作包 |
| DEMO-09 回滚冲突 | — | — | 需要 `assets.rollback` 授权 + 冲突夹具；本切片刻意不给该授权以支撑 DEMO-12，二者共用同一 action 的授权面，建议后续用独立 action/夹具同补 |
| DEMO-10 跨坐标比较 | — | — | /compare 的 diff 需要两张 Receipt，同冷启动约束；建议与 DEMO-02 同批 |
| DEMO-11 标准/分叉 | — | — | transport 分叉协调在 sync crate/CLI；UI /sync 只有 preview/apply，无分叉视图；归 sync 工作包 |
| DEMO-12 无可信批准 | — | `demo.spec.ts` DEMO-12：未授权的 `assets.rollback` 被 daemon 拒（`policy.approval_required`），磁盘不变，只读预览仍可用 | 主体已覆盖；过期例外/自报管理员的完整身份周期归 policy crate 测试与后续 e2e |
| DEMO-13 隐私/删除 | `pages.spec.ts` 会话正文不上页（metadata-only）、`[data-metadata-only]` 计数 | — | "删除导入记录→关联建议失效"未 e2e 覆盖（`sessions.delete` 授权已播种，可直接补）；建议归下一前端切片 |
| DEMO-14 Advisor | — | `advisor.spec.ts` DEMO-14：未双确认不发请求且页面显示所需同意状态；服务端独立再验（三种缺确认组合均 400）；确认后候选渲染、六个 false 标志可见、store 字节级无变化、Receipt 账本仍空 | — |
| DEMO-15 Effect 未执行 | — | — | UI 无"对冻结 contract 查询"入口（/lab 只列已持久实验）；API 的 not-executed 应答由后端 `product_loops.rs` 覆盖。若 UI 增查询入口再补 e2e |
| DEMO-16 Effect 不可判定 | `pages.spec.ts`「the lab list and detail…」：executed:true、inconclusive、冻结估计器 `paired-exact-binomial-v2` | — | — |
| DEMO-17 异步/离线 | `pages.spec.ts`「a late answer for the session the user left is dropped」「a delayed bootstrap cannot overwrite a newer manual inspect」「cancel stops a self-fetching page's request…」 | — | 断网（transport failure → offline 状态）未 e2e 覆盖；可用 route abort 模拟，建议归下一前端切片 |
| DEMO-18 窄屏/键盘 | `pages.spec.ts` 命令面板焦点/Escape、四档宽度（1440/1100/820/390）布局与截图、窄屏只读 | — | 全键盘走查的逐页覆盖可继续加宽；归可用性工作包 |

## 本切片改动索引

- `packages/ui/tests/e2e/advisor.spec.ts`（新增）：DEMO-14。
- `packages/ui/tests/e2e/demo.spec.ts`（新增）：DEMO-06、DEMO-12、DEMO-07（按共享状态顺序排列）。
- `packages/ui/tests/e2e/consistency.spec.ts`（新增）：R04 的 CLI↔API 对拍延伸到 UI 腿；事实选取避开了需要 Receipt 的 doctor/receipt 列表（原因见文件头注释：CLI doctor 会持久化 Receipt、API doctor 需要已选 Receipt、且 CLI/API 的 as_of 时间语义不同）。
- `packages/ui/tests/e2e/global-setup.ts`：种子注册资产 `skill-e2e`（合成别名、digest 固定）、加种 `assets.copy` 授权（`assets.rollback` 刻意不授权）、导出 `E2E_STORE`。
- `packages/ui/src/page-state.js` + `packages/ui/tests/page-state.test.mjs`：`observation_scope_excluded` 归类 partial。


## 后续修正：资产与离线闭环（2026-09-12）

新增独立 `test-daemon.ts` / `isolated-test.ts`，每个写入场景独占临时项目和 store。因此原表中“冷启动约束”及 DEMO-09 与 DEMO-12 权限冲突已不再是测试设施阻碍。当时尚未新增的 DEMO-02/03/10 后由下节补齐指定浏览器反例；不扩大为全场景验收。

| 场景 | 当前补充 |
| --- | --- |
| DEMO-06 | 授权复制后核对磁盘，页面链接新的静态 Receipt，并通过真实 API 读回；运行效果仍未观测 |
| DEMO-07 | 源摘要变化与预览后目标变化均拒绝；更换资产 ID 清除旧预览并禁用复制 |
| DEMO-09 | 独立授予 rollback，验证后续编辑冲突不被覆盖，恢复合法状态后回滚成功并生成不同的 post-Receipt，旧 Receipt 字节不变 |
| DEMO-12 | 独立未授予 rollback，真实 daemon 拒绝并保持磁盘不变 |
| DEMO-13 | CLI 删除导入记录后 UI 刷新移除记录，派生 insights 失效；原合成来源文件保持不变。未新增 UI 删除按钮 |
| DEMO-17 | 浏览器注入 transport failure，同路径刷新保留旧数据并标注；恢复连接后替换；切换至缺失资源时不沿用旧结果 |

`demo.spec.ts` 当前 5 条，`retained-snapshot.spec.ts` 3 条。其余场景保持原表未覆盖范围。真实 daemon/store 与合成会话分开陈述，断网注入不等于真实网络环境全矩阵。


## 提交前接续：DEMO-02/03/10

`truth-boundaries.spec.ts` 使用各测试独立的真实 daemon/store：

- DEMO-02：静态 eligible present 与 model-visible/use-evidence/outcome-affecting indeterminate 同屏，并对照真实 Receipt。
- DEMO-03：`.ctxpect-ignore` 排除 AGENTS.md 后 eligible 为 indeterminate、原因 `observation_scope_excluded`；预算所有格仍 unknown/null。未新增 Claude 实测预算或所有 harness 预算矩阵。
- DEMO-10：同项目 Codex/Claude 两个坐标真实 inspect；比较明确 same_domain=false、baseline_allowed=false、verified=false；更换选择移除旧结果，取消在途旧请求，重跑绑定新选择。
- 补充：比较列表 transport failure 后可重试恢复，不再误调用尚未选择 Receipt 的 diff。

四条针对性测试通过；截图在该跑次的本地 E2E 输出中。剩余 DEMO-01 选择范围、04 版本矩阵、08 多文件恢复、11 分叉视图、12 完整身份周期、15 查询入口和18 全键盘矩阵仍按原表边界，不以补测试基础设施宣称全部完成。
