# ADR 0001：Rust workspace + Tauri 2 + React/TypeScript + SQLite/FTS5

> 状态：已接受（规范冻结；尚未实施产品运行时）
> 日期：2026-09-04

## 决策

完整产品的实现栈冻结为：

- 核心：Cargo workspace 中的 Rust crates，负责扫描、路径安全、resolver、SQLite、加密、CLI、daemon、adapter runtime
- UI：Tauri 2 + React + TypeScript，共享本地 API/commands
- 数据：SQLite + FTS5；敏感正文进入独立加密 vault
- Schema：JSON Schema + versioned SQL migrations
- 可视化：React 原生 SVG/Canvas；图由关系数据生成

浏览器本地模式仅作为调试和无桌面壳入口，不构成第二条真值路径。

## 理由

本地文件系统、daemon、跨平台单包、安全边界和资源占用都是核心要求。Rust 适合路径安全、原子写入和稳定 CLI。Tauri 2 把 renderer 放在信任边界之外。SQLite+FTS5 满足 Receipt/inventory/search，无需图数据库。

## 后果

- WP-02 起按 [implementation-plan](../process/implementation-plan.md) 创建 crates，而不是临时脚本核心
- 若维护能力变化，PRD 允许改 Electron/TypeScript 单栈，但必须新 ADR，且不能牺牲路径安全、原子写入和稳定 CLI
- 引入图数据库需要真实规模证明 SQLite 不足，并另开 ADR

## 非目标

Foundation 阶段只冻结选择。后续工作包按 implementation-plan 写运行时代码；本 ADR 不重新选型。SQLite+FTS5 仍是目标引擎；当前 JSON ledger 是为保持 Cargo 零第三方依赖的实现偏离，需在引擎切换时保留同一 DTO。
