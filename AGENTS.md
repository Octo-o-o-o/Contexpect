# AGENTS.md

> 状态：规范（WP-02 已授权基础切片开发中，其余产品运行时尚未实施）
> 本文件只约束本仓库的项目级命令与不变量。它不覆盖 owner 的全局工作流、模型分工或监督预算。

## 项目是什么

Contexpect / `ctxpect`：local-first 的 AI coding context 核对与控制。现有 core/schema/fs/collect 已有代码与阶段合同，存在实现不代表已验收。按当前用户授权和有效阶段合同继续；未授权的 Rust/Tauri 工作包不得提前实施。不能用早期“只允许文档”的状态说明阻止已经明确授权的切片。

## 本项目接续与评审边界

本节落实 2026-09-06 owner 对本项目 Prompt/工作流调整的授权，校准问题严重性与接续方式；全局模型分工、3/3 预算及有效冻结记录保持。

- 已授权接续时，可在独立副本接收带逐文件身份的未验收快照并先收口。源仓后续仍可能变化；这影响合并回源仓，不阻止隔离副本中的工作。不覆盖并发修改，不将快照稳定当作验收通过。
- P0/P1 必须同时给出可达路径、违反的具体验收行为以及实质影响。受管文档位置可达不是充分条件：错误命令、权限/范围冲突、会误导实施的有效合同矛盾、关键证据或发布结果造假仍可阻断；历史引文节选、非关键保存位置或措辞精度，若不改变有效合同、执行路径和证据结论，记 P2/P3，不单独阻断产品收口。
- 同因以同一缺陷机制和失败结果判定，不以同一个验收编号、主题或沿用的 blocker ID 代替。原功能缺陷消除后新发现的文档问题独立定级；不得为规避预算改名，也不得把所有后续文字问题都累加为原功能的反复失败。历史报告/计数不追溯改写。
- 文档和流程说明的修复按实际影响复核；相关代码、测试、合同未变且身份可核验时，沿用其有效基线证据。不把每个文字修改都变成全仓重审，不要求对纯描述条目做代码机制删除实验。有效冻结合同的显式特殊要求仍按原记录处理，变更需留痕。
- 运行状态只认实际工作区的 control/state、候选、有效 review 和门禁收据；方案和 Prompt 的历史状态不能代替它们。原始 CLI 事件、含本机路径的日志和输入备份放忽略的 `.octoworkflow/`，不要因它们已被 gitignore 而认为可以放入可发布 `docs/`。

## 本阶段允许的命令

本阶段 required gate 共 13 条（原九条与新增四条前端检查并列）。名称 + 命令必须一起跑。

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
python3 scripts/generate_acceptance.py
python3 scripts/check_semantic_team.py
TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

不要为这些命令安装 pip 依赖，不要访问网络，不要读取凭据或私人会话历史。

前端/UI 四条是本阶段新增的 required gate（需要 Node 22 / pnpm 11.20.0，lockfile 尚未冻结前以实际 `pnpm` 版本为准）。失败判据：缺路由、C03 token 漂移、typecheck/build 失败。env 不强制 `CI`。

| 名称 | cwd | 命令 |
| --- | --- | --- |
| ui-routes | 仓库根 | `python3 scripts/check_ui_routes.py` |
| ui-unit | `packages/ui` | `pnpm test` |
| ui-typecheck | `packages/ui` | `pnpm typecheck` |
| ui-build | `packages/ui` | `pnpm build` |

未执行或失败时不得把这些命令报告为已通过。Tauri 桌面壳尚未创建，因此也不在 Cargo workspace 内；`cargo build --workspace` 不构建桌面壳。

## 不变量

1. Unknown / not-installed / connector-required 必须保持诚实。配置目录或 Recent Documents 不是可运行安装。
2. 禁止编造 native oracle 结果。本冻结只声明两个可重复 oracle：Codex `debug prompt-input`、Grok `inspect --json`。
3. 不得把 LLM 建议写入 Claim provenance、policy、CI、baseline 或 semantic reconciliation。
4. 不得把 F-01–F-18 或 WP-01–WP-12 裁成 MVP。
5. 不得在可发布文件中写入绝对用户家目录、真实 token 或未脱敏会话正文。
6. Acceptance cutoff 为 `2026-09-04T23:59:59+08:00`。cutoff 之后的 harness/OS 不能偷偷变成 required scope。
7. 设计图 `docs/gpt-img-2-design/.../images/10-context-doctor-final.png` 是 Doctor 视觉参考；语义以 Markdown spec 与 PRD 为准。图中 4/9/5 coverage 是 fixture，不是 live 安装矩阵。
8. 生成夹具必须带 digest 与 Apache-2.0，且 `live_tested: false`。

## 术语

- Static resolution ≠ Native evidence ≠ User-attested ≠ Unknown
- Unknown 描述缺证据的 facet；Indeterminate 描述无法给出 pass/deny 的决策
- 六个 lifecycle facet 互相不蕴含

## 代码尚未存在时的路径约定

未来实现按 [implementation-plan](docs/process/implementation-plan.md) 落在 `crates/`、`packages/ui`、`apps/desktop`。现在不要创建空的运行时骨架来假装进度，除非后续工作包明确要求。
