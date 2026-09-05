# Deferred P2 ledger — contexpect-wp02 / core-truth-model

> 本任务唯一 P2 台账。默认不当轮修；最终交付前只做一次 P2 sweep。
> 来源：`.octoworkflow/wp02-core-review-1.md` §7、`.octoworkflow/wp02-core-review-2.md`。

| id | 验收项 | 摘要 | 状态 |
| --- | --- | --- | --- |
| F3 | W6 | **本行经 review-2 指出描述失实，已于 repair-2 订正。** 原描述称余项只有两条且方向都是「Rust 更严」，实际不成立：另有尾随小数点（`1.` / `0.e5`，Rust 更松）与孤立代理项（Python 接受、Rust 拒绝）。前者已随浮点移除一并修复；后者是语言级差异。当前准确边界见下方 F13 与 `crates/ctxpect-schema/src/lib.rs` 顶部。 | 已订正，余项见 F13 |
| F4 | W4 | 无测试把 Rust 判定钉在合同 invariant 约束字段上。repair-1 新增 `invariant_semantics.rs` 关闭；review-2 以 16 组协同变异复核，15 组转红。 | **已关闭** |
| F5 | W4 | `illegal_examples_are_rejected` 仅断言违规非空，不校验命中的是哪条 invariant。合同的 illegal example 行也没有承载期望 id 的字段，要修需同时扩展合同 schema 与生成器。 | 未修 |
| F6 | W4 | `contract_parity.rs` 的 `claim_from_example` 对 effect claim 自动补 `experiment_id`/`contract_digest`，并用 `\|\|` 合并合同里以 `∧` 定义的 invariant 7 条件。当前两处都不影响判定，失效方向 fail-loud。 | 未修 |
| F7 | W5 | 「`Indeterminate` 必须带 reason」原为注释承诺而非实现。**owner 于 2026-09-05 裁决升为第 10 条不变量**，见 `scope-addendum-1.md`，已于 repair-2 落地。 | **已关闭（owner 裁决）** |
| F8 | W6 | `frozen_fixture_digests_reproduce` 下限过松。repair-1 收紧为 `== 21`；review-2 复核增删双向均转红。 | **已关闭** |
| F9 | — | P3，进普通 backlog。 | 未修 |
| F12 | W4 | `cannot_override` 出现在 `CONSUMED` 白名单却无任何测试读取——键被豁免而非被检查。与 F4 同类。已于 repair-2 新增 `higher_provenance_honours_its_cannot_override_scope` 关闭。 | **已关闭** |
| F13 | W6 | **浮点不被支持**（repair-2 起 fail-closed）。达成与 Python 字节一致需要 Ryu 级最短表示 + round-half-to-even：Rust 的最短表示在进位平局上远离零，Python 取偶，`1000000000000000.25` 两侧分别渲染为 `…0.3` 与 `…0.2`。当前所有冻结制品浮点数为 0，产品不需要；将来 Context Effect Lab 的统计量可能需要，届时必须实现该算法而非近似。 | 未修（显式债务） |
| F14 | W6 | 其余两侧输入差异，均 fail-loud：超出 `i64` 的整数（Python 任意精度）、`NaN`/`Infinity`/`-Infinity` 字面量（Python 接受）、孤立代理项（Python 接受，Rust `String` 无法表示——语言级差异，不可消除）。已在 crate 文档准确声明。**本行两度失实**：review-3 指出「由测试逐条核对文档」不成立（测试当时不读文档）；review-4 指出 repair-3 的子串绑定只挡得住删除类漂移，语义反转可绕过。**repair-4 起改为生成关系**：`BOUNDARY` 表是唯一真值源，crate 文档由它渲染，每行对两侧实测对拍——文档、表、行为三方互锁。 | 未修（受生成关系保护） |
| F15 | W6 | `documented_boundary_matches_both_implementations` 使用自带硬编码表、从不读取 crate 文档，文档可被改成假话而测试全绿。**与 F10 同一 root cause 的第二次出现**。repair-3 的子串绑定只闭合了删除类攻击，语义反转、限定语挪位、整段替换均可绕过（review-4 / R4-01）。**repair-4 以策略重置关闭**：文档不再手写，由 `BOUNDARY` 表渲染。 | **已关闭（结构性）** |
| F16 | W4 | invariant 3 的 `default_truth_state` 断言过弱：只检查「不被禁止」，改成 `absent` 不转红。已于 repair-3 收紧为「必须是合同指名的 fallback」。 | **已关闭** |
| F17 | W8 | `scripts/check_docs.py:330` docstring 仍称「all six required gates」，实为 9 条。已于 repair-3 改为不硬编码数字。 | **已关闭** |
| F18 | W6 | 深度 ≥10000 的嵌套文档使 Rust `parse` 栈溢出 abort（exit 134），Python 抛可捕获的 `RecursionError`。本切片无不受信输入路径，属资源上限而非语义分歧；若将来 parser 接受外部输入（import / adapter），必须改为显式深度限制并 fail-loud。 | 未修 |
| R4-01 | W6 | repair-3 的文档守卫是纯子串存在性检查，被引用串多为无谓语关键词，四类语义漂移攻击全部绕过；且其自身注释与台账把它描述得比实际强——「承诺强于实现」的第三次出现。**已于 repair-4 关闭**：owner 裁决 1(b)+3，措辞订正 + 策略重置为生成关系。 | **已关闭** |
| F19 | W8 | `scripts/contexpect_contract.py:739-740` 的 SoT 注释称 "six required gates"，紧接其下的 `REQUIRED_GATES` 实为 9 条。与 F17 同档。已于 repair-4 一并订正。 | **已关闭** |
| F20 | W8 | `docs/process/implementation-plan.md` 称完成定义"六条"却列出九条、称 WP-02 追加"两条"实为三条（漏 cargo-clippy）。与 F17 同档。已于 repair-4 一并订正。 | **已关闭** |
| R5-01 | W6 | `Rationale::as_prose()` 的固定句子**不受任何测试约束**，改成假话仍全绿（复验确认）。repair-5 把可写假话的面积从「每行任意散文」收窄到「7 条固定句子」，**未消除**。「一句话是否为真」不是此处的测试能判定的。已在 `boundary.rs` / `boundary_doc.rs` 的「What they do not observe」明确声明。 | **收窄，未关闭** |
| R5-02 | W6 | `render_markdown()` 章节归属可颠倒。repair-5 加入章节成员集合对账。 | **已关闭** |
| R5-03 | W6 | `lib.rs` 可手写 `//!` 反话。repair-5 加入词汇黑名单（词边界匹配）。 | **已关闭** |
| R5-05 | W6 | 未被清单列举的真分歧行可静默删除。repair-5 把表内容精确钉死。 | **已关闭** |
| R5-06 | W6 | `include_str!` 可改指别处。repair-5 断言 `lib.rs` 含该 include 字面量。 | **已关闭** |
| R5-07 | W8 | 六份受管文档硬编码「九条门禁」。P3，未处理。 | 未修 |

## 无待 owner 决策项

F7 已由 owner 裁决并落地。当前台账其余条目均不需要 owner 介入。

## 关于 R5-01 的说明

它是本任务里唯一一条**明知未关闭而保留**的条目。保留而非继续加固的理由：
「`as_prose()` 这句话是否为真」需要机器判断自然语言真值，此处的测试做不到。
能做的是把面积压到最小（7 条固定句子）并**如实声明**这一点 —— 后者由 owner 裁决的
描述纪律强制。若未来有更好的做法（例如让句子本身也从行为派生），再行处理。
