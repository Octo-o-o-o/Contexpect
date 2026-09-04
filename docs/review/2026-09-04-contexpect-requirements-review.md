# Contexpect 需求文档独立评审记录

> Review type：零上下文、只读产品/技术方案复核  
> Reviewer：Codex `gpt-5.6-sol`，reasoning effort `max`  
> 执行时间：2026-09-04  
> 首轮终态：RED  
> 原始事件证据：`turn.completed`；input 790,511 tokens（cached 711,296），output 21,657，reasoning output 15,731  
> 本文件保存 reviewer 结论和修订闭环，不代表产品代码实施验收。

## 首轮结论

Persona、使用节奏、产品边界、隐私默认值和完整交付原则覆盖较强，范围大本身不是 RED 原因；但存在 6 个 P1，当前版本不宜作为可直接实施的需求基线。

## P1 blockers

1. `R1-source-traceability`：无法独立证明完整吸收两份外部 AI 方案。只有摘要，没有原文不可变身份及 `来源命题 → 处置 → 需求` 的追踪。
2. `R1-claim-model`：六级状态随解析坐标变化，却被放在 ContextItem 自身；Expected、Observed partial、Exact/Estimated、Inferred、Unknown 把 claim 类型、覆盖度、精度、来源、知识状态混成同一轴。
3. `R1-unknown-version`：未知 harness 版本一处允许最近 adapter + warn，另一处要求 fail closed，存在 CI exit 0 假绿路径。
4. `R1-acceptance`：`≥95% reconciliation` 缺 corpus、分母、算法、Unknown 规则和 OS/version matrix；用户任务门禁缺起始状态、人数和通过阈值；F→WP→Gate 未完整映射。
5. `R1-effect-contract`：OutcomeAffecting 缺主指标、最小效应、设计、样本停止、多重比较和失败处理，可能把噪声写成因果事实。
6. `R1-crypto-contract`：Receipt 签名和 E2EE 缺 signer/trust/revocation/覆盖范围、replay/rollback/equivocation 和设备失陷语义；对外裸 hash 可能成为小文件内容确认 oracle。

## P2 findings

1. “永未激活/从未使用”要统一限定在已导入、可观察窗口。
2. 内部全功能可用性门禁与正式用户验证的顺序要唯一。
3. 不可变 Receipt 与本地/派生删除、tombstone、外部副本的语义要一致。
4. 外部集成方式必须在 WP-01 冻结为 CLI/API/file import/borrow，并有不可用 fallback。
5. semantic equivalence 应拆成 declared/structurally verified/behaviorally verified/user confirmed，删除裸数值 confidence。
6. 官方规范 URL 要保存 digest、访问时间、适用版本；开源来源保存 commit，受许可约束。

## Reviewer 建议删除的过度设计

1. 不自建完整 projection/apply/snapshot/rollback executor；保留 Intent、loss、preview、调用审计和 apply 后 Receipt，把 mutation 委托给明确集成 executor。
2. 不自建 package install/update/disable/lock/SBOM 生命周期；由 APM/Git executor 完成，Contexpect 做 catalog、风险、policy 和结果验证。
3. 不自建 WebDAV transport、账号式设备控制面和新密码协议；实现标准 encrypted bundle、设备成员/冲突语义和 provider adapter，复用成熟加密库和用户已有传输。
4. 不自建通用 agent runner/sandbox/statistics platform；Effect Lab 拥有 experiment contract、treatment、Receipt 和解释，执行委托已有 runner。

## Reviewer 指出的补充缺口

- 增加“单工具但复杂仓库”的普通开发者 persona。
- 增加 TOCTOU symlink swap、FIFO/socket/device file、权限变化、watcher race、压缩炸弹。
- preflight/启动命令增加 shell quoting、恶意路径/环境变量、secret 注入和日志脱敏。
- registry/update/webhook/LLM/external adapter 增加 egress allowlist、SSRF、proxy/TLS、目标认证。
- fixture/corpus 增加再分发许可、脱敏和 provenance。

## 首轮验证证据

```text
Runner status: completed
Codex event: turn.completed
Review verdict: RED
Product files modified by reviewer: none（read-only）
```

## 修订状态

首轮 RED 保留，不覆盖。第一次修订已完成：

| Finding | Resolution | 证据位置 |
|---|---|---|
| R1-source-traceability | 当时新增两份原文路径、大小、SHA-256 和 24 条来源命题逐项去向；后续扩为 26 条 | `docs/research/source-snapshots.md`；需求文档头部链接 |
| R1-claim-model | 新增多轴 `ContextStateClaim`，从 ContextItem 删除坐标相关状态和裸 confidence | 需求 §4.4、§10.1–§10.6、§9.2 |
| R1-unknown-version | 未知版本权威 claim 一律 indeterminate/unknown；CI 默认 exit 3；预览不进入 baseline | F-01、F-16、§11.2、§17.0–17.4 |
| R1-acceptance | 冻结 acceptance artifacts、OS/harness anchor、corpus 数量、F1/tuple 公式、内部 8 人门禁和 F→WP→Gate | §17.0–§17.6 |
| R1-effect-contract | 增加预注册 primary outcome、最小效应、power/alpha、随机/配对、停止、多重比较、失败处理和四种结论 | F-15 |
| R1-crypto-contract | 增加 signer/trust/revocation/canonical envelope/keyed digest、E2EE authenticity/replay/rollback/equivocation/设备失陷语义 | §13.4–§13.6、§17.3 |

P2 也已处理：观察窗口措辞统一；内部门禁与外部验证分离；Receipt tombstone/delete 语义明确；integration contract 在 WP-01 冻结；semantic equivalence 拆为四种可解释状态；adapter source 增加 digest/commit/license。

过度设计已收缩：mutation/rollback 委托 executor；package 生命周期委托 APM/Git；sync 只定义加密 bundle 和 provider contract；Effect Lab 不自建 runner/sandbox。

补充缺口已加入：单工具复杂仓库 persona；TOCTOU/FIFO/watcher/压缩炸弹；shell/env/secret 启动边界；统一 egress/SSRF/TLS；fixture/corpus 许可与脱敏。

机械核验：17 个 `F-*`、12 个 `WP-*`、24 个来源命题；禁用/旧术语检查为 0；两份附件 SHA-256 与 source snapshot 一致。

## 第二轮复审

> Reviewer：全新零上下文 Codex `gpt-5.6-sol/max`  
> 事件：`turn.completed`  
> 用量：input 1,593,783（cached 1,451,520），output 29,191，reasoning output 22,652  
> Verdict：RED，无 P0，有 5 个 P1

### P1 blockers

1. `R2-use-evidence`：`Used` 把调用、引用、行为一致和不可观察的模型内部使用混成事实；缺少合法 claim tuple、最低证据等级和冲突优先级。
2. `R2-policy`：企业 persona 和 CI 承诺 policy pass，但没有 Policy/Revision/Binding/Approval/AuditEvent、信任和离线语义。
3. `R2-version-cutoff`：WP-01 说冻结版本，验收又要求 code freeze 最新版，目标会移动。
4. `R2-projection-authority`：Contexpect 自己的 projection/loss 与外部 executor plan/loss 可能双重权威；外部 executor 的临时文件、备份、环境和 stdout/stderr 尚未进入 secret gate。
5. `R2-webview`：产品展示不可信 Markdown/路径/URL，却缺少 XSS、CSP、导航、deep link 和 Tauri bridge 边界。

### P2 与缺口

- 增加 “Where should this live?” 去向分类。
- 把完整本机 anchor 命令/输出持久化。
- 给 Doctor 阻断规则定义误报门槛。
- 增加 cadence-specific E2E 与 daemon soak。
- 冻结 stable identity、copy/move/fork、digest/key rotation 语义。
- 把“CPU 接近 0”和“有意义结果”改成可测指标。
- 补 remote collector、可信时间边界、临时文件/备份/crash/swap/indexer/卸载残留、产品与 adapter/executor 更新供应链。

### 第二轮过度设计收缩建议

- Receipt 签名不自建设备 PKI，优先复用 SSH/GPG/minisign/Sigstore/组织 trust store。
- E2EE 不自建协议，复用 age/SOPS、OS keystore、Git/现有文件传输；不承诺没有透明日志时的全局 equivocation 防护。
- LLM Advisor 不维护完整 provider 管理面，改成 adapter 或脱敏分析包。

第二轮 RED 保留。对应修订和第三轮终审结果追加在下方。

## 第二轮修订

| Finding | Resolution | 证据位置 |
|---|---|---|
| R2-use-evidence | 第五级改为 UseEvidence，拆 invocation/reference/behavior-consistent/internal-attribution；增加合法 claim tuple 与冲突优先级 | 需求 §4.1、§4.4–4.5、F-12、§17.0 |
| R2-policy | 新增 F-18，定义 PolicyRevision/Binding/Evaluation、tighten-only、Approval、offline、audit 和 policy pass | F-18、§10.8、WP-11、§17.3–17.6 |
| R2-version-cutoff | required scope 固定在 2026-09-04 23:59:59+08:00；之后版本仅 smoke/unknown-honesty，升级须显式修订合同 | WP-01、§17.0 |
| R2-projection-authority | 每个 target transaction 唯一 ProjectionAuthority；Contexpect 不生成第二份 native projection；executor 全 artifact 进入 secret gate | F-09、§15、WP-06、§17.3 |
| R2-webview | 增加不可信内容、纯文本默认、安全 Markdown、CSP、bridge allowlist、导航/外链/deep-link 合同与安全 corpus | §13.1、§13.7、§17.3 |

P2 已处理：新增 P25 PlacementRecommendation；新增完整本机环境 Receipt；Doctor corpus/precision/recall 和 blocking precision；72 小时 daemon 与 cadence E2E；Stable identity/copy/move/fork/key rotation；CPU/P95 与 meaningful UI 定义。

补充缺口已处理：remote/container/CI collector 和 SSH host verification；签名时间非可信边界；temp/crash/backup/swap/indexer/uninstall 生命周期；core/adapter/corpus/executor 签名更新与 revoked version。

第二轮指出的过度设计已收缩：Receipt 使用 SignerAdapter 复用 SSH/GPG/minisign/Sigstore；sync 复用 age/SOPS 和现有 transport，不自建设备 PKI或全局 equivocation 承诺；LLM Advisor 改为 AnalysisAdapter/脱敏 bundle，不维护 provider 平台。

## 第三轮终审

> Reviewer：全新零上下文 Codex `gpt-5.6-sol/max`  
> 事件：`turn.completed`  
> 用量：input 2,846,740（cached 2,630,528），output 33,441，reasoning output 25,750  
> Verdict：RED，无 P0，有 4 个 P1  
> 审查预算：这是第三次独立 review；修订后不再启动第四轮，不伪称独立 GREEN。

### P1 blockers

1. `R3-account-coordinate`：研究要求的 account/team policy snapshot 未进入 Receipt、ResolutionCoordinate、Diff、PolicyBinding 和 UI，可能把不同账号/组织 claim 合并。
2. `R3-capability-taxonomy`：custom commands/prompts、agent/subagent/child window、@/retrieval/file read、steering、tool search、MCP prompts/resources 等没有封闭的 capability matrix 和逐项 gate。
3. `R3-llm-claim`：`llm-suggestion` 仍被列为 ContextStateClaim provenance，可能进入真值 precedence、Receipt、baseline 或 policy。
4. `R3-projection-completeness`：`export-only` 可作为 ProjectionAuthority，且内部 QA 只抽验一条路径，可能让多数 harness/asset/scope 写入退化为空壳仍过完整门禁。

### P2 与缺口

- 为各 lifecycle stage 定义 value schema，说明六级是 facet 而非单调链。
- corpus 拆 development、sealed conformance、live oracle，防过拟合。
- 环境变量只声明 observed injection channel/user-attested/unknown，不假装能还原来源。
- WP-02 在 acceptance artifacts 独立评审前不得开始；traceability 覆盖所有 MUST，不只 F 编号。
- 研究台账命题计数修正并增加 commands/agents/subagents 命题。
- 被动扫描绝不执行 MCP/hook/script/plugin/dynamic command。
- 每个 importer 有 field-to-claim mapping，原生命令不自动等于 ModelVisible。
- 定义 semantic reconciliation/equivalent Receipt 算法。
- 增加 encrypted vault key lifecycle。

### 第三轮过度设计收缩建议

- package/source/install policy 由 APM authority 评估；Contexpect 只评估 Unknown/Receipt/privacy/egress 等 context-specific policy。
- Codex adapter 优先把 CtxWise 的稳定 JSON/fixture 作为 provider。
- sync 复用 age/SOPS、Git signed history 和现有文件同步，不自建版本图/通用 merge 协议。
- Effect Lab 使用成熟 eval/CI runner 和统计库，不发展通用实验平台。

第三轮 RED 保留。下方记录最后修订与确定性检查；按审查预算不再给独立 GREEN 结论。

## 第三轮最后修订

| Finding | Resolution | 证据位置 |
|---|---|---|
| R3-account-coordinate | 把 account profile、organization context、policy snapshot 加入 Receipt、ResolutionCoordinate、Diff、PolicyBinding、identity realm 与 UI 坐标；不可见身份保持 user-attested/unknown | 需求 §4.3、F-01、F-07、F-18、§9.1–9.2、§10.1–10.8 |
| R3-capability-taxonomy | 固定 16 类静态/动态 taxonomy；每个 harness/version/surface/category 必须 supported、unknown-honesty 或 N/A，并有 importer/field mapping/gate | 需求 §4.6、F-02、F-12、WP-01、WP-05、§17.0–17.1；来源 P26 |
| R3-llm-claim | 从 provenance 删除 LLM；Advisor 只能生成 non-authoritative suggestion/relation，不能进入 Claim、precedence、policy、CI、baseline 或 reconciliation | 需求 §4.4–4.5、F-13、§10.2 |
| R3-projection-completeness | 明列 core required-write matrix；引入逐 cell 唯一 authority，required-write 不得 export-only；逐 cell preview/apply/rollback/post-Receipt，不得抽一条代表路径 | 需求 F-09、WP-06、§17.0、§17.4 |

第三轮 P2/缺口也已收敛：六个 lifecycle 改为独立 facet 并增加各 stage value schema；Effect `inconclusive` 与 truth 分离；corpus 拆为 development/sealed/live；环境变量只保留 observed channel/user-attested/unknown；WP-02 增加 acceptance freeze 独立评审硬前置；traceability 扩至全文所有规范性语句；被动扫描禁止执行；importer 固定 field-to-claim；新增四态 semantic reconciliation、Receipt equivalence 与 vault key lifecycle。

第三轮指出的过度设计已继续删除：APM 成为 package/source/install policy 的唯一 authority；CtxWise 固定 surface 优先作为 Codex AdapterProvider；folder sync 不自建版本图/自动 merge，Git 场景直接复用 Git；Effect Lab 复用现有 runner 和成熟统计库。

## 最终确定性检查

2026-09-04 在当前 workspace 执行：

```text
F_COUNT=18
WP_COUNT=12
P_COUNT=26
MVP_COUNT=0
LLM_PROVENANCE_OLD_COUNT=0
MONOTONIC_ARROW_COUNT=0
OLD_EQUIV_ENUM_COUNT=0
REQUIRED_WRITE_ROWS=7
POLICY_AUTHORITY_COUNT=1
CAPABILITY_TAXONOMY_COUNT=1
RECONCILIATION_SECTION_COUNT=1
VAULT_LIFECYCLE_COUNT=1
TRACEABILITY_ALL_MUST_COUNT=1
SEALED_CORPUS_COUNT=2
```

附件复核：

```text
S1: 2806 lines, 38248 bytes, sha256 e90f4b4ca4d77c4244faeadee5dceb6a4c384a231c55393a9616d191253a6f6d
S2: 2240 lines, 31002 bytes, sha256 1698aed2e7a905f1a3d3fd78ea7a5d7e04d7745a7ee500dc8bd5e42f83bc2ee1
```

最终独立评审状态仍为 `RED`：以上是针对第三轮问题的修订和机械一致性证据，不是第四位/第四轮 reviewer 的语义 GREEN。依据既定三轮预算不再发起复审；因此此版本可作为 owner 决策和后续 acceptance-contract 编写输入，但不能声称“已独立验收通过”。
