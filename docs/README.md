# Contexpect 文档索引

> 状态：规范（尚未实施完整产品运行时）
> 当前仓库是产品定义、架构合同与验收基线。WP-02 的开发切片已提供可构建的
> `ctxpect inspect`（Codex 与 Claude Code 两个 anchor 的 `instructions`，静态只读）；另有
> Receipt 迁移、JSON ledger、Doctor（20 条语料规则）、localhost UI 与 macOS 已验的 Tauri 壳。
> 其余 OS lane 的 WebView、SQLite/FTS5 引擎、E2EE 与其余完整验收尚未实施；逐项见
> [交付状态](process/2026-09-08-delivery-status.md)。

## 阅读顺序

1. [完整产品需求方案](requirements/2026-09-04-contexpect-complete-product-requirements.md)
2. [架构总览](architecture/overview.md)、[数据/真值模型](architecture/data-and-truth-model.md) 与 [语义对齐 / Team Context Standard](architecture/semantic-alignment-and-team-standard.md)
3. [实施计划](process/implementation-plan.md)
4. `acceptance/` 八份 §17.0 工件，以及额外的 `acceptance/semantic-team-contract.yaml`
5. 需要依据或避坑时再读研究台账与设计稿

## 开源根文件

仓库根目录：`README.md`、`LICENSE`（Apache-2.0）、`NOTICE`、`CONTRIBUTING.md`、`CODE_OF_CONDUCT.md`、`SECURITY.md`、`GOVERNANCE.md`、`SUPPORT.md`、`.gitignore`、`AGENTS.md`。

## Canonical 规范

### 架构

- [architecture/overview.md](architecture/overview.md)
- [architecture/data-and-truth-model.md](architecture/data-and-truth-model.md)
- [architecture/semantic-alignment-and-team-standard.md](architecture/semantic-alignment-and-team-standard.md)

### 安全

- [security/privacy-and-threat-model.md](security/privacy-and-threat-model.md)
- [security/encrypted-sync-protocol.md](security/encrypted-sync-protocol.md)

### Adapter

- [adapters/architecture.md](adapters/architecture.md)
- [adapters/authoring-guide.md](adapters/authoring-guide.md)
- [adapters/grammar/codex-cli-0.147.0-instructions.md](adapters/grammar/codex-cli-0.147.0-instructions.md) — Codex anchor `instructions` grammar（G1–G6）canonical 文本：来源、范围、正负例 id；后续 anchor grammar 的模板
- [adapters/grammar/claude-code-cli-2.1.259-instructions.md](adapters/grammar/claude-code-cli-2.1.259-instructions.md) — Claude Code anchor `instructions` grammar（CL1–CL6）：additive 合并、无原生字节 cap、用户 memory 根不读取

### 用户 / 开发者 / 运维指南

- [guides/user-guide.md](guides/user-guide.md) — one-shot / periodic / continuous
- [guides/cli-reference.md](guides/cli-reference.md)
- [guides/desktop-ui.md](guides/desktop-ui.md)
- [guides/configuration.md](guides/configuration.md)
- [guides/operations.md](guides/operations.md)
- [guides/llm-advisor-and-effect-lab.md](guides/llm-advisor-and-effect-lab.md)

### 过程

- [process/implementation-plan.md](process/implementation-plan.md)
- [可视化与完整产品闭环补充实施方案](process/2026-09-06-contexpect-visualization-closure-supplement.md) — 2026-09-06 基线对账；14 个主入口与 2 个配套入口、共用视觉合同及剩余闭环
- [process/test-strategy.md](process/test-strategy.md)
- [process/2026-09-08-delivery-status.md](process/2026-09-08-delivery-status.md) — 当前交付状态：清单逐项、清单外发现、未做项的真实原因
- [process/2026-09-08-gap-analysis.md](process/2026-09-08-gap-analysis.md) — 缺口分析：计划未成文、成文未实施、已实施仍有缺口三类清单与跨类别优先级，附两路交叉复核记录
- [process/2026-09-09-deepseek-harness-review.md](process/2026-09-09-deepseek-harness-review.md) — DeepSeek Harness 建议逐项核验：已修 / 仍成立 / 新发现，与实施交接的映射
- [process/doctor-rule-map.md](process/doctor-rule-map.md) — Doctor 20 条语料规则的语义、输入合同、实现位置与阻断性；与既有 `D-*` 规则的别名关系
- [process/2026-09-08-keyboard-and-responsive-verification.md](process/2026-09-08-keyboard-and-responsive-verification.md) — 键盘/焦点/响应式/隐私边界取证；逐项标注取证强度与未覆盖项
- [process/release.md](process/release.md)
- [process/dependency-and-provenance.md](process/dependency-and-provenance.md)

### ADR

- [adr/0001-rust-tauri-react-sqlite.md](adr/0001-rust-tauri-react-sqlite.md) — Rust workspace + Tauri 2 + React/TypeScript + SQLite/FTS5
- [adr/0002-trust-boundaries.md](adr/0002-trust-boundaries.md)
- [adr/0003-encrypted-sync-and-signing.md](adr/0003-encrypted-sync-and-signing.md)
- [adr/0004-adapter-isolation.md](adr/0004-adapter-isolation.md)
- [adr/0005-exception-identity-and-mutation-boundary.md](adr/0005-exception-identity-and-mutation-boundary.md) — 例外授权的身份源（仓内已登记 principals）与 mutation 门禁边界
- [adr/0006-third-party-dependency-policy-and-estimator.md](adr/0006-third-party-dependency-policy-and-estimator.md) — Rust 零第三方依赖保持、Effect Lab 估计器仓内冻结实现、SQLite 偏离结束条件、前端 Playwright 可选门禁

## 应用

- [apps/desktop/README.md](../apps/desktop/README.md) — Tauri 2 桌面壳：独立 workspace，不注册 Tauri command，只加载回环 daemon

## PRD §17.0 验收工件

| 工件 | 路径 |
| --- | --- |
| compatibility matrix | [acceptance/compatibility-matrix.yaml](../acceptance/compatibility-matrix.yaml) |
| corpus manifest | [acceptance/corpus-manifest.json](../acceptance/corpus-manifest.json) |
| claim validity | [acceptance/claim-validity-matrix.yaml](../acceptance/claim-validity-matrix.yaml) |
| capability matrix | [acceptance/context-capability-matrix.yaml](../acceptance/context-capability-matrix.yaml) |
| projection matrix | [acceptance/projection-matrix.yaml](../acceptance/projection-matrix.yaml) |
| traceability | [acceptance/traceability.csv](../acceptance/traceability.csv) |
| reference hardware | [acceptance/reference-hardware.md](../acceptance/reference-hardware.md) |
| integration contracts | [acceptance/integration-contracts.yaml](../acceptance/integration-contracts.yaml) |
| artifact digest manifest | [acceptance/artifact-digest-manifest.json](../acceptance/artifact-digest-manifest.json) |
| semantic-team contract | [acceptance/semantic-team-contract.yaml](../acceptance/semantic-team-contract.yaml) |

Field-to-claim 草稿：`acceptance/field-to-claim/`。生成夹具：`acceptance/corpus/`（development 可见答案；sealed 答案 withheld；live 仅为 recipe）。`artifact-digest-manifest.json` 里的 digest 是 `check_acceptance.py` 以文本方式读取（换行规范化为 LF）后的 sha256，不是原始字节摘要；`traceability.csv` 含 CRLF，其原始字节摘要与清单值不同，这是既有约定而非篡改。

## 需求与研究（既有）

- [完整产品需求方案](requirements/2026-09-04-contexpect-complete-product-requirements.md)
- [研究台账](research/2026-09-04-context-management-research-ledger.md)
- [Agent / Harness 扩展调研](research/2026-09-04-agent-harness-expansion.md)
- [用户方案来源快照](research/source-snapshots.md)
- [本机环境 Receipt](research/2026-09-04-local-environment-receipt.md)
- [Source-backed freeze coordinates](research/2026-09-04-source-backed-coordinates.md)
- [独立评审记录](review/2026-09-04-contexpect-requirements-review.md)
- [评审输入合同](plan/2026-09-04-requirements-review-prompt.md)
- [`docs/plan` 控制记录迁移索引](plan/2026-09-08-control-record-migration-index.md) — 各监督交付线的控制面已迁仓外归档；仓内记录不追溯改写
- 产品 schema 冻结文本：[ctxpect-receipt-v1](schemas/ctxpect-receipt-v1.schema.json)、[ctxpect-doctor-v1](schemas/ctxpect-doctor-v1.schema.json)（校验测试 `crates/ctxpect-cli/tests/schema_conformance.rs`）

## 设计稿

- [Doctor 设计包](gpt-img-2-design/20260904-1200-contexpect-doctor/00-brief.md)
- [实施映射（设计侧）](gpt-img-2-design/20260904-1200-contexpect-doctor/06-implementation-plan.md) — 指向本文档集，不表示已开始开发
- 最终图：`gpt-img-2-design/20260904-1200-contexpect-doctor/images/10-context-doctor-final.png`

设计图中的 4/9/5 adapter 分组是 Doctor fixture，不是 live compatibility matrix。

## 门禁

本阶段 16 条 required gate（名称 + 命令）必须一起跑：

| 名称 | 命令 |
| --- | --- |
| docs-structure | `python3 scripts/check_docs.py` |
| acceptance-validation | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'` |
| cargo-build | `cargo build --workspace` |
| cargo-test | `cargo test --workspace` |
| cargo-clippy | `cargo clippy --workspace --all-targets` |
| corpus-conformance | `cargo test -p ctxpect-cli --test corpus_conformance` |
| doctor-corpus | `cargo test -p ctxpect-cli --test doctor_corpus` |
| native-conformance | `cargo test -p ctxpect-cli --test native_conformance` |

```bash
python3 scripts/check_docs.py
python3 scripts/check_acceptance.py --structure
python3 scripts/check_acceptance.py --traceability
python3 scripts/check_acceptance.py --corpus
python3 scripts/check_semantic_team.py
TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test -p ctxpect-cli --test corpus_conformance
cargo test -p ctxpect-cli --test doctor_corpus
cargo test -p ctxpect-cli --test native_conformance
```

离线、无第三方依赖。它们不证明产品运行时已经落地。

前端/UI 四条是本阶段新增的 required gate（需要 Node 22 / pnpm；与上表原十二条并列，共 16 条）。cwd：`ui-routes` 为仓库根，其余为 `packages/ui`。失败判据：缺路由、C03 token 漂移、typecheck/build 失败。另有**可选门禁** `ui-e2e`（cwd `packages/ui`，`pnpm test:e2e`）：Playwright 驱动真实 daemon 的浏览器交互测试，需要已安装的 Chromium 与已构建的 `ctxpect`，因此不进 required 表（[ADR 0006](adr/0006-third-party-dependency-policy-and-estimator.md)）；本机交付前必须实跑并记录退出码。

| 名称 | cwd | 命令 |
| --- | --- | --- |
| ui-routes | 仓库根 | `python3 scripts/check_ui_routes.py` |
| ui-unit | `packages/ui` | `pnpm test` |
| ui-typecheck | `packages/ui` | `pnpm typecheck` |
| ui-build | `packages/ui` | `pnpm build` |

## 当前判定

Foundation 文档与合同在本阶段落地。需求没有第四次独立 GREEN。完整交付仍要求一次全产品集成门禁绿与一次独立 readback 绿。
