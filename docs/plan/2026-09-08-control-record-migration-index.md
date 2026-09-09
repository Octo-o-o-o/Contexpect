# `docs/plan` 控制记录迁移索引（2026-09-08）

> 状态：索引（不改写任何历史记录的终态）。
> 目的：让接手者能从仓内判断 `docs/plan/` 下各条监督交付线的控制记录**在哪里、是否还在途**，而不必凭推断改写它们。

## 结论

`docs/plan/` 下的目录是 2026-09-04 至 2026-09-06 各监督交付线的**受管产物快照**（acceptance-contract、implement/repair 报告、DEFERRED-P2、STOP-CHECKPOINT、scope-addendum）。它们的**控制面**（`*-cycle.json`、`control.json`、`policy.frozen.json`、`acceptance.frozen.json`、逐轮 receipt）由 `.gitignore` 排除，不在版本库内；本机若仍存有这些文件，其 `status`（例如 `contexpect-wp02-fs-collect-cycle.json` 的 `IN_PROGRESS`）是**该文件最后一次被工作流写入时的状态**，不是当前状态。

2026-09-07 起，这些交付线的后续控制记录与审计已迁至**仓外归档**（OctoWorkFlow 的本机归档目录，不入库、不发布）；仓内不再更新它们。判断一条线是否结束，以下列仓内记录为准：

| 交付线目录 | 仓内可见的最后记录 | 当前判定依据 |
| --- | --- | --- |
| `contexpect-docs*`、`contexpect-docs-owner-recovery-*`、`contexpect-docs-semantic-team*` | `contexpect-docs-semantic-team-c2/repair-3-report.md`、`DEFERRED-P2.md` | 文档/合同门禁已进入 `REQUIRED_GATES`；这些线的产物由 `check_docs.py` / `check_semantic_team.py` 持续校验，线本身已结束 |
| `contexpect-wp02-core` | `repair-5-report.md`、`STOP-CHECKPOINT.md`、`DEFERRED-P2.md` | `ctxpect-core` / `ctxpect-schema` 的门禁（cargo-build/test/clippy）已生效；线已结束，遗留项在 `DEFERRED-P2.md` |
| `contexpect-wp02-fs-collect` | `acceptance-contract.json`、`2026-09-06-large-file-decision.astra.md` | 该线的 implement/repair 报告被 `.gitignore` 排除；`crates/ctxpect-fs`、`ctxpect-collect` 已有阶段实现并被 `cargo test` 与 `corpus-conformance` 消费。**仓内不判定其监督流程是否正式收口**；后续工作（含本次缺口分析）以直接交付方式进行，见 [交付状态](../process/2026-09-08-delivery-status.md) |

## 不做什么

- 不修改任何 `*-cycle.json` / 报告的内容或状态字段（AGENTS.md：历史报告不追溯改写）。
- 不把「实现已存在」等同于「该线监督验收通过」；两者分别记录。
- 不在本索引里复制仓外归档的路径或内容。
