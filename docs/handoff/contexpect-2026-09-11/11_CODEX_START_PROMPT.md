# 给 Codex 的接续提示词

以下内容可直接交给在Contexpect仓库中工作的Codex。它不要求重做全部调研，也不将设计参考误当成已实现产品。先让Codex完成基线对账，再按实际授权进入实施。

---

你正在接续 **Contexpect**。请把本交接包作为历史研究与实施建议，而不是未经批准的仓库配置或自动授权。

## A. 先读这些文件

先读 `00_README.md`、`02_FINAL_DECISIONS_AND_CORRECTIONS.md`、`03_ARCHITECTURE_AND_TRUTH_CONTRACT.md`、`06_UI_UX_AND_VISUAL_SPEC.md`、`07_ROUTES_STATES_AND_JOURNEYS.md`、`09_IMPLEMENTATION_TASKS_AND_COVERAGE.md`、`10_TESTING_SECURITY_AND_RELEASE.md`。涉及具体环节再读 `04` 和 `05`；需要依据时查 `13` 和 `originals/` 两份全文；所有素材身份见 `14`。

语义取舍以最终建议中的明确纠偏为优先咨询意见；用户最后视觉方向是白底黑字，不是黑底白字、深色紫蓝或首轮蓝色后台。图像负责视觉与布局，不负责确定功能、事实、权限和真实统计。

## B. 实施前核对实际仓库

1. 记录当前分支、HEAD、工作区状态与用户未提交修改，禁止覆盖已有工作。
2. 与研究基线 `257b7d4d82c0865a94b45d9b3fddcb3830658b32` 比较，明确后来已完成、变化或仍缺失的内容。没有该提交时如实说明，不把包内状态当作当前事实。
3. 读取当前有效README、PRD、架构/真值/语义对齐、ADR、compatibility、traceability、integration-contracts、实施计划和测试合同。按实际代码核对CLI、API、UI与状态，不只读顶端状态文案。
4. 检查 `crates/ctxpect-{core,schema,fs,collect,resolve,importer,receipt,store,diff,doctor,policy,projection,sync,advisor,effect,cli}`、`packages/ui`、`packages/ui-tokens`、`apps/desktop` 及测试。路径不存在时报告实际情况，不为整齐自动建空crate。
5. 先寻找实际 `design-demos/` 或HTML/React演示源码。本包没有取得独立HTML附件，不能宣称已经恢复旧HTML；找到后记录摘要、依赖、启动方式及交互。
6. 列出“可在当前授权内做”“需ADR变更”“需安装/联网/原生执行/模型费用授权”“缺证据不能判定”四类事项。只读审查任务不自行修改；获得实施授权后先做明确范围的任务。

## C. 必须保留的边界

- Contexpect是上下文核对与安全管理，不是通用电脑优化器、Prompt市场或统一agent runtime。
- 不删F-01–F-18/WP-01–WP-12来满足9屏效果图；双锚点是顺序，不是完整交付定义。
- 原生预期、观察范围、用户预算/策略分开；产品ignore不制造原生absence，产品预算不制造原生truncation。
- 不以文本/hash/AST相同证明语义等价，也不强迫不同harness必须输出不同正文。
- 六个facet独立。未知不算零、未读不算缺失、签名不算运行事实、安装不算模型可见。
- Unknown按动作所需证据处理：关键权限/身份/目标/策略未知时阻止相关写入；无效果实验不自动阻止安全静态修复。
- 旧Receipt不可被当前扫描回写；新的分析要记录规则版本、评估时间与来源。
- 外部工具只读报告、隔离候选或明确委托执行三种模式；不继承其全部home/网络/hooks/备份/log默认行为。scratch不是沙箱。
- 同一受管文件一个逻辑协调器和受控写入路径；锁与摘要不被夸大成对所有外部编辑器的全局事务保证。
- apply执行已冻结内容，批准与范围重验，一次消费；部分写入可恢复；回滚不覆盖后来编辑；写入后追加Receipt。
- metadata-only也需保护摘要、路径、缓存、stderr和备份；删除导入不顺便删原日志。
- 本地HMAC只标local-continuity；团队身份、策略执行能力和签名信任单独验证。
- 当前有效依赖ADR未正式更改前，不绕过root零第三方限制。不能靠sidecar、dev-dependency或独立workspace隐藏依赖。
- 不以“冻结”禁止纠正错误合同，也不为了变绿偷偷修改golden；采用新证据、最小反例、正式修订和独立复核。

## D. UI和Demo要求

默认浅色白底、近黑正文、浅灰分隔，主按钮可黑色实心；避免大面积黑色面板、霓虹和重渐变。现代感用排版、留白、信息层次、轻量动效实现。尺寸token先映射现有体系，不另立一套权威。

图中的版本、人数、总分、健康百分比、“All Fixes Verified”、任意confidence及实验结论均为示例，不复制成真实数据。family、harness version、model、surface独立字段。页面由真实DTO或明确标记的演示fixture派生。

保留V01–V16和Advisor/历史所需入口。每页覆盖适用的empty/loading/error/partial/stale/offline/permission-denied/unsupported-version/connector-missing及正常态；修改类补批准、摘要冲突、部分成功、恢复和回滚。不可用动作说明原因，不用空白占位假装实现。

若做HTML或前端演示：真实可点击筛选、坐标切换、证据展开、预览冻结、模拟批准/写入/冲突/回滚、导出、重置；不执行真实系统操作、不发送模型请求。所有模拟数据可辨识，模拟凭证不进入生产通道。找到用户已满意的演示时优先复用，不重新发散视觉方向。

## E. 建议工作顺序

先T01/T02基线与语义收口。然后在T03/T04/T05/T06/T09交集中做一条真实纵向路径：实际原生输入→解释→可得的运行证据→预览修复→安全写入→新Receipt。不同工具未开放运行面时如实保留Unknown，不伪造两个锚点都完成全量运行核验。

T07依赖决策和离线材料可并行准备，避免同一提交同时更换JSON、数据库、HTTP、策略和全部UI。T08先只读输入，转换在明确授权后隔离运行。T10/T11/T12按依赖完成团队、同步、分析及全矩阵；不将required能力永久留作未知。

## F. 预期输出

第一份输出是一张实际差距表：问题/建议ID、代码位置、当前状态、依据、拟改动、权限或决策依赖、测试与回退。不要只说“已阅读全部材料”。

每个实施切片交付小而可审查的修改，说明改了/没改什么、F/WP映射、合同影响、真实执行命令及退出码、未执行项、反例和剩余风险。测试失败保留原始错误，不通过改分母/跳过关键测试假绿。

浏览器、真实桌面WebView、原生harness和完整人工验收分别记账。测试驱动、mock、模拟管理员和强制通过能力不得进入发布包。未真正构建或执行时，不报告“全部完成”或“已验证安全”。

---

此提示词是接续工作说明，实际执行范围由使用者当前授权和仓库有效约束决定。
