# AI Coding Context 管理项目研究台账

> 日期：2026-09-04  
> 状态：Canonical research ledger  
> 作用：在完整产品需求生成前，固化本会话中已经形成的事实、推断、反例、错误路线、命名判断与来源。后续方案若与本文冲突，应显式说明新证据及替代关系，不得静默覆盖。

## 1. 结论先行

这个项目有真实且持续扩大的价值，但它不应被定义为“规则编辑器”“Prompt 模板库”或“又一个跨工具配置同步器”。更准确的产品定义是：

> 跨 AI coding harness 的上下文核对与控制平面。它针对指定设备、工具及版本、项目、工作目录和任务，解释哪些上下文按规则应该出现、哪些被运行时观察到、哪些仍不可见，以及这些状态之间为何发生漂移。

产品成立的关键不是收集更多文件，而是建立一套可验证的真值模型：

- 把“已安装、可发现、满足激活条件、模型可见、外部使用线索、影响结果”拆成六个不同层级，避免声称看见模型内部归因。
- 把静态解析、运行时观察和效果评估拆成三个不同对象。
- 每个结论都附带证据、采集器版本、工具版本、时间、内容指纹、测量质量和未知原因。
- 对不透明部分明确显示 `Unknown`，不能用估算、模型猜测或空值伪装成事实。

项目可以一次完成全部既定产品范围，但“完整交付”不等于没有内部依赖顺序。实现仍必须先建立解析和证据底座，再接可视化、同步、分析、历史评估和团队能力；全部工作包完成并通过统一验收后，才算项目完成，不以中间工作包代替最终产品验证。

## 2. 用户原始问题

用户同时使用 Codex、Claude Code、Cursor、Grok Build 等工具，并在多台设备上开发。每次提问的有效上下文可能来自：

- 全局、团队、项目、目录、本机或会话级指令文件；
- skills、plugins、MCP server 声明及动态 tool schema；
- 用户或团队 UI 设置、记忆、会话历史、`@` 文件、压缩摘要；
- 工作目录、Git 根、环境变量、工具版本、账号策略和远程执行环境；
- 子代理、动态工具发现、运行时工具结果和其他不可静态读取的内容。

由此形成三个核心问题：

1. 跨工具是否对齐：某条规则在 Claude Code 生效，不代表 Codex、Cursor 或 Grok 会发现或以相同语义激活。
2. 跨设备是否一致：项目文件可以随 Git 移动，本机配置、skills、MCP、UI 设置和账号策略可能只存在于设备 A。
3. 实际状态是否符合预期：用户不知道当前上下文由什么组成、占用多少、有没有遗漏、冲突、过时、泄密或冗余，也不知道某项配置是否真正影响结果。

## 3. 两份外部 AI 回复的价值与局限

本会话完整阅读了两份用户提供的 AI 回复：

- 第一份以 “AI Coding Context Control Plane” 为题，提出 ContextOps、Effective Context、Semantic Drift、Context Debt、Context IR、Expected/Observed/Opaque、Bundle Analyzer、Context Doctor、云同步、社区与历史分析等概念。
- 第二份以“织境 Loom”为题，整理了四家工具的加载规则、always/conditional/progressive/runtime/opaque 加载态、意图对齐、示范工作区和按 L0–L6 的建造顺序。

两份原文的路径、字节数、SHA-256 和 26 条核心命题逐项去向已固定在[用户提供方案的来源快照清单](source-snapshots.md)。原始附件不是本文可改写的内容；后续若归档到仓库，副本 digest 必须一致。

### 3.1 应保留的洞见

- `Inspector first`：先知道发生了什么，修改与同步才有可信依据。
- `Native files first`：尊重各工具原生文件和能力，不强迫用户迁移到专有格式。
- `Canonical intent + native overlays`：跨工具共享的是意图，不是强行复制完全相同的文件。
- `Expected / Observed / Opaque`：区分规范推演、运行时证据和不可见部分。
- 工作目录是解析输入，不是 UI 上可忽略的参数。
- skills 必须区分启动时目录/元数据成本与激活后的正文/资源成本。
- 云同步首先是信任、密钥和冲突问题，不是网盘问题。
- 动态层即使不可读取，也必须在可视化中有位置。
- 转换必须展示损失，禁止静默丢字段。
- 规范解析器应以版本化夹具和 conformance 测试维护。

### 3.2 必须纠正或收紧的部分

- “静态文件只占故事三成”是启发式表达，不是可泛化统计。产品不能展示该比例为事实。
- “真正拼进模型的东西”不能作为所有页面的无条件承诺。只有原生诊断或受控运行时证据覆盖到的 surface 才能称 `Observed model-visible`；其他只能称 `Expected` 或 `Unknown`。
- “loaded”不能直接翻译为“有用”。文件可发现不等于被激活，被激活不等于被模型采用，被模型采用也不等于改变结果。
- 语义对齐不能输出无校准的 83%、91% 一类精确分数。默认使用可解释的结构化差异；若以后出现数值，必须给出标注集、误差和阈值来源。
- 任务模拟不能输出“82% 可能加载”之类无验证概率。可以输出规则满足/不满足和未知原因。
- LLM 评审只能是建议器，不能充当真值来源，也不能在没有对照会话时断言效果。
- 云同步、社区市场、团队治理和历史效果分析不是“没有价值”，但不应在数据模型之前各自发明独立系统。
- `Context Graph` 是有用的视图，不是必须采用图数据库。初始持久层用 SQLite 的关系表和边表足够。
- 不应再发明一个替代所有原生配置的公共 manifest。已有 APM、Agentpack、agentsync 等项目覆盖依赖、投影和部署；本项目应读取或集成它们。
- “AGENTS.md 是唯一圣经”过强。它适合作为跨工具共享的项目指令之一，但无法无损表达 Cursor glob、Claude path rules、工具权限、hooks、MCP transport、用户/团队 UI 规则等方言。
- “项目走 Git、机器走云”是默认建议，不是硬规则。企业托管策略、remote/SSH/container、账号级规则和本机秘密需要分别建模。
- 第二份回复使用 “Loom” 作为英文名不可取。Loom 已是 Atlassian 旗下广泛使用的视频协作产品，搜索、商标和用户心智冲突都很大。
- 第一份回复使用 `ContextOps` 也不宜作为产品名。已有同名 GitHub 治理框架和 PyPI 工具。

## 4. 严格的真值模型

### 4.1 六级状态

每个 context item 在给定解析坐标下应分别产生以下 `ContextStateClaim`，不允许把状态固化在资产实体上，也不允许合并成一个 `active: true`：

1. `Installed`：文件、包、server 或设置存在于某个来源。
2. `Discoverable`：当前工具版本按其规则能发现该项。
3. `Eligible`：当前 cwd、文件集合、任务、glob、描述选择、账号策略等满足激活条件。
4. `ModelVisible`：有运行时或原生诊断证据证明内容/元数据对模型可见。
5. `UseEvidence`：分别记录调用、明确引用和行为一致线索；这些线索不等于模型内部因果使用，内部归因默认 `Unknown`。
6. `OutcomeAffecting`：在受控对照中对质量、成本、时间或错误类型产生可重复差异；默认 `Unknown`。

这些状态不是单向保证。例如已安装的 skill 可能不可发现；可发现的 skill 可能未激活；模型可见的规则可能没有被遵循。

### 4.2 三类核心对象

- `Resolved Context`：版本化解析器依据静态文件、设置和当前坐标推导的预期结果。
- `Observed Context`：原生诊断、日志、受控 wrapper、导入快照或用户明确提供的证据。
- `Context Effect`：同任务、同门禁、控制模型/版本/随机性等混杂因素后的对照结果。

### 4.3 解析坐标

任何 Receipt 或 diff 都必须绑定：

- device/profile；
- operating system 与架构；
- harness 及精确版本；
- surface：desktop、CLI、IDE、cloud、headless 等；
- project root、Git root、worktree、cwd；
- task probe 和被引用文件集合（若可得）；
- account/team policy snapshot（若可得）；
- environment roots，如 `CODEX_HOME`、XDG 路径、容器/SSH home；
- timestamp。

脱离这些坐标讨论“会不会加载”没有确定答案。

### 4.4 证据质量

按可信度从高到低：

1. 原生 runtime/API 明确返回；
2. 原生日志或官方诊断导出；
3. 开源 harness 的实际 resolver 代码和匹配版本；
4. 基于官方文档的静态重建；
5. 启发式或 LLM 推断。

证据不能压成单轴标签：claim kind、lifecycle stage、truth state、provenance、coverage、precision、knowledge status 分别记录。未知值不等于零；字符数不等于 token；会话累计 input tokens 不等于当前上下文占用。

## 5. 本机会话核验

2026-09-04 在当前设备和目录 `.` 执行了只读命令。完整命令与输出已固定在[本机调研环境 Receipt](2026-09-04-local-environment-receipt.md)：

```text
codex --version
=> codex-cli 0.147.0

codex debug --help
=> 包含 prompt-input：Render the model-visible prompt input list as JSON

grok --version
=> grok 1.0.13 (5e9a58528b76)

grok inspect --help
=> Show the configuration Grok discovers for this directory；支持 --json
```

另对一个独立 CLI probe 执行：

```text
codex debug prompt-input 'context inspection probe'
```

只统计结构、不保存正文，得到 5 条 message、41,460 个字符；其中 developer 3 条/23,631 字符，user 2 条/17,829 字符。

这份证据只能说明该独立 Codex CLI probe 的 `prompt.input` 列表，不能代表当前 Codex Desktop 会话，也不能声称是完整 provider wire payload。OpenAI Codex 的公开 issue #35706 明确指出当前命令未返回同一 core Prompt 中的 base instructions 和 request-scoped tool schemas，因此产品必须把该 surface 标成 partial。

## 6. 公开规范事实

以下事实均以 2026-09-04 可访问的官方资料为准，适配器必须记录适用版本并持续回归：

### 6.1 Codex

- Codex 从项目根向 cwd 逐层查找；每层顺序为 `AGENTS.override.md`、`AGENTS.md`、配置的 fallback filenames，每层最多取一份。
- 聚合项目文档受 `project_doc_max_bytes` 限制，默认 32 KiB。
- `CODEX_HOME` 和 fallback 配置会改变发现结果。
- skills、plugins、MCP、desktop/CLI/IDE/cloud 等 surface 不能混为一个适配器结果。

来源：[Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)、[Codex Skills](https://learn.chatgpt.com/docs/build-skills)、[Codex MCP](https://learn.chatgpt.com/docs/extend/mcp)、[App Server](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)、[`prompt-input` 可观察性缺口](https://github.com/openai/codex/issues/35706)。

### 6.2 Claude Code

- `CLAUDE.md` 与 auto memory 都是上下文，不是强制权限边界。
- managed、user、project、local 以及目录层级会影响拼接；祖先文件启动加载，子目录文件可按读取路径延迟加载。
- Claude Code 原生读取 `CLAUDE.md`，不是 `AGENTS.md`；可以在 `CLAUDE.md` 中导入 `@AGENTS.md`。
- `/context` 提供当前会话分类可视化，但仍需记录具体 Claude Code 版本和 surface。
- auto memory 启动读取存在 200 行或 25KB 边界；官方建议每份 `CLAUDE.md` 控制在约 200 行以内，这属于指导，不是普遍的硬截断规则。

来源：[Claude memory](https://code.claude.com/docs/en/memory)、[Claude context window](https://code.claude.com/docs/en/context-window)、[Claude features](https://code.claude.com/docs/en/features-overview)。

### 6.3 Cursor

- `.cursor/rules` 的 frontmatter 组合决定 always、glob 自动附加、agent-selected 和手动 `@` 四种行为。
- Cursor 支持根和嵌套 `AGENTS.md`，更具体目录指令优先。
- Team Rules、User Rules、Project Rules、plugins 及不同 UI surface 需要分别采集；单纯磁盘扫描不能还原团队或用户云端规则。
- 不能因为 Cursor CLI 能读取某类文件，就推断 IDE Chat、Inline Edit、Cloud Agent 等 surface 一致。

来源：[Cursor Rules](https://prod.cursor.com/docs/rules)、[Cursor Plugins](https://prod.cursor.com/docs/plugins)。

### 6.4 Grok Build

- `grok inspect [--json]` 官方定义为展示当前目录发现的 rules、skills、plugins、hooks 与 MCP servers。
- Grok 的兼容读取范围较广，但兼容发现不自动等于跨工具语义无损。
- 必须分别核对 TUI/headless、项目/用户、插件和兼容目录的实际版本行为。

来源：[Grok CLI reference](https://docs.x.ai/build/cli/reference)、[Skills, Plugins & Marketplaces](https://docs.x.ai/build/features/skills-plugins-marketplaces)、[MCP servers](https://docs.x.ai/build/features/mcp-servers)、[grok-build source](https://github.com/xai-org/grok-build)。

### 6.5 跨工具标准

- [AGENTS.md](https://agents.md/) 是广泛采用的开放项目指令格式，但它本身只是 Markdown，无法表达所有 harness 方言和权限。
- [Agent Skills specification](https://agentskills.io/specification) 定义 `SKILL.md`、元数据、可选 scripts/references/assets 和渐进披露；scripts 意味着供应链与执行风险。
- [Agent Plugins specification](https://agent-plugins.org/specification) 已定义 manifest、skills、MCP 与路径 containment；containment 不等于 subprocess sandbox。
- [OpenTelemetry GenAI semantic conventions](https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-agent-spans.md) 已提供 agent/conversation/system instruction 等字段方向，运行时导出应优先兼容而不是自创封闭协议。

## 7. 现有开源项目与可复用边界

没有发现一个项目同时完成“四工具按版本静态解析 + 原生运行时核对 + 可视化证据 Receipt + 多设备 desired state + 安全同步 + 历史因果评估 + 团队治理”。但许多局部能力已经存在，不应重造。

| 项目 | 已覆盖能力 | 可借鉴/集成 | 不直接等同本项目的原因 |
|---|---|---|---|
| [Microsoft APM](https://github.com/microsoft/apm) | manifest、lockfile、跨 agent 安装/编译、依赖、市场、策略、安全、SBOM、drift | 读取 `apm.yml`/lock；复用包解析、来源和安全思想；把 APM 视为 desired-state provider | 核心是 package manager，不是每次任务的 Expected/Observed reconciliation |
| [Agentpack](https://github.com/liqiongyu/agentpack) | Rust、本地 manifest、投影、preview/apply、snapshot/rollback、Git 同步 | 研究安全变更、快照、回滚和跨机器 Git 模式 | 偏部署控制平面，缺少运行时真值和效果证据 |
| [agentsync](https://github.com/spxrogers/agentsync) | canonical source、跨工具 projection、age secrets、dry-run、revert、loss report | 直接借鉴三态 reconcile、secret refs、lossy report；评估作为外部执行器集成 | 重点是同步和投影，不是可视化 Context Receipt |
| [CtxWise](https://github.com/FramY2/ctxwise) | Codex audit、`prompt-input` xray、profiles、redacted lock、drift、exact/estimated/unknown | 可参考测量标签、Codex resolver fixtures、fail-closed 和隐私设计；Apache-2.0 代码需保留 NOTICE | 目前专注 Codex，非四工具统一模型 |
| [Scopeon](https://github.com/sorunokoe/Scopeon) | 多 agent 会话 token/cost/TUI、MCP/skill activity、CI、OTLP | 研究 runtime importer、时间线、成本视图和 OTLP 输出；MIT/Apache 双许可 | 偏运行时和成本，不解析完整的跨工具静态意图/设备 desired state |
| [ctxray](https://github.com/ctxray/ctxray) | 多工具本地会话分析、隐私检查、deterministic lint、CI | 可作为历史会话 importer/规则参考；MIT | 更偏 prompt/session 分析，不解决各 harness 的配置解析与同步 |
| [ContextSpy](https://github.com/RimantasZ/contextspy) | API 代理、请求级上下文 profiler、SQLite、本地 dashboard | 只作为高级、显式授权的 runtime adapter；Apache-2.0 | cloud 模式需安装 CA 并中间代理流量，不能成为默认采集方案 |
| [Ratel](https://github.com/ratel-ai/ratel) / [ratel-local](https://github.com/ratel-ai/ratel-local) | 动态 tool/skill 检索与本地运行 | 研究大规模目录和动态激活观测 | 解决检索/路由，不是上下文资产核对 |
| [SkillDuck](https://github.com/william-zheng-tw/skillduck) | skill 发现/管理方向 | 可研究 skill catalog UX | 不覆盖 rules、MCP、runtime 和设备 drift |
| [Strata](https://github.com/kytheros/strata) | 持久记忆 | 研究 memory provenance 与保留策略 | 记忆系统不是上下文控制平面 |
| [ContextLedger](https://github.com/manthan787/context-ledger) | context ledger 方向 | 仅参考概念 | 调研时未确认清晰 SPDX license，未澄清前不得复制代码 |
| [yelmuratoff/agent_sync](https://github.com/yelmuratoff/agent_sync) | 配置同步 | 可比较交互 | GPL-3.0，不嵌入拟采用宽松许可的核心 |

### 7.1 明确的 build / borrow / integrate 决策

应自己构建：

- 统一的 Context IR、解析坐标、Evidence Ledger 和 Context Receipt；
- 按 harness + version + surface 的 resolver adapter 与 conformance corpus；
- Expected/Observed/Unknown reconciliation；
- 跨工具、跨设备、跨版本的证据化 diff；
- 主可视化和“Why is this here?”、“How do you know?”解释链；
- 受控 context effect 实验编排与混杂因素记录。

应优先借鉴或集成：

- 包、manifest、lock、SBOM、marketplace：APM；
- desired-state deployment、preview、rollback：Agentpack/agentsync；
- Codex 专项解析与测量：CtxWise；
- 会话成本、OTLP 和 runtime timeline：Scopeon；
- 会话历史本地分析：ctxray；
- 标准格式：AGENTS.md、Agent Skills、Agent Plugins、MCP、OpenTelemetry。

禁止直接复制：

- 无许可证或许可证不兼容的代码；
- 仅凭 README 宣称就当作已验证实现的功能；
- 未保留 attribution/NOTICE 的 Apache-2.0 代码；
- 代理安装 CA、读取秘密或上传正文的默认行为。

## 8. 研究证据对产品价值的约束

公开研究并不支持“上下文文件越多越好”或“只要管理好就必然提高正确率”的营销口径：

- 一项覆盖 10 个仓库、124 个 PR 的研究观察到加入 `AGENTS.md` 后中位运行时间下降 28.64%、输出 token 下降 16.58%，任务完成行为相当。[论文](https://arxiv.org/abs/2601.20404)
- 另一项 2 个 agent、3 个仓库、17 个任务、288 次运行的消融没有测到正确率显著变化，并把差异界定在约 10–15 个百分点内。[论文](https://arxiv.org/abs/2607.27250)
- 对 100 个热门开源仓库的研究发现配置 smell 普遍存在：Lint Leakage 62%、Context Bloat 42%、Skill Leakage 35%。[论文](https://arxiv.org/abs/2606.15828)
- 对 2,853 个仓库的研究识别了八类 harness configuration mechanism，发现 context files 占主导、AGENTS.md 正在形成互操作标准，而 Skills/Subagents 等高级机制采用仍浅。[论文](https://arxiv.org/abs/2602.14690)

因此产品价值应表述为：提高可见性、可复现性、诊断速度、安全性和受控优化能力；不能提前承诺普遍提升代码正确率。效果结论必须由用户自己的可比任务和相同质量门禁产生。

## 9. 潜在用户与使用节奏初步结论

### 9.1 潜在用户

1. 同时使用 2–4 个 coding agent、维护大量 skills/MCP、经常切换设备的个人开发者。
2. 在多个客户仓库间切换的自由职业者、顾问和 agency 工程师。
3. AI-native 小团队的 tech lead、开发体验/平台负责人。
4. 大型组织的 Developer Productivity、AI Platform、Security、Compliance 团队。
5. skills、plugins、MCP、agent harness 的作者和维护者。
6. 开源项目维护者，需要让外部贡献者使用不同 agent 仍遵循项目约定。
7. 研究、评测和模型平台团队，需要运行可复现的 context ablation。
8. 受监管或高保密组织，需要证明“哪些上下文到达了哪个 surface”，同时默认不上传正文。

### 9.2 使用节奏不是不同产品，而是同一数据模型的不同入口

- 长期开启：可选 local daemon 监听文件、工具版本、MCP/skill surface、会话元数据和 drift；提供安静的事件式告警。
- 每次任务前：CLI/编辑器 preflight 根据项目、cwd、工具和任务生成 Receipt，确认本轮“模型将看到什么/仍看不到什么”。
- 偶尔使用：工具升级、换机、换仓库、行为异常、MCP 失效时做一次诊断。
- 定期使用：每日/每周 drift scan、月度 context debt 清理、季度策略与供应链审计。
- 事件触发：配置 PR、harness 升级、安装 skill/plugin/MCP、团队 policy 变化时在 CI 检查。
- 一次性迁移：新设备 bootstrap、agent 切换、项目 onboarding、团队标准化。
- 评测活动：在一组任务上做 context A/B，对质量、时间、token、成本和失败类型生成可重复报告。

完整的用户回报应包括：诊断时间下降、意外行为减少、设备/团队复现性提升、上下文和成本浪费减少、安全与审计证据增强，以及“我知道模型为什么看到这些”的控制感。详细 persona、工作流和回报模型在完整需求文档中定义。

## 10. 关键缺口与必须进入需求的边界条件

- Adapter version drift：厂商规则随版本变化，必须支持版本探测、fixture、golden result 和未知版本 fail-closed。
- Surface drift：Codex Desktop/CLI/IDE/cloud、Cursor Chat/Inline/Cloud、Claude local/cloud、Grok TUI/headless 分开建模。
- 文件系统差异：symlink、hard link、case sensitivity、Git worktree、nested repo、Git ignore、XDG、自定义 home、网络盘。
- 执行环境差异：SSH、Dev Container、Docker、remote workspace、CI、sandbox、云端 agent；设备不等于 execution environment。
- 动态上下文：compaction、tool search、subagent、memory、`@` 文件、会话 steering、MCP 动态 schema 和 tool result。
- 云端/账号规则：Team/User managed settings 未必存在于本地文件，需导入/API/人工声明并标证据质量。
- 权限与指令不同：行为指令不能作为 sandbox、deny policy 或审批机制的替代。
- 安全：隐藏 Unicode、prompt injection、恶意 scripts/hooks、MCP 命令、symlink escape、来源、签名、SBOM、依赖传递信任。
- 隐私：会话正文、源码、prompt、tool result 的最小采集、分级保留、加密、删除和脱敏。
- 修改安全：preview、显式选择、原子写入、幂等、保留注释、并发冲突、备份与 rollback。
- 因果评估：模型版本、harness 版本、任务难度、随机性、缓存、网络和门禁都可能混淆结果。
- 组织治理：owner、policy scope、例外、审批、audit 与内容读取权限必须分离。
- Token 与成本：只有原生 tokenizer/runtime 返回时才称 exact；否则标 estimated；价格必须带日期和 provider。

## 11. 明确不做与已否决路线

以下不是“晚点再说的功能”，而是产品契约中的负面边界：

- 不承诺提取厂商隐藏 system prompt 或还原无法观察的 provider wire payload。
- 不把 discoverable、eligible、model-visible、used、outcome-affecting 合并成一个“已生效”。
- 不输出无校准的语义百分比、加载概率或“价值分”。
- 不把 LLM 评分当事实，不在无历史/无对照证据时断言某规则有用或可删除。
- 不默认安装 root CA、MITM 代理或读取所有请求正文。
- 不默认把仓库内容、会话正文、Prompt 或秘密上传云端。
- 不同步明文 secret；同步配置只保存 secret reference 和来源。
- 不把未知值显示为零，不把字符数显示为精确 token。
- 不默认自动修改真实配置；所有 mutation 必须 preview、解释影响、可回滚。
- 不静默做有损格式转换。
- 不再造通用 package manager、marketplace、lockfile 和依赖解析器；优先集成 APM 等现有项目。
- 不强迫所有工具改用专有 universal config，也不以专有 manifest 取代原生文件。
- 不以图数据库作为产品成立条件。
- 不把 context path containment 宣称成执行 sandbox。
- 不混合不同工具 surface 的结论。
- 不把 LLM suggestion 写入 ContextStateClaim、policy、baseline 或 CI 真值。
- 不让 Contexpect 与 APM 对同一个 package/source/install policy check 各自判一次；package 域由 APM 唯一负责。
- 不允许 required projection cell 退化为 export-only；同时也不承诺改写厂商没有稳定外部写入面的 UI-only 设置。
- 不为文件夹同步自建版本图、CRDT 或通用三方 merge；Git 场景使用 Git，非 Git 分叉要求选择或外部处理。
- 不把“建完整产品”解释为“所有未来可能性无限纳入”。完整范围以需求文档中的验收合同为界。

## 12. 命名结论

推荐项目名：`Contexpect`；CLI：`ctxpect`；核心导出物：`Context Receipt`；英文主张：`Expected. Observed. Reconciled.`

命名理由：

- 组合 `context + inspect/expect`，直接表达“预期上下文与实际观察的核对”。
- `ctxpect` 适合作为短 CLI 命令。
- 比 `ContextView` 更强调核对而非单纯查看，也比 `ContextOps` 更少治理/运维歧义。
- 2026-09-04 的 GitHub 精确名称检索未发现 `contexpect` 或 `ctxpect` 同名仓库；这只是初筛，不是商标、域名或软件包注册结论。

不推荐：

- `织境 Loom`：中文“织境”意象很好，但 `Loom` 与 Atlassian Loom 强冲突，难以搜索和注册。
- `ContextOps`：已有同名 GitHub framework 和 PyPI package。
- `ContextView`：可作临时目录名，但产品差异点不是“查看”，而是证据化 reconciliation。
- `PromptStack`、`ContextHub`、`RuleManager`：容易被理解为模板库、门户或后台 CRUD。

中文可以把产品描述为“上下文鉴证台”或“上下文核对台”，暂不强行绑定另一个品牌名，避免中英文双品牌增加认知成本。

## 13. 商业与护城河判断

- 个人、本地、可观察核心适合开源，建议 Apache-2.0，便于企业采纳且提供专利授权；最终仍需在正式建仓前确认依赖许可矩阵。
- 付费价值主要在团队/企业：私有 adapter、fleet/device/environment compliance、组织 policy、RBAC、审计、E2EE sync、托管控制面和支持。
- 手写 resolver 本身是早期负债，不是长期护城河。真正护城河是跨版本 compatibility corpus、原生验证 Receipt、失败样本、用户信任和第三方 adapter 生态。
- Context Graph 是重要 UI，但不是护城河。
- 历史效果数据潜力大，但受隐私和混杂因素制约，不能在缺乏严谨 eval 时宣传为智能护城河。

## 14. 进入完整需求的已决策项

- 完整产品一次定义、一次集成验收，不用中间子集代替最终产品范围。
- 同时覆盖 Codex、Claude Code、Cursor、Grok Build 的主要本地 coding surface；adapter 架构允许扩展，但首个完整交付不宣称支持所有 agent。
- 产品形态同时包含视觉桌面/本地 Web UI、CLI、可选 daemon、CI 和导入导出；它们共享同一个 core 和数据库。
- local-first、metadata-first、private-by-default；云同步可选、端到端加密、secret reference only。
- 项目级 Git 是优先 desired-state 来源，但不是唯一来源。
- SQLite 作为本地存储；Context Graph 通过关系数据生成。
- deterministic Doctor 与 LLM Advisor 分开；前者可作为门禁，后者只输出带证据的建议。
- package/marketplace/部署优先集成现有开放项目，不重建生态基础设施。
- 支持持续、任务前、偶发、定期、事件触发、迁移和评测七类使用方式。
- resolution coordinate 固定包含 account、organization 与 policy snapshot，避免同设备同项目下跨账号误合并。
- 使用 16 类 context capability taxonomy；动态面不可观察时必须明确 Unknown。
- 核心 native instructions/skills/MCP projection 有逐 harness/scope 的 required-write 下限与全矩阵门禁。
- policy 按 domain 绑定唯一 authority；APM 负责 package/source/install，Contexpect 只负责 context-specific evidence/privacy/egress/Unknown。
- corpus 分 development、sealed conformance、live native-oracle，避免只对公开 fixture 过拟合。
- metadata-only 不创建 vault key；敏感正文 vault 具有明确 create/unlock/auto-lock/rotation/backup/loss lifecycle。

## 15. 仍需在实现合同中精确化的事项

这些不阻塞需求生成，但实施前必须在技术设计中做出可判定选择：

- macOS/Linux/Windows 的首个正式支持矩阵和测试设备；
- Cursor/Grok 是否存在足够稳定的 runtime export API，哪些 surface 只能标 `Unknown`；
- age/SOPS/OS keystore/external secret manager 中最终选用的 recipient provider 与平台集成细节；产品不自建恢复码或设备 PKI；
- 组织管理员是否能看正文，默认建议只能看 hash、metadata 和 policy 状态；
- 历史会话 importer 的合法来源、保留策略和用户删除语义；
- LLM Advisor 的本地模型/远端模型选择及数据发送确认；
- APM/Agentpack/agentsync 是 CLI integration、library reuse 还是只读 import；
- 品牌、域名、npm/crates/PyPI 和商标正式清查。

## 16. 来源索引

官方与标准：

- [Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)
- [Codex Skills](https://learn.chatgpt.com/docs/build-skills)
- [Codex MCP](https://learn.chatgpt.com/docs/extend/mcp)
- [Codex App Server](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
- [Claude Code features](https://code.claude.com/docs/en/features-overview)
- [Claude Code memory](https://code.claude.com/docs/en/memory)
- [Claude Code context](https://code.claude.com/docs/en/context-window)
- [Cursor Rules](https://prod.cursor.com/docs/rules)
- [Cursor Plugins](https://prod.cursor.com/docs/plugins)
- [Grok Build overview](https://docs.x.ai/build/overview)
- [Grok CLI reference](https://docs.x.ai/build/cli/reference)
- [Grok Skills/Plugins/Marketplaces](https://docs.x.ai/build/features/skills-plugins-marketplaces)
- [Grok MCP](https://docs.x.ai/build/features/mcp-servers)
- [AGENTS.md](https://agents.md/)
- [Agent Skills specification](https://agentskills.io/specification)
- [Agent Plugins specification](https://agent-plugins.org/specification)
- [OpenTelemetry GenAI semantic conventions](https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-agent-spans.md)

开源项目与研究：

- [Microsoft APM](https://github.com/microsoft/apm)
- [CtxWise](https://github.com/FramY2/ctxwise)
- [Scopeon](https://github.com/sorunokoe/Scopeon)
- [ctxray](https://github.com/ctxray/ctxray)
- [ContextSpy](https://github.com/RimantasZ/contextspy)
- [Agentpack](https://github.com/liqiongyu/agentpack)
- [agentsync](https://github.com/spxrogers/agentsync)
- [AGENTS.md efficiency study](https://arxiv.org/abs/2601.20404)
- [Context file ablation study](https://arxiv.org/abs/2607.27250)
- [Configuration smells study](https://arxiv.org/abs/2606.15828)
- [Harness engineering study](https://arxiv.org/abs/2602.14690)

命名冲突：

- [Atlassian Loom](https://www.atlassian.com/software/loom)
- [ContextOps GitHub](https://github.com/kannanokannan/ContextOps)
- [ContextOps PyPI](https://pypi.org/project/contextops/)
