# DeepSeek Harness 建议核验记录（2026-09-09）

> 状态：核验记录，不是验收结论，也不是实施授权。
> 对象：用户提供的《ContextView：基于 DeepSeek Harness 的实施建议》（2026-09-08，仓外文件 `~/Documents/Codex/2026-09-08/.../contextview-review/contextview-harness-advice.md`，不入库）。该文件已同步标注本记录的结论。
> 基线：`main` @ `9be2842` + 当时工作区内**另一会话正在交付的 S1–S6 未提交候选**（见 [缺口分析 §8](2026-09-08-gap-analysis.md#8-实施后状态2026-09-08未提交候选)）。核验期间该候选仍在变化（23:48–23:50 新增 `docs/schemas/`、`schema_conformance.rs`、`render.test.mjs` 等），本记录只对读取时的文件版本负责。
> 方法：逐项阅读当前源码 + 用构建出的 `ctxpect` 二进制在隔离临时目录实测；没有执行任何 harness、oracle 或网络访问。实施交接：`docs/process/2026-09-09-deepseek-harness-review-IMPL-PROMPT.md`（gitignored，不入库；因此这里不作链接）。

## 1. 一句话结论

建议文件四项里，CV2 的 projection 部分与 CV3 的主体已被 S1–S6 候选修掉，**不必再做**；CV1 全部、CV2 的 store/签名部分、CV3 的两项残余、CV4 全部仍成立；另发现一个建议文件没列出的缺口（持久化 preview 不绑定 project）。

## 2. 逐项裁决

「已核验：无需再做」= 当前源码已修且有入口级或单元负例，重做没有价值；「仍成立」= 复现与源码都确认缺口仍在。

| 建议条目 | 子项 | 裁决 | 证据 |
| --- | --- | --- | --- |
| CV1 DSH 原生事件闭环 | 全部 | **仍成立，未动工** | `ctxpect-importer` 的 `KNOWN_EVENT_TYPES` 仍是 `user/assistant/tool/system/compaction`；`ctxpect-resolve` 无 deepseek resolver；用种下 `sessions.import` 例外后跑建议文件的最小 JSON（`--mapping deepseek-harness-cli`），返回空 `timeline` 与两条 `import_parse_failed`，与建议描述一致。补充：`compatibility-matrix.yaml` 已把 `deepseek-harness 0.1.2-rc.1 cli` 冻结为 `required-supported` 静态坐标，静态 resolver 不算扩大 required scope；native 证据导入才是实验性接入 |
| CV2 mutation 可恢复 | preview 持久化并绑定 apply（C32） | **已核验：无需再做** | `intent preview` 写 `store/previews/<tx>`，`apply --tx` 拒绝无 tx 且不重算；入口级负例 `product_loops.rs` 的 `projection.concurrent_hash` / `projection.preview_missing` |
| | rollback 校验并发、新文件回滚为删除、tx 先落盘（C33） | **已核验：无需再做** | 完整 CLI 入口实测：apply 后用户改写 → `projection.rollback_conflict` 且用户内容保留；新建文件回滚 `removed-created-file`；`tx.json` pending → committed；负例 `rollback_conflict` 已在 `product_loops.rs`；assets executor 同步 |
| | projection secret 门（C35） | **已核验：无需再做** | `preview`/`apply` 对 desired 与 backup 内容跑 `secret_literal`，命中 `projection.contains_secrets`，单测 `secret_bearing_content_is_refused_before_any_backup_exists` |
| | symlink 逃逸 | **已核验：无需再做** | 指向仓外文件的 symlink 与 symlink 祖先目录都被 `projection.escapes` 拒绝，仓外文件未被触碰。悬空 symlink 被当作「不存在」，apply 用 rename 替换链接本身，无逃逸；只是原链接会丢失，记为可接受 |
| | 错误 scope / 过期例外拒绝 | **已核验：无需再做** | 例外按 `(action, project_digest, target)` 绑定，跨项目实测 `policy.approval_required`；负例 `an_exception_for_one_project_and_action_does_not_authorize_another`、`exception_request_requires_a_lifetime_and_the_exception_expires` |
| | store：index 损坏时 Receipt 已写而 index 未修、重试不补 | **仍成立，且比建议描述更糟** | 新快照跑 `inspect --store`：Receipt 文件写入、index 仍损坏、audit 无记录，**退出码 0 且 JSON 无任何错误字段**（`crates/ctxpect-cli/src/lib.rs` inspect 分支用 `if let Ok` 吞掉 `persist_inspect` 错误）；index 修回 `[]` 后重试，`put_receipt` 因 first-write-wins 直接返回，index 永远为空；store 无 rebuild/reconcile 路径 |
| | 跨进程竞争边界 | **仍成立** | 全 workspace 无 lock 文件或 flock；CLI 与 daemon 共用 store 无任何互斥；文档没有宣称单写者（未过度声明，但边界未定义未验证） |
| | 崩溃后 in-doubt 判定 | **部分** | `rollback` 能处理 `pending` 状态；没有列出/判定 pending 事务的入口，Receipt/index/audit 三写没有共同恢复策略 |
| | `digest_body` 剥离 `created_at`，签名不覆盖时间（C37） | **仍成立** | scratch crate 打开真实 store 验签：篡改 `created_at` 后 `verify_local_continuity` 仍 `Ok`，篡改 `receipt_kind` 则 `receipt.manifest_mismatch`；根因是 `TIME_KEYS` 按键名剥离。注意 S1–S6 新增的 `docs/schemas/ctxpect-receipt-v1.schema.json` 明确把 `created_at` 声明为 display-only 且不进 `manifest.digest`——这与「稳定内容摘要排除时间」一致，但签名 envelope 仍应覆盖它 |
| CV3 conformance 绑定真实实现 | Rust runner 跑全部 1,924 行 | **已核验：无需再做** | `corpus_conformance.rs` 为 required gate；实跑 pass 22 / fail 0 / unimplemented 1,902；未实现不计 pass |
| | 门禁能失败 | **已核验：无需再做** | 变异测试：改一条 codex instructions golden 的 `truth_state`，gate 立即失败并指出该行；随后 `git checkout` 还原，`acceptance/corpus` 干净 |
| | `adapter test` 撤掉 `ran:true` | **已核验：无需再做** | 核验时返回 `ran:false`、`adapter.test_unimplemented`、exit 3（T3 后已改为真实运行语料：`ran:true` + `adapter.family_unimplemented` / `adapter.rows_failed` / `adapter.corpus_missing`，见 §5；本行保留核验时的事实） |
| | 报告分列 implemented / 未实现 / unknown-honesty / sealed-live 未执行 | **仍成立（部分）** | 只分 pass/fail/unimplemented；unknown-honesty 行计入 pass；没有 sealed/live 未执行行（test-strategy 要求分列） |
| | `adapter test` 真实运行指定语料并绑定版本 | **仍成立** | 现在是诚实的 unimplemented，但 runner 已存在于测试里，可复用 |
| | UI 浏览器集成测试 | **仍成立，但与项目决策冲突** | `packages/ui` 只有 `node --test` 单测与新增的 SSR 渲染测试；缺口分析 §5 明确不引入 Playwright。归为待用户决策，不在交接里实施 |
| CV4 Effect Lab 诚实语义 | 全部 | **仍成立，原样重现** | `ctxpect experiment --json` 无 project/store 时输出 `decision=supported-equivalent-within-margin`、`n=4`、`runs=[]`、exit 0；`experiment_cmd` 与 HTTP `lab_api` 的 control/treatment 仍来自 `run_local_instructions_probe(false/true)`。`preconstructed()` 无调用点，fixture 没有流入结果（比建议担心的好）。缺口分析 C24 未纳入 S1–S6 |
| 不采纳的方向 | 全部 | **同意，无需再议** | 与 AGENTS.md 不变量、缺口分析 §5 一致 |

## 3. 建议文件没列出的新发现

**持久化 preview 不绑定 project。** preview 记录只有 `target_rel` 与摘要，`tx_id` 由 `intent_id|target_rel` 派生。实测：项目 A `intent preview` 得到 tx，项目 B（内容相同的 `AGENTS.md`，并为 B 种下 apply 例外）直接 `apply --tx <A 的 tx>` 成功写入 B，同时覆盖 `store/apply/<tx>/` 下 A 的事务记录与 backup；同一 tx 在 rollback 后可无限次重新 apply，没有消费状态。这是建议文件「跨项目、过期或已消费批准不复用」的反例，目前无测试覆盖。

## 4. 仍成立的价值项 → 实施交接映射

| 价值项 | 交接阶段 |
| --- | --- |
| preview 绑定 project + tx 消费状态（§3） | T1(a) |
| store 持久化失败不被吞、index 可修复、journal 与恢复、advisory lock、pending 事务 in-doubt | T1(b) |
| 签名覆盖 `created_at` + `signed_at`；摘要按 schema 声明剥离（C37） | T1(c) |
| `ci` / `collect` 的 Receipt kind（C20，同根顺带） | T1(d) |
| Effect Lab：无 runs 即未执行；runs 输入按 F-15 校验；estimator 抽象；fixture 只在测试命名空间（C24、C2） | T2 |
| conformance 报告分列 + `adapter test` 真实运行并绑定 digest | T3 |
| DSH 原生 importer：pinned 输入合同、DSH API 生成夹具、前缀重建、field-to-claim 对齐、`/sessions/:id` 请求证据页、幂等/删除（CV1，B8，C19 最小字段） | T4（含检查点 CP-1） |
| UI 浏览器 E2E、统计库/SQLite 依赖政策、DSH 真实 runner | 不实施，汇报里列为待用户决策 |

## 5. 状态跟踪

实施会话完成后在此更新每行状态（已收口 / 部分 / 未做）并给证据；本记录不改 §2 的核验结论。

| 交接阶段 | 状态 | 证据 |
| --- | --- | --- |
| T1(a) | 已收口 | `product_loops::a_preview_is_bound_to_its_project_and_consumed_by_its_apply`（§3 的复现现在被 `projection.preview_scope` 拒绝，A 的记录未被覆盖；二次 apply → `projection.tx_consumed`） |
| T1(b) | 已收口 | `inspect_store_reports_a_persistence_failure_and_repair_recovers_it`（exit 1 + `persist_error`，`store repair` 后重试落一份）；`ctxpect-store` 三个恢复/锁单测；两进程并发 apply 单成功；`pending_transactions_are_judged_not_rewritten` |
| T1(c) | 已收口 | `ctxpect-receipt` 两单测 + `ci_and_collect_persist_their_kinds_and_verify_covers_the_times`（篡改 `created_at` → `receipt.signature_mismatch`；无 `signed_at` → `receipt.signature_legacy`） |
| T1(d) | 已收口（`ci` / `device-baseline`；`runtime` / `post-session` 仍无生产者） | 同上测试 |
| T2 | 已收口（同日追加仓内冻结估计器 `paired-exact-binomial-v1`，ADR 0006） | `ctxpect-effect` 单测 + `tests/estimator.rs`；`l10_single_pair_not_causal_and_n_locked`；`render.test.mjs` |
| T3 | 已收口 | `corpus_conformance.rs`（五列 + NOT_EXECUTED）；`adapter_test_runs_the_family_corpus_and_binds_digests` |
| T4 CP-1 | 通过 | 见 [交付状态](2026-09-08-delivery-status.md#2026-09-09-deepseek-harness-核验记录实施t1t4)：脚本退出码 0，`dsh_sha` 匹配，无家目录，DSH checkout 未变 |
| T4(c)–(f) | 已收口 | `native_conformance.rs` 四测试（required gate `native-conformance`）；`native_session_requests_follow_import_and_delete`；`render.test.mjs` / `generation.test.mjs`；UI 交互 E2E 由可选门禁 `ui-e2e`（Playwright，4 用例）覆盖 |

## 6. 本记录的验证边界

- 所有实测在 scratch 临时目录进行，用 `cargo build --workspace` 产出的 `target/debug/ctxpect`；例外与 policy 用与 `product_loops.rs` 相同的五层 fixture 种下。
- `cargo test --workspace` 与 `corpus_conformance` 在当时的 WIP 树上全绿。
- 唯一触碰过仓内文件的动作是变异测试临时改写 `acceptance/corpus/development/static/codex__cli.jsonl`，已用 `git checkout` 还原并确认干净。
- DSH 参考 checkout 存在于 `~/WorkSpace/Reference/deepseek-harness`，HEAD `67eac41d8788681901915b9f755e9f81d2962a7b`、版本 `0.1.2-rc.1`、MIT；tracked 文件无修改，只有 `proposals/`、`.omo/`、`.wrangler/` 等 untracked 内容。建议文件引用的七个源文件均存在；`deriveMessages` 是 `Session` 实例方法（`agent-loop/src/invariant.ts:39` 调用），不是独立导出。
- 没有独立 reviewer；本记录不能据以宣称任何实现通过独立验收。2026-09-09 实施完成后另有一轮两路交叉 review（Claude 子代理 + Codex `gpt-6-astra`），结论与修复见 [交付状态「交叉 review 与修复」](2026-09-08-delivery-status.md#2026-09-09-交叉-review-与修复)；其中 DSH importer 的「每个 header 一个请求」模型被指出与 DSH 定理相反并已改为每个 step 一个请求——本记录 §2 CV1 行与 §5 T4 行描述的是当时的实现。
