# Contexpect · Codex 接续资料包

**目标：把本会话的竞争分析、工程调研、最终取舍、设计迭代和后续实现要求交给Codex继续对齐，而不是让它从头再研究一遍。**

本包包含18份Markdown（16份工作/索引文档 + 2份完整原始报告）。完整版另附11张设计图；纯Markdown包不包含PNG，视觉方向由06描述，素材身份仍在14登记。

## 1. 先记住这六件事

1. 继续建设Contexpect，不整体推倒，不把多个完整工具拼成新平台。
2. 差异化是跨坐标的证据核对和安全修复，不是配置数量、token图表或通用电脑体检。
3. 最终视觉是**白底黑字、现代、优雅、大胆但不压迫**；黑底白字和深色科技图已被后续偏好替代。
4. 核心Claim/Receipt、动作授权和变更协调保持统一；外部工具不能直接取得最终事实与写入权。
5. 本包是研究与建议，不是新ADR批准、实际集成结果或完整产品验收。研究基线是`257b7d4d82c0865a94b45d9b3fddcb3830658b32`，接手先查实际分支差异。
6. 当前可取附件未找到独立HTML源码；不把设计图冒充HTML。08提供接续交互合同，Codex先检查真实仓库中的既有演示。

## 2. 阅读路径

先将 [11_CODEX_START_PROMPT.md](11_CODEX_START_PROMPT.md) 交给Codex；首次对账依次看02、03、06、07、09、10。需要某环节依据时再读04/05和13；需要完整历史时读originals两份原文。不要一次把所有图和全部历史当作同等有效需求。

| 文件 | 用途 |
| --- | --- |
| [01 产品与竞争](01_PRODUCT_AND_COMPETITION.md) | 定位、竞争组合、亮点/不足/缺口及价值验证 |
| [02 最终决策与纠偏](02_FINAL_DECISIONS_AND_CORRECTIONS.md) | 不同轮次冲突处理、8项关键语义修正、图片内容纠偏 |
| [03 架构与真值](03_ARCHITECTURE_AND_TRUTH_CONTRACT.md) | 坐标、六面、Claim、Receipt、权限与账本所有权 |
| [04 完整20环节研究](04_MODULE_RESEARCH_AND_SELECTION.md) | 每环节1–2个优先参考、阅读入口、借鉴方式、风险与验收 |
| [05 整合边界与25冲突](05_INTEGRATION_BOUNDARIES_AND_CONFLICTS.md) | 最终组合、准入方式、冲突处理及后续收紧解释 |
| [06 UI/UX最终规范](06_UI_UX_AND_VISUAL_SPEC.md) | 白底黑字、布局/token建议、组件、响应式、隐私与无障碍 |
| [07 页面/状态/旅程](07_ROUTES_STATES_AND_JOURNEYS.md) | V01–V16、Advisor/历史、通用10状态与5条核心旅程 |
| [08 Demo交接合同](08_DEMO_CONTRACT_AND_FIXTURES.md) | HTML取件边界、交互、模拟隔离、18个建议场景 |
| [09 实施任务与范围](09_IMPLEMENTATION_TASKS_AND_COVERAGE.md) | D0–D5、T01–T12、T09子任务、F-01–F-18回查 |
| [10 测试/安全/发行](10_TESTING_SECURITY_AND_RELEASE.md) | Q01–Q16、证据分层、平台与隐私、停止条件 |
| [11 Codex启动提示词](11_CODEX_START_PROMPT.md) | 可直接使用的接续说明与首轮交付要求 |
| [12 会话决策记录](12_CONVERSATION_DECISION_LOG.md) | 从研究到视觉反馈的沿革、被替代方案与未核实项 |
| [13 来源台账](13_SOURCES_AND_EVIDENCE.md) | 原始55条研究来源、最终建议引用与动态复核项 |
| [14 资产清单](14_ASSET_MANIFEST.md) | 2份原文、11图、内容摘要、别名与源码边界 |
| [15 交付检查](15_DELIVERY_AUDIT.md) | 本包文件/链接/摘要检查，不等于产品验收 |
| [原始工程报告](originals/Contexpect_Engineering_Reference_and_Integration_Report_2026-09-11.md) | 完整字节归档，不改历史 |
| [原始最终建议](originals/Contexpect_Final_Recommendations_2026-09-11.md) | 完整字节归档，保留最终咨询取舍 |

## 3. 建议放进仓库的方式

可把本目录整体放到仓库一个独立研究/交接目录，例如`docs/handoff/contexpect-2026-09-11/`，保留originals和assets相对路径。该路径是建议，不表示本轮已经写入GitHub。不要用这些文件覆盖根AGENTS.md、PRD、ADR、生产schema或现有UI实现。

执行变更前以当前有效仓库合同为准。最终建议中需要修订的条款必须正式变更；用户确认视觉不自动批准新增依赖、网络、原生执行、付费实验或扩大数据读取范围。

## 4. 已有内容与新增交接细化

两份原始MD和11图是既有会话产物。根目录文档是有来源的重组；CSS token、Demo场景和T09子任务是本次根据已确认方向整理的实现建议，不假称之前已有对应代码。

没有重新搜索外部最新资料、重新构建产品或执行原生工具；因此当本包提及“当前上游”等历史措辞时，应按原报告核验日期理解，后续锁定具体版本再用。

## 5. 第一批动作

T01核对真实工作区与基线；T02修正原生/观察/策略、文本等价、动作级Unknown、历史/当前状态与并发保证；随后选T03/T04/T05/T06/T09的交集做一条真实纵向闭环。UI与测试并行，不再发散主题或横向增加相似平台。

**判断每个新功能的标准：是否让Contexpect更准确地说明差在哪里、证据是什么、哪些未知，以及如何安全修改并说明验证到哪一步。**
