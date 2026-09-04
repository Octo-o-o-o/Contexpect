# Contexpect 需求文档独立评审任务

你是一个零上下文、只读的产品与技术方案 reviewer。不要修改任何文件，不要采信作者自述，只依据你实际读到的文件做判断。

必须完整阅读：

1. `/Users/wangyixiao/WorkSpace/ContextView/docs/research/2026-09-04-context-management-research-ledger.md`
2. `/Users/wangyixiao/WorkSpace/ContextView/docs/requirements/2026-09-04-contexpect-complete-product-requirements.md`

用户的验收意图：

- 项目是 Codex / Claude Code / Cursor / Grok Build 等工具的可视化上下文管理与核对工具。
- 必须完整讨论潜在用户，以及长期开启、偶尔开启、定期开启等使用方式解决的问题和回报。
- 不用功能子集或试验版本替代完整项目范围；完整产品做完后再进行用户验证。
- 必须完整吸收两份外部 AI 方案，但保留有价值部分、纠正错误和过度设计。
- 调研、反例、明确不做项和错误路线必须先持久化，再形成完整需求。

请重点审查：

1. 是否遗漏用户请求或关键 persona / cadence / reward。
2. 产品定义、六级状态、Expected/Observed/Unknown、Receipt 是否自洽。
3. 完整范围是否有不必要的过度设计；若有，指出可在不违背用户“完整做完”要求下删除或改成集成的具体项。
4. 是否把不可行或不可观察的能力写成了可验收事实。
5. 现有开源项目的 build/borrow/integrate 边界是否合理，是否还在重复造轮子。
6. 功能需求、工作包和验收合同是否可判定、互相覆盖且没有矛盾。
7. 隐私、安全、跨版本/跨 surface、设备与执行环境是否有缺口。
8. 命名结论是否有充分理由，是否把初步检索误写成正式可用性结论。

严重度：

- P0：根本无法实施或会导致严重安全/真实性问题。
- P1：用户核心要求遗漏、重大矛盾、关键功能无法验收。
- P2：重要但不阻断这份需求文档作为基线的问题。
- P3：文字和偏好建议。

输出格式：

```text
VERDICT: GREEN | RED
SUMMARY: 一段话

BLOCKERS:
- [P0/P1] 标题 — 文件:行号；证据；为什么阻断；最小修订建议

P2:
- [P2] 标题 — 文件:行号；证据；建议

OVERDESIGN_TO_REMOVE:
- 条目；理由；建议替代

GAPS:
- 条目；理由；应进入哪一节

WHAT_IS_STRONG:
- 最多 8 条
```

没有 P0/P1 才能给 GREEN。不要因为产品范围大就自动判 RED；只在范围不自洽、无法验收、明显重复造轮子或违背用户明确要求时判阻断。不要要求把完整产品改回功能子集。

