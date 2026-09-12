# 完整页面、状态矩阵与用户旅程

**状态：基于本会话公开路由文档与最终建议重组的交接合同。** 路由是否在接手时已实现、API是否相同，由T01核对。下表不声称已生成每个状态的图片或已经跑通全部按钮。[^R2][^CHAT]

## 1. 信息架构：9阶段不等于9个功能

最终白底图用Onboarding→Scanning→Overview→Mismatches→Details→Fix & Apply→Verification→History→Settings表达体验。实际产品按诊断、记录、操作、治理分组，保留原有16个入口族；不要为迁就概念图删除Assets、Standards、Exceptions、Integrations、Sync或Effect Lab。

可把Onboarding放在Checkup初次状态，把Scanning放在任务进度视图，把Details放在可链接详情页/侧区。导航可以简化，功能、深链接和返回路径不能丢。

## 2. 既有入口族的完整映射

| ID | 页面/历史文档路径 | 核心内容 | 主动作与成功落点 | 特别不能混淆 |
| --- | --- | --- | --- | --- |
| V01 | Checkup `/checkup` | 项目、cwd、harness、授权范围、上次检查 | 只读inspect→Receipt及Doctor | 选择目录不是授权读取整个home |
| V02 | Inspector `/inspector` | 六面、预算来源、来源链与覆盖 | 展开证据、请求补证→相应明细 | 静态预期不是运行观察 |
| V03 | Compare `/compare` | 两Receipt、坐标、EquivalenceProfile | 比较→分类差异/不可比较原因 | 两边未知不等于一致 |
| V04 | Receipts `/receipts`、`/receipts/:id` | 历史记录、验证、来源、导出、删除 | 查看/验证/脱敏导出/删除→结果与tombstone | 验签通过不等于内容真实或当前有效 |
| V05 | Assets `/assets`、`/assets/:id` | 上下文资产、包源、版本、许可/SBOM | 查看来源、导入报告；安装仅准入后 | installed不等于model-visible |
| V06 | Sessions `/sessions`、`/sessions/:id`；Monitor `/monitor` | 原生导入、请求证据、时序、增量变化 | 导入/查看/删除/定位Receipt | Monitor不替代请求证据页 |
| V07 | Effect Lab `/lab`、`/lab/:id` | 冻结合同、runner结果、判定与限制 | 预览实验/导入结果/授权运行→结果 | 没运行不产生正式判定；评分非因果证明 |
| V08 | Sync `/sync` | bundle、信任、父revision、分叉和接收者 | 导入/验证/协调/采用→正常投影流程 | transport success不等于semantic verified |
| V09 | Doctor `/doctor`（默认） | 真实finding、Unknown、suppression、下一动作 | 看原因/补证/建Care Plan | 规则命中不等于可自动修复 |
| V10 | Policy `/policy` | 实际生效策略、动作判断、执行能力 | 评估/解释→授权依据 | detect-only不等于强制阻止 |
| V11 | Standards `/standards`、`/standards/:id` | 标准revision、谱系、兼容、签名、采用状态 | preview/adopt/pin/update/rollback等按合同 | 发布、传输、采用与核验分开 |
| V12 | Settings `/settings` | 隐私、保留、adapter、资源、显示 | 预览改变/保存/撤销→已保存设置 | 配置预算不改变原生加载事实 |
| V13 | Exceptions `/exceptions` | 请求、批准/拒绝/撤销/过期、scope | 有可信身份通道才执行；否则显示要求 | UI自报管理员不是身份 |
| V14 | Team `/team/compliance` | 已披露标准、drift、loss、Unknown、freshness | 查看披露/重新核对→成员范围结果 | 默认不得读取私人prompt、secret或完整历史 |
| V15 | Care Plan `/care-plan/:findingId` | 冻结计划、文件、authority、loss、前置检查 | preview→批准→写入→重验→可回滚 | 没有native证据时仍显示待核验 |
| V16 | Integrations `/integrations`、`/integrations/:id` | family/version/surface/capability与独立安装/授权状态 | 看能力及最小补证动作 | 18个名字不是18个已运行适配器 |

以上路径来自本会话阅读过的 `docs/guides/desktop-ui.md`；接手时要查现有路由表和handler，而不盲建同名重复页。[^routes]

**Advisor / 历史洞察（F-13/F-14）：**必须有可到达的发送预览、建议、观察窗口和失效状态入口。具体路由以现有实现为准；若无，则先决定是独立 `/advisor` 还是Sessions内明确子入口。这里的 `/advisor` 是候选，不是已确认存在的接口。

## 3. 通用页面状态

| 状态 | 展示 | 可做操作 | 数据和权限规则 |
| --- | --- | --- | --- |
| 正常/当前 | 具体结果、scope、Receipt与观测时间 | 对应安全动作 | 不额外推断未覆盖字段 |
| empty | 尚未选择/无记录的具体原因 | 选择、导入或开始检查 | 无数据不是零问题 |
| loading | 正在执行的任务与范围 | 安全取消/离开 | 保留上一有效快照，不伪造进度证据 |
| error | reason code、失败步骤、影响范围 | 诊断/安全重试 | 外部失败不返回空列表冒充成功 |
| partial | 可用结果和未覆盖/缺段部分 | 查看结果、按最小范围补证 | 覆盖不全不能支撑不存在 |
| stale | 结果时间、陈旧原因 | 重新检查、看历史 | 不回写旧Receipt；缓存刷新不等于证据更新 |
| offline | 可用历史与本地功能 | 只读、允许的本地检查、稍后补证 | 不触发默认外联或清空快照 |
| permission-denied | 缺少的具体权限 | 说明/请求精确范围授权 | 不自动扫描更大目录作为fallback |
| unsupported-version | 已发现版本与支持边界 | 查看静态安全元数据、选已支持坐标或更新adapter | 不把旧规则偷偷套上 |
| connector-missing | 安装/授权/connector各自状态 | 显示接入说明或导入已有产物 | 不伪装成资源缺失 |

取消和重试属于操作状态，不宜全部混成新truth_state。某页面不适用某状态时有具体理由；不以统一组件为由为只读空页制造connector错误。

## 4. 修改类页面的额外状态

预览未批准、批准失效、批准已消费、目标摘要变化、关键策略未知、写入中、部分成功、恢复待处理、写入已完成但运行未验证、回滚冲突、回滚完成必须可区分。最终编码复用仓库状态机，不照此列表新建另一套状态权威。

前置证据按动作判断；缺少效果实验不能成为所有静态修改的永久阻塞，关键权限未知也不能因用户点击“确认”被省略。批量选择只能形成候选集合；每个文件和副作用仍需相应授权。[^R2]

## 5. 五条核心产品旅程

### A. 复杂目录诊断

Checkup选择项目/cwd→Inspector展示根到cwd链→Doctor显示确认问题和不确定项→查看具体规则来源。只能验证静态时明确写出；需要额外日志时提供最小补证方案，不默读home。

### B. 磁盘变更与旧会话

打开Sessions选择实例/request→比较该请求证据与当前磁盘快照→显示差异时间与加载语义→取得授权的新证据→产生新Receipt。不能用重扫按钮直接把旧会话显示为已更新。

### C. 有限意图的跨工具采用

选择受管意图和目标坐标→各目标renderer输出候选/loss→预览scope和权限→批准→统一写入→静态重验和按需原生核验。文本可相同也可不同，等价必须按目标机制判断。

### D. 并发与回滚

创建预览→模拟或发现用户后续编辑→执行前检查冲突→拒绝旧计划→重新预览。对已提交操作，回滚前再核对after-image；存在后续编辑时保留并请求协调，不能覆盖。

### E. 团队标准与跨设备

负责人发布有信任来源的revision→成员看变更与披露预览→收取bundle并验证→发现分叉先协调→按目标采用→分别看transport、installed、static、runtime状态。管理员仅见已披露元数据。

以上旅程承接最终建议，不替代全产品验收。[^R2]

## 6. 截图与交互取证清单

后续实现建议用 `route__state__viewport__locale` 标识截图，并带构建commit、fixture、浏览器/WebView版本和拍摄范围。只有实际拍摄的状态标已取证；示例图不作为执行证据。

优先取Doctor正常/partial/无权限，Inspector六面展开，Compare不可比较，Care Plan预览/摘要冲突/写入后待补证，Receipt历史与删除，Sync分叉，Advisor未授权发送，Lab未执行，Settings离线恢复。再覆盖其余路由的适用状态；不要声称一张拼图验证了全部情况。

## 7. 国际化与窄屏

主要文案用中文，技术ID/枚举保留英文并配解释；既有中英支持范围不得回退。路径与错误码不截断成不可理解内容。窄屏主要支持只读查看，涉及写入的操作显示明确限制或符合既有合同的入口，不静默隐藏必需功能。


## 本文依据

[^R2]: 本会话《最终建议与实施决策书》，[原文](originals/Contexpect_Final_Recommendations_2026-09-11.md)。这是最终咨询取舍，不是仓库已接受的新 ADR，也不是已完成的实现。

[^CHAT]: 本会话可见的用户要求及助手公开分析。过程与效力区分见 [会话决策记录](12_CONVERSATION_DECISION_LOG.md)；未取得独立原文的外部断言不转为工程事实。

[^routes]: 本会话曾读取 Contexpect [desktop-ui.md 固定基线](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/guides/desktop-ui.md)。路径作为历史文档依据，实际可用性需在接手分支复核。
