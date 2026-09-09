# 缺口分析（2026-09-08）

> 状态：分析记录，不是验收结论，也不是实施授权。
> 基线：`main` @ `9be2842`，工作区干净；本文写作时逐条重跑 13 条 required gate，退出码全为 0（见 §6）。
> 目的：把「计划里有但没写成文档」「文档写了但没实施」「实施了但仍有缺口」三类缺口分开列出，再给一张跨类别的优先级清单；每条给证据位置、影响与可判定的收口方式。
> 与 [交付状态](2026-09-08-delivery-status.md) 的关系：那份文档记录上一轮 review 清单 17 项的逐项状态；本文从 PRD、实施计划、补充方案与代码实际对照出发，覆盖那份清单之外的缺口，并指出其中几处被清单遮住的问题。
> 本文经两路独立复核（零上下文 Claude 子代理、Codex `gpt-6-astra` xhigh 只读会话），复核结论与采纳情况见 §7。
> **2026-09-08 实施后状态**：§8 逐条记录本次直接实施（S1–S6，当时为未提交候选，已随 2026-09-09 的提交入库）后各条目的新状态与证据；未在 §8 出现的条目状态不变。交叉 review 后的修正见 [交付状态](2026-09-08-delivery-status.md#2026-09-09-交叉-review-与修复)。

## 0. 一句话结论

产品已有一条可运行、诚实、有负例测试的**单 anchor 只读主链**，以及门禁、Receipt 迁移、policy 同源、审计链等基础设施。但**验收合同要求的绝大多数「量」尚未接上**：19 个 required-supported 静态坐标只实现了 1 个坐标的 1 个 capability（Codex instructions，该坐标 60 条 golden 里只有 3 条属于它）；1,924 条静态夹具只有约 11 条进入 Rust 测试；589 条 Doctor 语料（20 条规则）没有任何 Rust Doctor 消费者，且两边的规则 ID 命名空间不同；21 个 required-write projection cell 只有一个通用单文件 executor。此外复核发现**四处安全/事务缺口**（授权不绑定动作与目标、apply 的并发 guard 在产品路径上失效、rollback 非原子且不校验并发、导入「脱敏」只是截断），它们不在上一轮清单上。四项决定（存储引擎偏离何时结束、E2EE 依赖来源、例外身份通道是否永久 CLI-only、第三方依赖政策）没有完整决策记录，卡住 17 项清单里 #14/#16 两项。

## 1. 计划里有、但没有形成文档或合同工件的

| # | 缺口 | 计划出处 | 现状证据 | 影响 | 收口方式 |
| --- | --- | --- | --- | --- | --- |
| A1 | **Codex anchor 的 instructions grammar（G1–G5）没有 canonical 规范文档** | 补充方案 S1「冻结……合同」；evidence-first §3 P02「规则逐项绑定来源」 | 规则只存在于 `crates/ctxpect-resolve/src/lib.rs`、`crates/ctxpect-resolve/tests/grammar.rs`（G1–G5 共 11 个测试）与 `crates/ctxpect-cli/tests/inspect_cli.rs`；`docs/` 里没有任何文件定义 G1–G5；夹具的规则来源在 `scripts/generate_acceptance.py` 的 `official-spec` 字段 | 其余三个 anchor 要接入时没有可对照的规范格式；reviewer 无法独立核对「规则依据」 | 写 `docs/adapters/grammar/codex-cli-0.147.0-instructions.md`：每条规则的来源、范围、正负例 ID，来源回溯到生成器；作为后续每个 anchor grammar 的模板 |
| A2 | **本地 API 的合同分散且不完整**：分页、取消、`generation`/`stale`、错误 envelope、方法维度没有统一说明 | 补充方案 S1 第 3 条：「冻结……本地 API、redaction、版本迁移与取消/进度合同」 | `desktop-ui.md` 已有请求/安全/响应语义与页面契约，reason code 词表已在 `data-and-truth-model.md` 与 `reason.rs`；缺的是字段覆盖与一致性（C17、C18） | 新页面接入靠读源码 | 在 desktop-ui.md 补一节语义合同；把契约测试扩到比对 method。不另建生成文档 |
| A3 | **Receipt / adapter manifest / ExperimentContract 的 JSON Schema 工件不存在** | 实施计划 WP-01「Schemas：Receipt、adapter manifest、ExperimentContract、claim-validity、capability、projection」；WP-03「Receipt JSON Schema 稳定 major」 | 仓库内没有任何产品 `*.schema.json`；`ctxpect-schema` 提供 canonical JSON、sha256、HMAC 与 Python 边界对拍，但没有 schema 校验 | 「前端 DTO 由 Receipt schema 生成」（WP-04）无从做起；F-17 的 schema 兼容性无法测 | 先冻结 `ctxpect-receipt-v1` 与 `ctxpect-doctor-v1` 两份 schema 文本，加 schema 校验测试；evidence 字段（C19）随此一起定 |
| A4 | **四项决定的记录不完整** | delivery-status 明确写「这是项目决策，需要明确取舍」「阻塞在协议选型」 | (a) ADR 0001 已记录「JSON ledger 是偏离、SQLite 仍是目标」，但没写偏离结束条件与引入第三方 crate 的条件；ADR 正文在 `5cbeec2` 被就地改写，无 superseding 记录。(b) E2EE 的 age/SOPS pin 来源与获取方式无记录。(c) ADR 0005 已决定「身份源是仓内 principals、daemon 不承载」，未决的是它是否永久、Exceptions 页是否永久只读。(d) 根 workspace 是否允许第三方 crate 及离线 vendoring 方式无记录 | #14、#16 被卡；B10 的统计库、C7 的 Tauri 门禁都取决于 (d) | 不重开已有决定。新增 ADR 0006（第三方依赖政策与存储偏离结束条件，supersede 0001 的对应段）、ADR 0007（E2EE 依赖来源）；(c) 在 ADR 0005 现有小节下补「是否永久」的决定 |
| A5 | **8 人可用性门禁与早期形成性反馈的材料** | PRD §17.2 六项任务；§18；evidence-first §6 | 无 QA 任务书、fixture 项目、问卷或录像/trace 脱敏规则文档 | 阻塞是参与者，不是文档；材料可以先准备，但优先级低于 §4 全部条目 | 一份 `docs/process/usability-gate-protocol.md` 同时覆盖两者；不单独排产 |
| A6 | **daemon unit 模板与 CI 模板**（合同工件） | WP-05 release artifact「launchd/systemd/Task Scheduler 模板」；WP-11「CI 模板」 | 仓库无 `.github/workflows`、无任何 unit 文件；`operations.md` 自述「没有 daemon unit、没有 CI action 可安装」 | 定期/持续两种节奏（PRD §6）没有可交付入口 | 先交付 CI 模板（依赖 `ctxpect ci` 已有 exit 契约，但见 C30）；unit 模板随 daemon watcher 一起 |
| A7 | **traceability 没有「实现位置 / 测试」列** | 补充方案 §6：「在对应实现阶段补实现位置/测试与 UI 证据」 | `acceptance/traceability.csv` 列为 `test_or_gate; artifact`，值多为「security/privacy gates in WP-07/WP-11/WP-12」这类泛称；F-01 的 8 行 artifact 同时列了 collect、catalog、product_loops，其中 catalog 是静态表（此前独立评审已指出） | 「F-01 至 F-18 均有实现、文档、测试」（§17.5）无法机器核对 | 生成器增加 `implementation` 与 `evidence_test` 两列，每行逐条标 `partial` / `unimplemented` 与证据，不整项清零；`check_acceptance.py --traceability` 同步校验 |

## 2. 文档已写、但尚未实施的

按依赖顺序排列；「需外部条件」列表示实施前必须先有设备/授权/依赖。

| # | 缺口 | 规范出处 | 现状证据 | 需外部条件 |
| --- | --- | --- | --- | --- |
| B1 | **其余 required-supported 静态坐标与 capability 的 resolver**：19 个坐标里只做了 `codex/0.147.0/cli/macos` 的 `instructions` 一个 capability | compatibility-matrix `static_support: required-supported` 19 条；PRD §17.0「每个 required-supported 坐标 ≥60 golden」 | `ctxpect-resolve/src/lib.rs:52-97` 明列未覆盖 config fallback、cap override 与其他 capability，是单 anchor 常量；`args.rs` 默认 harness 同样写死；codex 坐标 60 条 golden 里 `instructions` 只有 3 条，其余 11 类 capability（skills、plugins、commands、agents、mcp-declarations、tool-schema-catalog、task-message、history、memory、environment-metadata、permission-description）无实现 | 无（静态夹具已在仓内） |
| B2 | **静态语料没有被 Rust 产品实现跑**：1,924 条静态输入中只有 2 正 1 负 + 8 条 honesty cell 进入 Rust 测试 | test-strategy「item discovery F1 ≥0.95；claim tuple accuracy ≥0.95」 | `crates/ctxpect-cli/tests/corpus_inspect.rs:106-180`；`corpus-validation` 门禁（`check_acceptance.py:686-740`）校验的是输入 digest 与 **Python golden parser** 对 golden 的自洽，不是 Rust 产品实现；`acceptance/corpus/development/oracle/*.jsonl`（34 条）与 `loss-cells.jsonl` 同样无 Rust 消费者 | 无 |
| B3 | **Doctor 20 条规则语料没有 Rust Doctor 消费者；Rust Doctor 规则集与语料命名空间不同** | PRD §17.1「doctor-corpus ≥250；可阻断规则 precision = 1.00」；语料 8 阻断（各 40 例）+ 12 非阻断（各 12 例）+ 125 条 clean | 语料 589 行、20 个 `rule_id`；Rust `ctxpect-doctor` 只有 6 个 `D-*` 规则，来自 inspect 输出；`doctor-corpus.jsonl` 只被 Python 脚本读取。`archive_traversal` / `passive_scan_exec` 两类夹具是**惰性声明**（`archive-manifest.json` 条目含 `..`、`hooks.json` 的 `on_scan` 非空，见 `contexpect_fixtures.py:917-926`），跑它们不需要解压或执行能力 | 无 |
| B4 | **projection-matrix 21 个 required-write cell 逐格 preview/apply/rollback/post-Receipt** | PRD F-09、§17.4；WP-06 | 只有一条通用单文件写路径（见 C3）；没有 per-harness native projection（如 Cursor scoped rules、MCP config 合并）、没有 loss 合同、APM authority 的 4 个 cell 无 executor；`projection-matrix.yaml` 的 `native_target.path_glob` 无消费者 | APM cell 需要 integration pin（见 A4-(b) 同类问题） |
| B5 | **E2EE sync、recipient、key rotation、恢复、vault 生命周期** | F-10、§13.2.1、§13.5、`security/encrypted-sync-protocol.md`、ADR 0003 | `ctxpect-sync` 只有 folder bundle + replay 拒绝；`ctxpect-crypto` crate 不存在；settings `vault` 可设为 `required`，但设了之后 sync 报 `sync.encryption_unavailable`，即「可设不可用」 | 决策 A4-(b)(d)；age/SOPS pin |
| B6 | **18-family 真实探测**（executable / App bundle / 版本 / surface / home roots） | F-01；PRD §11 | `crates/ctxpect-cli/src/catalog.rs` 是静态表；非 codex 全部 unknown；全仓无进程探测 | 真实安装外部 harness（部分本机已有：Codex、Claude Code、Cursor、Grok，见 local-environment receipt） |
| B7 | **两个冻结 oracle 的受控执行**（Codex `debug prompt-input`、Grok `inspect --json`） | evidence-first §5「首个 resolver 能运行后尽早安排」；test-strategy「≥12 oracle cases」 | `acceptance/corpus/live/oracle-recipes.jsonl` 2 条，`executed: false` | 本机已装 Codex 0.147.0；需一次显式授权在临时项目执行（AGENTS.md 不变量 2、被动扫描零执行） |
| B8 | **原生 importer 与 field-to-claim** | F-12；WP-05；`acceptance/field-to-claim/` 24 份草稿 | `ctxpect-importer` 只接受通用 `events[]`（五种 type，含 `tool` 与 `compaction`）；缺的是原生字段映射、tool-call/skill 语义与 Inspector 图层；24 份 YAML 无消费者 | 需要真实 session 样本（可用合成） |
| B9 | **daemon 的 watcher、增量 snapshot、去重通知、scheduler、通知 adapter（desktop / terminal / webhook）** | F-16；`operations.md` | `/monitor` 按需重算；无 watcher；UI 显示 `notifications.unimplemented`；`notifications` 只是 settings 里的一个布尔键；`daemon_rss_mb` 不执行 | 无 |
| B10 | **Effect Lab 的 versioned runner 与统计合同** | F-15（alpha≤0.05、power≥0.80、ITT、multiplicity、混杂锁定）；WP-10 | 已有的是四决策骨架与 n 锁定（C2/C24）；缺 runner adapter 与统计库 | 决策 A4-(d)：零依赖政策 vs「冻结的成熟统计库」 |
| B11 | **Advisor 的真实 AnalysisAdapter 与 egress allowlist** | F-13；`llm-advisor-and-effect-lab.md` | 已有 consent 门与 payload preview 骨架；settings 只接受 `none`；全仓无出站网络代码，因此也无 allowlist/SSRF 实现（Advisor 对 adapter 参数的虚报见 C29） | 付费模型调用未授权（可先做本地 command adapter） |
| B12 | **Receipt 的 SignerAdapter（SSH/GPG/minisign/Sigstore）** | §13.4；ADR 0003 | 只有 store 内 HMAC 连续性签名，`org_identity: false` | 无（可先做 SSH 签名 adapter） |
| B13 | **多数 UI 入口仍是 JSON 转储** | 补充方案 §2 V01–V16；desktop-ui.md 自述 | 顶层入口：V06 Sessions/Monitor、V07 Lab、V10 Policy、V11 Standards、V13 Exceptions、V14 Team compliance 为纯 `StateView` 转储（`App.tsx:263-282`）；V01/V02/V03/V05/V08/V15/V16 是「设计 + 尾部转储」；V04 明细/V09/V12 按页设计。六个 `:id` 明细页全部为 `EntityPage` 转储 | 无 |
| B14 | **C08 截图证据、screen reader、Tauri Windows/Linux、≤2s 性能** | C06/C08 | `06-implementation-plan.md:64-69` 六个 Doctor 截图目标仍 `not started`，`browser-screenshots/` 为空；键盘取证仅浏览器预览 | 设备与辅助技术 |
| B15 | **CLI 未实施的全局参数与能力**：`--config`、`--privacy`、`--allow-unknown`、`--force`、`--sarif`、`launch --execute`、CI Markdown summary、SSH collector、配置文件查找顺序 | cli-reference；configuration.md | `args.rs` `UNIMPLEMENTED_FLAGS` / `LISTED_COMMAND_FLAGS`；`configuration.md` 自述「本阶段不提供可加载的用户配置文件」；`--offline` 只写进 envelope 不改行为 | 无 |
| B16 | **archive traversal 的真实解压安全、压缩炸弹、Windows junction** | §13.3、§17.3；fs-collect 合同显式 out_of_scope | 未实现（与 B3 的惰性 grammar fixture 是两回事） | Windows lane 需设备 |
| B17 | **性能数据生成器与三 lane 性能门禁** | §14.2；`reference-hardware.md`「generator:medium-repo:v1 content digest awaits generator implementation」 | 生成器未实现 | 三台参考硬件 |
| B18 | **安装包、签名、更新 denylist、卸载演练** | `release.md`；WP-11 | `tauri.conf.json` 有 bundle 段但未做产物；无签名；`release.md` 发布前检查 6 项无对应脚本 | 发布授权 |
| B19 | **WP-12：72h soak、1,000 变更、4 周期、50-session、全 lane** | §17.2.1、§17.4 | 全部未执行 | 设备、参与者、时间 |
| B20 | **adapter SDK、IDE deep link、OTLP / SARIF / CycloneDX-SPDX 导出**（F-17） | F-17；WP-11 | 本地 API 已存在（部分实施，见 C17/C28）；SDK 与 deep link 未实现；导出格式只有 JSON | 无 |
| B21 | **sealed corpus 的答案不存在、无托管人** | test-strategy「sealed 答案在 RC 冻结前不可供实现者读取」 | `acceptance/corpus/sealed/stubs.jsonl` 60 条 stub；`generate_acceptance.py` 的 `sealed_stubs()` 只产 `answers_status: withheld-until-rc`、`origin: independent-maintainer-slot`，不产生任何答案；仓内没有托管记录 | 需要独立维护者 |
| B22 | **stable asset identity 与 identity corpus**（managed/unmanaged、rename/copy/fork、inode、跨设备、key rotation、低熵内容） | PRD §10.4、§17.1 | `corpus-manifest.json` 无 identity 类；`collect` 的 `FileIdentity` 是 per-scan inode 且不参与判定 | 无 |
| B23 | **第三方 adapter 进程隔离 / WASI** | F-17；ADR 0004；`adapters/authoring-guide.md` | 无 adapter 加载机制；`adapter list` 只读静态 catalog | 无 |
| B24 | **malicious package corpus** | WP-08；`traceability.csv` 引用它 | `acceptance/` 内无任何 malicious fixture | 无 |

## 3. 已实施、但仍有缺口的

### 3.1 授权、事务与隐私（复核新增，优先级最高）

| # | 缺口 | 证据 | 为什么算缺口 | 收口方式 |
| --- | --- | --- | --- | --- |
| C31 | **Approval 不绑定动作、目标与范围**：`authorize_mutation` 只要 store 里存在任一存活的 approved 例外就放行，不接收当前 action / project / target / 使用次数 | `ctxpect-policy/src/lib.rs:167-220`（找到第一条 `grants: true` 即 break）；`dispatch.rs:144` 加载全部例外 | 一条为 A 项目 A 动作批准的例外可用于任何 mutation；违反 F-18 Approval 的 scope/single-use 与 R05 | `authorize_mutation` 增加 `(action, project_digest, target_rel)` 输入，例外记录携带 covering scope；补跨项目、跨动作负例 |
| C32 | **apply 的并发 guard 在产品路径上失效**：CLI 与 API 在 apply 时**重新生成** preview，再拿这份新 preview 的 `current_digest` 与文件比对；用户看到的 preview 与实际 apply 的没有绑定 | `dispatch.rs:881-895`（`proj_preview` 在 `apply` 分支内再次调用）；`http.rs:1378` 同法；`ctxpect-projection/src/lib.rs:141-147` 只比较传入 preview 的摘要 | `projection.concurrent_hash` 只有单元测试能触发；用户预览后文件被改，apply 仍以「新摘要」通过；违反 F-09 / L03「hash 已变化仍覆盖」反例 | 持久化 preview（tx_id + current_digest + desired_digest），apply 只接受 `--tx <id>` 并用持久化的 preimage 校验 |
| C33 | **rollback 不校验并发且非原子；新建文件回滚成空文件** | `ctxpect-projection/src/lib.rs:176-200`：读 `before` 直接写回，不比较当前内容与 `after_digest`，也不记录原文件是否存在；`apply`（`:155-171`）先写目标再写 `tx.json`；`ctxpect-assets/src/lib.rs:330-350` 同类 | 违反 §17.3「rollback 保留用户并发修改」与 §14.3「apply 原子或可恢复；崩溃后能判断 committed/rolled-back/in-doubt」；中途失败会留下已改目标和不完整事务 | rollback 先比对 `after_digest`，不符则停止；事务记录含 `existed_before`，不存在则删除；写入用 temp + rename，`tx.json` 先于目标落盘并带状态 |
| C34 | **importer 的「脱敏」只是截取前 40 字符**；正文与 secret 会进入 metadata-only store | `ctxpect-importer/src/lib.rs:92-98` `redact_preview`；`dispatch.rs:741`、`http.rs:357` 持久化结果 | 违反 §17.3「测试 secret 不出现在 store」与 metadata-only 默认；C05 的 UI 遮罩只覆盖带 `data-mask` 的元素，Session 明细页的 JSON 转储会直接展示这些内容 | 接入现有 `redact.rs` 的路径/secret 规则；metadata-only 下不保存正文预览，只保存长度与摘要 |
| C35 | **projection 没有 secret 门**：`desired` 与原文可明文进入目标与 backup；`secret_scan` 只在 `standard publish` 使用 | `dispatch.rs:349-352, 1058`；`ctxpect-projection/src/lib.rs:155-161` | PRD F-09「secret-bearing executor path 必须被禁用」与 §17.3 executor artifact 门禁未成立 | apply 前对 `desired` 与 backup 内容跑 secret gate，命中即拒绝 |
| C36 | **随机源缺失时连续性密钥静默退化为时间哈希** | `ctxpect-store/src/lib.rs:592-608` `fill_random`：`/dev/urandom` 打不开就用纳秒时间戳的 sha256 填充 | Windows 常规路径没有 `/dev/urandom`，会进入此分支；MAC 密钥可预测而不报错 | 使用平台安全随机源；不可用时 fail-closed 并给 reason code |
| C37 | **签名不覆盖时间且时间字段按名剥离**：signature 无签名时间；`digest_body` 递归删除任何名为 `time`、`now`、`timestamp` 等键 | `ctxpect-receipt/src/lib.rs:244-256, 405-415`；`ctxpect-schema/src/json.rs:172-186` `TIME_KEYS` | 与 §13.4「签名时间非可信边界但需记录」不符；以 `time` 为名的业务字段会被排除出摘要而无 schema 声明 | schema 显式声明 display-only 字段清单，摘要按 schema 而非键名剥离；signature 加 `signed_at` |
| C38 | **CI 不执行 policy gate**：`ci_cmd` 只做 inspect + diagnose，不调用 `authorize_mutation` / `evaluate`；`policy eval --from` 恒 exit 0 | `dispatch.rs:1557-1580`、`:1495-1521` | PRD F-18「policy pass = 所有 required checks 确定通过」在 CI 入口不成立；不能用 `ctxpect ci` 证明 policy 通过 | `ci` 接入 store 有效 policy 并按 0/2/3 出码；明确 `policy eval --from` 是查询而非 gate |

### 3.2 其它已实施项的缺口

| # | 缺口 | 证据 | 为什么算缺口 | 收口方式 |
| --- | --- | --- | --- | --- |
| C1 | **Doctor 规则 ID 双轨，且 `ci` 不读 `--fail-on`** | Rust 用 `D-UNKNOWN-SURFACE` 等 6 个，rule_id 进入产品输出（`ctxpect-doctor/src/lib.rs:249`）；语料与 PRD 用 `secret_literal` 等 20 个；`ci_cmd` 不读 `--fail-on`，confirmed > 0 即 exit 2；`--fail-on` 只在 `doctor_cmd`（`dispatch.rs:512`）生效 | 「可阻断规则 precision = 1.00」无法度量；CI exit 2 的语义与冻结语料脱节 | 先建规则语义 → grammar → fixture → 实现的映射表；语料 20 条作为新规则实现，只有语义相同的才对现有 `D-*` 做别名迁移，不整体重命名以免改变已发布 finding id；`ci` 与 `doctor` 共用同一阻断判定 |
| C2 | **Effect `decide()` 不满足 F-15 合同字段** | `crates/ctxpect-effect/src/lib.rs:21-42`：无 alpha/power、无 ITT、无 multiplicity、无混杂锁定；`mean()` 为整数均值 | 四种 decision 的「supported-*」缺预注册判据即可产出，违反 F-15「前三者必须同时满足预注册判据」 | 在依赖决策前，先把 contract 扩到 F-15 全字段，缺统计实现的判据一律返回 `inconclusive` 并写明原因 |
| C3 | **Projection 的 `Intent` 没有语义字段，loss 与投影结果没有封闭枚举** | `ctxpect-projection/src/lib.rs:27-32` `Intent { intent_id, authority, target_rel, desired }`，13 个 `canonical_intent_fields` 只覆盖 id/authority 两个；`:108-115` loss 是自由字符串，唯一启发式是 `target_rel` 含 `cursor` 且含 `user` → `export-only`；`:217` `native_diff` 是字节数；PRD 的投影结果 7 值枚举与 loss 5 分类未建模 | 与 ST1「CanonicalIntent 不是文件副本」冲突；preview 展示的 loss 没有封闭语义，UI 无法据此禁用 apply。B4 的 21 个 cell 都建立在这个结构上 | 先定义 CanonicalIntent 结构与两个枚举（未知一律 `unknown`），单文件 executor 作为 `project-file` cell 的实现保留 |
| C4 | **没有任何可到达 `verified` 的对账路径**：sync 的 `semantic` / `reconciliation` 在 apply 结果里是常量 `structural-only` / `indeterminate`；diff 的 `verified` 恒 false | `ctxpect-sync/src/lib.rs:128-137`；`ctxpect-diff/src/lib.rs:187`。diff 的 `reconciliation` 三态（indeterminate / structural-only / failed）是诚实结果，不算缺口 | §17.1「semantic reconciliation 四状态」缺 `verified` 一侧 | 获得 native evidence（B7）与完成 verified 判定路径是两件事：后者还需 intent / plan / transaction / policy digest 绑定（PRD F-09 第 516 行），分别验收 |
| C5 | **store 密钥与审计链的信任边界只写在 Rust 文档里，规范文档未同步** | `store/src/lib.rs:414-419` 注释已写明持有 store 密钥者可重写整条链；`store/keys/continuity.key` 明文 0600 与 ledger 同目录 | `privacy-and-threat-model.md` 没有 store 威胁模型；用户文档只说「不是组织签名」 | 把该段同步进 `privacy-and-threat-model.md`（含 C28 的 daemon 无 token、C36 的随机源）；随 B12 引入外部 signer 后再升级 |
| C6 | **ui-unit 不渲染组件；没有 UI 交互 E2E 门禁** | `packages/ui/tests/` 六个测试全部只读源码文本或纯模块；补充方案 §7.2 要求的「UI route/interaction E2E」不在 `REQUIRED_GATES` | 页面契约（C04）由纯模块测试守住，但页面是否按契约渲染没有测试；取证记录里的键盘/响应式结果无法回归 | 用 `react-dom/server` 渲染每个路由**契约声明为适用**的状态做展示断言（不机械生成 16×10，合同禁止伪状态）；需先经 `tsc` / `vite build --ssr` 产出 JS，用 `MemoryRouter`；SSR 不执行取数 effect，交互 E2E 仍是独立未完成项 |
| C7 | **Tauri 壳没有门禁** | test-strategy 仍写「Tauri 桌面壳尚未创建，因此没有 Tauri required gate」；壳已存在于独立 workspace | 壳的 URL 校验只有手工实测记录，没有自动测试 | `daemon_url()` 抽成可测函数加单测；`cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml` 是否进 required gate 取决于 A4-(d) |
| C8 | **importer 通用化过头；API 导入把 session id 写死** | `import_session` 对任意 `mapping_id` 都接受同一套事件；`POST /api/v1/sessions/import`（`http.rs:357-363`）把 id 写死为 `"s_api"`、mapping 写死为 `"generic-json"`，`put_named` 允许替换，每次导入覆盖上一份 | 「每个 importer 覆盖声明」（WP-05）不成立；API 导入两次只剩一份 | mapping_id 必须对应 `acceptance/field-to-claim/*.yaml` 的一份，否则 `import.mapping_unknown`；API 导入从请求体取 id 或按内容摘要生成，补两份不同 session 连续导入用例 |
| C9 | **integrations / assets API 的坐标写死为 codex** | `http.rs:308-317`：`GET /api/v1/integrations` 与 `/assets`、`/assets/:id` 都调用 `integrations_json("codex", true)` / `family_entry(id, "codex", true)` | API 不随请求坐标变化；C01「切换坐标」在这两个端点上不可能成立 | 从会话当前坐标取值 |
| C10 | **Sync API 只对 `<store>/sync/folder`，Git provider 缺** | desktop-ui.md「Sync API 的目标是固定的」 | F-10 要求 folder + Git 两种 provider | Git provider 可用 `git` 子进程实现，缺失时显示 capability gap |
| C11 | **当前规范页的状态语句互相矛盾** | `AGENTS.md:60`、`implementation-plan.md:39`、`test-strategy.md:86` 仍写「Tauri 桌面壳尚未创建」；`architecture/overview.md:108` 写「仓库中没有 crates/ 运行时、没有 Tauri 窗口、没有可执行 ctxpect」；`cli-reference.md:220`「当前可运行的不是 CLI」；`operations.md` 末尾「可运行的只有文档门禁脚本」；`README.md:58-59`「其余 17 个 adapter family……diff/store/daemon/UI 都尚未实施」与同文件下方状态表矛盾；`desktop-ui.md` 两段自相矛盾（C18） | 与 R04 同一性质：同一个问题在不同文档得到不同答案 | 只改**当前规范页**的句子并链接 [交付状态](2026-09-08-delivery-status.md)；历史记录（plan/archive/review）不动。不建黑名单式 lint |
| C12 | **`docs/plan` 控制记录状态不可判断** | `contexpect-wp02-fs-collect-cycle.json` 仍 `IN_PROGRESS`；仓内没有该阶段的收口记录 | 接手者无法从仓内判断这条线是在途还是已结束 | 不凭推断改写终态（AGENTS.md 禁止追溯改写历史报告）；在 `docs/plan` 加一份来源明确的迁移索引，指明控制记录已迁到仓外归档 |
| C13 | **`resource_limits.scan_files` 只在 `collect --store` 时读取** | `dispatch.rs:417-426`：限额从 `args.store` 打开的 settings 读；不传 `--store` 时遍历无上限。inspect 的 named inventory 不限额是明确设计（`ctxpect-resolve/src/lib.rs:616-647`），cli-reference 已写明只约束 `collect` | cli-reference 没说「只在有 store 时」 | 文档写明条件，或无 store 时用内置默认上限 |
| C14 | **子命令别名问题在 `standard` 之外仍有三处**：`intent validate\|show\|project\|preview` 四个子命令不读 `subcommand`，都返回同一份 preview；`sync status` 与 `sync preview` 同一分支、同一 body，且 `command` 字段都写 `"sync preview"`；`align status\|diff` 在无 `--left/--right` 时不读子命令，`command` 恒为 `"align status"` | `dispatch.rs:861-884`、`:821-859`、`:1523-1539` | 与已修的「`standard` 五个子命令是别名」同一缺陷机制；cli-reference 把它们列为不同操作 | 各子命令有各自的语义与输出，或在 cli-reference 明确「本切片等价」并让 `command` 字段如实回显 |
| C15 | **三处「看起来像结果的常量」**：`adapter test` 返回 `{ran: true, families: 18, live_oracle_executed: false}`，实际没有运行任何 fixture；`daemon stop` 只删除 `daemon.pid` 就报 `stopped: true`，不向进程发信号；`daemon status` 只看 pid 文件是否存在 | `dispatch.rs:1541-1555`、`:793-819` | 与已修的「四个常量端点」同类：`ran: true`、`stopped: true` 是对未发生动作的肯定断言 | `adapter test` 在 fixture runner 存在前返回 `adapter.test_unimplemented`；`daemon stop` 读 pid 发送终止信号并等待退出，或如实报 `pid_file_removed_only` |
| C16 | **`inventory` 与 `collect` 是同一函数** | `dispatch.rs:43` `"collect" \| "inventory" => collect_cmd` | cli-reference 把 `inventory` 描述为「资产目录，不含动态伪事件」，与 `collect` 不是一回事 | 文档改为「本切片为别名」，或补独立实现 |
| C17 | **`/api/v1/care-plan/:id` 接受任意 HTTP 方法**；`PUT`/`POST` `/settings` 并存但契约只声明 `POST`；`page-state.test.mjs` 对前缀路由把方法记为 `*`，因此「声明的 method + path 必须是真实路由」对前缀路由的方法不成立 | `http.rs:429, 1270-1272`；`page-state.test.mjs:156-167` | desktop-ui.md 声称契约测试「写不出幻觉 API」，对方法维度不成立 | 路由表按 `(method, path)` 精确匹配；契约测试对前缀路由也比对方法 |
| C18 | **desktop-ui.md 两处不一致**：「只有 `/compare`、`/doctor`、`/receipts`、`/settings`、`/sync` 声明了动作」（5 个），而测试与下一节的动作面都包含 `/assets`（6 个）；「主要路径」漏列实现已有的 8 条路径（`/coordinate`、`/receipts/:id`、`/sessions/:id`、`/sessions/import`、`/assets/:id`、`/integrations/:id`、`/lab/:id`、`/collect`）与 2 处方法（`POST /lab`、`PUT /settings`） | `page-state.test.mjs:232-239`；`http.rs:288-437` | 文档与测试给出不同答案 | 改文档；随 A2 |
| C19 | **Evidence ledger 只有最小结构**：`Evidence { root_kind, path, content_digest }`，没有 evidence id、采集方式、时间、collector/resolver 版本、质量等级；claim 不引用 evidence id | `crates/ctxpect-resolve/src/lib.rs:178-184`；`inspect.rs` 的 `evidence_to_json` 只输出三个字段 | F-05「每个 claim 引用 evidence id」与 WP-03 evidence ledger 未成立 | 与 A3 schema 一起定 evidence 的完整字段与 id 规则；本文不预定公式 |
| C20 | **六类 Receipt 只产生两类；`ci` 与 `collect` 不持久化 Receipt** | `persist_inspect` 生产调用 6 处（`dispatch.rs:528,894,997`；`http.rs:492,706,1402`）全是 `"one-shot"`，另 1 处 `"preflight"`（`dispatch.rs:665`）；`ci_cmd` 与 `collect_cmd` 不调用它 | runtime / post-session / device-baseline / ci 四类从不生成；`ctxpect ci` 的判定无 Receipt 可核验；Compare 无法区分 baseline 与普通快照 | `ci` 生成并持久化 `ci` kind；`collect` 产出 `device-baseline` 或 `runtime` kind |
| C21 | **policy 规则模型缺三态与 revision**：只有 `required: bool + effect`，PRD 的 `recommended \| required \| prohibited` 三态、PolicyRevision（digest/签名/生效期）、PolicyBinding 均不存在；`lower_relaxes` 只对 `user`/`session` 层给出显式错误码，team/project 层放宽时没有专门错误（高层 deny 仍留在集合中，verdict 仍为 deny，因此**不构成假 pass**） | `crates/ctxpect-policy/src/lib.rs:71-77, 87-118, 150-160` | F-18 的 PolicyRevision/Binding、三态与「personal 只能覆盖 recommended」无法表达 | 三态与 revision 随 WP-11；tighten-only 的错误码扩展到所有低层 |
| C22 | **例外记录的 scope / 期限是 CLI 硬编码**：`exception request` 写入 `scope = "project"`、`expires_at = 9_999_999_999`；`exception_status` 在 CLI 侧的 `offline_fresh` 恒为 `true`；记录缺 reason / created_at / single-use，而 `--reason` 已被 `args.rs` 解析却不写入 | `dispatch.rs:1253`、`:1039`；`ctxpect-policy/src/lib.rs:238-250, 274-283`；`args.rs:330` | 「过期 fail-closed」「离线 stale 不放行」只有单元测试能触发；F-18 Approval 字段缺一半（与 C31 同根） | `exception request` 必填 `--expires-in`，把已解析的 `--reason` 写进记录；`offline_fresh` 由 policy 来源的刷新时间计算 |
| C23 | **TeamContextStandard 输出 7 个顶层键，与合同 20 个 `team_standard_fields` 不逐项对应**；`leave` / `rollback` / `revoke` 三个子命令行为相同（都是删除） | `dispatch.rs:1068-1081`、`:1198`；`acceptance/semantic-team-contract.yaml` `team_standard_fields` | 无 publisher / semantic_version / compatibility_floors / target_harness_coordinates / canonical_intent_set，「伪造 publisher」无法检测；rollback 应回到上一 revision 而不是删除 | 做字段逐项映射表；先补 publisher 与 revision 并进签名 payload；rollback 需要 revision 历史，随 B4 |
| C24 | **Effect Lab 的实验结果由构造方式预先决定**：`experiment_cmd` 把 control 全部传 `false`、treatment 全部传 `true`，`run_local_instructions_probe` 只返回 0/1，`margin` 写死为 1，因此 delta 恒为 1、决策恒为 `supported-equivalent-within-margin`（n=1 时 inconclusive）；`runs` 恒为空数组 | `dispatch.rs:1468-1492`；`ctxpect-effect/src/lib.rs:73-77, 97, 110-112` | 产出的「结果」不是观测 | 在 runner adapter 出现前，`experiment` 只接受外部提供的逐 run 结果文件，不自造 |
| C25 | **Advisor 候选与 Session insight 是常量文本**：candidates 恒为一条硬编码句子；`insight()` 只包装调用方传入的固定字符串 | `ctxpect-advisor/src/lib.rs:67-73`；`ctxpect-importer/src/lib.rs:100-107`、`dispatch.rs:746` | consent / non-authoritative 边界是真的，但「建议」与「洞察」本身不存在；F-14 九类分析零实现 | 与 B11 同步：先做本地 command adapter 与至少一条基于 timeline 的确定性 insight |
| C26 | **Doctor 的 PlacementRecommendation 与 `first_seen` 是占位**：placement 恒 `{authority: "project-file", loss: "none-until-preview"}`；`first_seen` 恒 `"current-receipt"` | `ctxpect-doctor/src/lib.rs:257, 294-298` | F-08 P25、F-07「drift 追溯首次出现」无实现 | placement 在没有历史与载体规则前返回 `unknown` + reason |
| C27 | **四个 settings 字段被存储、校验，但产品从不读取**：`retention_days`、`copy_confirm`、`privacy_mode`、`screenshot_privacy` 在 `crates/*/src`（settings.rs 之外）与 `packages/ui/src` 没有任何读取点；`unenforced` 只列了 `daemon_rss_mb` | `grep -rl` 逐项为空 | 与 delivery-status 已记的「声明与实现脱节」同类 | 要么接入（隐私两项应由 UI 从 settings 读取），要么进 `unenforced` 列表 |
| C28 | **daemon 是单线程阻塞服务，无 token / Unix socket** | `http.rs:40-111` `listener.incoming()` 逐连接处理 | F-17「随机 token 或 Unix socket」未做，目前只靠 loopback + Host/Origin/CSRF 头；一个慢请求阻塞 UI 全部请求 | 写进威胁模型（C5）；token 随 A4-(c) 一起决定 |
| C29 | **Advisor 对未实现的 adapter 参数照单回显**：传入任意 `adapter` 字符串即在输出 `adapter` 字段回显，而没有任何对应实现 | `ctxpect-advisor/src/lib.rs:46-63` | 输出声称了一条不存在的分析路径；settings 侧已限制为 `none`，CLI 侧没有 | 非 `none` 一律返回 `advisor.adapter_unavailable` |
| C30 | **Task Probe 不按 task/files 计算 eligibility；session 不绑定 preflight id** | `dispatch.rs:646-662` 只把 `--files` 拼进 argv，inspect 输入不含 task/files；`ctxpect-importer/src/lib.rs:59` 无 preflight 字段 | F-04「确定性计算 eligible」与 L02「session 引用 preflight」只有 Receipt id 一层成立 | 条件解析接入 resolver（随 B1 的 capability 扩展）；importer 接受 `preflight_id` 并写入 session |
| C31–C38 | 见 §3.1 | | | |
| C39 | **Diff 只有约 3/10 个维度，baseline lock 只是布尔** | `ctxpect-diff/src/lib.rs:106-155, 191-197`：比较坐标、文件摘要与六 facet truth；无 scope / activation / semantic intent / permission / secret ref / loss / budget / evidence quality；`baseline_allowed` 不持久化 | F-07 维度清单与「baseline lock + CI」未成立 | 随 C3 的 Intent 结构与 C20 的 device-baseline Receipt 一起 |
| C40 | **UI Doctor 流程的两次请求没有 generation / Receipt id 绑定** | `App.tsx:70-95`：`POST /inspect` 后紧接 `GET /doctor`，不带刚返回的 `receipt_id`；`useResource`（`:938-950`）有 AbortController，但 Doctor 页这条链不经它 | C01「切换坐标后旧请求不能覆盖新坐标结果」在主入口未成立 | `GET /doctor?receipt_id=` 绑定；旧请求结果按 generation 丢弃 |

### 3.3 补充方案 L01–L11 闭环与本文条目的映射

| 闭环 | 当前状态 | 对应缺口 |
| --- | --- | --- |
| L01 首次使用与环境 | 单 anchor instructions 静态部分成立；探测全 unknown | B1、B6、C9 |
| L02 任务前与运行时 | preflight → launch_argv → Receipt id 引用在 CLI 成立；不执行、无运行时 | C30、B15、B8 |
| L03 诊断与修复 | 单文件 preview/apply/rollback/post-Receipt 成立，但 guard 与事务有洞 | C31、C32、C33、C35、C3、B4、C26 |
| L04 Receipt 与 Diff | 迁移、tombstone、baseline 布尔成立 | C19、C20、C37、C39、B12 |
| L05 设备迁移与同步 | folder bundle + replay 拒绝 | B5、C4、C10 |
| L06 团队标准与治理 | 签名/验签、采纳状态成立 | C21、C22、C23、C31 |
| L07 生态资产 | 复制 executor 成立，事务同 C33 | B24、C9、B4（APM cell）、C33 |
| L08 Session 与历史 | 导入/删除失效成立；脱敏不成立 | C34、C8、C25、B8 |
| L09 Advisor | consent 门成立 | B11、C25、C29 |
| L10 Effect Lab | 四决策与 n 锁定成立 | C2、C24、B10 |
| L11 持续、定期、CI | CI exit 契约成立，但无 policy gate、无 Receipt | C38、C1、C20、B9、A6 |

## 4. 优先级清单（跨类别）

这不是第四类缺口：下面每一项都能在 PRD、实施计划或补充方案里找到依据，它们只是按「价值 / 成本 / 无外部依赖」排出的先后。P1–P2 不需要外部设备或授权。

| 优先级 | 项 | 为什么现在做 | 做法 | 验收 |
| --- | --- | --- | --- | --- |
| P1 | **修 §3.1 的 C31–C35**（授权绑定、preview 绑定、rollback 校验与原子性、导入脱敏、projection secret 门） | 都是 R05/§17.3 红线，且不依赖任何决策或设备；上一轮清单没有它们 | 各补负例先转红再修；C32/C33 需要持久化的 preview 与带状态的事务记录 | `product_loops.rs` 新增 L03/L08 负例通过 |
| P1 | **静态语料一致性门禁（corpus conformance runner）** | B1/B2 的共同前提；现在的 `corpus-validation` 只证明 Python golden parser 自洽。全部 1,924 条 `input_path` 已核实存在 | Rust 集成测试：遍历 `acceptance/corpus/development/static/*.jsonl`，用 compatibility-matrix 判定坐标支持等级，逐条比对 `expected_output` / `expected_claim`；按坐标 × capability 报告分母、通过、失败与未实现三类计数（未实现不能靠 honesty 分支算通过，Unknown 按 PRD §17.1 计分）。Doctor 语料那一半等 C1 的映射表（P2） | 门禁名 `corpus-conformance`；当前只有 codex instructions 3 条 + honesty cell 可通过，其余以「未实现」计入分母 |
| P1 | **决策记录（A4）** | 卡住 #14/#16 和 B10 | ADR 0006/0007，ADR 0005 现有小节补一句；不重开已定结论，只补依赖引入条件、来源/pin 与偏离结束条件 | `check_docs.py` 通过；delivery-status 引用它们。ADR 存在不解除实现阻塞 |
| P1 | **第二个 anchor：Claude Code 2.1.259 instructions 切片** | 已有 fixture（`claude-code__cli.jsonl`，其中 `instructions` 6 条）、本机安装记录；是从「单 anchor」到「跨工具对齐」的最小一步，也是 8 人门禁任务 1 的前提 | 先把 resolver 从单 anchor 常量改成多 anchor 分发（B1）；写 grammar 文档（A1 模板）；实现 instructions 分支。**切片验收 = 6 条 instructions 通过 + 其余 54 条如实报未实现**；完成整个坐标需实现其余 capability，不能只扩充 instructions 用例 | runner 中该坐标 instructions 6/6，其余计入未实现 |
| P2 | **Doctor 规则映射表 + 逐条实现 + doctor-corpus runner** | C1 | 20 个语料规则 → 语义 / 输入合同 / 实现状态 / 阻断性；先接能从现有 inspect+collect 输出判定的规则（`secret_literal`、`hidden_unicode`、`symlink_escape`、`path_containment_escape`、`cap_truncation`、`version_incompatible`、`undiscoverable_path`、`duplicate`、`conflict`、`stale`、`oversized_resident`、`bad_frontmatter`、`gitignore_mismatch`），惰性声明类（`archive_traversal`、`passive_scan_exec`、`unapproved_lossy_projection`、`required_asset_missing`）读声明文件；其余标 `not-implemented`。Python golden parser 不能替代被验产品 | 每条规则在 doctor-corpus 上的 precision/recall 可打印；阻断规则按合同零误报 |
| P2 | **文档状态一致性（C11、C18）+ traceability 实现列（A7）** | 成本极低，直接减少接手误判 | 只改当前规范页；生成器加列并同步校验器，逐行 partial/unimplemented | `check_docs.py`、`check_acceptance.py --traceability` 通过 |
| P2 | **UI 展示层渲染测试（C6）** | 现有取证靠人工浏览器，回归靠肉眼 | 先加编译步骤，再用 `react-dom/server` + `MemoryRouter` 覆盖各页契约声明为适用的状态；交互 E2E 保留为未完成项 | 进 `ui-unit` |
| P3 | **Codex oracle 受控执行（B7）** | 本机条件已具备，只差一次显式授权 | 按 evidence-first §5：核版本、临时项目、隔离 roots、记录命令/版本/输出摘要；结果进 live 入口，保留生成 fixture 的 `live_tested: false` | 一条真实 live 记录，含脱敏证据；它只是 native evidence，不等于 `verified`（C4） |
| P3 | **CI 工作流（A6）** | 现在完全没有非 macOS 证据 | 模板可离线写；跑 9 条 cargo/python 门禁（前端四条需另配环境）；需 push 授权，远端能否绿在本地无法预知 | 远端 required checks 绿；未绿前只算「已提交模板」，不算 lane 已验 |

## 5. 明确不建议做的（避免过度设计）

- 不为「零依赖」再造 SQLite、age、统计库的替代实现；这些是 A4 决策问题，不是编码问题。
- 不重开 ADR 0001/0005 已定的结论；只补它们没写的条件与结束判据。
- 不在决策 A4-(c) 之前给 daemon 加 HTTP 身份通道；`api.identity_required` 与 Exceptions 页只读是 ADR 0005 已记录的决定，不是缺陷。剩余的只是 PRD §9.1 第 13 项的文字与之不一致，随 C11 一起改。
- 不整体重命名现有 `D-*` Doctor 规则（会改变已发布 finding id）；按语义做映射与别名。
- 不为跑惰性 Doctor 夹具给 collect 加解压或执行能力；真实 archive/executor 安全合同独立实现（B16）。
- 不为 sealed corpus 立刻建独立仓库或托管人流程；先把「答案不存在、未托管」写清（B21）。
- ~~不为 C6 引入 Playwright 之类新依赖~~（2026-09-09 用户决定并由 [ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md) 替代：`@playwright/test` 作为 `packages/ui` dev 依赖，`ui-e2e` 为可选门禁）；服务端渲染测试继续覆盖契约声明的状态，也不机械生成合同不适用的状态。
- 不凭推断改写 `docs/plan` 历史控制记录的终态（C12）；不建过时语句黑名单 lint（C11）。
- 不把 B14/B17/B19 这些需要设备与人的项拆成更细的文档任务；A5 同理，排在 §4 全部条目之后。

## 6. 本文的验证边界

- 13 条 required gate 于 `9be2842` 逐条单独取退出码全 0；命令与 `AGENTS.md` 一致。`ui-build` 会改写 `packages/ui/dist` 的 sourcemap，已还原，不属于本文改动。
- 代码事实来自直接读取源码与 `grep`；数量（19 坐标、1,924 输入、codex 坐标 60 条中 3 条 instructions、claude-code 60 条中 6 条 instructions、589 Doctor 行、20 条 Doctor 规则、21 required-write cell、20 个 team_standard_fields）来自对 `acceptance/` 工件的脚本统计。
- 本文没有执行任何 harness、oracle、网络访问或安装。C36 的 Windows 分支未实机运行，结论来自代码路径。

## 7. 交叉复核记录

两路复核读的是同一份任务书：逐条核验证据、找疏漏、指出过度设计、判断可实施性与分类。两份原始报告留在会话 scratchpad（含本机绝对路径，不入库）。

### 7.1 Claude 零上下文子代理（只读）

采纳并已改：头部引用了不存在的「日志」与「复核」节；A1 漏引 `grammar.rs`；A3 贬低了 `ctxpect-schema`；A4 的 (c) 忽略了 ADR 0005 已有决定、且引用了仓外评审编号；原 A8「答案可算出」不成立，改为 B21「答案不存在」；B5 vault「只能 metadata-only」改为「可设不可用」；B13 数字口径；B14 引用位置；B15 去掉已实现的 `adapter list`；C1 关于 `ci --fail-on` 的描述；原 C3/C27 合并；C4 把 diff 三态从缺口中剔除；原 C9（例外 UI 只读）移入 §5；C12/C13 只保留可核事实；C14 补「无 `--left/--right`」条件；C15 合并 daemon stop；C20 调用点数量与「ci 不产生 Receipt」；C22 `--reason` 已解析；C23 字段数；C24/C25/C26 行号；新增 C8 的 `s_api`、C9 坐标写死、C27 settings 未读字段、L01–L11 映射、B22–B24；A2 降为扩展契约测试；C11 去掉黑名单；A5/A9 合并降级；C19 不预定公式；P1 静态/Doctor 两半拆开；P1-3 与 P2-3 补工作量说明；P3-2 标注需授权且结果未知。

未采纳：无。

### 7.2 Codex `gpt-6-astra`（effort xhigh，`-s read-only`，`turn.completed`，退出码 0，历时 1,581 s）

采纳并已改：§0/B1 覆盖率口径从「1/19 坐标」改为「1 个坐标的 1 个 capability（60 条里 3 条）」；B2 修正样本分类（2 正 1 负）与 `corpus-validation` 的真实能力（Python golden parser 自洽）；B3 去掉「rule_id 字符串 0 次」的说法，改为「无 Doctor 消费者」，并纠正惰性夹具不需要解压/执行能力（同时进 §5）；B8 承认通用 importer 已有 `tool`/`compaction` 事件；B11 拆出 C29（Advisor 回显未实现 adapter）；B20 把本地 API 从「未实施」中分离；C4 收窄为「缺 verified 路径」，并区分 native evidence 与 verified 判定（O7）；C5 改为「规范文档未同步」；C16 删去 `import`（有独立分支）；C18 区分路径与方法；C21 **撤回「假 pass 路径」**——高层 deny 仍留在集合、verdict 仍为 deny，保留三态/revision 缺口；C23 改为「7 个顶层键 vs 20 项合同」；C24 修正默认结果为 equivalent 而非 beneficial；行号修正（C3 `:217`、C25 `:746`、C26 `:257/:294`）；P1 runner 改用 `expected_claim` 与 compatibility-matrix 关联，未实现项不得靠 honesty 变通过；P1-3 Claude 切片验收改为 6 条 instructions；P2-1 不整体重命名 `D-*`（O2）；P2-3 SSR 只测契约适用状态、不替代交互 E2E（O4）；C11/C12 不动历史记录、不建黑名单（O5）；A7 逐行 partial 而非整项清零（O6）；A4 不重开已有决定（O1）；§4 改名为跨类别优先级清单（K6）。新增：C31（Approval 不绑定动作/目标）、C32（apply 重新生成 preview，并发 guard 失效）、C33（rollback 非原子且不校验并发、新文件回滚成空文件）、C34（导入脱敏只是截断）、C35（projection 无 secret 门）、C36（随机源退化）、C37（签名时间与时间字段剥离）、C38（CI 无 policy gate）、C30（Task Probe 不算 eligibility）、C39（diff 维度）、C40（UI Doctor 链无 generation 绑定）。

部分采纳：C13——Codex 认为「已被文档覆盖，应删除」；实际 cli-reference 未写「只在 `--store` 时生效」，保留这一窄条目。N09「tombstone 验签会 manifest mismatch」——`receipt verify` 已先检测 tombstone 并返回 `receipt.tombstoned`（exit 3），不会走到 mismatch；但它确实无法表达「签名仍有效、附件已删」，并入 C20 的 Receipt 生命周期工作，不单列。N12/N15/N17/N18 与 B22/B4/C27/C8 重复，不再单列。

未采纳：无实质拒绝项。

## 8. 实施后状态（2026-09-08，未提交候选）

按 [IMPL-PROMPT 的六个阶段](2026-09-08-delivery-status.md#2026-09-08-缺口分析实施s1s6)直接实施。「已收口」= 有转红负例与通过证据；「部分」= 实现了交接范围内的部分，剩余写明。

| 条目 | 新状态 | 证据 / 剩余 |
| --- | --- | --- |
| C31 | 已收口 | `authorize_mutation` 增加 `MutationScope{action, project_digest, target}`，例外记录携带三者，`*` 只允许作 target；负例 `an_exception_for_one_project_and_action_does_not_authorize_another`。单次使用（single-use）未做 |
| C32 | 已收口 | `intent preview` 持久化 `previews/<tx_id>`（含 `current_digest`、`existed_before`、desired）；`apply --tx` / `POST /api/v1/apply {tx_id}` 只接受持久化预览；负例 `apply_is_bound_to_the_persisted_preview_not_a_recomputed_one` |
| C33 | 已收口 | `tx.json` pending → committed → rolled-back，先于目标落盘；目标经 temp+rename；rollback 比对 `after_digest`，冲突 `projection.rollback_conflict`，新建文件回滚为删除；assets executor 同步；负例 `rollback_preserves_a_later_edit_and_removes_a_created_file`、`the_transaction_record_lands_before_the_target`、`a_target_edited_after_the_copy_is_not_rolled_over` |
| C34 | 已收口 | importer metadata-only：只存长度与摘要，未知类型不回显；`BodyPolicy::RedactedPreview` 经调用方 redactor（产品当前不启用）；mapping 须为 field-to-claim 之一；负例 `import_keeps_no_secret_or_path_bytes_in_the_store` |
| C35 | 已收口 | `ctxpect_doctor::secret_literal` 作 projection secret 门（desired 与 backup 内容），`standard publish` 共用；负例 `a_secret_bearing_apply_is_refused_before_any_backup_exists` |
| C36 | 已收口 | `fill_random` 只读平台安全随机源，失败 `store.random_unavailable`，非 Unix lane fail-closed；单测 `a_missing_random_source_fails_closed_instead_of_degrading`。Windows 真机未跑 |
| C37 | 已收口（2026-09-09，[核验记录](2026-09-09-deepseek-harness-review.md) T1(c)） | `signature.signed_at` 新增；MAC 覆盖 `manifest.digest + created_at + signed_at`；`manifest.digest` 按 schema `x-display-only` 路径剥离而非 `TIME_KEYS` 键名；旧 Receipt 验签 → `receipt.signature_legacy`（exit 3）。负例 `ctxpect-receipt::the_signature_covers_created_at_and_signed_at_but_the_digest_does_not`、`the_digest_strips_only_schema_declared_paths_not_key_names`；入口级 `product_loops::ci_and_collect_persist_their_kinds_and_verify_covers_the_times` |
| C38 | 已收口 | `ci` 合并 inspect / Doctor 阻断 / store policy 三个出码（无 policy → 3，deny → 2），不创建 store；`policy eval --from` 仍为查询；负例 `ci_gates_on_store_policy_and_never_reads_no_policy_as_pass` |
| C22 | 已收口 | `exception request` 必填 `--action`、`--expires-in`，写入 `--reason`、`created_at`；`offline_fresh` 由 policy 来源文件 mtime 计算（30 天）；负例 `exception_request_requires_a_lifetime_and_the_exception_expires`、`an_exception_over_a_stale_policy_source_does_not_grant` |
| C1 | 已收口 | [doctor-rule-map.md](doctor-rule-map.md)；`blocking_exit` 为 `ci` 与 `doctor --fail-on` 共用判定；`D-TRUNCATED` 别名 `cap_truncation`，不重命名 |
| C8 | 已收口 | API 导入 id 来自体或内容摘要，mapping 校验；`api_routes_are_exact_and_follow_the_session_coordinate` |
| C9 | 已收口 | AppState 会话坐标随 `POST /inspect` 移动，catalog 端点据此回答；同上测试 |
| C11 | 已收口 | AGENTS.md、implementation-plan、test-strategy、architecture/overview、cli-reference、operations、README 的矛盾句改正并链接交付状态；PRD §9.1 第 13 项**未改**（改 PRD 会变更 traceability golden 行） |
| C12 | 已收口 | [`docs/plan` 迁移索引](../plan/2026-09-08-control-record-migration-index.md)；未改任何历史记录 |
| C13 | 已收口（文档） | cli-reference 写明 `scan_files` 只在 `collect --store` 时读取；未加无 store 默认上限 |
| C14 | 已收口 | `intent validate/show/project/preview`、`sync status/preview/apply`、`align status/diff` 各自语义，`command` 如实；`subcommands_are_distinct_operations_not_aliases` |
| C15 | 已收口 | `adapter test` 先改为诚实的 `adapter.test_unimplemented`（exit 3），T3 后真实运行语料：`adapter.family_unimplemented`（exit 3）/ `adapter.rows_failed`（exit 2）/ `adapter.corpus_missing`（exit 1）；`daemon status` 以健康探测判断；`daemon stop` 报 `pid_file_removed_only`（不发信号，daemon 仍可达时 exit 3）；`constants_are_replaced_by_honest_unimplemented_answers` |
| C16 | 已收口（文档） | cli-reference 写明 `inventory` 为 `collect` 别名 |
| C17 | 已收口 | `ROUTE_TABLE` 按 (method, path) 精确匹配，未路由方法 405 `api.method_not_allowed`；契约测试按方法比对并含 DELETE 负例 |
| C18 | 已收口 | desktop-ui：六个动作页、43 条路由按方法列出（T4 加 `GET /sessions/:id/requests` 后为 44 条，文档同步）、语义合同一节（A2） |
| C27 | 已收口 | `unenforced` 列出 `daemon_rss_mb`、`retention_days`、`copy_confirm`、`privacy_mode`、`screenshot_privacy` 及原因 |
| C29 | 已收口 | 非 `none` adapter → `advisor.adapter_unavailable`；`advisor_refuses_an_unimplemented_adapter` |
| C40 | 已收口 | UI Doctor 链传 `receipt_id` 并按 generation 丢弃过期结果；API 侧 `GET /doctor?receipt_id=<旧>` 不被新 inspect 覆盖（测试同 C8） |
| B1（Claude Code instructions） | 部分 | 第二个 anchor 的 `instructions` 切片完成（6/6），resolver 多 anchor 表驱动；该坐标其余 54 条与其它 17 个坐标仍未实现，`corpus-conformance` 如实计数 |
| B2 | 已收口（门禁） | `corpus-conformance` required gate：1,924 行全部进入 Rust 产品实现，按坐标 × capability 报告四类计数；Python golden parser 不再是唯一消费者 |
| B3 | 已收口（门禁） | `doctor-corpus` required gate：589 行进入 Rust Doctor，20 条规则逐条 precision/recall；命名空间以别名衔接 |
| A1 | 已收口 | `docs/adapters/grammar/codex-cli-0.147.0-instructions.md`（G1–G6，来源回溯到台账 §6.1 与生成器的 `official-spec`） |
| A2 | 已收口 | desktop-ui「语义合同（A2）」一节 + 契约测试按方法比对 |
| A3 | 已收口 | `docs/schemas/ctxpect-receipt-v1.schema.json`、`ctxpect-doctor-v1.schema.json`；`ctxpect_schema::validate` 子集校验器；`schema_conformance.rs` 对真实 Receipt / tombstone / Doctor 输出通过、漂移拒绝。evidence 完整字段（C19）未随此扩展 |
| A7 | 已收口 | traceability 新增 `implementation`、`evidence_test` 两列（243 partial / 7 unimplemented，路径存在性由 `--traceability` 校验）；`acceptance/` 改动只是 traceability 加列与其 digest |
| C6 | 已收口（展示层） | `render.test.mjs`：SSR 渲染 23 条路由首屏与各页契约声明适用的状态横幅，不适用状态被标为矛盾；交互 E2E 于 2026-09-09 由可选门禁 `ui-e2e`（Playwright，[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md)）覆盖，不进 required 表 |
| A4 | 部分（2026-09-09） | [ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md) 已建：Rust 零依赖保持、估计器仓内冻结、SQLite 偏离结束条件、Playwright 可选门禁、DSH runner 暂不执行；ADR 0007（E2EE 依赖来源）与 ADR 0005「是否永久」仍未建 |
| C2（2026-09-09，T2） | 已收口 | `ExperimentContract` 扩到 F-15 全字段（primary outcome、margin、pairing、n_planned、alpha、power、multiplicity、ITT、混杂锁定、frozen_at、失效条件），缺字段 `effect.contract_invalid`；协议偏离各有原因码且一律 `inconclusive`；`mean()` 整数均值删除，估计经 `Estimator` trait；产品实现为仓内冻结的 `paired-exact-binomial-v1`（精确 McNemar/符号检验 + Clopper–Pearson 等价区间，ADR 0006），不支持的判据 `effect.estimator_inconclusive` 并输出 detail；2026-09-09 交叉 review 发现 v1 的等价区间把不一致率当已知值，已升为 `paired-exact-binomial-v2`（见交付状态「交叉 review」一节）。单测 `the_frozen_estimator_reaches_every_decision_for_the_right_reasons`、`the_exact_binomial_matches_known_values`；注入 estimator 的 `tests/estimator.rs` |
| C24（2026-09-09，T2） | 已收口 | `run_local_instructions_probe` / `preconstructed()` 从产品路径删除；`experiment` 无 `--runs` → `executed: false` + `effect.runs_required`，exit 3，不写 store；`POST /api/v1/lab` 同义。负例 `product_loops::l10_single_pair_not_causal_and_n_locked`（重写）；UI `/lab`、`/lab/:id` 显示未执行与原因（`render.test.mjs`） |
| C20（2026-09-09，T1(d)） | 部分 | `ci --store` 持久化 `ci` kind、`collect --store` 持久化 `device-baseline` kind（清单条目为 evidence），`RECEIPT_KINDS` 不变；`runtime` / `post-session` 仍无生产者（需要真实 runtime 采集）。负例 `ci_and_collect_persist_their_kinds_and_verify_covers_the_times` |
| C19（2026-09-09，T4(d)） | 部分 | 仅 DSH 原生 importer 所需最小字段：Receipt/session 的 evidence 条目带 `evidence_id = sha256(input digest ":" seq)`、`collection: import`、`importer_version`、`event_type`、`content_digest`；`collected_at` / `quality` / `root` / `path` 标 `unknown` 或 null；claim 引用 `evidence_id`。静态 resolver 的 `Evidence` 结构与 Receipt schema 的 `$defs/evidence` 未扩展 |
| B10（2026-09-09） | 部分 | 统计合同与冻结估计器已实施（`paired-exact-binomial-v1`，ADR 0006）；versioned runner adapter 未做（DSH 真实执行暂不授权执行） |
| B8（2026-09-09，T4） | 部分 | `deepseek-harness-cli` 原生 importer：pinned 输入合同（未压缩未打包 JSONL，其它编码拒绝）、header fold / surface fold 的 Rust 移植（每个函数注释引用 DSH 源文件）、精确 partial/Unknown（`seq_discontinuity` / `import_parse_failed` / `request_prefix_incomplete` / `provenance_incomplete` / tail closers）、field-to-claim 扩展、`GET /api/v1/sessions/:id/requests`、幂等导入与删除级联、`/sessions/:id` 请求证据页；`native_conformance` required gate。其余 23 个 mapping 仍只有 `declared-static-path`；tool-call/skill 语义与 Inspector 图层未做 |
