# 桌面 UI 指南

> 状态：规范；`packages/ui` 与 `ctxpect daemon` localhost API 已可构建。Tauri WebView 全 OS 验收与完整产品运行时尚未实施。
> 视觉参考：`docs/gpt-img-2-design/20260904-1200-contexpect-doctor/`
> 语义以 PRD §9、design Markdown spec 和本文为准；生成图的小字可能失真。4/9/5 只是 Doctor fixture。

实现入口：`packages/ui`（React/TypeScript）、`packages/ui-tokens`、`ctxpect daemon start --listen 127.0.0.1:7420`。UI 只消费 `/api/v1/*`，不自行扫盘、不计算 Claim、不决定 policy pass。

## Shell

- 224px 深蓝导航 + 56px coordinate bar + 主工作区
- 右侧 368–384px evidence/care drawer，可折叠
- 主视口 1440×900；≥1280 三栏；1024–1279 drawer overlay；768–1023 主从；<768 只读 Receipt/通知

Doctor 是默认主入口（`/` → `/doctor`）。V01–V16 全部可达，可分组，但不能用隐藏菜单或空白占位代替功能。

| ID | 页面 | 路由 | DTO |
| --- | --- | --- | --- |
| V01 | Checkup | `/checkup` | inspect Receipt + unknown/findings |
| V02 | Inspector | `/inspector` | facets, budget, explanation, evidence |
| V03 | Compare | `/compare` | `GET /api/v1/diff?a=&b=` |
| V04 | Receipts | `/receipts`, `/receipts/:id` | list/show/verify/export/delete |
| V05 | Assets | `/assets`, `/assets/:id` | catalog; APM unique authority |
| V06 | Sessions / Monitor | `/sessions`, `/sessions/:id`, `/monitor` | import/timeline/delete; Monitor ≠ session detail |
| V07 | Effect Lab | `/lab`, `/lab/:id` | ExperimentContract + four decisions |
| V08 | Sync | `/sync` | transport vs semantic split |
| V09 | Doctor | `/doctor` | doctor findings; treatment lock |
| V10 | Policy | `/policy` | store 有效 policy 与 mutation 判定（与 apply 同源；无 layers 为 unknown，不是 pass） |
| V11 | Standards | `/standards`, `/standards/:id` | local-continuity standard |
| V12 | Settings | `/settings` | privacy/retention/vault/adapter/limits |
| V13 | Exceptions | `/exceptions` | request 走 mutation 授权；approve 无外部身份源，fail-closed |
| V14 | Team compliance | `/team/compliance` | redacted leader view |
| V15 | Care Plan | `/care-plan/:findingId` | preview/authority/apply/rollback |
| V16 | Integrations | `/integrations`, `/integrations/:id` | 18 families; independent install/auth/connector/version/surface |

## C01 坐标与异步

完整坐标：device、environment、account alias、organization、policy snapshot、harness、version、surface、project、cwd、task、snapshot。探测值、用户声明与 Unknown 分列。

切换坐标递增 `generation`；旧响应标记 `stale=true` 且不得覆盖当前 Receipt。扫描取消/失败保留上一有效 snapshot 并标 stale。`dev-inspect-v0` 只有经过 `migrate_dev_inspect_v0` 才成为 `ctxpect-receipt-v1`。

## C02 Inspector

六 facet 独立格子。每条 summary 可展开 claim_kind、lifecycle_stage、truth_state、provenance、coverage、precision、knowledge_status。固定提供 “Why is this here?” 与 “How do you know?”。预算 Unknown 不画成 0、不相加成假总量。

## C03 状态词表

- `truth_state`：present / absent / indeterminate / not-applicable。Expected 不是 truth_state。
- Unknown 不是 severity。Severity 只有 confirmed / suspected。
- Needs connector 是 compact 分组；展开必须是 installation/authentication/connector/version/surface。
- 显示遮罩、按住显示、复制确认、导出脱敏、egress consent 分别生效。

## C04 每页状态

每页至少：入口/返回、URL、DTO、主动作、成功落点、empty/loading/error/partial/stale/offline/permission-denied/unsupported-version/connector-missing、取消/重试、持久化边界、敏感数据边界。

Drawer：Esc 关闭、焦点返回、未保存编辑确认、执行中关闭不扩大授权。失败显示 reason code。

| 页面 | empty | loading | error | 主动作 |
| --- | --- | --- | --- | --- |
| Checkup | 未选项目 | skeleton | inspect 失败 reason | inspect → Receipt |
| Inspector | 无当前 Receipt | skeleton | stale | 展开 claim / 证据 |
| Compare | 少于两个 Receipt | skeleton | 不同域 | diff |
| Receipts | 无 ledger | skeleton | tombstone | show/verify/export/redact |
| Assets | 无 catalog | skeleton | pin unavailable | 只读展示；缺许可不复制 |
| Sessions | 未导入 | skeleton | parse failed | import / delete |
| Monitor | 无变更 | skeleton | daemon 缺失仍可 one-shot | 下钻 Receipt |
| Lab | 无实验 | skeleton | n locked | run frozen contract |
| Sync | 无 bundle | skeleton | encryption unavailable | preview；transport ≠ verified |
| Doctor | `No current findings in observed coverage` | skeleton | adapter reason | Diagnose / Collect evidence |
| Policy | 无规则 | skeleton | personal relax | eval |
| Standards | 无 standard | skeleton | unsigned | validate/adopt |
| Settings | 默认 metadata-only | skeleton | vault 缺失 | 保存/撤销 |
| Exceptions | 无请求 | skeleton | expired/stale | request/approve |
| Team | 无 disclosure | skeleton | 正文泄漏拒绝 | 重验 |
| Care Plan | 无 finding | skeleton | locked | preview；Indeterminate 不可 apply |
| Integrations | 18 names | skeleton | unknown family | 指向受支持证据动作 |

不适用状态给理由：例如 Checkup 无 connector-missing（那是 Integrations）。

## C05 隐私

默认正文遮罩；按住显示可键盘完成；失焦恢复。复制前确认。截图隐私覆盖 DOM 中带 `data-mask` 的正文与 aria/复制。不可信正文纯文本。deep link 不能直接 mutation、揭密、sync 或 LLM send。删除保留 tombstone，外部副本不可召回。

未覆盖：tooltip、toast、导出预览的按住显示与截图隐私。这三项当前没有对应控件，不得当作已覆盖。

## C06 桌面 / a11y / i18n

暖白/深蓝。WCAG 2.2 AA。键盘完成关键动作。zh-CN / en 覆盖导航、Doctor 与 Checkup/Inspector/Settings/Care Plan/Integrations 正文；其余页面仍以 API JSON 转储为主。已有 snapshot 时 coordinate bar、Receipt 时间、risk/unknown count、Inspector skeleton 目标 ≤2s（该性能数字尚未用 OS WebView 取证）。

## C07 真实动作

展示 preview 不是 apply。executor 0 不是生效。bundle 传完不是 semantic verified。用户点击不是 native evidence。Advisor 不是 finding。required-write 缺失是未完成，不能永久用 Unknown 按钮替代。

## C08 视觉产物

每页默认/空/错态与动作前后截图路径规划在 `docs/gpt-img-2-design/.../06-implementation-plan.md`。原始本机日志放 `.octoworkflow/`。Doctor 18 family DOM 断言求和为 18。

## 本地 API

`ctxpect daemon start --listen 127.0.0.1:PORT --store <dir> --project <dir> --ui-root packages/ui/dist`

- 只绑定 127.0.0.1
- **Host/Origin 精确校验**：主机名必须恰为 `127.0.0.1` / `localhost` / `::1`，不是前缀匹配——前缀会放行 `127.0.0.1.evil.com` 这类 DNS rebinding 绕过。缺失 `Host` 一并拒绝。POST 另需 `X-Ctxpect-Client: desktop`
- UI 不计算 Claim
- **静态文件服务用与读取项目文件相同的包含性检查**：路径先 canonicalize 再验证是否仍在 UI root 内。仅做 `..` 字符串比对是不够的——UI root 内的一个符号链接指向外部，请求里根本不会出现 `..`。不存在的路径回退到 `index.html`（SPA 路由），逃逸出 root 的路径报 `api.path`。
- **安全响应头**：`Content-Security-Policy`（`script-src 'self'`、`frame-ancestors 'none'`、`object-src 'none'`；`style-src` 允许 inline，因为 React 通过 `style` prop 设置样式）、`X-Frame-Options: DENY`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`、`Cache-Control: no-store`。
- **不发 `Access-Control-Allow-Origin`**：没有任何东西需要跨源访问——UI 由本 daemon 提供，开发时 vite 代理 `/api`，两种情况都是同源。该头此前存在但漏了端口，因而匹配不上任何真实 origin：一条什么也没授予、看起来却像策略的规则，迟早会被人「修」成真正的授权。
- **请求体必须是 JSON 对象**。空体表示「无参数」，可以；数组或标量则报 `api.body_not_object`——它们不携带处理函数要读的任何字段，接受它们等于用静默的默认值执行请求，再把结果当成功回给调用方。

**同一问题只有一个答案**（R04）。`GET /api/v1/standards/:id` 与 `ctxpect standard status`、`GET /api/v1/exceptions/:id` 与 `ctxpect exception status` 调用的是同一个函数，而不是各写一遍。此前 API 直接返回存储的标准文档，既不含本项目的采纳状态，也把「不存在」报成错误而非报成 `absent`——同一个查询给出两种答案。逐项对照已固化为测试。

主要路径：`/api/v1/health`、`/inspect`、`/receipts`、`/receipts/:id/verify`、`/receipts/:id/delete`、`/doctor`、`/diff`、`/integrations`、`/settings`、`/settings/schema`、`/sessions`、`/monitor`、`/policy`、`/exceptions`、`/exceptions/:id`、`/standards`、`/standards/:id`、`/assets`、`/assets/:id/preview`、`/assets/:id/copy`、`/assets/:tx/rollback`、`/sync`、`/sync/preview`、`/sync/apply`、`/advisor`、`/lab`、`/team/compliance`、`/care-plan/:id`、`/apply`、`/rollback`。

### 四个只读端点报告什么

以下端点此前返回固定常量。现在它们各自计算真实状态，且在算不出时报 Unknown 而不是补一个好看的默认值。

| 端点 | 计算方式 | 算不出时 |
| --- | --- | --- |
| `/monitor` | 对当前 Receipt 声明的每条 evidence 重算 `whole_digest` 与记录值比对，得出 `staleness.status = current\|stale\|unknown`，并列出 `changed_evidence` / `unreadable_evidence` | 无当前 Receipt 或有 evidence 读不到 → `unknown`，附 `reason_code`；不报 `stale: false` |
| `/sync` | 从 settings 读 `vault_required`，从 store 数本地 bundle 与可同步 Receipt | E2EE 未实现，如实报 `encryption: "unavailable"` + `sync.e2ee_unimplemented`；无远端传输报 `sync.no_remote_transport` |
| `/team/compliance` | standards 逐条**重新验签**后计数，adoptions 给出真实采纳态与 pinned digest，exceptions 逐条过 `exception_status` 只计存活的；drift/unknown/freshness 取当前 Receipt 的诊断；`audit` 报告审计链的验证结果 | 无当前 Receipt → `drift`/`unknown` 为 `null`、`freshness.status = unknown`；不报 0 |
| `/care-plan/:id` | 在当前 Receipt 的诊断结果里定位该 finding，返回它自己的 `treatment` 与 `placement` | 无当前 Receipt → `api.no_current_receipt`；id 不在诊断里 → `api.not_found`；不为未知 id 回显一份通用计划 |

`/monitor` 的 `mode: "oneshot"` 与 `daemon_required: false` 仍是常量，因为它们是架构事实：本产品按需观测，不常驻 watcher，CLI 不依赖 daemon。`/team/compliance` 的 `redacted` / `member_bodies_included` 同理，是隐私不变量。

`/team/compliance` 的 `disclosure.scope = "this-store-only"`：本切片没有团队传输，它统计的是本地 store，不是跨成员汇总。

## C04 页面状态契约

每个入口声明它**能到达**哪些状态，并为到不了的状态给出理由。声明与理由在
`packages/ui/src/page-contract.js`，判定逻辑在 `page-state.js`，两者都是纯模块，
由 `packages/ui/tests/page-state.test.mjs` 覆盖。

C04 明确要求「对不适用状态给理由，不能机械制造伪状态」，因此判定只从产品**实际
会产生**的 reason code 出发：

| 状态 | 判定依据 |
| --- | --- |
| `offline` | fetch 本身失败（daemon 不可达），reason code `api.unreachable` |
| `permission-denied` | envelope 的 `error.code` 以 `policy.` / `principal.` / `exception.` 开头，或是 `api.identity_required`、`advisor.consent_required` |
| `error` | 其它 envelope 错误 |
| `stale` | 载荷里 `stale === true`、`staleness.status === "stale"`，或 reason code `evidence_stale` |
| `unsupported-version` | 载荷里出现 reason code `unsupported_harness_version` |
| `connector-missing` | 载荷里出现 `connector_required` |
| `partial` | Unknown 计数 > 0，或出现 `runtime_snapshot_missing` 等表示答案不完整的 reason code |
| `empty` | 载荷无实质内容（envelope 记账字段不算内容） |

后两类之所以按**载荷内的 reason code** 而不是 HTTP 错误判定：`unsupported_harness_version`
与 `connector_required` 是 catalog 与 finding 里的 reason code，请求本身是成功的。

失败一律显示 reason code 与可行下一步。**重试只在故障类状态提供**：重试是重发同一个
请求，不改任何参数，因此不会扩大被拒绝的授权；对 `permission-denied` 提供重试按钮会
暗示再点一次可能通过，所以不提供。取消用 `AbortController` 真正中止在途请求，取消后
报 `api.cancelled` 而不是伪装成离线。

若某页出现了它未声明的状态，横幅会标出「本页不适用」，让契约的错误暴露而不是被吞掉。

**接入范围**：V01–V16 全部 16 个入口与 6 个 `:id` 明细页。自取数的页面走 `StateView`；
`/doctor`、`/checkup`、`/inspector` 共享同一次 inspect + doctor 请求，其判定在顶层算好
后经 `SharedStateBanner` 渲染同一套横幅；`/compare` 自己取 Receipt 列表与 diff，同样
经分类器判定。Doctor 里的适配器覆盖行在失败时显示 reason code，而不是渲染成空行——
空行与「没有 family」是两个不同的断言。

### 页面契约覆盖 C04 的哪些项

`page-contract.js` 里每个入口声明：

| 项 | 字段 | 由测试守住的方式 |
| --- | --- | --- |
| 入口与返回路径 | `entry` / `back` | `back` 必须是 `routes.ts` 里真实存在的路由；两者都不能是敷衍的短语 |
| URL / 选择状态 | `selection` | 明确写出哪些选择进 URL（可分享可刷新）、哪些只在会话内 |
| DTO 与查询 | `query` | 声明的每个 `method + path` 必须是 daemon **真实路由**的端点——从 `http.rs` 解析比对，写不出幻觉 API |
| 主要动作与成功落点 | `actions[].effect` / `.lands` | 每个动作必须写明副作用与成功后界面变成什么；只写「成功」不算 |
| 10 类状态 | `applicable` / `notApplicable` | 每状态要么适用要么给理由；理由不能以「不适用」开头复述结论 |
| 取消 / 重试 | 实现在 `useResource` | 取消真中止请求；重试只对故障类提供 |
| 持久化边界 | `persistence` | 写明什么落盘、什么随会话丢弃、什么不可撤销 |
| 敏感数据边界 | `sensitive` | 写明本页展示什么、**不**展示什么 |

两处声明之间也有交叉校验：**声明了写操作的页面，必须把 `permission-denied` 列为可达状态**。测试还断言只有 `/compare`、`/doctor`、`/receipts`、`/settings`、`/sync` 声明了动作，且它们的动作标签在 `App.tsx` 里确有对应控件——契约不能承诺一个页面并不渲染的动作。

### 动作面（V04 / V05 / V08 / V12）

| 页面 | 动作 | 成功落点 | 边界 |
| --- | --- | --- | --- |
| V04 Receipts | 验签、删除 | 验签显示 `org_identity: false`（本地 MAC 不是组织签名）；删除后显示 tombstone 并禁用两个动作 | 删除前用原生 `<dialog>` 模态确认，逐条说明：只保留 tombstone 且同 id 不可重建、派生分析失效、**已导出的外部副本无法召回** |
| V05 Assets | 核验并预览、复制到项目、回滚 | 预览显示许可证/来源/落点/损失；复制后显示事务 id 并刷新 lock 与 SBOM | 复制只在**核验通过的预览之后**可用；asset id 指向仓内登记，**来源与落点都不可由界面指定**，请求体里的路径被忽略 |
| V08 Sync | 预览、应用 | 预览给出 transport/semantic；应用后显示 `transport: success` 与 `semantic: structural-only`、`reconciliation: indeterminate` | 应用只在**干净预览之后**可用，冲突时禁用；transport 与 semantic 分列显示，且传输结果旁始终标注「传输成功不等于语义已验证」 |
| V12 Settings | 编辑、保存、撤销 | 保存成功后刷新已保存值并提示；失败保留用户编辑并显示 store 的 reason code | 编辑器由 `/api/v1/settings/schema` 生成，不硬编码字段；保存中禁用按钮防重复提交 |

**不生效的设置会被标注出来**。`GET /api/v1/settings/schema` 的 `unenforced` 字段列出「存储并校验、但产品不据以行动」的字段及原因，编辑器在该字段旁显示它。当前只有 `resource_limits.daemon_rss_mb` 在列（进程无法可移植地限制自身常驻内存）；`scan_files` 已真正约束 `collect` 的遍历，因此不在列。

**Settings 的验证在 store，不在 UI**。`put_settings` 校验整份文档：枚举取值、整数区间、未知字段、缺失字段，以及 `unmask_does_not_grant_egress` 这类**产品不变量**——它被记为 `const_bool`，设置不能把它改成 `false`。UI 侧的即时校验只是便利，发出相同的 reason code，最终判定仍以 store 为准（R04）。`analysis_adapter` 目前只接受 `none`：本切片没有已实现的 LLM adapter，允许填别的值会让 advisor 声称一条不存在的分析路径。

**资产的来源与落点由仓内登记决定**，不由界面或请求体决定。未登记、无许可证、或字节与登记摘要不符的资产在预览阶段就被拒绝，因此不会留下半个文件。详见 [cli-reference 的资产复制 executor](cli-reference.md#资产复制-executor)。

**Sync API 的目标是固定的**：`POST /api/v1/sync/preview|apply` 只对 `<store>/sync/folder` 操作。从请求体接受目的地路径等于让页面内容驱动任意文件写入，因此跨设备传输仍只走 CLI 的显式 `--dest`。`bundle_id` 会被写入 append-only 日志，故校验字符集并拒绝 `..`。

### 颜色与对比度

设计语言不变（暖白画布、深蓝侧栏、证据与严重性分离），但色值按 WCAG 2.2 AA 校准过：文本对背景 ≥ 4.5:1，控件边框与焦点环 ≥ 3:1。修正只压明度、保持色相。

`--border` 与 `--border-strong` 是两个用途不同的令牌：前者做面板边缘、表格行分隔这类装饰性描边；后者（3.00:1）给 input / select / textarea / 次要按钮，因为 SC 1.4.11 管的是需要被感知的**控件边界**。把控件改回 `--border` 会让边框回到 1.38:1。

严重性色之间的对比度很低，这是有意的：状态从不只靠颜色承载，每个徽章都带文字。`tests/contrast.test.mjs` 会同时守住色值与「不只靠颜色」这两点。

### 窄视口（<768）

C06 规定该宽度下产品是**只读的 Receipt/通知界面**，不是完整应用。实现上 <768 不渲染完整 shell，改由 `NarrowReadOnly` 提供 Receipt 列表与明细的只读查看；明细沿用默认遮罩，tombstone 条目单独标注，以免被读作仍然存在的 Receipt。

通知显示 `notifications.unimplemented`：store 会创建 `notifications` 目录，但本切片没有任何端点产生或读取通知。「暂无通知」与「功能不存在」是两个不同的断言，界面说的是后者。

窄屏检测同时监听 `matchMedia` 的 `change` 与 `window` 的 `resize`，并从查询对象读结果而非从事件读——只依赖 `change` 时，一个不派发该事件的视口变化会把用户困在错误布局里直到刷新。

### Drawer 契约

Doctor 的证据抽屉是**常驻区域**而非模态，因此没有打开/关闭、焦点返回与 Esc 可言；
它需要而此前缺少的是：选中项用 `aria-selected` 宣告、执行中用 `aria-busy` 标记，
以及**收集证据期间冻结选中**——否则结果会落到另一条 finding 上。重复提交由按钮的
`disabled` 阻止。抽屉内没有编辑表单，所以没有未保存编辑要处理。

## 启动教程

见 [user-guide](user-guide.md)「只读桌面主链」。本文件不是已完成全 OS WebView 验收的声明。

## 本切片明确未做 / 未覆盖

- **其余页面的呈现**：V04/V05/V08/V09/V12 已有动作面（见下节），其余入口的正文仍是 API JSON 转储，没有按页设计的呈现。C04 的逐页声明本身已补齐（见「页面契约覆盖 C04 的哪些项」），但声明中标为「本页当前只读」的动作（会话导入、实验发起、标准发布与采纳、例外申请与批准）仍只能走 CLI 或 API。
- **团队汇总**：`/team/compliance` 统计本地 store。没有团队传输，因此它不是跨成员的合规汇总，界面也不得这样呈现。
- **应用截图、Tauri 桌面壳、OS WebView / a11y / 屏幕阅读器 / ≤2s 性能证据**：未采集，不得当作已覆盖。
- **例外批准身份源**：UI/API 不能用调用方自报角色完成 approve。身份来自仓内已登记 principals + 调用方持有的登记密钥，而密钥只经环境变量传入 CLI，HTTP 请求无法安全携带。因此 `POST /api/v1/exceptions` 明确返回 `api.identity_required`，例外生命周期只经 CLI。
- **i18n**：导航、Doctor、以及 Checkup / Inspector / Settings / Care Plan / Integrations 的页面正文已接进 zh/en 表。API JSON 转储字段名仍是英文协议键。`ui-unit` 只做字符串表 grep，不渲染组件。
