# WP-02 第一刀 · repair-3 报告

> 上游：`.octoworkflow/wp02-core-review-3.md`，verdict **RED**，1×P1、0×P0。
> reviewer 独立确认 repair-2 的实质修复全部成立：浮点移除彻底（类型层面不可表达，
> 4,109 组自建向量零分歧）、第 10 条不变量正确（33,177,600 组合穷举 misfire 0 / 误杀 0）、
> F2/F4/F8/F12 真实闭合、18 处产品代码变异全部被特定测试捕获。
> **唯一空转就是 F15。**

## F15 / P1 / W6 — 守卫本身是空转，且这是同一 root cause 的第二次出现

指控：`documented_boundary_matches_both_implementations` **从不读取 crate 文档**，
用的是测试自带的硬编码表。reviewer 把 `lib.rs` 的三条边界声明改成彻底的假话
（包括把 review-2 判定为不实承诺的 "byte-compatible with Python, floats included" 原句写回），
**63 个测试全绿**。

据此三处受管陈述不实：repair-2 报告里我写的「文档再想漂成一句做不到的承诺，测试就会红」、
测试自身的注释、以及台账 F14 行。

**reviewer 的定性我完全接受，而且它比缺陷本身更重要**：这是 F10
「写下比代码更强的承诺」这一 root cause 的**第二次出现** —— repair-1 在 crate 文档层面犯，
repair-2 在「我建了守卫」这一层再犯。声称的保护强度超过实际，换了个楼层而已。
reviewer 建议计入停滞判断，这个建议成立（见下方预算）。

处置：把边界表的**每一行绑定到 crate 文档里主张它的那句话**。

- 表结构由 `(document, python_accepts)` 改为 `(document, python_accepts, claim)`，
  `claim` 是文档中对应的声明串。
- 用 `include_str!("../src/lib.rs")` 读入文档，逐行断言该声明**仍在文档中**。
- 另断言 byte-compatibility 的**限定语**（"Over the supported subset it stays"）仍在场，
  且 "floats included" / "floats are supported" 这类被撤回的措辞**不得重现**。

反向验证（复现 reviewer 的三种攻击）：

| 攻击 | 结果 |
| --- | --- |
| 把 "floats included" 写回文档 | FAILED — `the docs dropped the qualifier on the byte-compatibility claim` |
| 删掉「Integers outside `i64` are rejected」一条 | FAILED — `the crate docs no longer state …, which {"v":9223372036854775808} relies on` |
| 把「Floating point is not supported」改成 "fully supported" | FAILED — `the crate docs no longer state "**Floating point is not supported.**"` |
| 全部恢复 | 4 passed |

一处自我修正：我最初写的撤回串检查过粗，把文档里**准确的**那句
"Over the supported subset it stays byte-compatible…" 也匹配掉了，测试立刻报红。
改为「限定语必须在场 + 撤回措辞不得重现」。这次是守卫先抓住了我。

## F16 / P2（升格当轮修）— `default_truth_state` 断言过弱

reviewer 发现把合同的 `default_truth_state` 改成 `absent` 时测试不转红 ——
原断言只检查「该默认值不被禁止」，而「不被禁止」弱于「是合同指名的那个 fallback」。

与 F15 同属「断言强度低于其自称覆盖的语义」，故本轮一并修：现在断言默认值不在 forbidden 集合内、
确为 `Indeterminate`（未经证实的归因，诚实 fallback 只能是它）、且完整可表达。

反向验证：把生成器的 `default_truth_state` 改为 `absent` 并重新生成合同 →
`an unproven attribution's honest fallback is indeterminate, not absent`，转红。

## F17 / P2（升格当轮修）— 受管注释仍称 six required gates

`scripts/check_docs.py:330` 的 docstring 写「all six required gates」，实际已是 9 条。
这本身就是「受管文档与事实矛盾」，且修复成本接近零，故一并修：
改为「every REQUIRED_GATES entry」——不再硬编码会随时变化的数字。

## 未修

- **F18 / P2**：深度 ≥10000 的嵌套文档使 Rust `parse` 栈溢出 abort（exit 134），
  Python 抛可捕获的 `RecursionError`。本切片无不受信输入路径，属资源上限，记入台账。
- F5 / F6 / F9 / F13 / F14：维持 P2，本轮不修。

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
| cargo-test | 0 | 63 passed |
| cargo-clippy | 0 | 0 error/warning |

两次生成 `acceptance/` 树摘要一致：
`091eedc5010ab6ae637d595e30ae21cbed09608075bd2447bf3b794dc202d734`（与上轮相同 ——
本轮只改测试与文档，未动合同内容）。`git diff --check` 通过。

## 预算与停滞判断

- product repair：**3/3 — 已用尽**
- product rereview：本轮复审将计第 4 次（上限 3，**已超**）
- strategy reset：0/1
- root cause「承诺强于实现」：修复次数 **2/2 — 已达上限**

按合同，本轮复审若再 RED，**不得继续自动返工**，须停下来交 owner 决策。

## 未做

- 未 commit、未 push。
- WP-02 其余部分（collect / resolve / doctor / cli）尚未开始。
