# semantic-team c2 · repair-3 报告

> 上游：`.octoworkflow/docs-semantic-team-c2-review-2.md`，verdict **RED**，1×P0。
> reviewer 独立复核确认：c2-B1/B2/B3 三条真实关闭且无误杀；c1 的 B2–B6、B9 结论抽样复核均成立；
> 正向控制 8/8 零违规；生成完全确定。唯一 blocker 是新发现的 C2-N1。

## C2-N1 / P0 / ST2 — `required_capability` 未与 canonical intent 绑定

指控：`capability_negotiation.required_capability` 由 projection 自选，不与 canonical intent 的同名字段
对账。connector `coze` 可以改称自己协商的是它确实支持的 `mcp-declarations`，把「无法表达所需语义」的诚实
`unsupported` 叙事整体翻转成 `required-supported` / `transformed`，`collect_violations` 与
`check_fixture_semantics` 双双零错误。

根因确认（独立读码）：`CANONICAL_INTENT_FIELDS` 里本就有 `required_capability`，
`vitest_intent()` 取值 `"instructions"`，但 `_validate_projection` 只校验 negotiation 的
`required_capability` 是否属于 `CAPABILITY_IDS`，从不与 intent 对账。

处置：

1. `_validate_projection` 新增 `declared_capabilities` 参数；`collect_violations` 从
   `payload["canonical_intents"]` 收集所有 `required_capability` 并传入。
   negotiation 声明的能力不在该集合内时，落 `silent-capability-loss` + `family-native-mismatch`。
   语义：harness 不能自行决定 intent 需要什么能力。
2. 顺带关闭同 root cause 的 **P2-c2-1**：当 status 属 unsupported 类时，`unsupported` 列表必须
   包含 `required_capability` 本身 —— 声明「别的能力不支持」不能替代回答「所需能力能不能提供」。
   reviewer 在 review-2 中指出该台账条目的严重度被实质低估，且与 C2-N1 同源；一并修复。

复现验证：

| 向量 | 结果 |
| --- | --- |
| coze 改称协商 `mcp-declarations` + `required-supported` + `transformed`（reviewer 原始向量） | REJECTED |
| 18 个 family 逐个把 `required_capability` 改为 `mcp-declarations` | false accept **0/18** |
| coze 的 `unsupported` 列为 `["tool-invocation"]` / `["memory"]` / `["skills"]`（P2-c2-1） | 全部 REJECTED |

防误杀验证（产品语义红线）：

- `deepseek-harness` 的诚实 Unknown 仍可表达：`status=unknown` / `outcome=unknown` /
  `reconciliation_state=indeterminate`，ST2-pos 零违规。
- `coze` 的诚实 unsupported 仍可表达：`status=not-applicable` / `outcome=unsupported` /
  `unsupported=["instructions"]`，ST2-pos 零违规。
- 8 个 positive fixture 全部零违规。

## 门禁证据（本轮真实执行）

| 名称 | exit | 摘要 |
| --- | ---: | --- |
| docs-structure | 0 | PASS；root files 10；canonical docs 22 |
| acceptance-validation | 0 | PASS |
| traceability-validation | 0 | statements 249；rows 249；PASS |
| corpus-validation | 0 | declared oracles 2；doctor cases 589；PASS |
| semantic-team-validation | 0 | scenarios 16；malformed fixtures 5；PASS |
| validator-negative-tests | 0 | `Ran 148 tests in 78.418s`；`OK`；0 skip |

- 负向测试 144 → **148**：新增 `SemanticTeamC2Review2` 共 4 个方法（含 1 个防误杀的正向保护用例）。
- 生成器连跑两次，`acceptance/` 树聚合摘要一致：
  `687c8090b058cd9e35dbc225a4712c66d64c6dcd2e61888ed02bb162dbb4190d`
  （与 repair-2 相同 —— 本轮只加严 validator，未改 fixture 内容）。
- `git diff --check`：通过。

## 候选漂移说明（外部并发写入）

review-2 报告 §末尾指出评审窗口内 fingerprint 发生漂移。已查明来源：**另一个并发会话**在
`docs/research/2026-09-05-ecc-everything-claude-code-research.md` 与
`docs/plan/2026-09-05-ecc-borrowing-assessment.fable.md` 写入 ECC 借鉴调研，与本任务无关。

- 其中一个瞬时中间状态（research 文档已写入、它引用的 plan 文档尚未写入）曾使
  `check_docs.py` 因 broken relative link 短暂转红；该 plan 文档随后补齐，门禁恢复 exit 0，已复跑确认。
- 本次修复涉及的文件（`scripts/contexpect_semantic_team.py`、`tests/acceptance/test_semantic_team.py`、
  `README.md`、`acceptance/` 树、冻结控制面）均未被该会话触碰。
- reviewer 已独立核验：所有被评审面 digest 全程未变，被跟踪文件改动数始终 23，
  其 RED 结论成立于 `bbb8fb65…`。

## Deferred P2

- **P2-c2-1**：本轮已关闭（与 C2-N1 同 root cause，见上）。
- **P2-c2-2**：仍未修 —— `deepseek-harness` 作为「诚实 Unknown」范例，但
  `registry_capability_status("deepseek-harness","instructions")` 返回 `required-supported`，
  该诚实性无 registry 依据。属 fixture 与 registry 的语义错配，非 validator 缺陷，留待最终 P2 sweep。

## 预算

- product repair：**3/3**（repair-1、repair-2、repair-3）— 已用尽
- product rereview：2/3 已用（c2 review-1 为首轮 review 不计；review-2 计 1；本轮复审计第 2）
- strategy reset：0/1
- C2-N1 root cause 修复次数：1（上限 2）

## 未做

- 未 commit、未 push。
- 未开始 Rust/Tauri 产品运行时编码。
