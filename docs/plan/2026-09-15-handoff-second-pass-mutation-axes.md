# 交接包二次核对：mutation 结果分轴与 C-F03 执行链接线（2026-09-15）

> 状态：本地实施候选，宿主自检；不是独立 reviewer GREEN，不是新 ADR，不改变 F-01–F-18 / WP-01–WP-12 范围。
> 输入：`docs/handoff/contexpect-2026-09-11/` 全部 29 个文件（16 份主文档、2 份 originals、11 张图）在本轮重新完整阅读，并与 `Downloads` 副本逐文件比对（`diff -rq`，仅 `.DS_Store` 差异）。基线：`main` @ `2ae179f`，工作区无未提交源码改动（仅两个未跟踪的 `.wav`）。
> 与 [2026-09-12 裁决](2026-09-12-handoff-value-implementation.md) 的关系：那份记录 T01–T12 / M01–M20 的首轮裁决与已落地切片；本文只处理其列出的"尚待独立工作包"中**当前授权、离线、零依赖即可判定**的部分，其余保持原状态与原因。

## 1. 二次核对后的实际差距表

按启动提示词 §F 的要求，逐条给出：问题/建议 ID、代码位置、当前状态、拟改动、决策依赖。表中"已落地"以 2026-09-12/13 提交的源码与测试为据，不重复叙述。

| 交接包条目 | 代码位置 | 当前状态 | 本轮处理 |
| --- | --- | --- | --- |
| C-F01 观察范围 ≠ 原生预期 | `ctxpect-resolve`、`truth-boundaries.spec.ts` DEMO-03 | 已落地（排除只降 coverage，预算 unknown/null） | 不动 |
| C-F02 字节相同/不同非等价判据；ST2 过强条款 | `semantic-alignment-and-team-standard.md` 修订头、`check_semantic_team.py`、`ST2-same-bytes-pos` | 已正式修订（规范、断言、夹具三处同步） | 不动 |
| C-F03 Unknown 按动作处理 | `ctxpect-policy/src/precondition.rs`（纯判定表）；`ctxpect-doctor` 只用它决定 treatment lock | **判定表未接入执行链**：`apply` / `rollback`（CLI 与 API）既不报告本次满足了哪些前置证据，也不在拒绝时指出缺的是哪一项；模块文档自述"接线是另一个工作包" | **本轮实施**，见 §2 |
| 03 §8 / 最终建议 §9.4 结果分轴 | `dispatch.rs` `projection_cmd`、`http.rs` `apply_api` / `rollback_api` | `apply` 只返回 `{transaction, post_receipt_id}`；资产 copy 已有 `runtime_verification: not-observed`，projection apply 没有同样的分轴，"已写入、运行未验证"只能靠读者推断 | **本轮实施**，与 C-F03 报告同一对象 |
| C-F04 声明不创造事实 | `doctor-rule-map.md`、`declaration-validation` 标记 | 已落地 | 不动 |
| C-F05 历史/当前/重析分离 | `doctor --as-of`、post-Receipt 追加、旧 Receipt 字节不变测试 | 已落地 | 不动 |
| C-F06 锁只约束受控参与者 | `cli-reference.md` 同机 advisory lock 段 | 已如实说明 | 不动 |
| C-F07 golden 可正式修订 | ST2 修订即先例 | 已有路径 | 不动 |
| C-F08 tombstone 与删除传播 | `receipts/` + `tombstones/`、`schema_conformance.rs`、DEMO-13 | 已落地 | 不动 |
| Q14 / T09.U06 / C24 发布包测试能力排除 | `packages/ui/tests/e2e/test-daemon.ts`、`crates/ctxpect-cli/src` | 记录为"未执行"。本轮核查：e2e 夹具只通过**普通 store 文件格式**（`policies/active.json`、`exceptions/*.json`）植入授权，与操作者手工放置的文件无差别；产品源码没有 `cfg(test)` 之外的测试旁路、没有测试专用环境变量（仅 `CTXPECT_PRINCIPAL_SECRET` 为正式身份通道）；UI 源码不含 e2e 标识 | 结论写入本文；不建"检查永不存在的旁路"的脚本制造假绿。桌面 WebDriver 测试构建尚未建立，届时再补发布包差分 |
| T08 APM / agnix 只读产物导入合同 | 无 | 未开始 | **不做**：离线且无 pinned 样本，凭链接臆造字段合同等于制造第二套未验证 schema；保留 [T07–T12 决策材料](2026-09-12-t07-t12-decision-materials.md) 的决策点 |
| M06/M03/M01 SQLite、jsonschema、cap-std | ADR 0006 | 条件 (a) 未触发；本轮所有改动 std 可完成 | 不引入依赖 |
| M14 daemon 无 token（C28、ADR 0005 D6） | `privacy-and-threat-model.md` §daemon | 已如实披露；退出条件已在 ADR 0005 写明 | 属 owner 决策项，不在本轮擅自加身份通道 |
| T10 / T11 / T12 组织信任、真实模型、三 OS lane、72h、8 人 QA | — | 外部条件未满足 | 保持未完成，不改标签 |
| 06 视觉 / 07 路由状态 / 08 DEMO-01–18 | `packages/ui`、`docs/process/2026-09-12-visual-evidence.md`、[DEMO 评估](2026-09-12-demo-01-18-evaluation.md) | 白底黑字、16 入口、43 条 Chromium E2E 已有 | 不动 |

## 2. 本轮实施：mutation 结果分轴 + 前置证据报告（C-F03 接线）

### 2.1 为什么值得做

交接包三处指向同一件事：02 C-F03（Unknown 按动作所需证据处理）、03 §8（产物分开提供操作执行状态 / 静态核对状态 / 运行核对状态 / 效果结论 / 信任验证状态，禁止只返回一个 `success:true`）、最终建议 §9.4（"已写入，运行时未验证"是合法且有价值的结果，不能显示为失败也不能显示为完整生效）。当前 `apply` 的返回值恰好是那种需要读者自行猜测含义的对象。判定表已存在但只用于 Doctor 的 treatment lock，执行链本身没有引用它——这是可在当前授权内、零依赖、有可判定反例的缺口。

### 2.2 设计

新增 `crates/ctxpect-cli/src/mutation_report.rs`，CLI 与 API 共用（R04：同一问题同一答案）。

**成功结果**（`apply` / `rollback`，CLI 与 `POST /api/v1/apply|rollback`）在既有 `transaction` / `post_receipt_id` 之外追加两个对象：

```json
"preconditions": {
  "action_class": "limited-static-fix",
  "complete": true,
  "satisfied": {
    "target-identity": "projection.contained_target",
    "original-bytes":  "projection.current_digest",
    "fix-semantics":   "projection.desired_digest",
    "permission":      "policy.exception_covers_scope",
    "policy-eval":     "policy.layers_pass",
    "approved-plan":   "projection.preview_previewed"
  },
  "missing": [],
  "may_remain_unknown": ["outcome"]
},
"verification": {
  "operation": "committed",
  "static_reverification": "observed",
  "post_receipt_id": "…",
  "runtime_verification": "not-observed",
  "effect": "not-evaluated",
  "trust": "local-continuity"
}
```

每个 `satisfied` 值是**实际执行过的产品检查**的名字，不是自声明：`contained_target` + `probe_target`（目标在项目内、常规文件、非控制路径）、`current_digest` 比对（并发编辑拒绝）、`desired_digest` + secret gate、例外覆盖 action/project/target、policy layers `pass`、持久化预览处于 `previewed` 且属本项目。报告只在这些检查全部通过、写入已提交后生成，因此成功时 `complete` 恒为 true——这不是空话：`missing` 由 `ctxpect_policy::precondition::evaluate` 真实计算，且单元测试锁定 `satisfied` 的 key 与判定表 `required()` 逐项同序，表一改报告就红。rollback 的 `satisfied` 对应 `after_digest` 比对、backup 字节与 `before_digest` 比对、`tx.json` 属本项目且未回滚。

`trust: local-continuity` 表示 post-Receipt 只带 store 的本地连续性 HMAC，不是组织签名（C-F04 / 最终建议 §9.6）。`effect: not-evaluated` 表示本次没有 Effect Lab 合同参与（DEMO-15 语义）。

**拒绝结果**：`InspectFailure` 新增 `Refused` 变体，错误信封多一个 `error.precondition` 字段，把拒绝码映射回缺失的前置证据 id：

| 拒绝码 | 缺失前置 |
| --- | --- |
| `projection.preview_missing` / `preview_scope` / `tx_consumed` / `export_only` / `no_tx` / `rollback_scope` / `already_rolled_back` | `approved-plan` |
| `projection.concurrent_hash` / `rollback_conflict` | `original-bytes` |
| `projection.preview_malformed` / `contains_secrets` / `backup_corrupt` | `fix-semantics` |
| `projection.not_a_file` / `control_path` / `path` / `escapes` / `io` / `parse` | `target-identity` |
| `policy.unknown` / `indeterminate` / `denied` / `detect_only_not_enforceable` | `policy-eval` |
| `policy.approval_required` | `permission` |
| `store.busy` 等 | 不映射（争用不是前置证据缺失） |

`permission` 与 `approved-plan` 的分工：例外覆盖 scope 是"写权限"，持久化预览是"已批准的计划"；两者都缺时以先失败的检查为准（预览状态先于授权读取，与锁顺序一致）。

### 2.3 不做的事

- 不改 `ctxpect_policy::precondition` 的表内容与 `evaluate` 语义。
- 不给 UI 加新页面；`packages/ui` 现有 care-plan 页不调用 `POST /apply`（DEMO-06 走 CLI），本轮不扩前端动作面。
- 不把 `verification` 写进正式 Receipt schema；它是命令/端点返回值，不是 Receipt 字段。
- 不为 sessions.import / standard publish 接 `ModelVisibleClaim` / `PublishOrAdoptStandard`：这两类的"满足"检查尚无对应的真实生产者（T04 / T10 前置），接了只能是自声明。

### 2.4 验收

新增 `product_loops.rs` 用例：

1. CLI `apply` 成功：`preconditions.action_class = limited-static-fix`、`complete = true`、`satisfied` key 与判定表同序、`missing = []`、`may_remain_unknown = ["outcome"]`；`verification.operation = committed`、`runtime_verification = not-observed`、`post_receipt_id` 与顶层一致。
2. 同一 store 经 daemon `POST /api/v1/apply`：`preconditions` 与 `verification`（去掉 `post_receipt_id`）与 CLI 逐字节相同。
3. 拒绝：预览后外部编辑 → `projection.concurrent_hash` + `error.precondition = original-bytes`；无例外 → `policy.approval_required` + `permission`；已消费 tx → `tx_consumed` + `approved-plan`；rollback 前用户再编辑 → `rollback_conflict` + `original-bytes`；CLI 与 API 各验一条。
4. `mutation_report` 单元测试：`satisfied` 顺序等于 `ActionClass::LimitedStaticFix.required()`；映射表里每个前置 id 都在判定表内。

门禁：16 条 required gate 全部实跑；`ui-e2e` 本轮不涉及前端改动，按 required 表外可选项处理，实跑结果如实记录。

## 3. 自审记录

- **是否制造假通过**：成功时 `complete` 恒 true 的风险已在 §2.2 说明，用"表与报告同序锁定"和"拒绝映射"两条反向测试对冲；若将来有某个检查被移出 apply 路径而报告未同步，映射测试不会发现——因此 `satisfied` 值直接写检查函数名，reviewer 可以 grep 核对。
- **是否越权**：只改命令返回值与错误信封新增字段，不改任何现有字段、拒绝码或 exit code 合同（`0/2/3/1` 不变）；`error.precondition` 只在 apply/rollback 路径出现。
- **是否与 R04 冲突**：CLI 与 API 共用同一函数与同一映射；测试 2 直接比对。
- **是否扩大范围**：Q14 只记录核查结论，不建脚本；T08 明确不做并给出理由。

## 4. 本轮验证（宿主自检，2026-09-15）

改动文件：`crates/ctxpect-cli/src/mutation_report.rs`（新增）、`dispatch.rs`（apply/rollback 拆为 `apply_tx` / `rollback_tx` 并挂 `attach_precondition`）、`http.rs`（`json_refused`、两个端点同源报告）、`inspect.rs`（`InspectFailure::Refused` 与 `error.precondition`）、`lib.rs`、`ctxpect-policy/src/precondition.rs`（仅模块文档）、`tests/product_loops.rs`（新增 `c_f03_mutation_results_report_preconditions_and_verification_axes`）、`cli-reference.md`、`desktop-ui.md`、`doctor-rule-map.md`、`docs/README.md`。未改任何现有拒绝码、exit code 或 UI 源码；`packages/ui/dist` 重建后产物 hash 与已提交版本一致（`index-_8KYYNxR.js`）。

16 条 required gate 在本工作区逐条实跑，退出码均为 0：

| 门禁 | 结果 |
| --- | --- |
| docs-structure / acceptance-validation / traceability-validation / corpus-validation / semantic-team-validation | PASS，exit 0 |
| validator-negative-tests | exit 0 |
| cargo-build / cargo-test / cargo-clippy | exit 0（`cargo test --workspace` 合计 427 个用例通过，含新增 3 个单元测试与 1 个集成测试） |
| corpus-conformance / doctor-corpus / native-conformance | exit 0 |
| ui-routes | `ui-routes-ok`，exit 0 |
| ui-unit | 63/63，exit 0（首次运行因本机 PATH 无 `pnpm` 可执行文件而 8 项 `spawnSync pnpm ENOENT`；用 `corepack enable --install-directory` 生成离线 shim 后复跑通过，与代码无关） |
| ui-typecheck / ui-build | exit 0 |
| ui-e2e（可选门禁，真 daemon + Chromium） | 43/43 通过，exit 0，耗时 1.1 分钟 |

新增集成测试实际覆盖：CLI apply 成功的 `preconditions` / `verification`；`tx_consumed → approved-plan`、`concurrent_hash → original-bytes`、`rollback_conflict → original-bytes`、`policy.approval_required → permission`、`policy.unknown → policy-eval`；同一 store 上 daemon `POST /api/v1/apply|rollback` 的 `preconditions` 与 CLI 逐字节相等、`verification`（去 `post_receipt_id`）相等，API 拒绝同样带 `error.precondition`。

未执行：真实 harness、WebView、双设备同步、真实模型、跨 OS 发行、独立 readback；未 commit、未 push。
