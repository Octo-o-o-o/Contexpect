# 数据与真值模型

> 状态：规范（尚未实施产品运行时）

本文是 Context IR、Claim、Receipt、身份和未知语义的 canonical 说明。机器可读不变量在 `acceptance/claim-validity-matrix.yaml` 与 `acceptance/context-capability-matrix.yaml`。

## 六个证据 Facet

同一 ContextItem 在给定解析坐标下有六个独立 facet。Installed 不蕴含 Discoverable，Observed UseEvidence 不蕴含 OutcomeAffecting。UI 不得画成单调状态机，也不得用一个绿色 Active 覆盖它们。

| Facet | 判定 | 合法 present 的最低证据 |
| --- | --- | --- |
| Installed | 是否存在 | 文件/配置/包/server 声明 |
| Discoverable | 当前 harness/version/surface 会不会发现 | resolver、官方规范、匹配版本源码 |
| Eligible | cwd/task/files/policy 是否满足激活条件 | glob、description、选择记录、task probe |
| Model-visible | 模型能否看到内容或目录元数据 | 语义明确的 native runtime/log |
| UseEvidence | 是否观察到调用、引用或行为一致 | 外部线索；内部归因默认 indeterminate |
| Outcome-affecting | 是否改变质量/时间/成本/失败类型 | 冻结 ExperimentContract 的 effect claim |

每个 facet 的 `value_json` 使用独立 schema，见 PRD §4.8。

## 三类对象

- `Resolved Context`：规范推演的 expected context
- `Observed Context`：原生或受控运行时导入
- `Context Effect`：对照实验产生的 outcome evidence

## 多轴 Claim

状态不是资产的固有属性，而是绑定 `Receipt + Snapshot + ResolutionCoordinate` 的 `ContextStateClaim`。

| 轴 | 取值 |
| --- | --- |
| claim_kind | resolved / observed / effect |
| lifecycle_stage | 六个 facet |
| truth_state | present / absent / indeterminate / not-applicable |
| provenance | native-runtime / native-log / harness-source / official-spec / heuristic / user-attested |
| coverage | full-declared-surface / partial-declared-surface / unknown |
| precision | exact / derived / estimated / not-applicable |
| knowledge_status | current / stale / conflicted / unknown |

LLM 不在 provenance 轴上。Advisor 输出只能是 `AdvisorSuggestion` 或 `CandidateRelation`。

UseEvidence 再拆为 `invocation-observed`、`reference-observed`、`behavior-consistent`、`internal-attribution`。任何外部线索都不能把 internal-attribution 改为 present。

`absent` 必须有足够覆盖；无法证明不存在时用 `indeterminate`。

## 合法组合

- resolved 可对 installed/discoverable/eligible 给 present/absent；不能仅凭规范把 model-visible / use-evidence / outcome-affecting 写成 present
- model-visible 的 present 需要 native runtime/log
- outcome-affecting 的 `truth_state=present` 只表示存在有效格式的 effect result；`decision` 才是结论。`inconclusive` 不等于 absent
- 未暴露的 timeline capability 只生成 indeterminate/unknown，不生成伪事件

冲突时先比较覆盖和新鲜度，再按 `native-runtime > native-log > harness-source > official-spec > heuristic > user-attested`。较高来源不能覆盖其未覆盖字段。同覆盖的高质量矛盾必须保持 `conflicted`。

## Capability taxonomy

16 类，无法分类时用 `other-unknown` 并阻止“完整覆盖”结论。每个 `harness × version × surface × category` 只能是 `required-supported`、`required-unknown-honesty` 或 `not-applicable`。

动态类别不得在只实现文件资产后被省略。

## Receipt

Receipt 是一次核对的不可变摘要。至少包含 PRD §4.3 列出的字段。默认不含完整 prompt、源码正文、tool result 或 secret。

六类：one-shot、preflight、runtime、post-session、device baseline、CI。

生成后 header/manifest 不可变。删除 evidence 时保留 tombstone 与原 digest。派生分析必须失效。已导出副本无法召回。

## 语义核对与等价

只允许 `verified` / `structural-only` / `indeterminate` / `failed`。structural-only 与 indeterminate 不得显示为已生效或跨设备完全一致。投影结果枚举 `exact | native-equivalent | transformed | lossless-native-overlay | lossy | unsupported | unknown` 与上述核对状态正交，不得把 `verified` 当作 projection outcome。text/hash 相等只证明文件相同，不得当作 CanonicalIntent 语义等价。详见 [semantic-alignment-and-team-standard](semantic-alignment-and-team-standard.md)。

算法固定顺序：schema/signature → coordinate/policy → desired-state/intent → projection transactions → resolver 重扫 → native oracle → 汇总 loss/Unknown。早步骤失败仍保留后续未执行项。

两个 Receipt 只有在同一 `EquivalenceProfile` 下可称等价。设备 id、绝对 home、mtime、本机 signer、本地 keyed digest 可按 profile 忽略；asset UUID、intent revision、scope、native target、loss、policy、required claim 不可忽略。

## 身份

- 受管资产：manifest 不可变 `asset_uuid`
- 未受管：域隔离 keyed digest(`source-realm + logical path + kind + native target`)
- 路径消失且 digest 相同只产生 `move-candidate`
- 双路径同时存在是 copy；共同祖先后独立演化是 fork
- 低熵/敏感内容没有对外 digest 时只能标 declared/user-confirmed/unknown

## Unknown reason codes

至少：`surface_not_exposed`、`unsupported_harness_version`、`permission_not_granted`、`runtime_snapshot_missing`、`cloud_setting_unavailable`、`dynamic_agent_selection`、`tool_schema_not_exported`、`current_occupancy_not_reported`、`content_redacted_by_policy`、`import_parse_failed`、`evidence_stale`，以及安装诚实码 `not_installed`、`connector_required`、`config_residue_only`、`authentication_unavailable`、`sandbox_unavailable`。

## SQLite 实体

核心实体树与建议表见 PRD §10。实现时迁移文件按 [implementation-plan](../process/implementation-plan.md) 分工作包落地。本阶段不创建数据库。

在 PRD §10 既有表之外规划（不执行 migration）：`intent_revisions`、`projection_outcomes`、`native_overlays`、`loss_reports`、`equivalence_bindings`、`team_context_standards`、`standard_revisions`、`standard_bindings`、`layer_assignments`、`exception_records`、`member_disclosures`、`leader_compliance_views`。`intents` 表保存 CanonicalIntent，不另建平行 Intent 对象。

## UI 词表

| UI label | 含义 |
| --- | --- |
| Native evidence | 原生诊断/事件/导出确实观察到声明字段 |
| Static resolution | resolver 按规范与本地文件推导；不证明模型收到 |
| User-attested | 用户对不可观察事实作带时间和坐标的确认 |
| Unknown | 该 facet 没有足够证据 |
| Indeterminate | finding/policy 因 Unknown、冲突或证据不足无法 pass/deny |

`Resolved` 不得单独作为 UI 标签。Unknown 不是 severity。
