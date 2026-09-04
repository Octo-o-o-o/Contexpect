# Contexpect Agent / Harness 扩展调研

> 日期：2026-09-04  
> 状态：已完成本机探测、官方资料核对与支持分层；星标数仅是当日热度快照，不是产品质量结论。  
> 目的：决定哪些 agent 应进入 Contexpect 的适配器机制，以及“支持”应承诺到什么证据层级。

## 1. 结论

Contexpect 不应停在 Codex、Claude Code、Cursor、Grok Build 四个名称，也不应承诺“所有工具都能看到完整最终 prompt”。完整产品应维护 **18 个 adapter family**，并让每个 adapter 按版本、surface 和证据能力分别声明：

1. `native-evidence`：能导入厂商/开源 harness 暴露的运行时事件、诊断或 session evidence；
2. `static-resolution-only`：能可靠解析本地规则、scope、precedence 和配置，但没有足够原生证据证明模型实际收到；
3. `connector-required`：产品知道其资产与集成语义，但当前设备未安装、未授权或没有可用采集 surface；
4. `unsupported-version`：已安装，但版本尚未通过 conformance；必须显示 Unknown，不能猜。

这四类不是产品阶段，也不是高低级套餐；它们是同一完整产品里的真实性状态。一个 adapter 在静态面可以是 `required-supported`，在模型可见性面同时是 `required-unknown-honesty`。

## 2. 本机实测清单

探测只读取 executable path、app bundle version 和配置根目录是否存在；没有读取 token、credential 或敏感配置正文。

| Family / Surface | 本机证据 | 版本 / 状态 | 结论 |
|---|---|---:|---|
| Codex CLI | `~/.local/bin/codex` | `0.147.0` | 已安装，可做 anchor |
| Claude Code | `/opt/homebrew/bin/claude` | `2.1.259` | 已安装，可做 anchor |
| Cursor IDE / Agent CLI | `/Applications/Cursor.app`、`~/.local/bin/cursor-agent` | `3.19.7`、`2026.08.25-3e8eec8` | 已安装，IDE 与 CLI 必须分 surface |
| Grok Build CLI | `~/.grok/bin/grok` | `1.0.13` | 已安装；本轮只读模拟被本机 Docker socket sandbox 前置检查阻断 |
| OpenCode | `/opt/homebrew/bin/opencode` | `1.18.21` | 已安装；本轮默认模型配置失效并返回 401 |
| DeepSeek Harness | `~/.dsh` 存在；当前 PATH 无 `dsh` | executable 未找到 | 不能把配置残留误报成可运行安装 |
| Kimi Code | `~/.kimi-code/bin/kimi` | `0.40.1` | 已安装；可读图模拟成功 |
| Kimi CLI（旧入口） | `~/.local/bin/kimi-cli` | `1.49.0` | 与 Kimi Code migration 分开记录 |
| ZCode | `/Applications/ZCode.app`、`~/.zcode` | `3.10.2` | App 已安装，未发现 standalone CLI |
| Qwen Code | `/opt/homebrew/bin/qwen` | `0.18.0` | 已安装；文本审查成功，当前 CLI 没有把 PNG交给模型 |
| Goose | `/opt/homebrew/bin/goose` | `1.37.0` | 已安装 |
| Gemini CLI | `/opt/homebrew/bin/gemini` | `0.55.1` | 已安装；本轮账号/客户端组合被服务拒绝，不能据此判定产品整体废弃 |
| GitHub Copilot CLI | `/opt/homebrew/bin/copilot` | `1.0.82` | 已安装；读图模拟成功 |
| Kiro CLI | `/Applications/Kiro CLI.app` | `2.9.0` | App bundle 内含 `kiro-cli` 等 executable，未加入 PATH |
| Coze | 未发现 executable、App bundle 或全局 npm package | 有历史 Recent Documents 痕迹 | 当前只能标 `connector-required`；不能声称已安装可运行 |
| Cline / Aider / OpenHands / Windsurf | 本机未发现 | 未安装 | 纳入机制与 fixtures，不伪装成本机验证 |

补充说明：用户提到本机有 Coze；本轮 PATH、`/Applications`、全局 npm 和常见目录探测未定位到有效安装，只找到 `cn.coze.desktop` 的历史 recent-item 痕迹。这是一个非常典型的 Doctor finding：**“用户认为存在”与“当前 coordinate 可发现、可运行”必须分开显示。**

## 3. 为什么扩到这 18 个

### 3.1 开源热度快照

截至 2026-09-04，GitHub 页面显示：DeepSeek Harness 约 211.2k stars、OpenCode 约 203.6k、OpenHands 约 86.1k、Cline 约 67.4k、Goose 约 53.9k、Aider 约 48.7k、Qwen Code 约 27.6k、Kimi CLI 约 11.3k。数值会变化，只用于决定调研优先级，不进入 adapter 的真实性判定。

- [DeepSeek Harness](https://github.com/deepseek-ai/DeepSeek-Harness)
- [OpenCode](https://github.com/anomalyco/opencode)
- [OpenHands](https://github.com/OpenHands/OpenHands)
- [Cline](https://github.com/cline/cline)
- [Goose](https://github.com/aaif-goose/goose)
- [Aider](https://github.com/Aider-AI/aider)
- [Qwen Code](https://github.com/QwenLM/qwen-code)
- [Kimi CLI](https://github.com/MoonshotAI/kimi-cli)

商业 IDE/CLI 无法用 stars 横比，因此另外按本机存在、官方 context surface 丰富度和跨用户影响纳入：Codex、Claude Code、Cursor、Grok Build、GitHub Copilot CLI、Gemini CLI、Kiro、Windsurf、ZCode 和 Coze。

### 3.2 Context surface 足够复杂

这些工具不仅有一个 Markdown 文件。官方资料显示，它们普遍组合了 instructions/rules、skills、MCP、hooks、custom agents、memory、extensions/plugins 或 cloud scope：

- GitHub Copilot CLI 同时合并 `AGENTS.md`、`CLAUDE.md`、`GEMINI.md`、`.github/instructions`，并有 skills、MCP、plugins、hooks 与 custom agents；`copilot plugins list` 还是一个很有价值的原生 inventory surface。[官方 CLI reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference)
- Qwen Code 有 user/project skills、MCP、hooks、memory、subagents、extensions 和 daemon/SDK surface。[Skills](https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/skills.md) · [MCP](https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/mcp.md)
- Kiro 在 IDE/CLI/Web/Mobile 间共享 harness，但 local/global/cloud steering 并不等价；custom agent 又需要显式把 steering 加到 resources。[Steering](https://kiro.dev/docs/steering/) · [Configuration scopes](https://kiro.dev/docs/cli/chat/configuration/)
- Windsurf 同时存在 Rules、按目录作用的 `AGENTS.md`、Workflows、Skills 和自动生成的本机 Memories；其 activation mode 和设备边界正是 Contexpect 要解释的对象。[Windsurf Memories & Rules](https://docs.windsurf.com/zh/windsurf/cascade/memories)
- ZCode 同时追加 global/workspace `AGENTS.md`，支持 skills 的外部导入和 `.agents/mcp.json`；还明确区分本地与远程 workspace 的 skill 同步。[ZCode Agent](https://zcode.z.ai/en/docs/agents) · [ZCode Skills](https://zcode.z.ai/en/docs/skill) · [ZCode MCP](https://zcode.z.ai/en/docs/mcp-services)
- OpenHands 明确区分 always-on `AGENTS.md` 与按需 `SKILL.md`，并公开 skill injection behavior；这为 Expected/Observed 对账提供了很好的 reference implementation。[OpenHands Skills](https://docs.openhands.dev/overview/skills) · [SDK context injection](https://docs.openhands.dev/sdk/guides/skill)
- Cline 的 `.clinerules`、skills、MCP、CLI JSON/headless 和多 surface 共享规则，适合做静态与运行时双层 adapter。[Cline repository](https://github.com/cline/cline)
- Aider 的 home/repo/cwd `.aider.conf.yml` 有明确加载顺序，`read:` conventions 与 repo map 形成不同 context source；它不应被硬套成 SKILL.md 型产品。[Aider config](https://github.com/Aider-AI/aider/blob/main/aider/website/docs/config/aider_conf.md) · [Conventions](https://aider.chat/docs/usage/conventions.html)
- Goose 通过 MCP extensions 与 ACP 连接 provider/agent，且存在 `.goosehints` 等本地约定；适合作为协议型 harness adapter。[Goose](https://github.com/aaif-goose/goose)
- Gemini CLI 仍是 Google 官方文档中的开源本地 agent，并支持本地/远程 MCP；本机一次认证失败不能把它降为 legacy。[Google Cloud Gemini CLI](https://docs.cloud.google.com/gemini/docs/codeassist/gemini-cli)
- Coze CLI 主要是让 Codex、Claude、TRAE 等 agent 调用 Coze 的项目/资源/生成能力；它更像 context-bearing external executor/connector，而不是同构 coding harness。[Coze CLI](https://docs.coze.cn/developer_guides_coze_cli) · [npm package](https://www.npmjs.com/package/%40coze/cli)

## 4. 18 个 adapter family 的交付口径

| Family | 必须解析的主要资产 | 优先原生 / 半原生证据 | 完整交付状态 |
|---|---|---|---|
| Codex | AGENTS chain、skills/plugins/MCP、config/profile | prompt-input、app-server、JSON events 的声明字段 | anchor `required-supported` |
| Claude Code | CLAUDE chain、rules/imports、skills/MCP/hooks/memory | `/context`、config diagnostics、允许导入的 session records | anchor `required-supported` |
| Cursor | project rules、AGENTS、skills/MCP、可见 settings | official active/export/log surface | anchor；缺口 `required-unknown-honesty` |
| Grok Build | rules、skills/plugins/hooks/MCP、compat dirs | `inspect --json`、streaming events | anchor `required-supported` |
| OpenCode | AGENTS/rules、agents、skills、commands、MCP、config | JSON run/events 与可用 session exports | expansion `required-supported` |
| DeepSeek Harness | plugins、AGENTS/CLAUDE、session config、tool surfaces | append-only session/event log、plugin graph | expansion，高优先级 native evidence |
| Kimi Code | AGENTS、skills、MCP、hooks/subagents、flow | stream/session exports、ACP surface | expansion `required-supported` |
| ZCode | global/workspace AGENTS、skills、commands、MCP、plugins/hooks | App task/export surface；不可得处 Unknown | expansion `required-supported` |
| Qwen Code | QWEN/AGENTS、rules、skills、MCP、hooks、agents、memory/extensions | headless JSON、daemon/SDK event | expansion，高优先级 native evidence |
| Goose | hints、recipes、extensions/MCP、provider/ACP config | diagnostics/session export/API | expansion `required-supported` |
| Gemini CLI | GEMINI、settings、skills/extensions、MCP、memory | headless JSON/session/diagnostics | expansion `required-supported` |
| GitHub Copilot CLI | merged instructions、skills、agents、MCP、plugins/hooks | `plugins list`、MCP JSON、CLI event/result | expansion，高优先级 native inventory |
| Kiro | steering、AGENTS、skills、hooks、agents、MCP/powers/specs | CLI headless/ACP、hook/session evidence | expansion `required-supported` |
| Cline | `.clinerules`、skills、MCP/plugins、agents | CLI `--json`、SDK/session surface | expansion `required-supported` |
| Aider | `.aider.conf.yml`、`read` conventions、repo map、model config | chat history/analytics 中明确字段 | expansion；不要假装支持 skills |
| OpenHands | AGENTS、skills、agent config、setup/hooks | Agent Server/SDK event 与 prompt injection record | expansion，高优先级 native evidence |
| Windsurf | global/workspace/system rules、AGENTS、workflows、skills、memories、MCP | IDE 官方 export/log surface；否则 Unknown | expansion `required-supported` |
| Coze | `.cozerc.json`/global config、org/space/project coordinate、bundled skill target | CLI schema/status/JSON 与 Coze API receipt | ecosystem connector，不参与完整 prompt parity |

`required-supported` 的含义是“对声明的 cell 有 resolver/collector、fixtures 和明确输出”，不是每个 cell 都必须为 Observed。产品完整性通过 Unknown honesty 保持，而不是靠伪造覆盖率。

## 5. 实施优先顺序（不是 MVP）

这不是删功能或分试验版，而是一次完整交付内部的工程依赖顺序：

1. 先稳定 Adapter SDK、manifest、fixture runner、evidence vocabulary 与 unknown reason code；
2. 复用四个 anchor 验证核心 resolver/receipt；
3. 接入本机可跑且 machine-readable 较强的 OpenCode、Kimi Code、Qwen Code、Copilot CLI、Kiro；
4. 接入 DSH/OpenHands/Cline，因为它们的 event/SDK surface 对 native evidence 很有价值；
5. 接入 ZCode、Goose、Gemini、Windsurf、Aider 的静态和可用诊断面；
6. 将 Coze 作为 connector/executor adapter，而非伪装成同一 prompt pipeline；
7. 对未安装的 family 仍运行 hermetic fixtures；只有 live lane 标 `not-installed`。

## 6. 明确不做与避免踩坑

- 不按 stars 自动给予更高证据等级；stars 只影响 adapter 优先级。
- 不把 config 目录存在等同于 executable 可运行；本机 DSH 已证明这是错误的。
- 不把一次 CLI 登录/模型配置失败等同于产品废弃；OpenCode、Gemini 本轮均证明需要把 install/config/auth/runtime health 分开。
- 不把 Coze Studio 的 prompt/RAG/workflow 资产硬塞进 coding-agent prompt schema；通过 connector capability map 表达。
- 不用一个 `supported: true` 布尔值吞掉 IDE/CLI/cloud、global/project/task、static/runtime 的差异。
- 不为 18 个工具复制 18 套 UI；UI 消费统一 Receipt，差异留在 adapter manifest 和 evidence chain。
- Continue、旧 Roo Code、旧 Kimi CLI 等迁移对象只做 legacy import/migration lane；除非重新进入当前使用与维护范围，不占 active conformance 的硬门禁。

## 7. 对产品设计的直接影响

- Doctor 首屏必须显示“原生证据 / 仅静态解析 / 需要连接器”，不能只显示 agent 在线/离线。
- 顶部不能写模糊的“覆盖 13 个工具”，应显示 adapter 总数和证据构成。
- Standards 页面需要按 `family × version × surface × capability` 展开，而不是一行一个 logo。
- Checkup 必须把 install、discoverable、eligible、model-visible、used、effect 六个 facet 分开。
- 连接器失败本身就是 finding：missing executable、stale model、auth rejected、sandbox unavailable、image attachment unavailable 都应形成证据，不应被普通 toast 吞掉。

