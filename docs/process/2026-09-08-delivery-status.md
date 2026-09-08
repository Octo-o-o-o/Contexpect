# 交付状态（2026-09-08）

记录本仓库当前的真实完成度。写它的原因是：上一份接续交接里的坐标、清单状态与已知问题都已过时，接手的人需要准确的起点。

## 坐标

| 项 | 值 |
| --- | --- |
| HEAD | `a6a38f8`（已推送 `origin/main`） |
| 分支 / worktree | `main`，单一 worktree，工作区干净 |
| Rust workspace | 17 个本地 crate，**0 个第三方依赖** |
| 桌面壳 | `apps/desktop/src-tauri`，**独立 workspace**，不在根 `members` 里 |
| required gate | 13 条，逐条单独取退出码全为 0；干净副本复跑通过；`cargo test` 连跑 10 次全 0 |

## 上一轮 review §9 清单的逐项状态

| # | 事项 | 状态 |
| --- | --- | --- |
| 1 | 补授权、intent 持久化移到授权后、破坏性删除补审计 | 已完成 |
| 2 | 例外生命周期 bootstrap 死锁 | 已解开——身份源定为**仓内已登记 principals**，见 [ADR 0005](../adr/0005-exception-identity-and-mutation-boundary.md) |
| 3 | `standard preview/adopt/pin/update` 实现为各自不同的操作 | 已完成 |
| 4 | policy `required:true, effect:"allow"` 语义倒置 | 已修 |
| 5 | `EquivalenceProfile` 真正参与 `diff()` | 已完成 |
| 6 | `error_envelope` 硬编码 inspect 的 command/schema | 已修 |
| 7 | 截图隐私下 `projectFocused` 绕过遮罩 | 已修 |
| 8 | 四个常量端点换真实计算 | 已完成 |
| 9 | V12 Settings 编辑面与 V04/V05/V08 动作面 | 已完成 |
| 10 | C04 十类状态、取消/重试、drawer 契约 | 已完成，逐页契约由测试守住 |
| 11 | Tauri 2 桌面壳 | 壳已建成，**macOS lane 已验**；Windows/Linux 未验（缺设备） |
| 12 | C08 截图与 a11y/性能取证 | 部分完成，见 [取证记录](2026-09-08-keyboard-and-responsive-verification.md) |
| 13 | 18-family 真实探测 | **未做**——需真实安装外部 harness |
| 14 | E2EE sync | **未做**——见下节 |
| 15 | assets copy executor + SBOM | 已完成 |
| 16 | SQLite + FTS5 | **未做**——见下节 |
| 17 | WP-12 全矩阵 / 72h soak / 8 人 QA | **未做**——缺设备与人工验证 |

## 清单之外发现并修复的问题

这些不在原清单上，是实施与自测过程中查出的。三处是安全漏洞。

**安全**

- **DNS rebinding 绕过**：`Host`/`Origin` 用 `starts_with("127.0.0.1")`，`127.0.0.1.evil.com` 因此通过。攻击者注册该域名并指向 127.0.0.1，其页面即可同源读取本地 API 的全部响应。改为精确匹配主机名，缺失 `Host` 一并拒绝。
- **静态资源的符号链接逃逸**：路径检查是 `contains("..")` 字符串比对，UI root 内一个指向外部的符号链接就能读出任意文件。改用 `Root::contain`——产品读项目文件一直用它，是内部标准不一致。
- **审计链名不副实**：`audit_chain()` 只是读 jsonl，无前序哈希、无序号、无签名，删改重排不留痕，且无任何调用点。补 MAC 链接，接入合规视图。

**声明与实现脱节**（同一类，反复出现）

- `resource_limits` 两字段从不被读取；`copy_executor` 恒为 unimplemented；四个端点返回常量；CORS 头漏端口因而什么也没授予；`put_settings` 完全无验证（连隐私不变量都能改掉）。
- `/receipts/:id` 渲染了却没写进 `routes.ts`，而 C04 契约测试正是从那里取清单——**测试全绿，覆盖面缺一块**。
- L07 的「未覆盖」声明在 executor 实现后过时。

**可访问性**（WCAG 2.2）

折叠导航失去可访问名称（SC 2.4.4 / 4.1.2）、四处对比度不达 AA（SC 1.4.3 / 1.4.11）、缺 skip link 与 `main` landmark（SC 2.4.1 / 1.3.1）、表单错误只在汇总不在字段（SC 3.3.1）、`document.title` 不随路由变化（SC 2.4.2）、两页各有两个 `h1`、长译文撑破布局。

**核心不变量**

- R04：`GET /api/v1/standards/:id` 与 `standard status` 给出不同答案，且 API 无单条例外端点。改为共用同一函数。
- R05：四处 `rollback` 无 post-Receipt——撤销之后，最后一条观测描述的是撤销之前、已不成立的状态。
- 诊断响应不指明所依据的 Receipt，因而无法核验。

## 未做的项与真实原因

原交接把这些归因于「离线无法下载依赖」，**那是错的**：本机 cargo registry 缓存完整，`rusqlite` 与 `tauri` 都实测能离线构建。真实原因如下：

| 项 | 原因 |
| --- | --- |
| #16 SQLite + FTS5 | 技术可行，但会把第三方依赖引入根 workspace，破坏「零依赖 + 13 条门禁离线通过」。这是**项目决策**，需要明确取舍：ADR 0001 冻结 SQLite 为目标引擎，而当前 JSON store 实现了相同的 DTO/ledger 合同，该偏离已被记录 |
| #14 E2EE sync | 阻塞在**协议选型**而非密码学库：产品指定 age/SOPS，其 crate 不在缓存中；用 `ring` 自行实现一套协议不等于实现 age/SOPS |
| #13 18-family 真实探测 | 需要真实安装外部 harness（非 cargo crate） |
| #11 Windows / Linux lane | 缺设备。壳的代码与 macOS lane 已验 |
| #17 8 人 QA | 人工验证不能代答 |
| WebView 内的键盘取证 | macOS 的 WKWebView 不支持 CDP，缺自动化通道 |

## 取证强度

[取证记录](2026-09-08-keyboard-and-responsive-verification.md)逐项标注了强度，未把做不到的记成通过。两处工具限制值得接手的人知道：

- 浏览器自动化注入的 `keydown` 其 `key` 与 `code` 为空，页面无从判断按了什么。Tab 不受影响（浏览器层处理），但 Enter/Space 的激活只能用合规事件验证。
- CDP 改视口后 `innerWidth` 与 `matchMedia().matches` 都已更新，却不派发 `resize` 或 `change`。断点切换因此只验到代码路径。
