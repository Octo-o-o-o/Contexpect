# Grammar：Claude Code CLI 2.1.259 / cli / macos-27-arm64 — `instructions`

> 状态：规范（已实施于 `crates/ctxpect-resolve/src/claude_code.rs`，`Grammar::ClaudeCodeInstructions`）。模板沿用 [Codex grammar](codex-cli-0.147.0-instructions.md)。
> 坐标：`claude-code/2.1.259/cli/macos-27-arm64`（`acceptance/compatibility-matrix.yaml`，`static_support: required-supported`；版本来源 [本机环境 Receipt](../../research/2026-09-04-local-environment-receipt.md)）。
> 来源回溯：夹具生成器 `scripts/generate_acceptance.py` 为该坐标写入 `provenance: official-spec`；规范事实记录在 [研究台账 §6.2](../../research/2026-09-04-context-management-research-ledger.md)（访问日期 2026-09-04；一手来源 [Claude Code memory](https://code.claude.com/docs/en/memory)）。台账 §6.2 记录了层级（managed / user / project / local）与「祖先文件启动加载」；**CL2 的 `.claude/CLAUDE.md` 路径、CL3 的 `CLAUDE.local.md` 文件名与「additive、无 override」的合并语义来自同一份 memory 文档，但未逐字进入台账**（2026-09-09 交叉 review 指出）——它们是本文相对台账的补充断言，来源同为上述一手页面，离线未复核；补记台账前以本文为准并保留此标注。台账未记录字节上限，本文亦不声称存在。

## 输入

- 项目根、cwd（同 Codex）。**不读**用户 memory 根（`~/.claude/CLAUDE.md`），也不读 `HOME`；`--codex-home` 对本 grammar 无效（它是 Codex home，不是 Claude 的）。
- 只分类命名候选并包含性读取，不全树扫描，不执行进程，不跟随 `@path` 导入。

## 规则

| 规则 | 语义 | 来源 | 范围 | 正例 | 负例 |
| --- | --- | --- | --- | --- | --- |
| CL1 | 从项目根向 cwd 逐层查找 `CLAUDE.md`；每个存在且可读的文件都被采纳（**additive**，无 override）；cwd 以下与旁支目录不读取 | 台账 §6.2「祖先文件启动加载」「Claude Code 原生读取 `CLAUDE.md`」 | 已实施；子目录文件的**延迟加载**（Claude 读到该目录时才加载）**不观察** | `dev:static:claude-code/2.1.259/cli/macos-27-arm64:instructions:positive`、`…:include:00`、`…:progressive:18`；`grammar.rs::cl1_claude_md_is_additive_from_root_toward_cwd_and_skips_below_cwd` | `…:instructions:negative`、`…:negative:09`（CL4 排除 → 观察范围排除 indeterminate / `observation_scope_excluded`，C-F01，不再是 absent） |
| CL2 | 同层 `.claude/CLAUDE.md` 是项目 memory 的替代位置，与 `CLAUDE.md` 一同采纳 | 台账 §6.2「project … 层级会影响拼接」；memory 文档的 `./.claude/CLAUDE.md` 位置 | 已实施 | `grammar.rs::cl2_cl3_alternate_location_and_local_file_are_adopted_alongside` | 同上（不存在则不计） |
| CL3 | 同层 `CLAUDE.local.md`（local 层）一同采纳 | 台账 §6.2「managed、user、project、local … 会影响拼接」 | 已实施 local；**managed** 层无本机来源，**不观察** | `grammar.rs::cl2_cl3_…` | 同上 |
| CL4 | `.ctxpect-ignore` 精确相对路径排除候选（product-user-exclusion）；被排除文件不读取。**C-F01**：排除只是 Contexpect 观察范围收窄，claim 为 indeterminate + `observation_scope_excluded`，不得折叠成原生 absence | **product** | 已实施（与 G4 同一解析器：绝对路径 / `..` / 控制字符行跳过） | `…:instructions:negative`（`native_paths_used = [".ctxpect-ignore","CLAUDE.md"]`，indeterminate / `observation_scope_excluded`）；`grammar.rs::cl4_ignore_exclusion_is_observation_scope_unknown_not_absent`（含"另有采纳候选仍 present"与"完全无候选仍 absent"正反例） | 无 ignore 时 `…:positive` present |
| CL5 | 用户 memory 根 `~/.claude/CLAUDE.md` 在本切片**不读取**：全局层 root kind `harness-home`，unknown-because `permission_not_granted`，不阻断项目层结论 | 台账 §6.2「user」层 + **product** 权限模型 | 未实施读取（无授权入口）；显式记为 unknown | `grammar.rs::cl5_user_memory_is_permission_not_granted_and_does_not_block` | — |
| CL6 | 项目根 `budget.json`（`{"path", "max_bytes"}`）是 **Contexpect 的 cap 声明**：当它命名的文件被采纳且实际字节 > `max_bytes`，记 truncated-after（offset = max_bytes，related_path = budget.json），`budget.json` 计入 `native_paths_used` 与 evidence；命名未采纳文件的声明不生效 | **product** 声明（台账 §6.2 明确 200 行 / 25KB 是指导而非硬截断，因此**不存在** Claude Code 原生字节 cap） | 已实施 | `dev:static:claude-code/2.1.259/cli/macos-27-arm64:cap:27`（`native_paths_used = ["CLAUDE.md","budget.json"]`，present）；`grammar.rs::cl6_budget_declaration_truncates_the_named_file_only` | 同一测试的第二段（声明命名 `README.md` 时不生效） |
| G6（共用） | 坐标不是本 anchor 时不应用本 grammar；顺序 lane → surface → version | **product**（同 Codex G6） | 已实施（`coordinate_unknown_reason` 表驱动） | 5 条 claude-code honesty cell：`dev:static:claude-code/2.1.259/cli/ubuntu-24.04-x86_64:instructions:indeterminate`（lane）、`…/cli/windows-11-24h2-x86_64:…`（lane）、`dev:static:claude-code/unknown-honesty/cloud/macos-27-arm64:…`（surface）、`…/cloud/ubuntu…`、`…/cloud/windows…`（lane） | — |

不可读候选（目录、FIFO、逃逸的符号链接、多硬链接）记 unknown-because `content_redacted_by_policy`；只有**所有层**都没有采纳文件时 claim 才保持 indeterminate——另一层采纳了文件则为 present 并带该 unknown（`grammar.rs::claude_non_regular_candidate_is_unknown_not_absent` 覆盖 present 分支；全层不可读的 indeterminate 分支由 `claude_all_layers_unreadable_is_indeterminate` 覆盖）。

## 与 Codex grammar 的差异（对齐要点）

- 文件名：`CLAUDE.md` 系列，不是 `AGENTS.md`；Claude Code 可用 `@AGENTS.md` 导入，但导入不在本切片（见下）。
- 合并方式：**additive**，没有 override / shadow 语义（Codex G2 不适用）。
- 字节上限：Codex 有官方默认 32 KiB（G3）；Claude Code 无官方硬上限，只有 CL6 的 Contexpect 声明。
- 全局层：Codex 可经 `--codex-home` 授权读取（G5）；Claude Code 的用户 memory 根在本切片不可授权读取（CL5）。

## 不观察的项（显式）

`@path` 导入、子目录 `CLAUDE.md` 的延迟加载、**项目根之上的祖先目录**（台账「祖先文件启动加载」未以项目根为界；实现只在项目根到 cwd 之间查找，根之上是否被 Claude Code 读取既不声明观察也不声明不观察）、managed policy memory、用户 memory 根、auto memory、`/context` 等运行时可视化、其它 capability（skills、rules、plugins、MCP、commands、agents、memory…；该坐标 60 条 golden 中 54 条属于它们，`corpus-conformance` 计为未实现）、native oracle（本阶段未为 Claude Code 声明可重复 oracle，AGENTS.md 不变量 2）。

## 门禁

- `cargo test -p ctxpect-resolve --test grammar`（CL1–CL6）。
- `cargo test -p ctxpect-cli --test corpus_conformance`（该坐标 `instructions` 6/6，其余 54 条计未实现；5 条 honesty cell 通过）。
