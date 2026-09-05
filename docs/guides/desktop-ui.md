# 桌面 UI 指南

> 状态：规范（尚未实施产品运行时）
> 视觉参考：`docs/gpt-img-2-design/20260904-1200-contexpect-doctor/`
> 语义以 PRD §9、design Markdown spec 和本文为准；生成图的小字可能失真。

## Shell

- 224px 深蓝导航 + 56px coordinate bar + 主工作区
- 右侧 368–384px evidence/care drawer，可折叠
- 主视口 1440×900；≥1280 三栏；1024–1279 drawer overlay；<768 只读 Receipt/通知

导航（设计稿）映射到 PRD IA：

| 设计稿 | PRD | 路由 |
| --- | --- | --- |
| Checkup | Overview | `/checkup` |
| Doctor | Doctor | `/doctor` |
| Care Plan | Doctor/Apply 流程 | `/care-plan/:findingId` |
| Monitor | Sessions + drift | `/monitor` |
| Receipts | Receipts | `/receipts` |
| Lab | Effect Lab | `/lab` |
| Assets | Assets | `/assets` |
| Integrations | Standards/adapters | `/integrations` |
| Policy | Policy | `/policy` |
| Settings | Settings | `/settings` |

完整产品还需要 Inspector、Compare、Sync、Standards 路由。它们与 Doctor 共享 token 和 Receipt DTO。缺少它们不等于可以把范围裁掉。

Team Context Standard 与分层 policy 路由（扩展，不删除现有项）：

| 路由 | 作用 |
| --- | --- |
| `/standards` | Team Context Standard catalog / preview / adopt |
| `/standards/:id` | 成员 disclosure + per-harness projection |
| `/policy` | 分层有效 policy + detect-only 诚实性 |
| `/exceptions` | request/approve |
| `/team/compliance` | leader redacted compliance/drift |
| `/care-plan/:findingId` | 继续拥有 preview/apply/rollback |

## Doctor 合同

- Symptom input：`Describe what feels wrong…`，主按钮 `Diagnose`
- 状态带：Confirmed / Suspected / Unknown 分开计数，**没有健康总分**
- Findings 表：Finding、Affected surfaces、Evidence、Impact、First seen；默认 severity/impact
- Evidence chain：Declared setting → Resolver → Model-visible claim → Native observation；缺项显示断点，不用连续绿色
- 六 facet 独立格子
- Treatment 在 Indeterminate 时锁定；主 CTA 是 `Collect evidence`
- Empty copy：`No current findings in observed coverage`，不能说 Everything is healthy

## 证据词表

Native evidence / Static resolution / User-attested / Unknown 必须同时有文字、图标和颜色。禁止单独使用 `Resolved`。`Observed` 只有在 provenance 达到 native 门槛时才显示为 Native evidence。

Unknown ≠ Indeterminate。Unknown 不是 severity。

## Adapter coverage

UI 按当前 coordinate 的 Receipt 汇总 18 个 family。设计图 fixture 为 4 native / 9 static / 5 connector，并用于视觉回归。Live 数据必须来自 compatibility matrix 与探测结果，例如 DeepSeek 在本机 live lane 是 not-installed。

实现时用 DOM 断言 18 个名字，并确认分组计数求和为 18。不要 OCR 生成图。

## 无障碍与隐私

WCAG 2.2 AA。状态不能只靠颜色。键盘完成所有关键动作。Privacy mode 隐藏路径、用户名、repo、prompt 片段、server URL。大图提供表格替代。数字旁可查看口径；Unknown 不以 0 长条展示。

## 截图门禁（WP-04 之后）

路径与状态见设计 `06-implementation-plan.md` 与 [test-strategy](../process/test-strategy.md)。本阶段没有应用可截。

## 诚实性

本文不是已实现 UI 的操作手册。仓库没有 Tauri 窗口。
