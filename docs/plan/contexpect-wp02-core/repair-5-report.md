# WP-02 第一刀 · repair-5 报告（(a) + 描述纪律）

> 上游：`.octoworkflow/wp02-core-review-5.md`，verdict **RED**，4×P1。
> owner 裁决 **(a) + 描述纪律**。
> 本报告刻意只陈述**观察到的**结果，并逐条列出**未关闭**的项。

## 结果概览

review-5 列出六条绕过路径。本轮：**四条关闭，一条收窄但未关闭，一条未处理**。

| 路径 | 处置 | 复验结果 |
| --- | --- | --- |
| R5-02 渲染章节归属可颠倒 | 章节成员集合对账 | **FAILED**（转红，附实际/期望集合差异） |
| R5-03 `lib.rs` 可手写反话 | `lib.rs` 的 `//!` 行禁止出现边界词汇（词边界匹配） | **FAILED** — `lib.rs:8 hand-writes boundary prose ("floats")` |
| R5-05 可静默删除未列举的分歧行 | 表内容精确钉死为显式清单 | **FAILED** — `the boundary table changed` |
| R5-06 `include_str!` 可改指别处 | 断言 `lib.rs` 含该 include 字面量 | **FAILED** — `lib.rs must include the generated boundary document` |
| **R5-01 `note` 自由散文可写假话** | **收窄，未关闭**（见下） | **仍然 PASS** |
| R5-07 六份文档硬编码「九条门禁」 | 未处理（P3） | — |

## R5-01：收窄，不是关闭

`note: &'static str`（每行任意散文）改为 `Rationale` 封闭枚举（7 个变体）。
`Rationale::permits(we_accept, python_accepts)` 约束**哪个变体可被哪种行为模式引用**——
例如 `SupportedSubset` 不能用于被拒绝的行，`InvalidJsonSyntax` 不能用于 Python 接受的行。

**但每个变体的 `as_prose()` 仍是硬编码字符串，改成假话仍然全绿。** 我复验过：
把 F10 那句原话写进 `FloatLiteral` 变体并重新渲染 → 9 passed。

所以准确的说法是：**可写假话的面积从「每行任意散文」缩小到「7 条固定句子」，
而不是消除了它。** 「这句话是否为真」不是这里的测试能判定的。

这一点已写进 `boundary.rs` 与 `boundary_doc.rs` 的模块文档的
「What they do not observe」小节，并记入 P2 台账，不作为已关闭项。

## 描述纪律的落实

owner 裁决的第二半，也是我认为比 (a) 更重要的一半。四次同型缺陷都是自述超出机制。
本轮起：

1. **两个模块的文档各有一节「What these tests observe」与「What they do not observe」**，
   后者逐条列出未覆盖面，包括 R5-01 这条明知未关闭的。
2. **不再写推断性断言**。此前用过的「没有中间地带」「从结构上终结」「文档不再手写」
   一律不用；只写可由某个测试观察到的事实。
3. `lib.rs` 的 crate 概述里删除了 "byte-compatible" 措辞 ——
   那正是 repair-1 埋下 F10 的地方，且现在由 `lib_rs_carries_no_hand_written_boundary_prose` 强制。
4. 台账里 R5-01 记为「收窄，未关闭」，不写成已修。

## 一处我自己的误报

`lib.rs` 守卫初版用朴素子串匹配，把 "**accept**ance fixture" 判成了边界声明。
改为词边界匹配，并补了 `word_matching_does_not_fire_on_substrings`
（断言 "acceptance fixture" 不触发、"floats are accepted here" 触发）。

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
| cargo-test | 0 | **77 passed**（71 → 77） |
| cargo-clippy | 0 | 0 error/warning |

`acceptance/` 两次生成一致 `091eedc5…`（未变，本轮不涉及合同内容）；`git diff --check` 通过。

## 代价（review-5 要求的工程判断，我的观察）

- `boundary.rs` + `boundary_doc.rs` 合计 654 行。review-5 估计 (a) 约 40 行、
  且总量会比原先 190 行更短 —— **实际没有更短**。原因是我把「显式列出未覆盖面」
  与两个防误报测试也算了进去。若只算新增约束逻辑，接近它的估计。
- 新的失效模式：忘记重新生成 `BOUNDARY.md`。由 `documentation_is_rendered_from_the_table`
  捕获，错误信息直接给出重新生成的命令。
- 推广到后续切片：`lib.rs` 词汇黑名单是**按 crate 手写**的，推广时每个 crate 都要维护一份。
  这是这个方案最不优雅的部分，我没有更好的做法。

## 未关闭项（P2 台账）

R5-01（收窄未关闭）、R5-07（P3）、以及既有的 F5 / F6 / F9 / F13 / F18。

## 预算

- product repair：**5/5**（owner 追加至 5）
- product rereview：5/5 已用尽
- strategy reset：1/1

## 未做

- 未 commit、未 push。
- 本轮交付物未经零上下文独立复审。
- WP-02 其余部分（collect / resolve / doctor / cli）尚未开始。
