# Contexpect Doctor Design Brief

## Product Context

- Project: Contexpect
- Mode: new product / high-fidelity architecture exploration
- Target platform: local-first desktop app，1440×900 主视口；CLI/CI 是同一 core 的辅助入口
- Target users: 多工具开发者、单工具复杂仓库用户、团队 DevEx、agent/skill/MCP 作者

## Goals

- 把抽象的 context observability 转译成“体检 → 问诊 → 治疗方案 → 复查 → 持续关注”。
- 让首屏同时回答：现在有什么问题、证据强度如何、影响哪些 agent、下一步能安全做什么。
- 不用单一健康分；分别呈现 Confirmed、Suspected、Unknown 和不同健康维度。
- 让跨工具、跨设备与 runtime evidence 成为产品差异，而不是普通 Markdown linter。

## Non-Goals

- 不做营销页、医疗插画或游戏化健康分。
- 不在图片里假装所有工具都有完整 Observed API。
- 不把 Doctor 建议自动写入配置；诊断与 Treatment 授权分开。

## Page Inventory

- Context Checkup / Overview
- Context Doctor / Diagnosis（本轮 canonical screen）
- Care Plan / Preview / Treatment
- Recheck / Context Receipt
- Monitor / Drift timeline
- Lab / Effect experiment
- Assets / Integrations / Policy

## Constraints

- 真实产品数据来自现有 PRD；Expected、Observed、Unknown 必须视觉区分。
- Agent 机制覆盖 18 个 family：Codex、Claude Code、Cursor、Grok Build、OpenCode、DeepSeek Harness、Kimi Code、ZCode、Gemini CLI、Qwen Code、Goose、GitHub Copilot CLI、Kiro、Coze、Cline、Aider、OpenHands、Windsurf。
- 首屏按 `native-evidence / static-resolution-only / connector-required` 分组；这表示当前 coordinate 的证据能力，不是工具质量排名。
- 生成图片中的小字可能失真，精确 copy、token 和布局以 Markdown 规范为准。
- 所有修改动作必须有 preview、authorization、rollback、post-change recheck。

## Style Policy

- Preserve current UI spec unless explicitly changing style: 当前没有既有 UI，因此建立新的“calm clinical observability”视觉语言；强调证据、安静、精密，不使用医疗拟物。

## Adjustment Level

- Level: Level 4 architecture refactor
- Rationale: 空项目，需要同时确定产品心智、导航、诊断流程和跨 agent 信息架构。

## Assumptions And Open Questions

- 桌面优先；移动端只用于通知和 Receipt 查看，不在本轮绘制。
- `Contexpect` 是品牌，`Doctor` 是默认主入口；避免直接使用已被多个项目占用的 `Context Doctor` 作为独立品牌。
- 以 Doctor cockpit 验证信息架构；完成三轮架构迭代，再根据本机 agent 模拟与互联网范围调研做不改变核心信息架构的验证校正。
