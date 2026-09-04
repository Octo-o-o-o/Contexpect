# Contexpect 完整产品需求方案

> 日期：2026-09-04  
> 状态：完整需求候选基线（已完成方案修订、高保真原型与 18-family adapter 范围对齐；尚未实施）  
> 暂定产品名：Contexpect  
> CLI：`ctxpect`  
> 核心产物：Context Receipt  
> 产品主张：Expected. Observed. Reconciled.  
> 研究依据：[AI Coding Context 管理项目研究台账](../research/2026-09-04-context-management-research-ledger.md) · [Agent / Harness 扩展调研](../research/2026-09-04-agent-harness-expansion.md)  
> 外部方案快照与追踪：[用户提供方案的来源快照清单](../research/source-snapshots.md)

## 0. 文档约定

本文定义的是一次完整交付的产品，不用试验版或功能子集替代最终范围。所有标为“必须”的能力都进入同一完成合同，只有全部工作包集成并通过最终验收，项目才算做完。

这不意味着开发可以无依赖地同时开始。解析器、证据模型、可视化、同步、分析和历史评估存在客观依赖，本文会给出交付顺序；顺序只用于降低返工，不改变最终范围。

本文同时主动排除已经被研究证明属于过度设计或错误承诺的部分，例如自建通用 marketplace、默认流量中间人、隐藏 prompt 提取、图数据库和无证据的“价值分”。这些是产品边界，不是未完成项。

## 1. 产品定义

### 1.1 一句话

Contexpect 是一个 local-first 的可视化 AI coding context 核对与控制工具：它解释指定设备、工具版本、项目、工作目录和任务下，哪些上下文按规则应该出现，哪些被运行时实际观察到，哪些仍不可见，并帮助用户安全地对齐、同步、管理和验证这些上下文。

### 1.2 用户真正购买的结果

用户不是为了看文件树而来，而是为了快速回答：

1. 这个 AI coding agent 此刻可能知道什么、为什么知道？
2. 哪些内容已被原生运行时证明确实可见，哪些只是按规范推演？
3. 为什么同一项目在 Codex、Claude Code、Cursor、Grok Build 上行为不同？
4. 为什么设备 A、设备 B、容器、SSH 或云端任务不一致？
5. 哪些上下文占空间、重复、冲突、过时、泄密、未加载或被截断？
6. 如何把同一意图安全地投影到不同工具，而不掩盖语义损失？
7. 一条规则或 skill 是否真的改善了质量、时间或成本？证据是什么？

### 1.3 产品类别

`Context observability + reconciliation + safe management`。

Contexpect 处在以下工具之间，但不替代它们：

- coding harness 自带的 `/context`、`grok inspect`、debug/log surface；
- AGENTS.md、CLAUDE.md、Cursor Rules、skills、plugins、MCP；
- APM、Agentpack、agentsync 等 package/deployment/sync 工具；
- Scopeon、ctxray、ContextSpy 等 runtime/session observability 工具；
- Git、dotfiles、secret manager 和组织 policy 系统。

它的独特职责是把这些来源对齐到同一个证据模型，并生成可解释的 Context Receipt。

## 2. 背景与机会

### 2.1 问题为何扩大

AI coding context 已从用户输入的一句话变成一组跨层级、跨工具、跨设备和跨运行时的配置系统：

- instruction：全局、managed、team、user、project、nested、local；
- rule：always、glob/path-scoped、agent-selected、manual；
- skill/plugin：启动目录元数据、激活正文、按需资源、脚本；
- MCP/tool：server 配置、工具 schema、权限、调用结果；
- memory/session：自动记忆、对话历史、压缩摘要、`@` 文件、steering；
- execution：cwd、Git 根、worktree、容器、SSH、云端 agent、账号策略；
- dynamic routing：tool search、skill discovery、subagent 与单独上下文窗口。

每个 harness 有自己的加载顺序、覆盖规则、体积限制、兼容层和不透明面。跨工具、多设备使用越深入，漂移越接近默认状态。

### 2.2 为什么现有工具没有完整解决

- 原生诊断通常只看一个工具或一个会话。
- 配置同步工具擅长 desired state，但未证明本轮实际注入了什么。
- 会话分析工具擅长 token/cost，却不知道磁盘上的某条意图为何缺失。
- package manager 擅长安装和 lock，不解释跨 harness 的 effective semantics。
- 通用 dotfiles 工具只复制文件，不理解加载顺序、scope 和有损映射。

Contexpect 的机会是建立一个跨工具的 reconciliation layer，而不是再做一份配置源。

### 2.3 价值假设

产品优先创造五类价值：

1. 可解释：把“agent 怎么突然这样做”从猜测变成证据链。
2. 可复现：让同一意图在设备、工作目录和 harness 间可核对。
3. 可优化：识别常驻税、重复、冲突、过时和无效激活。
4. 可治理：在不上传正文的前提下提供 drift、来源和审计证据。
5. 可验证：用受控对照判断某项 context 是否改变结果，而不是相信模型点评。

公开研究对 correctness 的结论并不一致，因此产品不承诺“安装后普遍提高代码正确率”；它承诺提供做出可靠判断的观察和实验能力。

## 3. 产品原则

1. 真值优先于自动化：先解释发生了什么，再允许修改。
2. Unknown 是合法结果：不知道时清楚说不知道。
3. Native oracle first：优先原生 runtime/API/log，其次源码解析、官方规范重建，最后才是启发式。
4. 一切结论有出处：任何“已生效”“缺失”“截断”“有损”都能回答“为什么”和“证据是什么”。
5. 对齐意图，不强求文件相同：允许 harness 方言和 native overlay。
6. Local-first、metadata-first、private-by-default：正文和历史默认留在本机。
7. 管理动作可预览、可回滚：不静默覆盖，不静默丢字段。
8. 权限不是 Prompt：instructions 与 sandbox/deny/approval policy 分开呈现。
9. 确定性检查和 LLM 建议分开：前者可进 CI，后者不得作为唯一门禁。
10. 标准与现有生态优先：不重造 package manager、通用 marketplace、Git 或 OpenTelemetry。
11. Adapter 按版本和 surface 维护：不存在一个永远正确的“Cursor 规则”。
12. 完整交付，边界有限：把定义内的事情做完，不把“未来所有 agent 和组织系统”混入完成条件。

## 4. 真值与术语模型

### 4.1 Context item 六个证据 Facet

| Facet | 名称 | 判定问题 | 可能证据 |
|---|---|---|---|
| 1 | Installed | 它是否存在？ | 文件、配置、包、server 声明 |
| 2 | Discoverable | 当前 harness/version/surface 是否会发现？ | resolver、官方规范、源码 |
| 3 | Eligible | 当前 cwd/task/files/policy 是否满足激活条件？ | glob、description、选择记录、任务 probe |
| 4 | Model-visible | 是否有证据证明模型能看到内容或目录元数据？ | 原生诊断、runtime export、受控 wrapper |
| 5 | UseEvidence | 是否观察到调用、明确引用或与规则一致的行为线索？ | invocation/reference/behavior trace；不等于模型内部归因 |
| 6 | Outcome-affecting | 它是否改变了质量、时间、成本或失败类型？ | 有门禁的受控 A/B；默认未知 |

UI 不能用一个绿色“Active”覆盖这六个 facet，也不能暗示前一项必然推出后一项。

### 4.2 三类上下文对象

- `Resolved Context`：按工具规范和当前解析坐标计算的 expected context。
- `Observed Context`：原生或受控运行时导入的 observed context。
- `Context Effect`：对照实验产生的 outcome evidence。

### 4.3 Context Receipt

Receipt 是一次上下文核对的不可变摘要，至少包含：

- receipt id、schema version、生成时间；
- device/environment profile；
- 不暴露 PII 的 account profile、organization context 和 policy snapshot；
- harness、version、surface、model（若可知）；
- project root、Git root/commit（若存在）、cwd；
- task probe、引用文件和 session id（按隐私设置）；
- resolved items、observed items、unknown surfaces；
- context budget：exact/estimated/unknown；
- resolution edges、shadow/override/truncation/loss；
- drift 与 issues；
- evidence ledger、collector/resolver version、按敏感级别生成的本地 keyed digest 或对外 redacted digest；
- redaction policy；
- 可签名的 canonical digest。

默认 Receipt 不包含完整 prompt、源码正文、tool result 或 secret；正文只在用户本地显式打开时显示。

### 4.4 多轴 Claim 与证据标签

一个状态结论不是 ContextItem 的固有属性，而是绑定 `Receipt + Snapshot + 完整解析坐标` 的 `ContextStateClaim`。同一资产在不同 harness、cwd、task 或 session 下可以有不同 claim。

标签不能压成单个互斥枚举，必须分别表达：

| 轴 | 可选值 | 回答的问题 |
|---|---|---|
| `claim_kind` | resolved / observed / effect | 这是规范推演、运行时观察还是效果结论？ |
| `lifecycle_stage` | installed / discoverable / eligible / model-visible / use-evidence / outcome-affecting | 结论描述六个 facet 中的哪一个？ |
| `truth_state` | present / absent / indeterminate / not-applicable | 对该命题知道什么？ |
| `provenance` | native-runtime / native-log / harness-source / official-spec / heuristic / user-attested | 结论来自哪里？LLM 不在此轴。 |
| `coverage` | full-declared-surface / partial-declared-surface / unknown | 证据覆盖声明 surface 的多少？ |
| `precision` | exact / derived / estimated / not-applicable | 数字或结构的精度如何？ |
| `knowledge_status` | current / stale / conflicted / unknown | 证据当前是否可用？ |

例如，Codex `debug prompt-input` 可以表达为：`observed + model-visible + present + native-runtime + partial-declared-surface + exact（只对返回的 message/character count）+ current`。这避免“观测是真实的但覆盖不完整”无法表达。

`UseEvidence` 再拆为：

- `invocation-observed`：观察到某 skill/tool/MCP 被调用，只证明调用发生。
- `reference-observed`：输出或 trace 明确引用某项内容，只证明引用发生。
- `behavior-consistent`：行为与规则一致，但不能证明由该规则导致。
- `internal-attribution`：模型内部是否依赖该内容，除非 harness 提供语义明确的原生证据，否则恒为 `indeterminate`。

所有 `indeterminate/unknown/stale/conflicted` 必须给 reason code。UI 可用组合徽标，但任何 summary 都必须能展开到完整多轴 claim。

### 4.5 Claim 合法组合与核对优先级

WP-01 必须生成机器可读 `acceptance/claim-validity-matrix.yaml`，至少执行以下不变量：

- `resolved` 可对 installed/discoverable/eligible 给出 present/absent；不能仅凭规范把 model-visible、use-evidence 或 outcome-affecting 写成 present。
- model-visible 的 present 需要语义明确的 native runtime/log evidence；静态规范最多支持 eligible，或对 model-visible 给 indeterminate expectation。
- use-evidence 的三类外部线索分别记录；任何线索都不能把 internal-attribution 改为 present。
- outcome-affecting 只有通过冻结 ExperimentContract 的 effect claim 才能得到 `supported-*` decision；`inconclusive` 可以是有效结果，但不能被编码为 absent 或成功效果。
- 未暴露的 timeline capability 生成 capability claim `indeterminate/unknown`，不生成伪事件。

同一坐标出现冲突时，优先比较覆盖范围和新鲜度，再按 `native-runtime > native-log > matching harness source > version-matched official spec > heuristic > user-attested` 排序。较高来源不能覆盖其未覆盖的字段；两个 current、同覆盖的高质量证据矛盾时保留 `conflicted`，禁止静默择一。LLM suggestion 不属于 Claim provenance，永不进入该序列。

### 4.6 Context capability taxonomy

`context-capability-matrix.yaml` 使用下列版本化类别，任何发现项或动态事件必须归入一类；无法分类时使用 `other-unknown` 并阻止“完整覆盖”结论：

1. system/developer/managed/team/user/project/nested/local instructions；
2. conditional/manual/agent-selected rules；
3. user/auto/subagent memory；
4. skills 与按需 references/assets/scripts；
5. plugins/extensions；
6. custom commands 与 reusable prompts；
7. agent/subagent definitions、delegation 和 child-context boundary；
8. MCP server instructions、tools、prompts、resources；
9. built-in/MCP tool schema、tool search/catalog metadata；
10. tool/MCP invocation 与 result；
11. current user/task message、`@` files、attachments、retrieval/file reads；
12. conversation history、assistant messages/prefill；
13. compaction summary、steering、handoff；
14. environment/repository/worktree/model metadata；
15. model-visible permission/policy/sandbox description（与实际 enforcement 分开）；
16. `other-unknown`。

每个 `harness + version + surface + category` 单元只能标 `required-supported`、`required-unknown-honesty` 或 `not-applicable`，并链接 collector/importer 和 field-to-claim mapping。矩阵是对“完整上下文”承诺的边界，不允许只实现文件类资产后把动态类别省略。

### 4.7 Semantic reconciliation 与等价 Receipt

同步或投影后的语义核对只允许四种状态：

- `verified`：desired-state revision 与 required intent set 相同；§17.0 中全部 required-write projection cell 已应用；唯一 authority 的 plan digest 与实际 transaction 一致；目标 resolver 能发现并判为 eligible；没有未批准 loss；绑定的 policy snapshot 确定通过；凡矩阵要求 native oracle 的字段，Observed 也匹配。
- `structural-only`：文件/manifest/plan 与 Expected 结构一致，但至少一个 required observed surface 不可取得。它不能被显示为“已经生效”或“跨设备完全一致”。
- `indeterminate`：缺少 resolver、policy、identity、runtime 或版本证据，无法完成合同内核对。
- `failed`：required intent 缺失、authority plan/transaction 不一致、出现未批准 loss、policy deny，或可观察的 native result 与 Expected 不符。

算法按固定顺序执行：验证 Receipt/schema/signature → 比对 resolution coordinate 与 policy binding → 比对 desired-state revision/intent membership → 验证每个 required projection transaction → 用目标 harness resolver 重扫 → 按 capability matrix 调用 native oracle → 汇总 loss/Unknown。任何较早步骤失败都保留后续未执行项，禁止用总分掩盖。

两个 Receipt 只有在同一 `EquivalenceProfile` 下才可称等价。Profile 必须固定 schema major、desired-state revision、required intent set、policy snapshot、required claim tuple 与允许的 device-specific 差异。device id、绝对 home 路径、mtime、本机 signer 和本地 keyed digest 可以按 profile 忽略；asset UUID、intent revision、scope、native target、loss、policy result 和 required claim 不可忽略。不能比较的本地 keyed digest 只能产生 `indeterminate`，不能被正文相似度替代。

### 4.8 生命周期 Facet 与 Effect claim 编码

六个 lifecycle stage 是对同一 item 的六个独立 facet，不是单调状态机：Installed 不蕴含 Discoverable，Observed UseEvidence 不蕴含 OutcomeAffecting；UI 不得用一条“必然向右推进”的箭头误导。

每个 stage 的 `value_json` 使用独立 schema：

- installed：安装位置、revision、owner、integrity；
- discoverable：发现入口、搜索根、命中规则；
- eligible：task/cwd/scope 条件与解析路径；
- model-visible：原生 message/segment id、位置、可见计量与 partial boundary；
- use-evidence：`invocation-observed`、`reference-observed`、`behavior-consistent` 和恒独立的 `internal-attribution`；
- outcome-affecting：experiment id、contract digest、decision、effect estimate/interval、sample、validity 与 protocol deviation。

OutcomeAffecting 的 `truth_state=present` 只表示“存在一份有效格式的 effect result”，具体结论写入 `value_json.decision`。`inconclusive` 是合法 decision，不等于 `truth_state=absent`；实验合同失效或证据不足以形成结果时才使用 `indeterminate` 并给 reason code。

## 5. 潜在用户

### 5.1 Persona A：多工具个人重度用户

特征：同时用 2–4 个 coding agent；有个人 AGENTS/CLAUDE、skills、MCP；在台式机、笔记本和远程环境间切换。

核心问题：脑内维护所有加载规则；“明明配过”但不知道在哪一层；换设备后行为改变；上下文越来越胖。

高频用法：任务前 Receipt、常驻 drift 提醒、换机同步、异常时诊断、月度清理。

回报：减少重复解释和试错；快速定位哪个工具/设备缺配置；降低精神负担；保留对工具的控制感。

### 5.2 Persona B：自由职业者、顾问与 agency 工程师

特征：频繁切换客户仓库；不同客户允许的工具、MCP、数据发送和规则不同；个人全局偏好可能污染客户项目。

核心问题：scope 泄漏、客户配置混用、秘密或私有 skill 误进入另一个项目、环境难以复现。

高频用法：进入项目时 preflight、客户 profile 切换、session 后 receipt、离场清理和导出。

回报：减少跨客户污染和合规风险；更快建立工作环境；向客户交付可复现的上下文清单。

### 5.3 Persona C：AI-native 小团队 Tech Lead / DevEx

特征：团队成员自选 Codex、Claude、Cursor 或 Grok；项目已有多套规则；常见问题由少数熟悉配置的人救火。

核心问题：团队规范在某些 agent 上形同虚设；新成员配置时间长；配置 PR 无法预知影响；难分辨工具 bug 和项目 context bug。

高频用法：配置 PR diff、CI conformance、团队基线 Receipt、每周 drift、onboarding bundle。

回报：减少支持和 onboarding 成本；提高跨工具一致性；把隐性经验变成可审计资产。

### 5.4 Persona D：开源维护者

特征：贡献者使用不同 agent；维护者希望项目约定可被主流工具读取，但不想维护四份相互漂移的文档。

核心问题：文档兼容性未知；嵌套规则和技能在不同工具上触发不同；贡献 PR 可能修改上下文供应链。

高频用法：仓库 conformance report、Context Receipt 随 release 发布、配置变更 CI、公共规范包导出。

回报：降低贡献者环境差异；使 agent 指令成为可测试项目接口；减少重复文档。

### 5.5 Persona E：Skill / Plugin / MCP / Harness 作者

特征：开发可复用 agent 资产或新 harness，需要验证发现、激活、权限、跨工具投影和版本兼容。

核心问题：在自己的机器可用不代表用户可发现；元数据、路径、权限和 schema 变化难以覆盖；有损转换难以解释。

高频用法：fixture lab、adapter conformance、发布前安全扫描、兼容矩阵、用户 bug receipt。

回报：更快复现兼容问题；减少支持工单；用 Receipt 作为可交换的 bug artifact。

### 5.6 Persona F：研究与评测工程师

特征：研究 context 文件、skills、tool schema、compaction 或不同 harness 对 agent 结果的影响。

核心问题：实验环境和 prompt surface 不可复现；“with/without context”并非单一变量；token 与 outcome 口径混乱。

高频用法：锁定配置、批量生成 receipts、A/B runner、门禁与统计导出。

回报：降低实验污染；明确 unknown surface；产出可审计、可复跑的数据集。

### 5.7 Persona G：企业 AI Platform / Security / Compliance

特征：组织允许多个 agent，但需要限制来源、MCP、数据外发和可执行 skills；管理员不应默认读取开发者源码和 prompt 正文。

核心问题：shadow configuration、未批准组件、设备 drift、无法证明某策略在哪个 surface 生效。

高频用法：metadata-only fleet receipt、policy CI、例外审批、季度供应链审计、事件调查。

回报：获得最小暴露的治理证据；更快识别违规 surface；保持工具选择弹性。

完整产品支持该 persona 的本地 policy、签名 Receipt、CI 与导出接口，但不在本次范围内自建多租户 SaaS、SSO/SCIM、计费和企业管理后台。

### 5.8 Persona H：单工具但复杂仓库的普通开发者

特征：只长期使用一个 coding agent，但仓库是 monorepo，存在祖先/嵌套规则、多个 worktree、团队配置、skills 和 MCP；本人不想研究 harness 内部实现。

核心问题：工具没切换，cwd、版本、工作区或项目规则仍会改变上下文；入职或升级后很难解释行为变化。

高频用法：项目 onboarding、工具升级后的基线 diff、偶发诊断、配置 PR 检查。

回报：无需成为多工具专家也能看到继承链、截断、冲突和未知面；把“工具是不是坏了”转化为可复现证据。

## 6. 使用节奏、问题与回报

使用节奏是同一产品的入口模式，不拆成彼此不兼容的版本。

| 使用模式 | 触发 | 用户动作 | 解决的问题 | 直接回报 |
|---|---|---|---|---|
| 长期开启 | 文件、版本、session 或设备状态变化 | daemon 后台监听；只在重要变化时提醒 | 静默漂移、context bloat、MCP/skill 消失、秘密暴露 | 事前发现；连续审计；减少突发故障 |
| 任务前 | 开始一个 coding task | 选 harness/cwd/task，生成 preflight Receipt | 不知道本轮会加载什么；选错 profile | 更少返工；更有把握；可保存基线 |
| 会话中 | context pressure、compaction、tool/skill 激活变化 | 查看 live timeline 或轻量状态 | 会话为何变慢/变贵；何时压缩；动态层变化 | 避免中途失控；及时调整 |
| 偶尔诊断 | 行为异常、更新工具、换仓库 | one-shot scan + observed import + diff | “昨天可以今天不行”；工具间差异 | 将数小时人工排查压缩为分钟级解释 |
| 定期保养 | 每周/月/季度 | scheduled scan、debt review、供应链审计 | 过时、重复、在已观察窗口长期未用、版本漂移 | 配置保持清洁；减少长期隐性成本 |
| 事件触发 | PR、版本升级、安装/删除资产、policy 变化 | CI/Hook 生成 diff 并按 deterministic rule 门禁 | 不安全或有损变更进入主分支 | 在合并前阻断回归；留下证据 |
| 迁移/入职 | 新设备、新成员、新 agent、remote env | 导入 desired state、预览、同步、验证 Receipt | A 有 B 无；手工配置遗漏 | 更快 bootstrap；复现而不复制秘密 |
| 评测活动 | 优化规则、比较工具/版本 | 锁定变量并运行 A/B | 不知道某条 context 是否有实际作用 | 用自己的任务数据做去留决策 |

### 6.1 长期开启模式的产品约束

- daemon 必须可选，产品不能要求后台常驻才能使用。
- 默认只监听 metadata、hash、mtime、版本和明确授权的 runtime event，不持续保存正文。
- 相同状态不重复通知；只在新增高风险项、无法解析新版本、配置漂移、预算异常或用户阈值触发时提醒。
- CPU、内存、文件句柄和电量占用可见且有上限；可按项目暂停。
- 用户能一键查看“为什么提醒”和“从哪个快照变化而来”。

### 6.2 偶尔使用模式的产品约束

- 无需账号、无需 daemon、无需初始化数据库即可运行一次扫描。
- `ctxpect inspect` 在当前目录自动探测工具和配置，终端内给出紧凑结果并可打开本地 UI。
- 首次使用不要求用户理解所有术语；优先回答三个问题：工具差异、设备依赖、当前风险。
- 一次性用户也能导出自包含、默认脱敏的 Receipt 交给同事或 issue。

### 6.3 定期使用模式的产品约束

- 支持系统 scheduler、CI 和用户自己的 cron/launchd/Task Scheduler，但不自造任务调度云。
- 每次扫描只保存差量和 digest；保留策略可配置。
- 报告聚焦新增/恶化/已解决，不把相同问题每周重复当新发现。
- 支持月度 context debt review：过时、重复、在已导入且可观察窗口中未激活、过大、无来源、版本不兼容；同时展示未导入历史和不可观察 surface。

### 6.4 回报指标

产品应让用户能量化而不是只感到“更整洁”：

- `Time to Explain`：从发现行为差异到指出根因和证据所需时间。
- `Reproducibility Rate`：同一 desired state 在目标设备/surface 上生成等价 Receipt 的比例。
- `Unexpected Context Incidents`：用户未预期的新增、缺失、覆盖或外发事件数。
- `Context Debt`：过时、重复、冲突、无来源，以及在明确观察窗口内未激活项的数量和负担。
- `Verified Budget`：exact/estimated/unknown 的占比，而不是伪造统一 token 总数。
- `Effect Evidence Coverage`：有受控对照证据的优化结论比例。

## 7. 核心用户工作流

### 7.1 第一次打开

1. 产品解释 local-first 与数据边界。
2. 用户选择本机项目目录；可选授权扫描个人配置目录。
3. 产品探测已安装 harness 和版本，不要求登录。
4. 为每个 harness/surface 生成 Expected Receipt；不可见层显示为 Unknown。
5. 主界面立即展示工具差异、设备依赖、context budget 和高置信问题。
6. 用户可选择导入原生 runtime snapshot 做 Observed reconciliation。

### 7.2 任务前核对

1. 选择项目、cwd、harness、surface 和任务描述。
2. resolver 计算 discoverable/eligible items。
3. UI 展示本轮预计常驻、条件、渐进、运行时和不透明层。
4. 用户确认 profile、MCP 和敏感数据边界。
5. 生成 preflight Receipt；可从 Contexpect 启动 harness 或只复制启动参数。

### 7.3 行为异常诊断

1. 导入本次或相邻会话的原生诊断/日志。
2. 对比 Expected 与 Observed，按 evidence quality 排序。
3. 点击异常项查看 resolution path、override、cap、ignore、surface、版本变化。
4. Doctor 给出 deterministic repair options；LLM Advisor 可选地解释语义冲突。
5. 用户预览修改，选择应用并获得 rollback point。
6. 重新扫描并生成 before/after Receipt。

### 7.4 新设备同步

1. 设备 A 选择要纳入 desired state 的非秘密资产。
2. 产品生成加密 bundle 或 Git-backed manifest；秘密只保存 reference。
3. 设备 B 先 preview：新增、冲突、有损和无法支持项。
4. 用户选择 resolution；原子应用并保留备份。
5. 在设备 B 用目标 harness 生成 Receipt，与设备 A 进行语义对齐核对。

### 7.5 Context 效果评估

1. 选择一个或多个 context items 和可重复任务集。
2. 固定 harness/model/version/cwd/代码基线/门禁；记录不可固定项。
3. 运行 control 与 treatment，多次重复。
4. 比较任务通过率、时间、token/cost、tool usage 和失败类型。
5. 只在证据满足阈值时给出 outcome-affecting 结论；否则显示 inconclusive。

## 8. 完整功能需求

以下需求均属于完整交付合同。`可选` 表示用户可以关闭或不授权，不表示产品可以不实现。

### F-01 环境与 Harness 探测

- 必须探测 §11 声明的 18 个 adapter family 的 executable、App bundle、版本、可用 surface 和相关 home/config roots；探测不到的 family 显式为 `not-installed`，不能从配置目录或 recent-item 痕迹推断为可运行。
- 安装状态必须拆成 `executable-present / app-present / config-residue-present / authenticated / connector-ready`，避免把“曾安装”“有配置”“当前可运行”合成一个 Active。
- 必须记录 OS、架构、设备 profile、cwd、project root、Git root/worktree，以及环境变量的 `observed injection channel / user-attested / unknown`；进程只有最终值时不得反推 shell rc、IDE 或父进程来源。
- 必须建立 `AccountProfile`、`OrganizationContext`、`PolicySnapshot` 并纳入 ResolutionCoordinate。标识使用用户本地 alias 与域隔离 keyed opaque id，不持久化原始邮箱、账号名或 tenant id；官方 surface 不暴露时为 user-attested/unknown。
- 必须支持显式覆盖路径，且在 receipt 中显示覆盖来源。
- 必须把本机、SSH、Dev Container/Docker、CI、云端执行环境作为不同 environment profile。
- remote/container/CI 的默认采集方式是在目标环境运行版本匹配的 `ctxpect collect`，生成签名/脱敏 Receipt 后离线导入；host UI 不通过共享文件系统猜测远端 home。
- 可选 SSH collector 必须调用系统 SSH、严格验证 `known_hosts`/host key，不保存密码，显式授权远端 project/home roots，进行 collector schema/version negotiation；认证、路径、升级或网络失败时受影响 claim 为 indeterminate/unknown。
- 未知版本所有依赖 harness 行为的 claim 一律为 `indeterminate + unknown`，不能用于通过 CI 或建立新 baseline。最近兼容 adapter 只可在用户显式开启的“探索预览”中运行，其输出全部标 `unverified preview`，不得改变权威 Receipt；任何非阻断例外必须由显式 policy 授权并写入 Receipt。

### F-02 资产发现与 Inventory

- 必须发现 taxonomy 中可静态存在的 instructions、rules、memory、skills、plugins、custom commands/prompts、agent/subagent definitions、MCP server/tool/prompt/resource 声明、hooks、profiles、managed/user/team 声明和遗留格式；动态类别必须建立 capability placeholder。
- 必须覆盖 global、managed、team、user、project、nested、local、runtime scope。
- 必须识别 symlink、hard link、重复内容、来源、内容 hash、Git tracked/ignored 状态。
- 默认不得扫描 `.git`、依赖目录、构建产物、`.env`、私钥和用户排除路径的正文。
- 被动扫描绝不启动 MCP server、执行 hook/script/custom command、加载 plugin 代码或触发动态 retrieval；只解析允许的声明和 inert content。需要执行的验证进入显式 sandbox/executor 流程。
- 对云端或 UI-only 设置必须保留占位节点，并允许 API 导入、导出文件导入或用户声明。
- 必须给每个 item 分配稳定 identity，规则见 §10.4；未经 persistent asset id 或用户确认，内容相同不能自动证明跨设备同一资产。

### F-03 版本化 Resolver

- 每个 resolver 以 `harness + version range + surface + OS` 为适用坐标。
- 必须实现 §11 中 18 个 adapter family 的声明 surface：四个 anchor 覆盖主要本地 coding surface；扩展 cohort 覆盖已声明的静态 resolver、可用原生 importer 和 Unknown honesty；Coze 按 connector/executor contract 实现，不伪装为同构 prompt harness。
- 必须模拟目录链、优先级、加法/覆盖、fallback、ignore、cap、glob/path、manual、agent-selected、progressive activation。
- 必须输出 resolution edges：included-by、excluded-by、shadowed-by、overridden-by、truncated-after、ignored-by、activated-by、unknown-because。
- 必须区分内容常驻成本、skill catalog/metadata 成本、激活正文成本、资源按需成本、tool schema 和 tool result。
- resolver 规则必须来自可链接的官方文档、匹配版本源码或原生观察；来源和更新时间进入 adapter manifest。
- 每个 adapter 必须有 fixtures、golden receipts、负例、未知版本测试和真实原生诊断对账样本。

### F-04 Task Probe 与 Preflight

- 用户必须能输入任务描述、目标文件或 glob、cwd、harness/surface/profile。
- 产品必须确定性计算已知的 eligible 条件；无法知道模型是否会自主选择时标 `agent-selected/unknown`。
- 不得输出未校准的加载概率。
- 必须允许保存常用 task profiles，例如 code review、frontend、release、docs、incident。
- 必须能从 preflight 启动 harness，或生成可复制、可审计的启动命令/环境说明。
- 启动后 Receipt 必须引用对应 preflight id，便于 expected/observed 对账。
- 启动命令必须以参数数组/结构化 process API 生成，禁止字符串拼接后交给 shell；复制为 shell 文本时按目标 shell 严格 quoting，并对恶意路径、换行、环境变量注入做 fixture 测试。
- secret 通过 secret manager/环境引用在执行边界注入，不进入命令预览、进程参数、日志或 Receipt。

### F-05 Context Receipt 与 Evidence Ledger

- 必须支持 one-shot、preflight、runtime、post-session、device baseline、CI 六类 Receipt。
- Receipt schema 必须版本化、可 JSON 导出、可签名、可脱敏、可比较。
- 每个 claim 必须引用 evidence id；evidence 包含来源、采集方式、时间、collector/resolver version、hash、质量和 reason code。
- 必须提供“Why is this here?”和“How do you know?”两个固定解释入口。
- Receipt 必须显式列出未覆盖 surface 和 unknown；不得只展示成功发现的部分。
- 默认分享包只含 metadata/hash/规则解释；用户可逐项授权附带片段或正文。
- 必须支持 receipt verification，检测 schema、signature、digest 和附件完整性。
- Receipt header/manifest 在生成后不可变；用户删除 evidence 或附件时保留不含正文的 tombstone 和原 digest，使历史签名仍可判定为“签名有效但附件已按策略删除”。派生分析必须删除或失效；已经导出的外部副本无法召回，UI 和删除确认必须明确说明。

### F-06 可视化 Context Inspector

- 主视图必须以 context lifecycle 和 scope 为主，而不是简单文件浏览器。
- 必须提供：上下文带状/堆叠视图、加载状态、scope、来源、budget、置信度和未知区。
- 必须能从总量下钻到 item、resolution path、原文片段和证据。
- 必须支持按 harness、surface、device、environment、cwd、task、snapshot 切换。
- 必须明确区分 expected、observed 和 effect 三层视觉编码，不能都用同一种绿色。
- 必须展示 context cap、截断位置、override/shadow、MCP schema、skill catalog/body、compaction event。
- 必须支持敏感正文遮罩、按住显示、复制前确认和截图隐私模式。

### F-07 跨工具、设备、版本与快照 Diff

- 必须提供两两 diff 和多列矩阵：harness、device/environment、account/organization/policy snapshot、snapshot、version、task。
- diff 维度至少包括：presence、scope、activation、content hash、semantic intent、permission、secret reference、loss、budget、evidence quality。
- 必须区分 `declared-same-intent`、§4.7 的 reconciliation status、独立的 behavioral evidence、`user-confirmed` relation、有损映射、仅一端支持和未知。相同 Intent ID 只代表人为归组，不自动证明行为等价。
- 语义对齐默认由显式 intent id、来源关系和可解释规则判定；LLM 仅提供候选匹配供用户确认。禁止输出没有标注集和校准合同的裸数值 confidence。
- 每个 drift 必须能追溯到首次出现的 snapshot/event。
- 必须支持 baseline lock 与 CI exit code；Unknown 不得自动等价于 pass。

### F-08 Context Doctor

- deterministic Doctor 必须检测：缺失、重复、冲突、过时线索、过大常驻项、cap/truncation、错误 frontmatter、不可发现路径、gitignore 差异、只在单设备、secret literal、隐藏 Unicode、symlink escape、来源不明、版本不兼容、有损 projection。
- 每条 finding 必须包含 severity、受影响坐标、证据、可复现步骤和修复候选。
- Doctor 必须输出 `PlacementRecommendation`：把项目事实放 source docs/生成事实源，把可确定执行的约束放 formatter/linter/type/test，把动作拦截放 hook/CI，把能力限制放 permission/sandbox，把长期行为偏好放 always/scoped instruction，把可按需流程放 skill，把外部能力放 MCP，把可更新经验放 memory。输出当前载体、推荐载体、理由、可自动判定程度和迁移/验证步骤；LLM 只能补充候选，不自动搬移。
- linter 是 defense-in-depth，不是 security boundary；无法从声明支持的输入到达的问题不得夸大为阻断漏洞。
- 支持 suppress/exception，但必须记录 owner、理由、scope、期限和 evidence。
- deterministic rules 可进入 CI；LLM 建议不能独立使 CI 失败。

### F-09 安全编辑、意图与 Native Projection

- 用户必须能把多个 native items 归到一个 `Intent`，例如“测试必须使用 Vitest”。
- Intent 由 stable id、目标、scope、正文/结构化约束、目标 harness 和 native overlays 构成。
- 每个 target profile/transaction 必须绑定唯一 `ProjectionAuthority`：`contexpect-native`、冻结版本的 APM、Agentpack、agentsync 或 `export-only`。同一目标和资产不能组合两个 projector。
- `contexpect-native` 只覆盖下表的核心 required-write cell：由版本化 adapter 生成 machine-readable native plan/loss，再交给受限 File/Git transaction executor 原子写入。外部 authority 负责其被分配的 cell；Contexpect 不为同一 cell 再生成第二份 plan。
- Contexpect 拥有 preview UI、执行授权、调用审计和 apply 后 Receipt，并用自己的 resolver 对投影结果产生独立 `post-projection finding`。它不能改写 authority 的 loss report；两者矛盾时显示 conflicted 并阻止 apply，直到 integration contract 或用户选择被记录。
- 任何应用动作必须显式选择目标；被委托 executor 必须提供原子/可恢复语义、保留原格式/注释（能力允许时）和 rollback point。若没有满足合同的 executor，该 projection 只能导出 plan，不能由 Contexpect 直接写入。
- 不支持的映射必须 skip 或要求 native overlay，禁止静默删除。
- 必须处理并发修改：apply 前重新 hash，检测变化后停止并要求 merge。
- 必须通过 executor integration 支持 undo/rollback 和 dry-run；不能把 executor 成功等同于目标 harness 已生效，应用后还要重新生成 Receipt。
- executor 必须在最小化环境和任务级受限临时目录运行；其 argv、env、stdin、stdout/stderr、debug log、临时文件、backup、snapshot、local Git history 与 crash artifact 全部进入 secret/redaction gate。不能证明不落明文 secret 的 executor 不得处理 secret-bearing transaction，只能 read-only/import/export plan。

完整产品的 required projection 下限如下；`acceptance/projection-matrix.yaml` 可以增加 cell，不能把这些 required-write 降为 export-only：

| Asset / scope | Codex | Claude Code | Cursor | Grok Build | 默认 authority |
|---|---|---|---|---|---|
| project instructions | required-write | required-write | required-write | required-write | `contexpect-native` |
| user instructions | required-write | required-write | observe/import/export（anchor 版本无稳定外部写入 User Rules surface） | required-write | `contexpect-native`；Cursor 为 `export-only` + UI handoff |
| packaged project/user skills | required-write | required-write | required-write | required-write | APM |
| unmanaged local project/user skills | required-write | required-write | required-write | required-write | `contexpect-native` |
| project scoped/conditional rules 这一独立 primitive | N/A；用 project/nested instruction 表达且必须显示 loss | required-write | required-write | N/A；用 project/nested instruction 表达且必须显示 loss | `contexpect-native` |
| native MCP config：厂商在截止版本公开支持的 user/project scope | 每个公开可写 scope required-write | 每个公开可写 scope required-write | 每个公开可写 scope required-write | 每个公开可写 scope required-write | `contexpect-native`；package 安装仍归 APM |
| custom commands/prompts、plugins、hooks、managed/team UI-only rules | observe/import/export；只有 §17.0 固定了稳定 native executor 的 cell 才可 required-write | 同左 | 同左 | 同左 | 冻结 external authority 或 `export-only` |

`N/A` 只表示目标 harness 没有该独立 native primitive，不得把同义 intent 删除；必须给出 loss/native overlay 或投影到表中另一个 required primitive。每个 required-write cell 必须固定一个 authority/version，具备 preview、apply、rollback、并发 guard、loss report 和 post-apply Receipt；缺任一能力即产品验收失败。`export-only` 只能用于 observe/import/export cell，不能满足 required-write。

### F-10 多设备 Desired State 与加密同步

- 必须定义 transport-neutral encrypted bundle 和 provider contract；完整交付内置本地文件夹与 Git-backed provider，可放入用户已有的 iCloud Drive、Dropbox、Syncthing 等文件同步目录。其他云/WebDAV/S3 通过 provider adapter，不在 core 中自建通用传输客户端。
- 产品不要求自建 Contexpect 云账号或多租户服务。
- 同步内容按 asset allowlist；secret 只保存引用，不保存明文。
- 加密 bundle 必须端到端加密且具备 authenticity、版本链和 replay/rollback detection；远端只能看到 opaque blob、最少 metadata 和版本。
- 必须支持标准 recipient 的加入/移除、内容密钥轮换、用户自主管理的密钥备份/恢复和冲突检测；不自建设备 PKI、恢复码、通用版本图或通用三方 merge 协议。
- Git-backed provider 直接使用 Git 的 commit/merge/signing 语义。文件夹 provider 只发布不可变 generation bundle 与 latest pointer；观察到同父分叉时进入 conflict，允许用户选择 head、导出双方或转交 Git/external authority 处理，禁止静默自动合并。
- 同步前后均生成 Receipt；设备“同步成功”必须同时区分 transport success 和 semantic reconciliation success。
- 项目资产优先引用 Git source；不得把 Git 已管理的正文无理由复制进第二套云数据库。
- 支持 `local-only`、`project-shared`、`device-group`、`never-sync` 分类。

### F-11 Skills / Plugins / MCP 管理与外部生态集成

- 必须提供本地 catalog：来源、版本、license、scope、兼容 harness、权限、scripts/hooks、MCP transport、secret refs、安装状态、激活/使用证据。
- 必须支持 AGENTS.md、Agent Skills、Agent Plugins、MCP 等公开标准的 import/export/validate。
- 必须读取并展示 APM manifest/lock；package resolution、安装、更新、disable、lock 与 SBOM 生命周期由受控 APM/Git executor 完成，Contexpect 不复制其依赖解析器和 package manager。
- 可支持 Agentpack/agentsync 的 desired-state import/export 或外部执行，但必须清楚显示由谁负责 mutation。
- 社区内容通过 Git URL、APM registry/marketplace 或用户配置的 registry 浏览，不自建通用 marketplace 后端。
- 安装前必须展示来源、commit/tag、license、hash/signature、scripts/hooks、MCP commands、权限、网络需求和安全扫描结果。
- 安装必须由 executor pin 版本并返回 SBOM/lock reference；Contexpect 展示 update preview、风险、调用审计、rollback/disable 能力和应用后 Receipt。
- 对没有许可证、来源不明或越界路径的包默认阻止复制/执行。

### F-12 Runtime Importers 与 Session Timeline

- 必须实现可用原生 surface 的 importer：Claude `/context`、Grok `inspect --json`、Codex `debug prompt-input`/app-server/JSON events，以及 Cursor 可获得的官方导出或活动规则信息；扩展 cohort 优先接入 DeepSeek Harness append-only events、OpenHands Agent Server/SDK events、Qwen headless/daemon events、Copilot inventory/events、Cline JSON/SDK、Kimi/ACP 和其他 adapter manifest 明确声明的原生字段。
- 每个 importer 必须声明覆盖范围和缺口。例如 Codex `prompt-input` 不能被标成完整 wire payload。
- 必须支持手动文件导入、stdin、CLI wrapper 和受控 API connector；默认不要求代理流量。
- Dev Container/Docker/CI/cloud agent 通过目标环境内 collector 或官方 export 导入；Receipt 必须区分“远端实测”和“本机对远端配置的推断”。
- 高风险 proxy/CA 捕获只作为高级 opt-in integration，单独说明风险，不作为完整功能的必经路径。
- timeline 对 importer 明确暴露的 turn、compaction、memory change、skill activation、MCP/tool call、schema change、context budget 和 cost 记录真实事件；未暴露的事件类型只记录 capability `indeterminate/unknown`，不能补造 event。
- importer 必须覆盖或诚实标 Unknown：custom command/prompt、agent/subagent/child context、`@`/attachment/retrieval/file read、steering/handoff、tool search/catalog、MCP prompt/resource/server instruction 与 tool result。
- 每个 importer 发布机器可读 `field-to-claim` mapping：原生字段的语义、lifecycle stage、coverage 和最低 evidence。来自原生命令不自动等于 ModelVisible；例如“discovered configuration”默认只证明 discoverable，除非官方明确其已注入模型。
- 必须区分 session 累计 input、当前 context occupancy、cache read/write、output、tool fee；无法获得时为 Unknown。
- 用户必须能删除单个 session、项目历史或全部历史，并确认相关派生分析被删除或重新计算。

### F-13 LLM Advisor

- LLM Advisor 必须是用户主动触发、可预览发送内容，并通过 `AnalysisAdapter` 调用用户已有的本地 command、标准 endpoint 或外部 AI 工具；也可只导出脱敏 analysis bundle。Contexpect 不维护完整 provider catalog、模型路由或独立 API-key 管理面。
- 默认只发送用户选中的脱敏片段、finding 和结构 metadata，不发送整个仓库或会话。
- 可做：语义冲突解释、过时线索核对、拆分建议、intent 候选、skill 候选、修复草案、历史模式摘要。
- 不可做：把推断写成 observed、无对照地判定“有用/无用”、直接应用修改、绕过 policy、生成不解释来源的精确评分。
- 每条建议必须引用输入 evidence；用户接受后仍走 preview/apply/receipt。
- LLM 输出只保存为独立 `AdvisorSuggestion` 或 `CandidateRelation`，状态固定为 non-authoritative candidate。它不得创建/修改 ContextStateClaim，不参与 precedence、签名真值、policy、CI、baseline 或 semantic reconciliation；用户确认后也只生成 user-authored Intent/decision，随后由独立 resolver/evidence 重新核对。
- adapter 必须返回其声明的 provider/model/版本、时间、发送字段类别和 retention 元数据；secret 由 OS keystore/现有 secret manager 或外部工具持有，不进入 Contexpect 配置和日志。不默认保存原始响应。
- LLM/provider、registry、update check、webhook 和外部 adapter 共用 egress policy：域名/IP allowlist、DNS rebinding/SSRF 防护、proxy/TLS 设置、目标认证和每次发送审计。URL 参数、redirect 和本机/云 metadata endpoint 必须进入负例测试。

### F-14 历史会话分析

- 必须支持用户授权的本地历史 importer，并按 harness/version/surface 标注来源。
- 分析至少包括：重复纠正、反复补充的提示、skill/tool 激活、从未观察到的资产、context growth、compaction、失败模式、隐私暴露和成本趋势。
- “从未使用”必须表述为“在已导入、可观察的 N 个 session 中未观察到”，不能推广到全部使用历史。
- 可以提出 rule/skill/memory 候选，但必须展示样本会话和反例。
- 历史分析默认本地；任何云端或 LLM 分析逐次授权。
- 支持时间范围、项目、任务类型、harness 和 device 过滤。

### F-15 Context Effect Lab

- 必须能定义 task suite、代码基线、control/treatment context、harness/model/version、重复次数、随机性设置和质量门禁。
- 必须保存每次运行的 Receipt、输出摘要、门禁结果、时间、token/cost 和异常。
- 必须检测关键混杂：代码漂移、模型变化、harness 更新、不同 tool availability、缓存、网络失败、门禁变化。
- 必须提供 pass rate、分布、置信区间/不确定性和 failure taxonomy；样本不足时显示 inconclusive。
- 不把单次前后对比宣传为因果证据。
- Contexpect 不自建通用 agent runner、sandbox、统计估计器或门禁执行平台；通过 versioned runner adapter 接入已有 coding harness、CI/eval runner，统计检验与区间使用冻结版本的成熟库。运行涉及成本和写操作时，experiment contract 必须声明预算、sandbox、approval 和工作目录隔离，并保存 executor receipt。

每个 `ExperimentContract` 在第一轮运行前冻结，至少包含：

- 唯一 primary outcome、计算方式和 blind/automatic evaluator；
- 预先声明的最小实际效应或 equivalence margin；
- paired/randomized order、task sampling、control/treatment 唯一差异；
- 固定样本量或基于 `alpha ≤ 0.05`、`power ≥ 0.80` 的样本量计算；不得在看到结果后随意停止；
- 多个 secondary outcome/多 treatment 的 multiplicity correction；
- timeout、crash、refusal、infra failure 和缺失数据的 intention-to-treat 处理；
- 独立重复、模型/harness/cache/tool availability 和代码基线锁定方式；
- 排除项、协议偏离和使实验失效的条件。

OutcomeAffecting facet 的 decision 只允许：`supported-beneficial`、`supported-harmful`、`supported-equivalent-within-margin`、`inconclusive`。默认是 `inconclusive`。前三者必须同时满足预注册判据、方向与最小实际效应，并且没有使实验失效的协议偏离；“未显著”不等于“等价”。结论只适用于该 experiment coordinate，不推广为普遍因果规律。

### F-16 Daemon、Scheduler、CI 与通知

- CLI one-shot 是一等公民；daemon 为可选增强。
- daemon 监听配置和版本变化，去重事件，生成增量 snapshot，并提供本地 API 给 UI/IDE。
- scheduler 支持 daily/weekly/monthly policy 和系统原生调度集成。
- CI 必须支持 JSON、SARIF、Markdown summary、稳定 exit codes 和 baseline diff。
- CI 默认只使用 deterministic rules 和 metadata；需要 secret/runtime/account 的检查应明确 skip/unknown。
- 未知 harness version 或 required claim 为 indeterminate 时，CI 默认返回独立非通过状态和稳定 exit code `3`；`0` 只代表 policy pass，`2` 代表确定性 drift/finding failure。组织可显式把 indeterminate 降为非阻断，但例外必须写入 Receipt，不能改变 claim 本身。
- 通知支持本地桌面、终端和 webhook adapter；默认只通知 actionable change，不发送正文。
- 离线时核心扫描、diff、Doctor、Receipt 和历史查询必须可用。

### F-17 导入导出、API 与扩展

- 必须提供稳定的 CLI JSON envelope、Receipt JSON Schema、adapter manifest schema 和 version migration。
- 支持 JSON、Markdown、SARIF、CycloneDX/SPDX reference、OTLP-compatible telemetry 导出。
- 提供只绑定 localhost 的本地 API，默认随机 token/Unix socket，支持 UI 和 IDE integration。
- 第三方 adapter 使用进程隔离或 WASI/明确权限模型，不默认获得整个 home 和网络；网络访问遵循同一 egress allowlist、SSRF、TLS 和目标认证策略。
- adapter SDK 必须包含 fixture runner、golden test、capability declaration、evidence quality 和 compatibility range。
- import 必须验证 schema、大小、路径、签名和压缩包穿越；失败不得部分应用。

### F-18 Policy、Approval 与 Audit

- 每个 policy domain 必须有唯一 `PolicyAuthority`，同一 check 不能由两个 evaluator 竞争判定。APM 是 package/source/install/update/license/digest/SBOM 域的唯一 authority；Contexpect 只导入并展示 APM evaluation，不复制或覆盖它。
- Contexpect 自身只评估 context-specific 域：Required/Unknown 处理、Receipt/Observed 证据下限、敏感级别与 retention、sync/LLM egress、capability/surface 要求、CI severity。MCP command/network 与 script/hook 若属于 package 安装时由 APM 判定；运行时是否符合 context egress/secret 约束由 Contexpect 判定，二者是不同 check id。
- 必须支持项目/用户本地 context policy、签名 policy bundle，以及通过冻结 integration 读取 APM `apm-policy.yml` evaluation；Contexpect 不自建组织控制面。
- `PolicyRevision` 不可变并带来源、schema、digest、签名/信任状态、生效期；`PolicyBinding` 绑定 device/environment/account/organization/policy snapshot/project/harness/surface/asset scope。
- precedence 采用 tighten-only：更低 scope 可以增加限制，不能放宽更高 authority 的限制。冲突或无法验证的高层 policy 进入 indeterminate，不能当 pass。
- context policy 至少可约束：asset kind、数据敏感级别/retention、sync/LLM egress、Unknown 处理、Receipt/signature/evidence 要求、capability/surface 和 CI severity；package/source/install/license/digest/SBOM 的结果必须引用 APM check id 与 evidence。
- `Approval` 必须由 policy 指定的签名身份作出，记录 action、scope、reason、created/expiry、single-use/persistent 和关联 evidence；过期或越界不能复用。
- 离线时使用最后一个已验证且仍有效的 revision；需要在线刷新但无法完成、policy 过期或 revocation 状态未知时，结果为 indeterminate/exit `3`。
- `AuditEvent` 以本地 append-only hash chain 保存 policy load/bind/evaluate/exception/approval/apply；可签名导出。管理员默认只看到 metadata，不得到正文。
- `policy pass` 的唯一含义是：当前 coordinate 下所有 required policy checks 确定通过，且没有未被有效 Approval 覆盖的 deny 或 indeterminate。

## 9. 信息架构与视觉需求

### 9.1 主导航

1. `Overview`：当前项目/设备/account/organization/policy snapshot 的风险、drift、unknown 和最近变化。
2. `Inspector`：指定 harness/cwd/task 的上下文分层与下钻。
3. `Compare`：跨工具、设备、版本、snapshot 的矩阵与 diff。
4. `Receipts`：所有核对凭据、签名、分享和验证。
5. `Assets`：instructions、rules、skills、plugins、MCP、hooks、memory catalog。
6. `Sessions`：runtime timeline、compaction、tool/skill activity、history insights。
7. `Effect Lab`：task suite、A/B runs 和结果。
8. `Sync`：desired state、设备、remote、conflict、secret refs。
9. `Doctor`：deterministic findings、exceptions、LLM Advisor。
10. `Policy`：当前绑定、来源、评估、Approval、例外和 audit events。
11. `Standards`：adapter 规则、来源、版本、conformance 状态。
12. `Settings`：privacy、retention、analysis adapters、notification、resource limits。

### 9.2 Inspector 视觉层级

首屏同时回答“多少、来自哪、为何进来、证据质量”：

- 顶部坐标条：device / environment / account / organization / policy snapshot / harness / version / surface / project / cwd / task / snapshot。
- 中部 Context Facets：并列显示 installed、discoverable、eligible、visible、use evidence、effect 六条独立证据泳道；连线只表示有证据的 resolution relation，不暗示必然单调推进。
- 预算区域：constant、conditional、skill catalog/body、tool schema/result、history、memory、current user、unknown。
- Scope rail：managed、team、user、project、nested、local、runtime。
- Evidence overlay：分别展示 claim kind、truth、provenance、coverage、precision、knowledge status，不把 Expected、Partial、Exact、Unknown 压成一个互斥颜色。
- 右侧 detail drawer：内容预览、resolution path、来源文档、hash、版本、相关 finding、before/after。

### 9.3 Compare 视觉层级

- 行是 Intent 或资产，列是 harness/device/version。
- 单元格不只显示“有/无”，还显示 discoverable、eligible、observed、loss 和 unknown。
- 点击单元格展示 native representation；点击行展示 semantic difference。
- 支持 `only differences`、`only current verified evidence`、`include unknown`、`security-impacting`。

### 9.4 可访问性与隐私视觉

- 状态不能只靠颜色；需要文字/图标和屏幕阅读器标签。
- 敏感字段默认遮罩；录屏/截图模式隐藏路径、用户名、repo、prompt 片段和 server URL。
- 大图必须提供列表/表格替代；所有关键动作可键盘完成。
- 每个数字旁可查看计算口径，Unknown 不以 0 长条展示。

## 10. 逻辑数据模型

### 10.1 核心实体

```text
Device
  └─ EnvironmentProfile
       ├─ AccountProfile ── OrganizationContext ── PolicySnapshot
       └─ HarnessInstallation
            └─ Surface

Project ── Worktree ── ResolutionCoordinate
                         ├─ Snapshot
                         ├─ Receipt
                         └─ Session

ContextItem ── ContextRevision
     ├─ NativeRepresentation
     ├─ IntentMembership
     ├─ Evidence
     ├─ Finding
     └─ EffectObservation

Receipt / Snapshot / ResolutionCoordinate
     └─ ContextStateClaim
          ├─ ResolutionEdge
          ├─ Evidence
          └─ ContextItem / ContextRevision

DesiredState ── ProjectionPlan ── ApplyTransaction ── RollbackPoint
SyncBundle ── DeviceEnvelope ── Conflict
Policy ── PolicyRevision ── PolicyBinding ── PolicyEvaluation
                              ├─ Approval
                              └─ AuditEvent
```

### 10.2 建议 SQLite 表

- `devices`, `environment_profiles`, `account_profiles`, `organization_contexts`, `policy_snapshots`, `harness_installations`, `surfaces`
- `projects`, `worktrees`, `resolution_coordinates`
- `context_items`, `context_revisions`, `native_representations`, `context_state_claims`, `item_aliases`, `identity_events`
- `intents`, `intent_memberships`, `projection_capabilities`
- `snapshots`, `receipts`, `receipt_items`, `receipt_digests`
- `resolution_edges`, `evidence`, `unknown_surfaces`
- `findings`, `exceptions`, `repair_options`
- `advisor_suggestions`, `candidate_relations`
- `sessions`, `turns`, `runtime_events`, `compactions`, `tool_calls`
- `task_suites`, `experiments`, `runs`, `gate_results`, `effect_observations`
- `desired_states`, `projection_plans`, `apply_transactions`, `rollback_points`
- `sync_bundles`, `device_envelopes`, `sync_conflicts`
- `policies`, `policy_revisions`, `policy_bindings`, `policy_evaluations`, `approvals`, `audit_events`
- `adapters`, `adapter_sources`, `conformance_runs`

不要求图数据库。图和路径查询由 `resolution_edges`、索引和递归 CTE 支撑；只有真实规模证明 SQLite 无法满足时才重新评估。

### 10.3 ContextItem 关键字段

```text
id, stable_key, kind, scope, owner
source_uri, native_path, logical_path
local_keyed_digest, share_digest_policy, size_bytes, line_count
activation_mode, sensitivity, sync_policy
license, provenance, signature_state
created_at, observed_at, retired_at
```

ContextItem 只保存资产身份和相对稳定的修订信息，不保存 discoverable/visible/used 等坐标相关状态。

### 10.4 Stable identity 合同

- 由 desired-state/sync/external manifest 管理的资产使用 manifest 中不可变 `asset_uuid`；跨设备等价优先依赖该 ID 和签名 provenance。
- 未受管本地资产的初始 key 为 `source-realm-id（含适用的 opaque account/organization realm）+ normalized logical path + kind + native target` 的域隔离 keyed digest；inode/file-id 只可在同一文件系统和 snapshot window 内作为辅助证据。
- 如果旧路径消失、新路径出现且本地 keyed content digest 相同，只生成 `move-candidate`；有 manifest rename event 或用户确认后才沿用 identity。
- 原路径与新路径同时存在视为 copy，分配新 identity。共同祖先后独立演化视为 fork，以 `identity_event` 记录 lineage，不合并 revision history。
- key rotation 重新计算 keyed digest，但不改变已有 `asset_uuid`；alias 表记录旧 key 的受限迁移映射。
- 跨设备比较优先使用 shared asset UUID、Intent membership 和签名来源；低熵/敏感内容没有对外 digest 时只能标 `declared/user-confirmed/unknown`，不能靠正文猜测等价。

### 10.5 ContextStateClaim 关键字段

```text
id, receipt_id, snapshot_id, resolution_coordinate_id
context_item_id, context_revision_id
claim_kind, lifecycle_stage, truth_state
provenance, coverage, precision, knowledge_status
value_json, unknown_reason_code
evidence_id, resolver_version, observed_at
```

同一 ContextItem 在不同 coordinate 或 lifecycle stage 下有多条 claim。`absent` 必须有足够覆盖证据；无法证明不存在时使用 `indeterminate`。

### 10.6 Evidence 关键字段

```text
id, claim_type, subject_id
source_type, source_uri, source_version
collector_name, collector_version
harness, harness_version, surface
captured_at, content_digest
source_snapshot_digest, license/provenance_policy
redaction_policy
```

Evidence 保存来源事实；coverage、precision 和 knowledge status 属于 claim。不得保存无校准的裸数值 `confidence`。

### 10.7 Unknown reason codes

至少包含：

- `surface_not_exposed`
- `unsupported_harness_version`
- `permission_not_granted`
- `runtime_snapshot_missing`
- `cloud_setting_unavailable`
- `dynamic_agent_selection`
- `tool_schema_not_exported`
- `current_occupancy_not_reported`
- `content_redacted_by_policy`
- `import_parse_failed`
- `evidence_stale`

### 10.8 Policy 关键字段

```text
Policy: id, domain, authority, source_uri
PolicyRevision: policy_id, schema, digest, signer, trust_state, valid_from, expires_at
PolicyBinding: revision_id, device/environment/account/organization/policy-snapshot/project/harness/surface/asset selectors
PolicyEvaluation: binding_id, coordinate_id, authority, check_id, result(pass|deny|indeterminate), evidence
Approval: evaluation/action, signer, scope, reason, created_at, expires_at, use_mode
AuditEvent: sequence, previous_digest, event_type, actor, subject, evidence_digest
```

Policy 内容与 evaluation 分开；修改 policy 不重写历史 evaluation。离线/时钟/签名状态是 evaluation evidence，不隐藏在 UI 设置中。`domain + check_id` 只能绑定一个 authority；APM evaluation 作为外部只读结果导入，不由 Contexpect 重算。

## 11. Adapter 支持矩阵

### 11.1 完整交付的支持口径

| Family | 静态 Expected / Inventory | 优先原生 Observed | 支持口径与主要限制 |
|---|---|---|---|
| Codex | AGENTS chain、fallback/cap、config、skills/plugins/MCP、profiles | `debug prompt-input`、app-server/JSON events 中明确可得部分 | anchor；prompt-input 不含完整 core prompt 字段；Desktop/CLI/cloud 分开 |
| Claude Code | managed/user/project/local CLAUDE、rules/imports、skills/MCP/hooks、memory path | `/context`、配置诊断、允许导入的 session/runtime records | anchor；auto memory、subagent、cloud/Cowork 分开 |
| Cursor | project rules、AGENTS、skills/plugins/MCP、本地可见设置 | 官方可导出/活动规则/日志中确实暴露的部分 | anchor；User/Team Rules 与 IDE/cloud 可能不透明 |
| Grok Build | inspect-able rules、skills/plugins/hooks/MCP、兼容目录 | `grok inspect --json`、允许导入的 streaming/runtime events | anchor；兼容发现不等于语义等价；TUI/headless 分开 |
| OpenCode | AGENTS/rules、agents、skills、commands、MCP、config | JSON run/events、允许的 session export | build/plan、CLI/Desktop 和 provider config 分开 |
| DeepSeek Harness | plugins、AGENTS/CLAUDE、session/tool config | append-only session/event log、plugin graph | developer preview；breaking changes 触发 unsupported-version |
| Kimi Code | AGENTS、skills、MCP、hooks/subagents、flow | stream/session export、ACP | 新 Kimi Code 与 winding-down 的 Kimi CLI 分开迁移 |
| ZCode | global/workspace AGENTS、skills、commands、MCP、plugins/hooks | App task/export 中确实暴露部分 | App 与 remote workspace 分开；无 CLI 时不伪造 CLI surface |
| Qwen Code | QWEN/AGENTS、rules、skills、MCP、hooks、agents、memory/extensions | headless JSON、daemon/SDK event | user/project/system/extension precedence 分开 |
| Goose | hints、recipes、extensions/MCP、provider/ACP config | diagnostics、session export/API | coding 与 general-purpose workflow surface 分开 |
| Gemini CLI | GEMINI、settings、skills/extensions、MCP、memory | headless JSON/session/diagnostics | 账号/edition/client eligibility 纳入 coordinate |
| GitHub Copilot CLI | merged instructions、skills、agents、MCP、plugins/hooks | `plugins list`、MCP JSON、CLI result/event | CLI、IDE、cloud agent 和 org policy 分开 |
| Kiro | steering、AGENTS、skills、hooks、agents、MCP/powers/specs | CLI headless/ACP、hook/session evidence | local/global/cloud steering 与 custom-agent resources 分开 |
| Cline | `.clinerules`、skills、MCP/plugins、agents | CLI JSON、SDK/session surface | CLI、VS Code、JetBrains 分开 |
| Aider | `.aider.conf.yml`、`read` conventions、repo map、model config | chat history/analytics 中声明字段 | 不把它硬套成 SKILL.md 型 harness |
| OpenHands | AGENTS、skills、agent config、setup/hooks | Agent Server/SDK events、prompt injection record | CLI/SDK/GUI/cloud 分开 |
| Windsurf | global/workspace/system Rules、AGENTS、Workflows、Skills、Memories、MCP | 官方 export/log 中确实暴露部分 | 自动 Memories 具有本机/工作区边界；IDE/cloud 分开 |
| Coze | global/project config、org/space/project coordinate、bundled skill target | CLI schema/status/JSON、Coze API receipt | connector/executor；不参与隐藏 prompt parity |

“支持某 harness”意味着：

- 已覆盖的功能有版本和 surface 声明；
- 未覆盖处显示 Unknown；
- 有 conformance fixtures 与至少一个 native oracle 对账样本；
- 不意味着能提取隐藏 system prompt 或所有 provider-wire 字段。

首屏可把当前 coordinate 汇总成 `Native evidence / Static resolution only / Needs connector / Unsupported version`，但 canonical 状态仍落在 `family × version × surface × capability` 的 matrix。18 个 family 是完整交付的声明范围，不意味着每个 family 的每个 surface 都是 Native evidence。详细调研和本机依据见[扩展调研](../research/2026-09-04-agent-harness-expansion.md)。

### 11.2 Adapter 更新流程

1. 发现新 harness version。
2. 比较 capability manifest 和上次已验证版本。
3. 运行 static fixtures、negative fixtures、native oracle sample。
4. 若结果一致，扩大 verified version range；若变化，保留旧 adapter 并新增版本分支。
5. 未完成验证前，用户可运行探索预览，但受影响 claim 必须为 `indeterminate + unknown`；CI 默认 exit `3`，不能产生 pass 或新 baseline。只有显式 policy 例外可使流水线继续，例外仍不改变 claim。
6. 更新必须记录来源 URL、访问时间、允许保存的内容 digest、适用版本；使用开源实现时再记录源码 commit、license、行为变化和迁移说明。厂商文档或样本的归档/再分发必须服从其许可。
7. adapter/corpus/executor 更新包必须验证签名和 pinned digest；被撤销版本停止产生权威 claim，自动回到最近仍受信版本只用于其原 compatibility range，不能覆盖新 harness version。

## 12. 系统架构建议

这是技术方向，不替代实施前的详细 ADR。

### 12.1 组件

```text
Native files / Settings / Runtime exports / External tools
                         │
                Collectors & Importers
                         │
                Normalized Context IR
                         │
        Versioned Resolvers + Evidence Ledger
                         │
       Receipt / Diff / Doctor / Effect Engine
                         │
     SQLite + encrypted local content vault (optional)
            ┌────────────┼────────────┐
           CLI      Local API/Daemon   Sync Engine
                           │
                    Desktop/Web UI
```

### 12.2 推荐实现栈

- 核心：Rust workspace，负责扫描、路径安全、resolver、SQLite、加密、CLI、daemon 和 adapter runtime。
- UI：Tauri 2 + React + TypeScript，共享本地 API/commands；保留浏览器本地模式作为调试和无桌面壳入口。
- 数据：SQLite + FTS5；敏感正文如需保存，进入单独加密 vault，不与 metadata 表混存。
- Schema：JSON Schema + versioned migrations。
- 可视化：React 原生 SVG/Canvas；先用关系数据生成图，不引入图数据库。
- 加密：成熟 audited primitives；设备 envelope、外部恢复方式和密钥轮换按 §13.2.1 冻结 ADR，禁止自创密码算法或恢复码协议。
- 运行时扩展：外部 adapter 进程或 WASI，能力声明和最小权限。

选择 Rust/Tauri 的理由是本地文件系统、daemon、跨平台单包、安全边界和资源占用都属于核心要求。若团队对 Rust 没有维护能力，可改为 Electron/TypeScript 单栈，但不能牺牲路径安全、原子写入和稳定 CLI。

### 12.3 进程与信任边界

- UI renderer 不直接读整个文件系统，通过 core 的受限 command/API。
- adapter 只获得声明路径和 capability；第三方 adapter 默认无网络、无 secret access。
- sync engine 只能读标记为可同步的 redacted representation。
- LLM Advisor 使用独立 outbound consent boundary。
- runtime proxy 如启用，使用独立 profile、明确证书和一键撤销；主产品无需它即可完整运行。

## 13. 安全、隐私与数据治理

### 13.1 威胁模型

需防范：

- instruction/skill/plugin 中的 prompt injection 和隐藏 Unicode；
- scripts/hooks/MCP command 的任意代码执行；
- symlink、junction、archive traversal、path escape、扫描前后 TOCTOU symlink swap；
- FIFO、socket、device file、权限在扫描中变化、watcher race、超大文件和压缩炸弹导致阻塞或越界读取；
- secret 出现在配置、diff、日志、Receipt、截图或远端同步；
- 恶意 registry/package 替换、tag 漂移、依赖投毒；
- UI renderer 或第三方 adapter 越权读取 home；
- 恶意 Markdown/HTML、路径、URL、图片和 deep link 通过 WebView XSS、协议处理或 Tauri bridge 提权；
- 本地 API 被其他进程访问；
- SSH host impersonation、恶意 remote collector、版本降级或远端路径授权扩大；
- history/LLM 分析把源码或客户数据发送到外部；
- 管理员借治理功能查看开发者 prompt 正文；
- rollback 删除用户自有、未受管文件。

### 13.2 数据分类

- `Public metadata`：schema/version、adapter capability、公开 package metadata。
- `Local metadata`：路径的脱敏表示、hash、mtime、scope、findings。
- `Sensitive content`：instructions、source snippets、session text、tool results。
- `Secret`：tokens、credentials、private keys、auth headers、secret values。

默认策略：local metadata 可持久化；sensitive content 仅按用户选择存入加密 vault；secret 永不进入 Receipt、sync bundle 或 LLM payload。

### 13.2.1 本地 Vault 密钥生命周期

- `metadata-only` 模式不创建、不请求也不解锁 vault key；敏感正文只在内存中即时预览且不进入历史/索引。
- 第一次持久化敏感正文时生成独立随机 data-encryption key（DEK），内容使用成熟 AEAD 加密。DEK 由 macOS Keychain、Windows DPAPI/Credential Manager、Linux Secret Service，或用户明确选择的 age/SOPS/external secret manager recipient 包裹；不能安全存储时拒绝开启 vault。
- vault 有显式 create/unlock/lock 状态；默认闲置 15 分钟自动锁定，锁屏、登出、休眠和用户命令立即锁定。锁定时 daemon 只能处理 metadata，不能缓存解密正文。
- `wrapping-key rotation` 只重包裹 DEK；`content-key rotation` 生成新 DEK、以事务方式重加密并保留可回滚到上次完整状态的加密 checkpoint。轮换不改变 asset UUID；Receipt 使用的本地 keyed digest 重新派生，对外 redacted digest 依 §13.4 不暴露可关联的旧新值。
- 备份只包含加密 vault 与必要 manifest，默认不捆绑解密 key。恢复由 OS keystore 同步能力或用户选定的外部 age/SOPS/secret manager 完成；产品必须提供恢复演练，且明确提示丢失所有 wrapping recipient/key 会永久丢失正文。
- 私有目录权限在 Unix 上为 `0700`、key/encrypted files 为 `0600`；Windows 使用当前用户 ACL。启动和恢复时拒绝宽权限或其他 OS 用户可读的 vault，并在 macOS/Linux/Windows 多用户 fixture 中验证。

### 13.3 安全要求

- 所有 archive/package path 在解析后必须保持根目录 containment；明确说明这不等于执行 sandbox。
- 安装/更新锁定不可变 commit/digest；tag 变化触发 drift。
- Contexpect core、adapter、corpus 和外部 executor 更新必须来自签名发布/固定 digest，具有版本回退保护和 revoked-version denylist；更新失败不破坏当前已验证版本。
- secret scanner 在读取预览、diff、导出、同步和 LLM 发送前重复执行。
- 内容脱敏必须发生在日志和序列化之前，而不是只在 UI 遮罩。
- mutation 记录受管文件清单；rollback 不删除未受管文件。
- 本地 API 绑定 loopback/Unix socket，使用短期 credential 和 CSRF/origin 防护。
- SQLite/vault 支持完整删除、备份和迁移；删除后的派生索引同步失效。
- 企业/团队报告默认只含 policy status、hash、版本、scope 和 risk，不含正文。
- 敏感正文不写入普通 OS temp；确需中间落盘时使用应用私有、权限 `0600`、加密且有 crash recovery 的目录。不得声称在 APFS/SSD/备份上可保证物理安全擦除。
- crash report/core dump、diagnostic log、搜索索引和 telemetry 默认排除正文与 secret；本地内容目录设置平台可用的 no-index/backup-exclusion 标记。
- 数据库 backup、migration copy 和 executor artifact 遵循与源数据相同或更强的加密/保留策略；swap/系统快照等 OS 层风险在 threat model 和“只存 metadata”模式中明确说明。
- 卸载默认不删除用户原生配置；必须提供单独、可预览的“删除 Contexpect 数据/keys/cache/backups”流程，并列出无法召回的外部 Receipt、系统备份和远端 bundle。

### 13.4 Signed Receipt 安全合同

- 签名覆盖 canonical receipt envelope：schema、coordinate、claim/evidence digest、unknown、redaction policy、附件 manifest、parent receipt digest、signer id 和签名时间。未覆盖字段必须被 schema 明确列为 display-only。
- canonicalization 采用公开标准（建议 RFC 8785 JCS），签名 envelope 采用 DSSE 或等价公开格式；签名通过 `SignerAdapter` 复用 SSH signing、GPG、minisign、Sigstore 或组织已有 trust store，不自建设备 PKI。
- signer identity、信任链、过期和撤销由所选签名系统定义。产品可提供存于 OS keystore 的“本机连续性签名”，但它只证明同一安装连续性，不自动代表真实用户/组织身份。
- verification 必须区分：`trusted-valid`、`valid-untrusted-signer`、`revoked-signer`、`invalid`、`attachment-deleted`、`unsupported-schema`。
- trust policy 和本地 denylist 版本化；撤销不能让历史签名“消失”。`signed_at` 默认只是未受信本机时间，不能单独证明签名发生在撤销前；只有 SignerAdapter 提供可信 timestamp attestation 时才显示可验证时间结论。
- 原始内容 digest 只保存在加密本地存储。对外 Receipt 使用域隔离的 keyed/redacted digest；高度敏感或低熵小文件不导出可用于离线猜测的 digest，只导出随机 item id 和存在性声明。

### 13.5 E2EE Sync 安全合同

- bundle encryption 直接复用 age/SOPS 或安全 ADR 选定的成熟、可互操作格式；recipient 私钥由 OS keystore、用户现有 secret manager 或外部工具管理，Contexpect 不发明密钥交换、设备 enrollment 或恢复协议。
- 加密提供 confidentiality；bundle 的 sender authenticity 和 metadata 完整性由 §13.4 的 SignerAdapter 对 envelope 签名提供。header、generation、parent digest、recipient set、schema 和 ciphertext digest 均在签名覆盖内。
- 每台设备保存最近接受的 signed head 和单调 generation。旧 generation、父链断裂、未知/不可信 signer 或已从 recipient set 移除的写入必须拒绝或进入冲突态。
- 无透明日志时不承诺全局阻止服务器 equivocation；当同一父节点的不同 signed heads 被同一设备或设备间同步观察到时，必须检测分叉并显示，禁止静默选择。
- 移除 recipient 后轮换后续 bundle key；被移除设备不能解密未来 bundle。已经下载过的历史数据不能被收回，产品不得声称可远程擦除。
- 新 recipient 默认只能解密加入后的 bundle；历史 re-encrypt 必须显式选择。密钥备份和恢复由用户选择的 age/SOPS/secret manager 机制负责；丢失全部 recipient key 可能永久失去数据，UI 必须预警。
- 端点失陷可暴露其当时可访问的数据；产品不宣称对已失陷端点提供前向保密。recipient 移除只限制未来访问。
- transport provider 只负责存取 opaque bundle，不获得正文、secret 或 recipient private key。
- Git-backed provider 的历史、签名与 merge 完全委托 Git；文件夹 provider 只维护单调 generation、parent digest 和 latest pointer 以检测 replay/分叉，不实现自定义 DAG、自动 merge 或 CRDT。冲突解析只记录用户选择或外部 authority 返回的结果。

### 13.6 Egress 与 Corpus 治理

- registry、update、webhook、LLM、sync provider 和外部 adapter 的出站请求统一经过 allowlist/denylist、SSRF 防护、redirect 重验、proxy/TLS 和目标认证层。
- 默认阻止 loopback 之外的本机地址、link-local、云 metadata endpoint 和解析后落入私网的外部 URL，除非用户对具体 connector 显式授权。
- preflight/runner 参数、URL、header 和 secret 在写日志前结构化脱敏。
- fixture/corpus 中的厂商文档、真实会话和第三方配置必须记录来源、许可、允许的再分发方式、脱敏状态和 content digest；不能公开再分发的样本只保存本地生成 recipe 或 metadata。
- corpus 分为三套且 case id 不重叠：日常开发可见的 `development corpus`；由独立维护者生成并密封、直到 release candidate 冻结后才交给 gate runner 的 `sealed conformance corpus`；发布时从真实 anchor harness 临时采集、只保存脱敏 evidence/recipe 的 `live native-oracle cases`。实现代码不能根据 sealed/live 答案写特判。
- gate 报告分别列三套结果，不用 development corpus 的高分代替 sealed/live。任何 security-critical、secret、path containment、Unknown 假绿或 CI exit `2` case 在 sealed/live 上出现已知错误都不得发布。

### 13.7 不可信内容与 WebView 边界

- instructions、skills、plugin README、路径、URL、tool result 和会话正文全部视为不可信数据；默认用纯文本 viewer。
- Markdown 预览使用安全 profile：禁用 raw HTML、script、style、iframe、object、SVG active content、事件属性和 `javascript:/data:/file:` 等危险 scheme；远程图片/资源默认不加载。
- Content Security Policy 至少要求 `default-src 'self'`、无 `unsafe-eval`/内联脚本、connect 仅受控 localhost API 与明确授权 egress；打包资源使用 hash/nonce。
- Tauri bridge/本地 API 使用显式 command allowlist、结构化参数、路径重新解析和能力检查；不可信正文不能拼接 command 名、shell、文件路径或任意 URL。
- WebView 禁止任意导航和新窗口。外链只允许 `https` allowlist，经域名/重定向重验和用户确认后交给系统浏览器。
- `ctxpect://`/IDE deep link 必须有 versioned schema、长度/数量上限、严格 URL decode、project/receipt id 授权和 replay-safe nonce。未认证 link 最多打开只读页面；mutation、secret reveal、sync、LLM send 必须在本地 UI 二次确认。
- 安全 corpus 必须覆盖 stored/reflected XSS、Markdown link/image、SVG、Unicode scheme、双重编码、超长 URI、路径穿越、command injection 和恶意 deep link，全部不得到达 bridge 能力。

## 14. 非功能需求

### 14.1 正确性

- 在 adapter 明确可观察的 acceptance corpus 上，静态解析与 native oracle 的 item-level reconciliation 目标不低于 95%。
- 所有差异必须能分类为 parser defect、version behavior、surface gap、dynamic choice 或 unknown。
- 相同输入和版本重复扫描应产生相同 canonical digest；时间字段不参与内容 digest。
- Unknown 永不自动转成 pass、zero 或 absent。

### 14.2 性能与资源

- 中型仓库（生成器固定为 10 万普通文件、1,000 个候选 context asset、100 个 symlink、20 个 nested roots）冷扫描目标 5 秒内，单文件变化的增量扫描 500ms 内。
- 最低参考 lane：Apple M2/16GB/NVMe/APFS、Ubuntu 24.04 VM 4 vCPU/8GB/ext4、Windows 11 4 core/16GB/NTFS；WP-01 记录精确硬件、数据集 digest、冷缓存和热缓存各 5 次结果，门禁取中位数且任何一次不得超过目标 2 倍。
- daemon 稳定后 30 分钟空闲窗口按 1 秒采样：平均 CPU <1%、P95 <3%，RSS P95 <150MB；后台 importer 必须处于 idle。超过阈值可见并可单独禁用 importer。
- UI “首个有意义结果”定义为 coordinate bar、最近 Receipt 时间、risk/unknown count 和可点击 Inspector skeleton 同时可交互；在已有 snapshot 时目标 2 秒内，后台刷新不阻塞浏览。
- 大文件、历史和 archive 有明确大小/数量上限及跳过理由。

### 14.3 可靠性

- 扫描和导入失败不得破坏现有数据库；事务回滚并保留上一个有效 snapshot。
- apply 是原子或可恢复的；崩溃后能判断 committed/rolled-back/in-doubt。
- sync 冲突不自动 last-write-wins；必须可重放、可人工合并。
- schema/adapter migration 有备份和 downgrade/restore 说明。

### 14.4 跨平台

- 产品架构必须支持 macOS、Linux、Windows；完整交付的正式矩阵需在实施 ADR 中锁定具体版本。
- 测试覆盖 case sensitivity、Windows junction/path、WSL、Unix symlink、不同 home/config roots。
- 不假设 Git、Node、Python 或特定 shell 永远存在；依赖缺失显示 capability gap。

### 14.5 可访问性与本地化

- 核心 UI 达到 WCAG 2.2 AA；完整键盘操作、屏幕阅读器、非颜色状态。
- 首发支持简体中文和英文；术语在两种语言中映射稳定。
- CLI 默认英文机器字段，human output 可本地化。

## 15. 现有项目的复用方案

### 15.1 直接集成优先级

| 能力 | 方案 |
|---|---|
| Package/manifest/lock/registry | 读取并可调用 Microsoft APM；Contexpect 只展示和核对结果 |
| Cross-tool desired-state projection | §F-09 的 required core cell 由 `contexpect-native` 承担；其他 cell 可冻结 APM/Agentpack/agentsync 为唯一 authority；同一 cell 不做第二 projector |
| Codex 专项验证 | 优先把固定 commit 的 CtxWise JSON/fixture surface 作为 Codex `AdapterProvider`；若该版本/环境没有对应 capability，则退回 bundled static resolver 并标明 Observed 缺口，二者都跑独立 golden/native-oracle test；遵守 Apache-2.0 NOTICE |
| Runtime/cost/OTLP | 导入 Scopeon 输出或对接 OTLP，不重造通用 metrics backend |
| Session history | 提供 ctxray-compatible import 或直接读取其标准输出（若稳定） |
| Runtime proxy | ContextSpy 作为高级外部 adapter，不内置默认 CA/MITM |
| Standards | 原生支持 AGENTS.md、Agent Skills、Agent Plugins、MCP、OTel GenAI |

### 15.2 不直接依赖的风险控制

- 所有 integration 通过 versioned capability contract，不解析不稳定的人类终端文本作为唯一接口。
- 外部项目没有稳定机器接口时，先采用导入文件，不 fork 大量代码。
- 依赖前核验 license、维护状态、security policy、release pin 和 transitive licenses。
- README 宣称的指标不作为 Contexpect 的 truth；仍由自身 receipt/evidence 标注。
- agentsync 等可能在 backup/local Git 或其他 artifact 留下解析后 secret 的 executor，在通过 secret artifact gate 前只允许 read-only/import/export plan，不作为 secret-bearing 完整路径。

## 16. 完整交付工作包

以下是一个产品的依赖顺序，不是范围裁剪路线。任何工作包都不能作为“产品已完成”的单独发布判定。

### WP-01 契约、Fixture 与威胁模型

- 固定 Context IR、Receipt/Evidence schema、Unknown taxonomy、context capability/claim validity/projection matrices 和 adapter manifest。
- 依据 §17.0 的固定版本截止时间建立并冻结四 harness 的精确 version/surface/OS acceptance matrix；冻结来源、fixture corpus、计算公式和 reference hardware。
- 对 APM、Agentpack、agentsync、Scopeon、ctxray、ContextSpy 逐项冻结为 `CLI`、`stable API`、`file import` 或 `borrow only`，并定义外部项目不可用时的降级：只读展示/导出 plan/Unknown，不能悄悄启用自建替代实现。
- 创建跨平台 fixture workspace 和 golden receipts。
- 完成数据分类、威胁模型、许可矩阵、corpus 再分发规则和 redaction contract。

验收：schema 可验证；fixtures 覆盖各类 include/exclude/override/cap/loss/unknown；没有后续组件自创第二套术语。§17.0 的全部 acceptance artifact 必须已生成、固定 digest，并由未参与编写的 reviewer 逐项核对；在该 gate 通过前不得开始 WP-02。

### WP-02 Core Collector、Resolver 与 CLI

- 实现环境探测、inventory、四 harness static resolver。
- 实现 Task Probe/Preflight、one-shot CLI、Receipt 生成/验证、deterministic Doctor。
- 建立 native oracle reconciliation tests。

验收：fixtures 全绿；重复扫描 digest 稳定；未知版本 fail closed；CLI 在无 UI/daemon/账号下完成核心检查。

### WP-03 SQLite、Snapshot、Diff 与 Evidence Ledger

- 实现持久化、迁移、snapshot、跨坐标 diff、baseline lock、SARIF/JSON/Markdown export。
- 建立 retention、delete、backup/restore。

验收：断电/失败不损坏上次有效 snapshot；diff 可追溯首次变化；删除会话清理派生结果。

### WP-04 Inspector UI 与本地 API

- 实现主信息架构、Inspector、Compare、Receipts、Assets、Standards。
- 实现隐私模式、可访问性和中英文。

验收：用户能在两分钟内从异常进入 resolution path 和 evidence；Unknown 不被隐藏；UI 不直接获得任意文件系统权限。

### WP-05 Runtime Importer、Session 与 Daemon

- 实现四 harness 可用的 native importers、field-to-claim mappings、timeline、session storage，以及 capability taxonomy 中动态类别的 observed/unknown 表达。
- 实现可选 daemon、incremental scan、local notifications、scheduler。

验收：每个 importer 明确覆盖范围；Codex partial prompt 不被标 full；daemon 资源和通知去重达标；one-shot 仍独立可用。

### WP-06 Intent、Projection、Apply 与 Rollback

- 实现 Intent、ProjectionAuthority binding、`contexpect-native` required adapter、preview、authority loss report、独立 post-projection finding、concurrency guard、调用审计和 apply 后核对。
- 逐 cell 使用 WP-01 冻结的唯一 authority 与 File/Git transaction executor 执行 projection、mutation、snapshot 和 rollback；§F-09 的 required-write cell 不允许退化为导出，observe/export cell 无合格 executor 时才只导出 intent/plan。
- 对 executor argv/env/stdin/stdout/stderr/temp/backup/snapshot/local Git/crash artifact 运行 secret gate；不合格 executor 禁止 secret-bearing transaction。

验收：`acceptance/projection-matrix.yaml` 每个 required-write cell 都完成 preview/apply/rollback/post-Receipt；有损字段逐项显示；并发变化阻止覆盖；rollback 不删未受管文件；不得用一条“代表路径”代替矩阵全量执行。

### WP-07 加密多设备同步

- 复用 age/SOPS 类加密格式和 SignerAdapter，实现 recipient add/remove/rotation、用户现有密钥备份接口、replay/rollback/divergence detection 和 conflict handoff；Git provider 委托 Git history/merge，folder provider 不实现自定义版本图或自动 merge；不自建设备 PKI。
- 内置本地文件夹与 Git-backed provider；其他远端由 provider adapter 或用户已有文件同步传输，core 不自建通用 WebDAV/S3/账号控制面。

验收：远端无解密密钥不能读取正文；secret 值从未进入 bundle；设备同步成功后还需 semantic reconciliation；离线冲突不丢数据。

### WP-08 生态资产管理与供应链安全

- 实现 Assets catalog、standard validation、registry/Git/APM import、license/provenance/signature/SBOM 展示、update plan 和应用后核对；package 生命周期交给 APM/Git executor。
- 实现 hidden Unicode、path escape、script/hook/MCP permission 检查。

验收：恶意 fixture 被阻止或清楚警告；无许可证内容不能复制进核心；安装 pin 可复现；没有自建 marketplace backend。

### WP-09 LLM Advisor 与历史洞察

- 实现 consent UI、payload preview、本地/远端 provider、evidence-linked suggestions。
- 实现历史 importer、重复纠正、skill candidate、unused-within-observed-sessions、隐私和趋势。

验收：默认不联网；发送字段可见；建议不伪装 observed；删除历史后派生建议失效；无证据不输出“有用/无用”。

### WP-10 Context Effect Lab

- 实现 experiment contract、task suite、control/treatment、runner adapter、gate capture、成熟统计库适配和混杂检测；不自建通用 runner/sandbox/统计估计器。
- 与 Receipt、history、cost/OTLP 打通。

验收：单次 before/after 不产生因果结论；代码/model/harness 漂移会使实验失效或分层；结果可复跑和导出。

### WP-11 Policy、CI、IDE/API 与集成收口

- 完成稳定 exit codes、SARIF、CI 模板、本地 API、adapter SDK、IDE deep links、OTLP/export。
- 实现 context-specific PolicyRevision/Binding/Evaluation、tighten-only、签名信任、Approval、offline indeterminate 和 AuditEvent；通过冻结 integration 只读导入 APM authority 的 package/source/install evaluation，验证 `domain + check_id` 唯一归属。
- 完成跨平台安装、升级、迁移、卸载和文档。

验收：CLI/UI/daemon/CI 使用同一 core 和 Receipt schema；policy pass/deny/indeterminate 与 0/2/3 一致；过期/冲突/未验证 policy 不假绿；卸载不删除用户原生配置；第三方 adapter 权限最小化。

### WP-12 全产品集成验收

- 在代表性个人、多设备、team repo、plugin author、research eval 场景运行端到端测试。
- 完成安全测试、隐私复核、许可审计、性能、可访问性、恢复演练和数据删除验证。
- 全部工作包通过后才进入用户验证。

## 17. 产品级验收合同

### 17.0 冻结的 Acceptance Surface

WP-01 必须生成并评审以下机器可读基线，后续工作包不得用文字自行放宽：

- `acceptance/compatibility-matrix.yaml`：精确 OS、架构、harness version、surface 与 `required-supported / required-unknown-honesty / not-applicable`。
- `acceptance/corpus-manifest.json`：development/sealed/live 三套 corpus 的每个 fixture、golden claim、来源、许可、digest、敏感级别、独立维护者/密封状态和适用坐标。
- `acceptance/claim-validity-matrix.yaml`：合法 claim tuple、最低 provenance/coverage、冲突与 precedence。
- `acceptance/context-capability-matrix.yaml`：16 类静态/动态上下文在每个 harness/version/surface 上的 supported/unknown-honesty/N/A、collector/importer 和 field mapping。
- `acceptance/projection-matrix.yaml`：不得低于 §F-09 明列的 harness × asset kind × scope required-write 下限；逐 cell 固定唯一 authority/version、native target、loss contract、preview/apply/rollback/post-Receipt gate。`export-only` 不能填入 required-write cell。
- `acceptance/traceability.csv`：覆盖本文全部规范性语句，而不只 F-01–F-18。每行含 stable requirement id、章节、原文 digest、MUST/不得类型、WP、test/gate、artifact、owner；由 lint 提取“必须/不得/不能/禁止/只有……才”等词并验证零未追踪条目。§17.6 表只作高层摘要。
- `acceptance/reference-hardware.md`：性能测试硬件、文件系统、冷/热缓存条件和数据集生成器 digest。
- `acceptance/integration-contracts.yaml`：外部项目的版本、集成模式、capability 和不可用时降级。

Acceptance version cutoff 固定为 `2026-09-04T23:59:59+08:00`。在该时间之后发布的 harness 或 OS build 不会自动进入 required scope；升级 required matrix 必须提交显式 acceptance-contract revision，重新跑受影响 gate，不能由 code freeze 日期隐式改变。

完整交付至少覆盖：

- OS lanes：macOS `27.0` build `26A5425a` arm64；Ubuntu 24.04 LTS x86_64 和 Windows 11 24H2 x86_64 选择截止时间前已发布的官方 image，并在 WP-01 用不可变 image digest/build number 固定。
- required anchor versions：Codex CLI `0.147.0`、Claude Code `2.1.259`、Cursor IDE `3.19.7`、Cursor Agent CLI `2026.08.25-3e8eec8`、Grok CLI `1.0.13`。这些是截止时间前[本机实测 anchor](../research/2026-09-04-local-environment-receipt.md)；在其他 OS 上若厂商未发布同版本/surface，标 `not-applicable` 并保存分发/官方证据。
- required local expansion versions：OpenCode `1.18.21`、Kimi Code `0.40.1`、ZCode `3.10.2`、Qwen Code `0.18.0`、Goose `1.37.0`、Gemini CLI `0.55.1`、GitHub Copilot CLI `1.0.82`、Kiro CLI `2.9.0`。每个 coordinate 至少进入 `required-static` 与 `required-unknown-honesty`；只有 adapter manifest 声明且 oracle fixture 可重复的 capability 才进入 `required-native-observed`。
- DeepSeek Harness、Coze、Cline、Aider、OpenHands、Windsurf 在本机 live lane 为 `not-installed` 或 `connector-required`；WP-01 必须按 cutoff 前官方 release 固定 hermetic fixture/version coordinate，不能虚构本机验证。DeepSeek 的 `~/.dsh` 配置残留与 Coze recent-item 痕迹不能改变该结论。
- 截止时间后的当前版本只进入 non-blocking `smoke/unknown-honesty` lane，不改变完成目标，也不能建立权威 baseline。
- surfaces：除四个 anchor surface 外，扩展 family 按 §11 表分别冻结 CLI/IDE/Desktop/SDK/ACP/cloud 中实际纳入的 coordinate。静态 Expected 和原生 Observed 的覆盖分别声明；没有官方 observed surface 时进入 `required-unknown-honesty`，不能从验收分母消失。

每个 `required-supported` adapter coordinate 至少有 60 个 static golden cases，覆盖 include、exclude、override、cap、ignore、conditional、progressive、legacy、filesystem edge 和 negative input；有 native oracle 的 observed surface 至少有 12 个可重复 oracle cases。安全关键 fixture 不计入上述最低数且必须另列。

WP-02 的硬前置条件是上述八份 artifact 已生成、内容 digest 冻结并通过独立 review；实现开始后不能用修改 acceptance 答案消除失败。确需修订时必须新建 contract revision、说明原因并重跑全部受影响 gate。

### 17.1 真实性门禁

- `item discovery F1 = 2PR/(P+R)`，其中预测和 golden 都按 stable item id + revision 计算；每个 required coordinate 必须 ≥0.95，任一 activation/cap/override 类别不得低于 0.90。
- `claim tuple accuracy = 正确的 lifecycle_stage + truth_state + resolution cause / 全部 golden claim tuple`，每个 required coordinate 必须 ≥0.95。
- 分母只排除 `not-applicable`。只有 golden 本身为 Unknown 时，预测 Unknown 才算正确；工具不暴露的 surface 进入独立 unknown-honesty corpus，不得从产品完整性中消失。
- truncation、secret redaction、path containment、未知版本和 partial coverage 分类必须 100% 通过，不允许用总体 95% 掩盖。
- `doctor-corpus.json` 至少 250 个标注 case（至少 125 clean、125 reachable issue，且每个可阻断规则不少于 20 个正负例）。全部 deterministic Doctor 规则在声明 grammar 内 precision ≥0.98、recall ≥0.95；任何能触发 CI exit `2` 的规则在该 corpus 上必须 precision = 1.00。达不到的规则只能 non-blocking warning，并显示误差说明。
- identity corpus 必须覆盖 managed/unmanaged、rename/copy/fork、inode 变化、跨设备、key rotation 和低熵敏感内容；只有合同允许的场景自动沿用 identity，其余产生 candidate/unknown。
- 每个数字和状态都分别带 claim kind、truth、provenance、coverage、precision、knowledge status；不再使用混合单轴标签。
- context capability matrix 的每个 `required-supported` 单元都有正负 golden；每个 `required-unknown-honesty` 单元证明 UI/Receipt/Gate 显示 Unknown；任何 taxonomy 类别都不能无状态缺席。
- development、sealed conformance、live native-oracle 三套结果必须分别过线；sealed 答案在 release candidate 冻结前不可供实现者读取。security-critical、secret、path containment、Unknown 假绿和 CI exit `2` fixture 为零容错。
- 任一不可见项都不能显示成 absent、0 token 或已对齐。
- 用户能从任何 finding 进入 evidence 与 resolution path。
- Codex `prompt-input`、session aggregate token 等已知 partial surface 有专门回归用例。
- semantic reconciliation fixture 必须覆盖四个状态、每个 required projection cell、policy deny/indeterminate、native oracle 缺失、unapproved loss 与 transaction digest mismatch；`structural-only/indeterminate` 不能被 UI、CLI 或 CI 展示为 verified。
- Receipt equivalence fixture 必须覆盖允许的 device/path/signature 差异和不可忽略的 intent/scope/loss/policy/claim 差异；每设备 keyed digest 不可比较时不得假等价。

### 17.2 内部全功能可用性门禁

这是 WP-12 的内部 release gate，发生在 F-01–F-18 已实现之后、18 节外部用户验证之前。使用 8 名未参与设计/开发且未读本文的 QA 参与者，在固定 fixture、干净设备 profile 和内置帮助下完成；不得由作者口头提示。每项至少 7/8 成功，时间从打开指定项目起算，答案必须点击到 evidence 才算正确：

1. 在 2 分钟内解释一条“Claude 生效、Codex 缺失”的根因。
2. 在 2 分钟内指出一条只在设备 A 的配置及同步风险。
3. 在 3 分钟内识别 cap/truncation 或有损 projection，并找到证据。
4. 在 5 分钟内完成一次安全 preview/apply/rollback，且未修改无关文件。
5. 在 5 分钟内生成并验证一个默认脱敏 Receipt。
6. 能准确说出哪些信息是 Expected、Observed partial 和 Unknown。

第 4 项由冻结的外部 executor 完成 mutation/rollback；Contexpect 自身只负责 preview、授权、审计和核对。失败录像、操作 trace 和问卷原文作为 gate artifact 保存并脱敏。

### 17.2.1 Cadence-specific E2E 与 Soak

- 长期开启：全部 required OS lane 运行 72 小时 daemon soak，注入 1,000 次重复/不同变更；同一 digest 不重复通知，变化无丢失，资源满足 §14.2。
- 定期保养：模拟 4 个周期，报告只列新增/恶化/解决，观察窗口和 Unknown 始终可见。
- 迁移同步：设备 A→B 完成 encrypted bundle、recipient 添加、冲突、移除与 key rotation；secret 不随 bundle，语义核对生成独立结果。
- 历史洞察：导入带标注的 50-session corpus，未导入和不可观察 surface 不得被归为“从未使用”；删除后派生结果失效。
- Effect Lab：用预构造 beneficial、harmful、equivalent、inconclusive 数据和至少一个真实小型 runner fixture，决策必须与冻结 ExperimentContract 一致。
- 偶发诊断/任务前/会话中/事件触发分别有端到端 fixture，覆盖 one-shot、preflight link、partial timeline 和 CI exit 0/2/3。

### 17.3 安全与隐私门禁

- 测试 secret 不出现在日志、SQLite metadata、Receipt、sync bundle、LLM payload、SARIF 或崩溃报告。
- 测试 secret 不出现在 ProjectionAuthority 的 argv/env/stdin 回显、stdout/stderr、temp、backup、snapshot、local Git history 或 crash artifact；不合格 executor 的 secret-bearing path 必须被禁用。
- archive traversal、symlink escape、恶意 script/hook/MCP fixture 被阻止或要求显式隔离授权。
- TOCTOU symlink swap、FIFO/socket/device file、权限变化、watcher race、压缩炸弹和恶意 shell/path/env 输入均有负例且全部通过。
- sync transport/storage provider 在仅持有远端数据时无法解密用户内容。
- metadata-only 模式不生成 vault key；vault create/unlock/auto-lock/lock-screen、wrapping-key/content-key rotation、中断恢复、key 丢失提示、外部恢复演练与 Unix/Windows 多用户权限 fixture 全部通过。
- Receipt 对篡改字段/附件、未知/撤销 signer、重放旧 bundle、远端 rollback 和设备间分叉必须产生合同规定的非通过/冲突状态。
- 对外 Receipt 不包含可对低熵敏感文件做离线确认的裸 digest。
- WebView/XSS/deep-link corpus 全部不得触发脚本、导航、任意网络、bridge command、文件读取或 mutation；CSP 和 Tauri command allowlist 有自动化检查。
- 删除 session/project/all data 后，无可查询的正文和派生 insight 残留。
- rollback 保留未受管文件和用户并发修改。

### 17.4 可靠性与兼容门禁

- `compatibility-matrix.yaml` 中全部 required lane 执行，不做抽样推断；`not-applicable` 必须有工具官方支持证据。
- 相同输入重复扫描 canonical digest 一致。
- 中途终止 scan/import/apply/sync/migration 后能恢复到可判定状态。
- 新 harness 版本未验证时清楚降级，不产生假绿。
- CI 对 required indeterminate 默认 exit `3`；验证不存在 exit `0` 假绿路径。
- policy precedence、签名、过期、Approval scope/use count、离线 stale 和 audit chain fixture 全部通过；`policy pass` 与 F-18 定义一致。
- policy authority fixture 证明 package/source/install/license/digest/SBOM check 只采用 APM evaluation，context-specific check 只由 Contexpect 评估；同一 `domain + check_id` 的双 authority 配置必须失败而非择一。
- projection matrix 的每个 required-write cell 在全部适用 anchor OS/harness lane 执行完整 preview/apply/rollback/post-Receipt；任一 cell 为 export-only、缺 authority 或缺 rollback 时 gate 失败。
- CLI one-shot 在没有 daemon、UI、账号、网络的环境可完成核心 inspect/diff/doctor/receipt。

### 17.5 完整性门禁

- F-01 至 F-18 均有实现、文档、测试和端到端路径。
- WP-01 至 WP-12 均通过各自验收。
- 所有明确不做项没有被变相引入。
- 开源复用通过 license/NOTICE/SBOM 审计。
- 一次全产品集成门禁绿 + 一次独立 readback 绿，才可宣布完整交付。

### 17.6 F → WP → Gate 追踪

| Requirement | Work package | 主要 gate / artifact |
|---|---|---|
| F-01 | WP-01, WP-02 | compatibility matrix；environment fixtures；unknown-version exit 3 |
| F-02 | WP-01, WP-02 | corpus discovery F1；filesystem/security negatives |
| F-03 | WP-01, WP-02 | claim tuple accuracy；native oracle reconciliation |
| F-04 | WP-02, WP-04, WP-05 | task probe golden；shell/env injection negatives；preflight→runtime link |
| F-05 | WP-01, WP-03 | Receipt schema/signature verification；tombstone/delete tests |
| F-06 | WP-04 | internal usability gate；WCAG audit；privacy screenshot mode |
| F-07 | WP-03, WP-04 | cross-coordinate golden diff；no numeric-confidence check |
| F-08 | WP-02, WP-08 | deterministic Doctor corpus；false-positive review；SARIF |
| F-09 | WP-06 | executor contract；loss report；concurrency/rollback/unmanaged-file tests |
| F-10 | WP-07 | crypto vectors；replay/rollback/equivocation/device revoke tests |
| F-11 | WP-01, WP-08 | integration contract；license/provenance/SBOM；malicious package corpus |
| F-12 | WP-05 | importer coverage manifest；partial/unknown honesty；retention/delete |
| F-13 | WP-09 | consent/payload/egress tests；evidence-linked suggestion checks |
| F-14 | WP-09 | observed-window wording invariant；history deletion/derivation invalidation |
| F-15 | WP-10 | frozen ExperimentContract；statistical decision fixtures；runner receipt |
| F-16 | WP-05, WP-11 | daemon resource/notification tests；CI exit contract；offline gate |
| F-17 | WP-01, WP-11 | JSON Schema/API compatibility；adapter sandbox/egress；import traversal |
| F-18 | WP-01, WP-11 | policy precedence/binding/signature/expiry/offline fixtures；Approval scope；audit chain；CI 0/2/3 |

## 18. 用户验证与成功指标

验证发生在完整产品通过内部验收之后，符合用户“一次做完整再验证”的选择。

### 18.1 验证对象

- 5–8 名多工具个人重度用户；
- 2–3 名顾问/agency 工程师；
- 2 个小团队或开源维护者；
- 2 名 skill/MCP/harness 作者；
- 1 个研究/eval 用户；
- 若可获得，1 个 metadata-only 安全/平台团队。

### 18.2 North Star

`Evidence-backed context deviations resolved per active user`：用户通过证据链确认并处理的上下文偏差，而不是扫描次数或“优化分”。

### 18.3 核心指标

- 首次扫描后能指出至少一项真实偏差的用户比例。
- Time to Explain 的中位数和 P90。
- Receipt 在另一设备/成员处成功复验的比例。
- Doctor finding 的确认率、误报率和修复后不复发率。
- Unknown coverage 随 adapter/runtime integration 提升的变化，但不得通过猜测降低 Unknown。
- sync 冲突率、自动无损合并率、rollback 成功率。
- 历史建议被接受并经 effect lab 证实的比例。
- daemon 被长期保留的比例与资源投诉。
- 偶发用户从安装到得到首个答案的时间。

### 18.4 不应作为主指标

- 扫描文件数、生成 finding 数、LLM 建议数。
- 单纯 token 减少；若质量下降，节省无意义。
- 统一“context health score”。它会掩盖 evidence quality 和不同风险。
- 社区包数量；供应链质量比数量重要。

## 19. 产品边界：完整交付仍明确不做

- 不提取或声称还原隐藏 system prompt、内部 reasoning 或不可观察的完整 provider-wire payload。
- 不支持所有 coding agent；完整交付冻结 §11 的 18 个 adapter family 与声明 surface，并提供 adapter SDK。新增 family 必须走 acceptance-contract revision，不能由动态 registry 自动升级成权威支持。
- 不自建通用 package manager、依赖解析器或 marketplace backend。
- 不自建多租户企业 SaaS、SSO/SCIM、计费和管理员内容后台。
- 不默认 MITM、安装 root CA 或代理全部模型流量。
- 不把 instructions 当权限 enforcement。
- 不提供无证据的“健康总分”“价值百分比”“加载概率”。
- 不默认上传正文、历史或 secret，不同步明文 secret。
- 不自动修改真实配置；所有修改有 preview、确认和 rollback。
- 不以 proprietary universal config 替代 native files。
- 不引入图数据库，除非真实测量证明 SQLite 达不到要求并有 ADR。
- 不把一次 A/B 或模型主观点评当因果结论。

## 20. 主要风险与应对

| 风险 | 后果 | 应对 |
|---|---|---|
| 厂商规范快速变化 | resolver 过时、假结论 | versioned adapter、native oracle、fixture corpus、unknown fail-closed |
| 运行时 surface 不透明 | “真正 prompt”承诺落空 | Expected/Observed/Unknown、partial coverage、Receipt 明示缺口 |
| 范围看似小实际跨多个系统 | 交付周期失控 | 以 18 个功能合同和 12 个工作包锁边界；大量复用现有生态 |
| 云同步泄密 | 信任崩塌 | E2EE、secret refs、allowlist、metadata-first、无自建账号要求 |
| LLM 建议制造新错误 | 用户误改配置 | 建议与 deterministic finding 分开；evidence link；preview/rollback |
| 跨工具 projection 有损 | 表面一致、行为更差 | capability matrix、loss report、native overlay、应用后 Receipt |
| 历史效果分析混杂 | 错删有价值规则 | controlled effect lab、inconclusive、样本和反例可见 |
| 后台常驻太重/太吵 | 用户卸载 | daemon 可选、增量 hash、事件去重、安静默认、资源上限 |
| 自建生态重复 APM 等 | 浪费且难维护 | integration-first；Contexpect 专注 reconciliation 和 evidence |
| 品牌冲突 | 难搜索/注册 | 暂用 Contexpect，开发前做正式 trademark/domain/package clearance |

## 21. 商业与开源建议

### 21.1 开源边界

建议核心 scanner、resolver、Receipt schema、CLI、adapter SDK、local UI 和 deterministic Doctor 采用 Apache-2.0。原因是企业采纳、专利授权和 adapter 生态更匹配；正式决定前完成依赖许可审计。

### 21.2 可收费能力

不依赖封闭核心也有商业空间：

- 托管但 E2EE 的多设备同步；
- 私有/优先 adapter 与兼容性 SLA；
- 团队 policy、签名 Receipt、fleet metadata、审计导出；
- 私有 registry integration 和供应链策略；
- 企业支持、air-gapped package、定制 conformance；
- 大规模 effect lab runner 和报告协作。

本次完整产品不要求先建设多租户 SaaS；付费服务可在本地产品被验证后围绕既有开放协议提供。

### 21.3 护城河

- 高质量、跨版本、跨 surface 的 compatibility corpus；
- 与 native oracle 对账的 Evidence Ledger 和 Receipt 可信度；
- 可交换的 bug/incident artifact；
- 安全且诚实的跨工具 diff 和有损说明；
- 第三方 adapter、fixture 和组织 policy 生态。

## 22. 命名建议

最终推荐：`Contexpect`。

- 产品名：Contexpect
- CLI：`ctxpect`
- 核心产物：Context Receipt
- 默认主入口：Context Doctor（体检/问诊/证据收集）；它是产品内工作流名，不作为独立品牌
- 英文短句：Expected. Observed. Reconciled.
- 中文描述：可视化 AI coding context 核对与控制工具

不推荐 `织境 Loom`，因为 Loom 已是 Atlassian 的大型软件品牌；“织境”可保留为内部视觉概念，但不与 Loom 绑定。也不推荐 `ContextOps`，因为 GitHub 和 PyPI 已存在同名项目。

名称初筛不等于可注册。进入代码与公开发布前必须做 GitHub、crates.io、npm、PyPI、域名和商标正式清查。

## 23. 最终产品判断

这个项目值得做，且最有价值的不是“把配置放到一个漂亮页面”，而是把 AI coding context 变成一种可核对、可携带、可解释、可验证的工程资产。

它也并不算一个天然“小项目”。如果完整包含 18 个 adapter family 的声明 surface、运行时观察、跨设备加密同步、安全 mutation、生态资产、历史分析和因果评测，复杂度主要来自兼容性和信任边界，而不是页面数量。通过统一 Adapter SDK、按 capability 验收并复用 APM、Agentpack/agentsync、OTel 等现有能力，可以避免把它做成 18 套特例；但不能通过删掉证据模型或把 Unknown 猜成真值来假装简单。

最合适的产品姿态是：

> 平时安静地守着，任务前给一张 Receipt，出问题时两分钟解释原因，换设备时安全复现，想优化时用自己的任务做对照。

当这五件事在 18 个 adapter family 的声明 surface 上形成同一套 Receipt 语言，并且无法观察处仍能诚实解释 Unknown，Contexpect 才完成了它的完整价值。
