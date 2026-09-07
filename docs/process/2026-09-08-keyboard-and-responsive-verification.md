# 键盘与响应式取证（2026-09-08）

> 范围：C06 的键盘、焦点、非颜色状态、reduced motion、响应式断点，以及 C05 的截图隐私边界。
> **不覆盖** screen reader 实操、其余 OS lane、Tauri WebView 与 ≤2s 性能门槛——原因见末节。

## 取证环境

| 项 | 值 |
| --- | --- |
| 日期 | 2026-09-08 |
| 候选 | `a3cf1c0` |
| OS | macOS 27.0（Darwin arm64） |
| 运行形态 | `ctxpect daemon start --ui-root packages/ui/dist`，Chromium 浏览器预览 |
| **不是** | Tauri WebView。浏览器预览不能替代任何 OS lane（C06） |
| locale | zh-CN（表内另注 en 的项） |
| fixture | 合成项目：单个 `AGENTS.md`，store 内植入 pass policy + 一条存活例外 |
| 主视口 | 1440×900 |

## 取证强度说明

三种强度不可混同，本文逐项标注：

- **真实按键**：由浏览器自动化注入到页面的物理按键事件。
- **合规事件**：用 `KeyboardEvent` 派发、带正确 `key`/`code` 的事件。验证的是处理器逻辑，不是按键通路。
- **DOM/可访问性树读取**：读取渲染结果或无障碍树。

其中 **Enter / Space 只能以「合规事件」取证**：本次使用的浏览器自动化注入的 `keydown` 到达了目标元素，但 `key` 与 `code` 均为空字符串，页面无从判断按下的是哪个键。这是取证工具的限制，不是应用缺陷；Tab 不受影响，因为它由浏览器层处理而非页面读取。**该项仍需真实键盘复核。**

## 逐项结果

| # | 断言 | 强度 | 结果 |
| --- | --- | --- | --- |
| K1 | 页面无正 `tabindex`，不破坏自然 Tab 顺序 | DOM 读取 | 通过：22 个可聚焦元素，正 tabindex 计数为 0 |
| K2 | Tab 按 DOM 顺序移动焦点 | **真实按键** | 通过：医生 → 体检 → 检查器 |
| K3 | 焦点可见 | **真实按键** + 计算样式 | 通过：`outline-style: solid`，`outline-width: 2px` |
| K4 | 发现表格行可用 Enter 激活 | 合规事件 | 通过：选中由第 0 行移到第 2 行 |
| K5 | 发现表格行可用 Space 激活且不滚动页面 | 合规事件 | 通过：选中移到第 4 行，`scrollY` 保持 0 |
| K6 | 删除确认对话：焦点移入、Esc 关闭、焦点返回触发按钮 | 真实交互（`<dialog>` 原生行为） | 通过（详见 desktop-ui.md 的动作面一节） |
| R1 | ≥1280：drawer 与主区并列 | DOM 读取 | 通过：`.workspace` 双列 |
| R2 | 1024–1279：drawer 转 overlay，主区单列 | DOM 读取 | 通过：1200px 下 `position: fixed`，单列 |
| R3 | 768–1023：导航折叠为 64px 图标栏 | DOM 读取 | 通过：`grid-template-columns: 64px 1fr`，标签隐藏 |
| R4 | 折叠态下导航链接仍有可访问名称 | 可访问性树 | **初次失败 → 已修**，见下节 |
| R5 | <768：无横向溢出 | DOM 读取 | 通过：`scrollWidth == innerWidth` |
| R6 | <768：符合 C06「只读 Receipt/通知」 | DOM 读取 | **初次未达成 → 已实现**，见下节 |
| R7 | 断点变化时视图随之切换 | 合规事件 | 通过（代码路径），见下节的工具限制 |
| A1 | 状态不只靠颜色区分 | DOM 读取 | 通过：计数徽章带文字（"0 已确认"/"0 疑似"/"0 未知"），状态横幅带状态名与 reason code |
| A2 | reduced motion 无需特殊处理 | 计算样式 | 通过：全页动画/过渡元素计数为 0，无可减的动效 |
| A3 | 文本对比度 ≥ 4.5:1（SC 1.4.3） | 计算 | **初次 3 项失败 → 已修**，见下节 |
| A4 | 控件边框与焦点环 ≥ 3:1（SC 1.4.11） | 计算 | **初次失败 → 已修**，见下节 |
| A5 | 表单校验错误可在字段上感知（SC 3.3.1 / 4.1.2） | DOM 读取 | **初次失败 → 已修**，见下节 |
| A6 | 存在跳过导航的机制（SC 2.4.1） | **真实按键** + 点击 | **初次缺失 → 已实现**，见下节 |
| A7 | main / nav landmark 与表格名称（SC 1.3.1） | DOM 读取 | **初次缺失 → 已补**，见下节 |
| P1 | 201 字符长路径不撑破布局 | DOM 读取 | 通过：页面与 topbar 均无横向溢出 |
| I1 | en 模式下无中文残留 | DOM 读取 | 通过：16 个入口逐一切换后，唯一中文是语言选择器的「简体中文」（语言名应以该语言书写） |
| I2 | 长译文不撑破布局 | DOM 读取 | **初次失败 → 已修**，见下节 |
| P2 | 截图隐私下整份 DOM 无路径残留 | DOM 读取 | 通过：`outerHTML`、全部 `aria-label`、全部 `title` 中均无路径片段 |
| P3 | 截图隐私下输入框只读 | DOM 读取 | 通过：`readOnly = true`，值为遮罩字符 |

## 本次取证发现的问题

### R4 折叠导航失去可访问名称（已修）

768–1023 断点下 `.nav a span { display: none }` 把标签从**可访问性树**里一并移除，16 个导航链接的可访问名称全部为空。屏幕阅读器用户在折叠态下只会听到 "link"，无从判断去向。违反 WCAG 2.2 **SC 2.4.4 Link Purpose (In Context)** 与 **SC 4.1.2 Name, Role, Value**，均为 Level A。

修复采用两层：CSS 改为视觉隐藏（`position: absolute` + 1px + `clip`）而非 `display: none`；并在 `NavLink` 上加显式 `aria-label`。加 `aria-label` 是因为不同 AT 与工具对 visually-hidden 文本的处理并不一致——本次取证所用的可访问性树在只有 CSS 修复时**仍**报告名称为空。修复后 16 个链接全部恢复名称。

### R6 <768 缺少规格要求的窄屏视图（已实现）

初次取证时，C06 规定的 <768「只读 Receipt/通知」并未实现：代码只是把导航 `display: none`，其余仍渲染完整应用，结果是**窄屏下没有任何页面间导航手段**，只能靠直接改 URL。那不是「窄屏只读视图」，是「完整应用少了导航」。

现已实现 `NarrowReadOnly`：<768 时不渲染完整 shell，改为提供 Receipt 列表与明细的只读查看。明细沿用默认遮罩（实测 `data-revealed="false"`，显示遮罩字符），tombstone 条目单独标注以免被读作仍然存在的 Receipt。

**通知部分诚实声明未实现**：`notifications` 目录虽被 store 创建，但没有任何端点产生或读取它。界面因此显示 `notifications.unimplemented` 而不是「暂无通知」——后者与「功能不存在」是两个不同的断言。

### I2 长译文撑破 /assets 的布局（已修）

en 模式下 `/assets` 整页横向溢出 84px（`scrollWidth` 1524 vs 视口 1440），zh 模式下不溢出——英文文案更长，先暴露了这个问题。

根因是两层，只修一层无效：

1. `pre.mono` 是 `white-space: pre` 且 `overflow-x: visible`，一行长 JSON 不换行也不滚动，直接把父容器推宽。
2. 加了 `overflow-x: auto` 后仍然溢出：`.panel` 是 grid item，默认 `min-width: auto` 使它增长到贴合内容，于是 `max-width: 100%` 量的是那个已经被撑宽的容器，滚动条永远不出现。

修复同时给 `.mono` 加 `overflow-x: auto`，并让 `.panel` 取 `min-width: 0`、`.shell` 与 `.workspace` 的弹性列改为 `minmax(0, 1fr)`。修复后 `pre` 在自身内部滚动（`scrollWidth > clientWidth`），页面 `scrollWidth` 1425 < 1440。

复测覆盖 16 个入口 × zh/en 两种 locale，均无横向溢出。

### A3 / A4 四处对比度不达 WCAG 2.2 AA（已修）

对比度是可精确计算的硬指标，逐对算过之后有四项不达标：

| 组合 | 原比值 | 门槛 |
| --- | --- | --- |
| `--suspected` 徽章 / surface | 2.90 | 4.5（SC 1.4.3） |
| `--confirmed` 徽章 / surface | 4.40 | 4.5 |
| `--text-muted` / canvas | 4.39 | 4.5 |
| `--border` 用作控件边框 / surface | 1.38 | 3.0（SC 1.4.11） |

C06 要求保留暖白/深蓝与「证据、严重性分离」的设计语言，同时写明**图像仅约束构图**——即图稿不锁定具体色值。因此修复只压明度、**保持色相不变**：`--suspected` 36° → 36°，`--confirmed` 5° → 5°，`--text-muted` 214° → 215°。

控件边框另做区分：新增 `--border-strong`（#95937F，3.00:1）用于 input / select / textarea / 次要按钮——SC 1.4.11 管的是**需要被感知的控件边界**；`--border` 继续做面板边缘与表格行分隔这类装饰性描边，不受该条约束，也就不必加重视觉。

**严重性色两两之间的对比度只有 1.0–1.1，这不是问题**：WCAG 不要求语义色互相可辨，只要求各自对背景达标；而本产品不靠颜色区分状态——每个徽章都带文字（见 A1）。测试里为此单独断言了「严重性从不只靠颜色承载」，以免将来有人把文字去掉。

检查已固化为 `tests/contrast.test.mjs`：从 `tokens.css` 读色值、按 WCAG 公式计算、逐对断言。测试自身带一个校验（黑对白必须恰为 21:1）。有效性实测过：把 `--suspected` 改回原值后立即失败并报出 `2.90:1`。

### A5 表单错误只在汇总里，字段上无从感知（已修）

Settings 的校验错误原本只渲染成页面底部的一个列表，字段本身既没有 `aria-invalid` 也没有 `aria-describedby`。用 Tab 走到出错字段的人不会知道它有问题——一个在别处的列表不承担这个职责（WCAG 2.2 SC 3.3.1 Error Identification、SC 4.1.2 Name, Role, Value）。

现在每个可编辑控件在自身出错时带 `aria-invalid="true"` 与指向就近错误文本的 `aria-describedby`；底部改为只报**数量**，不再重复每条消息——消息已经在字段上，重复朗读是噪音。

实测：`retention_days` 填 9999 后该字段 `aria-invalid="true"`、`aria-describedby="settings-error-retention_days"` 且该 id 能解析到写着 `settings.out_of_range` 的元素，而其它字段的 `aria-invalid` 保持为空；嵌套字段 `resource_limits.daemon_rss_mb` 同样正确；把越界值改回合法后无效标记随之清除。

### A6 / A7 缺少绕过机制与 landmark（已补）

三处 Level A 缺失：

- **没有 skip link**（SC 2.4.1 Bypass Blocks）。16 个导航链接排在每个页面内容之前，只用键盘的人每换一页都要 Tab 过全部 16 个才能碰到内容。
- **没有 `main` landmark**（SC 1.3.1）。主区是 `<div className="main">`，辅助技术无法直接跳到内容，只能顺序走文档。
- **发现表格没有自己的名称**（SC 1.3.1）。旁边的标题并不与表格关联。

现在：`<a className="skip-link" href="#main-content">` 是页面的第一个 Tab 目标，平时移出屏幕、获得焦点时显示；主区改为 `<main id="main-content" tabIndex={-1}>`（`-1` 使其可作为焦点目标但不进 Tab 序列）；表格加 `<caption className="sr-only">`。窄视口的只读界面同样用 `<main>`。

实测（Tab 是真实按键）：页面加载后第一次 Tab 即落在 skip link 上，其 `left` 由 -9999 变为 8 即可见；激活后 `location.hash` 变为 `#main-content` 且 `document.activeElement` 就是 `MAIN#main-content`。激活用的是点击而非 Enter——Enter 仍受前述按键 `key` 为空的工具限制。

### R7 断点切换的取证受工具限制

视图随断点切换的**代码路径已验证**：在 1440 宽度下手动派发一次 `resize` 事件后，视图正确从 `NarrowReadOnly` 切回完整 shell。

但本次使用的浏览器自动化以 CDP 视口模拟改变宽度，它**既不派发 `matchMedia` 的 `change` 事件，也不派发 `window` 的 `resize` 事件**：改变宽度后 `window.innerWidth` 与 `matchMedia().matches` 都已更新，却没有任何事件通知页面。这与前述键盘事件是同一类工具限制。

因此该项只到「代码路径正确」为止，**真实窗口拖动仍需人工复核**。

顺带做了一处健壮性改进：窄屏检测同时监听 `change` 与 `resize`，并从查询对象读取结果而不是从事件读取。只依赖 `change` 的话，一个不派发该事件的视口变化会把用户困在错误布局里直到刷新。

## 明确未覆盖

| 项 | 原因 |
| --- | --- |
| screen reader 实操（VoiceOver / NVDA / Orca） | 需要真实辅助技术；可访问性树读取**不能**替代（C06 明确要求实际 AT 操作）。本文的 R4 只证明名称存在，未证明读序与朗读体验 |
| Enter / Space 的真实物理按键 | 取证工具注入的按键缺少 `key` 标识，见「取证强度说明」 |
| Linux / Windows lane | 本机只有 macOS |
| Tauri WebView | 桌面壳尚未创建；本次全部结果来自浏览器预览，按 C06 不能替代任何 OS lane |
| 首个有意义结果 ≤2s | 该阈值针对 Tauri WebView 下的实际运行，浏览器预览的数字不构成对它的取证 |
| C08 的逐页截图 | 未采集。本次优先做行为断言：按 C08，截图不能替代后端结果或完整 E2E |
| 真实窗口拖动触发的断点切换 | 取证工具的视口模拟不派发 `resize` / `change` 事件，见 R7 |

## 复现方式

```bash
cargo build -p ctxpect-cli
ctxpect daemon start --project <fixture> --store <store> --listen 127.0.0.1:7480 --ui-root packages/ui/dist
```

浏览器打开 `http://127.0.0.1:7480/doctor`，按上表逐项操作。响应式各项改视口宽度后**重新加载**再读取，避免读到缓存的样式表。
