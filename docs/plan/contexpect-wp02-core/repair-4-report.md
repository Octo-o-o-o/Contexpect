# WP-02 第一刀 · repair-4 报告（1(b) + 策略重置）

> 上游：`.octoworkflow/wp02-core-review-4.md`，verdict **RED**，1×P1（R4-01）。
> owner 于第二次 STOP_FOR_OWNER 检查点裁决 **1(b) + 3**：
> 1(b) 追加 repair 预算仅订正措辞；3 动用 strategy reset（0/1 → 1/1）改为生成关系。

## 为什么需要策略重置

同一 root cause 已出现三次，每次的修法都是「更强的断言」，而每次那个断言的**自述又超出它的实际**：

| 层级 | 轮次 | 声称 | 实际 |
| --- | --- | --- | --- |
| crate 文档 | repair-1 | byte-compatible, floats included | 浮点数字生成分歧 |
| 守卫 | repair-2 | 文档漂移会被测试抓住 | 测试根本不读文档 |
| 守卫已闭合 | repair-3 | 文档再想漂成做不到的承诺就会红 | 只有删除类成立 |

review-4 的四类攻击（语义反转、单行反转、限定语挪位、整段替换成假话）全部绕过了 repair-3 的
子串绑定，其中「整段替换」下 `cargo test --workspace` 仍 63 passed —— 与 review-3 当初的观测一字不差。

问题不在断言强度，在于**手写的描述这一环节本身**。只要文档由人写、由测试近似地检查，
描述就有余地比实现强。

## 处置 3：文档与实现改为生成关系

新增 `crates/ctxpect-schema/src/boundary.rs`：

- `BOUNDARY: &[BoundaryCase]` 是边界的**唯一**陈述，每行含 `document` / `we_accept` /
  `python_accepts` / `note`。
- `render_markdown()` 把它渲染成文档章节。
- `crates/ctxpect-schema/BOUNDARY.md` 是渲染产物，由
  `cargo run -p ctxpect-schema --example render_boundary` 生成。
- `lib.rs` 通过 `#![doc = include_str!("../BOUNDARY.md")]` 把它作为自己的文档。

新增 `crates/ctxpect-schema/tests/boundary_doc.rs`，**三方互锁**：

| 链路 | 测试 | 断开时 |
| --- | --- | --- |
| 文档 ↔ 表 | `documentation_is_rendered_from_the_table` | 文档任何改动（含语义反转）与渲染不符 → 红 |
| 表 ↔ 我方行为 | `every_row_states_our_actual_behaviour` | 表说的与 `parse` 实际不符 → 红 |
| 表 ↔ Python 行为 | `every_row_states_pythons_actual_behaviour` | 表说的与 `json.loads` 实际不符 → 红 |

外加三条完整性守卫：`the_render_mentions_every_row`（行不能只在表里不在文档里）、
`divergences_are_labelled_as_divergences`（真分歧不能被写成「两侧一致」，且三个章节标题必须在场）、
`the_table_retains_every_case_earlier_reviews_found`（前几轮发现的七个案例不得被悄悄删除）。

**关键差别**：文档不再是需要被检查真实性的散文，而是表的一个确定性投影。
想让文档说假话，必须改表；改表就会与实测行为冲突。没有中间地带了。

同时**删除**被取代的 `documented_boundary_matches_both_implementations`（75 行）——
它正是 R4-01 的载体。

## 反向验证：复现 review-4 的四类攻击

| 攻击 | 旧守卫 | 现在 |
| --- | --- | --- |
| 保留关键词、反转语义（`Rejected by both` → `Accepted by both`） | GREEN（绕过） | **FAILED** — `BOUNDARY.md is out of date` |
| 整段文档换成「Floats are fully supported」 | GREEN（绕过） | **FAILED** — `BOUNDARY.md is out of date` |
| 改表说假话（声称我们接受浮点）并重新渲染 | — | **FAILED** — `the table says we_accept=true for {"v":1.0}, observed false` |
| 删掉一行已发现的差异并重新渲染 | — | **FAILED** — `{"v":01} was found by an earlier review and must stay documented` |
| 全部恢复 | — | 7 passed |

第三、第四类是新方案才有的防护：旧方案下「改表」这个动作根本不存在。

## 处置 1(b)：订正失实的受管陈述

- 台账 `DEFERRED-P2.md` 的 **F14 行**：原称「由测试逐条核对文档」，两度失实
  （review-3 指出测试不读文档；review-4 指出子串绑定只挡删除类）。已改为准确描述当前的生成关系。
- 台账 **F15 行**：原称 repair-3 已关闭，实际只闭合了删除类攻击。已如实改写。
- `python_parity.rs` 的两处注释随被取代的测试一并删除。
- 新增台账行 R4-01（已关闭）、F19、F20。

## 一并订正的同类缺陷（review-4 记为 P2）

- **F19**：`scripts/contexpect_contract.py:739-740` 的 SoT 注释称 "six required gates"，
  紧接其下的 `REQUIRED_GATES` 实为 9 条。改为不写数字，并注明「条数已变过两次，
  每次残留的提法都成了缺陷」。
- **F20**：`docs/process/implementation-plan.md` 称完成定义「六条」却列出九条、
  称 WP-02 追加「两条」实为三条。改为以 `REQUIRED_GATES` 为准、本文件不复述数字。

这两条与 F17 同属「受管文档硬编码会过期的数字」。全仓 grep 确认受管文件中已无残留
（`.octoworkflow/` 下的历史评审报告除外，那些是当时的如实记录）。

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
| cargo-test | 0 | **71 passed**（63 → 71） |
| cargo-clippy | 0 | 0 error/warning |

`acceptance/` 两次生成一致 `091eedc5…`（未变 —— 本轮不涉及合同内容）。`git diff --check` 通过。

## 预算

- product repair：**4/4**（owner 追加至 4）
- product rereview：4/4（owner 追加至 4，已用尽）
- strategy reset：**1/1**（本轮动用）
- root cause「承诺强于实现」：以结构性方案终结，不再依赖断言强度

## 未做

- 未 commit、未 push。
- 本轮交付物（生成关系 + 三方互锁）**未经零上下文独立复审**，rereview 预算已用尽。
- WP-02 其余部分（collect / resolve / doctor / cli）尚未开始。
