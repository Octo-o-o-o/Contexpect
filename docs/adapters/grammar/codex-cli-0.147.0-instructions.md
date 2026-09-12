# Grammar：Codex CLI 0.147.0 / cli / macos-27-arm64 — `instructions`

> 状态：规范（已实施于 `crates/ctxpect-resolve/src/lib.rs`，`Grammar::CodexInstructions`）。本文是该 anchor `instructions` capability 静态解析规则的 canonical 文本；其它 anchor 的 grammar 文档沿用本模板。
> 坐标：`codex/0.147.0/cli/macos-27-arm64`（`acceptance/compatibility-matrix.yaml`，`static_support: required-supported`，版本来源 local-environment receipt）。
> 来源回溯：夹具生成器 `scripts/generate_acceptance.py` 为静态用例写入 `provenance: official-spec`（经 `scripts/contexpect_contract.py` 的 claim 生成），其规范事实记录在 [研究台账 §6.1](../../research/2026-09-04-context-management-research-ledger.md)（访问日期 2026-09-04；一手来源 [Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)）。本文不新增未在台账记录的规范断言。

## 模板约定

每条规则给出：**语义**、**来源**（一手来源或台账条目；`product` 表示 Contexpect 产品规则而非 harness 原生规则）、**范围**（本切片实施到哪一步、哪些情况显式不观察）、**正例 / 负例 id**（`acceptance/corpus/...` 的用例 id 或 `crates/*/tests` 的测试名）。规则 id 出现在产品输出 `explanation[].rule_id` 与 `results[].edges[].rule_id` 里。

## 输入

- 项目根（`--project`），cwd（`--cwd`，须在项目内），可选 `--codex-home`（显式授权的 Codex home 根；不传则不读，也不读 `HOME` / `CODEX_HOME`）。
- 只对命名候选路径做分类（`Root::classify`）与包含性读取（`read_contained`），不全树扫描，不执行任何进程，不读 `config.toml`。

## 规则

| 规则 | 语义 | 来源 | 范围 | 正例 | 负例 |
| --- | --- | --- | --- | --- | --- |
| G1 | 从项目根向 cwd 逐层查找，每层考虑 `AGENTS.override.md`、`AGENTS.md`，**每层最多采纳一份**；cwd 以下与旁支目录的文件不被读取 | 台账 §6.1「从项目根向 cwd 逐层查找；每层最多取一份」 | 已实施；配置的 fallback filenames **不观察**（`assumptions.fallback_filenames`） | `dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:positive`、`…:include:00`；`grammar.rs::g1_walks_root_toward_cwd_and_skips_files_below_cwd` | `dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:negative`（被 G4 排除 → indeterminate / `observation_scope_excluded`，C-F01）|
| G2 | 同层 `AGENTS.override.md` 存在时 `AGENTS.md` 被 overridden-by 且 shadowed-by，两条边都指向 override 文件；不可读的 override（符号链接逃逸、FIFO、多硬链接）仍占据该层名额，`AGENTS.md` 不被采纳、claim 保持 indeterminate | 台账 §6.1 每层顺序 `AGENTS.override.md`、`AGENTS.md` | 已实施 | `grammar.rs::g2_override_marks_agents_overridden_and_shadowed`、`regular_override_still_adopts_override_and_marks_agents` | `grammar.rs::unknown_override_escape_symlink_blocks_same_layer_agents`、`unknown_override_fifo_blocks_same_layer_agents` |
| G3 | 采纳文件按顺序累计字节，累计到 `project_doc_max_bytes`（官方默认 32768）后截断；被截断的文件与偏移记为 truncated-after；恰到上限不记；上限后的零字节文件不记 | 台账 §6.1「聚合项目文档受 project_doc_max_bytes 限制，默认 32 KiB」 | 已实施默认值；`config.toml` 覆盖 **不观察**（`assumptions.project_doc_max_bytes.source`） | `grammar.rs::g3_truncates_aggregate_at_official_spec_cap`、`g3_one_byte_file_after_exact_cap_is_truncated_at_file_offset_zero` | `grammar.rs::g3_does_not_truncate_when_aggregate_is_at_or_below_cap`、`g3_zero_byte_file_after_exact_cap_is_not_truncated` |
| G4 | 项目根 `.ctxpect-ignore` 中与候选完全相同的相对路径排除该文件（excluded-by，`exclusion_class: product-user-exclusion`）；被排除文件不读取、证据摘要为 null；绝对路径、含 `..` 段、含控制字符的行跳过并只记 1 基行号。**C-F01**：排除只是 Contexpect 观察范围收窄——若无其它采纳候选，claim 为 indeterminate + `observation_scope_excluded`，不得折叠成原生 absence；完全无候选才保持 absent | **product**（Contexpect 用户排除规则，不是 Codex 原生规则） | 已实施；glob / 取反 / 注释语法 **不观察** | `dev:static:codex/0.147.0/cli/macos-27-arm64:instructions:negative`；`grammar.rs::g4_ignore_exclusion_is_observation_scope_unknown_not_absent`（含"仅排除 override、AGENTS.md 仍采纳 → present"与"完全无候选 → absent"正反例） | `grammar.rs::g4_ignore_skips_absolute_and_parent_lines_with_line_warnings` |
| G5 | 全局层（`$CODEX_HOME/AGENTS.md`）只在显式 `--codex-home` 下读取；未授权时该层 unknown-because `permission_not_granted`（不阻断项目层结论）；授权后只分类 `AGENTS.md` / `AGENTS.override.md`，不列目录、不读其它文件 | 台账 §6.1「`CODEX_HOME` … 会改变发现结果」+ **product** 权限模型（不读 `HOME`） | 已实施 | `grammar.rs::g5_explicit_codex_home_includes_global_agents`、`g5_global_layer_root_kind_is_codex_home` | `grammar.rs::g5_without_codex_home_global_layer_is_permission_not_granted`、`explicit_codex_home_does_not_inventory_non_agents_files` |
| G6 | 坐标不是本 anchor 时不应用本 grammar：OS lane 不在捕获集 → `official_distribution_not_captured`；surface 不在捕获集 → `surface_not_exposed`；版本非冻结 → `unsupported_harness_version`；顺序 lane → surface → harness → version。非 `instructions` 的 capability → `surface_not_exposed` | **product** 诚实性规则（PRD §17.1 Unknown 语义；`acceptance/compatibility-matrix.yaml` 的 `required-unknown-honesty`） | 已实施（`coordinate_unknown_reason`，多 anchor 表驱动） | `dev:static:codex/0.147.0/cli/ubuntu-24.04-x86_64:instructions:indeterminate` 等 8 条 codex honesty cell（`corpus_inspect.rs::corpus_unknown_honesty_cells_match_expected_reason`） | `product_loops.rs::l01_…`（`--version 0.99.0` → `unsupported_harness_version`，exit 3） |

## 不观察的项（显式）

fallback filenames、`project_doc_max_bytes` 的 config 覆盖、其它 OS lane / surface 的真实执行、native oracle 对账（`codex debug prompt-input`）、其它 capability（skills、plugins、MCP、commands、agents、memory…）、`.ctxpect-ignore` 的 glob/取反/注释、Windows junction / 大小写折叠、cwd 以下或旁支目录的文件是否会被 Codex 读取。这些在 `crates/ctxpect-resolve/src/lib.rs` 模块文档的「What they do not observe」保持同步。

## 门禁

- `cargo test -p ctxpect-resolve --test grammar`（G1–G5 单元夹具，真实文件系统）。
- `cargo test -p ctxpect-cli --test corpus_inspect`（3 条 codex instructions golden + 8 条 honesty cell，从二进制 JSON 断言）。
- `cargo test -p ctxpect-cli --test corpus_conformance`（required gate `corpus-conformance`：该坐标 `instructions` 3/3，其余 57 行计未实现）。
