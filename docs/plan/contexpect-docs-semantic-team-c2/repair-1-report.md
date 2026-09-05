# semantic-team c2 · repair-1 报告

> 上游：c1 `STOP_FOR_OWNER`，final review `.octoworkflow/docs-semantic-team-review-4.md`（RED，4×P0 / 4×P1）。
> 本轮由 owner 在 2026-09-05 明确授权开启，并指定 review 角色改由 Claude 承担。

## 范围

修复 review-4 的 B2–B9，全部落在 `scripts/` 与 `tests/acceptance/`，不新增产品运行时骨架。

## 逐项处置

### B2 / P0 / ST2 — capability negotiation 与 native path set 未绑定权威 registry

- 新增 `FAMILY_NATIVE_BODIES` 权威 native 内容表，`family_native_content()` / `family_required_paths()`
  成为 builder 与 validator 的唯一来源；`four_native_files()`、`_projection_for_declared_family()`、
  `honest_unknown_projection()` 全部改为从它取值。
- `_validate_projection` 现在要求 native_files 的 path 集合**精确等于**权威集合（无缺、无多、无重复），
  每个文件 content 与权威逐字节一致，且 `native_digest` 等于权威 bundle digest。
- `native_path` 必须属于权威 required path 集合，而不只是"允许路径"。
- 新增封闭 enum `NEGOTIATION_STATUSES`；`required_capability` 必须是已声明 capability；
  `unsupported`/`dropped` 元素必须是 capability id，`unknown` 额外允许 `UNKNOWN_CAPABILITY_TOKENS`。
- 新增 `registry_capability_status()` / `negotiation_states_allowed()`：negotiation 可以**诚实降级**，
  不得声称高于权威 registry 的支持度。
- 新增 status / `projection_outcome` / `reconciliation_state` 三方一致性约束。

### B3 / P1 / ST3 — overlay / receipt 引用与 transaction 绑定

- overlay 的 `intent_id` 必须解析到 payload 的 canonical intent，`policy_id` 必须解析到 standard 的
  policy rule；`scope.family_id` / `scope.surface` / `scope.path` 必须是真实 harness 坐标与原生路径。
- overlay 的 `projection_outcome`、`weakens_required`、`authority` 类型化；native_files 去重、逐项 digest
  自洽，且 `scope.path` 必须真的在 overlay 文件里。
- receipt 的 `reconciliation_state` 收紧为 `CLAIM_RECONCILIATION_STATES` 封闭 enum，
  `native_result_captured` 必须是 bool，声称 captured 时必须有 declared repeatable oracle 支撑。
- `_transaction_body` 从三个布尔值扩展为绑定实际内容：`apply_at`、rollback target revision/digest、
  standard revision、overlay digests。

### B4 / P0 / ST4 — Team Standard 类型 / 唯一性 / lineage / trust / manifest

- `signatures` 非数组（对象、字符串、数字）现在判为 `unsigned-standard` + `missing-signature`，
  不再静默通过；重复签名 key 判为身份元组不一致。
- lineage：history 条目类型化，revision 不得重复、必须升序，最后一条必须等于 `previous_revision`，
  对应 digest 必须等于 `previous_digest`，当前 revision 不得已存在于 history（replay）。
- `trust.state` 为 revoked / expired / unknown 时分别落 code；publisher key 必须在 registry 内。
- `target_harness_coordinates` 必须是权威 coordinate 且不重复。
- `policy_rules` 新增封闭 schema `POLICY_RULE_FIELDS`：layer / mode / honesty 枚举、intent_ids 类型与
  引用闭合、rule id 唯一。
- content manifest 要求 digest 形状合法且 path 不重复（bijection）。

### B5 / P0 / ST5 — effective enforcement 未消费 negotiation 与 exception

- `recompute_effective_row` 重写：缺 projection 不再回退到 registry managed，而是按"无原生通道证据"处理；
  `managed_channel` 非 bool 一律按 false；negotiation 为 unsupported 类降为 `non-enforceable`，
  为 unknown 类或有 dropped 降为 `detect-only`。
- 新增 `_effective_exception_applies()`：当前有效且 scope 覆盖该 harness 的 exception 会把
  `enforceable` 降为 `detect-only`。

### B6 / P1 / ST6 — policy evaluation 与 exception 状态 / 时间 / audit 绑定

- 新增 `POLICY_EVAL_FIELDS` / `POLICY_EVAL_RESULTS` 封闭 schema：result 限定
  pass / deny / indeterminate，mode 限定 `ENFORCEMENT_MODES`，`policy_id` 必须解析到已声明规则，
  `covered_by_exception` 必须指向真实 exception，`inspection_export_allowed` 必须是 bool。
- required 规则报 pass 而没有具名 covering exception，判 `false-enforcement-claim`。
- `request` / `rejection` 状态的 exception 不再能为 required pass 背书。
- 新增 `AUDIT_EVENT_TIMESTAMP`：audit 条目的 `at` 必须等于其所记录状态对应的 exception timestamp。

### B7 / P1 / ST7 — consent / preview 类型与 Team Standard 身份绑定

- `upload_consented` 必须是 bool（null / 空串 / 字符串 / 0 / 列表全部拒绝）。
- preview 的 `per_harness_projection` / `loss_unknown` / `exception_metadata` 必须为 true。
- preview 的 `standard_version` 必须等于 Team Standard 的 `semantic_version`（无 standard 时必须是合法
  semver），并与 leader view 的 `standard_version` / `standard_id` 一致。

### B8 / P0 / ST8 — lifecycle 与 update digest 内容绑定

- lifecycle step 的 `actor` 必须是 `LIFECYCLE_STEP_ACTORS`，`preview` / `disclosure` 必须为 true，
  `state` / `drift_class` / `pin` / `per_harness` 类型化；null 与无关值全部拒绝。
- `expected_update_digest` 改为内容派生：绑定 semantic_version、policy_rules、content_digest_manifest、
  receipt 的 `bound_digest_manifest`、disclosure preview 与 report payload digest。
- 为打破 update digest ↔ transaction digest 的循环依赖，`_transaction_body` 不再绑定 `update.digest`
  （`apply_at` 已承载其实质）；同时把 fixture 绑定拆为
  `bind_report_payload` → `bind_receipts` → `bind_update` → `bind_disclosure` 四步。
- rollout：cohort 必须是已声明 stage，`order` 必须等于 stage 序列且无重复，`percent` 必须等于起始
  cohort 的 percent，stage 名唯一。
- conflict：`silent_merge` 必须是 bool，`inputs` 至少两个具名 head。
- LKG：revision 类型化，且必须等于 standard lineage 的 `previous_revision` / `previous_digest` /
  `stable_id`。
- rotation：`applied` 必须是 bool，`old_trust_state` 必须是合法 trust state，新旧 key 不得相同。

### B9 / P1 / ST8 — 根 README 门禁表不完整

- 根 `README.md` 补齐第六条 `validator-negative-tests`，并改为与其他 canonical 文档一致的
  "名称 + 命令"六行表 + 六行命令块。
- `contexpect_contract.py` 新增 `REQUIRED_GATES` / `GATE_TABLE_DOCS` 作为唯一权威门禁表；
  `check_docs.py` 新增 `check_gate_tables()`，强制 6 份 canonical 文档各自列全六条门禁的名称与命令。

## 门禁证据（本轮真实执行）

| 名称 | exit | 摘要 |
| --- | ---: | --- |
| docs-structure | 0 | `check_docs.py PASS`；root files 10；canonical docs 22 |
| acceptance-validation | 0 | `check_acceptance.py --structure PASS` |
| traceability-validation | 0 | statements 249；rows 249；PASS |
| corpus-validation | 0 | declared oracles 2；doctor cases 589；PASS |
| semantic-team-validation | 0 | scenarios 16；malformed fixtures 5；PASS |
| validator-negative-tests | 0 | `Ran 137 tests in 74.608s`；`OK`；0 skip |

- 负向测试从 108 增至 137：新增 `SemanticTeamAuthoritativeBinding` 共 29 个方法，逐项覆盖 B2–B8。
- 生成器连续两次执行，`acceptance/` 树聚合摘要一致：
  `5cc77831d8c1058980b31ccac6980f3b60c7438ee6f34542e46be9575cc517b5`。
- `git diff --check`：通过。

## 反向验证（证明新约束不是空转）

把 review-4 点名的三个函数临时还原为修复前实现，再跑同一组变异：

| 还原对象 | 变异 | 结果 |
| --- | --- | --- |
| `expected_update_digest` | policy rule 文本变更 | `update-preview-digest-mismatch` 不再触发 |
| `expected_update_digest` | disclosure preview 变更 | `update-preview-digest-mismatch` 不再触发 |
| `_transaction_body` | `apply_at` 变更 | `bound-digest-mismatch` 不再触发 |
| `_transaction_body` | rollback target 变更 | `bound-digest-mismatch` 不再触发 |
| `recompute_effective_row` | 覆盖 harness 的有效 exception | 完全无报错（修复前 false accept） |

`recompute_effective_row` 还原后，"negotiation 降级为 Unknown" 与 "projection 缺失" 两个变异仍被拒 —— 那是
B2 新增的 negotiation 一致性检查在兜底，属于重叠覆盖，不是本项修复失效。

## 未做

- 未 commit、未 push。
- 未开始 Rust/Tauri 产品运行时编码（`crates/`、`packages/`、`apps/` 仍不存在）。
