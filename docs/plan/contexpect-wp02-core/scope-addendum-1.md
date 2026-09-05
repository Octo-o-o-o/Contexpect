# Scope addendum 1 — 第 10 条不变量

> owner 于 2026-09-05 在 review-2 之后明确裁决：把「`Indeterminate` 必须带 reason」
> **升为第 10 条不变量**（对应 P2 台账 F7，该条原标注 `needs_owner_decision=true`）。

## 变更

冻结合同 `acceptance/claim-validity-matrix.yaml` 的 `invariants` 由 9 条增至 10 条，新增：

```json
{
  "id": "indeterminate-requires-unknown-reason",
  "if": {"truth_state": "indeterminate"},
  "required_fields": ["unknown_reason"],
  "required_vocabulary": "unknown_reason_codes"
}
```

置于既有九条之后，不改动它们的 id 与顺序。单一真值源仍是
`scripts/contexpect_contract.py` 的 `CLAIM_INVARIANTS`。

## 对验收合同的影响

- W4「9 条 invariant」→「10 条 invariant」。
- W5 的「Unknown 必须带 reason」自此有冻结 invariant 支撑，不再只是代码注释。

## 影响面（实施前只读盘点，事后复核一致）

- 合同的 `legal_examples` / `illegal_examples` 均非 `indeterminate`，不受影响。
- Rust 侧 10 处构造 `TruthState::Indeterminate` 的位置全部已带 `unknown_reason`，
  既有测试无需改写。
- `Claim::indeterminate()` 构造器本就强制 reason 参数。
- `acceptance/` 树摘要因合同变更而变化，属预期。

## 词表封闭性

`required_vocabulary` 指向 `unknown_reason_codes`。Rust 侧 `UnknownReason` 是封闭枚举，
新增测试断言它与冻结词表**逐字相等** —— 即词表外的理由**无法被表达**，而不只是「未被检查」。
