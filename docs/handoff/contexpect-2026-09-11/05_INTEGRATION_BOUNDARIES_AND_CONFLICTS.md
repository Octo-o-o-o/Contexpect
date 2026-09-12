# 整合边界、最终组合与 25 项冲突

**目标：复用各项目长处，而不继承彼此矛盾的默认行为。** 详细项目能力见 [20环节研究](04_MODULE_RESEARCH_AND_SELECTION.md)；本文件固定采用边界，不构成工具执行授权。

## 1. 默认组合

| 层 | 保留或优先采用 | 条件与退出方式 |
| --- | --- | --- |
| 核对与坐标 | 自有 core、Claim、Receipt、EquivalenceProfile | 不交给外部 active/health 状态体系 |
| 原生解析与导入 | 自有版本化 adapter；原生源码、Scopeon/ContextSpy局部参考 | 精确字段映射；外部失效时历史可读 |
| Doctor | 自有规则/证据；agnix、固定离线检测器的受限产物 | 不直接 `--fix`，不以仓内自授权 allowlist 放宽保护 |
| 资产来源 | APM lock/audit/SBOM；Syft用于软件物料 | 先读，不自动安装 |
| 原生转换 | Rulesync scratch renderer 候选 | 输入输出冻结；读写/网络未验不得执行 |
| 修改事务 | 自有 mutation 服务 | 单次批准、受管文件、恢复与 post-Receipt |
| 文件/schema/存储 | cap-std / jsonschema / SQLite+rusqlite 等少量候选 | 正式 ADR、vendor、feature、迁移逐项通过 |
| 同步与信任 | age格式/rage候选 + folder/Git + 独立签名合同 | 不把加密当签名，不同步活动库 |
| 策略 | 先保留 ctxpect-policy；复杂度需要才考虑 Cedar | 不同时开 Cedar 与 OPA 共同批准 |
| 应用 | Tauri+React；按需要 Axum/notify/TanStack Query | 不新增fs/sql/shell旁路；共享服务优先于换框架 |
| 分析 | AnalysisAdapter；按需 Rig provider；Inspect runner候选 | 发送、日志、费用单独授权；建议/运行不是最终判定 |
| 测试发行 | 既有Playwright；条件proptest、原生WebView验证、dist/cargo-deny | 测试服务不出货；开发工具不常驻产品 |

agentsync、Agentpack、chezmoi、Syncthing、Scopeon、ContextSpy 不默认作为完整后台运行；OPA、promptfoo、OTel Collector 不成为第二套必要平台。**不引入其运行时，不意味着删除对应产品能力。**[^R2]

## 2. 最小准入合同

外部产物携带 producer/version、输入与来源摘要、坐标、观测时间、字段来源、coverage、缺口、错误和隐私级别。proposal 再增加目标文件、owner、before/after摘要和 loss；mutation 再绑定可信principal、动作scope、策略/例外、期限和操作ID。

运行外部工具之前固定读取/写入范围、环境变量、进程与网络能力、日志去向、输出schema和失败回收方式。scratch 只是工作目录，不是沙箱；不能将默认home、包下载、hooks或debug日志一起继承。[^R2]

## 3. 原冲突表的解释修订

下面 C01–C25 保留研究追踪ID，表示“接入时需处理的合同冲突”，不是对上游发布新的漏洞通告。

C03 应按动作所需证据处理 Unknown，不是任何未知就禁止一切操作。C05 的文件级 CAS/锁表示受控参与者间的冲突保护，不保证任意外部进程的原子隔离。C09 不允许隐式改签名字节，但明确版本化迁移是允许的。C14 在应用外层处理关键错误，不假称上游策略算法已被修改。C23/C24 的 Tauri 方案是前次研究的候选；实际平台与目标版本仍须复核。[^R2]

## 4. 原始整合冲突清单
## 五、整合冲突清单与处理方式

下表中的 P0/P1 是**不处理就进行接入时的风险优先级**，不是对上游项目发布漏洞结论。许多默认行为对原产品合理，只是与 Contexpect 的合同冲突。

| 编号 | 冲突对象 | 风险与后果 | 统一决策 | 必须验证的反例 |
| --- | --- | --- | --- | --- |
| C01 / P0 | 库级选型 × ADR 0006 | 推荐库被直接加进根 workspace，破坏既有依赖/离线门禁 | 先 D/T；L 必须独立 ADR、vendor、回归与许可门禁 | 在未批准前出现任何新 registry 依赖，门禁转红 |
| C02 / P0 | 上游 main × 已冻结 harness 坐标 | 用新行为解释旧版本，产生可信但错误的 Expected | 规则绑定原生版本与源码 ref；cutoff 后材料只作候选 | 同一个输入在旧/新版本不同规则下产生不同预期 |
| C03 / P0 | ignore/预算工具 × native resolver | “Contexpect 没看”被误当成“原生不会加载” | 区分观察范围、原生预期与用户策略 | 修改产品 ignore/预算不能无证据改变 native claim |
| C04 / P1 | CanonicalIntent × `.rulesync` × `~/.agentsync` | 三个源轮流覆盖，用户不知道改哪里 | 每类资产登记一个 authority；其他源只作导入或 renderer input | 用户修改外部源时不自动夺取原 authority |
| C05 / P0 | APM/Rulesync/agentsync/Agentpack × 内置 executor | 同文件双写；共享 JSON 的不同 key 也会丢更新 | 一个文件级协调器；scratch renderer 与 delegated writer 二选一 | 两个修改不同 key 的并发请求不能丢失彼此变更 |
| C06 / P0 | dotfiles 模板/包 hooks × 被动 inspect | 扫描变成执行，扩大网络、进程和文件权限 | passive collector 永不执行；测试/执行器单独授权 | 仓库携带 hook、模板 exec、恶意本地配置时不执行 |
| C07 / P0 | agentsync 式本地 Git 备份 × secret gate | 已解析 secret 进入历史、临时文件或崩溃输出 | 默认不接管此备份策略；无安全正文合同则拒绝 secret-bearing 修改 | 故意失败时 backup/stdout/stderr/temp/Git 都不含 canary |
| C08 / P0 | schema 库外部 `$ref` × offline/private-by-default | 验证恶意对象触发联网或读取本机文件 | 禁用默认外部解析；固定内置 registry | `$ref` 指向网络、环回或 home 路径均不被访问 |
| C09 / P0 | JSON 替换/SQL 迁移 × Receipt 签名 | 数值词素、排序或归一化变化破坏旧摘要 | 原签名字节单独保存；新算法新版本；不隐式重签旧 Receipt | 数字不同词素、Unicode、重复 key 和旧签名回归 |
| C10 / P0 | Cosign/in-toto × 原生证据判断 | 验签成功被误当作模型真的收到上下文 | 身份真实性、字节完整性、来源声明、观察覆盖分开 | 用户签一份虚构日志，仍不能升级为可信 native observation |
| C11 / P0 | age/rage × 标准来源与撤销 | 加密被误当成签名；移除成员被说成召回旧数据 | 加密/签名/防回退/新鲜度独立验证 | 被移除者仍持有旧密钥时不得承诺旧副本不可读 |
| C12 / P0 | 文件同步器 × SQLite/WAL | 活动账本在跨机复制中形成不一致状态 | 只同步不可变 bundle，必要时通过一致性备份 API 导出 | 同步目录中不能出现 hot DB、WAL、私钥或 token 文件 |
| C13 / P1 | Syncthing 冲突赢家 × 语义分叉 | mtime 或设备顺序变成团队标准决定 | 传输层无权选择标准；父 revision 分叉必须显式协调 | A/B 离线修改同一标准，不自动 LWW |
| C14 / P0 | Cedar/OPA × Contexpect Unknown | 关键策略求值失败/undefined 被误当 Allow | 有类型输入与完整性前置门；检查 diagnostics；缺证据阻止 mutation | permit 为真但关键 forbid 求值错误时拒绝写入 |
| C15 / P1 | 两套策略引擎/包策略 × 团队授权 | 谁能批准和何时生效产生不同答案 | APM 只管包域；核心授权只选一种实现 | 外部安装 allow 不能绕过本地 context prohibited |
| C16 / P0 | OTel/性能采样 × evidence ledger | 丢弃事件后仍声称 full coverage | 性能 trace 可采样，必需证据不可采样；丢失要降低 coverage | 高压丢 trace 不改 Claim；证据落盘失败不能报完整成功 |
| C17 / P0 | Tauri capabilities × localhost HTTP | 以为 IPC 限权自动保护本地 API | 两个通道都显式授权；不新增旁路 SQL/fs 命令 | 非 Tauri 本地进程/恶意网页不能读写授权外项目 |
| C18 / P1 | TanStack Query × 坐标/证据新鲜度 | 不同项目复用缓存；旧返回覆盖当前状态 | query identity 包含实际输入；核心 freshness 独立于缓存 | 快速切换项目后旧 receipt 的诊断绝不覆盖新页面 |
| C19 / P0 | Inspect/promptfoo × 私人数据默认不落地 | 原始输入、错误请求、转录保存到 runner 日志 | 专用受限工作区；明确留存与清理；仅受控转换产物进账本 | API 错误、异常退出、重试后扫描所有旁路产物 |
| C20 / P1 | 外部评测结果 × 冻结统计合同 | 重试、缓存或筛掉失败改变样本与因果解释 | runner 只报告事实；Contexpect 校验后判定 | 一次失败重跑成成功，原失败仍可追踪且按合同处理 |
| C21 / P1 | Rig 全 agent runtime × Advisor | 为建议器引入自主工具执行和第二套记忆 | 优先 provider-only；不给 mutation 工具能力 | 提示注入要求写文件/读凭据时无法获得执行通道 |
| C22 / P1 | Rust/依赖 latest × 已固定工具链 | main 示例、MSRV、feature 和构建环境不兼容 | 锁定 released 版本和完整构建矩阵；升级显式提交 | 在固定 Rust/OS 下干净离线重建失败即停止升级 |
| C23 / P1 | Playwright Chromium × 全 OS 桌面验收 | 浏览器通过被包装成 Tauri 全平台通过 | 浏览器、WebView、CLI OS lane 分开报告；验证新的内嵌驱动路线 | 未执行的 macOS WebView case 必须显示未验证 |
| C24 / P0 | 内嵌 WebDriver / IPC mock × 发布构建 | 为测试增加的通道变成产品控制旁路 | 测试专用构建、最小插件；发布时移除测试服务和权限 | 检查最终安装包，测试端口、驱动和 mock 能力均不存在 |
| C25 / P0 | 检测器更换 / 仓内规则 × secret gate | 新工具自动联网验证凭据，或敌意仓库自行关闭扫描规则 | 固定可信规则与扫描配置；检测器可替换但必须重验 | 仓内 ignore/allow 不能静默豁免；替换后 canary 不产生网络请求 |

关键默认行为的一手依据：ADR 0006、jsonschema features、agentsync known limits、Cedar authorization、SQLite WAL、Syncthing conflicts、Inspect logs、Tauri capabilities/WebDriver 和检测器的配置说明。[^5][^14][^31][^38][^23][^36][^46][^42][^53][^27][^56]

**不存在一种仅靠统一接口名就能消除这些冲突的整合办法。** 必须减少真正参与写入、持久化、认证和判定的主体；适配器在外部边界做语义降级，而不能把外部“成功”直接搬进核心。

## 5. 最终引入判断

每次引入只问：解决哪个已验证缺口、增量维护成本是什么、它无权做什么、怎样证明不回退、怎样退出。未满足准入时可保留设计参考或只读导入；不能永远把 required 能力留作 Unknown，也不能为了赶进度先执行后补隐私。

原生资料、规则反例、UI状态和输入合同可以先改善，不需要以一次大规模换库作为前提。根workspace零依赖政策仍有效；建议的受控依赖目标必须经正式变更，不通过sidecar或dev-dependency绕过。


## 本文依据

[^R2]: 本会话《最终建议与实施决策书》，[原文](originals/Contexpect_Final_Recommendations_2026-09-11.md)。这是最终咨询取舍，不是仓库已接受的新 ADR，也不是已完成的实现。

[^5]: [Contexpect，ADR 0006](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/adr/0006-third-party-dependency-policy-and-estimator.md)。零第三方依赖、存储偏离退出条件、v2 估计器和 Playwright 决策。

[^14]: [jsonschema，Rust 实现](https://github.com/Stranger6667/jsonschema)。schema 验证、feature 表和外部引用解析；接入必须关闭不受控解析。

[^31]: [agentsync，README 与 Known Limits](https://github.com/spxrogers/agentsync/blob/main/README.md)。投影与对账思路；重点检查 JSONC、owned keys、明文备份、Windows 权限及 rollback 范围。

[^38]: [Cedar，Authorization semantics](https://docs.cedarpolicy.com/auth/authorization.html)。default deny、forbid 优先、求值错误跳过与 diagnostics；与 Contexpect Unknown 的映射必须另行处理。

[^23]: [SQLite，Write-Ahead Logging](https://www.sqlite.org/wal.html)。WAL、共享内存、并发与网络文件系统边界。

[^36]: [Syncthing，Synchronization](https://docs.syncthing.net/users/syncing.html)。冲突副本、文件变化与大小写等行为；项目 https://github.com/syncthing/syncthing 。

[^46]: [Inspect AI，Evaluation Logs](https://inspect.aisi.org.uk/eval-logs.html)。输入/输出/转录和 API 错误数据的日志行为，重点用于隐私准入。

[^42]: [Tauri 2，Capabilities](https://v2.tauri.app/security/capabilities/)。窗口/WebView 的 IPC 权限及合并边界；不等于 HTTP API 授权。

[^53]: [Tauri 2，WebDriver](https://v2.tauri.app/develop/tests/webdriver/)。当前官方内嵌 WebDriver / WebdriverIO 路线覆盖 macOS；与直接 tauri-driver 及浏览器 mock 路线区分。页面标示更新于 2026-06-29。

[^27]: [Gitleaks，CLI 与维护说明](https://github.com/gitleaks/gitleaks)。检测、配置优先级、忽略项、脱敏与安全补丁维护状态；另读 Action 使用条件 https://github.com/gitleaks/gitleaks-action ，不与 CLI 混同。

[^56]: [Betterleaks](https://github.com/betterleaks/betterleaks)。Gitleaks 后续开发方向；HTTP 凭据验证和远程数据源解释了为什么不自动纳入被动扫描。
