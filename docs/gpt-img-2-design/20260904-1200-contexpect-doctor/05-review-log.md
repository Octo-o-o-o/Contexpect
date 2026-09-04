# Review Log

## Round 1

| Category | Score | Notes |
| --- | --- | --- |
| Product fit | 8.2 | Doctor cockpit 成立，但缺 symptom-first entry |
| Information architecture | 8.4 | finding + detail rail 清楚；六 facet 尚未显式出现 |
| Visual hierarchy | 9.0 | 主次关系稳定，像开发者诊断工具 |
| Layout/composition | 9.0 | shell、主列表、右 rail 比例成熟 |
| Typography | 8.6 | 层级清楚，小字以 spec 为准 |
| Color/accessibility | 8.7 | 状态色克制；Unknown 与 severity 有混用风险 |
| Component consistency | 8.5 | 基础组件语言已形成 |
| State/error design | 7.4 | 缺 evidence facet 与覆盖缺口表达 |
| Responsive readiness | 7.5 | 只有 desktop reference |
| Implementation readiness | 8.8 | 可映射到常规 React/headless primitives |

平均：8.41。进入下一轮，因为 product/truth-state 缺口比视觉问题更重要。

### Round 1 Adjustments

- 加 symptom input 与常见症状 chips。
- Evidence 与 Impact 分栏，Unknown 不再充当 severity。
- 加六个 context facet 和明确的 Evidence Chain 断点。
- 修正“设置存在但 native visibility 不可见”的 Cursor 示例。

## Round 2

| Category | Score | Notes |
| --- | --- | --- |
| Product fit | 9.0 | 问诊入口和证据流程完整 |
| Information architecture | 9.0 | six facets、chain、coverage 已能支撑三类 persona |
| Visual hierarchy | 9.1 | primary diagnosis 与 detail rail 平衡 |
| Layout/composition | 9.0 | 信息增加后仍未退化成 card wall |
| Typography | 8.8 | 少量 coverage 小字仍需 spec 兜底 |
| Color/accessibility | 8.9 | 图标+文字+颜色基本成立 |
| Component consistency | 9.0 | chips、pills、rows、rail 统一 |
| State/error design | 8.3 | Indeterminate 已出现，但 Treatment gate 不够硬 |
| Responsive readiness | 7.8 | desktop 成熟，窄屏尚需明确策略 |
| Implementation readiness | 9.0 | 布局与状态可拆组件 |

平均：8.69。进入下一轮，主要修正 coordinate、权限与导航闭环。

### Round 2 Adjustments

- 补 account/policy coordinate 和 Assets 导航。
- `Collect evidence` 成为主 CTA；Treatment 在 Indeterminate 时锁定。
- coverage 从简单 agent chips 改为按证据能力分组。
- 修正 adapter count 与信息密度。

## Round 3

| Category | Score | Notes |
| --- | --- | --- |
| Product fit | 8.8 | 工作流完整，但词义仍可能误导 |
| Information architecture | 9.2 | Doctor → Care Plan → Receipt → Monitor 完整 |
| Visual hierarchy | 9.2 | evidence-first CTA 明确 |
| Layout/composition | 9.2 | 大屏利用率和阅读路径稳定 |
| Typography | 8.9 | legible；精确 copy 进入 spec |
| Color/accessibility | 8.7 | `Resolved only` 与 `Not exposed` 使用 pass-like 视觉，是实质问题 |
| Component consistency | 9.0 | 结构一致 |
| State/error design | 8.8 | Treatment gate 正确；evidence icon 语义待修 |
| Responsive readiness | 8.0 | spec 已补 breakpoint 行为 |
| Implementation readiness | 9.2 | 已可交给实现 |

平均：8.90，但 Kimi 的独立图片审查发现证据颜色/措辞 P1，因此不直接定稿。

## Validation Patches

### Evidence semantic patch

- `Resolved only` → `Static resolution only`；amber/green → blue document。
- `Native observation: Not exposed` 改成 neutral gray，禁止 pass-like check。
- 增加自然语言解释：`The rule exists, but Cursor cannot prove the model received it.`
- `DSH` 明确为 `DeepSeek DSH`。

### Adapter scope patch

- 本机又发现 Kiro CLI app bundle；互联网调研确认 OpenHands、Cline、Aider、Windsurf 应进入机制。
- 最终 coverage 改成 18 family：4 native-evidence、9 static-only、5 need connector。
- 图中全部 18 个名称逐一可见，没有用 `+N` 隐藏范围。

## Final Visual QA

最终图 `images/10-context-doctor-final.png` 已以原始分辨率逐区检查：

- 1586×992，保持 1440×900 目标比例；
- 18 个 adapter chips 无重叠、无截断；分组计数 4 + 9 + 5 = 18；
- `Static resolution`、`Indeterminate`、`Not exposed` 颜色/图标语义互不冒充；
- Evidence Chain、six facets、Treatment lock、Collect evidence 均可见；
- 导航、coordinate bar、findings 和 detail rail 没有碰撞。

最终主观 rubric：Product 9.2、IA 9.4、Hierarchy 9.3、Layout 9.2、Type 8.9、Color/A11y 9.1、Components 9.2、States 9.2、Responsive 8.2、Implementation 9.3；平均 9.10，无低于 7.5 的类别。

## Decisions

- 品牌用 `Contexpect`，`Context Doctor` 是产品内主工作流，不把已被多个项目使用的 Context Doctor 当独立品牌。
- 不用健康总分；总览必须保留 Confirmed/Suspected/Unknown 与 coverage composition。
- 默认排序为 severity/impact 优先、first-seen 次序；允许用户切换。
- `Expected` 留在数据模型；面向用户的 resolver 结果统一叫 `Static resolution`。
- Recheck 是 evidence action，完成后产生 Receipt；导航不再增加一个与 Receipts 竞争的一级入口。
- 18 个 adapter 是机制范围；不同 adapter 可以在不同 surface 上得到不同 evidence class。

## Rejected Ideas

- “Context Health 87/100”：精确但虚假的单一分数掩盖 Unknown。
- 所有 agent 都用绿色 Active：混淆 installed、discoverable、visible、used 和 effect。
- Doctor 列表直接放 Fix：越过 evidence、authority、preview、authorization 与 rollback。
- 只展示四个 logo：错误暗示产品边界，也无法服务真实多工具用户。
- 把 Coze 与 coding harness 强行做 prompt parity：它更适合作为 external connector/executor adapter。
- 用 stars 决定 evidence 等级：社区热度只影响调研顺序，不决定真实性。

## Agent Validation

完整记录见 `reviews/agent-simulation-report.md`。Claude 与 Copilot 对 scope-13 candidate 判 PASS；Kimi 判 PARTIAL 并发现已修复的证据语义 P1；Qwen 无法看图但发现文档词表和数量冲突；Grok、OpenCode、Gemini 分别暴露 sandbox、model config、account/client compatibility 问题。
