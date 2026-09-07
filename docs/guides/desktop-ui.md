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
- Host/Origin 校验；POST 需 `X-Ctxpect-Client: desktop`
- UI 不计算 Claim

主要路径：`/api/v1/health`、`/inspect`、`/receipts`、`/doctor`、`/diff`、`/integrations`、`/settings`、`/sessions`、`/monitor`、`/policy`、`/exceptions`、`/standards`、`/sync`、`/advisor`、`/lab`、`/team/compliance`、`/care-plan/:id`、`/apply`、`/rollback`。

## 启动教程

见 [user-guide](user-guide.md)「只读桌面主链」。本文件不是已完成全 OS WebView 验收的声明。

## 本切片明确未做 / 未覆盖

- **C04 全套页面状态**：16 个入口里，Doctor 以外多数页面仍是 API JSON 转储，没有按页实现 empty/loading/error/partial/stale/offline/permission-denied/unsupported-version/connector-missing 与主要动作闭环。
- **应用截图、Tauri 桌面壳、OS WebView / a11y / 屏幕阅读器 / ≤2s 性能证据**：未采集，不得当作已覆盖。
- **例外批准身份源**：UI/API 不能用调用方自报角色完成 approve。身份来自仓内已登记 principals + 调用方持有的登记密钥，而密钥只经环境变量传入 CLI，HTTP 请求无法安全携带。因此 `POST /api/v1/exceptions` 明确返回 `api.identity_required`，例外生命周期只经 CLI。
- **i18n**：导航、Doctor、以及 Checkup / Inspector / Settings / Care Plan / Integrations 的页面正文已接进 zh/en 表。API JSON 转储字段名仍是英文协议键。`ui-unit` 只做字符串表 grep，不渲染组件。
