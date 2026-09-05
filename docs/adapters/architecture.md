# Adapter 架构

> 状态：规范（尚未实施产品运行时）

Adapter 把某个 `family × version × surface × OS` 的声明资产和原生证据翻译成 Context IR。UI 不理解 18 种方言。

## 支持口径

“支持某 harness”意味着：

1. 已覆盖功能有版本和 surface 声明
2. 未覆盖处显示 Unknown
3. 有 conformance fixtures，并在声明了可重复 oracle 时有 oracle 样本
4. 不意味着能提取隐藏 system prompt 或全部 provider-wire 字段

首屏可汇总为 Native evidence / Static resolution only / Needs connector / Unsupported version，但 canonical 状态仍在 capability matrix。

## 18 个 family

| id | 名称 | cohort | 本机 live（macOS 27.0 26A5425a） |
| --- | --- | --- | --- |
| codex | Codex | anchor | CLI 0.147.0 installed；desktop/cloud unknown |
| claude-code | Claude Code | anchor | CLI 2.1.259 installed |
| cursor | Cursor | anchor | IDE 3.19.7 与 Agent CLI `2026.08.25-3e8eec8` installed |
| grok-build | Grok Build | anchor | CLI 1.0.13 installed |
| opencode | OpenCode | expansion-local | 1.18.21 installed，曾出现 auth 401 |
| kimi-code | Kimi Code | expansion-local | 0.40.1 installed |
| zcode | ZCode | expansion-local | App 3.10.2 installed；无 standalone CLI |
| qwen-code | Qwen Code | expansion-local | 0.18.0 installed |
| goose | Goose | expansion-local | 1.37.0 installed |
| gemini-cli | Gemini CLI | expansion-local | 0.55.1 installed，账号/client 曾被拒绝 |
| github-copilot-cli | GitHub Copilot CLI | expansion-local | 1.0.82 installed |
| kiro | Kiro | expansion-local | App 2.9.0，bundle 内有 executable，PATH 无 `kiro` |
| deepseek-harness | DeepSeek Harness | expansion-hermetic | **not-installed**（仅 config residue） |
| cline | Cline | expansion-hermetic | not-installed |
| aider | Aider | expansion-hermetic | not-installed |
| openhands | OpenHands | expansion-hermetic | not-installed |
| windsurf | Windsurf | expansion-hermetic | not-installed |
| coze | Coze | connector | **connector-required**（recent-item 痕迹不是安装） |

Ubuntu/Windows 同版本 live 未捕获，坐标为 `required-unknown-honesty` 或在无官方 App 证据时 `not-applicable`。禁止编造 ISO digest。

旧 Kimi CLI 1.49.0 只进入 legacy import lane，不占 18 family 硬门禁。

## 可重复 native oracle（本冻结）

只有下列 surface 被声明为可重复机器 oracle，因此必须各有 ≥12 个 recipe 级 oracle case：

- Codex CLI 0.147.0 `codex debug prompt-input`（partial；不含完整 core prompt）
- Grok CLI 1.0.13 `grok inspect --json`（discovered configuration 默认只证明 discoverable）

Claude `/context`、Copilot `plugins list`、Cursor 官方导出等可以在后续 contract revision 中升级；本阶段不得假装已经有可重复 JSON oracle。

## Manifest

每个 adapter 发布：

- `id`、`version_range`、`surfaces`、`os`
- collectors / resolvers / importers
- capability cells 与 field-to-claim mapping
- 来源 URL、访问时间、允许保存的 digest、license
- fixtures、负例、未知版本测试

未知版本：权威 Receipt 中相关 claim 为 indeterminate；探索预览必须标 `unverified preview`。

CanonicalIntent 投影必须使用该 family 的 native path/syntax/scope/precedence，不得把一份通用 MD 复制到所有 adapter。无独立 primitive 时必须显示 loss 或 Unknown。投影结果不得写成核对状态 `verified`。详见 [semantic-alignment-and-team-standard](../architecture/semantic-alignment-and-team-standard.md)。

## 运行时隔离

见 [ADR 0004](../adr/0004-adapter-isolation.md)。第三方 adapter 默认进程隔离或 WASI，无网络、无 secret、无整个 home。网络访问走同一 egress 策略。

## 更新

发现新版本 → 比较 capability → 跑 static/negative/oracle → 一致则扩大 verified range，否则新增版本分支。更新包必须验证签名和 pinned digest。被撤销版本停止产生权威 claim。

## Doctor 设计 fixture 对照

`10-context-doctor-final.png` 使用 4/9/5 分组（DeepSeek 出现在 Native evidence）。那是 OctoWorkflow 演示数据。Live compatibility matrix 不得照抄该分组。
