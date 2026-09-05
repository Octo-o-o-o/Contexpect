# WP-02 第一刀 · repair-1 报告

> 上游：`.octoworkflow/wp02-core-review-1.md`，verdict **RED**，2×P1、0×P0。
> 首次 RED，两条均 `in_scope=true` / `needs_owner_decision=false`，按合同自动返工一次。
> reviewer 同时确认：Rust 侧 26 组篡改 26/26 转红；九条不变量在 4,147,200 点扫描下可达且无误杀；
> SHA-256 独立 208 组对拍全对；clippy deny 确为真实执行；六份门禁文档九条齐全且逐份删除均转红。

## F1 / P1 / W6 — canonical_json 浮点渲染与 Python 真分歧

指控：`write_float` 在整数浮点且 `abs()<1e16` 时用 `{:.1}`，否则用 Rust `Display`，而 Rust `Display`
**永不输出指数形式**；Python `repr` 在 `|x| ≥ 1e16` 或 `0 < |x| < 1e-4` 时切到指数形式。
`1e16 / 1e20 / 1e300 / 1e-7 / -1e-320` 五组文本与摘要均不同。
更要命的是 W6 的 13 组对拍向量里**一个浮点都没有**，门禁对此完全失明。

处置：

1. 重写 `write_float`，实现 Python `repr` 规则：`|x| ≥ 1e16` 或 `0 < |x| < 1e-4` 走指数形式，
   指数补齐至少两位并带符号（`1e+16`、`1e-05`）；否则走十进制并对整数值补 `.0`。
   非有限值输出 `NaN` / `Infinity` / `-Infinity`，与 `json.dumps(allow_nan=True)` 一致。
2. 向对拍向量补 6 组浮点，覆盖整数浮点、分数、大数、小数、以及**边界两侧**
   （`9.999999999999998e15` vs `1.0000000000000002e16`、`0.00010000000000000002` vs `9.999999999999999e-5`）
   与 `0.0/-0.0` 及次正规数 `5e-324`。

这条缺陷之所以能存在，根因是我自己的向量集设计漏洞——注释写着"数字"，实际只放了整数。
补的向量直接针对这个盲区。

## F2 / P1 / W4+W5 — invariant 1 的 allowed_truth_state 未实现

指控：合同 `resolved-no-present-runtime-facets` 同时声明
`forbidden_truth_state:[present]` 与 `allowed_truth_state:[indeterminate,not-applicable]`，
实现只查 `present`，因此 `resolved` 可在 runtime facet 上断言 `absent`。
全量扫描确认 **172,800** 个组合被合同禁止而被实现接受。

reviewer 的定性我完全接受：**静态 resolver 断言 runtime facet「不存在」与断言「存在」是同一种不诚实**，
而本阶段整个立意就是守这条线。

处置：把判定从「禁 present」改为「只允许 allowed_truth_state 白名单内的取值」，
并补两个测试分别锁定 `absent` 被拒、`not-applicable` 仍可表达。

## F4 / P2（升格当轮修）— 无任何测试把 Rust 判定钉在合同的约束字段上

reviewer 判 P2，但明确指出它是 **F2 得以存在的结构性原因**：`contract_parity.rs` 只比对 invariant 的
id 与顺序，没有任何东西把 `violations()` 的判定钉在合同的
`if / allowed_* / forbidden_* / required_* / minimum_coverage` 上。reviewer 在副本里协同改了合同语义
三次，`cargo-test` 三次全绿。

不修它，同类问题必然再犯，所以本轮一并修（同 root cause）。

新增 `crates/ctxpect-core/tests/invariant_semantics.rs`，9 个测试**从合同字段派生判定**：

- invariant 1：读 `allowed_truth_state` / `forbidden_truth_state` 与 `if.lifecycle_stage`，
  对 3 stage × 4 truth state 全枚举，白名单内必须接受、白名单外必须拒绝。
- invariant 2：读 `required_provenance` 与 `minimum_coverage`，对 6 provenance × 3 coverage 全枚举。
- invariant 3：读 `forbidden_truth_state` 与 `default_truth_state`，并验证声明的默认值本身可表达。
- invariant 4：读 `if.decision` / `required_claim_kind` / `required_fields`，
  逐个 required field 单独缺失都必须触发；`inconclusive` 在声明集合外必须不触发。
- invariant 5 / 6 / 7 / 9：分别读 `forbidden_truth_state` / `required_truth_state` +
  `forbid_synthetic_events` / `required_knowledge_status` + `silent_choice` / `forbidden_coverage`。
- `every_contract_constraint_key_is_accounted_for`：合同里出现任何**未被测试消费的约束键**即失败 ——
  这正是 invariant 1 的 allowlist 当初被漏掉的方式。

## 一并处置的其余 review 项

- **F3（部分）**：修掉两条「Rust 比 Python 更松」的分歧 —— 前导零（`01` / `-01`）与 `\u+0hh`。
  它们与我在 implement-1 报告里自陈的判据（"两侧可解析性不一致对 Receipt 验证是致命的"）直接冲突。
  剩余两条「Rust 更严」（超 i64 整数、`NaN`/`Infinity` 字面量）保持 fail-closed，
  但不再默认承诺，已在 `lib.rs` 顶部**显式声明为已知限制**——过去 crate 文档笼统承诺
  "byte-compatible"，那个承诺在这两点上并不成立。
- **F8**：`frozen_fixture_digests_reproduce` 的 `checked >= 16` 收紧为 `== 21`，
  静默丢失 fixture 不再可能通过。

## 门禁（本轮真实执行，九条）

| 名称 | exit | 摘要 |
| --- | ---: | --- |
| docs-structure | 0 | PASS |
| acceptance-validation | 0 | PASS |
| traceability-validation | 0 | PASS |
| corpus-validation | 0 | PASS |
| semantic-team-validation | 0 | PASS |
| validator-negative-tests | 0 | `Ran 148 tests`，`OK` |
| cargo-build | 0 | — |
| cargo-test | 0 | **58 passed**（47 → 58） |
| cargo-clippy | 0 | 0 error/warning |

`git diff --check` 通过。

## 反向验证（证明修复不是空转）

| 退回 | 结果 |
| --- | --- |
| F2 的判定改回「只禁 present」 | FAILED — `model-visible/absent is outside allowed_truth_state but was accepted` |
| F1 的 `write_float` 改回原实现 | FAILED — `canonical text diverged for {"big":[1e16,...]}` |
| 两者恢复 | 全部 58 passed |

即两条缺陷现在都被会失败的测试守住，而在修复前这两个测试并不存在。

## 仍未处置（记入 P2 台账）

- **F5**：`illegal_examples_are_rejected` 只断言违规非空，不校验「是哪条 invariant」。
  合同的 illegal example 行本身也没有承载期望 id 的字段，要修需同时改合同 schema。
- **F6**：`claim_from_example` 的默认值与 `||` 合并有潜在掩盖面（当前不影响判定，失效方向 fail-loud）。
- **F7**：`unknown_reason` 的「Indeterminate 必填」是注释承诺而非实现，
  **且 reviewer 标注 needs_owner_decision=true** —— 是否把它升为第 10 条不变量是产品决定，
  我不替 owner 决定，留待 owner 裁决。
- **F3 剩余**：超 i64 整数与 `NaN`/`Infinity` 字面量的输入侧差异，已文档化。
- **F9（P3）**：进普通 backlog。

## 预算

- product repair：1/3
- product rereview：本轮复审计第 1 次
- strategy reset：0/1

## 未做

- 未 commit、未 push。
- WP-02 其余部分（collect / resolve / doctor / cli）尚未开始。
