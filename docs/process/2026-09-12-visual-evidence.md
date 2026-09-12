# 2026-09-12 U06 白底黑字主题实机视觉取证

> 切片：U06（T09.U02 monochrome 主题迁移与 T09.U03 `/advisor` 路由之后，补实机截图取证；此前对比度只有 `packages/ui/tests/contrast.test.mjs` 的计算值）。
> 取证合同：[07_ROUTES_STATES_AND_JOURNEYS.md §6](../handoff/contexpect-2026-09-11/07_ROUTES_STATES_AND_JOURNEYS.md) —— 命名 `route__state__viewport__locale`，带构建 commit、fixture、浏览器版本与拍摄范围；只有实际拍摄的状态标已取证。

## 运行元数据

- 拍摄时间：2026-09-12T03:13:40Z（`pnpm test:e2e` 跑次，29/29 通过，含本文对应的 `packages/ui/tests/e2e/visual-evidence.spec.ts`）
- 构建 commit：`257b7d4`（工作区含未提交改动；dist 由当次 `pnpm build` 重建，bundle `index-DUbq_hL2.js` + `index-D_a2Vglq.css`）
- 浏览器：Chromium 141.0.7390.37（headless，Playwright；本机 `~/Library/Caches/ms-playwright` 既有安装，未联网）
- fixture：global-setup 临时目录种子——合成 project（AGENTS.md、登记的 skill-e2e 资产）、s-alpha/s-beta 两条 deepseek-harness-cli native 会话（`acceptance/corpus/development/native`，`live_tested: false`）、e-lab 已执行实验；Receipt 为本跑次真实 inspect 产物（pages.spec.ts 一次；visual-evidence.spec.ts 改写 AGENTS.md 后再次 inspect 取得第二份内容寻址 Receipt）；receipt.delete 例外由 spec 末个用例按 plantGrants 同形写入以走完「拒绝→授权→tombstone」全程
- viewport：desktop = 1440×900；narrow = 390×844；locale = zh（默认 zh-CN）/en
- 拍摄范围：全部为整页截图（fullPage）
- **截图本体路径：`packages/ui/tests/.e2e-out/evidence/`——该目录被 gitignore（`.gitignore:106`），属本地产物，不入库**；每次重跑 e2e 会覆盖重拍，且因 Receipt id、daemon 端口、时间戳变化，截图字节与 sha256 逐跑次漂移。下表 sha256 对应上述跑次。

## 已拍清单（27 张，全部实际拍摄）

| 文件 | sha256 | route | state | viewport | locale | 证明了什么 | 不证明什么 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `doctor__partial__desktop__zh.png` | `f8e76cd61a3e0c09f6be81fb2038fdfc60a64cdb8da41c45e877db39a0960ebd` | doctor | partial | desktop | zh | 白底黑字主题在 /doctor 实机渲染：统计带、findings 表、证据抽屉与 partial 状态横幅同时可读，中文界面 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `doctor__partial__desktop__en.png` | `25e93870b0700fa69a91a40b677b1728f4fdfa877ad4ea2328d749003a06171d` | doctor | partial | desktop | en | 同一状态在英文 locale 下渲染，i18n 切换无布局破坏 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `doctor__narrow-readonly__narrow__zh.png` | `dea2ce6bfb3980d9336f5b51bf6decb128071dad4b0c9d34b17abded22b91e1b` | doctor | narrow-readonly | narrow | zh | 窄视口（390px）按 C06 渲染只读窄屏面而不是残缺全壳，白底黑字主题生效 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；窄屏面是路由无关的，此图代表全部路由的窄屏行为 |
| `inspector__six-facets__desktop__zh.png` | `38ed0205b4de57e5c4969edf0f62b8b65b80b197b8820b228516327a3937d5e4` | inspector | six-facets | desktop | zh | /inspector 六面（installed/discoverable/eligible/model-visible/use-evidence/outcome-affecting）与 provenance/coverage/precision/knowledge_status 四轴实机渲染 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；facet 真值多为 unknown 是种子 fixture 的事实，不是主题缺陷 |
| `compare__diff__desktop__zh.png` | `054100c0dee738cb0482b07fc3f6ef4de8f6f02caa40818e74bd9c74ebf1b890` | compare | diff | desktop | zh | /compare 对两份真实 Receipt 跑出结构化 diff 并渲染 same_domain 等字段 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；「不可比较」（same_domain=false）需要不同 kind/坐标的两份 Receipt，本 fixture 只有同项目同类 inspect Receipt，未拍到 |
| `receipts__list__desktop__zh.png` | `64bd95ba1648322f056adc8ab8c7ccc3a548ea9a0e94556833f6e68f38624f08` | receipts | list | desktop | zh | /receipts 历史列表实机渲染本跑次 inspect 产生的 Receipt 行 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `care-plan__preview__desktop__zh.png` | `563b47b401cf60c43014613658b323593edb577e95bb2b1cf9367aa48582849e` | care-plan | preview | desktop | zh | /care-plan/:findingId 对真实 finding 渲染冻结预览（treatment_locked、preview_required: true） | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；摘要冲突与写入后待补证在本切片没有可达的写入路径（页面声明 carePlanLocked），未拍到 |
| `advisor__blocked__desktop__zh.png` | `2a269dc9846bae7a55c1d2b5662adf3591c3dadab0092a4202c07ffef5b64f67` | advisor | blocked | desktop | zh | /advisor 未授权态：发送按钮禁用、预览与阻止说明实机渲染（advisor.spec.ts 另证明此态零请求） | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `lab__list__desktop__zh.png` | `773f5192231e908fc0d0fe967387b1871af64e69685a8dae2467bb212338167d` | lab | list | desktop | zh | /lab 列表实机渲染 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `lab__executed__desktop__zh.png` | `8df3be062c94d9b69e784d2a245ea8e3df35364113fb4b8a85e807ba4b290e25` | lab | executed | desktop | zh | /lab/e-lab 已执行实验的判定（inconclusive / paired-exact-binomial-v2）实机渲染 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；「未执行」态的 POST 应答不落盘，/lab/:id 只服务已持久化（即已执行）实验，未拍到 |
| `sync__preview__desktop__zh.png` | `48975f623bf45fa4c3b62aa1ed981772db61645fcc8d29b45f5bc2c0da0e0a3d` | sync | preview | desktop | zh | /sync 对空目标文件夹的只读预览分列 transport 与 semantic，并标注 transport 未核验 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；分叉（conflict）需要 `<store>/sync/folder` 里已有另一 bundle，种子 fixture 无法经 API 制造，未拍到 |
| `sync__apply-refused__desktop__zh.png` | `60b7d7ca647c93c7374eb898f05bd6124cb66cac9702019897f4b50d64360efa` | sync | apply-refused | desktop | zh | 无 sync.apply 例外时应用被 daemon 拒绝，页面如实显示 policy.approval_required | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `settings__offline__desktop__zh.png` | `ae785e7e169c4051841052016603b681e8651569ef905718fe9a66422f2b43e6` | settings | offline | desktop | zh | /settings 在 transport 失败时渲染 offline 态横幅与重试入口 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为；失败由 page.route abort 注入——对页面而言与 daemon 宕机同为 transport 失败，但 daemon 实际未停 |
| `settings__ok__desktop__zh.png` | `9c660ae86c89acfec068fdd129c19d5060b646fcc115a9d7ff52e9ffb8d7a897` | settings | ok | desktop | zh | 重试后 /settings 表单从 schema 生成并渲染（离线恢复回路） | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `checkup__ok__desktop__zh.png` | `e0b7f6bd40d959e04debc44e5b24dc90473ef147eaec04c3a6704fd2cccdf5e2` | checkup | ok | desktop | zh | 白底黑字主题在 /checkup 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `assets__ok__desktop__zh.png` | `53dd05069b10aa45aab74e84adcc17be02548a68e3c63ddd7c99a9a577ad09ef` | assets | ok | desktop | zh | 白底黑字主题在 /assets 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `sessions__ok__desktop__zh.png` | `65a3c9f5fab9db8c9872b264878a69056fc0c751968a45f4ef4a709de9ab14d3` | sessions | ok | desktop | zh | 白底黑字主题在 /sessions 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `monitor__ok__desktop__zh.png` | `e9089601bb38ef3e64885a9ef1a13c95072c45352e6f59fd47316612fe85ce09` | monitor | ok | desktop | zh | 白底黑字主题在 /monitor 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `integrations__ok__desktop__zh.png` | `cca9a0831235a4b5a43aa95e821c728a0f72a095f2940cd4bba0a742f6f31fe7` | integrations | ok | desktop | zh | 白底黑字主题在 /integrations 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `policy__ok__desktop__zh.png` | `d5c57c2211852b902a8b2318c937c10dc3288f6d55ab3a3ca8af820ce323bafa` | policy | ok | desktop | zh | 白底黑字主题在 /policy 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `standards__ok__desktop__zh.png` | `d102004e7f5eed66bc4d5902e1c90d23ddcb2128a6deecec5929a8de086ab675` | standards | ok | desktop | zh | 白底黑字主题在 /standards 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `exceptions__ok__desktop__zh.png` | `4eea9ccde4c4b45a85c74275c177058935801d28d12a4bbd5e84b4a49c9b4a06` | exceptions | ok | desktop | zh | 白底黑字主题在 /exceptions 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `team-compliance__ok__desktop__zh.png` | `adcf070aeedcf7fd1f4d312650e86cba9fba3ca5d1065adcadd12aaff677d054` | team-compliance | ok | desktop | zh | 白底黑字主题在 /team/compliance 实机渲染，内容加载完成且无错误态 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `sessions__detail__desktop__zh.png` | `462b85adc39245164cae3eb18aa6837f78a3847e86574e028f4576041293c5e7` | sessions | detail | desktop | zh | /sessions/s-alpha 请求证据表（metadata-only，正文不进页面）实机渲染 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `receipts__delete-confirm__desktop__zh.png` | `eeb0d23958b40c2d602e2e8ae0fdad33415f9d3418dbc8c2e824de070031c885` | receipts | delete-confirm | desktop | zh | 删除前的后果对话（tombstone/派生失效/外部副本不可召回）实机渲染 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `receipts__delete-refused__desktop__zh.png` | `28180bdb30c7840b03557b2c6f117d1729ebef248652fa0f0f13eda90f807cc7` | receipts | delete-refused | desktop | zh | 无 receipt.delete 例外时删除被 daemon 拒绝，页面如实显示 policy.approval_required 且不产生 tombstone | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |
| `receipts__deleted__desktop__zh.png` | `f925837b46e40f8c63157cc3b5e4f44273b4897251997c8baa12133203217810` | receipts | deleted | desktop | zh | 有已批准例外时删除落 tombstone，页面显示删除回执并禁用后续动作 | 不证明 WKWebView/Tauri 桌面壳或其它 OS/浏览器的渲染；也不证明信息以外的交互行为 |

## 优先清单对账：未拍摄状态（如实列出，不伪造）

| 优先清单条目 | 状态 | 原因 |
| --- | --- | --- |
| Doctor 正常（ok） | 未拍 | 合成 fixture 恒有 Unknown facet，诊断 verdict 恒为 partial；ok 态在此种子下不可达 |
| Doctor 无权限 | 未拍 | /doctor 的 permission-denied 横幅需要 inspect/recheck 被策略拒绝的配置；种子里 inspect 不经 mutation 授权。抽屉内 collect 拒绝（抽屉内联错误，非横幅）可后续补拍 |
| Doctor partial | 已拍 | `doctor__partial__desktop__zh.png` / `__en.png` |
| Inspector 六面展开 | 已拍 | `inspector__six-facets__desktop__zh.png` |
| Compare 不可比较 | 未拍 | same_domain=false 需要不同 kind/坐标的两份 Receipt；本 fixture 只能产生同项目同类 inspect Receipt |
| Care Plan 预览 | 已拍 | `care-plan__preview__desktop__zh.png` |
| Care Plan 摘要冲突 / 写入后待补证 | 未拍 | 本切片没有可达的写入路径：页面声明 carePlanLocked，API 恒回 preview_required: true |
| Receipt 历史与删除 | 已拍 | 列表、删除确认对话、无授权拒绝、授权后 tombstone 四张 |
| Sync 分叉 | 未拍 | conflict 需要 `<store>/sync/folder` 已有相异 bundle；sync.apply 未授权且直接写 store 违背「只经 API 播种」约定 |
| Advisor 未授权发送 | 已拍 | `advisor__blocked__desktop__zh.png`（零请求由 advisor.spec.ts 断言） |
| Lab 未执行 | 未拍 | POST /api/v1/lab 的 not-executed 应答不落盘；/lab/:id 只服务已持久化（已执行）实验 |
| Settings 离线恢复 | 已拍 | `settings__offline__desktop__zh.png` + `settings__ok__desktop__zh.png`；transport 失败由 page.route abort 注入，daemon 未停（见表中"不证明"列） |

## 抽查结论（ReadMediaFile 实读 6 张）

首轮跑次抽查 `doctor__partial__desktop__zh`、`inspector__six-facets__desktop__zh`、`settings__offline__desktop__zh`、`doctor__narrow-readonly__narrow__zh`、`receipts__delete-refused__desktop__zh` 五张；最终跑次（即上表 sha256 对应的产物）复核 `doctor__partial__desktop__zh`，内容等价：

- 白底黑字主题生效：画布纯白、正文近黑、shell 浅灰，语义 tint（unknown 紫、suspected 琥珀、confirmed 红）与 tokens.css 一致；**无大面积深色面板残留**。
- 文字无溢出/重叠；长 reason 文本（policy.approval_required 说明）正常换行；窄屏只读面排版完整。
- 遮罩约定生效：Doctor 抽屉证据与 Inspector 预算块渲染为 `••••`（MaskedText）。

发现项（记录，不在本轮必须修）：

1. Receipt 明细页整页截图高达 10173px——删除拒绝/成功后页面直出整份 Receipt JSON dump。属既有设计（RawJsonDetails 合同），但对"截图可读性"而言信息量极低；是否给明细页截图限定可视区或折叠 raw dump，留待后续切片决策。
2. 上表 `inspector` 行的"证明了什么"在 spec 源码中修正为真实六面名称（installed/discoverable/eligible/model-visible/use-evidence/outcome-affecting）；本表为修正后文字，截图字节不受影响。

## WebView lane（apps/desktop，Tauri）：已尝试，未取证

`apps/desktop/src-tauri` 是独立 workspace、无 required gate。本次实跑了 lane 但**未取得有效 WebView 截图**，过程与原因如实记录：

- `cargo build`（独立 workspace，`CARGO_NET_OFFLINE=true`）4.92s 完成；二进制 `target/debug/contexpect-desktop` 以 `CONTEXPECT_DAEMON_URL=http://127.0.0.1:7421` 指向真实 daemon（临时 project/store，`/api/v1/inspect` 已产生 Receipt `r_6bf39d1309668bb2`）启动成功。
- 窗口真实创建（CGWindowList：1440×900 @ (560,139)，标题 Contexpect），unified log 显示 WebKit「Page Load」活动正常结束——**页面在 WKWebView（系统 WebKit 22625.1.29.11.27，macOS 27.0）里加载完成**这一点有日志证据。
- 但窗口不在当前 Space 可见（全屏截图对应区域只有桌面壁纸），WebContent 进程随即被悬挂（log：`freezeAllLayerTrees` / `prepareToSuspend` / App Nap），图层被标记 volatile；`screencapture -l<windowid>` 只能拍到窗框 + 黑色内容。对照实验：同法截 Calculator 内容完整，排除屏幕录制权限问题。`defaults write dev.contexpect.desktop NSAppSleepDisabled` 未能阻止悬挂（悬挂由窗口遮蔽/非活动 Space 触发，非 App Nap 计时），该 default 已删除。
- 结论：WebView 渲染**未取证**。可能的后续路径：在真实用户会话里以前台方式启动 .app bundle 使窗口落在活动 Space，或用 Safari Web Inspector 取 DOM 级证据。本轮没有把任何 Chromium 截图冒充 WebView 验收。

## 可复现

`cd packages/ui && pnpm test:e2e`——global-setup 自构建 UI、起真实 daemon 并播种，`visual-evidence.spec.ts` 重拍全部截图并输出 `tests/.e2e-out/evidence/manifest.json`（含逐张 sha256）。注意该 spec 按文件名字母序在 pages.spec.ts 之后运行，复用其冷启动之后的 daemon 状态；单独运行该文件会因缺少既有 Receipt 而失败（与 advisor.spec.ts 的空台账断言同为套件内顺序约定）。
