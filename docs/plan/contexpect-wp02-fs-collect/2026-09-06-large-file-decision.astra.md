# WP-02 fs/collect 大文件读取裁决 · 2026-09-06

> 状态：规范记录（产品运行时其余阶段尚未实施）。记录 owner 对 OR1-01 的产品取舍，不是独立 review 结论，也不宣称 fs/collect 已收口。

## Owner 原话

「可以放宽超限停止读取。」

写入恢复控制 `docs/plan/contexpect-wp02-fs-collect-owner-recovery-1/owner-decision-2026-09-06.json`，绑定有效范围 `acceptance-scope.revision-2.json`。

## 旧语义

原 A8：「大小上限：超过声明阈值的文件不读全文，记录为截断并保留其原因，不得静默截断成看似完整的内容。」

旧 A8 完整正文保存在恢复控制的 `acceptance-scope.json`；历史 `review-1.md` 保留相关节选、冲突说明和评审结果。机器 `acceptance.frozen.json` 原样保留，仅承载 ID/severity/dimensions/required gates，不含新旧验收正文；新正文由 `acceptance-scope.revision-2.json` 与 `owner-decision-2026-09-06.json` 的 hash 绑定。

## 新语义

普通文件允许流式读取全文并计算全文件摘要；仅在内存保留不超过 `MAX_READ_BYTES`（当前 1 MiB）的正文前缀，另有固定大小的流式读取/摘要缓冲。超过保留阈值时必须标明保留正文已截断及原因/实际长度，不能将前缀冒充完整正文。该阈值不是总 I/O 或扫描时长上限。

`truncated` 标明保留正文已截断；`declared_len` / 清单 `len` 是真实长度。截断的是保留正文，不是摘要覆盖范围。

## A6 不变

重复扫描摘要稳定；任一被收录文件的内容变化必须使摘要变化，包括超过保留前缀之后的尾部变化。实现仍为既有 `stream_digest` 全文件 SHA-256。

## 原文保存位置

- 有效范围与 SHA：恢复控制 `owner-decision-2026-09-06.json` 的 `effective_scope` / `effective_scope_sha256`
- 旧范围原文：恢复控制 `acceptance-scope.json`（`original_scope_sha256` 见 revision-2 的 `owner_amendment`）
- 历史 RED 与当时 `stop_for_owner`：恢复控制 `review-1.md`；当时检查点：`closeout-checkpoint.astra.md`
- 产品合同 A8 已同步到本目录 `acceptance-contract.json`
- PRD §14.2 单条按本裁决修订后，`acceptance/traceability.csv` 与 `acceptance/artifact-digest-manifest.json` 已经 canonical 入口 `scripts/generate_acceptance.py` 重生成（新增 `REQ-14.2-9afd24c1-9a960f0b1cbbd1ad`），未手改任何验收答案
