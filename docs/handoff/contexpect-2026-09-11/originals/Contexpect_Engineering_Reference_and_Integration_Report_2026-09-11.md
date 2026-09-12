# Contexpect 分环节工程借鉴与整合决策报告

**阅读导航：** [摘要](#summary) · [基线](#scope) · [20 环节总表](#modules) · [逐项调研](#research) · [接口合同](#contracts) · [25 项冲突](#conflicts) · [推荐组合](#combination) · [实施顺序](#delivery) · [依赖治理](#dependencies) · [整体复审](#review) · [来源](#sources)

<a id="summary"></a>

## 执行摘要

**建议保留 Contexpect 自己的上下文真值与核对内核，在通用基础设施上选择性复用成熟组件，在同赛道产品上主要借鉴协议、规则、测试和交互，而不是把几个完整产品拼装成一个更大的产品。**

最重要的区分是：**值得研究的项目，不等于应该成为运行时依赖；有用的外部报告，不等于可以直接写入 Contexpect 的权威 Claim；能生成配置，不等于可以获得修改真实配置的权限。**

本报告把完整产品拆成 20 个有明确输入、输出和验收边界的环节。每个环节选择 1–2 个最值得优先研究的项目，说明工程依据、适合借鉴的部分、不应照搬的行为、接入方式、优先级与验收条件。随后对这些选择做整体复审，给出冲突处理、唯一推荐组合、接口合同和实施顺序。

整体结论如下。

| 决策问题 | 结论 |
| --- | --- |
| 是否应该整体采用某一个相似产品作为底座？ | 不建议。它们的真值模型、写入权、隐私默认值和产品边界与 Contexpect 不一致。 |
| 是否每个环节都需要新增依赖？ | 不需要。相当一部分收益来自源码规则、反例、测试模式和接口设计，不来自链接整个库。 |
| 最值得保留的自主能力是什么？ | 解析坐标、Claim 六个独立证据面、Unknown、证据覆盖、Receipt、核对、投影损失和修改后重新取证。 |
| 最值得优先研究的同赛道项目是什么？ | agnix、CtxWise、APM、Rulesync；Scopeon、Agentpack 和 agentsync 提供有用的局部工程设计，但应更谨慎地复用。 |
| 通用基础设施的推荐方向是什么？ | 能力受限文件访问、严格 schema、SQLite、本地 API、标准加密、属性测试与跨端测试；库级引入必须先满足项目自己的依赖政策。 |
| 最大整合风险是什么？ | 多个配置写入者、多个事实来源、采样日志冒充证据、明文备份、语法相同冒充语义相同，以及外部组件默认行为绕过授权。 |
| 应该先证明什么？ | 一个真实项目中“预期 → 原生观察 → 找出差异 → 经批准修复 → 重新核对 → 可回滚”的可重复闭环。 |

**最终方案不是“安装 30 多个工具”，而是“一个 Contexpect 内核、一个变更协调入口、一个权威本地账本，以及少量经过授权的外部适配器”。** 表中的优先级是依赖与价值顺序，不是裁剪 F-01–F-18 或 WP-01–WP-12 的最终合同。[^2][^4]

<a id="scope"></a>

## 一、分析基线与适用边界

### 1.1 项目基线

项目基线为 `Octo-o-o-o/Contexpect` 的 `main` 提交 `257b7d4d82c0865a94b45d9b3fddcb3830658b32`，提交时间为 2026-09-11 02:16:10 UTC。本报告的外部资料核验日期为 2026-09-11。Contexpect 的原验收 cutoff 为 `2026-09-04T23:59:59+08:00`；新发现的上游实现只能成为研究输入，不能自动变成已支持的冻结坐标。[^1][^4]

当前项目已有本地 Rust crates、CLI、localhost API、React UI、独立 workspace 的 Tauri 壳和 JSON ledger；目标 SQLite/FTS5、完整加密同步及完整适配矩阵不能按已完成能力计算。源码存在、测试存在、测试执行成功、真实工具互操作验收成功，是四种不同的证据。[^2][^3]

**一个影响全部选型的约束：ADR 0006 仍明确要求根 Rust workspace 零第三方 crate。** 它已规定第三方依赖引入的条件：出现需要解决的功能缺口、准备离线 vendoring、通过新的 ADR supersede 并同步相关门禁。Playwright 是已经获准的前端开发依赖，不代表 Rust 可以随意增加库。[^5]

### 1.2 本报告中的“工程化好”如何判断

优先看可核对的工程资产：明确的输入输出合同、异常与降级行为、测试和反例、兼容性与迁移说明、发布产物、安全边界、许可证，以及是否能以小组件接入。Star 数、功能数量、README 的覆盖百分比和维护者基准数字，不作为质量排名依据。

证据强度分为三类，避免把所有入选项目都包装成同等成熟。

| 证据类别 | 含义 | 应如何使用 |
| --- | --- | --- |
| 基础设施型 | 有明确标准、长期维护的实现或公开一致性/故障测试体系 | 可成为库级候选，但仍需目标版本、目标平台和目标配置验收 |
| 产品工程型 | 有清晰模块边界、CLI/协议、回归测试、发布或迁移说明 | 优先借鉴接口和局部实现，通过适配器接入 |
| 场景参考型 | 与问题高度相关，但独立生产验证或稳定性证据不足 | 用作样例、规则来源和研究对照，不放在安全或真值根上 |

这不是安全认证。本报告没有对每个外部项目进行完整源码审计、现场构建、真实 provider 调用或长时间压力测试。具体“已支持”均指相应一手资料公开描述的能力；“推荐”“建议”是针对 Contexpect 的分析判断。未核定的版本和构建摘要不伪造，接入前必须补齐。

### 1.3 四种借鉴方式

| 标记 | 方式 | 与现有约束的关系 |
| --- | --- | --- |
| D | 设计、规则、测试思路借鉴 | 通常不增加运行时依赖；复制代码或测试文件仍要处理许可和来源 |
| I | 导入外部产物或受控进程适配 | 先支持用户明确提供的产物；自动启动进程要另过现有 executor 与授权边界 |
| L | 库级引入 | 根 Rust workspace 需要先 supersede ADR 0006；不得以 dev-dependency 或 sidecar 名义规避 |
| T | 开发、测试和发布工具 | 不随产品常驻；也必须固定版本、来源、离线策略和允许执行范围 |

以下的“推荐项目”是每个环节的优先研究对象；只有第六章标为默认接入的部分，才进入最终组合。其他候选保留为设计参考或备选。

<a id="modules"></a>

## 二、完整产品拆分与覆盖映射

“独立环节”表示可以单独规定输入、输出和测试，不表示建立 20 个服务、20 个数据库或 20 套对象模型。已有 crate 可以继续承载相邻环节；不为本报告中的编号机械新建空 crate。

| 环节 | 独立职责 | 与现有实现/计划的对应 | 优先参考的 1–2 个项目 |
| --- | --- | --- | --- |
| M01 | 环境、资产发现与安全文件采集 | fs、collect；F-01/F-02，WP-02 | ripgrep/ignore、cap-std |
| M02 | 版本化原生规则解析与 adapter | resolve；F-03/F-04/F-17，WP-02/WP-11 | OpenAI Codex、Gemini CLI |
| M03 | Context IR、schema 与合同校验 | core、schema；WP-01 | jsonschema、JSON Schema Test Suite |
| M04 | 运行时导入与证据归一化 | importer；F-12，WP-05 | Scopeon、ContextSpy |
| M05 | Receipt、来源与签名验证 | receipt/evidence；F-05，WP-03 | in-toto Attestation、Cosign |
| M06 | 本地账本、快照、检索与删除 | store；WP-03 | SQLite、rusqlite |
| M07 | 跨坐标 diff、drift 与等价比较 | diff；F-07，WP-03/WP-04 | Difftastic、CtxWise |
| M08 | Doctor、泄漏检测与修复建议 | doctor；F-08，WP-02/WP-08 | agnix、Gitleaks |
| M09 | 资产包、依赖、来源与 SBOM | assets/catalog；F-11，WP-08 | Microsoft APM、Syft |
| M10 | CanonicalIntent 到原生配置的投影 | projection；F-09，WP-06 | Rulesync、agentsync |
| M11 | 批准、实际写入、事务与回滚 | projection/fs/policy；WP-06 | chezmoi、Agentpack |
| M12 | 加密 bundle 与多设备同步 | sync/crypto；F-10，WP-07 | rage（age 实现）、Syncthing |
| M13 | 团队标准、策略、例外与授权 | policy/standards；F-18，WP-07/WP-11 | Cedar、OPA |
| M14 | daemon、本地 API、增量监测与调度 | 当前 CLI 内 API；F-16，WP-05 | Axum、notify |
| M15 | 桌面壳、UI 状态与交互 | apps/desktop、packages/ui；F-06，WP-04 | Tauri 2、TanStack Query |
| M16 | LLM Advisor 与历史建议 | advisor；F-13/F-14，WP-09 | Rig |
| M17 | Effect Lab 与受控实验执行 | effect；F-15，WP-10 | Inspect AI、promptfoo |
| M18 | 产品自身的日志、资源与性能观测 | daemon/CLI 横切能力；F-16 | tracing、OpenTelemetry Collector |
| M19 | 属性、变异、端到端与跨平台测试 | acceptance、tests；WP-01/WP-12 | proptest、Playwright |
| M20 | CLI/桌面发布与依赖供应链门禁 | workflows/release；F-16/F-17，WP-11/WP-12 | dist/cargo-dist、cargo-deny |

映射依据为仓库的 F → WP → crate/命令/测试表。表中没有新增 F-19，也不改变十八个 family 的原声明范围。[^4]

<a id="research"></a>

## 三、逐环节调研与落地建议

### M01 环境、资产发现与安全文件采集

**职责。** 输入显式授权的项目根、工具目录、环境坐标和扫描策略；输出带来源、文件身份和不可读原因的 Inventory。发现路径不等于允许读取，读取成功不等于原生工具会加载。

**优先参考一：ripgrep 的 `ignore`。** 它把目录遍历、Git ignore、glob、文件类型过滤等通用工作封装为可配置组件；适合借鉴“遍历”和“匹配”的分离，而不是搬入文本搜索产品。阅读入口为 `crates/ignore` 和 ripgrep 的过滤说明。[^10]

**优先参考二：cap-std。** 它以目录能力和相对操作减少环境权限的无意扩散，提供跨平台能力导向文件接口及测试。它不是不可信 Rust 代码的完整沙箱；进程仍可能通过其他接口获取环境权限。[^11]

**建议接入。** D 优先，L 条件引入。保留 `ctxpect-fs` 作为唯一受控文件边界，内部再选择是否使用 cap-std；collector 不直接暴露任意绝对路径操作。将授权目录句柄/根对象传给 adapter，避免 adapter 自行推测 `$HOME`。

**不能照搬。** `ignore` 的隐藏目录和 ignore 默认行为不能成为原生上下文规则，否则 `.claude`、`.cursor` 等恰好重要的目录可能被漏掉。用户要求 Contexpect 不读某文件，只能得到“观察范围外”，不能得到“原生工具不会加载”。cap-std 也不能代替 hardlink、junction、跨卷和文件类型验证。

**验收与回报。** P0；改造中等、平台复核较高。测试 symlink 切换、祖先目录替换、FIFO、hardlink、Windows 大小写/路径别名、权限拒绝和隐藏配置目录。扫描不启动 hook/MCP、不写用户目录；错误输出没有绝对 home 或 secret。回报是减少所有后续模块重复实现文件边界的风险。

### M02 版本化原生规则解析与 adapter

**职责。** 输入 Inventory、精确工具版本/surface/OS、cwd、任务条件和授权设置；输出 Expected claims、纳入/排除理由以及适用范围。不得由自己的预算偏好修改原生预期。

**优先参考一：OpenAI Codex。** 本次定位到的上游快照 `fc948f8c…` 中，`codex-rs/core/src/agents_md.rs` 明确处理根标记、fallback 名称、预算、环境和权限路径。它是比二手“AGENTS.md 加载规则总结”更直接的语义来源。这个上游快照不是 Contexpect 已冻结版本的验收证明。[^12]

**优先参考二：Gemini CLI。** 快照 `ed2ac40d…` 的 `memoryContextManager.ts` 将全局、扩展、项目和用户项目记忆分开，并按文件身份处理去重；`memoryDiscovery.ts` 是相邻发现入口。适合研究多根、按需记忆和动态刷新，而不是把 Gemini 行为套给其他工具。[^13]

**建议接入。** D + T。每条 resolver 规则记录原生源 commit、精确版本、测试样本和证据等级；优先复用官方公开样例/测试思路构造独立对照。开源 harness 可在受控测试环境中执行原生解析；闭源工具只能使用实际公开且可获得的导出面，缺失则 Unknown。

**不能照搬。** 不把上游整个 agent 作为 core 依赖，不在被动 inspect 中启动它，也不把上游 main 的规则静默应用于历史版本。优先做深现有 Codex/Claude 锚点；Gemini 是扩展实现的优秀参考，不改变当前支持顺序或声明。

**验收与回报。** P0，改造较高。每个 supported cell 对同一输入比较原生输出和本地推演，覆盖 import 循环、根目录截断、权限、空文件、配置覆盖、Unicode 和目录别名。原生结果与 resolver 不一致时保留冲突，不取“更像正确答案”的一方。回报是项目真正的长期适配资产。

### M03 Context IR、schema 与合同校验

**职责。** 输入版本化结构化对象；输出校验结果、字段定位和拒绝原因。这里只校验对象表达是否合法，不判断模型是否看到了内容。

**优先参考一：`Stranger6667/jsonschema`。** Rust 实现有多个 JSON Schema draft、详细错误路径和公开一致性测试关联。一个关键默认值是 `resolve-http` 与 `resolve-file` 开启，会解析远程或文件引用；这与封闭、离线校验边界不天然相容。[^14]

**优先参考二：JSON Schema Test Suite。** 它提供与语言实现无关的规范测试，适合防止自写 schema 校验器只接受自己生成的样例。它校验的是 schema 语义，不是 Contexpect 的 Claim 业务真值。[^15]

**建议接入。** 当前 D/T，库级 L 需 ADR。保留现有 IR/Claim/Receipt 唯一对象体系；引入标准验证器时关闭默认外部引用，使用内置、版本固定的 schema registry。Rust 与 TypeScript 使用同一 DTO 合同，不再手写一套相似但不一致的前端状态枚举。

**不能照搬。** schema canonicalization 不是签名用的字节 canonicalization；普通 JSON 反序列化也不保证保留上游数字词素。现有签名与 DSH 导入的数值处理不能因换 parser 被悄悄改变。JCS/RFC 8785 对数字、排序等有自己的合同；采用它需要单独版本化迁移。[^20]

**验收与回报。** P0；换库中等、历史兼容较高。测试重复 key、深层嵌套、超大数字、浮点词素、未知 draft、恶意 `$ref`、超过限制的数组和旧 Receipt。校验时文件和网络访问必须为零；仅改变键顺序不能改变既定 canonical 结果，改变签名规则必须产生新算法版本。

### M04 运行时导入与证据归一化

**职责。** 输入明确授权的原生日志/导出；输出事件、请求、序列与覆盖证据。请求已准备、请求已派发、收到结果、模型内部使用和结果影响，必须分别处理。

**优先参考一：Scopeon。** 其架构将 provider 的增量 JSONL 解析、文件 offset、SQLite 持久化和无 I/O 的指标计算分开；`scopeon-collector` 与 `scopeon-metrics` 是最适合阅读的边界。它的领域对象偏 Session/Turn/metrics，不是 Contexpect Claim。[^16]

**优先参考二：ContextSpy。** 它提供请求级上下文分析，采用代理捕获和本地数据/界面。其价值是研究请求分区、前后比较和 profiler 交互；默认采集正文的路径与 Contexpect metadata-first 明显不同。[^17]

**建议接入。** Scopeon 以 D/I 为主：借鉴 partial line、轮转、截断、重放、provider dispatch，保留自己的 field-to-claim mapping。ContextSpy 只作为明确授权的外部 profiler/export 输入或设计参考，不加入默认常驻链路。现有 DeepSeek Harness 专用导入器继续保留，不降格为通用 `events[]` 的表面兼容。

**不能照搬。** 不导入另一个产品的完整数据库，不把它标记的 exact 自动升级成模型可见，也不默认导入原始请求正文。只有具备足够来源与覆盖语义的字段才能支撑相应 Claim；metrics-only 导出只能贡献它真正包含的字段。

**验收与回报。** P0/P1，改造较高。日志写到半行、重复片段、sequence gap、压缩替换、重命名轮转、provider 版本变化和撤销授权均有负例。幂等键应包含来源域和事件身份，不能仅凭全局 session_id。回报是把“已登记适配名称”转化为可信的原生数据覆盖。

### M05 Context Receipt、来源与签名

**职责。** 输入已经核对的快照和证据引用；输出不可变 Receipt、签名验证结果与导出包。签名证明签名者和字节关系，不自动证明记录内容真实、完整或观察方式可信。

**优先参考一：in-toto Attestation。** Statement 通过 subject digest 与 predicate type 把对象和声明内容分开，是为 Receipt 提供可互操作外部封装的合适参照，不是要求重写整个内部数据模型。[^18]

**优先参考二：Sigstore/Cosign。** 其验证工具与 bundle、身份/issuer 校验设计适合研究发布产物及组织签名验证。离线验证仍需要受信根、必要的验证材料和明确策略，不是“带着一个签名就可信”。[^19]

**建议接入。** D/I，后续受控 verifier。Receipt 内部格式与既有签名版本先保持不变；可在外部 attestation 的 subject 里绑定 Receipt 摘要。组织身份验证与本地连续性验证分别显示。公共透明日志或远程 KMS 都不是本地工作流的默认依赖。

**不能照搬。** 不能把私人 Receipt 上传公共签名/日志服务作为默认操作；不能因为 Cosign 验签通过就把 `user-attested` 改成 `native-runtime`。本地 HMAC 持有者可以生成新 MAC，这不是组织背书；缺少当前撤销信息时也不能谎称已经核对最新信任状态。[^3]

**验收与回报。** P1，组织互操作较高。测试换 predicate、换 subject、未知 algorithm、旧签名、过期/撤销材料、离线失效、同密钥跨用途重放、删除证据后的 tombstone。加密密钥、Receipt 签名密钥、组织标准签名密钥、软件更新密钥应区分用途。

### M06 本地账本、快照、搜索与删除

**职责。** 保存权威快照和事务记录，支持查询、迁移、修复与删除传播。索引是可重建视图，不能比不可变 Receipt 更有权威。

**优先参考一：SQLite。** 官方测试说明公开讨论崩溃、I/O/OOM 故障和多套测试体系，适合借鉴“用故障模型验证持久性”的方法。WAL 文档也明确共享内存与同机使用条件，不能据此把活动数据库当作跨机同步文件。[^22][^23]

**优先参考二：rusqlite。** 它提供 Rust 对 SQLite 的绑定及 bundled 等功能选项，适合实现已有 `ctxpect-store` 接口，而不是新增一层远程数据库服务。具体版本必须与固定 Rust 工具链和目标 SQLite 编译选项一起核对。[^21]

**建议接入。** 条件 L；不是当前无条件执行项。结束 JSON ledger 偏离前先定义功能/规模触发条件，再遵循 ADR 0006。一次迁移只有一个权威写入引擎；可以做对照读取与校验，但不让 JSON 和 SQLite 长期双写、各自成为真值。Receipt 签名绑定已有序列化字节，不能绑定 SQL 任意查询结果。

**不能照搬。** 不因引入 SQLite 就添加向量库或图数据库。FTS 只索引允许保存的内容；全文检索功能不是保存私人 prompt 正文的授权。不要同步 `.db`、`-wal`、`-shm`，不要在多台设备同时打开云盘数据库，也不要把自带扩展加载打开为默认功能。

**验收与回报。** P1，迁移较高。模拟每个提交点崩溃、磁盘满、锁冲突、迁移中断、索引损坏、tombstone 与派生数据删除。明确物理残留边界：删行不等于操作系统/备份中所有字节消失；如需强删除，应依赖明确的加密与密钥生命周期合同，而非界面的一句“永久删除”。

### M07 差异、漂移与等价比较

**职责。** 输入两个 Receipt 和同一个 EquivalenceProfile；输出差异类别、不可比较原因和证据变化，不输出未经校准的“语义相似百分比”。

**优先参考一：Difftastic。** 它基于语法结构展示代码差异，适合改善 JSON/TOML 等原生配置的可读性；不能证明自然语言要求或不同 harness 的激活语义等价。[^24]

**优先参考二：CtxWise。** 当前正式名称为 CtxWise，README 明确说明曾用 CtxRay。其活跃 root-to-cwd lock、drift、测量边界和公开修正记录适合研究“少而明确的用户解释”，不应混同于其他名为 ctxray 的项目。[^25]

**建议接入。** D/I。比较分成文件字节、结构化 native 配置、规范推演、运行证据四层；语法 diff 只是解释层，权威差异仍由 Contexpect 核心产生。跨设备 keyed digest 不可直接比较，按 EquivalenceProfile 使用允许的身份与结构字段，必要时报告不可比较。

**不能照搬。** 不把 CtxWise 的 lock 作为第二个 Contexpect 基线，不把“AST 没变”或“文本同义”当作 native-equivalent；同字节不同 cwd/版本/策略可以产生不同预期。外部工具的成本估计也不能混入上下文一致性结论。

**验收与回报。** P1，中等。测试相同文本不同 scope、不同文本相同受限 native primitive、全局层失去授权、未知版本、摘要域不同、证据 stale、一个文件改名/复制/分叉。回报是将用户看到的漂移与真正需要处理的漂移分开。

### M08 Doctor、泄漏检测与修复建议

**职责。** 输入快照、原生配置和已有 Claim；输出确定性 finding、证据位置、覆盖与修复候选。规则命中和“允许自动修复”是两种不同决定。

**优先参考一：agnix。** 它把规则元数据、共享校验引擎、CLI、LSP/MCP/WASM 入口拆开，提供规则解释与 safe/unsafe 修复级别。这比再收集几百条文字建议更值得借鉴：同一诊断内核可以服务编辑器、CLI 和 CI。[^26]

**优先参考二：Gitleaks。** 可研究其凭据规则、allowlist、baseline 与脱敏输出。但当前上游已明确进入功能完成、后续以安全补丁为主的状态，维护者将新开发重点转向 Betterleaks。因此本报告把它选为**可替换、固定版本的离线检测参考**，不假设它会持续扩展全部新凭据类型。开源 CLI 与 GitHub Action 的使用条件也必须分别核对。[^27]

**后续方向，但不自动替换：Betterleaks。** 该项目公开提供更灵活的规则过滤以及 HTTP 凭据有效性验证等能力。对于 Contexpect，被动扫描不应拿发现的凭据去联网试用；所以它只进入后续比较清单，不进入本次默认组合。新增检测器必须在禁止联网、固定规则和显式数据范围的配置下重新验收，不能因为上游推荐迁移就自动替换。[^56]

**建议接入。** D/I 优先。先选与真实加载错误相关的高精度规则组，通过带工具版本、扫描范围、规则版本的报告导入，映射为带外部来源的 finding。Gitleaks 作为显式范围内的补充扫描器，不替代最靠近写入和导出的内置 secret gate。扫描配置由可信调用方显式指定，不能让被扫描仓库自己的配置、ignore 或 allow 注释静默关闭检测；上游确实提供这些自定义入口，必须与 Contexpect 的例外审批分开。既有 Contexpect suppression 仍绑定所审查的字节和有效期。[^27]

**不能照搬。** 不把 agnix 的 safe fix 直接等同于 Contexpect 批准；不把文字风格、提示词偏好变成阻断性事实。Gitleaks 的“未发现”只说明在声明规则/扫描范围内没有命中，不证明没有 secret；token 形状也不证明是真实有效凭据。不能将未脱敏 finding 先落盘，再事后删除。

**验收与回报。** P0/P1，中等。校验原生正常文件、无关同名 JSON、多语言、占位符、未知字段、例外失效、原文件在 preview 后改变。外部退出码必须显式映射：进程失败不能当作扫描无问题。优先增加“真实配置 → 确切错误 → 可预览修复”的场景，不追求规则数量。

### M09 资产包、来源、依赖与 SBOM

**职责。** 管理配置资产及其来源、版本、依赖、许可和安装状态，不判断这些资产已经进入模型上下文。

**优先参考一：Microsoft APM。** 其 manifest/lock、依赖解析、来源审计、安装策略和 SBOM 导出与资产环节贴近。官方明确区分安装期供应链策略和运行时 harness 行为，适合成为 Contexpect 的上游输入。[^28]

**优先参考二：Syft。** 它擅长为软件文件系统/制品生成 SPDX、CycloneDX 等软件物料清单，适合产品发布与第三方 adapter 产物，不应拿它代替上下文资产的加载证据。[^29]

**建议接入。** I/T。APM 优先只读导入 lock、audit、SBOM 和安装结果。每个 ContextItem 保存可解析的来源引用，不重复实现完整包管理。Syft 生成的是 Contexpect 二进制、前端、适配器等软件的物料清单；APM 记录的是上下文包关系，两类清单关联但不混为一个“已生效列表”。

**不能照搬。** APM install 不是纯函数：不能因为它是成熟项目，就让它无条件进入 Contexpect 的写入流程。尚未证明一个目标版本支持精确受限、可预览且可回滚的执行合同前，不开放自动安装。没有许可证或锁定来源的资产不复制；摘要一致也不保证内容无恶意。

**验收与回报。** P1，中等；安装闭环较高。测浮动引用、同版本不同摘要、未授权源、恶意归档路径、包携带 hooks、依赖被撤销和离线来源不可验证。回报是把包解析工作交给合适工具，同时让 Contexpect 专注 installed → discoverable → observed 之间的差异。

### M10 CanonicalIntent 与原生投影

**职责。** 输入已经验证的 CanonicalIntent、目标坐标和能力矩阵；输出原生配置候选、修改范围和 loss report，不直接写真实目录。

**优先参考一：Rulesync。** 它提供多个 agent 格式的规则、命令、Skills、MCP 等转换，既有 `.rulesync` 源管理，也有不写 `.rulesync` 的一次性 convert 模式。对 Contexpect，后者更接近可隔离的 renderer，而不是强制迁移所有源文件。[^30]

**优先参考二：agentsync。** 它将源状态、原生状态和已应用状态分开，并明确报告有损映射。其 README 也说明处于 beta、格式仍在稳定中，且 JSONC/TOML 注释、owned keys 覆盖和备份存在限制。它适合研究逆向捕获与损失分类，不应按零风险成熟引擎采用。[^31]

**建议接入。** Rulesync 作为首选 D/I 候选，agentsync 以 D 为主。先支持有限语义：scope、激活方式、原生字段和明确 loss。候选工具在 scratch 工作区处理受控输入，只返回 proposal；Contexpect 再解析候选文件并验证能力、权限与已有规则。

**不能照搬。** 不在 `.rulesync`、`~/.agentsync` 和 CanonicalIntent 之间建立三个同等权威源；不让任意转换器写用户 home。一次 convert 成功不证明 runtime 已生效，语义不能表达必须返回 unsupported/loss。闭源工具的 UI-only 设置不能因转换器没有接口就谎称已写入。

**验收与回报。** P1，较高。测试 round-trip 的字段损失、注释变化、冲突 key、未知字段保留、project/global scope 差异、shared file、policy 越权和转换器升级后的输出变化。对每个目标记录 renderer 版本与产物摘要，固定预览的结果不在 apply 时重新生成。

### M11 实际写入、事务、回滚与单一写入权

**职责。** 输入已批准且未过期的 proposal；输出事务记录、备份/回滚结果及修改后 Receipt。它是配置修改的统一协调边界，不再承担自然语言解释或模型建议。

**优先参考一：chezmoi。** 值得研究的是 source/target/destination 三态，以及把机器差异作为生成目标的显式条件。其模板和脚本能力是完整 dotfiles 产品的特性，不是被动上下文检查应默认继承的能力。[^32]

**优先参考二：Agentpack。** 它围绕声明式 agent 资产部署提供 preview、diff、snapshot、rollback 和受管文件边界；公开发布资产与工程目录支持把它作为事务体验参考。不要把这理解为它已通过 Contexpect 的安全合同。[^33]

**建议接入。** D 优先，保留现有 Contexpect executor。借鉴受管清单和三方比较，统一所有 mutation：UI、CLI、外部 fixes、标准升级、同步落地必须经过同一个门。权限、preview 的项目/文件摘要、authority 和消费状态在真正提交前重新验证。

**不能照搬。** 同一共享 JSON 即使不同工具声称只管理不同 key，也可能因整文件重写丢失另一方变更；需要文件级协调和 CAS，不能只有字段 owner。多文件改写不能宣称天然原子；单文件 rename 也不构成跨文件事务。含 secret 的 desired 或 before-image 在没有安全合同的情况下应拒绝，而不是写入普通 Git 备份。

**验收与回报。** P0/P1，较高。测试每次文件替换前后中断、跨项目复用 preview、两个并发 applies、备份篡改、后续用户编辑后的 rollback、删除新建文件但保留不相关文件。落盘成功只生成 transaction success；核对状态必须等 resolver/native evidence 后单独产生。

### M12 加密 bundle、多设备冲突与同步

**职责。** 传输不可变、授权范围内的 desired-state/Receipt bundle；接收后验证、解密、判定分叉，再进入统一投影事务。传输成功不是语义一致。

**优先参考一：rage。** 它是 age 格式的 Rust 实现。age 的成熟格式和互操作生态比自行设计加密协议更合适，但加密能力与签名、成员撤销、历史版本防回退分别是不同问题。[^34][^35]

**优先参考二：Syncthing。** 值得研究断线、文件版本、冲突副本和跨平台路径边界。其标准文件冲突处理会选出一个文件版本并保存另一份冲突副本；这不是 Contexpect 所需的策略/意图语义合并。[^36]

**建议接入。** rage 条件 L；也可在明确允许外部执行后使用经过固定版本的 age/rage 进程，但不是绕过 ADR 的便捷通道。Syncthing 默认只做 D，不作为产品必需服务。默认仍采用既定 folder/Git provider，承载不可变密文文件与版本化 manifest。

**不能照搬。** 不同步活动 SQLite、签名私钥、token 文件、home、原始会话和明文备份。不让传输层的“最新文件”决定标准 revision；recipient 移除无法召回对方已持有的旧明文或旧密钥。随机加密会让同一内容产生不同密文，不能以 ciphertext 相等判断意图等价。

**验收与回报。** P1/P2，较高。测试 A/B 离线分叉、同 revision 不同内容、重放、退回旧标准、移除接收者、截断密文、manifest 调包、密钥丢失和跨平台路径冲突。语义摘要按隐私合同在受保护内容内计算，外部 bundle 名称避免泄露低熵文件名/秘密摘要。

### M13 团队标准、策略、例外与可信授权

**职责。** 解释组织/团队/项目/角色/个人限制，判断某动作是否获得授权，管理例外生命周期；输出可执行性与证据状态。策略引擎不是身份提供商，也不能证明控制通道已经部署。

**优先参考一：Cedar。** 其 principal/action/resource/context 模型、schema 和授权接口贴近本地细粒度 mutation。必须注意官方的 skip-on-error 语义：策略求值出错会被跳过，其他 permit 仍可能导致 Allow。[^37][^38]

**优先参考二：OPA。** 适合已有 Rego 策略体系的组织接入，具备成熟的通用策略表达。Rego 的 undefined 与求值失败需要显式映射，不能当作 Contexpect 的“合规”。[^39]

**建议接入。** 二选一，不同时做核心判定。短期保留现有 `ctxpect-policy`，从 Cedar 借鉴请求模型；只有复杂性达到迁移必要性且完成 ADR 后，再以 Cedar 作为本地求值实现。OPA 保留为组织现有策略的有界输入，不能另建一套竞争的批准系统。

**不能照搬。** 直接执行 `if decision == Allow { apply }` 不够。先验证输入 schema、required evidence、策略摘要、新鲜度、例外批准者身份与 scope，再核对 diagnostics；关键策略错误或关键证据未知时，mutation fail-closed。浏览器自报管理员、包工具返回成功或本地 HMAC 正确都不能替代组织身份。

**验收与回报。** P1/P2，较高。关键 forbid 出错但 permit 命中、stale policy、跨设备/项目借用例外、自己批准自己、重复消费、撤销后缓存、无运行控制通道等都必须有负例。输出同时保留 policy decision 和 `detect-only/non-enforceable/enforceable`，避免一张绿色合规卡掩盖执行缺口。

### M14 daemon、本地 API、增量监测与调度

**职责。** 提供同一核心的受控入口、增量任务调度和资源限制，不创建另一套真值推导。watch event 只是需要重扫的提示，不是一次有效 snapshot。

**优先参考一：Axum。** 它的 typed extractor、路由和 Tower middleware 适合拆分庞大的 HTTP 分发实现。仓库 main 与已发布系列可能不同，不能把 main 示例直接当作兼容固定工具链的版本。[^40]

**优先参考二：notify。** 其跨平台文件监测后端及相关限制适合处理不同 OS 的事件来源。它不保证每一事件都完整、有序且足以重建原生上下文。[^41]

**建议接入。** D，现在保持已有服务；L 在 ADR 通过后。HTTP 层只做认证、解码、授权范围检查和调用 application service。watch 合并为“根目录/资产变脏”，由核心调度重扫、校验和提交；周期性 reconciliation 补偿漏事件，扫描取消保留上一有效快照并标记陈旧。

**不能照搬。** 文档里绑定 `0.0.0.0` 的示例不适合默认本地 API。127.0.0.1 不等于身份认证；必须结合 token 或可信本地通道、严格 Host/Origin、CSRF 边界和每次请求的项目授权。外部 adapter 进程不能共享 daemon 的全部环境、token 和文件能力。

**验收与回报。** P1，较高。请求重放、跨 origin、恶意 Host、超大 body、重复 request id、同时扫描/删除、事件风暴、目录移动、网络文件系统降级、退出清理、调度跨睡眠恢复都应测试。健康检查、资源指标与当前 Receipt 的新鲜度单独表达。

### M15 桌面壳、界面状态与交互

**职责。** 将同一 Receipt/诊断和授权流程清晰呈现给用户；UI 不自行读 home、不推断 Claim、不因按钮点击成功就显示“已生效”。

**优先参考一：Tauri 2。** 现有技术方向应继续保留。Capabilities 有助于缩小窗口与 WebView 的 IPC 权限，但它管的是 Tauri 通道，不自动保护任意 localhost HTTP API；重叠 capability 也可能扩展实际权限。[^42]

**优先参考二：TanStack Query。** 其 query key 和服务端状态缓存机制适合减少手写请求状态错误。Key 必须包含影响结果的输入；这与 Contexpect 的 coordinate/receipt/generation 绑定特别相关。[^43]

**建议接入。** Tauri 是继续完善现有方案；TanStack Query 属于前端候选，应评估替换已有请求层的净收益，不强制为了统一框架重写。优先做好 Doctor → evidence → care plan → apply → post-Receipt 的一条完整路径，再复用交互模式到其他页面。

**不能照搬。** 不通过 Tauri fs/sql/shell 插件新增一条直通写入路径。UI 的缓存新鲜不代表证据新鲜；重试普通查询和重试带副作用命令必须分开。不能对 Claim、合规或投影核对使用“乐观更新为通过”。短期请求失败也不能把历史快照清空成零问题。

**验收与回报。** P1，中等。快速切换项目/receipt、旧响应晚到、同 id 不同设备、取消和离线恢复、键盘焦点、中文/英文长文本、小屏遮挡和错误落点要覆盖。浏览器测试不能替代 Tauri WebView 测试。**当前 Tauri 官方文档已提供 macOS 可用的路线：WebdriverIO 的 `@wdio/tauri-service` 配合内嵌 WebDriver 插件；直接使用原生 `tauri-driver` 的限制不能被扩大成“macOS 无法自动化”。** 这是一条应验证的接入候选，不是本项目已通过的测试结果。[^53]

建议为桌面 workspace 建立独立测试构建：只在该构建启用必需测试插件，默认不启用额外后端命令访问或 mock 能力；发布构建必须排除 WebDriver 服务及测试权限。该变更应遵循桌面依赖与测试门禁审批，不借独立 workspace 绕过治理。

### M16 LLM Advisor 与历史建议

**职责。** 输入经用户确认的最小分析包；输出 `AdvisorSuggestion`/`CandidateRelation`，不写 Claim、policy、baseline 或真实文件。历史洞察也必须绑定观察窗口与相关证据。

**优先参考：Rig。** 当前仓库将 provider/backend 合同与 agent orchestration 分离，存在 provider cassette 离线重放测试及独立 live 测试说明，同时明确提示未来存在 breaking changes。最值得参考的是 provider 抽象与失败/重放合同，不是把整个 agent runtime 搬进产品。[^44]

**建议接入。** D 优先，确有多 provider 需求时条件 L/I。继续以自己的 AnalysisAdapter 作为边界；若采用对应版本的 Rig，只启用必要 provider 能力。请求、重试、取消、超时和结构化输出验证走同一个授权发送流程。

**不能照搬。** 不默认打开完整 agent/tool runtime，不加入向量数据库、长期记忆或自动工具执行。完成类型化解析只证明输出格式正确，不能提升建议为观察事实。provider 请求失败或流中断不能保存半份结果为正式分析；debug 日志、回放 cassette 和错误响应也不能携带未授权正文。

**验收与回报。** P2，中等。测试模型建议删除重要规则、伪造引用、要求读取外部 URL、泄露 prompt、结构化 JSON 不合法、重试导致重复收费、原始 evidence 删除后建议失效。结果预览显示发送范围和未知项；默认不发送、不执行、不将建议自动导出给团队。

### M17 Effect Lab、实验执行与统计判定

**职责。** 将冻结 ExperimentContract 转成受控运行，采集每个任务/arm 的实际结果，再按固定程序生成 effect result。执行框架、评分器与因果判定不能相互替代。

**优先参考一：Inspect AI。** 它提供模型评测的任务、执行、sandbox、日志和结果接口，适合实现实验 runner adapter。公开日志合同明确可能包含输入、输出、转录、媒体及原始 API 数据；这恰好要求接入时隔离日志与 Contexpect 账本。[^45][^46][^47]

**优先参考二：promptfoo。** 声明式用例、provider、assertion 和 CLI/CI 组合适合小规模配置回归，以及比较不同输入配置。它是另一个 runner/测试工具候选，不应与 Inspect 同时成为同一实验的隐式执行权威。[^48]

**建议接入。** 默认选 Inspect 作为后续受控实验 runner；promptfoo 保留为开发期轻量回归或备选。先导入已授权的结果文档，再独立批准真实运行。Contexpect 保留 preregistration、配对/样本量、冻结时间、混杂因素、非完成处理、重复检测和既有 v2 估计器；外部框架不替换这些合同。[^5]

**不能照搬。** 一次 eval 成功、平均分更高或 token 更少，都不能自动产生 `supported-beneficial`。不能用接口重试悄悄改变预注册样本，也不能只保留成功任务。Inspect 文档中 `--no-log-model-api` 仍描述保留错误调用，不能把它当作“不留任何请求正文”的保证；原始 runner 输出必须置于专用受限目录，经脱敏转换才进入核心。[^46]

**验收与回报。** P2，较高。预先明确付费调用、最大运行数、环境和输入数据范围。测试 cache 命中、任务污染、模型/工具版本变化、timeout/refusal、失败重试、grader 漂移和追加样本。尚未执行时明确 `executed:false`；日志留存策略不足时拒绝运行，而不先执行再清理。

### M18 产品自身可观测性与资源治理

**职责。** 回答 Contexpect 自己为什么慢、失败、重试或占用资源；不为目标 coding agent 的模型可见性制造证据。

**优先参考一：tracing。** 其 span/event 模型和 subscriber 分离适合统一 Rust 组件诊断；不要求所有核心逻辑迁移到 Tokio。自动记录函数参数的便利也带来风险，敏感类型必须显式跳过。[^49]

**优先参考二：OpenTelemetry Collector。** 标准 receivers/processors/exporters 管线适合组织已有观测系统的可选对接，不值得为个人本地检查默认再运行一个常驻服务。[^50]

**建议接入。** tracing 条件 L；Collector 作为 I/T 选项。为 inspect、解析、导入、apply、rollback 和导出建立相关 id、耗时和资源指标。Evidence ledger、授权审计日志、运行性能日志是三条不同的数据通道，各自有留存、权限与脱敏策略。

**不能照搬。** OTel 的采样、批处理丢弃、非阻塞日志溢出可以服务性能观测，但不能作用于影响 Claim 的必需证据。传输 retry 和 span 成功也不能冒充原生请求已派发。不要把 pathname、prompt、tool 参数、env 或大对象自动放入 span。

**验收与回报。** P1，中等。观测后端不可用时核心离线工作不受影响；缓冲区有界，关闭 telemetry 后无外联。故意丢弃性能日志不得导致 Receipt 丢失；必需证据写入失败必须有显式错误。回报是降低长期维护成本，而不是增加一套漂亮仪表盘。

### M19 一致性、属性与端到端测试

**职责。** 分别证明 schema、业务不变量、真实工具一致性和用户操作闭环；不能把生成语料数量当作产品覆盖率。

**优先参考一：proptest。** 属性测试、失败收缩和可重放反例适合路径、安全事务、Receipt canonicalization 与未知状态空间。根 workspace 的 dev-dependency 同样受 ADR 0006 限制，不能以“只测试”绕过。[^51][^5]

**优先参考二：Playwright。** Web-first assertions 和受控浏览器交互适合验证请求取消、旧响应、路由、键盘和响应式。它已经是项目允许的前端工具，应优先把高价值闭环变成可靠门禁，而不是换一套测试栈。[^52][^5]

**建议接入。** D/T。保留并分开统计四类语料：生成回归样例、上游原生样例、真实故障样例、外部人工复核样例。每个事故都沉淀为最小反例；失败 golden 的修改必须先解释语义变化，不能让实现和答案同方向改完即算修复。

Playwright 继续负责浏览器闭环；真实桌面 lane 按 M15 评估 Tauri 官方给出的内嵌 WebDriver 路线。二者不互相替代，也不要求把全部现有测试改写成 WebdriverIO。真实后端验收禁用命令 mock；专门的 UI 单测才使用明确标记的 mock。测试构建和发布构建还要做功能差分，确保测试服务不随安装包出货。

**不能照搬。** 单元属性测试不验证真实 OS；Chromium 不代表 WKWebView；同一个生成器导出的输入和答案不等于独立 oracle。上游测试数据也要核对许可证与隐私，不能复制别人的真实会话当作公共 fixture。

**验收与回报。** P0/P1，持续投入。关键性质包括：未授权路径始终不读；Unknown 不升级为 pass；重复 import 幂等；新编辑不被旧 rollback 覆盖；删除证据后派生失效；不同坐标不能共享缓存。完整产品仍要完成原定 OS、soak 和人工验收，而非用属性测试替代。[^4]

### M20 发布、安装、依赖与供应链门禁

**职责。** 将正确源码转成可核验的 CLI/桌面产物，管理依赖与升级；产品签名与 Context Receipt 签名不是同一个信任域。

**优先参考一：dist，仓库仍名 `cargo-dist`。** 它按 plan/build/host/publish/announce 组织多平台制品，生成机器可读 manifest 和 CI，适合 CLI 的可重复发布流程。不是所有桌面打包/公证细节都会因此自动完成。[^54]

**优先参考二：cargo-deny。** 它适合依赖许可、来源、advisory 和重复版本等门禁。离线 advisory 数据可能过期；“本地检查绿”必须同时携带数据库新鲜度和适用边界。[^55]

**建议接入。** T，不随产品常驻。CLI 发布优先研究 dist；桌面继续 Tauri 平台打包与签名。依赖引入后固定 Rust、Node、包管理器、features、构建目标、vendored 依赖和 SBOM；禁止取 latest 作为可复现依据。新工具进入开发/CI 也需要批准和离线供应链准备。

**不能照搬。** 不沿用其他项目要求用户关闭系统保护、去除 quarantine 的安装说明作为自己的默认发行路线。不要把“二进制有 Cosign 签名”当作“macOS 已公证”，也不要把“应用可更新”当作“Receipt 签名密钥可安全轮换”。CLI 和独立桌面 workspace 的锁定与门禁分别核对。

**验收与回报。** P1，中等。干净机器离线运行核心检查、安装/升级/卸载、错误架构、缺失证书、篡改更新、schema 升级后回退和保留用户数据都要验证。发布说明以实际通过的坐标与能力生成，不展示 18 个名字就宣称所有能力可用。

<a id="contracts"></a>

## 四、跨环节接口与依赖：怎样保持一个有机系统

### 4.1 必须由 Contexpect 自己持有的核心

以下内容不建议委托给任何完整外部产品：**解析坐标、六个证据面、Claim 合法组合、Unknown 原因、证据来源与覆盖、EquivalenceProfile、核对状态、例外绑定和修改后 Receipt。** 外部工具可以提供输入或计算部件，不能另立一个 `active:true` 事实体系。[^3]

已有实现的安全资产也不应因引入参考项目而丢失。当前投影器具有预览与项目绑定、单次消费、摘要并发检查、事务记录和回滚保护；导入器区分专用原生路径与通用输入；同步切片明确拒绝尚不可用的加密要求。借鉴的目的应是补齐这些边界，不是以“替换更快”为由回退它们。[^6][^7][^8]

### 4.2 推荐逻辑数据流

```text
原生配置/文件/显式授权的日志/外部资产报告
                  |
        受控采集与输入验证
                  |
     +------------+----------------+
     |                             |
版本化 native resolver        原生/外部 importer
     |                             |
Expected + 规则证据           Observed + 覆盖/来源
     +-------------+---------------+
                   |
        Contexpect IR / Claim 内核
                   |
     Receipt + 单一本地证据账本
                   |
          Diff / Doctor / Policy
                   |
      候选修复/标准升级/同步提案
                   |
    CanonicalIntent + 受限 renderer
                   |
       Proposal + loss + 冻结摘要
                   |
      授权 + 唯一 mutation 协调器
                   |
     文件级 CAS / journal / rollback
                   |
        重扫 + 所需原生证据补采
                   |
       新 Receipt / 未核验则 Unknown
```

UI、CLI、daemon 是上述流程的入口；Advisor 和 Effect runner 是旁路受控输入。它们不绕过核心。以上是建议的逻辑分层，不要求拆成微服务。

### 4.3 三个最容易混淆的范围必须显式区分

| 范围 | 含义 | 不能替代什么 |
| --- | --- | --- |
| 原生预期 | 在声明版本和坐标下，目标工具预计如何发现、合并、激活 | 不能被 Contexpect 自己的偏好预算或忽略列表直接改变 |
| 观察范围 | Contexpect 被允许读取和实际检查了哪些来源 | 没读到不等于原生 absent；无权限不等于配置错误 |
| 期望策略 | 用户/团队希望符合什么预算、资产和约束 | 策略违规不等于目标工具已经截断或排除内容 |

这是对既有模型的边界强化建议，而不是创建三套新的 Context IR。当前工具特定规则和产品级排除/预算必须在同一个解释结果中标明各自作用对象。

### 4.4 五个建议的扩展合同

这些是**适配器边界合同草案，不是声明仓库已经存在这些 schema，也不要求新增平行 Receipt/Intent 对象**。正式实施应先映射到现有 DTO 与 integration-contracts；只有确有缺字段时才进行版本化扩展。

| 边界 | 必须携带 | 核心如何处理 |
| --- | --- | --- |
| 外部证据报告 | producer/version、来源摘要、坐标、观测时间、字段来源、coverage、缺口/错误、隐私级别 | 逐字段映射；拒绝仅凭报告标题升级为 native-runtime |
| 转换 proposal | renderer/version、intent revision、目标文件和 owner、before/after digest、loss、未知字段、输入授权摘要 | 验证并冻结；不接受“apply 时再重新生成” |
| mutation 请求 | proposal digest、principal 的可信来源、项目/动作 scope、policy/exception digest、有效期、单次标识 | 当前策略重新核对；文件锁和 CAS；幂等且可审计 |
| 同步 bundle | bundle identity、父 revision、加密/签名算法和 key id、接收者集合、内容 manifest、schema | 先验证/解密，再判重放和分叉；不能直接覆盖 ledger |
| 实验结果 | frozen contract digest、runner/version、task/arm/run id、环境、失败状态、评分器版本、原始证据引用 | 校验重复、样本、时间、混杂、重试；再运行固定估计器 |

外部工具的 `success`、`verified`、`safe`、`exact` 都必须经映射说明。默认不继承这些词在外部产品中的等级含义。

### 4.5 写入权要比“唯一 projector”更精确

唯一 authority 应绑定到**规范化目标文件身份、目标坐标和受管范围**，而不仅是插件名称。一个物理文件被多个 harness 共用时，要显式登记共享关系。两个不同字段的 owner 仍需同一个文件级写入协调器。

推荐默认路径为：外部工具只生成 scratch 候选，Contexpect 是实际文件写入者。对于必须由 APM 等工具直接执行的资产操作，应使用另一种明确的 `delegated executor` 模式；该工具成为该目标唯一写入者，但外层授权、锁、备份边界与核对仍由统一 mutation 服务协调。

**两种模式不能在同一目标上同时开启。** 如果一个版本的外部工具无法暴露确定的修改集合、无法避免隐式下载/执行、无法冻结批准内容，就不具备 delegated executor 准入条件，只保留报告导入。

### 4.6 并发、崩溃与证据一致性

同步到来的新标准、用户手改文件、Doctor fix 和 adapter 自更新，都可能并发。推荐每个 proposal 绑定确切 before digest；最终写入前按规范化文件身份取得锁，重新核对 before digest、权限、策略和已批准 loss。

多文件提交要有持久 journal。可以使用准备、写入、提交、补偿等状态，但不能宣称所有平台上的任意多文件操作具有数据库式原子性。部分成功必须被记录；恢复过程中发现用户新编辑，转为人工协调，不强行“恢复到计划”。

事务完成与证据核对是两个状态轴。已成功写入但原生导出暂不可用时，应显示“已写入，运行时尚未验证”，不能把它算成 failed write，也不能直接绿色 verified。新 Receipt 追加产生，不回改旧 Receipt 的历史结论。

<a id="conflicts"></a>

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

<a id="combination"></a>

## 六、整体最合理的组合建议

### 6.1 四种组合路线的比较

| 路线 | 优点 | 主要代价 | 结论 |
| --- | --- | --- | --- |
| 所有入选项目整套运行 | 表面功能覆盖快 | 多套源/库/后台、写入冲突、日志泄漏、授权分裂，维护成本最高 | 不采用 |
| 所有能力继续零依赖自写 | 维持当前严格依赖合同 | 通用格式、安全文件、存储和 HTTP 长期维护负担较大 | 当前合法基线，但不应自动成为永久最优 |
| fork 一款同赛道产品扩展 | 复用现成 UI/命令 | 继承其真值、配置源、默认行为和技术边界，迁移成本不确定 | 不作为主路线 |
| 自有核对内核 + 少量成熟基础设施 + 有界外部适配 | 保留差异化，减少通用重复建设，逐项可退出 | 需要先建立接口和依赖治理，不能一步拼装 | **推荐目标；通过 ADR 后分阶段兑现** |

### 6.2 当前约束下可以采取的方案

保留现有 Rust workspace 和 JSON ledger，不新增 root 依赖，不安装额外常驻服务。先把本报告中的收益转成：上游规则来源、真实反例、跨坐标测试、输入/输出合同、UI 闭环，以及明确授权的外部报告导入。

优先研究顺序是：**原生 Codex/Gemini 源码与已有 Claude 合同 → agnix 的规则组织 → CtxWise 的 lock/解释 → APM 的来源报告 → Rulesync 的受限转换。** 首轮不要求自动启动这些工具，更不要求读取真实 home。已有用户授权之外的模型调用、扫描和软件安装均不由本报告默许。

这条路径不会假装“已用上成熟库”，但能立即改善解析正确性、测试独立性和接入边界。它也是后续依赖政策变更的评估依据，而不是简单推翻已接受 ADR。

### 6.3 条件满足后的推荐目标组合

以下不是一次性安装清单。“引入条件”未满足的组件继续以设计参考存在；领域完整性不能通过把 required 能力永久保留为未实现来满足。

| 层 | 最终推荐 | 采用方式与引入条件 | 不进入默认运行时的其他候选 |
| --- | --- | --- | --- |
| 核对与真值 | Contexpect 自有 core/IR/Claim/Receipt | 保留；统一所有入口与判断 | 任何外部 `active/health score` 真值模型 |
| 文件边界 | `ctxpect-fs` 封装 cap-std；遍历按需要采用 ignore | ADR 后 L；先通过平台负例 | 不为遍历引入完整 ripgrep CLI |
| schema | 自有 schema + 严格 jsonschema 实现 | ADR 后 L 或受控一致性测试；禁外部 ref | 不建立第二套对象定义 |
| 存储 | SQLite + rusqlite | 达到 ADR 退出条件后一次迁移；唯一 ledger | Scopeon/ContextSpy 私有数据库不得并成第二账本 |
| 原生规则与观察 | 自有 adapter，跟踪原生源码；按需读 Scopeon/其他导出 | D/I；每个字段和版本校验 | 不常驻整套 Scopeon 或 ContextSpy |
| Doctor | 自有 findings + agnix 高精度规则参考/报告；固定 Gitleaks 离线检测器 | I 先于 L；明确其维护状态；修复仍由统一 executor | 不自动调用外部 `--fix`；不自动迁往联网验证检测器 |
| 资产来源 | APM lock/audit/SBOM | 只读 I 默认；包写入准入单独验证 | 不复制 APM 包解析器 |
| 原生转换 | Rulesync 受限 scratch renderer 候选 | D/I；freeze version/output，验证 loss | agentsync、chezmoi、Agentpack 不作为并行写入服务 |
| 写入与回滚 | Contexpect mutation 服务 | 唯一协调器；借鉴三态、受管清单和事务测试 | 不引入第二套 backup/history 权威 |
| 加密同步 | rage/age 格式 + 现有 folder/Git provider | ADR/执行授权后；密文不可变 bundle | 不把 Syncthing 变成强制依赖 |
| 策略 | 当前 `ctxpect-policy`；必要时由 Cedar 实现受限求值 | D 先行，需全量语义差分与错误诊断门 | OPA 仅已有组织策略桥，不与 Cedar 竞争判定 |
| 本地 API | Axum + notify，保留共享 application service | ADR 后 L；loopback 认证、watch 重扫 | 不另建后台任务平台或消息集群 |
| 桌面/UI | Tauri 2 + 现有 React；按收益采用 TanStack Query | 继续现栈；前端状态层单独验收 | 不为效果重建 Electron 应用 |
| Advisor | 自有 AnalysisAdapter；必要时 Rig provider 层 | 最小能力、显式发送授权 | 不启用完整 agent、memory、vector store |
| Effect Lab | Inspect AI 外部 runner + 自有冻结合同/估计器 | 明确授权后运行；受限日志 | promptfoo 开发期/备选，不重复运行同一实验 |
| 运行诊断 | tracing；OTel 导出可选 | ADR 后；默认本地 | Collector 不随个人版本必装 |
| QA/发布 | Playwright、条件 proptest、Tauri 原生测试 lane、dist、cargo-deny | 只在开发/CI；固定版本；桌面测试构建与发行隔离 | 不把测试驱动或 mock 服务加入产品常驻进程 |

**最小运行形态仍然是 CLI 可独立工作、daemon 可选、桌面只是入口。** 不因为某个被参考项目使用 SQLite、HTTP server 或 dashboard，就将其整套运行时带进 Contexpect。[^2]

### 6.4 为什么 Rulesync 优先于 agentsync 作为转换候选

不是因为 agentsync 没价值，而是 Contexpect 已经需要自己的 authority、policy、事务和 Receipt；再导入一个以 `~/.agentsync` 为中心、带备份和反向同步的系统，会扩大重叠面。Rulesync 的一次性转换更接近可受控 renderer；即便如此，也必须验证指定版本的实际读写行为，不把 convert 当作安全沙箱。[^30][^31]

agentsync 最值得留下的是损失报告、源/实际/上次应用三态，以及清楚记录限制的方式。它的局部设计可以借鉴，完整运行时不进入默认组合。

### 6.5 为什么 APM 首先是读入口，而不是立即成为写入器

APM 解决包依赖与来源，Contexpect 解决上下文核对。先读 lock/audit/SBOM 就能得到明显收益，不必立刻承担包安装副作用。只有某个冻结版本能在受控目录、受控网络、受控脚本环境中提供确定的变更集合和可核验结果，才进一步评估 delegated executor。

这避免两种错误：一是重新写一个 APM；二是因为不想重复建设就默认授予 APM 全部文件权限。APM 官方对包策略和 harness 运行时边界的区分，也支持这种有界整合。[^28]

### 6.6 为什么不直接采用 Scopeon/ContextSpy 的数据库和 UI

Contexpect 的核心查询围绕坐标、Claim、证据覆盖和核对，而这两款产品分别偏向会话指标与请求 profiling。整合它们的数据库会产生 session id、留存、删除、隐私、原始请求和聚合口径冲突。[^16][^17]

推荐只借鉴采集模式、分区展示与少量受控导出。即便接口可读，也不能把它们的现成 UI 框进应用后宣称统一真值完成；每个页面应消费 Contexpect 自己的解释结果。

### 6.7 没有选择的“大底座”

本方案没有引入图数据库、分布式事件总线、CRDT、统一云账号、完整 agent framework 或常驻向量数据库。它们不是天然错误，而是在当前问题里没有清楚抵消其新增复杂度的收益。

同样不推荐通过统一代理“看清所有请求”作为默认主路线。专用 profiler 可以成为用户明确选择的诊断方法，但默认 MITM 会改变凭据、证书、正文留存、网络信任和产品定位，且已超出项目边界。[^2][^4]

<a id="delivery"></a>

## 七、接入顺序、产物与完成条件

### 7.1 分阶段实施，而不是削减最终范围

| 阶段 | 主要目标 | 参考项目参与方式 | 必须留下的工程产物 | 转入下一阶段的门槛 |
| --- | --- | --- | --- | --- |
| S0 真值与授权收口 | 观察范围、原生预期、用户策略分离；authority 去重 | 以 D 为主 | 已更新的规则出处、坐标测试、接口合同、依赖准入 ADR 候选 | 修改产品设置不伪造 native claim；全部输入边界明确 |
| S1 真实双锚点闭环 | 已有 Codex/Claude 从实际输入走到解释和证据 | 原生源码、agnix/CtxWise 测试思路 | 真实故障 corpus、原生输出、前后 Receipt、限制说明 | 外部开发者能复现；不以合成 oracle 替代真实结果 |
| S2 Doctor 与资产输入 | 高频 native 配置检查、APM 来源读取、会话实际导入 | agnix/Gitleaks/APM 只读 I；Scopeon 设计 | 版本固定适配器、脱敏报告、字段映射和幂等测试 | 缺少输入/运行失败与“无问题”被区分 |
| S3 基础设施有条件升级 | 解决已证实的存储、文件/API 维护问题 | cap-std/jsonschema/rusqlite/Axum 等按需 L | superseding ADR、vendor、许可清单、迁移/回退脚本 | 旧 Receipt 与现有功能不回退；干净环境离线验收 |
| S4 可控修改与同步 | 受限 native 投影、团队标准、bundle 分叉协调 | Rulesync scratch、age/rage；Cedar 视必要性 | 冻结 proposal、事务恢复记录、组织签名/撤销合同 | 每次写入可追溯；无 secret 旁路；post-Receipt 独立 |
| S5 Advisor/Effect 与完整交付 | 建议、历史、真实实验、全平台和完整体验 | Rig provider 可选、Inspect runner、Playwright/dist | 实验合同与原始证据边界、发布包、全矩阵验收 | 完成 WP-12 和独立 readback，未执行项不记通过 |

S1 的双锚点不是新的 MVP 完成定义。其作用是先证明最关键的闭环，再按冻结矩阵扩展；最终仍覆盖原全部必需功能。阶段之间允许并行准备测试和文档，但不得让 UI/同步抢先定义另一套 IR。

### 7.2 六个最值得先做的可复现场景

| 场景 | 需要证明的增量价值 | 所需环节 |
| --- | --- | --- |
| 同一仓库在根目录与嵌套 cwd 的规则不同 | 指出具体 scope/继承原因，而非只列文件 | M01/M02/M03/M05/M07 |
| 配置修改后原生会话仍沿用旧上下文 | 区分磁盘已变、运行时未刷新与未取得证据 | M02/M04/M05/M14 |
| 同一共享规则投到两种 native primitive | 展示可表达项、损失和各自证据，不强求文件相同 | M07/M10/M11 |
| 技术负责人发布标准，成员带个人规则 | 高层限制、例外与隐私披露正确 | M09/M10/M12/M13 |
| 修复 preview 后用户又改了同一文件 | 拒绝旧计划；不覆盖新编辑；rollback 保持安全 | M01/M11/M14 |
| 同一 context 调整前后重复实验 | 先验证配对和失败处理，再判断是否有证据支持改善 | M04/M05/M16/M17 |

### 7.3 不能只凭绿色测试放行的事项

每个适配器至少记录四个不同计数：**实际实现且通过、正确返回 Unknown、未实现、未执行。** 被动测试和原生真实运行也要分开。一个 external report 的 parser 单元测试绿，不表示该工具所有真实日志版本都能读。

发布门禁应覆盖以下成果，而不只统计测试数：能够证明来源版本、能解释 Unknown、能恢复部分写入、能阻止未授权路径/进程/网络、能完成关键用户路径、能维护旧 Receipt 兼容，以及能让未参与实现的人复现结果。完整人工和长时间验证按既有 WP-12 执行。[^4]

<a id="dependencies"></a>

## 八、依赖、许可与维护成本控制

### 8.1 上游引入登记表

每一个将被执行、链接或复制的上游项目都应进入一份仓内登记表。以下是建议字段，不是已经核验完毕的版本清单。

```yaml
integration_id: rulesync-renderer
purpose: native-config-proposal
mode: external-renderer
source_repository: dyoshikawa/rulesync
version_pin: REQUIRED_BEFORE_EXECUTION
source_commit: REQUIRED_BEFORE_EXECUTION
artifact_digest: REQUIRED_BEFORE_EXECUTION
license_review: REQUIRED_BEFORE_COPY_OR_DISTRIBUTION
upstream_schema_version: REQUIRED_BEFORE_EXECUTION
allowed_reads: explicit-input-snapshot
allowed_writes: scratch-only
network: denied-by-default
home_access: denied
secret_values: forbidden
output_contract: existing-integration-contract-plus-reviewed-extension
core_claim_writer: false
native_file_writer: false
upgrade_policy: explicit-review-and-differential-tests
fallback: report-indeterminate-do-not-apply
```

实际字段应与已有 integration-contracts/adapter manifest 合并，不在运行时再维护一份含义相同的独立配置。版本锁定不仅包括主工具，还要包含 Node/Python/Rust 运行时、转译/解析依赖、features、模型 provider 合同及构建来源。

### 8.2 许可证与产品边界

本报告不把“GitHub 上可见”视作可自由复制。schema 测试、规则库、原生代码、二进制、浏览器包和数据语料都可能有不同的许可范围。进入发布包前核对具体文件、传递依赖、NOTICE、修改记录和再分发条件。

尤其要分清 Gitleaks CLI 与 Action、外部软件进程与直接复制源码、标准格式与参考实现，以及测试数据与工具代码。Syncthing 即使只是设计参考，也不意味着将来复制实现时无需重新审查其许可证；本报告默认不复制其同步引擎。[^27][^36]

复制少量经过审查的测试思路可以降低耦合；长期 vendor 一整个快速变化的同赛道项目则可能转移大量维护负担。对于安全边界，更重要的是“谁维护、怎样升级、失败如何被发现”，不是把依赖数量写成某个越低越好的指标。

### 8.3 升级与退出

建议对每个 adapter 保留固定旧版本的一套验证样本和真实来源摘要。升级先在隔离环境生成候选报告，与旧行为比较，再由明确规则决定哪些差异是原生新行为、哪些是回归、哪些只是格式变化。

外部依赖损坏、停更或无法获取时，核心应仍能读取历史 Receipt、导出和执行已支持的纯本地检查。不能因为一个可选 profiler 失效就拒绝启动整个应用，也不能为了可用性把失败悄悄转换成“无问题”。

**退出策略要在引入时设计。** 若一个 renderer 输出无法用普通 native 文件和公开 loss 合同解释，或者一个 provider 把重要状态锁在私有数据库中，就不应进入核心必需链路。


<a id="review"></a>

## 九、整体复审后的最终取舍

### 9.1 从“单环节优秀”到“整体合适”的调整

逐环节选择之后，仍需重新检查全局所有权和成本。以下取舍已经回写前文；本章不是与第六章并行的另一套架构。

| 单环节看起来合理的选择 | 最终组合中的取舍 | 判断依据与后果 |
| --- | --- | --- |
| agentsync 的双向投影和备份 | 保留设计参考，不默认整套接入 | 它与已有 intent、备份、反向同步和写入权重叠；其明文备份边界也与项目要求冲突。[^31] |
| Syncthing 的成熟多设备同步 | 只借鉴冲突和异常场景；不默认加入运行时 | 文件传输不能替代标准版本仲裁，更不能复制活动 SQLite 账本。[^36][^23] |
| ContextSpy 的请求级可见性 | 仅明确授权的 profiler / 导入来源 | 默认代理和正文日志不符合 Contexpect 的默认边界。[^17][^2] |
| APM 的安装与治理能力 | 先读 lock、来源、audit/SBOM；安装为条件接入 | 复用包域，不继承未经验证的写入副作用。[^28] |
| Rulesync 的多格式生成 | 仅作为隔离候选生成器；未验读写范围不得进入真实工作区 | 转换成功不是批准、无损或生效证明。[^30] |
| Cedar 的授权表达能力 | 先借鉴或保留现有求值器；有真实复杂度才替换底层 | 避免过早引入第二套策略语言；接入时必须处理错误与证据不完整。[^38] |
| SQLite、cap-std、jsonschema 等基础库 | 作为明确的目标候选，不在当前合同下直接加入 | 需要按 ADR 0006 的退出条件变更；不是以研究报告替代工程决策。[^5] |
| Rig 与 Inspect AI | 前者限制在建议请求，后者限制在实验执行 | 两者都不获得 Claim、策略和真实配置的自主写入权。[^44][^45] |
| Gitleaks 的规则和扫描接口 | 保留固定离线基线，记录其维护状态并允许未来替换 | 不把安全补丁期误写成持续功能扩展，也不自动迁入带 HTTP 验证的后续工具。[^27][^56] |
| Tauri 的跨平台测试 | 补入新的 macOS 内嵌 WebDriver 候选路线 | 纠正“没有可用自动化通道”的过时泛化；新测试能力须与发行能力严格隔离。[^53] |

**最终保留的是各项目的优势，不是它们全部的默认行为。** 一项外部能力只有在输入、输出、身份、权限、持久化和失败语义都能接入现有合同后，才属于产品组合。

### 9.2 复审后的硬边界

第一，只有 Contexpect 可以综合证据形成最终 Claim 与核对状态。外部校验器的 finding、格式转换的成功、签名通过、包已安装和实验已运行，都只是有不同强度和范围的输入，不是直接的最终结论。

第二，逻辑上只有一个修改协调入口；物理上同一文件在同一事务内只能有一个受控写入者。批准、冻结预览、路径能力、secret gate、写前重验、恢复与 post-Receipt 是不可跳过的链路。委托外部工具不是委托最终授权。

第三，只有一个权威本地账本。外部工具可以保有自己的运行日志，但不能偷偷成为第二份可相互覆盖的 Receipt、策略、标准或事务数据库。导入和删除都必须有清晰的来源与失效传播。

第四，只有一套范围和版本语义。原生预期、已授权观察、用户希望的状态分别表达；来自当前上游 main 的新行为不得无版本验证地套进旧坐标。文本、AST、hash、传输、签名和效果也必须分开表述。

第五，默认仍然本地、被动、无模型调用。需要执行工具、联网、读取更大范围、保存正文或运行付费实验的步骤，必须是独立、可预览、可拒绝的能力，不能因接入一个库而自动启用。

### 9.3 尚需实施验证的准入问题

这是架构级组合建议，不是已经完成的集成验证。以下问题在接入前必须得到实际证据：

| 待验证事项 | 必须取得的证据 | 没有证据时的行为 |
| --- | --- | --- |
| 选定版本与当前工具链兼容 | 确切 release/commit、features、MSRV、平台构建及离线材料 | 保持设计参考；不把最新 README 当成可编译证明 |
| Rulesync/APM 的真实读写和执行范围 | 对固定版本做文件、进程、网络观测；异常与取消后的产物检查 | 不启动，或仅接受显式提供的只读报告 |
| 原生日志的版本和完整性 | 来自对应工具版本的最小合法/损坏样本及字段语义映射 | 拒绝未知格式或保持 partial/Unknown |
| 旧 Receipt 在库或存储迁移后仍有效 | 旧字节、旧签名和原始算法的回归验证；可恢复迁移演练 | 不替换活动存储，不隐式重签 |
| Tauri 测试插件的适用范围 | macOS/Windows/Linux 真机测试和发行包排除检查 | 不宣称已完成原生桌面验收 |
| 检测器没有旁路外联或自授权 suppression | 禁网环境、可信规则、恶意仓内配置和失败日志 canary 测试 | 禁止将检测结果用于放宽写入边界 |
| Effect Lab 的外部 runner 符合冻结合同 | 有授权的真实样本、失败/重试/缓存记录、日志隐私验证 | 保留未执行/不可判定，不以合成数据宣称产品收益 |
| 组织签名与例外确有可信身份 | 外部信任根、身份映射、到期/撤销及离线旧状态验证 | 仅表示本地连续性或 detect-only，不冒充组织证明 |

版本准入失败应更换该实现或调整接入方式，而不是把 required 能力永久抹掉。某工具退出也不应破坏公开 DTO、历史可读性和核心纯本地检查。

### 9.4 最终结论

**最合理的组合，是“自有证据核对内核 + 可替换的原生适配器 + 少量成熟基础库 + 按需调用的专用工具”。**

优先让真实输入、原生行为和独立反例进入 Contexpect，再增加管理自动化；优先减少重复写入者与事实来源，再增加功能面。这样既能借鉴优秀工程项目，又不会把 Contexpect 变成若干工具默认行为相互冲突的包装层。

产品应最终能稳定回答：**在这个坐标下，为什么预期与实际不同；有什么证据；怎样安全修复；修复后究竟验证到了哪一步。** 这是应该继续自主维护的核心，也是整个借鉴组合的评判标准。

<a id="sources"></a>

## 十、参考来源与阅读入口

以下均为原项目仓库、官方技术文档或标准正文。核验日期为 2026-09-11。Contexpect 文件和两份原生源码入口使用已取得的提交坐标；其他链接多数是会变化的文档或默认分支，应在实现准入时重新固定版本、摘要和许可证。这里的 URL 是阅读证据，不是已经批准的依赖锁。

来源条目同时说明最值得阅读的部分。相同来源在正文重复引用，不代表完成了额外的独立验证；维护者声明、标准保证和本报告的实现建议不能相互替代。

[^1]: [Contexpect，基线提交](https://github.com/Octo-o-o-o/Contexpect/commit/257b7d4d82c0865a94b45d9b3fddcb3830658b32)。核定仓库版本与工具链固定记录。

[^2]: [Contexpect，架构总览](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/overview.md)。组件职责、单一真值、入口、隐私与非目标。

[^3]: [Contexpect，数据与真值模型](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/data-and-truth-model.md)。六个 facet、Claim、Unknown、Receipt、等价与签名边界。

[^4]: [Contexpect，实施计划](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/process/implementation-plan.md)。F-01–F-18 / WP-01–WP-12 映射、冻结范围与完成条件。

[^5]: [Contexpect，ADR 0006](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/adr/0006-third-party-dependency-policy-and-estimator.md)。零第三方依赖、存储偏离退出条件、v2 估计器和 Playwright 决策。

[^6]: [Contexpect，ctxpect-projection](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-projection/src/lib.rs)。预览绑定、事务消费、控制路径、并发摘要、回滚与 secret gate。

[^7]: [Contexpect，ctxpect-importer](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-importer/src/lib.rs)。原生 DSH 路径与通用 events 导入之间的实现边界。

[^8]: [Contexpect，ctxpect-sync](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/crates/ctxpect-sync/src/lib.rs)。加密未就绪时的拒绝行为及 folder bundle 语义。

[^10]: [ripgrep，ignore crate](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore)。遍历、过滤与 ignore 规则；不是上下文原生加载规则。

[^11]: [Bytecode Alliance，cap-std](https://github.com/bytecodealliance/cap-std)。目录能力和受限文件系统 API；配合平台安全测试。

[^12]: [OpenAI Codex，AGENTS.md discovery 源码](https://github.com/openai/codex/blob/fc948f8c473e5d11e780ffcf1fd7f812a2020932/codex-rs/core/src/agents_md.rs)。原生根标记、目录链、字节预算和环境/权限分支；该提交不是 Contexpect 旧锚点版本的替代品。

[^13]: [Gemini CLI，MemoryContextManager 源码](https://github.com/google-gemini/gemini-cli/blob/ed2ac40df67a319bf348bd7e3d10494696b31b38/packages/core/src/context/memoryContextManager.ts)。全局/扩展/项目分层、文件身份去重；关联读取同提交 utils/memoryDiscovery.ts。

[^14]: [jsonschema，Rust 实现](https://github.com/Stranger6667/jsonschema)。schema 验证、feature 表和外部引用解析；接入必须关闭不受控解析。

[^15]: [JSON Schema Test Suite](https://github.com/json-schema-org/JSON-Schema-Test-Suite)。按 draft 划分的独立一致性测试。

[^16]: [Scopeon，架构说明](https://github.com/sorunokoe/Scopeon/blob/main/docs/architecture.md)。collector、parse_incremental、纯指标层、SQLite 与文件 offset；产品入口 https://github.com/sorunokoe/Scopeon 。

[^17]: [ContextSpy](https://github.com/RimantasZ/contextspy)。请求 profiling、捕获与本地存储边界。

[^18]: [in-toto Attestation，Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md)。subject、predicateType 和声明包络；不等于运行事实自动可信。

[^19]: [Sigstore，Cosign verification](https://docs.sigstore.dev/cosign/verifying/verify/)。签名、bundle、身份和验证材料；离线使用须准备可信材料。

[^20]: [IETF，RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785.html)。JSON 规范化标准；不能静默替换既有签名序列化规则。

[^21]: [rusqlite](https://github.com/rusqlite/rusqlite)。SQLite Rust 绑定、features、bundled 构建与工具链要求。

[^22]: [SQLite，How SQLite Is Tested](https://www.sqlite.org/testing.html)。故障注入、I/O/OOM、模糊与恢复测试思路；并非所有测试套件都公开可复制。

[^23]: [SQLite，Write-Ahead Logging](https://www.sqlite.org/wal.html)。WAL、共享内存、并发与网络文件系统边界。

[^24]: [Difftastic](https://github.com/Wilfred/difftastic)。语法结构差异显示；不是业务语义等价证明。

[^25]: [CtxWise，README](https://github.com/FramY2/ctxwise/blob/main/README.md)。活跃根到 cwd 链、lock/drift、透明估计及独立发现问题后的回归说明；勿与无关 ctxray 项目混淆。

[^26]: [agnix](https://github.com/agent-sh/agnix)。规则元数据、共享引擎、解释、修复等级及多入口；贡献说明 https://github.com/agent-sh/agnix/blob/main/CONTRIBUTING.md 。

[^27]: [Gitleaks，CLI 与维护说明](https://github.com/gitleaks/gitleaks)。检测、配置优先级、忽略项、脱敏与安全补丁维护状态；另读 Action 使用条件 https://github.com/gitleaks/gitleaks-action ，不与 CLI 混同。

[^28]: [Microsoft APM](https://github.com/microsoft/apm)。包 manifest/lock、来源、策略、audit/drift 与 SBOM；官方文档 https://microsoft.github.io/apm/ 。

[^29]: [Anchore，Syft](https://github.com/anchore/syft)。制品和软件依赖的 SBOM 生成，补足软件供应链而非上下文事实。

[^30]: [Rulesync](https://github.com/dyoshikawa/rulesync)。多工具 native 转换、导入与生成；官方文档 https://rulesync.dyoshikawa.com/ 。

[^31]: [agentsync，README 与 Known Limits](https://github.com/spxrogers/agentsync/blob/main/README.md)。投影与对账思路；重点检查 JSONC、owned keys、明文备份、Windows 权限及 rollback 范围。

[^32]: [chezmoi，Concepts](https://www.chezmoi.io/reference/concepts/)。source、target、destination state 与应用模型；不直接采用模板执行权限。

[^33]: [Agentpack](https://github.com/liqiongyu/agentpack)。受管资产、preview/diff、snapshot/rollback；发布记录 https://github.com/liqiongyu/agentpack/releases 。

[^34]: [age，项目与格式入口](https://github.com/FiloSottile/age)。文件加密格式、收件人与互操作边界。

[^35]: [rage，Rust age 实现](https://github.com/str4d/rage)。既有加密格式的 Rust 实现，不自行设计密码协议。

[^36]: [Syncthing，Synchronization](https://docs.syncthing.net/users/syncing.html)。冲突副本、文件变化与大小写等行为；项目 https://github.com/syncthing/syncthing 。

[^37]: [Cedar policy language](https://github.com/cedar-policy/cedar)。策略表达、校验和授权求值实现。

[^38]: [Cedar，Authorization semantics](https://docs.cedarpolicy.com/auth/authorization.html)。default deny、forbid 优先、求值错误跳过与 diagnostics；与 Contexpect Unknown 的映射必须另行处理。

[^39]: [Open Policy Agent，Policy Language](https://www.openpolicyagent.org/docs/policy-language)。Rego、undefined 与策略求值；仅作备选或既有组织系统接口。

[^40]: [Axum](https://github.com/tokio-rs/axum)。extractor、router、中间件组合与 released/main 区别。

[^41]: [notify](https://github.com/notify-rs/notify)。文件监控后端、平台差异与轮询降级；事件不能替代重扫。

[^42]: [Tauri 2，Capabilities](https://v2.tauri.app/security/capabilities/)。窗口/WebView 的 IPC 权限及合并边界；不等于 HTTP API 授权。

[^43]: [TanStack Query，Query Keys](https://tanstack.com/query/latest/docs/framework/react/guides/query-keys)。查询身份与变量依赖，对应 coordinate/receipt 缓存键。

[^44]: [Rig](https://github.com/0xPlaygrounds/rig)。provider/backend 与 agent runtime 分层、测试和兼容性提醒。

[^45]: [UK AI Security Institute，Inspect AI](https://github.com/UKGovernmentBEIS/inspect_ai)。任务、评测、solver/scorer 与外部 runner 设计。

[^46]: [Inspect AI，Evaluation Logs](https://inspect.aisi.org.uk/eval-logs.html)。输入/输出/转录和 API 错误数据的日志行为，重点用于隐私准入。

[^47]: [Inspect AI，Sandboxing](https://inspect.aisi.org.uk/sandboxing.html)。实验执行环境；不代替自有文件、网络与数据留存授权。

[^48]: [promptfoo](https://github.com/promptfoo/promptfoo)。声明式评测用例、provider、assertion 和 CI 入口。

[^49]: [tracing](https://github.com/tokio-rs/tracing)。span/event、subscriber 和 Rust 诊断分层。

[^50]: [OpenTelemetry，Collector](https://opentelemetry.io/docs/collector/)。receiver/processor/exporter 管线；与证据账本分离。

[^51]: [proptest](https://github.com/proptest-rs/proptest)。属性测试、收缩与失败重放；开发依赖仍受项目 ADR 管理。

[^52]: [Playwright，Assertions](https://playwright.dev/docs/test-assertions)。Web-first 自动等待/重试断言和浏览器闭环。

[^53]: [Tauri 2，WebDriver](https://v2.tauri.app/develop/tests/webdriver/)。当前官方内嵌 WebDriver / WebdriverIO 路线覆盖 macOS；与直接 tauri-driver 及浏览器 mock 路线区分。页面标示更新于 2026-06-29。

[^54]: [dist / cargo-dist](https://github.com/axodotdev/cargo-dist)。跨平台构建、制品 manifest 与发布阶段；CLI 发布不替代桌面平台签名。

[^55]: [cargo-deny](https://embarkstudios.github.io/cargo-deny/)。依赖 advisories、bans、licenses 和 sources 门禁。

[^56]: [Betterleaks](https://github.com/betterleaks/betterleaks)。Gitleaks 后续开发方向；HTTP 凭据验证和远程数据源解释了为什么不自动纳入被动扫描。

