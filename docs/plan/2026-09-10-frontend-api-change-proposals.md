# 前端与 daemon API 配套收口（2026-09-10，2026-09-11 复核）

> 状态：P1–P3 与部分 P5 已实施，2026-09-11 按 owner 指示提交推送，未独立验收；P4 与剩余 P5 未实施。验证结果见文末。
> 本文接续前端呈现层重构的工作单。编号 P1–P5 是工作项编号，不是缺陷严重级别。
> Owner 已确认原后端会话停止并授权接续其未提交改动；既有改动保留，不表示它们已独立验收。
> 有效 UI/API 合同以 [desktop-ui](../guides/desktop-ui.md)、页面状态合同与实际源码为准。

## 对原总结的源码复核

| 原结论 | 当前证据与裁决 |
| --- | --- |
| Codex / Grok 都有 oracle 但没有 resolver，只有 Claude Code 有语法 | 不成立。`ctxpect-resolve::ANCHORS` 包含 Codex 与 Claude Code 的 instructions grammar；Grok 没有。Codex 同时具备已声明 oracle 与静态 grammar，四组展示采用 native 优先，不能由分组反推不存在另一能力 |
| `evidence_capability` 能解除前端字符串嗅探 | 采纳。它只是 catalog 声明的展示分组，不是安装、鉴权、运行或已经收集 native evidence 的事实。旧 daemon 回退保留；新 daemon 不认识的分组值显式显示 Unknown |
| 新 status 端点即可解决三页空白 | 不完整。必须把启动选择的 Receipt 明确加载，并让 Doctor / Monitor / Team / Care Plan 绑定该 id；摘要不能代替详情。首屏不隐式执行 inspect、不新建 Receipt |
| SessionsListView 已可接对象数组，无其他消费方 | 不成立。原实现使用 `data.sessions.map(String)`，对象会变成 `[object Object]`，链接也错误；没有证据证明 API 不存在其它消费者。采用增量字段保留旧数组 |
| `integration-contracts.yaml` 是新 daemon 路由的注册处 | 不适用。该工件冻结的是外部集成、版本 pin 和权限降级，不是 HTTP 路由表；本次不改冻结外部合同，更新 `ROUTE_TABLE`、页面合同、指南与新增 status schema |
| first_seen 显示占位字符串已经足够 | 保留诚实性，但可读性不足。现在显示“本次快照（历史首见未追踪）”，不伪装日期；历史首见仍未实施 |
| 小改已有 382 个测试与 gate 绿 | 属于原作者的历史报告，不能继承为当前候选的验收结果；本轮重新执行适用门禁 |

## P1：catalog 与前端事实展示

`crates/ctxpect-cli/src/catalog.rs` 保留四组增量字段与旧字段。优先级为已声明 native oracle → 本次 catalog 允许且已实现的 static resolver → connector cohort → unsupported；这个分组有损，不能代替五个安装/鉴权/connector/版本/surface 字段，也不能代替独立 capability 矩阵。

配套修复：

- coverage bar 拆成四段；未知未来 enum 保留独立 Unknown 计数。unsupported 文案不再武断声称“本机版本未受支持”。
- Doctor 未取得 counts 时显示 `—`，不会以三个 0 暗示没有问题。Confirmed/Suspected 按 daemon 的 confirmation 计数；Unknown 是知识缺口，计数可以重叠，不能求和成 finding 总数。
- severity 只展示 daemon 的 confirmed / suspected；缺失显示 `—`，不从 evidence_state 推断。
- 删除固定画出的“声明/解析已完成、后两步必断”的链；展示当前 finding 的 evidence_state、reason_code 与当前 Receipt 的 model-visible facet / claim_kind。
- `suppressed: true` 显式标记并保留 finding；`active_confirmed` / `active_blocking` 与事实 counts 分开展示，不把 suppression 当作问题消失。
- ⌘K 使用原生 modal dialog；Tab 不进入背景，Escape 关闭，焦点返回实际打开位置。
- daemon 地址取实际绑定地址；中等宽度的常驻证据区域随文档排列，避免无关闭按钮的固定层遮住正文。

## P2：只读启动状态与恢复

新增 `GET /api/v1/status`，schema 为 [ctxpect-status-v1](../schemas/ctxpect-status-v1.schema.json)。主要字段：

| 字段 | 语义 |
| --- | --- |
| `project` | 仅 `<project>` / `unset`，不返回主机绝对路径；留空的 inspect 请求使用 daemon 已声明的项目根 |
| `coordinate` / `generation` | 当前 daemon 声明的 harness/version/surface/os_lane 与请求代次；不补设备安装态 |
| `daemon` | 构建版本与实际 socket 绑定地址，包括端口 0 分配后的端口 |
| `selection` | `session-current` / `latest-matching` / `none`；历史恢复不改变 daemon 的当前 Receipt |
| `selected_receipt` | 选择后的 receipt_id、receipt_kind、created_at、完整 digest；无匹配为 null |
| `doctor_counts` | 绑定该 Receipt 的当前诊断计数，无 Receipt 为 null；不是历史时刻的已存诊断 |
| `diagnosis_basis` | `receipt-and-current-project-scan`；现有 Doctor 会重扫当前项目，不能称作历史诊断重放 |
| `staleness` | 复用 Monitor 的 `current / stale / unknown` 与原因码；不是 boolean，代次过时和文件证据变化是两种原因 |

选择规则：优先 daemon 当前 Receipt，必须仍符合项目 scope 与四个 harness 坐标；没有当前选择才从存储中选择同项目、同四坐标的最新非 tombstone Receipt。按可解析的 created_at（含小数秒）排序，相同时间用 id 稳定打破平局。缺失项目摘要的历史记录不猜归属；损坏索引、缺记录、不可解析的匹配记录时间返回错误，不伪装空库。当前选择已失效时显式报错，不悄悄切换另一份历史记录。

UI 启动只读 status，再按 `receipt_id` 取 Receipt 与 Doctor，核对两份返回的 id 后一起安装到页面。沿用 generation + AbortController；新手动 inspect 优先于迟到的启动响应。Monitor / Team / Care Plan 携带当前显示 Receipt 的 query 参数，避免刷新恢复的是历史 A、相关页面却默认使用 daemon 的 B。手动 inspect 不再强行指定 Codex，省略坐标时沿用 daemon 坐标。

限制：当前 JSON store 无分页/索引优化，历史选择读取列表与 Receipt，计数/新鲜度执行既有项目扫描；这是一次按需观测，不是 watcher，也不是恒定时间的 metadata 查询。大库性能优化应通过测量单独实施，不引入 SQLite 空骨架。用户输入另一个子目录与 daemon 根不一致时，既有 scope 边界仍适用。

## P3：兼容的 Sessions 元数据

`GET /api/v1/sessions` 保留 `sessions: string[]`，新增：

```json
{
  "sessions": ["session-id"],
  "session_summaries": [{
    "session_id": "session-id",
    "mapping_id": "codex-cli",
    "event_count": 1,
    "bodies_stored": false,
    "partial": true
  }]
}
```

这是说明形状的例子，不是 live 数据。`event_count` 精确定义为已存 `timeline.length`，不是输入文件所有事件行数、请求数或模型占用。字段缺失为 null，旧 daemon 没有 summaries 时 UI 对元数据列显示 `—`，仍按原 id 导航。API 使用五字段白名单，不返回 timeline、requests、正文或正文预览。现有 metadata-only importer 的 bodies_stored=false 不变；不会把任意已存记录的真值强制改成 false。读坏记录返回错误，不让它悄悄从列表消失。

## P4：历史 first_seen 仍需独立工作包

当前 finding_id 来自 `rule_id|title|affected` 的摘要；标题变化会变身份，路径归一化、项目范围、规则版本和同规则多实例的合并方式需要先定义，不能直接作为永恒身份。

实施前需定稿：

1. 身份键的项目 scope、规则 namespace/version、资源稳定标识与实例边界；排除 Receipt 时间和纯展示文案，并说明内容改变是否为同一问题。
2. 区分 first_seen_receipt_id、observed_at 与首次入库时间；导入更早 Receipt、乱序、重复导入如何影响结果。
3. 诊断输出目前结合历史 Receipt 和当前项目扫描：必须先保存可复核的诊断观测，不能用今天的重扫冒充历史时刻。
4. 删除/tombstone、保留策略、suppression、规则升级与索引重建的行为；历史不完整时返回 Unknown 并披露 coverage。
5. 正反例：同项目重现、跨项目同名、同规则多路径、纯标题变化、乱序导入、过期 suppression、删除最早观测、缺历史证据。

本轮不新增倒排存储，不承诺已实现真实首见。

## P5：结构化视图

Monitor 本轮已按 schema 展示 Receipt、新鲜度、reason_code、mode、compared，并保留原始负载。Policy / Exceptions / Team compliance 的结构化视图已于 2026-09-11 实施：Policy 以 verdict 横幅 + scope/layers kv + 规则分组表呈现（enforceable 与 detect-only 分列，detect-only 不显示为通过），Exceptions 为只读 id 表并注明请求/批准走 CLI 身份通道，Team 以 disclosure（this-store-only）领衔且 drift/unknown 在无 Receipt 时显示 — 而非 0；三者均以 RawJsonDetails 折叠保留原始负载，未知 payload 形状回退原始 JSON。

下一步分别遵守：Policy 五层原顺序与 enforceable/detect-only 边界；Exceptions 请求/批准仍走现有 CLI 身份通道；Team 明示 `this-store-only`，不能显示为跨组织汇总；未知 payload 形状回退原始 JSON。纯呈现工作不得新增评分、合并 facet、扩大 mutation 权限。

## 验证与剩余门禁

- 已添加真实 daemon 测试：空库 null、socket 地址脱敏、当前选择、文件变化/不可读、重启恢复不改 session-current、跨项目/跨 harness 排除、tombstone、索引损坏、增量 Sessions 白名单与无正文、status schema 正反例。
- 已添加浏览器回归：冷启动未测量计数、三个诊断页面刷新、Sessions 元数据与链接、迟到 bootstrap 不覆盖手动 inspect、命令面板焦点与 Escape。
- 2026-09-11 本地实测：16 条 required gate 均取得退出码 0。先整体执行 16 条；失败修正后整体复跑，再对最后的 Clippy 测试借用修正、UI 布局修正重跑受影响检查。Rust workspace 385 passed / 0 failed / 0 ignored；UI unit 60 passed；真实 daemon Chromium E2E 12 passed，包含 1440 / 1100 / 820 / 390 宽度与控件边界检查。
- 已目视检查四种宽度截图；修正 820px 顶栏挤压/裁切与中宽侧栏遮挡。截图来自临时合成项目与真实本地 daemon，不是用户私人项目的运行证据，也不是 Tauri WebView / 全 OS 验收。
- 本轮还修正阻断门禁的配套问题：nullable-object schema 的 required 误施加到 null；新路由固定数量断言；loading/cancelled 状态标记；既有三处 Clippy lint。历史交付记录引用的 ADR 0007 文件当前缺失，已将断链改为显式缺证据说明，未编造该 ADR。
- 原始各 gate 日志、执行汇总和截图在忽略目录内保留；可发布文档不嵌入含主机路径的日志。该轮验证时候选尚未 commit；未做独立 readback，未构建/验收 Tauri 安装包，不能称为完整产品 GREEN。
- 2026-09-11 P5 收口（纯前端）：Policy / Exceptions / Team compliance 结构化视图、Sessions 布尔列 i18n、findings 首见列防折行、vite 代理 CTXPECT_DAEMON_URL 可配、死 CSS（palette-backdrop/chain*）清理。`check_ui_routes.py`、`check_docs.py` 退出码 0；UI unit 60 passed / 0 failed；typecheck 与 vite build 通过。未改 Rust，未重跑 Rust 门禁。
- 2026-09-11 按 owner 指示提交推送前，在提交树的干净 worktree 复跑：16 条 required gate 退出码 0（Rust workspace 385 passed / 0 failed；UI unit 60 passed），`ui-build` 与 `generate_acceptance.py` 重跑后无差异。可选 `ui-e2e` 10 passed / 2 failed，未修：Sessions 布尔列已本地化为「否」而用例仍断言 `false`；Doctor 在 820px 出现横向溢出（根因未定位）。
