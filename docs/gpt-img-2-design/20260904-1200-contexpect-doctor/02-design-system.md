# Design System

## Visual Direction

Quiet clinical observability：像一台可信的诊断仪器，而不是普通 SaaS dashboard。大面积暖白和墨蓝负责稳定感，teal/amber/coral/violet 只承载语义。

## Color Tokens

| Token | Value | Usage |
| --- | --- | --- |
| `--bg-canvas` | `#F3F1EA` | 页面底色，暖灰白 |
| `--bg-surface` | `#FCFBF7` | 主面板 |
| `--bg-shell` | `#101722` | 左导航 |
| `--text-primary` | `#17202B` | 主要文字 |
| `--text-muted` | `#66717F` | 次要文字 |
| `--border` | `#D9D8D1` | 结构边界 |
| `--verified` | `#167C70` | 有原生/充分证据 |
| `--suspected` | `#C88825` | 待确认或可能干扰 |
| `--confirmed` | `#C64F45` | 确定问题/deny |
| `--unknown` | `#6F62B5` | surface 不可见或证据缺失 |
| `--info` | `#2878A8` | 中性信息/selected |

## Typography

| Role | Size | Weight | Line Height | Usage |
| --- | --- | --- | --- | --- |
| Page title | 28px | 650 | 36px | Doctor / Checkup 标题 |
| Section title | 16px | 650 | 24px | 模块标题 |
| Body | 14px | 450 | 21px | 主要内容 |
| Small | 12px | 500 | 18px | metadata、辅助说明 |
| Mono | 12px | 500 | 18px | version、path、digest、evidence id |

## Spacing And Layout

- 4px base；主 shell 1440×900；导航 224px；topbar 56px；main padding 24px；panel gap 16px；drawer 368px。
- 主内容避免“卡片网格海”；使用 1 个 summary band、1 个 findings list、1 个 evidence/care rail。

## Components

- `EvidencePill`：Native evidence / Static resolution / User-attested / Unknown；不得用 `Resolved` 单词单独表示静态解析成功。
- `SeverityMark`：Confirmed / Suspected，含 icon 和文字。Unknown 不是 severity；知识状态用独立 `UnknownMark`。
- `AgentChip`：agent 名称、version、coverage state；coverage state 只允许 Native evidence / Static resolution only / Needs connector / Unsupported version。
- `ContextFacetRail`：六条独立 facet，不使用单向箭头。
- `FindingRow`：症状、影响范围、evidence state、first seen、severity。
- `EvidenceChain`：Source → Resolver → Claim → Native observation；缺项显示断点。
- `CarePlanStep`：建议、loss、权限、authority、preview 状态。
- `CoordinateBar`：device、environment、account/policy、project、task、snapshot。

## States

- Loading：保留结构但所有值使用 skeleton，禁止默认绿色。
- Empty：说明没有 finding 不等于完整健康，仍显示 coverage/Unknown。
- Error：区分 permission、unsupported version、adapter failure、native surface absent。
- Dense：列表可切 40px row，右侧 rail 固定。
- Mutation disabled：必须说明缺 approval、authority 或 rollback 中哪一项。

## Evidence Vocabulary

四类证据词必须同时有文字、图标和颜色，不能只靠颜色表达：

| UI label | 精确定义 | 默认视觉 |
| --- | --- | --- |
| `Native evidence` | 当前 harness/version/surface 的原生诊断、事件或导出确实观察到声明字段 | teal + check |
| `Static resolution` | resolver 按文档/源码与本地文件推导出 expected state；不证明模型实际收到 | blue + document |
| `User-attested` | 用户对不可观察事实作了带时间和 coordinate 的确认 | teal outline + person/check |
| `Unknown` | 没有足够证据形成该 facet 的 claim | violet + question |

`Expected` 是 resolved context 的用户表述，**不是** `truth_state`。`truth_state` 只有 present / absent / indeterminate / not-applicable。在用户界面上，若来源是 resolver，应显示更具体的 `Static resolution`。`Observed` 只有在 provenance 达到 native/runtime 最低门槛时才显示为 `Native evidence`。

## Unknown And Indeterminate

- `Unknown` 描述某个 facet 的值没有证据，例如 `Visible: Unknown`。
- `Indeterminate` 描述整个 finding/policy decision 因关键 Unknown、冲突或证据不足而无法得出 pass/deny。
- 两者可以同时出现，但不可互换；Unknown 不是 severity，Indeterminate 也不是“低风险”。

## Adapter Coverage Taxonomy

- `Native evidence`：当前 coordinate 至少有一种声明范围清楚的原生运行时证据。
- `Static resolution only`：能解析资产和 precedence，但不可证明模型可见性。
- `Needs connector`：compact 分组标签，**不能**单独作为真值。展开后必须分别显示 installation / authentication / connector / version / surface 状态与 reason code；config residue ≠ installed。
- `Unsupported version`：已探测到版本但不在验证范围；不得自动归入 static resolution。未知版本 fail-closed。
