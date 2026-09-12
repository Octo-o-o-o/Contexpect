# 桌面 UI 指南

> 状态：规范；`packages/ui` 与 `ctxpect daemon` localhost API 已可构建。Tauri WebView 全 OS 验收与完整产品运行时尚未实施。
> 视觉规范：最终方向为白底黑字 monochrome，见 [handoff 06 UI/UX 视觉规范](../handoff/contexpect-2026-09-11/06_UI_UX_AND_VISUAL_SPEC.md)；`docs/gpt-img-2-design/20260904-1200-contexpect-doctor/` 仅作历史构图参考。
> 语义以 PRD §9、design Markdown spec 和本文为准；生成图的小字可能失真。4/9/5 只是 Doctor fixture。

实现入口：`packages/ui`（React/TypeScript）、`packages/ui-tokens`、`ctxpect daemon start --listen 127.0.0.1:7420`。UI 只消费 `/api/v1/*`，不自行扫盘、不计算 Claim、不决定 policy pass。

## Shell

- 236px 浅色导航（浅灰底 + 1px 分隔边框，无大面积深色壳）+ 56px coordinate bar + 主工作区
- 右侧 evidence/care 常驻 region；可折叠 drawer 尚未实施
- 主视口 1440×900；≥1280 三栏；768–1279 当前常驻证据 region 随文档排列以避免遮挡正文；<768 只读 Receipt/通知。可关闭 overlay drawer 尚未实施
- 导航按 诊断 / 记录 / 操作 / 治理 四组分组；顶栏右侧有 ⌘K 命令面板入口，可按页面名过滤跳转。

Doctor 是默认主入口（`/` → `/doctor`）。V01–V17 全部可达，可分组，但不能用隐藏菜单或空白占位代替功能。

| ID | 页面 | 路由 | DTO |
| --- | --- | --- | --- |
| V01 | Checkup | `/checkup` | inspect Receipt + unknown/findings |
| V02 | Inspector | `/inspector` | facets, budget, explanation, evidence |
| V03 | Compare | `/compare` | `GET /api/v1/diff?a=&b=` |
| V04 | Receipts | `/receipts`, `/receipts/:id` | list/show/verify/export/delete |
| V05 | Assets | `/assets`, `/assets/:id` | catalog; APM unique authority |
| V06 | Sessions / Monitor | `/sessions`, `/sessions/:id`, `/monitor` | import/timeline/delete；`/sessions/:id` 是**请求证据页**（每次请求的 header digest、消息数、来源/替换区间、派发证据、限制说明）+ 两个只显示类型/seq/长度/digest 的视图；Monitor ≠ session detail |
| V07 | Effect Lab | `/lab`, `/lab/:id` | ExperimentContract + four decisions；未执行的实验显示 `executed: false` 与原因，无判定 |
| V08 | Sync | `/sync` | transport vs semantic split |
| V09 | Doctor | `/doctor` | doctor findings; treatment lock |
| V10 | Policy | `/policy` | store 有效 policy 与 mutation 判定（与 apply 同源；无 layers 为 unknown，不是 pass） |
| V11 | Standards | `/standards`, `/standards/:id` | local-continuity standard |
| V12 | Settings | `/settings` | privacy/retention/vault/adapter/limits |
| V13 | Exceptions | `/exceptions` | request 走 mutation 授权；approve 无外部身份源，fail-closed |
| V14 | Team compliance | `/team/compliance` | redacted leader view |
| V15 | Care Plan | `/care-plan/:findingId` | preview/authority/apply/rollback |
| V16 | Integrations | `/integrations`, `/integrations/:id` | 18 families; independent install/auth/connector/version/surface；`evidence_capability`（native / static-only / connector-required / unsupported）是 daemon 计算的分组标签，调用方不再自行嗅探 |
| V17 | Advisor | `/advisor` | F-13/F-14 的可达入口；发送预览 + 双确认（consent / preview_ack）后才 `POST /api/v1/advisor`；返回 advisor-suggestion 候选（`is_claim=false`，不进 policy/CI/baseline/reconciliation，不解锁处置；无置信分） |

## C01 坐标与异步

完整坐标：device、environment、account alias、organization、policy snapshot、harness、version、surface、project、cwd、task、snapshot。探测值、用户声明与 Unknown 分列。

切换坐标递增 `generation`；旧响应标记 `stale=true` 且不得覆盖当前 Receipt。扫描取消/失败保留上一有效 snapshot 并标 stale。`dev-inspect-v0` 只有经过 `migrate_dev_inspect_v0` 才成为 `ctxpect-receipt-v1`。同一模式抽成 `src/generation.js`（`createGeneration().next()/isCurrent()`，单测 `generation.test.mjs`）：`/sessions/:id` 切换会话 id 时递增 generation，旧 id 的两个响应（记录、请求证据）晚到即被丢弃；Doctor 链的 `runInspect` 与通用的 `useResource` 也用同一个 helper（此前 `useResource` 只 abort 不判代，一个已离开 daemon 的旧响应仍能把新请求改成 cancelled）。浏览器级取证：`ui-e2e` 用 `page.route` 延迟 s-alpha 的请求证据 1.5 s，客户端跳到不存在的会话后 2.5 s 内错误横幅保留、表格不出现。

### 2026-09-11 启动状态

`GET /api/v1/status` 的 [schema](../schemas/ctxpect-status-v1.schema.json) 约束项目脱敏、daemon 实际地址、坐标/代次、`selection`、`selected_receipt`、nullable `doctor_counts` 和三态 `staleness`。优先匹配的 session-current；无当前选择时只读恢复同项目、同 harness/version/surface/os_lane 的最新非 tombstone，历史归属 Unknown 不猜。不会改变 daemon 当前选择，不执行 inspect 或写入新 Receipt。详情见 [配套工作单](../plan/2026-09-10-frontend-api-change-proposals.md)。

UI 按明确 id 加载 Receipt 与 Doctor，Monitor / Team / Care Plan 也带该 id；启动响应受相同 generation/abort 约束，不能覆盖更新的手动 inspect。status 中计数来自 Receipt 加当前项目扫描，不是持久化的历史 Doctor。无 Receipt 计数为 null，界面显示 `—`；新鲜度 current/stale/unknown 不压成 boolean。离线或旧 daemon 缺端点时保留错误码，用户仍可输入项目手动检查。

`GET /api/v1/sessions` 保持 `sessions: string[]`，另加五字段 `session_summaries`：session_id、mapping_id、event_count（已存 timeline.length）、bodies_stored、partial。不含 timeline/requests/正文；旧 daemon 元数据列显示 `—`。

Doctor 的 `first_seen: current-receipt` 显示为“本次快照（历史首见未追踪）”；severity 只读 daemon 字段，不由 evidence 推断。suppressed finding 保留并标记，active_confirmed/active_blocking 单独展示。coverage 是声明 catalog 分组，非 live 安装矩阵；Codex 同时有已声明 native oracle 和静态 resolver。

## C02 Inspector

六 facet 独立格子。每条 summary 可展开 claim_kind、lifecycle_stage、truth_state、provenance、coverage、precision、knowledge_status。固定提供 “Why is this here?” 与 “How do you know?”。预算 Unknown 不画成 0、不相加成假总量。

## C03 状态词表

- `truth_state`：present / absent / indeterminate / not-applicable。Expected 不是 truth_state。
- Unknown 不是 severity。Severity 只有 confirmed / suspected。
- Needs connector 是 compact 分组；展开必须是 installation/authentication/connector/version/surface。
- 显示遮罩、按住显示、复制确认、导出脱敏、egress consent 分别生效。

## C04 每页状态

每页至少：入口/返回、URL、DTO、主动作、成功落点、empty/loading/error/partial/stale/offline/permission-denied/unsupported-version/connector-missing、取消/重试、持久化边界、敏感数据边界。

Modal drawer 合同：Esc 关闭、焦点返回、未保存编辑确认、执行中关闭不扩大授权。当前 Doctor 是常驻只读 region，不伪装 modal；⌘K 命令面板使用原生 modal dialog，具备焦点约束、Escape 与返回焦点。失败显示 reason code。

| 页面 | empty | loading | error | 主动作 |
| --- | --- | --- | --- | --- |
| Checkup | 未选项目 | skeleton | inspect 失败 reason | inspect → Receipt |
| Inspector | 无当前 Receipt | skeleton | stale | 展开 claim / 证据 |
| Compare | 少于两个 Receipt | skeleton | 不同域 | diff |
| Receipts | 无 ledger | skeleton | tombstone | show/verify/export/redact |
| Assets | 无 catalog | skeleton | pin unavailable | 只读展示；缺许可不复制 |
| Sessions | 未导入 | skeleton | parse failed / seq_discontinuity / provenance_incomplete（以 `unknown[]` 原因列出，页面标 partial） | import / delete（走 CLI/API）；明细页只读 |
| Monitor | 无变更 | skeleton | daemon 缺失仍可 one-shot | 下钻 Receipt |
| Lab | 无实验 | skeleton | n locked | run frozen contract（需 runs 文档；无 runs 的实验显示未执行） |
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

白底黑字 monochrome。WCAG 2.2 AA。键盘完成关键动作。zh-CN / en 覆盖导航、Doctor 与 Checkup/Inspector/Settings/Care Plan/Integrations 正文；其余页面仍以 API JSON 转储为主。已有 snapshot 时 coordinate bar、Receipt 时间、risk/unknown count、Inspector skeleton 目标 ≤2s（该性能数字尚未用 OS WebView 取证）。

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

**诊断报告指明它依据的 Receipt**。`GET /api/v1/doctor` 的响应带 `receipt_id`：一份不说明所依据观测的诊断无法被核验，尤其是在省略 `receipt_id` 参数、由会话的当前 Receipt 决定时。

**同一问题只有一个答案**（R04）。`GET /api/v1/standards/:id` 与 `ctxpect standard status`、`GET /api/v1/exceptions/:id` 与 `ctxpect exception status` 调用的是同一个函数，而不是各写一遍。此前 API 直接返回存储的标准文档，既不含本项目的采纳状态，也把「不存在」报成错误而非报成 `absent`——同一个查询给出两种答案。逐项对照已固化为测试，覆盖 standard（存在与不存在）、exception、policy、assets、receipt 明细与 doctor。`doctor` 的对照剔除 CLI 因额外执行 inspect 而携带的元信息，比较诊断本身。

### 语义合同（A2）

路由表是 `crates/ctxpect-cli/src/http.rs` 的 `ROUTE_TABLE`，按 **(method, path)** 精确匹配；页面契约测试从该表比对方法与路径，写不出幻觉端点。

| 项 | 合同 |
| --- | --- |
| 路由 | 已知路径、未路由方法 → HTTP 405 + `error.code = api.method_not_allowed`（附 `allowed`）；未知路径 → 400 + `api.not_found`。字面段**排他**：一条字面模式命中该路径时 `:id` 模式对它一律不适用，因此 `POST /sessions/import` 不是名为 `import` 的会话，`GET /sessions/import` 是 405（`allowed: [POST]`）而不是查一个叫 `import` 的会话 |
| 错误 envelope | 所有失败都是 `{"error": {"code", "message"}}`，HTTP 400（授权拒绝也是 400，`code` 以 `policy.` / `principal.` / `exception.` 开头）；成功体带 `snapshot_digest` |
| 请求 framing | 请求头按 `\r\n\r\n` 收齐（上限 64 KiB → 431 `api.header_too_large`），请求体按 `Content-Length` **收满**再路由（上限 4 MiB → 413 `api.payload_too_large`；`Transfer-Encoding` → 501 `api.transfer_encoding_unsupported`）；单次 socket 读超过 5 s → 408 `api.timeout`，因此空闲连接不再能阻塞单线程 daemon。此前只做一次 `read()`，分段到达的请求体会被丢掉并按 `{}` 处理 |
| 请求体 | 必须是 JSON 对象；空体 = 无参数；数组/标量 → `api.body_not_object`（`PUT/POST /settings` 同样经此校验；`GET /settings` 附带的 `snapshot_digest` 原样 PUT 回去会被剥离，不算未知字段） |
| HEAD / OPTIONS | `HEAD` 按 GET 路由、同样的头与 `Content-Length`、不写体；`OPTIONS` 对已知路径回 204 无体，未知路径 400 `api.not_found`，且不经 CSRF 检查 |
| generation / stale | `POST /inspect` 递增会话 `generation` 并移动会话坐标；晚到的响应带 `stale: true` 且不更新当前 Receipt；`GET /coordinate` 与 `GET /health` 回显当前 generation |
| Receipt 绑定 | `GET /doctor?receipt_id=<id>` 诊断该 Receipt；不带参数时诊断会话当前 Receipt，响应始终带 `receipt_id`。项目扫描失败时**返回错误**（`io.*`），不退回一份缺了项目规则的诊断（与 CLI 同答）。UI 的 Doctor 链把 `POST /inspect` 返回的 `receipt_id` 传给 `GET /doctor`，用与 `generation.js` 相同的 `createGeneration()` 丢弃过期结果，并对上一轮的两个请求发 abort（C40） |
| 会话坐标 | daemon 只有**一个**会话坐标与当前 Receipt，所有标签页/客户端共享：另一客户端的 `POST /inspect` 会移动本客户端看到的坐标；`/monitor`、`/care-plan/:id`、`/team/compliance` 依赖当前 Receipt，Doctor 链靠 `receipt_id` 绑定不受影响 |
| 观测范围 | `POST /inspect` 的 project 必须在 daemon 声明根内；省略时使用该根。codex_home 只能等于启动时声明的值；无声明不允许请求扩大范围。跨根 Receipt 拒绝重扫，缺 scope 的旧 Receipt 保留 Unknown |
| verify | `POST /receipts/:id/verify` 与 `ctxpect receipt verify` 调用同一函数：tombstone → 200 `{ok:false, reason_code: receipt.tombstoned}`，旧签名 → 200 `{ok:false, reason_code: receipt.signature_legacy}`，摘要/MAC 不符 → 400 错误信封 |
| 坐标感知 | `GET /integrations`、`/integrations/:id`、`/assets`、`/assets/:id` 以**会话当前坐标**为 `active_coordinate`，随 `POST /inspect` 的 `harness/version/surface/os_lane` 变化（C9） |
| 分页 | 无。列表端点返回全部 id；本切片没有游标合同 |
| 取消 | 服务端无取消端点；客户端用 `AbortController` 中止，页面报 `api.cancelled` |
| mutation | 每个写端点经 `authorize_store_apply(action, target)`：授权绑定 action（`apply`、`rollback`、`assets.copy`、`assets.rollback`、`sessions.import`、`settings.put`、`receipt.delete`、`experiment.persist`、`sync.apply`）、项目摘要与目标；例外不覆盖则 `policy.approval_required`（消息含 `exception.*_mismatch` 原因） |
| apply | `POST /intent/preview` 要求 `target` 与 `desired`（缺失 → `usage.invalid`，不再用 `AGENTS.md` / `updated\n` 静默默认值算出一份预览并落盘），计算并**持久化**预览（`tx_id` 每次唯一、`current_digest`、`existed_before`）；`POST /apply` 只接受 `{"tx_id"}`，先取 store 锁再读预览状态，用持久化的 preimage 校验，目标已变 → `projection.concurrent_hash`；`POST /rollback` 与 `POST /assets/:tx/rollback` 的 `tx_id` 必须是 `tx_<16 hex>`（否则 `store.bad_id`，不拼路径），校验记录的 `project_digest` 与 `after_digest`，目标被改 → `projection.rollback_conflict`；控制路径与非常规文件的拒绝同 CLI（`projection.control_path` / `projection.not_a_file`） |
| 导入 | `POST /sessions/import` 从体取 `session_id`（缺省为内容摘要：原生 mapping 取 `jsonl` 字符串的 sha256，与 CLI 对同一文件算出的 id 相同；其它 mapping 取请求体）与 `mapping_id`；同一 id 下已有不同记录 → `store.session_exists`；导入同时写入与 CLI 相同的 insight 记录（R04）（须为 `acceptance/field-to-claim/*.yaml` 之一，否则 `import.mapping_unknown`）；`mapping_id: deepseek-harness-cli` 时原生 JSONL 放在 `jsonl` 字符串字段（缺失 → `import_parse_failed`）；store 是 metadata-only，不存正文预览 |
| 请求证据 | `GET /sessions/:id/requests` 返回该会话的 `requests[]`（每个 step 一条：seq = `step/start`、turn/step、header_seq、header_logged_in_step、reason、header_digest、message_count、message_digests、source_seq_ranges、replaced_ranges、dispatch_evidence；前缀无 header 的 step 以 `header_digest: null` 列出）、`tail`、`unknown[]`、`partial`、`bodies_stored: false`；会话删除后与 `GET /sessions/:id` 一样报 `store.missing` |
| Effect Lab | `GET /lab` 列出每个实验的 `executed` / `decision` / `reason_code`；`POST /lab` 无 `runs` → 未执行结果（不写 store），有 `runs`（`ctxpect-effect-runs-v1`）→ 判定并在 `experiment.persist` 授权下写入 |
| apply 绑定 | `POST /apply` 的 `tx_id` 必须是本项目的预览（否则 `projection.preview_scope`）且未被消费（否则 `projection.tx_consumed`）；每个写端点在授权时取同机 advisory lock，另一进程持锁 → `store.busy` |

主要路径（全部 45 条见 `ROUTE_TABLE`）：`GET /api/v1/status`、`GET /api/v1/health`、`GET /coordinate`、`POST /inspect`、`GET /receipts`、`GET /receipts/:id`、`POST /receipts/:id/verify`、`POST /receipts/:id/delete`、`GET /doctor`、`GET /diff`、`GET /integrations`、`GET /integrations/:id`、`GET /assets`、`GET /assets/:id`、`POST /assets/:id/preview`、`POST /assets/:id/copy`、`POST /assets/:id/rollback`、`GET /settings/schema`、`GET /settings`、`PUT|POST /settings`、`GET /sessions`、`GET /sessions/:id`、`GET /sessions/:id/requests`、`POST /sessions/import`、`GET /monitor`、`GET /policy`、`GET /exceptions`、`GET /exceptions/:id`、`POST /exceptions`（恒 `api.identity_required`）、`GET /standards`、`GET /standards/:id`、`GET /sync`、`POST /sync/preview`、`POST /sync/apply`、`POST /advisor`、`GET /lab`、`GET /lab/:id`、`POST /lab`、`GET /team/compliance`、`GET /care-plan/:id`、`POST /collect`、`POST /intent/preview`、`POST /apply`、`POST /rollback`。

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
| `permission-denied` | envelope 的 `error.code` 以 `policy.` / `principal.` / `exception.` 开头，或是 `api.identity_required`、`advisor.consent_required`、`advisor.preview_required` |
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

两处声明之间也有交叉校验：**声明了写操作的页面，必须把 `permission-denied` 列为可达状态**。测试还断言只有 `/advisor`、`/assets`、`/compare`、`/doctor`、`/receipts`、`/settings`、`/sync` 七页声明了动作，且它们的动作标签在 `App.tsx` 里确有对应控件——契约不能承诺一个页面并不渲染的动作。契约的 `method + path` 与 `ROUTE_TABLE` 逐项按方法比对：声明 `DELETE /api/v1/receipts/:id/verify` 这类未路由方法会让测试失败。

### 动作面（V04 / V05 / V08 / V12）

| 页面 | 动作 | 成功落点 | 边界 |
| --- | --- | --- | --- |
| V04 Receipts | 验签、删除 | 验签显示 `org_identity: false`（本地 MAC 不是组织签名）；删除后显示 tombstone 并禁用两个动作 | 删除前用原生 `<dialog>` 模态确认，逐条说明：只保留 tombstone 且同 id 不可重建、派生分析失效、**已导出的外部副本无法召回** |
| V05 Assets | 核验并预览、复制到项目、回滚 | 预览显示许可证/来源/落点/损失；复制后显示事务 id 并刷新 lock 与 SBOM | 复制只在**核验通过的预览之后**可用；asset id 指向仓内登记，**来源与落点都不可由界面指定**，请求体里的路径被忽略 |
| V08 Sync | 预览、应用 | 预览给出 transport/semantic；应用后显示 `transport: success` 与 `semantic: structural-only`、`reconciliation: indeterminate` | 应用只在**干净预览之后**可用，冲突时禁用；transport 与 semantic 分列显示，且传输结果旁始终标注「传输成功不等于语义已验证」 |
| V12 Settings | 编辑、保存、撤销 | 保存成功后刷新已保存值并提示；失败保留用户编辑并显示 store 的 reason code | 编辑器由 `/api/v1/settings/schema` 生成，不硬编码字段；保存中禁用按钮防重复提交 |

**不生效的设置会被标注出来**。`GET /api/v1/settings/schema` 的 `unenforced` 字段列出「存储并校验、但产品不据以行动」的字段及原因，编辑器在该字段旁显示它。当前在列：`resource_limits.daemon_rss_mb`（进程无法可移植地限制自身常驻内存）、`retention_days`（无保留期清扫）、`copy_confirm`（UI 总是确认，不读该字段）、`privacy_mode` 与 `screenshot_privacy`（UI 的隐私模式是会话内开关，不从 settings 读取）。`scan_files` 已真正约束 `collect` 的遍历，因此不在列。

**Settings 的验证在 store，不在 UI**。`put_settings` 校验整份文档：枚举取值、整数区间、未知字段、缺失字段，以及 `unmask_does_not_grant_egress` 这类**产品不变量**——它被记为 `const_bool`，设置不能把它改成 `false`。UI 侧的即时校验只是便利，发出相同的 reason code，最终判定仍以 store 为准（R04）。`analysis_adapter` 目前只接受 `none`：本切片没有已实现的 LLM adapter，允许填别的值会让 advisor 声称一条不存在的分析路径。

**资产的来源与落点由仓内登记决定**，不由界面或请求体决定。未登记、无许可证、或字节与登记摘要不符的资产在预览阶段就被拒绝，因此不会留下半个文件。详见 [cli-reference 的资产复制 executor](cli-reference.md#资产复制-executor)。

**Sync API 的目标是固定的**：`POST /api/v1/sync/preview|apply` 只对 `<store>/sync/folder` 操作。从请求体接受目的地路径等于让页面内容驱动任意文件写入，因此跨设备传输仍只走 CLI 的显式 `--dest`。`bundle_id` 会被写入 append-only 日志，故校验字符集并拒绝 `..`。

### 颜色与对比度

设计语言为白底黑字 monochrome（2026-09-11 最终方向）：纯白画布、浅灰侧栏与次级表面，近黑实心只给最重要动作；证据与严重性语义色保留原有色相、仅局部使用，状态从不只靠颜色承载。色值按 WCAG 2.2 AA 校准：文本对背景 ≥ 4.5:1，控件边框与焦点环 ≥ 3:1。

`--border` 与 `--border-strong` 是两个用途不同的令牌：前者做面板边缘、表格行分隔这类装饰性描边；后者（3.66:1）给 input / select / textarea / 次要按钮，因为 SC 1.4.11 管的是需要被感知的**控件边界**。把控件改回 `--border` 会让边框跌回 1.27:1。

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

- **其余页面的呈现**：V04/V05/V08/V09/V12/V17 已有动作面（见下节），`/sessions/:id`（请求证据页，`SessionRequestsView`）与 `/lab`、`/lab/:id`（执行状态 / 判定摘要，`LabListView` / `LabResultView`）已有按页呈现，其余入口的正文仍是 API JSON 转储。C04 的逐页声明本身已补齐（见「页面契约覆盖 C04 的哪些项」），但声明中标为「本页只读」的动作（会话导入与删除、实验发起、标准发布与采纳、例外申请与批准）仍只能走 CLI 或 API。
- **团队汇总**：`/team/compliance` 统计本地 store。没有团队传输，因此它不是跨成员的合规汇总，界面也不得这样呈现。
- **应用截图、Tauri 桌面壳、OS WebView / a11y / 屏幕阅读器 / ≤2s 性能证据**：未采集，不得当作已覆盖。
- **例外批准身份源**：UI/API 不能用调用方自报角色完成 approve。身份来自仓内已登记 principals + 调用方持有的登记密钥，而密钥只经环境变量传入 CLI，HTTP 请求无法安全携带。因此 `POST /api/v1/exceptions` 明确返回 `api.identity_required`，例外生命周期只经 CLI。
- **i18n**：导航、Doctor、以及 Checkup / Inspector / Settings / Care Plan / Integrations 的页面正文已接进 zh/en 表。API JSON 转储字段名仍是英文协议键。
- **渲染测试的边界**：`ui-unit` 的 `render.test.mjs` 用 `vite build --ssr` + `react-dom/server` 渲染 24 条路由的首屏、各页契约声明适用的状态横幅（不适用状态被标为矛盾而非隐藏）、请求证据页（`renderSessionRequests`：header digest、计数、区间、派发证据、限制说明、两个 metadata-only 视图，且断言不含正文）与 Effect Lab 结果（`renderLabResult`：`executed: false` + 原因）。SSR 不执行取数 effect；取数、导航与交互由可选门禁 `ui-e2e`（`packages/ui/tests/e2e/pages.spec.ts`，Playwright 驱动真实 daemon）覆盖：`/sessions/:id` 三条请求与选择后的派生 surface 视图、切换会话后旧错误横幅不残留、`/lab` 与 `/lab/:id` 的执行状态与判定、`/checkup` 真实 inspect 往返并渲染 `policy_result`。取消（中止请求）由 `ui-e2e` 的 cancel 用例覆盖（延迟响应后点击取消，页面进入 `cancelled` 且响应不落页）。


## 资产预览与写后观察（2026-09-12）

`POST /assets/:id/preview` 返回冻结计划的 `tx_id`；复制必须以 `POST /assets/:id/copy` 提交 `{"preview_id":"tx_…"}`。预览绑定项目与登记资产，15 分钟过期，一次写入尝试前持久标为 consumed；目标在预览后变化报 `assets.concurrent_hash`，登记元数据改变报 `assets.preview_changed`。重新预览使用新 tx，不覆盖旧备份。UI 修改 asset_id 或操作失败后必须重新预览。

资产 copy/rollback 的响应分别描述文件操作与验证：成功写入后生成 `post_receipt_id`，`static_verification` 为 `recorded`，运行面保持 `runtime_verification: not-observed`。若写后观察无法保存，响应仍表示文件操作已完成，静态验证为 `unavailable` 并附 `post_receipt_error`；asset lock 登记失败另附 `asset_lock_error`。不得把这些情况误报为“未写入”后盲目重试。回滚受独立授权，并拒绝覆盖后续编辑。

自取数 StateView（例如会话列表、实验详情）同路径刷新、取消或断网时保留上次成功数据并提示非最新；路径改变或服务端拒绝/缺失结果时清空旧数据。刷新恢复后以新响应替换。此规则不把缓存升格为当前事实。


比较页的结果绑定当前选择的两份 Receipt。更换任一选择立即清空旧 diff 并取消在途请求，迟到响应不再覆盖新选择；列表获取失败的重试会重新加载列表。跨类型或坐标的 `same_domain=false` 显式提示不可作为同域基线，字节相同不升级为语义等价。


资产 CLI/API 共用唯一事务和 provenance 恢复规则。每次复制独占备份目录，重复操作不覆盖旧 before-image；回滚同时恢复原资产登记，因此 SBOM 不保留已撤销的首次复制。若同一资产之后又复制过，即使字节相同，旧回滚也报 `assets.lock_conflict`，需先回滚新事务。备份中的 `project_digest` 必须匹配实际项目，缺失或跨项目报 `assets.project_mismatch`；旧事务缺少 provenance 历史报 `assets.lock_history_missing`，保留原备份供人工核对，不自动推测归属或伪造旧登记。
