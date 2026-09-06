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

## 与 Doctor 的关系

Doctor 的 PlacementRecommendation 由确定性规则产生。Advisor 只能补充候选，不能自动搬移资产，也不能解锁 Treatment。
