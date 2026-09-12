# 来源、证据等级与可追溯入口

## 1. 资料范围与时效

本次任务是归档本会话的公开研究、最终建议和设计反馈。依据包括可见对话、两份完整MD附件、11张可取的设计图；没有新增一轮互联网调研，没有读取其他会话/Library内容，也没有把外部README当前状态或项目HEAD重新验为最新。

研究基线沿用`257b7d4d82c0865a94b45d9b3fddcb3830658b32`；前次资料核验日期为2026-09-11。后续实施须重新锁定相应外部release/commit、features、摘要与许可，不能把下面的URL当成获准依赖锁。

## 2. 本地一级归档

| ID | 来源 | 完整性/效力 |
| --- | --- | --- |
| R1 | [工程借鉴与整合报告](originals/Contexpect_Engineering_Reference_and_Integration_Report_2026-09-11.md) | 原文件字节保留；20环节、25冲突、55条来源 |
| R2 | [最终建议与实施决策书](originals/Contexpect_Final_Recommendations_2026-09-11.md) | 原文件字节保留；最终咨询纠偏、T01–T12、Q01–Q16 |
| CHAT | 本会话可见的用户要求及公开分析 | 沿革见12；不是另做的完整聊天导出 |
| VIS | [最终浅色概念图](assets/approved/11_white_monochrome_overview.png) | 仅支持视觉方向，参数/统计/事实不得据图认定 |
| HISTORY | 首轮8图、深色科技图、黑底白字图 | 完整包保留；详见14，不作为默认风格 |

字节摘要、尺寸、文件身份和别名映射见 [资产清单](14_ASSET_MANIFEST.md)。如果只取得纯MD包，图片需从完整包读取；本文文字规范仍足以说明方向。

## 3. 证据强度与使用限制

源代码和官方文档可说明相应版本的设计与行为；只有实际执行记录能说明某个构建/测试跑过；只有真实目标输入和运行证据能说明特定互操作成立；完整独立验收需要独立输入、执行范围与人员/平台记录。

前报告对第三方项目的选择，不构成其整体安全认证、许可批准或已完成集成。维护者基准、Star数、规则数和屏幕上的数字不能升级为产品效果。未取得可追溯原文的论文、数字或引用不完整的断言，不进入任务验收依据。

## 4. 工程报告原始来源台账

保留原编号以便核对：原报告从1到56，缺9，共55条。下面是原有阅读入口，不表示本轮又完成55次独立核验。

### E01（原报告脚注 1）

[Contexpect，基线提交](https://github.com/Octo-o-o-o/Contexpect/commit/257b7d4d82c0865a94b45d9b3fddcb3830658b32)。核定仓库版本与工具链固定记录。

### E02（原报告脚注 2）

[Contexpect，架构总览](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/overview.md)。组件职责、单一真值、入口、隐私与非目标。

### E03（原报告脚注 3）

[Contexpect，数据与真值模型](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/data-and-truth-model.md)。六个 facet、Claim、Unknown、Receipt、等价与签名边界。

### E04（原报告脚注 4）

[Contexpect，实施计划](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/process/implementation-plan.md)。F-01–F-18 / WP-01–WP-12 映射、冻结范围与完成条件。

### E05（原报告脚注 5）

[Contexpect，ADR 0006](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/adr/0006-third-party-dependency-policy-and-estimator.md)。零第三方依赖、存储偏离退出条件、v2 估计器和 Playwright 决策。

### E06（原报告脚注 6）

[Contexpect，ctxpect-projection](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-projection/src/lib.rs)。预览绑定、事务消费、控制路径、并发摘要、回滚与 secret gate。

### E07（原报告脚注 7）

[Contexpect，ctxpect-importer](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-importer/src/lib.rs)。原生 DSH 路径与通用 events 导入之间的实现边界。

### E08（原报告脚注 8）

[Contexpect，ctxpect-sync](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-sync/src/lib.rs)。加密未就绪时的拒绝行为及 folder bundle 语义。

### E10（原报告脚注 10）

[ripgrep，ignore crate](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore)。遍历、过滤与 ignore 规则；不是上下文原生加载规则。

### E11（原报告脚注 11）

[Bytecode Alliance，cap-std](https://github.com/bytecodealliance/cap-std)。目录能力和受限文件系统 API；配合平台安全测试。

### E12（原报告脚注 12）

[OpenAI Codex，AGENTS.md discovery 源码](https://github.com/openai/codex/blob/fc948f8c473e5d11e780ffcf1fd7f812a2020932/codex-rs/core/src/agents_md.rs)。原生根标记、目录链、字节预算和环境/权限分支；该提交不是 Contexpect 旧锚点版本的替代品。

### E13（原报告脚注 13）

[Gemini CLI，MemoryContextManager 源码](https://github.com/google-gemini/gemini-cli/blob/ed2ac40df67a319bf348bd7e3d10494696b31b38/packages/core/src/context/memoryContextManager.ts)。全局/扩展/项目分层、文件身份去重；关联读取同提交 utils/memoryDiscovery.ts。

### E14（原报告脚注 14）

[jsonschema，Rust 实现](https://github.com/Stranger6667/jsonschema)。schema 验证、feature 表和外部引用解析；接入必须关闭不受控解析。

### E15（原报告脚注 15）

[JSON Schema Test Suite](https://github.com/json-schema-org/JSON-Schema-Test-Suite)。按 draft 划分的独立一致性测试。

### E16（原报告脚注 16）

[Scopeon，架构说明](https://github.com/sorunokoe/Scopeon/blob/main/docs/architecture.md)。collector、parse_incremental、纯指标层、SQLite 与文件 offset；产品入口 https://github.com/sorunokoe/Scopeon 。

### E17（原报告脚注 17）

[ContextSpy](https://github.com/RimantasZ/contextspy)。请求 profiling、捕获与本地存储边界。

### E18（原报告脚注 18）

[in-toto Attestation，Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md)。subject、predicateType 和声明包络；不等于运行事实自动可信。

### E19（原报告脚注 19）

[Sigstore，Cosign verification](https://docs.sigstore.dev/cosign/verifying/verify/)。签名、bundle、身份和验证材料；离线使用须准备可信材料。

### E20（原报告脚注 20）

[IETF，RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785.html)。JSON 规范化标准；不能静默替换既有签名序列化规则。

### E21（原报告脚注 21）

[rusqlite](https://github.com/rusqlite/rusqlite)。SQLite Rust 绑定、features、bundled 构建与工具链要求。

### E22（原报告脚注 22）

[SQLite，How SQLite Is Tested](https://www.sqlite.org/testing.html)。故障注入、I/O/OOM、模糊与恢复测试思路；并非所有测试套件都公开可复制。

### E23（原报告脚注 23）

[SQLite，Write-Ahead Logging](https://www.sqlite.org/wal.html)。WAL、共享内存、并发与网络文件系统边界。

### E24（原报告脚注 24）

[Difftastic](https://github.com/Wilfred/difftastic)。语法结构差异显示；不是业务语义等价证明。

### E25（原报告脚注 25）

[CtxWise，README](https://github.com/FramY2/ctxwise/blob/main/README.md)。活跃根到 cwd 链、lock/drift、透明估计及独立发现问题后的回归说明；勿与无关 ctxray 项目混淆。

### E26（原报告脚注 26）

[agnix](https://github.com/agent-sh/agnix)。规则元数据、共享引擎、解释、修复等级及多入口；贡献说明 https://github.com/agent-sh/agnix/blob/main/CONTRIBUTING.md 。

### E27（原报告脚注 27）

[Gitleaks，CLI 与维护说明](https://github.com/gitleaks/gitleaks)。检测、配置优先级、忽略项、脱敏与安全补丁维护状态；另读 Action 使用条件 https://github.com/gitleaks/gitleaks-action ，不与 CLI 混同。

### E28（原报告脚注 28）

[Microsoft APM](https://github.com/microsoft/apm)。包 manifest/lock、来源、策略、audit/drift 与 SBOM；官方文档 https://microsoft.github.io/apm/ 。

### E29（原报告脚注 29）

[Anchore，Syft](https://github.com/anchore/syft)。制品和软件依赖的 SBOM 生成，补足软件供应链而非上下文事实。

### E30（原报告脚注 30）

[Rulesync](https://github.com/dyoshikawa/rulesync)。多工具 native 转换、导入与生成；官方文档 https://rulesync.dyoshikawa.com/ 。

### E31（原报告脚注 31）

[agentsync，README 与 Known Limits](https://github.com/spxrogers/agentsync/blob/main/README.md)。投影与对账思路；重点检查 JSONC、owned keys、明文备份、Windows 权限及 rollback 范围。

### E32（原报告脚注 32）

[chezmoi，Concepts](https://www.chezmoi.io/reference/concepts/)。source、target、destination state 与应用模型；不直接采用模板执行权限。

### E33（原报告脚注 33）

[Agentpack](https://github.com/liqiongyu/agentpack)。受管资产、preview/diff、snapshot/rollback；发布记录 https://github.com/liqiongyu/agentpack/releases 。

### E34（原报告脚注 34）

[age，项目与格式入口](https://github.com/FiloSottile/age)。文件加密格式、收件人与互操作边界。

### E35（原报告脚注 35）

[rage，Rust age 实现](https://github.com/str4d/rage)。既有加密格式的 Rust 实现，不自行设计密码协议。

### E36（原报告脚注 36）

[Syncthing，Synchronization](https://docs.syncthing.net/users/syncing.html)。冲突副本、文件变化与大小写等行为；项目 https://github.com/syncthing/syncthing 。

### E37（原报告脚注 37）

[Cedar policy language](https://github.com/cedar-policy/cedar)。策略表达、校验和授权求值实现。

### E38（原报告脚注 38）

[Cedar，Authorization semantics](https://docs.cedarpolicy.com/auth/authorization.html)。default deny、forbid 优先、求值错误跳过与 diagnostics；与 Contexpect Unknown 的映射必须另行处理。

### E39（原报告脚注 39）

[Open Policy Agent，Policy Language](https://www.openpolicyagent.org/docs/policy-language)。Rego、undefined 与策略求值；仅作备选或既有组织系统接口。

### E40（原报告脚注 40）

[Axum](https://github.com/tokio-rs/axum)。extractor、router、中间件组合与 released/main 区别。

### E41（原报告脚注 41）

[notify](https://github.com/notify-rs/notify)。文件监控后端、平台差异与轮询降级；事件不能替代重扫。

### E42（原报告脚注 42）

[Tauri 2，Capabilities](https://v2.tauri.app/security/capabilities/)。窗口/WebView 的 IPC 权限及合并边界；不等于 HTTP API 授权。

### E43（原报告脚注 43）

[TanStack Query，Query Keys](https://tanstack.com/query/latest/docs/framework/react/guides/query-keys)。查询身份与变量依赖，对应 coordinate/receipt 缓存键。

### E44（原报告脚注 44）

[Rig](https://github.com/0xPlaygrounds/rig)。provider/backend 与 agent runtime 分层、测试和兼容性提醒。

### E45（原报告脚注 45）

[UK AI Security Institute，Inspect AI](https://github.com/UKGovernmentBEIS/inspect_ai)。任务、评测、solver/scorer 与外部 runner 设计。

### E46（原报告脚注 46）

[Inspect AI，Evaluation Logs](https://inspect.aisi.org.uk/eval-logs.html)。输入/输出/转录和 API 错误数据的日志行为，重点用于隐私准入。

### E47（原报告脚注 47）

[Inspect AI，Sandboxing](https://inspect.aisi.org.uk/sandboxing.html)。实验执行环境；不代替自有文件、网络与数据留存授权。

### E48（原报告脚注 48）

[promptfoo](https://github.com/promptfoo/promptfoo)。声明式评测用例、provider、assertion 和 CI 入口。

### E49（原报告脚注 49）

[tracing](https://github.com/tokio-rs/tracing)。span/event、subscriber 和 Rust 诊断分层。

### E50（原报告脚注 50）

[OpenTelemetry，Collector](https://opentelemetry.io/docs/collector/)。receiver/processor/exporter 管线；与证据账本分离。

### E51（原报告脚注 51）

[proptest](https://github.com/proptest-rs/proptest)。属性测试、收缩与失败重放；开发依赖仍受项目 ADR 管理。

### E52（原报告脚注 52）

[Playwright，Assertions](https://playwright.dev/docs/test-assertions)。Web-first 自动等待/重试断言和浏览器闭环。

### E53（原报告脚注 53）

[Tauri 2，WebDriver](https://v2.tauri.app/develop/tests/webdriver/)。当前官方内嵌 WebDriver / WebdriverIO 路线覆盖 macOS；与直接 tauri-driver 及浏览器 mock 路线区分。页面标示更新于 2026-06-29。

### E54（原报告脚注 54）

[dist / cargo-dist](https://github.com/axodotdev/cargo-dist)。跨平台构建、制品 manifest 与发布阶段；CLI 发布不替代桌面平台签名。

### E55（原报告脚注 55）

[cargo-deny](https://embarkstudios.github.io/cargo-deny/)。依赖 advisories、bans、licenses 和 sources 门禁。

### E56（原报告脚注 56）

[Betterleaks](https://github.com/betterleaks/betterleaks)。Gitleaks 后续开发方向；HTTP 凭据验证和远程数据源解释了为什么不自动纳入被动扫描。

## 5. 最终建议补充引用

以下保留最终建议的脚注键，便于定位其新增纠偏与原始依据。重复指向同一来源不算额外证据。

### F-research

《Contexpect 分环节工程借鉴与整合决策报告》，2026-09-11，本会话生成文件 `Contexpect_Engineering_Reference_and_Integration_Report_2026-09-11.md`。本轮已完整复核其 20 环节、25 项冲突、推荐组合与来源部分；文件摘要见本文开头。属于本会话综合研究，不是额外的独立产品验收。

### F-architecture

Contexpect，[架构总览](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/overview.md)。用于组件职责、四类入口、非目标和阶段实现边界。

### F-truth

Contexpect，[数据与真值模型](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/data-and-truth-model.md)。用于六个独立 facet、Claim 多轴、Unknown、Receipt 不可变、等价、身份与本地连续性签名。本文第 4、9 章的扩展均为实施建议，不表示这些新增边界已经全部实现。

### F-semantic

Contexpect，[语义对齐与 Team Context Standard](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/semantic-alignment-and-team-standard.md)。用于原生投影、authority、loss、团队标准、例外与披露。第 4.2 节建议修订其 ST2 中强制不同 native 正文的条款；这是本轮新增的明确规范修订建议。

### F-plan

Contexpect，[实施计划](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/process/implementation-plan.md)。用于冻结 cutoff、F-01–F-18 / WP-01–WP-12 映射及完整交付条件；基线提交入口为 [257b7d4](https://github.com/Octo-o-o-o/Contexpect/commit/257b7d4d82c0865a94b45d9b3fddcb3830658b32)。

### F-adr6

Contexpect，[ADR 0006：第三方依赖政策、Effect Lab 估计器与存储偏离的结束条件](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/adr/0006-third-party-dependency-policy-and-estimator.md)。用于当前零第三方依赖决策、SQLite 退出条件、v2 估计器及 Playwright/真实 runner 边界。本文提出的是正式复议建议，不是对此 ADR 的已批准替换。

### F-conformance

Contexpect，[静态 conformance 测试](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-cli/tests/corpus_conformance.rs)。用于 1,924 行开发语料、两锚点 instructions、honesty 与未执行范围的计数边界；不是本轮重新运行的结果。

### F-doctor

Contexpect，[Doctor 规则映射](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/process/doctor-rule-map.md)。用于 20 条规则、707 行语料、声明输入、suppression、阻断资格和固定 stale cutoff。运行时显式时钟与独立阻断准入是本文建议，不是已确认上线能力。

### F-projection

Contexpect，[ctxpect-projection 实现](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-projection/src/lib.rs)。用于预览绑定、单次消费、目标摘要、事务、控制路径及回滚保护；不等于对任意外部并发写入已实现全局事务隔离。

### F-importer

Contexpect，[ctxpect-importer 实现](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-importer/src/lib.rs)。用于 DSH 专用路径、通用 `events[]` 输入及 metadata-only 边界。

### F-sync

Contexpect，[ctxpect-sync 实现](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-sync/src/lib.rs)。用于加密尚未就绪时的拒绝行为与 folder bundle 阶段实现。

### F-ctxwise

CtxWise，[README](https://github.com/FramY2/ctxwise/blob/main/README.md)。用于 Codex 审计、lock/drift、测量边界和集中产品入口的参考。属于本会话已核验的外部资料，动态分支不作为依赖版本锁。

### F-agnix

agnix，[项目仓库](https://github.com/agent-sh/agnix)。用于规则元数据、解释、修复分级及共享内核服务多入口的工程参考；不是建议直接执行全部外部修复。

### F-apm

Microsoft APM，[项目仓库](https://github.com/microsoft/apm)与[官方文档](https://microsoft.github.io/apm/)。用于上下文包来源、manifest/lock、审计和 SBOM 产物；自动安装的真实副作用需要另行固定版本验证。

### F-rulesync

Rulesync，[项目仓库](https://github.com/dyoshikawa/rulesync)与[官方文档](https://rulesync.dyoshikawa.com/)。用于跨工具原生格式转换参考；候选 renderer 的隔离与只写 scratch 是 Contexpect 的建议接入条件，不是对任意 Rulesync 版本的安全保证。

### F-jsonschema

Rust jsonschema，[项目与 feature 说明](https://github.com/Stranger6667/jsonschema)。本轮复核其外部引用解析入口；正式准入还需核对所选发布版本的 `default-features` 与实际 feature 组合。独立一致性参考为 [JSON Schema Test Suite](https://github.com/json-schema-org/JSON-Schema-Test-Suite)。

### F-sqlite

SQLite，[How SQLite Is Tested](https://www.sqlite.org/testing.html)与[Write-Ahead Logging](https://www.sqlite.org/wal.html)；Rust 绑定参考 [rusqlite](https://github.com/rusqlite/rusqlite)。用于故障验证思路、本地 WAL 边界和目标存储实现；不是已经在 Contexpect 通过的迁移证明。

### F-age

[age 格式与实现入口](https://github.com/FiloSottile/age)、[rage Rust 实现](https://github.com/str4d/rage)。用于标准加密格式与实现选项。加密、签名、版本谱系、撤销和历史副本回收必须分别建模。

### F-cedar

Cedar，[Authorization semantics](https://docs.cedarpolicy.com/auth/authorization.html)。本轮复核 default deny、forbid overrides permit、skip-on-error 与 diagnostics。本文的 required 证据与关键错误阻断属于 Contexpect 外层应用合同。

### F-tauri

Tauri 2，[WebDriver](https://v2.tauri.app/develop/tests/webdriver/)与[Capabilities](https://v2.tauri.app/security/capabilities/)。本轮复核官方推荐的 WebdriverIO/内嵌 WebDriver 路线与 macOS 支持方式；浏览器、真实 WebView、测试插件和发行构建必须分开验收。

### F-inspect

Inspect AI，[Evaluation Logs](https://inspect.aisi.org.uk/eval-logs.html)、[Sandboxing](https://inspect.aisi.org.uk/sandboxing.html)及[项目仓库](https://github.com/UKGovernmentBEIS/inspect_ai)。用于外部 runner、日志隐私和受控执行设计；runner 成功不等于 Contexpect 的统计效果成立。


## 6. 实施前最值得重新核验的动态项

| 项目 | 为什么需要再核验 | 当前处理 |
| --- | --- | --- |
| 实际仓库HEAD与测试 | 基线之后可能已有变化 | T01差异核对，不按旧状态重做 |
| 上游harness规则 | main不等于被冻结的工具版本 | 对应版本取样、差分测试 |
| 第三方默认features | schema/HTTP/文件解析/网络默认会影响边界 | 固定features并以权限测试确认 |
| Gitleaks/Betterleaks维护与默认行为 | 前报告的维护状态是时点记录 | 检测器可替换；禁止默认联网验证凭据 |
| Tauri测试方案与平台支持 | 前次资料提供候选，不是本项目实测 | 目标版本和实际平台验；测试能力不出货 |
| Rulesync/APM执行范围 | 命令“成功”不说明完整副作用 | 只读优先，受控I/O/进程/网络准入 |
| Rig/Inspect和日志 | provider行为、breaking change、错误日志可能变化 | 固定版本/最小权限/留存检查 |
| 许可、传递依赖与advisory | 源码可见不等于随意复制 | 文件级许可证、NOTICE、时效与离线材料 |

## 7. 怎样引用

产品事实引用对应基线文件和字段；本包的建议引用R2章节；外部工程细节引用E/F来源并说明版本；视觉偏好引用VIS及用户反馈。概念图不作为后端事实来源；本包不能被后续当作“已跑通原生运行时”的证据。
