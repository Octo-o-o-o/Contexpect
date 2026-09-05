# Deferred P2 ledger — contexpect-docs / semantic-team / c2

> 本任务唯一 P2 台账。默认不当轮修；最终交付前只做一次 P2 sweep。

| id | acceptance item | 来源 | 摘要 | 状态 |
| --- | --- | --- | --- | --- |
| P2-c2-1 | ST2 | c2 review-1 | `negotiation.unsupported` / `unknown` 列表不要求包含 `required_capability` 本身。coze 可声明 `required_capability="tool-invocation"`、`status="not-applicable"`，而 `unsupported` 仍只列 `["instructions"]` 且不报错。整体仍是诚实的 unsupported 叙事。**c2 review-2 判定该严重度被实质低估**（与 C2-N1 同 root cause）。 | **已于 repair-3 关闭** |
| P2-c2-2 | ST2 | c2 review-1 | `deepseek-harness` 在 fixture 中作为"诚实 Unknown"范例，但 `registry_capability_status("deepseek-harness","instructions")` 返回 `required-supported`，该诚实性无 registry 依据、validator 无法强制。属 fixture 与 registry 的语义错配，非 validator 缺陷。 | 未修 |
| P2-c2-3 | ST2 | c2 review-3 | 设计注记：`declared_capabilities` 是集合语义，若将来单个 payload 出现基数 > 1 的 intent 集合，绑定会放宽为"属于任一 intent 所需能力"。当前所有 positive scenario graph 钉死 `canonical_intents: 1`，不可达。 | 前瞻，未修 |
| P2-c2-4 | ST2 | c2 review-3 | 设计注记：`declared_capabilities` 为空时静默跳过 capability 绑定，但此时 payload 因缺 canonical intent 已被拒，不产生假绿。 | 不产生假绿，未修 |
| P2-c2-5 | ST4 | c2 review-3 | `_validate_signature_object`（`scripts/contexpect_semantic_team.py:3214-3223`）不校验 `signatures[].live_tested`，可置 `true` 而无报错。判 P2 依据：`grep -rl '"live_tested": true' acceptance/` 命中数为 0（无实际虚假宣称）；承载 live-test 宣称的三个面（fixture 记录、oracle 对象、contract）均已强制；无受管文档矛盾。**最终 P2 sweep 时补一行判断，成本极低。** | 未修 |
| P2-c2-6 | ST2 | c2 review-3 | `scripts/contexpect_semantic_team.py:2936` 对列表内嵌套 list 抛 `TypeError` 而非落 `malformed-schema`。fail-closed，且输入超出声明 grammar。 | 未修 |

来源报告：`.octoworkflow/docs-semantic-team-c2-review-1.md` §7、`.octoworkflow/docs-semantic-team-c2-review-3.md` §9。

> P2-c2-2 经 c2 review-3 复核，严重度**未被低估**，维持 P2：`negotiation_states_allowed("required-supported")` 显式允许降级到 unknown，deepseek 的 Unknown 是合法诚实降级，剩余的只是范例选择上的教学性瑕疵。
