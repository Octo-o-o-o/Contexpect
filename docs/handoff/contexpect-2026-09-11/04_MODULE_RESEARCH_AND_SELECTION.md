# 20 个环节：完整研究与采用方式

**来源：**下面保留前置研究报告的完整环节总表及 M01–M20 调研正文，并保留其引用。它不是新增依赖安装清单，也不是本轮已完成的组件审计。

**实施读取规则：**先读 [最终纠偏](02_FINAL_DECISIONS_AND_CORRECTIONS.md) 和 [整合冲突](05_INTEGRATION_BOUNDARIES_AND_CONFLICTS.md)。最终建议关于动作级 Unknown、文件并发保证、纠正错误 golden 和受控依赖的解释，优先于早期文中可能被读得过强的表述。下列优先级说明局部价值与风险，不要求同时重构20个环节。

**资料时效：**沿用2026-09-11研究快照；未再次确认所有上游维护状态、默认feature或平台支持。正式执行前锁定具体版本并验证。

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


## 本文依据

[^4]: [Contexpect，实施计划](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/process/implementation-plan.md)。F-01–F-18 / WP-01–WP-12 映射、冻结范围与完成条件。

[^10]: [ripgrep，ignore crate](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore)。遍历、过滤与 ignore 规则；不是上下文原生加载规则。

[^11]: [Bytecode Alliance，cap-std](https://github.com/bytecodealliance/cap-std)。目录能力和受限文件系统 API；配合平台安全测试。

[^12]: [OpenAI Codex，AGENTS.md discovery 源码](https://github.com/openai/codex/blob/fc948f8c473e5d11e780ffcf1fd7f812a2020932/codex-rs/core/src/agents_md.rs)。原生根标记、目录链、字节预算和环境/权限分支；该提交不是 Contexpect 旧锚点版本的替代品。

[^13]: [Gemini CLI，MemoryContextManager 源码](https://github.com/google-gemini/gemini-cli/blob/ed2ac40df67a319bf348bd7e3d10494696b31b38/packages/core/src/context/memoryContextManager.ts)。全局/扩展/项目分层、文件身份去重；关联读取同提交 utils/memoryDiscovery.ts。

[^14]: [jsonschema，Rust 实现](https://github.com/Stranger6667/jsonschema)。schema 验证、feature 表和外部引用解析；接入必须关闭不受控解析。

[^15]: [JSON Schema Test Suite](https://github.com/json-schema-org/JSON-Schema-Test-Suite)。按 draft 划分的独立一致性测试。

[^20]: [IETF，RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785.html)。JSON 规范化标准；不能静默替换既有签名序列化规则。

[^16]: [Scopeon，架构说明](https://github.com/sorunokoe/Scopeon/blob/main/docs/architecture.md)。collector、parse_incremental、纯指标层、SQLite 与文件 offset；产品入口 https://github.com/sorunokoe/Scopeon 。

[^17]: [ContextSpy](https://github.com/RimantasZ/contextspy)。请求 profiling、捕获与本地存储边界。

[^18]: [in-toto Attestation，Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md)。subject、predicateType 和声明包络；不等于运行事实自动可信。

[^19]: [Sigstore，Cosign verification](https://docs.sigstore.dev/cosign/verifying/verify/)。签名、bundle、身份和验证材料；离线使用须准备可信材料。

[^3]: [Contexpect，数据与真值模型](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/data-and-truth-model.md)。六个 facet、Claim、Unknown、Receipt、等价与签名边界。

[^22]: [SQLite，How SQLite Is Tested](https://www.sqlite.org/testing.html)。故障注入、I/O/OOM、模糊与恢复测试思路；并非所有测试套件都公开可复制。

[^23]: [SQLite，Write-Ahead Logging](https://www.sqlite.org/wal.html)。WAL、共享内存、并发与网络文件系统边界。

[^21]: [rusqlite](https://github.com/rusqlite/rusqlite)。SQLite Rust 绑定、features、bundled 构建与工具链要求。

[^24]: [Difftastic](https://github.com/Wilfred/difftastic)。语法结构差异显示；不是业务语义等价证明。

[^25]: [CtxWise，README](https://github.com/FramY2/ctxwise/blob/main/README.md)。活跃根到 cwd 链、lock/drift、透明估计及独立发现问题后的回归说明；勿与无关 ctxray 项目混淆。

[^26]: [agnix](https://github.com/agent-sh/agnix)。规则元数据、共享引擎、解释、修复等级及多入口；贡献说明 https://github.com/agent-sh/agnix/blob/main/CONTRIBUTING.md 。

[^27]: [Gitleaks，CLI 与维护说明](https://github.com/gitleaks/gitleaks)。检测、配置优先级、忽略项、脱敏与安全补丁维护状态；另读 Action 使用条件 https://github.com/gitleaks/gitleaks-action ，不与 CLI 混同。

[^56]: [Betterleaks](https://github.com/betterleaks/betterleaks)。Gitleaks 后续开发方向；HTTP 凭据验证和远程数据源解释了为什么不自动纳入被动扫描。

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

[^53]: [Tauri 2，WebDriver](https://v2.tauri.app/develop/tests/webdriver/)。当前官方内嵌 WebDriver / WebdriverIO 路线覆盖 macOS；与直接 tauri-driver 及浏览器 mock 路线区分。页面标示更新于 2026-06-29。

[^44]: [Rig](https://github.com/0xPlaygrounds/rig)。provider/backend 与 agent runtime 分层、测试和兼容性提醒。

[^45]: [UK AI Security Institute，Inspect AI](https://github.com/UKGovernmentBEIS/inspect_ai)。任务、评测、solver/scorer 与外部 runner 设计。

[^46]: [Inspect AI，Evaluation Logs](https://inspect.aisi.org.uk/eval-logs.html)。输入/输出/转录和 API 错误数据的日志行为，重点用于隐私准入。

[^47]: [Inspect AI，Sandboxing](https://inspect.aisi.org.uk/sandboxing.html)。实验执行环境；不代替自有文件、网络与数据留存授权。

[^48]: [promptfoo](https://github.com/promptfoo/promptfoo)。声明式评测用例、provider、assertion 和 CI 入口。

[^5]: [Contexpect，ADR 0006](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/adr/0006-third-party-dependency-policy-and-estimator.md)。零第三方依赖、存储偏离退出条件、v2 估计器和 Playwright 决策。

[^49]: [tracing](https://github.com/tokio-rs/tracing)。span/event、subscriber 和 Rust 诊断分层。

[^50]: [OpenTelemetry，Collector](https://opentelemetry.io/docs/collector/)。receiver/processor/exporter 管线；与证据账本分离。

[^51]: [proptest](https://github.com/proptest-rs/proptest)。属性测试、收缩与失败重放；开发依赖仍受项目 ADR 管理。

[^52]: [Playwright，Assertions](https://playwright.dev/docs/test-assertions)。Web-first 自动等待/重试断言和浏览器闭环。

[^54]: [dist / cargo-dist](https://github.com/axodotdev/cargo-dist)。跨平台构建、制品 manifest 与发布阶段；CLI 发布不替代桌面平台签名。

[^55]: [cargo-deny](https://embarkstudios.github.io/cargo-deny/)。依赖 advisories、bans、licenses 和 sources 门禁。
