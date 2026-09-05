# Contexpect

> 状态：规范与验收合同已冻结；**尚未实施产品运行时**。
> Expected. Observed. Reconciled.

Contexpect 是一个 local-first 的可视化 AI coding context 核对与控制工具。它解释指定设备、工具版本、项目、工作目录和任务下，哪些上下文按规则应该出现，哪些被运行时实际观察到，哪些仍不可见，并帮助用户安全地对齐、同步、管理和验证这些上下文。

CLI 名称：`ctxpect`  
核心产物：Context Receipt  
默认桌面入口：Context Doctor（工作流名，不是独立品牌）

本仓库当前交付的是完整产品合同、架构文档、开源治理文件和 PRD §17.0 验收基线。Rust workspace、Tauri 2 桌面壳、React/TypeScript UI 和 SQLite/FTS5 运行时**尚未实现**。不要把生成夹具或设计图理解为已经跑通的产品。

## 为什么存在

AI coding context 已经跨过 instruction、rule、skill、plugin、MCP、memory、session 和 execution environment。原生 `/context` 或 `inspect` 只覆盖一个工具；配置同步工具证明不了本轮模型看见了什么；会话分析工具不知道磁盘上的意图为何缺失。

Contexpect 把这些来源对齐到同一套证据模型，而不是再做一份配置源。跨 harness 对齐是语义的、harness-native 的，不是把同一份 MD 复制到每个工具。团队负责人可以发布签名的 Team Context Standard，成员按 native projection 采用。

## 完整范围，不是 MVP

完整交付覆盖 PRD 中的 F-01 至 F-18 与 WP-01 至 WP-12，包括 18 个 adapter family 的声明 surface。顺序只用于降低返工，不缩小最终范围。

明确不做：提取隐藏 system prompt、自建通用 marketplace、默认 MITM、无证据的健康总分、用试验版替代完整合同。详见 [产品边界](docs/requirements/2026-09-04-contexpect-complete-product-requirements.md#19-产品边界完整交付仍明确不做)。

## 18 个 adapter family

| 分组 | Family |
| --- | --- |
| Anchor | Codex、Claude Code、Cursor、Grok Build |
| 本机 expansion | OpenCode、Kimi Code、ZCode、Qwen Code、Goose、Gemini CLI、GitHub Copilot CLI、Kiro |
| Hermetic / connector | DeepSeek Harness、Cline、Aider、OpenHands、Windsurf、Coze |

“支持”按 `family × version × surface × capability` 声明。配置残留或 Recent Documents 痕迹不等于可运行安装。本机 live lane 的诚实状态写在 [`acceptance/compatibility-matrix.yaml`](acceptance/compatibility-matrix.yaml)。

设计稿里的 4 native / 9 static / 5 connector 是 Doctor 界面的 **OctoWorkflow fixture**，不是本机 live 安装清单。

## 文档

从 [docs/README.md](docs/README.md) 进入全部规范、ADR 与验收件。

必读：

- [完整产品需求](docs/requirements/2026-09-04-contexpect-complete-product-requirements.md)
- [架构总览](docs/architecture/overview.md)
- [真值模型](docs/architecture/data-and-truth-model.md)
- [实施计划 WP-01–WP-12](docs/process/implementation-plan.md)

## 本阶段门禁（离线）

本阶段九条 required gate（名称 + 命令）必须一起跑：

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
```

这些命令不访问网络、不安装依赖、不启动 harness。它们验证文档完整性、八份 §17.0 验收件、语义对齐/Team Context Standard 合同、规范性语句追踪和语料数量/分类。它们**不是**产品运行时测试。

重新生成验收夹具（可选，确定性）：

```bash
python3 scripts/generate_acceptance.py
```

## 许可

Apache License 2.0。见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。依赖与 SBOM 政策见 [dependency-and-provenance](docs/process/dependency-and-provenance.md)。

## 安全与治理

- 报告漏洞：[SECURITY.md](SECURITY.md)
- 贡献：[CONTRIBUTING.md](CONTRIBUTING.md)
- 行为准则：[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- 治理：[GOVERNANCE.md](GOVERNANCE.md)
- 支持：[SUPPORT.md](SUPPORT.md)
- Agent 本地约定：[AGENTS.md](AGENTS.md)
