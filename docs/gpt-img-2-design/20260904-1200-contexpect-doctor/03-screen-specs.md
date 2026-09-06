# Screen Specs

## Screen Inventory

| Screen | Image | Route | States |
| --- | --- | --- | --- |
| Context Doctor | `images/10-context-doctor-final.png` | `/doctor` | populated/selected finding、empty、loading、permission error、adapter failure、dense |
| Context Checkup | 未单独绘图；复用 shell/facet primitives | `/checkup` | idle、scanning、complete、partial |
| Care Plan | 未单独绘图；由 Doctor drawer 展开 | `/care-plan/:findingId` | locked、preview、authorized、applied、rolled back |
| Recheck / Receipt | 未单独绘图；从 Doctor evidence CTA 进入 | `/receipts/:receiptId` | pass、deny、indeterminate、expired |
| Monitor | 未单独绘图 | `/monitor` | quiet、drift detected、notification suppressed |
| Assets / Integrations / Policy | 未单独绘图 | `/assets` 等 | inventory、conflict、connector required |

## Screen: Context Doctor

- Route: `/doctor`
- Final image: `images/10-context-doctor-final.png`
- Target viewport: 1440×900 desktop; generated reference is 1586×992 with the same proportion.
- Purpose: 让用户从“我感觉规则没生效”进入一个证据优先的诊断流程，并明确下一步是收集证据而不是直接修改配置。

### Layout

1. 224px dark shell navigation：Checkup、Doctor、Care Plan、Monitor、Receipts、Lab；Assets 分组下有 Assets、Integrations、Policy、Settings。
2. 56px coordinate bar：project、device、account/policy、18 agents、snapshot time、privacy mode。
3. Main column：title + symptom input、suggestion chips、multi-axis status band、findings table、adapter coverage。
4. 368–420px right rail：selected finding 的 Diagnosis & Evidence、evidence chain、six facets、next-best-evidence、primary/secondary actions。

### Components

- `SymptomInput`：placeholder `Describe what feels wrong…`；主按钮 `Diagnose`。
- `StatusBand`：Confirmed / Suspected / Unknown **分开计数**。fixture 文案 `3 Confirmed`、`2 Suspected`、`4 Unknown` 统计的是 **findings**（确认状态 + 知识缺口），与 findings 表行数不必相等（一条 finding 可同时有 Unknown facet）。`4 native-evidence · 9 static-only · 5 need connector` 统计的是 **18 个 adapter family 的 coverage 分组**，与 finding 计数对象不同、允许重叠解释；详情必须能从 Receipt 复算。4/9/5 只属于本视觉 fixture，不是 live 安装矩阵。
- `FindingTable`：Finding、Affected surfaces、Evidence、Impact、First seen；默认 severity/impact 优先，再按 first seen。
- `DiagnosisDrawer`：顶部显示 decision；本 fixture 为 `Indeterminate`。
- `EvidenceChain`：Declared setting → Resolver → Model-visible claim → Native observation；不得用连续绿色暗示整链通过。
- `ContextFacetGrid`：Installed、Discoverable、Eligible、Visible、Use evidence、Effect 六格独立状态。
- `NextBestEvidence`：最多三项，描述证据动作而不是修改动作。
- `AdapterCoverage`：三组 18 个 family，紧凑 wrapping chips。

### Fixture Data

Selected finding:

- Finding: `Cursor User Rules visibility is unknown`
- Coordinate: `Cursor · MacBook Pro · Personal · User scope`
- Plain-language diagnosis: `The rule exists, but Cursor cannot prove the model received it.`
- Declared setting: `Present / User-attested`
- Resolver: `Unsupported native surface`
- Model-visible claim: `Indeterminate`
- Native observation: `Not exposed`
- Treatment: locked until claim is verified

Adapter coverage:

- Native evidence: Codex、Claude、Grok、DeepSeek DSH。
- Static resolution only: Cursor、OpenCode、Kimi Code、ZCode、Gemini CLI、Qwen Code、Goose、Copilot、Kiro。
- Needs connector: Coze、Cline、Aider、OpenHands、Windsurf。

### Interaction States

- Finding selection changes only the right rail; list position and filters persist.
- `Collect evidence` routes to the best supported native action or a guided user-attestation flow.
- `Export bundle` exports a redacted diagnostic bundle; it does not unlock treatment.
- Treatment remains disabled when Model-visible is Indeterminate, even if Installed/Discoverable are Present.
- `Privacy mode` 拆成独立控件：显示遮罩、按住显示、复制确认、导出脱敏、egress consent。关闭遮罩不能自动授予发送正文权限。截图隐私模式还覆盖 DOM/tooltip/aria/toast/导出预览。
- Empty findings still show adapter coverage and any Unknown facets；copy 必须说 “No current findings in observed coverage”，不能说 “Everything is healthy”。
- Adapter failure appears as a finding with a reason code (`auth-rejected`、`unsupported-version`、`sandbox-unavailable`、`attachment-unavailable` etc.)。

### Responsive Behavior

- ≥1280px：two-column cockpit as reference image.
- 1024–1279px：navigation collapses to icons；right rail becomes 360px overlay drawer；main table remains full width.
- 768–1023px：status band wraps to two rows；findings switch to stacked rows；coverage groups become accordion.
- <768px：read-only receipt/notification experience only；full diagnosis prompts the user to continue on desktop.

### Accessibility

- Every evidence/severity state uses icon + text + color.
- Focus order: navigation → coordinate controls → symptom input → status/filter → finding rows → drawer → actions.
- Selected row uses border and background, not color alone.
- Minimum interactive target 36px desktop；keyboard opens/closes drawer and triggers evidence actions.
- Monospace is limited to paths、versions、digests and evidence IDs.

### Acceptance Checks

- User can identify the highest-impact finding、affected agent、evidence class and next evidence action within 10 seconds.
- Static resolution is never labeled Observed/Verified/Resolved.
- Unknown and Indeterminate are both visible and semantically distinct.
- All 18 adapter families fit without hiding product scope or implying equal native coverage.
- No direct Fix/Apply action appears before preview、authority、authorization and rollback are available.
- At 1440×900, no horizontal scroll and no collision between coverage chips and right rail.
