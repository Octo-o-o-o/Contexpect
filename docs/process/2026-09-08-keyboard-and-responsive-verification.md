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
| R6 | <768：符合 C06「只读 Receipt/通知」 | DOM 读取 | **未达成**，见下节 |
| A1 | 状态不只靠颜色区分 | DOM 读取 | 通过：计数徽章带文字（"0 已确认"/"0 疑似"/"0 未知"），状态横幅带状态名与 reason code |
| A2 | reduced motion 无需特殊处理 | 计算样式 | 通过：全页动画/过渡元素计数为 0，无可减的动效 |
| P1 | 201 字符长路径不撑破布局 | DOM 读取 | 通过：页面与 topbar 均无横向溢出 |
| P2 | 截图隐私下整份 DOM 无路径残留 | DOM 读取 | 通过：`outerHTML`、全部 `aria-label`、全部 `title` 中均无路径片段 |
| P3 | 截图隐私下输入框只读 | DOM 读取 | 通过：`readOnly = true`，值为遮罩字符 |

## 本次取证发现的问题

### R4 折叠导航失去可访问名称（已修）

768–1023 断点下 `.nav a span { display: none }` 把标签从**可访问性树**里一并移除，16 个导航链接的可访问名称全部为空。屏幕阅读器用户在折叠态下只会听到 "link"，无从判断去向。违反 WCAG 2.2 **SC 2.4.4 Link Purpose (In Context)** 与 **SC 4.1.2 Name, Role, Value**，均为 Level A。

修复采用两层：CSS 改为视觉隐藏（`position: absolute` + 1px + `clip`）而非 `display: none`；并在 `NavLink` 上加显式 `aria-label`。加 `aria-label` 是因为不同 AT 与工具对 visually-hidden 文本的处理并不一致——本次取证所用的可访问性树在只有 CSS 修复时**仍**报告名称为空。修复后 16 个链接全部恢复名称。

### R6 <768 未实现规格要求的窄屏视图（未做）

C06 规定 <768 为「只读 Receipt/通知」。当前实现只是把导航 `display: none`，其余仍渲染完整应用。后果：**窄屏下没有任何页面间导航手段**，只能靠直接改 URL。

这不是「窄屏只读视图」，是「完整应用少了导航」。窄屏专用视图尚未实现，此处记为未做，不计入通过。

## 明确未覆盖

| 项 | 原因 |
| --- | --- |
| screen reader 实操（VoiceOver / NVDA / Orca） | 需要真实辅助技术；可访问性树读取**不能**替代（C06 明确要求实际 AT 操作）。本文的 R4 只证明名称存在，未证明读序与朗读体验 |
| Enter / Space 的真实物理按键 | 取证工具注入的按键缺少 `key` 标识，见「取证强度说明」 |
| Linux / Windows lane | 本机只有 macOS |
| Tauri WebView | 桌面壳尚未创建；本次全部结果来自浏览器预览，按 C06 不能替代任何 OS lane |
| 首个有意义结果 ≤2s | 该阈值针对 Tauri WebView 下的实际运行，浏览器预览的数字不构成对它的取证 |
| C08 的逐页截图 | 未采集。本次优先做行为断言：按 C08，截图不能替代后端结果或完整 E2E |

## 复现方式

```bash
cargo build -p ctxpect-cli
ctxpect daemon start --project <fixture> --store <store> --listen 127.0.0.1:7480 --ui-root packages/ui/dist
```

浏览器打开 `http://127.0.0.1:7480/doctor`，按上表逐项操作。响应式各项改视口宽度后**重新加载**再读取，避免读到缓存的样式表。
