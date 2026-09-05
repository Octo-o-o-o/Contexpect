# semantic-team c2 · repair-2 报告

> 上游：`.octoworkflow/docs-semantic-team-c2-review-1.md`，verdict **RED**，3×P1、0×P0。
> 该轮为首次 RED，三条全部 `in_scope=true` / `needs_owner_decision=false`，按合同自动返工一次。
> reviewer 同时确认 repair-1 的 B2–B6、B9 属真实修复且非空转，正向控制项全绿。

## 逐项处置

### c2-B1 / P1 / ST1 — `equivalence_basis` 只有黑名单，白名单是零调用死常量

指控：`EQUIVALENCE_BASES` 定义后全仓引用数为 1（即定义本身）；两处校验只判黑名单，
所以 `text-hash-equality` / `sha256-digest-match` / `""` / `None` / `123` 带着 `equivalent: true` 零报错。

处置：把已声明的封闭枚举真正接上。`_validate_projection` 与 `collect_violations` 的
equivalence binding 循环各新增一条 `elif basis not in EQUIVALENCE_BASES` 分支，落
`malformed-schema` + `byte-copy-as-alignment` —— 未经声明的依据不能支撑等价主张。

复现验证（reviewer 原始用例，全部由 REJECTED 取代原先的零报错）：

| 取值 | binding 侧 | projection 侧 |
| --- | --- | --- |
| `text-hash-equality` | REJECTED | REJECTED |
| `sha256-digest-match` | REJECTED | — |
| `byte-identical-copy` | REJECTED | — |
| `文件哈希相等` | REJECTED | — |
| `""` / `None` / `123` | REJECTED | REJECTED |

两个已声明依据 `canonical-intent-semantics` / `documented-native-mapping` 仍然通过。

### c2-B2 / P1 / ST7 — `upload_consented` 不受约束且不进任何 digest

指控：成员的 acknowledgement 覆盖的是"看过什么预览"，不覆盖"是否同意上传"；
上传同意位可在成员确认之后被单方翻转，无 digest 移动、无 violation。这落在隐私红线上。

处置：把 `upload_consented` 与 `consent_status` 一并纳入 `disclosure_binding_digest` 的绑定体。
acknowledgement digest 自此覆盖成员做出的**决定**，而不只是它看到的预览。

复现验证（两个方向都测，因为 ST7/ST8 基线本身就是 `False`，只测单向会漏）：

| 基线 | 篡改 | ST7-pos | ST8-pos |
| --- | --- | --- | --- |
| `consented=False` | 事后翻为 `True` | REJECTED | REJECTED |
| `consented=True`（完整重绑后合法） | 事后翻回 `False` | REJECTED | REJECTED |
| `consented=True` | 事后把 `consent_status` 改为 `denied` | REJECTED | REJECTED |

均落 `disclosure-digest-mismatch` + `disclosure-missing`。
另直接验证 digest 函数本身：`upload_consented` 与 `consent_status` 任一变化都会使
`disclosure_binding_digest` 移动。

### c2-B3 / P1 / ST8 — rollout `percent` 与 `stages[0]` 比较而非与声明的 `cohort`

指控：`cohort=ga` + `percent=10` 零报错 —— rollout 自称处在 100% 的 ga 阶段却上报 10% 暴露面。
repair-1 报告的自述（"percent 必须等于起始 cohort 的 percent"）与代码取 `stages_list[0]` 不符。

处置：按 `rollout["cohort"]` 在 stages 中查找对应 stage，与它的 `percent` 比较；注释同步改为
"the percent of the cohort the rollout declares it is in"。

复现验证：

| cohort / percent | 结果 |
| --- | --- |
| `ga` / 10 | REJECTED（原先零报错） |
| `canary` / 100 | REJECTED |
| `ga` / 1 | REJECTED |
| `canary` / 55 | REJECTED |
| `canary` / 10 | 通过（基线，合法） |
| `ga` / 100 | 通过（合法：rollout 已推进到 ga 阶段） |

## 门禁证据（本轮真实执行）

| 名称 | exit | 摘要 |
| --- | ---: | --- |
| docs-structure | 0 | PASS；root files 10；canonical docs 22 |
| acceptance-validation | 0 | PASS |
| traceability-validation | 0 | statements 249；rows 249；PASS |
| corpus-validation | 0 | declared oracles 2；doctor cases 589；PASS |
| semantic-team-validation | 0 | scenarios 16；malformed fixtures 5；PASS |
| validator-negative-tests | 0 | `Ran 144 tests in 77.900s`；`OK`；0 skip |

- 负向测试 137 → **144**：新增 `SemanticTeamC2Review1` 共 7 个方法，逐条锁定 c2-B1/B2/B3，
  并含两个**正向**保护用例（已声明的 equivalence basis 不得被误杀；合法 cohort/percent 组合不得被误杀）。
- 生成器连跑两次，`acceptance/` 树聚合摘要一致：
  `687c8090b058cd9e35dbc225a4712c66d64c6dcd2e61888ed02bb162dbb4190d`
  （相对 repair-1 的 `5cc77831…` 变化，源于 `disclosure_binding_digest` 绑定体扩展，属预期）。
- `git diff --check`：通过。
- 8 个 positive fixture 的 `collect_violations` 全为空，无误杀。

## Deferred P2

reviewer 新增的 2 条已记入 `docs/plan/contexpect-docs-semantic-team-c2/DEFERRED-P2.md`，本轮不修。

## 预算

- product repair：2/3（repair-1、repair-2）
- product rereview：1/3 将用于本轮复审（c2 review-1 为首轮 review，不计 rereview）
- strategy reset：0/1

## 未做

- 未 commit、未 push。
- 未开始 Rust/Tauri 产品运行时编码。
