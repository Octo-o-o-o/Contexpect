# Contexpect

> 状态：规范与验收合同已冻结；产品运行时**尚未实施**。
> WP-02 的开发切片提供了可构建的只读 `ctxpect inspect`（见[快速开始](#快速开始)）。
> Expected. Observed. Reconciled.

Contexpect 是一个 local-first 的可视化 AI coding context 核对与控制工具。它解释指定设备、工具版本、项目、工作目录和任务下，哪些上下文按规则应该出现，哪些被运行时实际观察到，哪些仍不可见，并帮助用户安全地对齐、同步、管理和验证这些上下文。

CLI 名称：`ctxpect`  
核心产物：Context Receipt  
默认桌面入口：Context Doctor（工作流名，不是独立品牌）

本仓库当前交付的是完整产品合同、架构文档、开源治理文件、PRD §17.0 验收基线，WP-02
的 `ctxpect inspect` 切片，以及后续 crate 中的 Receipt 迁移、JSON ledger、Doctor/diff/policy
与 localhost UI。完整 WP-02–WP-12 验收、Tauri 全 OS WebView、SQLite/FTS5 引擎与真实 oracle
**尚未实施**。不要把生成夹具或设计图理解为已经跑通的产品，也不要把单 anchor 的
静态 `inspect` 当作完整 WP-02。

## 快速开始

需要 Rust 1.90+（edition 2024）。构建与试跑都在本地离线完成，不访问网络、不启动任何 harness、
不读取你的真实 home 或 session。

```bash
cargo build --release -p ctxpect-cli
```

对仓库内的开发语料跑一次静态检查：

```bash
CORPUS=acceptance/corpus/development/static/inputs
./target/release/ctxpect inspect --offline --project "$CORPUS/dev__static__codex__0.147.0__cli__macos-27-arm64__instructions__positive"
```

输出会说明：检查的 scope 与坐标、required 集合的结果、每条规则**为什么**纳入或排除、
依据在哪个文件、什么仍不确定、以及下一步需要什么证据。

三种结果各看一次（exit code 是结果的一部分）：

| 命令尾部 | 含义 | exit |
| --- | --- | ---: |
| `..._instructions__positive` | 规则命中，指令被纳入 | 0 |
| `..._instructions__negative` | 被 `.ctxpect-ignore` 列出的 `AGENTS.md` 排除（G4 产品排除，不是 Codex 原生规则），required 判定为 absent | 2 |
| `--require tool-invocation ..._tool-invocation__indeterminate` | 本切片未解析该能力，诚实报 Unknown | 3 |

完整参数见 `ctxpect --help`。常用的几个：`--json` 输出机器可读结果；`--codex-home <dir>`
显式授权一个包含全局 `AGENTS.md` 的目录（不传则按 G5 报 `permission_not_granted`，不会去猜
`$HOME` 或 `$CODEX_HOME`）；`--cwd <rel>` 指定项目内的工作目录。CLI 参考里列出但本切片未实施的
子命令与参数会明确拒绝，不会被静默忽略。

### 这个切片能做什么、不能做什么

能：对 Codex CLI 0.147.0 / cli / macOS lane 的 instructions（AGENTS.md 链）做静态解析，
给出带来源链和 scope 的解释、稳定的开发快照摘要，以及路径脱敏后的 JSON/human 输出。

不能：执行任何 harness、hook、MCP 或 plugin；读取真实私人 home、session 或凭据正文；
断言运行时事实。所有 model-visible / use-evidence / outcome-affecting 一律报
`indeterminate`，并指明需要 native runtime snapshot 才能确定。其余 17 个 adapter family、
其余 capability、Receipt 签名、diff/store/daemon/UI 都尚未实施。

## 当前实现状态

| 工作包 | 内容 | 状态 |
| --- | --- | --- |
| WP-01 | 契约、fixture 与威胁模型 | 文档与合同部分已交付 |
| WP-02 | Core collector、resolver 与 CLI | 进行中：`ctxpect-core`/`-schema`/`-fs`/`-collect`/`-resolve`/`-cli` 已有阶段实现与阶段验收记录 |
| WP-03 | snapshot、diff 与 evidence ledger | JSON document ledger 已落地；SQLite+FTS5 引擎仍为 ADR 目标，尚未切换 |
| WP-04 | Inspector UI 与本地 API | React UI + `ctxpect daemon` localhost API 已可构建；Tauri 窗口不在 Cargo workspace |
| WP-05 – WP-12 | daemon、projection、同步、生态、advisor、policy、集成验收 | 有本地实现与负例测试；全矩阵/真机/人工门禁尚未实施 |

存在源码不代表已通过该阶段的完整验收；以各阶段合同与有效交付记录为准。详见
[实施计划](docs/process/implementation-plan.md)。

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

本阶段 13 条 required gate（名称 + 命令）必须一起跑：

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

前端/UI 四条是本阶段新增的 required gate（需要 Node；与上述原九条并列，共 13 条）：ui-routes、ui-unit、ui-typecheck、ui-build。`ui-routes` cwd 为仓库根，其余 cwd 为 `packages/ui`。失败判据：缺路由、C03 token 漂移、typecheck/build 失败。

```bash
python3 scripts/check_ui_routes.py
```

在 `packages/ui`：

```bash
pnpm test
pnpm typecheck
pnpm build
```

等价写法：

```bash
python3 scripts/check_ui_routes.py
pnpm --dir packages/ui test
pnpm --dir packages/ui typecheck
pnpm --dir packages/ui build
```

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
