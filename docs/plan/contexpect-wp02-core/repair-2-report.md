# WP-02 第一刀 · repair-2 报告

> 上游：`.octoworkflow/wp02-core-review-2.md`，verdict **RED**，2×P1、0×P0。
> 另含 owner 于同期下达的裁决：把 F7 升为第 10 条不变量（见 `scope-addendum-1.md`）。
> reviewer 独立确认 repair-1 的 F2 已闭合且无误杀（8,294,400 组合零 false accept / 零 false reject）、
> F4 实质关闭（16 组协同变异 15 组转红）、F8 属实、58 个测试无一空转。

## F10 / P1 / W6 — 浮点：repair-1 只修了格式，且写下了一条更强的不实承诺

指控：repair-1 重写的是浮点**格式**（指数切换点、补零、`.0` 后缀），**数字生成**仍用 Rust 最短表示。
落在进位平局上时 Rust 取「半进位远离零」、Python 取「半进位取偶」，365,865 个探针中 15,794 条分歧。
更要紧的是 repair-1 在 `lib.rs` 写下的 "byte-compatible, **floats included**" 是**新引入的不实承诺**。

我自行复现确认：`1000000000000000.25` → Rust `1000000000000000.3` / Python `1000000000000000.2`；
`2⁻²⁵` → Rust `…313` / Python `…312`。这是最短表示算法的差异，格式层面修不掉。

**这一条我认得比其他都重。** 在一个立意就是「不夸大已知」的项目里，修缺陷时写进一条更强的假承诺，
比原缺陷本身更糟。

处置：**移除浮点支持，改为 fail-closed。**

- `Value` 删除 `Float` 变体，`write_float` 整个删除。
- `parse` 遇到任何浮点字面量直接拒绝，错误信息明确指出原因。
- 与 Python 达成字节一致需要 Ryu 级最短表示 + round-half-to-even；我刚证明了自己在这上面会出错，
  而 `acceptance/` 下浮点数为 0。**诚实的「不支持」胜过错误的「支持」**，债务显式记入台账。

这同时消掉了 F11 的尾随小数点（`1.` / `0.e5` 都是浮点字面量）。

## F11 / P1 / W6 — parser 第三、第四类分歧，且文档把方向说反了

指控：1,047 条 Rust 比 Python **更松**（尾随小数点），正是 `json.rs` 自陈必须避免的方向；
263 条 lone surrogate 是 Python 接受、Rust 拒绝，而 repair-1 的文档明写
「lone surrogates 两侧都拒绝」「只有两处更严」——**两句都是错的**。

自行核验：Python `json.loads('"\\ud800"')` 接受并产生一个 Rust `String` 无法表示的 str；
`1.` / `0.e5` / `{"k":1.}` / `[1.]` Python 全部拒绝。指控属实。

处置：

1. 尾随小数点随浮点移除一并被拒。
2. **重写 crate 文档的边界声明**，逐条准确列出：浮点不支持（含具体反例与原因）、
   超 i64 整数、`NaN`/`Infinity`、孤立代理项（并说明这是语言级差异而非疏漏）；
   以及两侧一致拒绝的项。
3. 新增 `documented_boundary_matches_both_implementations`：把文档里的**每一条声明**
   同时对我方行为与 Python 实际行为核对。文档再想漂成一句做不到的承诺，测试就会红。

## owner 裁决 — 第 10 条不变量

`indeterminate-requires-unknown-reason`：`truth_state == indeterminate` 时必须带
`unknown_reason`，取值限于冻结词表 `unknown_reason_codes`。

落地于生成器 `CLAIM_INVARIANTS`（单一真值源）→ 合同 → Rust `InvariantId` 与判定 →
语义对账测试。新增测试断言 Rust 的 `UnknownReason` 封闭枚举与冻结词表**逐字相等**，
即词表外的理由无法被表达，而不只是未被检查。

范围变更记入 `scope-addendum-1.md`，验收合同 W4（9→10 条）与 W5 同步更新。

影响面与实施前盘点一致：合同 examples 均非 indeterminate；Rust 侧 10 处构造全部已带 reason，
既有测试无需改写。

## F12 / P2（升格当轮修）— invariant 8 的 cannot_override 被白名单却无人读取

reviewer 发现 `cannot_override` 出现在 `CONSUMED` 白名单里却没有任何测试读它 ——
键被「豁免」而非被「检查」。这与 F4 是同一类结构性问题，且正是我上一轮亲手加的白名单，
所以本轮一并修：新增 `higher_provenance_honours_its_cannot_override_scope`，
读取声明的 override 范围与条件，并验证任何 provenance 都不豁免。

## 我自己在反向验证中抓到的一个空转

移除浮点守卫后测试**仍然全绿** —— 因为整数扫描停在 `.` 处，外层 trailing-content 检查兜住了拒绝行为。
守卫是冗余防御，但测试无法区分它在不在，等于没覆盖。

已修：断言错误信息必须指出是浮点，而不只是「被拒绝」。移除守卫后现在会红
（`"1.0" was rejected as "trailing content after JSON value", not as a float`）。

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
| cargo-test | 0 | **63 passed**（58 → 63） |
| cargo-clippy | 0 | 0 error/warning |

两次生成 `acceptance/` 树摘要一致：`091eedc5010ab6ae637d595e30ae21cbed09608075bd2447bf3b794dc202d734`
（相对上轮变化，源于第 10 条不变量入合同，属预期）。`git diff --check` 通过。

## 反向验证

| 退回 | 结果 |
| --- | --- |
| 删除第 10 条判定 | FAILED（可达性守卫 + 语义对账双双转红） |
| 删除浮点守卫 | FAILED — `"1.0" was rejected as "trailing content", not as a float` |
| 合同词表加一个 Rust 未实现的 reason | FAILED — `unknown reason vocabulary drifted` |
| 全部恢复 | 63 passed |

## 台账订正

reviewer 指出 **P2 台账 F3 那一行的描述本身已不成立**（余项不止两条、方向不止「更严」）。
已随本轮订正，见 `DEFERRED-P2.md`。

## 预算

- product repair：2/3
- product rereview：本轮复审计第 3 次
- strategy reset：0/1

## 未做

- 未 commit、未 push。
- 浮点支持（需 Ryu 级实现）记入台账，未做。
- WP-02 其余部分（collect / resolve / doctor / cli）尚未开始。
