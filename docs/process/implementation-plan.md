# 实施计划：F-01–F-18 × WP-01–WP-12

> 状态：规范；WP-02 已按切片启动，其余未进入阶段的运行时尚未实施。完成状态以各阶段有效交付记录为准。
> 这是完整交付的施工顺序，不是范围裁剪。禁止用 MVP、试验版或 TODO 占位符替代任何工作包。

Acceptance cutoff：`2026-09-04T23:59:59+08:00`。WP-02 不得在八份 `acceptance/` 工件通过独立评审前开始。

## 目标仓库布局（按工作包创建，不建空骨架）

```text
Cargo.toml                          workspace
crates/ctxpect-core                 IR, claims, reason codes
crates/ctxpect-schema               JSON Schema, canonicalization
crates/ctxpect-fs                   path containment, TOCTOU, archive
crates/ctxpect-redact               secret scan, redaction-before-log
crates/ctxpect-crypto               vault DEK wrap, age/SOPS adapter
crates/ctxpect-collect              environment + inventory collectors
crates/ctxpect-resolve              versioned resolvers
crates/ctxpect-evidence             evidence ledger
crates/ctxpect-receipt              receipt build/verify/sign
crates/ctxpect-doctor               deterministic doctor
crates/ctxpect-diff                 cross-coordinate diff
crates/ctxpect-store                SQLite+FTS5, migrations
crates/ctxpect-policy               context-specific policy
crates/ctxpect-projection           intent, authority binding, preview
crates/ctxpect-sync                 E2EE bundle, folder+git providers
crates/ctxpect-adapters-sdk         manifest, sandbox, fixture runner
crates/ctxpect-adapters             18 family modules
crates/ctxpect-importer             runtime importers + field-to-claim
crates/ctxpect-cli                  ctxpect binary
crates/ctxpect-daemon               optional daemon + localhost API
crates/ctxpect-advisor              AnalysisAdapter boundary
crates/ctxpect-effect               ExperimentContract + runner adapter
apps/desktop                        Tauri 2 shell
packages/ui                         React/TypeScript app
packages/ui-tokens                  design tokens from 02-design-system.md
```

已创建：`Cargo.toml`（workspace）、`crates/ctxpect-core`（claim 真值模型：轴、unknown reason code、10 条诚实性 invariant）、`crates/ctxpect-schema`（已声明范围内的 legacy canonical JSON 与内容摘要）。当前另有 `crates/ctxpect-fs`、`crates/ctxpect-collect` 的阶段实现，以及本切片新建的 `crates/ctxpect-resolve`、`crates/ctxpect-cli`；存在源码不表示已通过其完整验收；以相应阶段合同和有效交付记录为准。其余路径在对应工作包开始前不创建。

OS lanes：`macos-27-arm64`（已捕获）、`ubuntu-24.04-x86_64`（官方 `ubuntu-24.04.4-live-server-amd64.iso` SHA-256 已冻结）、`windows-11-24h2-x86_64`（官方 build `26100.9278` 已冻结；ISO digest 为 `digest-not-published-by-source`）。详见 `docs/research/2026-09-04-source-backed-coordinates.md` 与 `acceptance/artifact-digest-manifest.json`。

---

## WP-01 契约、Fixture 与威胁模型

覆盖：F-01（矩阵）、F-02（corpus 合同）、F-03（claim/capability）、F-05（schema）、F-11（integration pin）、F-17（schema）、F-18（policy 合同）。

| 项 | 落地 |
| --- | --- |
| Crates | 尚不编译产品；冻结 schema 文本于 `crates/ctxpect-schema` 计划路径与 `acceptance/` |
| Schemas | Receipt、adapter manifest、ExperimentContract、claim-validity、capability、projection |
| Commands | 本阶段 required gate：docs-structure `python3 scripts/check_docs.py`；acceptance-validation `python3 scripts/check_acceptance.py --structure`；traceability-validation `python3 scripts/check_acceptance.py --traceability`；corpus-validation `python3 scripts/check_acceptance.py --corpus`；semantic-team-validation `python3 scripts/check_semantic_team.py`；validator-negative-tests `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'`。WP-02 起追加 cargo-build `cargo build --workspace` 与 cargo-test `cargo test --workspace` 与 cargo-clippy `cargo clippy --workspace --all-targets`。生成器用 `python3 scripts/generate_acceptance.py`。 |
| Migrations | 规划 `0001_init.sql` 表清单，不执行 |
| Tests | 本阶段全部 required gate（名称+命令见本文件完成定义与 [test-strategy](test-strategy.md)）；extractor 零未追踪 |
| Screenshots | 无 |
| Security gates | 威胁模型、redaction contract、corpus 再分发规则 |
| OS lanes | 写入 compatibility-matrix；Ubuntu 24.04.4 live-server SHA-256 与 Windows 11 24H2 build 26100.9278 已冻结 |
| Release artifacts | 八份 acceptance 工件 + 文档集 |

验收：schema 可验证；fixtures 覆盖 include/exclude/override/cap/loss/unknown；无第二套术语；§17.0 工件固定 digest 并经独立评审。**本 foundation 阶段交付文档与合同，独立评审属于监督流程，不在本实施会话完成。**

---

## WP-02 Core Collector、Resolver 与 CLI

覆盖：F-01、F-02、F-03、F-04、F-08（Doctor 规则引擎）。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-core`, `fs`, `collect`, `resolve`, `doctor`, `receipt`, `adapters-sdk`, `adapters`, `cli` |
| Commands | `inspect`, `collect`, `doctor`, `inventory`, `preflight`, `launch` |
| Schemas | environment profile、inventory item、task probe |
| Migrations | 可选 ephemeral；允许无 DB one-shot |
| Tests | 每 required-supported 坐标 60 static；Codex/Grok 12 oracle recipes 真正执行后升级 live；未知版本 exit 3；shell/env injection 负例 |
| Screenshots | 无 |
| Security gates | 被动扫描不执行 hook/MCP；path containment；secret 不进 CLI 预览 |
| OS lanes | 先 macOS live；Linux/Windows 在 image digest 捕获后全量，禁止抽样 |
| Release artifacts | `ctxpect` 开发快照（内部），非产品完成 |

验收：fixtures 全绿；重复扫描 digest 稳定；未知版本 fail-closed；无 UI/daemon/账号完成核心检查。

2026-09-05 排程补充：按[统一交付方案](../plan/2026-09-05-contexpect-evidence-first-delivery.astra.md)接续现有 fs/collect → 单 anchor 可解释 inspect → 尽早受控原生对账 → 其余 anchor 分别扩展。下一切片只补它需要的规范与来源，ECC 按实际消费者的未覆盖缺口并入；不新开独立 ECC 批，也不修改活动 fs/collect 的冻结范围。只读核心验收后可按 PRD §18 准备早期形成性反馈。上述切片通过不等于完整 WP-02 完成，不豁免原 oracle、坐标、静态语料和安全门禁。

---

## WP-03 SQLite、Snapshot、Diff 与 Evidence Ledger

覆盖：F-05、F-07。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-store`, `evidence`, `diff` |
| Commands | `diff`, `receipt show\|verify\|export\|redact` |
| Migrations | `0001_init.sql` 起：devices、claims、evidence、receipts、edges、unknown_surfaces |
| Tests | 断电不损坏上次 snapshot；tombstone/delete；equivalence profile；无数值 confidence |
| Screenshots | 无 |
| Security gates | 对外 redacted digest；删除后派生失效 |
| OS lanes | 三 lane 的文件锁/WAL 行为 |
| Release artifacts | Receipt JSON Schema 稳定 major 0 或 1-rc |

---

## WP-04 Inspector UI 与本地 API

覆盖：F-06、F-04（preflight UI）、F-07（矩阵 UI）。

| 项 | 落地 |
| --- | --- |
| Packages | `packages/ui-tokens`, `packages/ui`, `apps/desktop` |
| Commands | Tauri commands allowlist；不直接扫盘 |
| Schemas | 前端 DTO 由 Receipt schema 生成 |
| Tests | WCAG、键盘、privacy screenshot mode；18 family DOM 断言 |
| Screenshots | `verification/doctor-1440x900.png` 等，对照 `images/10-context-doctor-final.png` |
| Security gates | CSP、纯文本默认 viewer、deep link 二次确认 |
| OS lanes | macOS/Linux/Windows WebView 差异按区域容差，而非整图相等 |
| Release artifacts | 内部桌面夜版 |

路由必须包含 Checkup、Doctor、Care Plan、Inspector、Compare、Receipts、Assets、Sessions/Monitor、Lab、Sync、Policy、Exceptions、Team compliance、Integrations、Standards、Settings。

---

## WP-05 Runtime Importer、Session 与 Daemon

覆盖：F-12、F-16（daemon/scheduler 部分）、F-04（preflight→runtime link）。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-importer`, `ctxpect-daemon` |
| Commands | `import`, `sessions`, `daemon start\|stop\|status` |
| Migrations | sessions、turns、runtime_events、compactions、tool_calls |
| Tests | 每个 importer 覆盖声明；Codex partial 不标 full；daemon 资源与通知去重；one-shot 仍独立 |
| Screenshots | Monitor quiet/drift |
| Security gates | 默认不代理流量；timeline 不补造事件 |
| OS lanes | 72h soak 在全部 required OS lane（WP-12 正式，本包先做短 soak） |
| Release artifacts | daemon 可选单元文件（launchd/systemd/Task Scheduler 模板） |

---

## WP-06 Intent、Projection、Apply 与 Rollback

覆盖：F-09。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-projection` + File/Git transaction executor |
| Commands | `intent validate\|show\|project\|preview`, `apply`, `rollback`, `align status\|diff` |
| Migrations | intents（CanonicalIntent）、intent_revisions、projection_plans、projection_outcomes、native_overlays、loss_reports、equivalence_bindings、apply_transactions、rollback_points |
| Tests | `projection-matrix.yaml` **每个** required-write cell 在适用 anchor OS/harness lane 上 preview/apply/rollback/post-Receipt；并发 hash；rollback 不删未受管文件；secret-bearing executor 门禁；semantic-team ST1–ST3 |
| Screenshots | `verification/care-plan-locked.png`, `verification/treatment-preview.png` |
| Security gates | executor argv/env/stdio/temp/backup/Git/crash 全过 secret gate |
| OS lanes | 四 anchor × 适用 OS，不得用一条代表路径代替矩阵 |
| Release artifacts | native plan schema |

Cursor user instructions 保持 export-only。Codex/Grok 无独立 scoped-rule primitive 时必须显示 loss。

---

## WP-07 加密多设备同步

覆盖：F-10。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-sync`, `ctxpect-crypto` |
| Commands | `sync preview\|apply\|status`, `standard validate\|publish\|preview\|adopt\|pin\|update\|status\|leave\|rollback\|revoke` |
| Migrations | sync_bundles、device_envelopes、sync_conflicts、team_context_standards、standard_revisions、standard_bindings |
| Tests | 无密钥不能读正文；secret 永不进 bundle；replay/rollback/分叉；recipient 移除；folder 禁止自动 merge；TeamContextStandard 签名/无 secret/rollback 保留个人文件 |
| Screenshots | Sync conflict 屏 |
| Security gates | 见 ADR 0003 与 PRD §13.5 |
| OS lanes | 设备 A/B 至少跨两个 lane 或两 profile |
| Release artifacts | bundle schema、provider contract |

---

## WP-08 生态资产与供应链

覆盖：F-11、F-08（安全类 Doctor 规则）。

| 项 | 落地 |
| --- | --- |
| Crates | adapters catalog 面；调用 APM/Git executor |
| Commands | `assets` |
| Migrations | adapters、adapter_sources、conformance_runs |
| Tests | 恶意包 corpus；无许可证阻止复制；hidden Unicode；path escape；SBOM 展示 |
| Screenshots | Assets / Integrations |
| Security gates | 安装 pin；不自建 marketplace backend |
| OS lanes | 路径/junction 负例 |
| Release artifacts | 无独立商店 |

---

## WP-09 LLM Advisor 与历史洞察

覆盖：F-13、F-14。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-advisor` |
| Commands | `advisor` |
| Migrations | advisor_suggestions、candidate_relations |
| Tests | 默认不联网；payload preview；建议不伪装 observed；删除历史后失效；observed-window 措辞 |
| Screenshots | Advisor consent modal |
| Security gates | egress allowlist、SSRF、不存原始响应 |
| OS lanes | 离线可用性 |
| Release artifacts | analysis bundle schema |

---

## WP-10 Context Effect Lab

覆盖：F-15。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-effect` |
| Commands | `experiment` |
| Migrations | task_suites、experiments、runs、gate_results、effect_observations |
| Tests | 预构造 beneficial/harmful/equivalent/inconclusive 与真实小型 runner fixture 必须匹配冻结 contract；单次前后对比不产生因果结论 |
| Screenshots | Lab 结果含 inconclusive |
| Security gates | 预算、sandbox、approval、工作目录隔离写入 executor receipt |
| OS lanes | 至少一条 live runner lane |
| Release artifacts | ExperimentContract schema |

---

## WP-11 Policy、CI、IDE/API 与集成收口

覆盖：F-16、F-17、F-18。

| 项 | 落地 |
| --- | --- |
| Crates | `ctxpect-policy`；CLI `ci`；adapter SDK 发布 |
| Commands | `policy eval\|show`, `exception request\|approve\|reject\|revoke\|status`, `ci`, `adapter test\|list` |
| Migrations | policies、revisions、bindings、evaluations、approvals、audit_events（append-only hash chain）、layer_assignments、exception_records、member_disclosures、leader_compliance_views |
| Tests | 0/2/3；tighten-only；双 authority 配置必须失败；deep link；import traversal；OTLP export；personal 不得 shadow required；stale exception fail-closed |
| Screenshots | Policy 评估与 Approval |
| Security gates | localhost API CSRF/origin；第三方 adapter 最小权限 |
| OS lanes | 安装/升级/迁移/卸载演练 |
| Release artifacts | CI 模板、IDE deep link schema、SBOM 生成器、跨平台安装包 |

---

## WP-12 全产品集成验收

覆盖：F-01–F-18 端到端；§17.1–17.5。

| 项 | 落地 |
| --- | --- |
| Tests | 内部可用性 8 人门禁；72h daemon soak；四周期保养；A→B 同步；50-session 历史；Effect Lab；安全/隐私/许可/性能/a11y/恢复/删除 |
| Screenshots | 全路由验证集 |
| Security gates | §17.3 全表 |
| OS lanes | compatibility-matrix 全部 required lane，禁止抽样 |
| Release artifacts | 签名发布候选 + SBOM + NOTICE。只有本包与独立 readback 都绿才可宣布完整交付 |

---

## F → 实现映射摘要

| Feature | WP | 主 crate / package | 主命令 | 主测试 |
| --- | --- | --- | --- | --- |
| F-01 | WP-01, WP-02 | collect, adapters | inspect, collect | compatibility fixtures; exit 3 |
| F-02 | WP-01, WP-02 | collect, fs | inventory | discovery F1; fs/security negatives |
| F-03 | WP-01, WP-02 | resolve, adapters | inspect | claim tuple; oracle recon |
| F-04 | WP-02, WP-04, WP-05 | cli, ui | preflight, launch | golden; injection; preflight link |
| F-05 | WP-01, WP-03 | receipt, evidence | receipt * | schema/sign/tombstone |
| F-06 | WP-04 | packages/ui | desktop routes | usability; WCAG; privacy shots |
| F-07 | WP-03, WP-04 | diff, ui | diff | golden diff; no numeric confidence |
| F-08 | WP-02, WP-08 | doctor | doctor, ci | doctor corpus; SARIF |
| F-09 | WP-06 | projection | intent, apply, rollback, align | full projection matrix; semantic-team ST1–ST3 |
| F-10 | WP-07 | sync, crypto | sync *, standard * | crypto vectors; fork/replay; TeamContextStandard |
| F-11 | WP-01, WP-08 | adapters, cli | assets | license/SBOM; malicious pkg |
| F-12 | WP-05 | importer, daemon | import, sessions | importer coverage; unknown honesty |
| F-13 | WP-09 | advisor | advisor | consent/egress |
| F-14 | WP-09 | advisor, store | sessions | wording; deletion |
| F-15 | WP-10 | effect | experiment | frozen contract fixtures |
| F-16 | WP-05, WP-11 | daemon, cli | daemon, ci | resources; exit contract; offline |
| F-17 | WP-01, WP-11 | schema, sdk | adapter test | schema compat; sandbox |
| F-18 | WP-01, WP-11 | policy | policy, exception, ci | precedence; audit chain; 0/2/3; detect-only honesty |

## 明确不做（实现时若出现即缺陷）

隐藏 prompt 提取、通用 marketplace、默认 MITM、健康总分、图数据库、required-write 降为 export-only、把 Unknown 猜成 pass。

## 本阶段完成定义

本阶段完成定义是下表全部 required gate（名称 + 命令必须一起列出）。条数随工作包推进而增加，以 `scripts/contexpect_contract.py` 的 `REQUIRED_GATES` 为准，本文件不复述数字。canonical 命令表亦见 [test-strategy](test-strategy.md)。

| 名称 | 命令 |
| --- | --- |
| docs-structure | `python3 scripts/check_docs.py` |
| acceptance-validation | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'` |
| cargo-build | `cargo build --workspace` |
| cargo-test | `cargo test --workspace` |
| cargo-clippy | `cargo clippy --workspace --all-targets` |

WP-01 的**文档与合同**部分在 foundation 完成：根开源文件、canonical docs、八份 acceptance 工件、生成夹具，以及上表前六条门禁。WP-02 第一刀（`ctxpect-core` 真值模型与 `ctxpect-schema` 规范化）追加 cargo-build、cargo-test 与 cargo-clippy 三条门禁；fs/collect 的后续切片已有阶段实现；本切片新建了 `ctxpect-resolve` 与 `ctxpect-cli`。存在源码不表示已通过其完整验收。`doctor` 等其余 crate 尚未开始。
