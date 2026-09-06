# Current State

## Inputs

- User screenshots: 无，项目为空。
- Browser screenshots: 无，尚未实施。
- Reference materials: 完整 PRD、研究台账、三轮需求评审记录；本机 agent inventory 与互联网调研。

## Repository Findings

- Framework: 技术栈已由 ADR 0001 冻结为 Rust + Tauri 2 + React/TypeScript + SQLite/FTS5，不重新选型。本包实现的桌面真值路径是 localhost API + React；Tauri 窗口不加入 Cargo workspace，以免离线 cargo 门禁拉取第三方 crate。产品运行时尚未完成全部 OS WebView 验收。
- Routes: V01–V16 全部可达（见 `docs/guides/desktop-ui.md` 与 `packages/ui/src/routes.ts`）。
- Theme/CSS: 尚无。
- Component system: 尚无；设计必须可用 headless primitives + CSS tokens 实现。
- Mock/seed data: 本轮定义 `OctoWorkflow` 项目，MacBook Pro 与 Workstation 两设备，18 个 agent adapter family，4 条诊断 finding。

## UI Problems

| Severity | Area | Problem | Evidence |
| --- | --- | --- | --- |
| High | Product entry | 原 PRD 能力完整，但用户需要先理解 context lifecycle 才知道从哪里开始 | 用户提出 Doctor/体检心智 |
| High | Truth | 容易把“发现文件”误画成“模型实际看见” | PRD Expected/Observed/Unknown 合同 |
| High | Scope | 只画四个 agent 会错误暗示产品边界 | 本机已有 OpenCode/Kimi/Gemini/Qwen/Goose/Copilot/DSH/ZCode |
| Medium | Health metaphor | 容易滑向医疗拟物和无证据总分 | 品牌/可信度要求 |
| Medium | Action safety | 诊断列表若直接提供 Fix 会弱化 preview/rollback | F-09 mutation contract |
| High | Evidence semantics | `Resolved` 容易被理解为“问题已解决”或“模型已收到” | Kimi 模拟发现；最终改为 `Static resolution` |
| High | Adapter availability | 安装痕迹、配置目录、可执行文件、认证可用性是四件不同的事 | 本机 DSH/Coze/OpenCode/Gemini/Kiro 探测 |
