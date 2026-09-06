# Contexpect 可视化与完整产品闭环补充实施方案

> 状态：补充实施规范，尚未实施本文要求的桌面 UI 与完整产品闭环。
> 核对日期：2026-09-06。核对基线：`main`，`1b6ea2a46968d80a4721a0f246f915fb04a4d9d5`；开始核对时工作区干净。
> 本文补充现有 PRD 和 WP 的可执行交付边界，不新增 F-19/WP-13，不改变 acceptance cutoff，也不是完成或独立验收报告。

## 1. 结论、范围与依据

**面向用户的可视化尚未对应完成。** PRD §9.1 的 14 个入口，目前没有一个具有可运行的桌面实现与交互验收证据。Doctor 已有高保真图、组件和主要状态的文字规范；其他入口有程度不同的需求或路由说明，不能视为设计、实现和验收均完成。

当前可运行能力是 WP-02 的 Codex instructions 单坐标静态 `inspect`。它可支撑后续只读 UI 的一部分数据，但没有实现正式 Receipt、数据库、本地 API、桌面壳、原生运行时对账和变更闭环。完整闭环需要 UI 与其依赖的 core/runtime 同时落地。

本次核对覆盖 PRD 的用户流程、F-01–F-18、§9 全部入口与视觉规则、§13–§18 相关验收，现有设计 Markdown 与最终 Doctor 图、canonical 架构/指南/计划、Cargo workspace 和 CLI/resolver 实现入口。生成夹具只作为合同和测试输入，不把其数量当作已执行的产品覆盖。

| 依据 | 本次确认的事实 |
| --- | --- |
| [完整 PRD](../requirements/2026-09-04-contexpect-complete-product-requirements.md) §7–§9、§14、§17 | 14 个主入口，加上 Care Plan、Integrations 等流程页；要求真实数据、可访问性、隐私和端到端结果 |
| [Cargo.toml](../../Cargo.toml) | 仅 cli、collect、core、fs、resolve、schema 六个 crate；没有 UI workspace |
| [CLI 实现](../../crates/ctxpect-cli/src/lib.rs)、[输出](../../crates/ctxpect-cli/src/inspect.rs) | `inspect` 和 help；`dev-inspect-v0` / `development-snapshot`，不是正式 Context Receipt |
| [Resolver](../../crates/ctxpect-resolve/src/lib.rs) | Codex `0.147.0 / cli / macos-27-arm64 / instructions`；其他行为不可冒充已支持 |
| Git 文件清单及工作区检查 | `packages/ui`、`packages/ui-tokens`、`apps/desktop`、前端 manifest、Tauri 配置及 UI 测试均未存在 |
| [Doctor screen spec](../gpt-img-2-design/20260904-1200-contexpect-doctor/03-screen-specs.md)、[实施映射](../gpt-img-2-design/20260904-1200-contexpect-doctor/06-implementation-plan.md) | Doctor 是唯一 canonical 高保真屏；6 项浏览器/交互截图目标仍为 `not started` |
| [桌面指南](../guides/desktop-ui.md) | 已明确 Inspector、Compare、Sync、Standards 等仍需实现；没有 Tauri 窗口 |
| [integration contracts](../../acceptance/integration-contracts.yaml) | 9 项 integration 的 version pin 均为 `evidence-backed-unavailable`；尚不能据此宣称真实 executor 已集成 |

本文不重新授予历史任务预算，不把旧 RED 当作当前源码缺陷，也不把 commit 文案当作阶段验收。后续接续必须核对实际 control/state、候选、有效 review 与门禁证据；依赖未验时可在隔离副本收口，不能据此假称依赖已交付。

## 2. 全部用户可视化入口对账

以下 V01–V14 对应 PRD §9.1，V15–V16 是已有 WP-04/设计指南要求的配套入口。路由是本补充方案的实施落点；既有别名保留，不能建立两套数据模型。**表中所有入口的运行时代码和用户交互验收均未完成。**

| ID / 页面 | 需求和现有设计 | 建议路由 | 必须补齐的用户闭环与依赖 |
| --- | --- | --- | --- |
| V01 Overview / Checkup | §7.1、§9.1；有页面定位，共享 shell | `/checkup` | 解释 local-first → 选项目/授权 roots → 探测环境 → 生成/打开 Receipt → 展示 drift、Unknown、最近变化 → 下钻 Inspector/Doctor；WP-02/03/04 |
| V02 Inspector | F-06、§9.2；有视觉层级，无完整 screen spec | `/inspector` | 坐标切换 → 六 facet 独立泳道、scope、budget → item → resolution path → evidence/授权正文 → related finding；WP-02/03/04/05 |
| V03 Compare | F-07、§9.3；有矩阵要求，无详细交互稿 | `/compare` | 选择两个或多个坐标/Receipt → Intent/asset 矩阵与差异筛选 → native 表示/语义差异 → 首次 drift → baseline/policy；WP-03/04/06/11 |
| V04 Receipts | F-05；有导航与合同，无屏幕全状态 | `/receipts`、`/receipts/:id` | 六类 Receipt → Why/How → 验签/附件 → 脱敏预览与导出 → 再导入复验；删除保留 tombstone、派生失效、说明外部副本不可召回；WP-03/04/11 |
| V05 Assets | F-02/F-11；catalog 字段与导航已定义 | `/assets`、`/assets/:id` | inventory → 来源/license/权限/激活证据 → 安装或更新预览 → 唯一 executor → lock/SBOM → post-Receipt → disable/rollback；WP-02/08，写入依赖 WP-06/11 |
| V06 Sessions / Monitor | F-12/F-14/F-16；有 Monitor 定位 | `/sessions`、`/sessions/:id`、`/monitor` | 授权导入 → field-to-claim/coverage → timeline → Expected/Observed 对账 → 历史洞察与反例 → 删除并失效；Monitor 负责变更，不替代会话详情；WP-05/09 |
| V07 Effect Lab | F-15；仅流程与结果要求 | `/lab`、`/lab/:id` | 冻结实验 → 预算/隔离预览 → runner → 逐 run Receipt/门禁 → 分布/区间/混杂/四种决策 → evidence；WP-10 |
| V08 Sync | F-10；协议详尽，缺 UI 全状态 | `/sync` | desired state/secret refs → recipient/provider → bundle → B 端预览/冲突 → apply → 本机重新核对 → 分列 transport 与 semantic 结果 → 轮换/恢复；WP-06/07/11 |
| V09 Doctor | F-08/F-13；已有最终图、组件、主要状态 | `/doctor` | symptom/规则发现 → finding → 证据断点/下一证据动作 → deterministic PlacementRecommendation → Care Plan → recheck；Advisor 保持独立候选；WP-02/04/08/09 |
| V10 Policy | F-18；有分层与路由合同 | `/policy` | 查看五层有效 policy/唯一 authority → deny/Unknown 原因 → detect-only/enforceable → Approval/exception → audit；WP-11，基础判断须先于任何写入 |
| V11 Standards | F-09/F-10/F-18；有 ST1–ST8 合同 | `/standards`、`/standards/:id` | validate/publish → 成员 disclosure → per-harness preview/adopt/pin → update/status → leave/rollback/revoke → reconciliation；WP-06/07/11 |
| V12 Settings | §9.1/F-13/F-16/§13；只有设置类别 | `/settings` | privacy、retention、vault、analysis adapter、notification、resource limits 的编辑/验证/保存/撤销；删除、锁屏、离线、依赖缺失有可判定状态；WP-03/04/05/07/09/11 |
| V13 Exceptions | F-18/ST6；有生命周期合同，无完整界面 | `/exceptions` | request → 有权限身份 approve/reject → scope/use/time limits → revoke/expire → policy 重算与 audit；离线 stale 不放行；WP-11 |
| V14 Team compliance | F-18/ST7；有字段 allowlist，无界面 | `/team/compliance` | 成员 disclosure → 已签名标准和按 harness 的状态 → drift/loss/Unknown/freshness → redacted evidence/exception → 重验；WP-07/11 |
| V15 Care Plan / Recheck | F-09；Doctor 锁定态与 preview 目标已定义 | `/care-plan/:findingId` | 目标/authority/loss → native diff → policy/Approval → 并发校验 → apply/rollback → 新 Receipt → Compare；WP-06/11 |
| V16 Integrations | F-01/F-03/F-17；已有导航和 18-family fixture | `/integrations`、`/integrations/:id` | family/version/surface/capability → 安装/认证/connector 独立状态 → 来源/pin → conformance/importer gap → 指向实际受支持的证据动作；WP-02/08/11 |

不强制 16 个入口永远平铺在导航栏；可分组、搜索和渐进展示，但全部功能必须可达，不能把隐藏菜单、禁用按钮或空白占位页当作完成。Doctor 保持默认主入口，Overview/Checkup 提供总览。

## 3. 可视化共有合同与设计缺口

### C01 坐标、数据与异步一致性

- 完整坐标包括 device、environment、account alias、organization、policy snapshot、harness、version、surface、project、cwd、task、snapshot；展开后均可查看来源。探测值、用户声明与 Unknown 分开。
- UI 只消费统一 core API/版本化 DTO，不自行扫目录、运行 harness、计算 Claim 或决定 policy pass。开发 fixture、静态实测、原生实测必须有可辨识的数据来源。
- 切换坐标后旧请求不能覆盖新坐标结果；保留筛选/行选择有明确规则。扫描取消、失败与部分完成不能破坏上一有效 snapshot；旧数据仍可浏览但必须标 stale/刷新失败。
- `dev-inspect-v0` 到正式 Receipt 的兼容/迁移必须显式处理；不能只改 schema 名称把开发快照“升级”为签名凭据。

### C02 Inspector 与 Evidence

- 六 facet 以独立泳道/格子呈现；图中的边仅表示实际 resolution relation。提供表格替代与键盘下钻。
- 每个 summary 可展开 claim_kind、lifecycle_stage、truth_state、provenance、coverage、precision、knowledge_status；固定提供 “Why is this here?” 和 “How do you know?”。
- 覆盖 installed/discoverable/eligible/visible/use/effect、managed/team/user/project/nested/local/runtime scope；展示 include/exclude/ignore/override/shadow/cap/truncation/activation/unknown 原因。
- 预算分列 constant、conditional、skill catalog/body、tool schema/result、history、memory、current user、unknown；区分 byte/character/token、session 累计量和当前 occupancy。每个数字标单位、口径、坐标、精度和 evidence；Unknown 不画成 0，也不相加成看似精确总量。
- cap、截断位置、MCP schema、按需 skill 正文、compaction 都能跳到证据；仅有静态数据时相关运行时层显示有原因的 Unknown。

### C03 状态词表先纠偏

以下是现有设计文件与 canonical 数据合同的实质差异，实施前同步修正文案、typed token 和组件合同；以 PRD §4 为准，不照抄图片小字。

| 位置 | 差异 | 统一要求 |
| --- | --- | --- |
| `02-design-system.md` / Evidence Vocabulary | 将 Expected 称为 truth state | Expected 是 resolved context 的用户表述；`truth_state` 只有 present/absent/indeterminate/not-applicable |
| 同文件 / `SeverityMark` | 将 Unknown 放在 severity 枚举，后文又明确不是 severity | 严重性、finding 确认状态、知识状态分轴；Unknown 单独计数，可进入 evidence 状态但不伪装风险等级 |
| 同文件 / Adapter Coverage Taxonomy | Needs connector 汇总 not-installed、未授权、surface 缺失等 | compact 分组可保留，但每项必须展开实际 installation/authentication/connector/version/surface 状态；不能用一个 enabled/Active 替代 |
| 旧 shell/导航、`01-current-state.md` | 只列 10 个视觉入口，仍称技术栈未选择 | 扩展为 V01–V16 的完整可达 IA；技术栈遵循 ADR 0001，不重新选型 |
| Doctor fixture summary 与 finding 列表 | 3/2/4 与 4 行列表口径未解释 | 明确各计数统计对象、过滤条件、是否重叠；详情可复算，不凭图硬编码 |
| Privacy mode 文案 | 截图隐藏与可出站内容边界容易混为一个开关 | 显示遮罩、按住显示、复制确认、导出脱敏与 egress consent 分别生效；关闭遮罩不能自动授予发送正文权限 |

Doctor 当前 Indeterminate visibility finding 的 Treatment 锁定必须保留；导出、用户声明或 Advisor 建议都不能解锁。其他具体 action 的证据/授权前提按 F-09/F-18 定义，不从这张 fixture 推导新的全产品权限规则。

### C04 每页状态与行为规格

V01–V16 每页至少定义：入口和返回路径、URL/选择状态、DTO 和查询、主要动作、成功落点、empty/loading/error/partial/stale/offline/permission-denied/unsupported-version/connector-missing、取消/重试、持久化边界、敏感数据边界。对不适用状态给理由，不能机械制造伪状态。

Drawer/dialog 必须规定打开与关闭、焦点返回、Esc、未保存编辑处理、重复提交和执行中关闭行为。失败要显示 reason code 与可行下一步；“重试”不能扩大原授权。

### C05 隐私、安全与 Vault

默认敏感正文遮罩；按住显示可由键盘完成，结束按压/失焦后恢复遮罩；复制前确认。截图隐私模式覆盖 DOM、tooltip、aria label、图表替代表格、toast、导出预览和应用截图，隐藏路径、用户名、repo、prompt 片段、server URL。

不可信正文默认纯文本；Markdown、安全链接、远程图片、CSP、Tauri bridge allowlist、localhost API、deep link 均按 PRD §13.7 实测。deep link 不能直接 mutation、揭密、sync 或 LLM send。

Settings 与相关弹窗覆盖 metadata-only 不生成 vault key、create/unlock/auto-lock/锁屏、key rotation、中断恢复、丢钥提示、外部恢复及多用户权限。删除前说明范围与不可召回外部副本；删除后正文和派生分析不可查询，历史签名的 tombstone 语义保留。

### C06 桌面、无障碍、本地化和性能

- 保留现有暖白/深蓝、证据与严重性分离的设计语言；图像仅约束构图。固定 1440×900 主视口；1024–1279 drawer overlay；768–1023 主从布局；<768 只读 Receipt/通知。
- 实现 WCAG 2.2 AA、完整键盘、焦点/读序、screen reader、reduced motion、非颜色状态、图表表格替代；自动扫描不能替代实际键盘和辅助技术操作。
- 简体中文/英文首发同等覆盖；长路径、长译文、数字单位、错误信息和术语映射均测试。避免只把导航翻译而遗留英文流程。
- 已有 snapshot 时，coordinate bar、Receipt 时间、risk/unknown count、可点击 Inspector skeleton 同时可交互的首个有意义结果目标 ≤2 秒；后台刷新不阻塞浏览。虚拟列表不能破坏键盘定位与 screen-reader 语义。
- macOS/Linux/Windows 使用实际 Tauri WebView 验证；浏览器预览与 macOS 截图不能替代其余 OS lane。

### C07 真实动作与权威结果

所有按钮都要有实际行为、真实结果和失败边界。展示 preview 不算 apply；executor 返回 0 不算目标 harness 生效；bundle 传完不算语义等价；用户点击确认不算 native evidence；模型生成解释不算 deterministic finding。

无法支持的 surface 应完整实现 Unknown 和安全下一步；缺失本应 required-supported/required-write 的 runtime 则是未完成，不能永久用 Unknown/禁用按钮替代。

### C08 视觉验收产物

每页至少有默认、关键空/错态和主要动作前后截图，另对高密度/长文本/隐私/键盘焦点取证。Doctor 保留既有 6 个截图目标及 18-family DOM 检查；4/9/5 只属于视觉 fixture，live 必须重新计算。

产物记录页面/状态、fixture 或脱敏数据集、viewport、OS/WebView、locale、候选和实际执行结果。对每个状态绑定行为断言；截图通过不能替代后端结果或完整 E2E。原始本机日志和敏感截图保留在忽略的运行目录，可发布材料须脱敏。

## 4. 完整产品应形成的闭环

主链：**选择环境和任务 → 发现与 Expected → 导入声明范围内的 Observed → 对账并解释偏差 → Doctor/Care Plan → preview/授权/apply/rollback → 重新扫描/原生核对 → 新 Receipt/Compare/baseline → 下一次变更再核对。**

当前只覆盖“显式项目 → 单 anchor instructions 静态解释”这一小段。以下闭环必须全部实现；各条的 Unknown 分支是诚实结果，不意味着免除可支持部分的实施。

| ID / 闭环 | 必须打通的输入 → 动作 → 可核验结果 | 关键失败反例 |
| --- | --- | --- |
| L01 首次使用与环境 | 不登录选项目 → 可选授权 home → 安装/版本/环境探测 → inventory/Expected Receipt → Inspector/Doctor → 保存/复验 | 有配置残留就显示已安装；未知版本借用最新版；默认扫描私人 home |
| L02 任务前与运行时 | task/files/glob/cwd/profile → eligible 与预算 → preflight → 结构化 launch 或安全复制参数 → session 引用 preflight → observed 对账 | shell/env 注入；launch 后丢失 preflight id；agent-selected 猜概率 |
| L03 诊断与修复 | finding → evidence/path → PlacementRecommendation → 唯一 authority/native plan/loss → policy/Approval → 并发 guard → apply → post-Receipt → rollback/再次复查 | advice 自动写文件；hash 已变化仍覆盖；rollback 删除无关文件；静态结果冒充生效 |
| L04 Receipt 与 Diff | snapshot/六类 Receipt → ledger/签名 → 两两/多列 diff → baseline lock → 脱敏分享/离线复验 → 删除 evidence/派生失效 | 裸低熵 digest 外泄；不同比较域误判相同；Unknown 可建通过 baseline；签名状态与附件删除混淆 |
| L05 设备迁移与同步 | allowlist/desired state/secret refs → Git/folder E2EE → recipient 管理 → B 端 preview/conflict/apply → reconciliation → revoke/rotate/recovery | 明文 secret；last-write-wins；回放/分叉被接受；transport success 直接显示 verified |
| L06 团队标准与治理 | 签名 standard → disclosure → native projection/adopt/pin → 分层 policy → exception → redacted leader view → update/leave/rollback/revoke | personal 放宽 required；过期例外复用；detect-only 显示 enforceable；成员正文进入 leader |
| L07 生态资产 | catalog/APM manifest/lock → 来源/pin/license/权限预览 → executor install/update → SBOM/lock → recheck → disable/rollback | Contexpect 重写 APM evaluator/projector；双 authority；缺许可仍复制执行 |
| L08 Session 与历史 | 用户授权导入 → field-to-claim → partial timeline → 按窗口/项目/任务/harness/device 分析 → 样本和反例 → 删除/重算 | 补造 event；session 累计 input 当 occupancy；未导入历史称从未使用；删除后 insight 仍可查询 |
| L09 Advisor | 选定 evidence → 脱敏 payload preview/逐次同意 → AnalysisAdapter → 带 evidence 的候选 → 用户形成 intent → 独立核对/修复流程 | 自动全仓上传；候选进入 Claim/policy/CI/baseline；接受建议直接解锁 Treatment |
| L10 Effect Lab | task suite/control/treatment → 冻结 contract/预算/隔离 → versioned runner → run Receipt/quality gate → 混杂/ITT/区间 → 四种 effect decision | 单次 A/B 称因果；未显著称等价；看结果后改样本量；无真实 runner 只展示 mock 图表 |
| L11 持续、定期、事件与集成 | 增量变更 → snapshot/diff → actionable 通知 → 用户下钻；四周期报告；CI/IDE/deep link → policy/Receipt → 修复重验 | 重复 digest 重复通知；旧数据冒充实时；无 daemon one-shot 不工作；CI indeterminate exit 0 |

## 5. 补充实施顺序

以下 **6 个实施段是依赖顺序，不是 6 次独立 review，也不重置任务或返工预算**。沿用 WP-01–WP-12 和有效阶段合同；默认完成整个授权验收包后统一 review。本方案新增中途独立检查点数为 **0**。如果接续时核实已有有效合同要求独立阶段验收，必须保留该边界，不能用本方案的默认值豁免。

### S1 基线接续、契约和全页面规格

1. 核对当前 HEAD、工作区与仍有效的阶段证据；复用已验证 core/schema/fs/collect，不重建平行体系。对未验依赖在隔离副本收口。
2. 为 V01–V16 补齐 C01–C08 的 screen/interaction specification，写回 `docs/guides/desktop-ui.md` 和现有设计包；明确组件、DTO、动作、空错态、权限、返回路径及测试。修正 C03 冲突。
3. 在现有 schema/架构中冻结 Receipt、Evidence、Diff、Finding、TaskProbe、本地 API、redaction、版本迁移与取消/进度合同，区分开发快照导入和正式 Receipt。锁定 node/package manager/Tauri 等实际版本、lockfile 与可复现命令。
4. 更新已过时的“没有 crate/可执行 CLI”“只需四条门禁”“技术栈未选”说明；修正 README 的 ignore 示例。`__ignore__04` 不是 instructions 排除示例，应核对 `__instructions__negative` 的 `.ctxpect-ignore`/G4。
5. 九项外部 integration 分别确认消费者、authority、冻结版本/来源/license/digest/能力和失败分支；不可凭空补 pin。有来源缺口的保留阻塞，继续独立的本地实现。

有限完成判据：V01–V16 每页均有可执行状态/动作合同，C03 无冲突，依赖/DTO/迁移和命令可确定，干净候选文档检查不依赖忽略文件。S1 完成不能声称任何页面已实现。

### S2 可测试的只读桌面主链

完成 WP-02 剩余 environment/inventory/resolver/Doctor/preflight 核心，WP-03 store/ledger/Receipt/diff，以及 WP-04 Tauri/local API/共用组件。四 anchor 与扩展 family 按冻结矩阵实施，而非把当前单坐标 CLI 直接包装成全工具产品。

先打通真实数据的 L01/L04 和 L03 的诊断部分：V01/V02/V03/V04/V09/V16、Settings 的隐私/存储基础，包含坐标选择、六 facet、预算、证据下钻、脱敏导出/复验与失败恢复。再补全 L02 的 preflight；runtime 连接由 S3 完成。没有 daemon 时仍能启动桌面 one-shot 数据服务及 CLI 核心检查。

最早的用户可测节点：在真实 Tauri 窗口中选一个受支持的临时项目 → inspect → Inspector/Doctor → 证据 → Receipt 导出和再验证；缺失 runtime 显示 Unknown。通过对应内部验收后可进行 PRD §18 的形成性反馈；不等于全产品测试就绪。

### S3 原生证据、会话与持续观测

完成 WP-05 与 F-14 的确定性历史基础：声明的 importer/field-to-claim、手动文件/stdin/wrapper/API、目标环境 collector 导入、preflight/session 关联、timeline、历史查询/删除、daemon 增量与 scheduler。

打通 L02/L08/L11 的观测链，完成 V06 与 Monitor、Settings 的 retention/通知/资源控制；V02 追加真实 compaction/tool/skill/budget 图层。受控执行冻结的 Codex/Grok oracle recipes 并保留脱敏证据；只有已有授权和精确版本可用时才能实际执行。尚缺的 OS、登录、依赖和 oracle 结果分列未验，不用模拟结果替代。

### S4 安全变更、同步、生态与团队治理

按依赖先实现 WP-11 的 context policy/Approval/authority/audit 基础，再实现 WP-06 native projection/transaction/rollback，然后接 WP-07 sync/standard、WP-08 assets，最后接 V14 leader view。不得先开放 Apply 再事后补 policy。

完成 V05/V08/V10/V11/V13/V14/V15，连通 L03/L05/L06/L07；现有 V02/V03/V04 显示 native plan/loss/transaction/reconciliation。每个 required-write cell 都执行 preview/apply/rollback/post-Receipt；合成用户根/设备/密钥与本地测试 executor 可用于开发，真实安装/团队发布/同步目标另受实际授权控制。

### S5 建议与效果验证

完成 WP-09/WP-10 和 V07、Doctor Advisor panel、Session insights、analysis settings。L09 的候选回到同一 Intent/Care Plan，L10 的实验结果回到对应 coordinate 的 Effect facet。

接入冻结版本的成熟统计库和已有 runner，不新建通用评测平台。四种预构造结论与至少一个真实小型 runner fixture 分别取证；付费模型调用、外部发送与真实写入必须有具体预算/目标授权。无法调用真实服务时仍完成本地边界和失败测试，真实 gate 保留未完成。

### S6 全矩阵、全路由、恢复与交付验收

完成 WP-11 其余 CLI JSON/API/SDK/IDE/CI/export 与安装包，执行 WP-12。V01–V16、C01–C08、L01–L11 均有运行证据；补齐用户帮助、测试教程、migration/rollback、安装/升级/卸载说明。

全部 required OS/harness/capability/projection lane，development/sealed/live 三套结果，性能、72h soak、四周期、50-session、8 人内部可用性均按原合同执行。缺机器、密钥、runner、参与者时保留明确未完成；不得自动生成“人工评价”或把计划中的截图路径当作实际截图。最终全产品集成与独立 readback 通过才可宣布完整交付；发布与签名使用仍需对应授权。

## 6. F/WP 与本补充验收的覆盖

| 原需求 | 主要 WP | 本补充覆盖 |
| --- | --- | --- |
| F-01 | WP-01/02 | V01/V16、L01、C01 |
| F-02 | WP-01/02 | V02/V05、L01/L07 |
| F-03 | WP-01/02 | V02/V16、L01/L02、C02/C03 |
| F-04 | WP-02/04/05 | V02/V06、L02 |
| F-05 | WP-01/03 | V04、L04、C01/C05 |
| F-06 | WP-04 | V02、C01–C08 |
| F-07 | WP-03/04 | V03、L04/L05/L06 |
| F-08 | WP-02/08 | V09/V15、L03/L07 |
| F-09 | WP-06 | V03/V11/V15、L03/L05/L06 |
| F-10 | WP-07 | V08/V11/V14、L05/L06 |
| F-11 | WP-01/08 | V05/V16、L07 |
| F-12 | WP-05 | V06、L02/L08 |
| F-13 | WP-09 | V09/V12、L09 |
| F-14 | WP-09 | V06、L08 |
| F-15 | WP-10 | V07、L10 |
| F-16 | WP-05/11 | V06/V12、L11 |
| F-17 | WP-01/11 | V04/V16、C05、L04/L11 |
| F-18 | WP-01/11 | V10/V13/V14、L03/L06/L11 |

WP-12 对全部行集成验收。沿用 `acceptance/traceability.csv`，在对应实现阶段补实现位置/测试与 UI 证据，不另建相互冲突的 F/WP 真值清单；本文 V/C/L ID 只补充用户入口和跨包连接。

## 7. 可判定验收与门禁

### 7.1 当前九条 required gate

以下命令从仓库根运行，离线、不开私人配置和会话；不安装 pip 依赖。每条分别记录实际退出码，不能仅取组合脚本最后一条的状态。

| 名称 | 命令 |
| --- | --- |
| docs-structure | `python3 scripts/check_docs.py` |
| acceptance-validation | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'` |
| cargo-build | `cargo build --workspace` |
| cargo-test | `cargo test --workspace` |
| cargo-clippy | `cargo clippy --workspace --all-targets` |

```bash
# cwd：包含 Cargo.toml 与 docs/README.md 的 ContextView 仓库根
export CARGO_NET_OFFLINE=true
python3 scripts/check_docs.py
python3 scripts/check_acceptance.py --structure
python3 scripts/check_acceptance.py --traceability
python3 scripts/check_acceptance.py --corpus
python3 scripts/check_semantic_team.py
TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

### 7.2 随实施新增的真实门禁

当前没有前端 package manifest 或可执行 UI gate，不能把尚不存在的 `pnpm test`/Tauri 命令报告成已通过。S1/S2 创建相应真实脚本时，把精确 cwd、env、命令、版本、fixture、lane 和失败判据写入 canonical test-strategy 与项目门禁；原九条继续保留。至少覆盖：

| 验收项 | 必须执行的检查与失败判据 |
| --- | --- |
| UI build/type/unit | 实际 packages/ui、tokens、Tauri bridge DTO 编译；类型漂移/共享语义不一致即失败 |
| UI route/interaction E2E | V01–V16 全入口，主要动作、返回/刷新/取消、C04 适用状态；只有链接或 mock response 不算产品闭环 |
| Desktop visual/a11y/privacy/i18n | C05/C06/C08；实际窗口、所有关键键盘动作和屏幕阅读器操作；任一敏感显示路径遗漏失败 |
| Runtime conformance | 冻结 compatibility/capability/projection 全 required 单元；UI/CLI/Receipt/Gate 的相同 claim 结果一致；不抽样推断 |
| Security/recovery | PRD §17.3 全表；XSS/deep-link 到 bridge、secret 泄漏、并发覆盖、删除残留、in-doubt 被显示成功均失败 |
| Cadence/performance | 10 万文件/1,000 asset/100 symlink/20 nested 数据生成器；每 lane 冷热各 5 次；PRD §14.2 阈值、72h/1,000 变更、4 周期、50-session |
| Human usability | 8 名符合 §17.2 条件的 QA、6 项任务各 ≥7/8 成功并点击证据；时间阈值沿用 PRD，不由开发者代答 |
| Package/CI/release | 干净候选可复现、各平台安装/升级/迁移/卸载、SBOM/NOTICE/license、远端 required checks；pending/RED/缺设备均非通过 |

独立 review 的统一验收包为 V01–V16 + C01–C08 + L01–L11 + F-01–F-18/WP-01–WP-12 原合同。阶段进展与全产品交付分别报告。每条“完成”至少绑定实现、真实测试/操作证据、候选和适用范围；未验只能记未验。

### 7.3 干净候选与文档可复现

原基线的 PRD §18 和 implementation-plan WP-02 曾链接一个被忽略的内部交付文件；本次补充文档将其改为公开 canonical 计划，保留原有形成性反馈和排程语义。后续必须在仅含受版本控制候选文件的临时副本验证文档，不能依赖本机忽略文件让门禁通过。

未提交候选导出必须显式纳入新增可发布文档、源码和测试，同时排除 `.octoworkflow/`、内部交接、私人日志和 sealed answers。内部 prompt 与任务记录另由 supervisor 提供给实施/评审，不让公开规范依赖其存在。

## 8. 文档补齐责任与完成口径

| 应补文档 | 具体内容与落地时机 |
| --- | --- |
| desktop-ui / 现有 design spec | S1 定义全页路由/交互/状态与 C03 修正；随实现补实际截图和操作证据，图与实现状态分别标注 |
| data-and-truth-model / schema boundary / API | S1/S2 正式 DTO、Receipt 迁移、签名/删除、查询/分页/取消/进度、reason codes；不把 legacy canonical JSON 冒充通用签名规范 |
| user-guide / cli-reference | 每个已实现入口的可复制教程、输入、结果/退出码、Unknown 示例、实际支持范围；区分预期功能与可用功能 |
| integration-contracts / dependency-and-provenance | S1 识别缺 pin；使用前补可核验版本/license/digest/authority 和 secret/rollback 能力，不能重写 cutoff 掩盖缺证据 |
| privacy / encrypted-sync / operations | S2–S4 补 vault、删除、key 生命周期、恢复、不同 OS/多用户、executor 和 remote collector 操作手册 |
| advisor/effect-lab | S5 补发送披露、候选接受后的独立核对、历史失效、实验冻结表单、真实 runner 和失败/混杂结果解释 |
| test-strategy / reference-hardware / release | 随阶段补真实可执行命令和状态，S6 补全矩阵/用户验证/发布证据；不以文档或生成 fixture 证明 runtime |

完成本文是“补充方案已准备”，不是“产品已闭环”。现在可测静态 CLI；S2 内部验收后可测只读桌面主链；S3–S5 逐步增加真实交互；只有 S6 和原完整验收通过后，才可把全部用户可视化和完整产品称为完成。
