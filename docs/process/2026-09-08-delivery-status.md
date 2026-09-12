# 交付状态（2026-09-08）

记录本仓库当前的真实完成度。写它的原因是：上一份接续交接里的坐标、清单状态与已知问题都已过时，接手的人需要准确的起点。

## 坐标

| 项 | 值 |
| --- | --- |
| HEAD | 本记录与 S1–S6、T1–T4 及交叉 review 修复在**同一次提交**进入 `main`（SHA 见 `git log -1 -- docs/process/2026-09-08-delivery-status.md`），已推送 `origin/main`；此前基线 `9be2842` |
| 分支 / worktree | `main`，单一 worktree |
| Rust workspace | 17 个本地 crate，**0 个第三方依赖**（`Cargo.lock` 无 registry 条目；新增的 crate 间依赖只有 `ctxpect-projection → ctxpect-doctor`） |
| 桌面壳 | `apps/desktop/src-tauri`，**独立 workspace**，不在根 `members` 里 |
| required gate | **16 条**（原 13 条 + `corpus-conformance` + `doctor-corpus` + `native-conformance`），在本次提交上逐条单独取退出码全为 0，并在同一提交的干净 worktree 上复跑（见「2026-09-09 交叉 review 与修复」） |

## 2026-09-08 缺口分析实施（S1–S6）

依据 [缺口分析](2026-09-08-gap-analysis.md) §4 P1/P2 中不依赖设备与决策的条目，六个阶段全部完成；逐条状态见该文档各表的「状态」标注与本节。证据 = 测试名或门禁。

| 阶段 | 结果 | 证据 |
| --- | --- | --- |
| S1 授权、事务与隐私 | C31–C36、C38、C22 完成 | `product_loops.rs`：`an_exception_for_one_project_and_action_does_not_authorize_another`、`apply_is_bound_to_the_persisted_preview_not_a_recomputed_one`、`rollback_preserves_a_later_edit_and_removes_a_created_file`、`import_keeps_no_secret_or_path_bytes_in_the_store`、`a_secret_bearing_apply_is_refused_before_any_backup_exists`、`ci_gates_on_store_policy_and_never_reads_no_policy_as_pass`、`exception_request_requires_a_lifetime_and_the_exception_expires`、`an_exception_over_a_stale_policy_source_does_not_grant`；`ctxpect-projection::the_transaction_record_lands_before_the_target`（只读目录中断→`tx.json` pending、目标未动）；`ctxpect-store::a_missing_random_source_fails_closed_instead_of_degrading` |
| S2 诚实性小修 | C8、C9、C14、C15、C17、C27、C29、C40 完成 | `constants_are_replaced_by_honest_unimplemented_answers`、`subcommands_are_distinct_operations_not_aliases`、`advisor_refuses_an_unimplemented_adapter`、`api_routes_are_exact_and_follow_the_session_coordinate`；`page-state.test.mjs::the routing table is parsed and routes by method as well as path` |
| S3 corpus-conformance | 门禁新增并注册 | `cargo test -p ctxpect-cli --test corpus_conformance`：分母 1,924；通过 22（codex instructions 3 + claude-code instructions 6 + 13 条两 anchor 的 instructions honesty cell）；失败 0；未实现 1,902 |
| S4 Doctor 规则映射 + doctor-corpus | 20 条规则全部实现；门禁新增并注册 | [doctor-rule-map.md](doctor-rule-map.md)；`cargo test -p ctxpect-cli --test doctor_corpus`：20 条规则 precision 1.00 / recall 1.00，589 行 `expected_exit` 与 `blocking_exit` 一致；`ci_and_doctor_block_on_the_same_project_rule` |
| S5 第二个 anchor | Claude Code 2.1.259 instructions 切片完成；resolver 多 anchor 表驱动 | `docs/adapters/grammar/{codex-cli-0.147.0,claude-code-cli-2.1.259}-instructions.md`；`grammar.rs::cl1…cl6`；`corpus-conformance` 中 `claude-code/2.1.259/cli/macos-27-arm64 instructions 6/6`，其余 54 行计未实现 |
| S6 文档与合同工件 | C11/C18/C13/C16/C5/C12/A2/A3/A7/C6 完成 | `check_docs.py`、`check_acceptance.py --traceability`（新增 `implementation` / `evidence_test` 列：243 行 partial、7 行 unimplemented）、`schema_conformance.rs`、`render.test.mjs`（S6 时 5 个渲染测试；T2/T4 追加后为 8 个） |

**本次未做（在交接范围外或需要决策）**：ADR 0006/0007 正文（只给候选）、PRD §9.1 第 13 项文字（改 PRD 会改变 traceability 的 golden 行，超出「加列」红线，留待与 ADR 0005 后续决定一起处理）、E2EE、SQLite、oracle 执行、CI 工作流、截图/OS lane、Codex 之外其它 capability 的 resolver。

## 2026-09-09 DeepSeek Harness 核验记录实施（T1–T4）

依据 [核验记录](2026-09-09-deepseek-harness-review.md) §4 与缺口分析 C2 / C19 / C20 / C24 / C37 / B8。实施时为未提交候选（叠加在 S1–S6 候选之上），随本提交入库；两路交叉 review 见下文。证据 = 测试名或命令。

| 阶段 | 结果 | 证据 |
| --- | --- | --- |
| T1(a) preview 绑定 project + 消费状态 | 完成 | `ctxpect-projection::a_preview_is_bound_to_its_project_and_consumed_by_its_apply`；`product_loops::a_preview_is_bound_to_its_project_and_consumed_by_its_apply`（项目 B 持有自己的 apply 例外仍被 `projection.preview_scope` 拒绝且 A 的记录未被覆盖；二次 apply / rollback 后 apply → `projection.tx_consumed`） |
| T1(b) store 可恢复性 | 完成 | `inspect --store` 持久化失败 → `persist_error` + exit 1（`inspect_store_reports_a_persistence_failure_and_repair_recovers_it`）；`ctxpect-store::a_corrupt_index_is_named_and_rebuilt_byte_identically`、`a_failure_in_every_gap_of_the_three_writes_is_recovered_without_duplicates`（三写每个间隙注入失败 → rolled-back / committed，无重复行）、`the_advisory_lock_refuses_a_foreign_holder_and_is_reentrant_within_the_process`（持有者退出后锁由内核释放、可立即重取，没有"接管"逻辑可测）；两个真实进程并发 apply 只一个成功（`a_live_lock_holder_refuses_a_mutation_and_concurrent_applies_yield_one_success`，此测试暴露并修掉了最初「pid 文件锁」实现的创建竞态；最终实现改为 OS 文件锁 `File::try_lock`，因为产品 crate 不得创建进程做 pid 探活）；`pending_transactions_are_judged_not_rewritten`；新命令 `store status\|repair`、`apply status` |
| T1(c) 签名覆盖时间 | 完成 | 见缺口分析 §8 C37 行；`docs/schemas/ctxpect-receipt-v1.schema.json` 新增 `x-display-only` 与 `signature.signed_at`，`schema_conformance.rs` 同步通过 |
| T1(d) ci / collect Receipt kind | 完成 | `ci_and_collect_persist_their_kinds_and_verify_covers_the_times` |
| T2 Effect Lab | 完成 | 见缺口分析 §8 C2 / C24 行；`ctxpect-effect` 单测（当时 4 个，同日追加估计器后 6 个，交叉 review 后 8 个）+ `tests/estimator.rs`；`l10_single_pair_not_causal_and_n_locked`；`render.test.mjs::an experiment that was not executed says so and shows no decision` |
| T3 conformance 分列 + adapter test | 完成 | runner 抽到 `ctxpect_cli::conformance`；`corpus-conformance` 报告五列（本次：1,924 = 9 implemented-pass + 13 unknown-honesty-pass + 0 fail + 1,902 unimplemented；`NOT_EXECUTED sealed=60 live=2 oracle=34 doctor=589`，`doctor` 一列是交叉 review 后补列的 development Doctor 语料，由 `doctor-corpus` 门禁执行）；`adapter_test_runs_the_family_corpus_and_binds_digests`（codex pass / deepseek-harness unimplemented exit 3 / 变异一条 golden 两条路径都报该行） |
| T4 CP-1 夹具生成 | 通过 | `pnpm --pm-on-fail=ignore --dir ~/WorkSpace/Reference/deepseek-harness exec tsx <CV>/scripts/fixtures/deepseek-harness/generate.ts --out <CV>/acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1`（退出码 0；不带 `--pm-on-fail=ignore` 时 corepack 因 DSH 的 `packageManager: pnpm@11.7.0` 与本机 11.20.0 不符而拒绝，该标志不改 checkout）；`meta.json` `dsh_sha = 67eac41d…f5a2962a7b`；首版 25 事件、3 请求、3 closers，交叉 review 后按 DSH 的 request-reconstruction 定理改为**每个 step 一个请求**并加入一个无 header 的 step 重新生成：28 事件、4 请求、3 closers（同一命令再跑一次，退出码 0，DSH checkout 仍未变）；`grep -r /Users/ acceptance/corpus/development/native` 为空；DSH checkout `git status --short` / `diff --stat` 与开工一致 |
| T4(c)–(f) 原生 importer、field-to-claim、UI、幂等/删除 | 完成 | `native_conformance.rs`：`the_importer_reconstructs_every_request_prefix_exactly_as_dsh_did`（header digest、消息数、逐条消息 digest、surface 节点、replacement 区间、closers 与 `expected.json` 全等）、`every_precise_partial_answer_has_a_red_negative`、`the_persisted_record_is_metadata_only`、`import_is_idempotent_and_delete_cascades`；`product_loops::native_session_requests_follow_import_and_delete`；`render.test.mjs` 两个请求证据页测试 + `generation.test.mjs`；`check_acceptance.py --structure/--corpus` 通过（field-to-claim 扩到 6 条原生字段、`corpus-manifest.json` 新增 `native_synthetic`） |

**同日追加（用户决定「按建议」执行，[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md)）**：

| 项 | 结果 | 证据 |
| --- | --- | --- |
| UI 浏览器交互 E2E | 完成，可选门禁 `ui-e2e` | `packages/ui` 新增 `@playwright/test@1.56.1`（唯一新 dev 依赖）、`playwright.config.ts`、`tests/e2e/{global-setup,global-teardown,pages.spec}.ts`；`pnpm test:e2e` 4/4 通过（真实 daemon，`127.0.0.1:0`），teardown 后无残留 daemon / 临时目录；Chromium headless shell 安装在 Playwright 缓存目录（家目录 `Library/Caches/ms-playwright`，这是本次唯一的家目录写入，为用户授权的浏览器安装） |
| Effect Lab 估计器 | 完成，零依赖 | `ctxpect_effect::PairedExactBinomial`（`paired-exact-binomial-v1`）；`decide()` 走该估计器；`Cargo.lock` 仍无 registry 条目；单测见缺口分析 §8 C2；`l10` 追加 10 对 → `supported-beneficial` exit 0 |
| SQLite / 第三方 crate | 继续延期 | ADR 0006 写明结束条件；未引入 |
| DSH 真实 runner | 未做（按建议） | 无可运行 dsh 与凭据；ADR 0006 决策四 |

**仍未做**：`runtime` / `post-session` 两类 Receipt 无生产者；其余 23 个 mapping 的原生字段；ADR 0007（E2EE 依赖来源）；浏览器级「取消」用例。

## 2026-09-09 交叉 review 与修复

按用户要求，S1–S6 + T1–T4 候选先经两路独立 review：一个零上下文 Claude 子代理（它把工作扇出成 8 个领域子审查：store journal/lock、Receipt 签名/schema、conformance runner 与估计器、projection/apply、文档 ↔ CLI、Doctor 规则/secret、DSH importer、HTTP/UI）与一个 Codex `gpt-6-astra` xhigh 只读会话（`turn.completed`，退出码 0，历时 1,220 s）。两路各自实测（scratch crate、真实 `ctxpect` 二进制、变异测试），报告原文留在会话 scratchpad（含本机路径，不入库）。这不是独立验收 GREEN：reviewer 与实施者是同一次交付里的角色，且修复后的候选没有再经第三方 readback。

### 已修（每条附转红负例）

| 领域 | 发现 | 修法 / 证据 |
| --- | --- | --- |
| projection / assets（P0） | 固定临时文件名 + `fs::write` 跟随预置 symlink：敌意仓库提交 `.AGENTS.md.ctxpect-tmp -> /任意路径`，一次合法 apply 就把 desired 写到项目外并让目标变成仓外链接；backup 目录同型；FIFO 目标挂死进程 | `ctxpect_fs::write_atomic`（`O_EXCL` + pid/计数临时名 + fsync + rename）供三处共用；`probe_target` 用 `symlink_metadata` 拒绝 symlink/FIFO/socket/目录（`projection.not_a_file`）；事务目录与其父目录不得是 symlink。`ctxpect-projection::symlinks_fifos_and_pre_placed_directories_are_refused_not_followed` |
| projection（P1） | `tx_id` 无 nonce：同 intent 二次 preview 复用事务目录、覆盖上一事务的 backup，rollback 回不到初态；rollback 不绑定 project；`--id`/`tx_id` 未校验可路径穿越；`apply *` 例外可自写 `store/exceptions` 与 `.git/config`；损坏 `tx.json` 被静默跳过；apply 在取锁前读预览状态 | 每次 preview 唯一 id；`tx.json` 带 `project_digest`，rollback 校验；`valid_tx_id` 先于任何拼路径；`.git/`、`.ctxpect/`、store 内路径 → `projection.control_path`（CLI 与 API 的 preview/apply 都检查）；`unreadable` 计入 in-doubt；CLI/API 的 apply/rollback 先取锁。`every_preview_owns_its_transaction_so_no_backup_is_overwritten`、`control_paths_are_never_targets`、`a_torn_record_is_listed_as_unreadable_and_other_projects_are_skipped`、`a_secret_smuggled_into_a_persisted_preview_is_refused_at_apply`（此前 secret 门的 apply 路径无测试：删掉 apply 内的门测试仍绿） |
| store（P1） | `Store::open` 无锁重放 journal，与持锁写者竞争；preview/exception/verify_caller 的写入无锁；固定 `.tmp` 名并发下 ENOENT、审计 seq 重复（8 进程 × 20 次 `intent preview` 97/160 失败）；撕裂的审计尾行让全店永久不可写且无修复；有 Receipt 而 index 缺失被读成空 ledger；审计链对**尾部截断**不可检出而文档声称可检 | 所有写入方法自取锁，open 只在持锁时重放；`atomic_write_bytes` 走共用写入；`audit/head.json` 链尾锚点（自带 MAC）：截断 → `audit.truncated`，被截断/锚点无效的日志拒写（含 repair 自身的审计条目，因此截断不会被下一次写入掩盖）；撕裂尾行 → `store.audit_torn`，`repair` 移入 `audit/torn-*.bin`；`store.index_missing`；`repair` 有残余 in-doubt 时 exit 3；密钥 0600 独占创建、宽权限拒用。`cutting_entries_off_the_end_of_the_audit_log_is_detected`、`a_torn_audit_tail_blocks_writes_and_is_mended_by_repair`、`a_missing_index_over_existing_receipts_is_never_read_as_empty`、`opening_while_another_process_holds_the_lock_leaves_the_journal_alone`、`a_world_readable_continuity_key_is_refused` |
| Receipt / schema（P2） | `x-display-only` 列了 `tombstone/deleted_at` 却不在 MAC 里；`signed_at` 非字符串被当旧格式；CLI 与 API 对 tombstone/legacy 的 verify 答案不同；校验器的 `additionalProperties`/`minItems` 只在有 `properties`/`items` 时生效、未访问分支的不支持关键字不报；`persist_inspect` 返回的不是落盘那份；两份 `TIME_KEYS` 漂移 | schema 清单改为两项并加顶层 `additionalProperties: false`；legacy 只当键缺失；`verify_receipt_report` 一处实现；校验器先静态扫描整棵 schema 再校验实例；`persist_inspect` 回读；`jsonutil` 改为 re-export。`keywords_apply_on_their_own_and_unreached_branches_are_still_swept` |
| Doctor / secret（P1） | 带引号/JSON 写法的 secret 漏检（`aws_secret_access_key = "…"`、`"Authorization": "Bearer …"`）；占位符误报（`Bearer <your-token>`、`${VAR}`、`sk-learn-…`、散文里的 `PRIVATE KEY-----`）；emoji ZWJ / 波斯语 ZWNJ 被判隐藏字符；tag 字符、软连字符漏报；单个非 UTF-8 字节让整个文件跳过全部内容规则；`--fail-on` 非法值静默当作无阈值；API `/doctor` 在项目扫描失败时退回缺项目规则的诊断 | `secrets.rs` 重写（引号、JSON、占位形状、`sk-` 需含数字、PEM 单行）；joiner 只在前后都是 ASCII 时算隐藏，补 U+E0000–E007F/U+00AD/U+2061–2064/U+206A–F/U+061C/U+180E/U+034F；`text_of` lossy；`--fail-on` 只接受 `confirmed`；API 扫描失败返回错误。`quoted_json_and_placeholder_spellings_are_classified_the_same_as_bare_ones`、`joiners_in_scripts_and_emoji_are_text_but_tag_characters_and_soft_hyphens_hide`、`a_stray_non_utf8_byte_does_not_switch_the_content_rules_off`；`doctor-corpus` 20 条规则仍 1.00/1.00 |
| Effect Lab（P1） | 等价区间把不一致率 `m/n` 当已知值：11 对 1 个不一致对即判"等价"（两路各自复算确认）；`n > n_planned` 静默接受；重复 `(task, arm)` 静默覆盖、可靠排列输入选结果；时间跨格式比较无意义、`frozen_at` 不可解析 fail-open；已持久化实验只比较 `n_planned` | `paired-exact-binomial-v2`（对 `r` 与 `q` 各取精确区间、四角取范围；ADR 0006 修订）；`effect.n_mismatch`、`effect.run_duplicate`（按 task/arm）、`effect.run_time_invalid`、`epoch_seconds` 统一时间轴、`alpha ≤ 0.05` 与 `frozen_at` 进合同校验、`effect.contract_locked`。`one_discordant_pair_among_eleven_is_not_equivalence`、`extra_pairs_duplicate_slots_and_unreadable_times_all_invalidate` |
| DSH importer（P1） | 「每个 `request/header` 一个请求」与 DSH 的 request-reconstruction 定理相反：header 只在变化时记录，真实日志里多数 step 无自己的 header，会被误报 `request_prefix_incomplete` 并少算请求（夹具恰好每步都换 header，遮住了它）；JSON 解析无深度限制，10 KB 请求体让 daemon 栈溢出 abort；`sourceEventSeqs` 区间无上界（1.6 GB）；`reason`/`createdAt`/`delegationDepth`/`usage.*` 原样透传可携带正文；closers 顺序按 callId 排序而非转写顺序 | 请求锚定 `step/start`，header 取首个 provider 输出前的最新快照（`header_seq`、`header_logged_in_step`），生成器同步改为按 step 计算期望并加入无 header 的 step 3，夹具由 DSH API 重新生成；`ctxpect_schema` 解析深度上限 128；区间按当前 seq 封顶；`reason` 词表、整数字段过滤；closers 保持插入顺序。`native-conformance` 四测试重写（含 `header_seq` 逐项比对、header-less step 的 null 与无 claim） |
| HTTP / UI（P1–P2） | 一次 `read()` 当完整请求：分段到达的体被丢弃按 `{}` 处理，>64 KiB 体必失败；无 read timeout，空闲连接阻塞 daemon；`POST /rollback` 的 `tx_id` 路径穿越；`POST /intent/preview` 空体用默认值算出预览并落盘；`GET /sessions/import` 命中 `:id`；settings 不经 `object_body`；`useResource` 的旧回调无代次守卫；Doctor 链的 generation 是另一份手写计数；e2e 没有真正制造重叠请求；桌面壳把 `http://127.0.0.1:7420@example.invalid` 读成回环主机 | 按 `Content-Length` 收满、64 KiB/4 MiB 上限、5 s 读超时、拒绝 `Transfer-Encoding`、HEAD/OPTIONS 语义；`tx_id` 形状校验；preview 必填字段；字面路由排他；settings 走 `object_body` 并剥离 `snapshot_digest`；`useResource`/`runInspect` 共用 `createGeneration()` 并 abort 旧请求；e2e 新增「离开会话后迟到响应被丢弃」用例；桌面壳改用 `Url::parse` 并拒绝 userinfo（`only_a_plain_loopback_http_url_is_accepted`，`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --offline` 通过） |
| 文档 ↔ 实现 | 引用了不存在的测试名；gap-analysis §8 C15 的 reason code 已被 T3 替换；`align.byte-equality-not-semantic` 不存在；`exception request` 示例缺必填参数；provenance 漏登记 `fsevents`；审计链"删除均可检测"超出实现；journal 路径与 legacy 代码名写错；grammar 文档 CL2/CL3 的来源不在台账；`NOT_EXECUTED` 漏列 doctor 语料；`adapter test` 的 `corpus_digest` 是现算不是核对；两个只解析不使用的 flag | 逐条改正（见各文档 2026-09-09 段落）；`--locale`/`--kind` 改为显式 `usage.unimplemented` |

### 已记录、未修（需要决策或超出本轮）→ 已于同日下午处理，见下节

- Doctor 语料信息量、声明文件命名空间、suppress 机制 → D1 / D2。
- MAC 域分隔 → D8。DSH 浮点字面量 → D3。daemon 会话坐标与 `POST /inspect` 范围 → D4。
- ADR 0007、ADR 0005 非永久性、PRD §9.1 第 13 项 → D5 / D6 / D7。

### 本轮验证

16 条 required gate 在修复后的候选上逐条退出码 0（cargo 六条在 staged 树的干净 worktree 上另行复跑），`ui-e2e` 5/5，桌面壳单测 1/1；`corpus-conformance` 计数不变（1,924 = 9 + 13 + 0 + 1,902），`doctor-corpus` 20 条规则 1.00/1.00，`native-conformance` 在重新生成的夹具上 4/4。

## 2026-09-09 续：决策项实施（D1–D8）

八项此前标为「需要决策」。每项先与 Codex `gpt-6-astra`（xhigh，只读会话）交叉讨论，采纳其修改意见后实施；Codex 的逐项立场记录在会话 scratchpad 的 `codex-decisions.last.md`，此处只记结论与证据。

| 项 | 决定 | 实施与证据 |
| --- | --- | --- |
| D1 声明文件命名空间 | 双轨识别（`.ctxpect/<name>` 或根目录 `<name>` + `schema: ctxpect-<name>-v1`），以 contract revision 重生成语料：旧答案不改，输入只多 `schema` 键；每条规则补内容不同的正反例 | `rules.rs` `declarations()`；`claude_code.rs` `budget.json` 同规则（`a_root_budget_without_the_schema_is_not_a_declaration_but_the_namespaced_one_is`）；`contexpect_fixtures.py` `_declarations` / `decl()` / 变体；语料 589 → 707 行（29 双轨 + 89 变体），旧 589 行答案逐 id 相同、280 个输入文件差异仅 `schema` 键（脚本核对）；`doctor-corpus` 20/20 1.00/1.00；`doctor_corpus.rs` 断言改 707 |
| D2 suppressions | `.ctxpect/doctor-suppressions.json`，finding 仍输出并标 `suppressed`，事实计数保留、另加 active 计数，阻断规则必须绑定 `evidence_digest`，无效条目非阻断报出 | `crates/ctxpect-doctor/src/suppressions.rs`（12 单测）；`blocking_exit` 只看 active；`a_suppression_keeps_the_finding_visible_and_is_bound_to_the_reviewed_bytes`（改一字节即失效） |
| D3 DSH 浮点字面量 | **不**扩展公开 canonical JSON：`parse` 仍拒绝浮点；新增 `parse_preserving_numbers` 只给 DSH importer 用，非整数按原始词素保留（`Value::Number(NumberLexeme)`）、原样回写、不做算术、`as_i64` 为 None；词素必须是 `JSON.stringify` 的形状（否则 `import_parse_failed`），因此 `header_digest` 与 DSH 自己的哈希一致 | `json.rs` `NumberLexeme::new` / `is_js_shortest_form`；validator `number` 接受词素、`integer` 不接受；`generate.ts` header 加 `config.temperature: 0.5` 等，夹具与 manifest digest 重生成；`native-conformance` 4/4；`BOUNDARY.md` 经 `render_boundary` 重生成 |
| D4 daemon 观测范围 | Receipt 记 `coordinate.project_digest`；`/monitor`、`/care-plan/:id`、`/team/compliance` 接受 `?receipt_id=`，显式 id 不回退；Receipt 的项目摘要与 daemon `--project` 不符 → `api.receipt_scope`，任一方缺失 → Unknown（Doctor 报 `project_rules.scanned: false` + `api.receipt_scope_unknown`，其余端点报错）；`POST /inspect` 的 `project` 必须落在 `--project` 内、`codex_home` 必须等于 `--codex-home`，否则 `api.project_scope`；无 `--project` 的 daemon 不接受任何 `project` | `http.rs` `ReceiptScope` / `scoped_receipt` / `selected_receipt`；`the_daemon_scans_only_its_root_and_reads_receipts_only_from_it`；`persist_inspect_in` 为 collect / doctor / preflight / post-Receipt / ci 盖章 |
| D5 E2EE 依赖来源 | ADR 0007：age 格式；两条实现路径（vendored `age` crate 经 ADR 0006 supersede；外部 `age` 可执行文件经受控 executor）都以「先修订进程护栏」为前置；当前 `sync.encryption_unavailable` 不变 | ADR 0007（原记录声称已形成；2026-09-11 核对当前工作区缺此文件，决策依据待补证，不能据此启用加密实现） |
| D6 ADR 0005 非永久 | 写明 CLI-only 的退出条件：daemon 取得访问凭证（`--token-file` 随机 token 或 Unix socket peer credential）**且**请求映射到已登记 principal 并过既有四眼；浏览器自报角色永不算 | [ADR 0005 修订](../adr/0005-exception-identity-and-mutation-boundary.md) |
| D7 PRD §9.1 第 13 项 | 原句就地改为「查看与状态；request/approve/reject/revoke 经 CLI 身份通道，UI 动作待身份通道」并附日期澄清 | `generate_acceptance.py` 重跑：traceability 250 行不变（Codex 复核 + `--traceability` PASS），只有 PRD 的 digest 变化 |
| D8 MAC 域分隔 | 新签名 `hmac-sha256/ctxpect-receipt-v2`（输入 `canonical_json({domain, digest, created_at, signed_at})`）、审计条目 `v: 2` 带域前缀、标准文档 `hmac-sha256/ctxpect-standard-v2`；验签按 `signature.algorithm` 严格选规则（未知 → `*.signature_algorithm_unknown`），v1 仍按 v1 验（同一密钥，不是降级面） | `ctxpect-receipt` / `ctxpect-store` / `dispatch.rs` 各一条「v1 仍验、tag 只选一条规则」测试；schema `algorithm` enum |

顺带修正：`generate_acceptance.py` 的 DSH `field-to-claim` 文案此前落后于手改过的 yaml（重生成会回退到旧文案），已把生成器对齐到已评审文本；`native-conformance` 新增的 `config` 对象同时覆盖 `top_p` 与整数 `maxTokens`。

## 上一轮 review §9 清单的逐项状态

| # | 事项 | 状态 |
| --- | --- | --- |
| 1 | 补授权、intent 持久化移到授权后、破坏性删除补审计 | 已完成 |
| 2 | 例外生命周期 bootstrap 死锁 | 已解开——身份源定为**仓内已登记 principals**，见 [ADR 0005](../adr/0005-exception-identity-and-mutation-boundary.md) |
| 3 | `standard preview/adopt/pin/update` 实现为各自不同的操作 | 已完成 |
| 4 | policy `required:true, effect:"allow"` 语义倒置 | 已修 |
| 5 | `EquivalenceProfile` 真正参与 `diff()` | 已完成 |
| 6 | `error_envelope` 硬编码 inspect 的 command/schema | 已修 |
| 7 | 截图隐私下 `projectFocused` 绕过遮罩 | 已修 |
| 8 | 四个常量端点换真实计算 | 已完成 |
| 9 | V12 Settings 编辑面与 V04/V05/V08 动作面 | 已完成 |
| 10 | C04 十类状态、取消/重试、drawer 契约 | 已完成，逐页契约由测试守住 |
| 11 | Tauri 2 桌面壳 | 壳已建成，**macOS lane 已验**；Windows/Linux 未验（缺设备）。规范页中「尚未创建」的过时语句已在 2026-09-08 统一改正（C11） |
| 12 | C08 截图与 a11y/性能取证 | 部分完成，见 [取证记录](2026-09-08-keyboard-and-responsive-verification.md) |
| 13 | 18-family 真实探测 | **未做**——需真实安装外部 harness |
| 14 | E2EE sync | **未做**——见下节 |
| 15 | assets copy executor + SBOM | 已完成 |
| 16 | SQLite + FTS5 | **未做**——见下节 |
| 17 | WP-12 全矩阵 / 72h soak / 8 人 QA | **未做**——缺设备与人工验证 |

## 清单之外发现并修复的问题

这些不在原清单上，是实施与自测过程中查出的。三处是安全漏洞。

**安全**

- **DNS rebinding 绕过**：`Host`/`Origin` 用 `starts_with("127.0.0.1")`，`127.0.0.1.evil.com` 因此通过。攻击者注册该域名并指向 127.0.0.1，其页面即可同源读取本地 API 的全部响应。改为精确匹配主机名，缺失 `Host` 一并拒绝。
- **静态资源的符号链接逃逸**：路径检查是 `contains("..")` 字符串比对，UI root 内一个指向外部的符号链接就能读出任意文件。改用 `Root::contain`——产品读项目文件一直用它，是内部标准不一致。
- **审计链名不副实**：`audit_chain()` 只是读 jsonl，无前序哈希、无序号、无签名，删改重排不留痕，且无任何调用点。补 MAC 链接，接入合规视图。

**声明与实现脱节**（同一类，反复出现）

- `resource_limits` 两字段从不被读取；`copy_executor` 恒为 unimplemented；四个端点返回常量；CORS 头漏端口因而什么也没授予；`put_settings` 完全无验证（连隐私不变量都能改掉）。
- `/receipts/:id` 渲染了却没写进 `routes.ts`，而 C04 契约测试正是从那里取清单——**测试全绿，覆盖面缺一块**。
- L07 的「未覆盖」声明在 executor 实现后过时。

**可访问性**（WCAG 2.2）

折叠导航失去可访问名称（SC 2.4.4 / 4.1.2）、四处对比度不达 AA（SC 1.4.3 / 1.4.11）、缺 skip link 与 `main` landmark（SC 2.4.1 / 1.3.1）、表单错误只在汇总不在字段（SC 3.3.1）、`document.title` 不随路由变化（SC 2.4.2）、两页各有两个 `h1`、长译文撑破布局。

**核心不变量**

- R04：`GET /api/v1/standards/:id` 与 `standard status` 给出不同答案，且 API 无单条例外端点。改为共用同一函数。
- R05：四处 `rollback` 无 post-Receipt——撤销之后，最后一条观测描述的是撤销之前、已不成立的状态。
- 诊断响应不指明所依据的 Receipt，因而无法核验。

## 未做的项与真实原因

原交接把这些归因于「离线无法下载依赖」，**那是错的**：本机 cargo registry 缓存完整，`rusqlite` 与 `tauri` 都实测能离线构建。真实原因如下：

| 项 | 原因 |
| --- | --- |
| #16 SQLite + FTS5 | 技术可行，但会把第三方依赖引入根 workspace，破坏「零依赖 + 16 条门禁离线通过」。这是**项目决策**，需要明确取舍：ADR 0001 冻结 SQLite 为目标引擎，而当前 JSON store 实现了相同的 DTO/ledger 合同，该偏离已被记录 |
| #14 E2EE sync | 阻塞在**协议选型**而非密码学库：产品指定 age/SOPS，其 crate 不在缓存中；用 `ring` 自行实现一套协议不等于实现 age/SOPS |
| #13 18-family 真实探测 | 需要真实安装外部 harness（非 cargo crate） |
| #11 Windows / Linux lane | 缺设备。壳的代码与 macOS lane 已验 |
| #17 8 人 QA | 人工验证不能代答 |
| WebView 内的键盘取证 | macOS 的 WKWebView 不支持 CDP，缺自动化通道 |

## 取证强度

[取证记录](2026-09-08-keyboard-and-responsive-verification.md)逐项标注了强度，未把做不到的记成通过。两处工具限制值得接手的人知道：

- 浏览器自动化注入的 `keydown` 其 `key` 与 `code` 为空，页面无从判断按了什么。Tab 不受影响（浏览器层处理），但 Enter/Space 的激活只能用合规事件验证。
- CDP 改视口后 `innerWidth` 与 `matchMedia().matches` 都已更新，却不派发 `resize` 或 `change`。断点切换因此只验到代码路径。

## 2026-09-12 原生运行与发行接续

本页较早记录中的“E2EE/oracle runner/daemon stop 未实现”为当时状态。当前新增外部 age/SSHSIG CLI transport、command Effect runner、真实 Codex/Grok observation、periodic-static daemon 和有所有权的停止流程；范围、证据与仍未完成项统一见 [本轮执行记录](../plan/2026-09-12-runtime-and-release-execution.md)。这些切片不等于完整 WP-07/WP-10/WP-12 或独立验收通过。
