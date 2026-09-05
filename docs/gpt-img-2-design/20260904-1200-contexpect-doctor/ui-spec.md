# Contexpect UI Specification

## Source Files

- Existing UI spec: 无；本文件是初始 canonical UI spec。
- Product requirements: `./docs/requirements/2026-09-04-contexpect-complete-product-requirements.md`
- Token/theme files: 尚无代码。
- Component files: 尚无代码。
- Screenshots: 三轮 image generation 产生。

## Style Policy

- 新产品建立明确风格：calm clinical observability × serious developer tooling。
- 不使用医院插画、听诊器、十字徽标、拟物病历夹或卡通医生。

## Adjustment Level

- Selected level: Level 4 architecture refactor。
- Rationale: 产品无既有 UI，需要同时建立导航与跨页面 workflow。

## Current Visual Language

- Color: warm off-white workspace + midnight navy shell；verified teal、suspected amber、confirmed coral、unknown violet、neutral slate。
- Typography: Inter/SF Pro UI；JetBrains Mono/SF Mono 只用于路径、hash、版本和 evidence id。
- Density: operational but breathable；列表行 44–52px，信息主要通过分组和 alignment 而非大量卡片。
- Radius: 8–12px；status pill 使用 999px，不做大面积圆角玩具感。
- Shadows: 只用于浮层/抽屉，主页面靠 1px border 和 surface tone 分层。
- Icon style: 16–18px 单色线性图标；状态同时用图标、文字和颜色。

## Layout System

- Shell: 224px 左导航 + 顶部 56px coordinate bar + 主工作区；右侧 344–384px evidence/care drawer 可折叠。
- Navigation: Checkup、Doctor、Care Plan、Monitor、Receipts、Lab；下方 Assets、Integrations、Policy、Settings。
- Grid: 12-column；Doctor 默认 central findings 7 columns + evidence/care 5 columns。
- Spacing: 4px base；8/12/16/24/32。
- Responsive behavior: ≥1280 三栏；1024–1279 右栏 drawer；<1024 列表/详情主从切换。

## Component Conventions

- Buttons: primary navy/teal，secondary outline，danger coral；Treatment 永远不是默认快捷按钮。
- Forms: 36px controls，label 常驻；复杂坐标使用可搜索 combobox。
- Tables: sticky header、44px row、状态文本 + icon；支持 dense mode。
- Cards: 仅用于 summary/vitals；诊断主体使用连续列表与 evidence rail。
- Dialogs/drawers: Evidence drawer 384px；Treatment preview 用宽 modal 展示 native diff、loss 和 rollback。
- Toasts/notifications: 只通知 actionable change；重复状态不再次弹出。
- Empty/loading/error states: skeleton 不伪造结果；Unknown、permission missing、unsupported version 分开表达。

## Accessibility And Platform Rules

- WCAG 2.2 AA；状态不能只靠颜色。
- body ≥13px，主操作 ≥14px；路径可截断但 hover/focus 可完整读取。
- 默认隐去用户名、绝对路径、prompt 片段、server URL 和 secret。
- 动画尊重 reduced motion；图表有表格替代。

## Inconsistencies To Resolve

- 不允许统一 Health Score。
- 不允许用绿色 Active 合并 Installed/Discoverable/Eligible/Visible/UseEvidence/Effect。
- 不允许把列表和右侧详情同时塞满同一层级的边框卡片。
