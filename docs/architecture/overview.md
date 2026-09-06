# 架构总览

> 状态：规范（完整产品运行时尚未实施；CLI/localhost API 已有阶段实现）

本文是 Contexpect 的 canonical 系统架构。它冻结组件边界、进程模型和与 18 个 adapter family 的关系。运行时栈由 [ADR 0001](../adr/0001-rust-tauri-react-sqlite.md) 冻结；信任边界由 [ADR 0002](../adr/0002-trust-boundaries.md) 冻结。

## 产品主张

Expected. Observed. Reconciled.

Contexpect 不替代 coding harness、package manager 或 Git。它把这些来源对齐到 Context IR、Evidence Ledger 和 Context Receipt。

## 组件

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
     SQLite + FTS5 + optional encrypted content vault
            ┌────────────┼────────────┐
           CLI      Local API/Daemon   Sync Engine
                           │
                    Desktop/Web UI
```

| 组件 | 职责 | 非职责 |
| --- | --- | --- |
| Collectors | 探测 executable/App/config residue/auth/connector；扫描声明资产 | 不把 residual 当成可运行；不启动 MCP/hook |
| Resolvers | 按 harness+version+surface+OS 计算 Expected | 不把 Expected 写成 Observed |
| Importers | 导入声明范围内的原生诊断/事件 | 不补造未暴露的 timeline 事件 |
| Evidence Ledger | 保存来源事实、digest、collector version | 不保存无校准 confidence |
| Receipt engine | 不可变摘要、签名、脱敏、等价比较 | 默认不含 prompt/源码/secret |
| Doctor | 确定性 finding 与 PlacementRecommendation | 不是 security boundary；LLM 不能单独让 CI 失败 |
| Projection | CanonicalIntent → 唯一 authority 的 harness-native plan | 不为同一 cell 做第二 projector；不以文件副本当对齐 |
| Sync | E2EE bundle + Git/folder provider；TeamContextStandard 的可选传输 | 不自建云账号、PKI、CRDT；云不是语义依赖 |
| Policy | context-specific 域；组织/团队/项目/角色/个人分层；导入 APM 结果 | 不重算 package 域；无托管通道不得假称 enforce |
| Advisor / Effect Lab | 候选建议 / 受控实验 | 不写入 Claim 真值 |
| UI / CLI / daemon / CI | 同一 core 的入口 | UI 不直接扫 home |

## 入口

同一 core 提供四种入口，行为必须共享 Receipt schema 和 reason code：

- CLI one-shot：`ctxpect inspect|doctor|diff|receipt|intent|standard|align`，无 daemon、无账号、无网络可用
- 可选 daemon：增量 snapshot、去重通知、localhost API
- 桌面 UI：Tauri 2 + React；Doctor 是默认主入口
- CI：JSON / SARIF / Markdown，exit `0/2/3`

浏览器本地模式仅用于调试和无桌面壳预览，不构成独立产品真值。

## 解析坐标

任何 Receipt、diff 或 claim 必须绑定：

device / environment / OS+arch / account alias / organization / policy snapshot / harness / version / surface / project root / Git root / worktree / cwd / task probe / timestamp

脱离坐标讨论“会不会加载”没有确定答案。

## Adapter 机制

18 个 family 通过 Adapter SDK 接入，而不是 18 套 UI。每个 adapter 声明：

- version range 与 surface
- 静态 collector / resolver
- 原生 importer 与 field-to-claim mapping
- capability matrix 单元格：`required-supported` / `required-unknown-honesty` / `not-applicable`
- fixtures 与（仅当声明可重复时）native oracle

Coze 是 connector/executor，不参与隐藏 prompt parity。Aider 不得被硬套成 SKILL.md harness。

## 失败语义

| 情况 | 结果 |
| --- | --- |
| 未知 harness 版本 | 依赖行为的 claim 全部 `indeterminate + unknown`；CI 默认 exit `3` |
| 原生 surface 不存在 | `surface_not_exposed`，不得标 absent |
| 权限未授予 | `permission_not_granted` |
| 扫描/导入失败 | 回滚事务，保留上一有效 snapshot |
| required claim indeterminate | 不能当 pass 或新 baseline |
| 探索预览 | 全部标 `unverified preview`，不得写入权威 Receipt |

稳定 reason code 列表见 [data-and-truth-model](data-and-truth-model.md) 与 `acceptance/claim-validity-matrix.yaml`。

## 语义对齐与团队标准

跨 harness 对齐是 CanonicalIntent 的 native projection，不是把同一份 MD 复制到每个工具。投影结果枚举与 `verified|structural-only|indeterminate|failed` 正交。TeamContextStandard 是签名的可移植 bundle，git/file local-first。细节见 [semantic-alignment-and-team-standard](semantic-alignment-and-team-standard.md)。本阶段没有可运行投影器。

## 数据面

- 元数据：SQLite + FTS5
- 敏感正文：可选加密 vault，与元数据表分离
- 图查询：`resolution_edges` + 递归 CTE，不引入图数据库
- 同步：transport-neutral encrypted bundle；内置 folder 与 Git provider

## 与设计稿的关系

Doctor 高保真稿位于 `docs/gpt-img-2-design/20260904-1200-contexpect-doctor/`。语义与文案以 Markdown spec 和本架构为准；图片负责构图。图中 adapter 4/9/5 分组是 fixture 数据。实现时 UI 必须消费 Receipt，不得自行推断 truth。

## 尚未实施

本文描述将要建造的系统。仓库中没有 `crates/` 运行时、没有 Tauri 窗口、没有可执行 `ctxpect`。
