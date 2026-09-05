# WP-02 第一刀 · implement-1 报告

> 基线：`492fd5fe7183fc5517fb6a1ca811d8d5233eb1f3`（semantic-team GREEN 的干净 post-commit HEAD）。
> 验收合同：`docs/plan/contexpect-wp02-core/acceptance-contract.json`（W1–W8，先冻结后实施）。

## 范围

Cargo workspace + `crates/ctxpect-core`（claim 真值模型）+ `crates/ctxpect-schema`（规范化与摘要）。
**不含** collect / resolve / doctor / cli / store / daemon / UI —— 那些属于 WP-02 后续切片与 WP-03+。

选这一刀的理由：`acceptance/claim-validity-matrix.yaml` 里已经冻结了 7 条轴、20 个 unknown reason code、
4 个 reconciliation state、4 个 use-evidence kind、6 级 provenance precedence 与 **9 条可执行不变量**。
它们是产品的诚实性内核，也让"文档合同 ↔ 产品代码"的对账成为可判定的验收目标 ——
正是 semantic-team 阶段 review-4 那类"门禁绿但语义没落地"问题的结构性预防。

## 交付

### `crates/ctxpect-schema`（零第三方依赖）

- `sha256.rs`：SHA-256（FIPS 180-4）自实现，含 FIPS 向量与全部 padding 分支测试。
- `json.rs`：JSON value、parser、canonical serializer。canonical 形式与 Python
  `json.dumps(ensure_ascii=False, sort_keys=True, separators=(",", ":"))` 字节一致。
- `lib.rs`：`digest_value` / `digest_json_text`，等价于生成器的 `_digest_obj`。

零依赖是刻意的：项目承诺离线、不安装依赖，且内容摘要必须能从源码本身复现。

### `crates/ctxpect-core`

- `axes.rs`：10 个封闭枚举（7 条 claim-validity 轴 + reconciliation state + use-evidence kind +
  effect decision）。`from_wire` 对未知输入返回 `None` —— fail closed，绝不猜测或回落。
  另有 `Coverage::at_least`、`Provenance::precedence/outranks/is_native`、
  `LifecycleStage::is_runtime_facet` 等判定原语。
- `reason.rs`：20 个 unknown reason code。产品无法确定事实时必须在封闭词表里说明**为什么**，
  而不是静默降级。
- `claim.rs`：`Claim` 结构与 **9 条不变量的可执行判定**。`violations()` 按合同顺序返回违规项，
  **从不自动修复** —— 静默降级会掩盖缺陷，诚实是调用方的责任。

## 逐项验收

| 项 | 判定 | 证据 |
| --- | --- | --- |
| W1 构建 | ✅ | `cargo build/test/clippy --workspace` 均 exit 0；47 个 cargo 测试通过 |
| W2 七条轴 | ✅ | `contract_parity.rs::every_axis_matches_the_frozen_contract` 逐轴对账取值与顺序 |
| W3 code/state/kind/precedence | ✅ | `reason_codes_states_and_kinds_match_the_frozen_contract`、`provenance_precedence_matches_the_frozen_order` |
| W4 九条不变量 | ✅ | `every_frozen_invariant_has_an_implementation`（集合与顺序）、`legal_examples_are_expressible`、`illegal_examples_are_rejected` |
| W5 诚实性红线 | ✅ | `claim.rs` 内 15 个单元测试逐条覆盖，另有 `each_invariant_is_reachable_from_some_claim` 证明每条规则都可被触发 |
| W6 跨语言摘要一致 | ✅ | `python_parity.rs::canonical_form_and_digest_match_python`（13 组向量真实调用 Python 对拍）、`frozen_fixture_digests_reproduce`（21 个冻结 fixture 摘要逐个复现） |
| W7 不建空骨架 | ✅ | 只创建 `Cargo.toml` + 两个 crate；`apps/`、`packages/` 及其余 crate 目录均不存在 |
| W8 既有门禁零回归 | ✅ | 原六条门禁全绿；门禁表一致性校验强制 6 份 canonical 文档同步到九条 |

## 门禁（本轮真实执行，九条）

| 名称 | exit | 命令 |
| --- | ---: | --- |
| docs-structure | 0 | `python3 scripts/check_docs.py` |
| acceptance-validation | 0 | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | 0 | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | 0 | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | 0 | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | 0 | `Ran 148 tests`，`OK`，0 skip |
| cargo-build | 0 | `cargo build --workspace` |
| cargo-test | 0 | `cargo test --workspace`（47 passed） |
| cargo-clippy | 0 | `cargo clippy --workspace --all-targets`（0 error/warning） |

`cargo-clippy` 是本轮**新增的第九条门禁**：workspace 对 `clippy::all` 设了 `deny`，但
`cargo build` / `cargo test` 都不跑 clippy，那个 deny 原本形同虚设。一个不被执行的 deny 就是假绿，
正是本项目最反对的东西，所以把它变成真实门禁。修复过程中它抓出 15 处真实问题
（12 处 `from_str` 与 `std::str::FromStr` 同名易混淆 → 改名 `from_wire`；2 处 `useless vec!`；
1 处手写 `Iterator::find`）。

## 反向验证（证明对账不是空转）

篡改冻结合同后，parity 测试必须转红：

| 篡改 | 结果 |
| --- | --- |
| 把 `truth_state[0]` 改成 `PRESENT` | FAILED — `axis truth_state drifted` |
| 删掉一个 unknown reason code | FAILED — `unknown reason codes drifted` |
| 新增一条 Rust 侧未实现的 invariant | FAILED — `invariant set or order drifted` |
| 恢复原文件 | 9 passed |

## 过程中被抓到的真实缺陷

跨语言对拍测试抓到一个我自己的 parser bug：Python 的 `json.loads` 严格模式**拒绝**字符串内的字面
控制字符，而我的 parser 接受。这意味着同一份文档在两侧的可解析性不一致 —— 对 Receipt 验证是致命的。
修的是 parser（现在同样拒绝），不是绕过测试；并补了对应负例。

## 未做 / 边界

- 未 commit、未 push。
- WP-02 其余部分（collect / resolve / doctor / cli 六个命令、每坐标 60 static、
  Codex/Grok 12 oracle recipes、未知版本 exit 3）尚未开始。
- `crates/` 下其余 21 个计划 crate、`apps/`、`packages/` 均未创建。
- 产品运行时仍不可用；`ctxpect` 二进制不存在。
