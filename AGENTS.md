# AGENTS.md

> 状态：规范（尚未实施产品运行时）
> 本文件只约束本仓库的项目级命令与不变量。它不覆盖 owner 的全局工作流、模型分工或监督预算。

## 项目是什么

Contexpect / `ctxpect`：local-first 的 AI coding context 核对与控制。当前阶段只允许文档、验收合同、合成夹具和离线校验脚本。不要开始实现 Rust/Tauri 产品运行时，除非当前任务明确授权下一个工作包。

## 本阶段允许的命令

六条 required gate（名称 + 命令）：

| 名称 | 命令 |
| --- | --- |
| docs-structure | `python3 scripts/check_docs.py` |
| acceptance-validation | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'` |

```bash
python3 scripts/check_docs.py
python3 scripts/check_acceptance.py --structure
python3 scripts/check_acceptance.py --traceability
python3 scripts/check_acceptance.py --corpus
python3 scripts/generate_acceptance.py
python3 scripts/check_semantic_team.py
TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'
```

不要为这些命令安装 pip 依赖，不要访问网络，不要读取凭据或私人会话历史。

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
