# LLM Advisor 与 Effect Lab 方法

> 状态：规范（尚未实施完整产品运行时）。`ctxpect advisor` 只产候选；`ctxpect experiment` 使用冻结 ExperimentContract。付费模型调用尚未授权。

确定性检查和 LLM 建议必须分开。前者可进 CI，后者不得作为唯一门禁，也不得写入 Claim 真值。

## LLM Advisor

- 用户主动触发
- 可预览将要发送的字段
- 通过 `AnalysisAdapter` 调用用户已有的本地 command、标准 endpoint 或外部工具；也可只导出脱敏 analysis bundle
- Contexpect 不维护完整 provider catalog、模型路由或独立 API-key 管理面
- 默认只发送选中的脱敏片段、finding 和结构 metadata，不发送整个仓库或会话
- 可做：语义冲突解释、过时线索、拆分建议、intent/skill 候选、修复草案、历史模式摘要
- 不可做：把推断写成 observed；无对照判定有用/无用；直接应用修改；绕过 policy；输出不解释来源的精确评分

每条建议引用输入 evidence。用户接受后仍走 preview/apply/receipt。

存储：`AdvisorSuggestion` / `CandidateRelation`，状态固定为 non-authoritative candidate。用户确认后只生成 user-authored Intent/decision，再由独立 resolver 核对。

Egress 与 registry/update/webhook 共用 allowlist 和 SSRF 防护。Secret 留在 OS keystore。不默认保存原始响应。

## 历史洞察

“从未使用”必须表述为“在已导入、可观察的 N 个 session 中未观察到”。删除历史后派生建议失效。分析默认本地。

## Effect Lab

Contexpect 不自建通用 agent runner、sandbox、统计估计器或门禁平台。它通过 versioned runner adapter 接入已有 harness/CI/eval runner，统计库使用冻结版本的成熟实现。

`ExperimentContract` 在第一轮运行前冻结，至少包括 PRD §F-15 列出的：唯一 primary outcome、最小实际效应或 equivalence margin、配对/随机顺序、样本量或 alpha≤0.05 且 power≥0.80 的计算、multiplicity、ITT 缺失处理、混杂锁定、失效条件。

Decision 只允许：

- `supported-beneficial`
- `supported-harmful`
- `supported-equivalent-within-margin`
- `inconclusive`（默认）

前三者必须同时满足预注册判据、方向与最小实际效应，并且没有使实验失效的协议偏离。“未显著”不等于“等价”。结论只适用于该 experiment coordinate。

混杂检测：代码漂移、模型变化、harness 更新、tool availability、缓存、网络失败、门禁变化。单次 before/after 不得宣传为因果证据。

### 当前实现的诚实边界

- `ctxpect experiment` / `POST /api/v1/lab` **不自造观测**：没有 runs 文档就是未执行（`executed: false`、`effect.runs_required`、无 decision），本地探针与预构造 fixture 已从产品路径移除，fixture 只存在于测试命名空间。
- runs 文档（`ctxpect-effect-runs-v1`，字段见 [cli-reference「Effect Lab 的 runs 文档」](cli-reference.md#effect-lab-的-runs-文档)）携带冻结合同（F-15 全字段）与逐 run 结果；样本不足、arm 不平衡、混杂锁定值漂移、run 早于冻结时间、重复 run 一律 `inconclusive` 并给出具体原因；非完成 outcome 按 ITT 规则计入并列在 `deviations[]`。
- 估计器是 `Estimator` trait。产品实现是仓内冻结的 `paired-exact-binomial-v2`（[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md)）：不一致对上的精确二项检验定方向（双侧 `p = 2·min(单侧) < alpha`，方向判定优先于等价判定），等价则对不一致率 `m/n` 与不一致对中的 treatment 份额 `b/m` **各取**精确 Clopper–Pearson 区间（各自单侧 `alpha`），配对差异 `(2q − 1)·r` 在两区间四个角上的范围严格落在 margin 内才算等价，无不一致对时用 `1 − alpha^(1/n)` 作不一致率上界（v1 把 `m/n` 当已知值，11 对里 1 个不一致对就判"等价"，2026-09-09 交叉 review 发现后换名修正）；样本数必须恰等于 `n_planned`（多收 `effect.n_mismatch`）、同一 `(task, arm)` 不得重复、时间必须可解析；其余 `inconclusive` + `effect.estimator_inconclusive`，并输出 `detail`（配对数、不一致数、p 值、区间）。`multiplicity != none` 在 v1 不实现，如实 inconclusive。没有引入统计库，`Cargo.lock` 仍无第三方条目。

## 与 Doctor 的关系

Doctor 的 PlacementRecommendation 由确定性规则产生。Advisor 只能补充候选，不能自动搬移资产，也不能解锁 Treatment。
