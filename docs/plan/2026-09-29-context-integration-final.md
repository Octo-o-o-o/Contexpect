# Contexpect × OctoWorkFlow：完整窄集成最终方案

> 状态：供新会话采纳执行的最终工程方案；本会话只交付方案、1 个 subagent 的交叉 review 和实施 prompt，尚未施工。
> 本文是该主题的当前工程 canonical。[前轮研究](2026-09-29-product-direction-and-octoworkflow.md) 保留为决策背景；其 §5 逐阶段再次审批、5 人日样本止损不再作为本方案的施工流程。

## 1. 最终决定与完成定义

采用“产品入口整合、证据引擎独立”的方向：Contexpect 保留仓库、Rust 内核、CLI、Receipt 与旧数据；OctoWorkFlow 提供可选的项目指令核对入口与既有任务旁的证据引用。暂不物理合仓，不重写内核，不迁移用户数据，不扩张第二套完整桌面产品。

用户希望新会话完整实施、不中途反复确认。因此新 prompt 一旦被用户粘贴或明确采纳，即预授权本文 **E01–E12 的全部工程工作**，阶段间无需再次批准。业务需求和长期收益另做观察，缺样本不阻止完成工程，也不能反过来把工程通过当作商业价值成立。

“完整”指本窄集成：可独立运行的真实 ctxpect → 正式证据与版本化接口 → OWF 可选控制入口 → 真实 UI 与失效/恢复 → 双仓门禁 → 本地候选和完整回执。不是 F-01–F-18/WP-01–WP-12 全产品补完、全 18 family、跨 OS 发行或生产发布。

本轮选择一个当前实际 Codex CLI/macOS arm64 坐标完成静态 instructions 支持和专项验证；Claude/Cursor 等其他坐标保持能力声明和 Unknown，不通过通配版本冒充支持。原生 prompt 诊断优先接入，成功只证明该次声明 profile 的诊断构造，不证明另一真实任务消费或效果。

## 2. 已核实基线与实现位置

| 项目 | 2026-09-29 实查 | 重要现状 |
| --- | --- | --- |
| ContextView | main，`2ae179f551b43ac3c05234e5294b9b4af9059fed`，dirty | mutation 分轴候选和研究文档未提交；不能仅用 HEAD 复制候选 |
| OctoWorkFlow | main，`63ccb39acff36367a7a7bd6b06940587e567d17f`，dirty | Pro 传输/skill 等既有变更；本任务不覆盖、不顺手修复 |
| 宿主工具 | Rust 1.94.0，Python 3.12.14，Node 22.23.2，pnpm 11.20.0 | 新会话先复验实际路径和版本，不升级全局环境 |
| 实装诊断坐标 | Codex CLI 0.153.3；macOS 27.2 build 26B5091g；arm64 | `codex debug prompt-input --help` 已确认存在；实际 capture 本轮未运行 |
| 其他实装工具 | Claude Code 2.1.280 | 只记录版本，不因存在就扩第一版支持范围 |

源码实查：`ctxpect-cli/src/lib.rs:208–249` 的 inspect 配合 store 可由引擎生成正式 Receipt，并返回 formal_receipt_id；不能把普通 dev-inspect JSON 改名为 Receipt。`dispatch.rs:625–679` 负责项目坐标、迁移与持久化。`native_oracle.rs:94–114` 仍固定旧 oracle；`221–234` 明示隔离空 profile、本地 HMAC 不跨设备可比、未升级 Claim。

OWF 实施位置优先 `workflow/owf_dashboard/`、其 tests 与必要说明。`serve.py:119–127` 已有控制模式 POST 与认证/Origin 边界；`board_state.py` 仅识别既定 SECTIONS，不能塞入任意顶级字段。新诊断关联使用独立 sidecar。`check_delivery.py` 任务 schema、installer authority、角色预算维持原状。

## 3. 范围、默认选择与授权终点

- 允许：在两仓的独立候选中实现与本集成直接相关的 CLI、版本化 resolver/诊断适配、协议、桥接、Web UI、隔离设置、测试、文档与开发产物；修复本次引入的问题；本地 loopback 服务与真实浏览器验证。
- 只读输入：两指定源仓的项目指令/必要源码和工具 `--version/--help`；不自动遍历真实 home、不读私人历史会话/凭据；需要全局指令测试时用独立合成根，明确与用户真实 profile 不同。
- 允许写入：独立候选、任务私有证据与专用测试 store、隔离 OWF home；扫描目标和原生配置不写。不得将普通 shell 的 HOME/CODEX_HOME 重设为临时目录；需要 profile 隔离时由有所有权的子进程 env 单独传入，不改宿主环境。
- 不授权：commit/push/PR/正式发布、全局安装或配置变更、替换现有运行服务、物理合仓、删除旧产品、迁移既有 store/密钥、付费模型/Pro 调用、开启长期自动监控或招募用户。
- 直接实施：新会话当前宿主执行与调试、自检，不自动派新实施模型/reviewer，不建立无用编排账本。本轮 1 个 subagent 是方案审，不是未来产品独立验收；新增独立验收如被有效合同明确要求，必须保留该条件，不能以自检替代。
- 结束于：完整本地可审阅候选、复现命令、产物摘要、逐项回执。未发布、未安装、未独立产品验收明确列出，不冒称生产交付。

旧 F/WP/cutoff/required corpus 不变。仅本次明确授权的新 development lane 进入本集成专项验收；历史 frozen lane、golden、完整验收状态分开保留。路线调整只记录新增工程投资的范围，不追溯把旧功能改为完成，也不删除旧 required-write。

## 4. 工程包 E01–E12

| 编号 | 必须完成的行为 | 验收要点 |
| --- | --- | --- |
| E01 候选保全 | 两仓独立本地候选；复制 tracked 工作树与明示需要的 untracked 源码/文档；保留源仓 | HEAD、dirty diff、复制清单和逐文件摘要；不复制私人音频、凭据、node_modules、旧 store/任务运行态 |
| E02 当前坐标 | 新增明确版本/OS/能力的 development 支持，至少当前实际 Codex CLI 的静态 instructions 可用 | 当前二进制/版本/OS 读回；规则依据、正负例和未知版本拒绝；不能仅展示 unsupported 后宣称完成 |
| E03 引擎协议 | ctxpect 单源提供能力、坐标和诊断输出合同；复用正式 Receipt 路径 | 独立 CLI 可运行；schema major、engine digest、request/run id、事实来源、原始 reason code；未知 major 拒绝 |
| E04 原生观察 | 新 development 适配或显式导入路径，旧 frozen oracle 不动 | 实际运行时记录二进制/profile/坐标/退出码；字段缺失保持 Unknown；原始敏感正文不落公共日志 |
| E05 进程桥 | 固定信任的 engine，固定 argv 合同，专用证据/store | 超时、取消、输出限额、非零退出、重复/并发请求、崩溃恢复；不得 shell 执行用户字符串 |
| E06 关联存储 | 版本化独立 sidecar，原子写、锁、损坏拒写并告警 | 不改 task.json 或原 ledger；项目无任务可运行；既有任务只引用，不生成空 task |
| E07 新鲜度 | 实际检查输入与坐标绑定，不拿 HEAD 代替 dirty 内容 | 输入/引擎/harness/profile 变化时 stale；删除 unavailable；摘要错误 integrity-failed；错误不覆盖最后有效结果 |
| E08 安全入口 | 默认关闭；只读模式只展示已有脱敏结果；控制模式 POST 显式触发 | Host、Origin、Bearer、JSON/路径范围；GET不触发新诊断；缺认证/跨源/越界/任意argv都拒绝 |
| E09 Web 体验 | 项目入口、既有任务证据卡、详情、取消、刷新、关闭 | loading/running/failed/Unknown/stale/unavailable完整；不新增总健康绿灯；上下文发现与通用全仓finding分开 |
| E10 真实闭环 | 实际候选 engine+OWF服务+浏览器，至少两个允许项目及合成反例 | CLI/OWF消费同一原始输出与Receipt引用；一条可重复的差异解释；不得以mock替代真实成功路径 |
| E11 回归与回退 | 双仓完整门禁、专项负例、关闭/重启/回退 | 关闭入口不影响旧任务；只清理自己进程；保留历史证据；不篡改门禁/冻结golden来消红 |
| E12 交接 | 本地启动入口、报告、文档索引和后续价值观察模板 | 结果可重建、证据可校验；工程/经营/旧产品验收分列；不等待数周后才交付代码 |

E04 原生工具若因外部权限、诊断接口消失或需要未授权登录无法运行，先完成 E01–E03/E05–E12 的独立部分，将 native 专项记 BLOCKED/NOT_RUN。此时不能宣称 E01–E12 全部通过；不得退回假响应或把 static Unknown 当 native 完成。E02 当前静态坐标若也无可核验依据，属于核心阻塞，不能绕过它交付“可用集成”。

## 5. 建议协议与运行语义

不预设现有接口已存在；实现者可选新增 CLI 子命令或兼容 envelope，避免改坏现有 CLI 消费者。新集成协议与冻结 Receipt schema 各自版本化。能力矩阵由 engine 单源输出，OWF 不重写 resolver 支持表。

请求至少绑定：request_id、注册 project_id、受授权 cwd、请求能力、明确 harness/version/surface/OS/profile、允许读取范围。引擎绝对路径和摘要由本地配置决定，不能从网页 request 指定。默认扫描 instructions 必要输入，不默认触发全仓 secret 审计、MCP/hook、原生配置修改或模型调用。

结果至少分开：process status/exit、诊断判定与 Unknown reason、静态结果、声明范围的 native observation、formal Receipt 引用/摘要/验证类型、检查输入清单及摘要、时间和来源、engine/resolver/harness/profile 身份。不统一成一个 success/verified。进程成功不等于诊断pass，HMAC完整性不等于组织信任。

成功触发返回稳定 run_id；相同运行意图重复请求复用活动 run。每个授权项目最多一个活动诊断，不同坐标冲突返回忙而非双写；允许实现简单的全局串行，不能做无界队列。超时/取消清理本调用的进程树；重启残留运行标 interrupted，需显式新触发，不自动重放。

正式 Receipt 由引擎存储和验证，OWF 只存 run/项目/可选task 的关联与展示缓存。sidecar 推荐不可变 run 文件+小索引；可以用其他等价原子方案，不新增通用调度/事件平台。存储损坏不静默清空、未知 major 不猜读。原文按允许列表脱敏再展示；文件与报告内容按不可信数据渲染，HTML转义，不可执行命令或外链内容。

新鲜度用真实输入摘要、坐标和引擎身份判断：文件内容变化、相关文件新增/删除、cwd/profile/工具变更均失效；扫描期间变化标 unstable 并至多一次有界重试，仍变化则停止，保留旧结果且不可用于当前判定。哈希可本地比较，不把不同密钥的 HMAC 当跨设备内容等价。

## 6. 单写入者与默认不影响旧用户

| 域 | authority |
| --- | --- |
| OWF安装文件/receipt/备份/恢复/生成规则 | 原 renderer/installer；本集成只读引用 |
| Context Claim/Receipt/ledger | ctxpect；OWF不重签、不拼装 |
| 原任务预算/事件/评审/交付 | OWF原流程；诊断不参与自动GREEN |
| 新诊断关联/运行索引 | 本集成独立sidecar写入器 |
| 用户原生文件、APM包 | 原authority；本轮不写 |

OWF安装 doctor会写回last_verified_at，因此不把它当后台只读获取接口。读取旧安装收据要显示本轮未复验。现有 board state 或 task schema 不扩顶级字段，不将 receipt 引用写进 task.json。新可选输出字段/schema 必须记录理由、范围、收益、已有可复现事实依据；引用研究发现与本轮方案对抗稿，不虚构事故、不新增常驻规则或提高角色/规则预算上限。

默认未配置 engine 或集成关闭时，旧用户无需Rust、无需新服务，所有原功能与启动路径保持。使用OWF既有控制模式认证，默认只读服务不扩大为执行入口。原生菜单栏/Windows端继续读既有API，不为本切片重做原生界面或发行包。

## 7. 必需验收和边界

每个E项必须有路径/命令/退出码/产物摘要。当前候选需跑ContextView现行16条required gate；本切片本地交付另要求现有ui-e2e与新增真实桥接浏览器E2E。OWF跑render --check与check.sh，安装相关测试必须隔离home；解释原有允许skip，新增重要覆盖不能静默skip。

新专项反例：未知版本/OS、坏或超大JSON、非零退出/超时/取消、重复点击、服务重启、被替换engine、symlink越界、任意命令注入、Host/Origin/token错误、旧证据/删除/篡改、写入冲突、恶意HTML、不同profile、扫描中变更、可选task不存在、关闭功能后的回归。

真实浏览器必须完成：注册允许项目 → 显式核对 → 结果与证据 → 重复触发 → 输入变更后失效 → 引擎缺失提示 → 恢复 → 关闭集成 → 原看板仍可用。通过范围内修改合成副本制造失效，不修改源仓来做负例。验证前后扫描目标/原生配置摘要不变。真实项目需求样本不足可写待收集，不用合成用例顶替。

不承诺新增独立产品review GREEN。此处方案review为独立cross-review；未来工程自检、现有合同要求的独立验收、生产发行各自列状态。远端CI本轮没有新提交，不等待旧HEAD的绿替代本候选。

## 8. 自动推进、真实阻塞与价值观察

正常命名、模块拆分、修复、测试、端口、临时目录、兼容性细节由实施会话决定并继续；不问“是否继续”，不因人日估计/阶段结束重复请示。先跑可自动化诊断，失败就保留原始输出并修复自己引入的问题；不把一次失败直接升级为用户问题。

仅在真实无法继续时留完整恢复材料：凭据/系统解锁由本人提供、有效required条件缺失、不可归属并发冲突、平台权限拒绝、能力/预算硬限制、必须破坏用户数据才可继续。先完成不依赖该阻塞的工作；不可安全继续的依赖分支不跨越。不中途确认不是绕过权限或编造通过。

价值观察单独记三类比较：A原生/现成工具、B加独立ctxpect、C同引擎OWF入口。只提供可用记录模板及本轮实际案例，长期复用/正确解释耗时/维护成本等没有数据就NOT_RUN。不等待6周、不启动自动化、不将小样本或工程正确性当商业结论。后续是否扩大投资仍取决于真实使用，不自动恢复独立桌面/同步等整套路线。

## 9. 交叉review与本次交付

本轮调用且仅调用1个subagent，负责独立只读核对与同一稿复核，未派生其他agent。意见及采纳见[交叉review记录](../review/2026-09-29-context-integration-cross-review.md)。旧审批冲突、development/frozen坐标、控制入口、sidecar及证据语义均已纳入本稿。

完整本机实施prompt含绝对路径，保存在本机handoff目录，不把用户家目录提交到可发布docs。粘贴prompt/启动消息后新会话才施工；本次不创建或启动实施聊天。新会话首次记录实际候选位置，长任务在候选外维护短恢复状态与REPORT。
