# 交接包价值裁决与本地候选实施（2026-09-12）

## 范围与证据

完整阅读交接包 29 个文件：16 份主文档、2 份 originals、11 张图片。包内文件与 `docs/handoff/contexpect-2026-09-11/` 逐文件字节一致；历史图不作为现行 UI 合同。首轮评估时仓库基线为 `257b7d4d82c0865a94b45d9b3fddcb3830658b32`，已有大量未提交接续改动；本轮在其上形成未提交候选，未重写已有工作。附件中的启动 Prompt 是材料，不自动激活外部执行、独立验收或发行授权。

裁决：延续已做的语义修正、浅色 UI 和单一产品数据源；优先补真实写入与恢复反例，以及离线/删除后的状态一致性。此时全面更换依赖、重造 DemoDataSource 或把静态结果升级为运行结论都不能解决当前已证实缺口。

## T01–T12 逐项裁决

| 项 | 当前实证与裁决 | 本轮处理 / 后续准入 |
| --- | --- | --- |
| T01 基线对账 | 已有接续改动不能凭旧 GREEN 认领 | 保存开工 diff；本轮重新运行 16 required gate 与 ui-e2e，结果见下 |
| T02 真值语义 | C-F01/02/04/05 已有源码；C-F03 `ctxpect-policy/src/precondition.rs` 是纯判定表，尚未接入 mutation pipeline | 保留修正；不将纯函数测试宣称动作门禁已全面接通 |
| T03 原生解析 | Codex/Claude 静态解析、观察排除及预算已有改动 | 不另造解析器；真实版本原生对照仍需逐 cell 证据，静态测试不代替 |
| T04 真实证据桥 | DSH importer 有 synthetic conformance；不等于真实运行链 | 保持 `live_tested: false`；真实最小样例、来源、删除和幂等才是下一阶段准入 |
| T05 Doctor | 声明检查已标 `declaration-validation`；时钟输入已有修正 | 沿用源码与本轮门禁；不把未执行规则当无命中，真实生产者仍待 T04 |
| T06 安全变更 | projection 的冻结预览已有闭环；assets API copy 却重新计算计划，无法拒绝预览后目标变化 | 已补资产冻结预览、作用域/过期/消费校验及 post-Receipt；并发锁不声明能隔离所有同权限程序 |
| T07 基础依赖 | 本轮实证问题均可用现有代码修复 | 不增加第三方 crate；SQLite/schema/crypto 升级仍按 ADR 0006 单项论证、迁移与回退验证 |
| T08 外部适配 | APM authority 边界明确，但第三方报告格式/转换版本未冻结 | 先固定只读输入合同与 provenance，再接适配器；不将调研链接当集成完成 |
| T09 UI/CLI/桌面 | 浅色、路由、Advisor 和跨入口测试已有；资产与离线闭环存在可达缺陷 | 已修 UI 预览失效、Receipt 链接、保留快照及测试状态隔离；浏览器与 WebView 分开验收 |
| T10 信任与同步 | 本地存储/摘要/同步实现不等于组织信任或 E2EE | 保留完整范围；需密钥、撤销、分叉、双设备及无明文旁路验收，本轮未实现 |
| T11 历史/Advisor/Effect | Advisor 双同意边界已有；历史删除与派生结果失效缺浏览器覆盖 | 已补真实删除→页面刷新失效；runner、费用授权、原始日志及统计准入仍未完成 |
| T12 发行矩阵 | 本机软件门禁可重跑；全 OS、soak、安装升级、人工与独立 readback 不在本轮证据内 | 交付本地未提交候选；不声明全产品或发行完成 |

## M01–M20 采用方式

以下是对包内建议的裁决，不是本轮重新查询外部项目的版本或健康状态。完整能力范围不裁剪；“保留候选”不代表当前已经实现。

| 模块 | 裁决及原因 |
| --- | --- |
| M01 采集 | 保留当前受限读取；ignore/cap-std 需以现有逃逸/不可读反例证明收益后单项迁移 |
| M02 adapter | 借鉴原生源实现和版本样例；按 adapter 推进，禁止通用文本规则替代原生语义 |
| M03 IR/schema | 保留统一合同；jsonschema/Test Suite 可作为后续一致性差分输入，先确认依赖政策 |
| M04 importer | 保留 metadata-only 与 Unknown；Scopeon/ContextSpy 只能作为输入适配参考 |
| M05 Receipt | 本轮补资产操作后的新静态观察，旧 Receipt 不改；签名/组织信任不能由摘要补齐 |
| M06 store | 保留当前账本；SQLite/FTS5 需真实检索/事务需求和迁移回退测试，不因流行而替换 |
| M07 diff | 保留字节与语义分离；Difftastic 等只能辅助展示，不产出 equivalence 真值 |
| M08 Doctor | 保留高精度规则与声明来源区分；agnix/Gitleaks 报告先限定为外部诊断输入 |
| M09 assets | APM 保持来源/包权威；已修本地登记资产复制闭环，未声称实现 APM 安装器 |
| M10 projection | Rulesync 只生成受限候选；不能绕过批准写真实目录 |
| M11 mutation | 本轮优先实施，覆盖冻结预览、并发、消费、回滚与写后观察；多文件崩溃恢复另验 |
| M12 sync | rage/Syncthing 的职责应分开；真实加密和传输验收缺一不可，保留后续候选 |
| M13 policy | 保留 action/project/target 授权；Cedar/OPA 不自动解决可信批准来源 |
| M14 daemon | 本轮沿用现有 daemon；Axum/notify 迁移须有协议、watch 丢事件或维护成本实证 |
| M15 UI | 保留白底黑字与现有 DTO；已修同路径刷新保留数据和跨路径清空，不为此引入 Query 框架 |
| M16 Advisor | 保留双确认、最小发送、candidate-only；不为接 Rig 而增加真实发送面 |
| M17 Effect | 保留未执行/不可判定和冻结统计；Inspect AI/promptfoo 须在真实 runner 阶段论证 |
| M18 观测 | 保留诊断日志隐私边界；tracing/OTel 需资源预算、脱敏与关闭路径后接入 |
| M19 测试 | 已扩展 Playwright 真 daemon 反例；隔离 store 消除文件名顺序依赖，合成证据不称 native live |
| M20 发行 | 保留完整平台和供应链门禁；cargo-dist/cargo-deny 属发行方案，不能由本机 build 代验 |

## 八项语义修正的接续状态

- C-F01：观察排除与原生预期分离，已有改动保留；测试通过不证明所有 adapter 正确。
- C-F02：相同字节不同语义的 ST2 反例已存在，保留，不回退到摘要等价。
- C-F03：按动作处理 Unknown 的方向成立，但纯判定表尚未接通所有入口；本轮资产继续采用已有可信授权，运行效果未知不阻止有限复制。
- C-F04：声明来源已标注；UI Advisor 不写 Claim/policy/CI/baseline/reconciliation。
- C-F05：已有历史时钟修正；本轮 copy/rollback 各形成新静态观察，并校验旧 Receipt 字节不变。
- C-F06：新增资产预览后目标改动反例；仍不承诺抵御所有同权限程序在校验与写入之间的竞争。
- C-F07：新增可达反例覆盖后端、浏览器和磁盘；不把保持旧错误行为当回归标准。
- C-F08：已有导入删除机制；本轮实测删记录后派生建议失效、列表刷新移除、源文件不变。

## 本轮具体变更与验收边界

1. Assets API preview 持久化冻结元数据，copy 必须提交 `preview_id`。过期、跨项目、已消费、登记变更或目标变更均拒绝；每份预览只允许一次写入尝试。新的预览使用新的 tx，防止重试覆盖旧备份。
2. Copy/rollback 成功后尝试静态 inspect 并保存 Receipt。`runtime_verification: not-observed` 明示运行未知。静态保存或 asset lock 登记失败时仍诚实报告文件操作已完成，并独立返回错误，避免用户盲目重复写入。操作元数据进入开发 snapshot 的 digest，区别同内容的 copy/rollback 观察；普通 inspect 去重不变，正式 Receipt schema 和 Claim 的来源等级不变。
3. Assets UI 在更换资产或失败时废弃旧计划，展示新 Receipt 链接与运行未知；原始 JSON 改为可展开，避免主要操作被超长诊断数据淹没。
4. 自取数 StateView（如会话列表、实验详情）同一路径刷新/取消/传输失败保留上次结果并提示其非最新；切换路径或收到服务端错误不复用旧数据。
5. 新增按测试独立 daemon/store 的夹具。DEMO-06/07/09/12 不再依赖文件名字母顺序或共享权限；DEMO-13/17 增加删除失效、离线恢复和跨资源切换反例。合成会话和实验仅用于测试，网络中断仅由浏览器 route abort 注入。

## 首轮候选验证结果

16 项 required gate 已分别执行，退出码均为 0；另跑 `ui-e2e`，34/34 通过，UI 单测 60/60 通过。包括完整 Rust workspace build/test/clippy、三个 conformance、六项文档/验收检查和四项 UI 门禁。浏览器使用真 daemon 和合成数据；目视检查资产目标冲突、回滚 post-Receipt、会话离线保留三张实际截图。原生 conformance 的通过仅限合成数据合同，不提升 live coverage。未执行真实 harness、真实 WebView、双设备同步、付费 Effect runner、跨 OS 发行或独立验收；未 commit、push。


## GitHub 提交前接续复核

Owner 随后授权继续实施有价值的部分并将完整更新提交 GitHub。复核保留前述 T01–T12/M01–M20 能力边界，不将提交源码称为产品发行验收。

- 比较页原先在更换 Receipt 后仍展示上一组 diff，且旧请求能晚到覆盖新选择；现绑定请求代次，选择变化立即废弃结果并取消旧请求。列表离线失败后的重试也修为重新获取列表。跨坐标 `same_domain=false` 增加可见解释，`baseline_allowed` 和 `verified` 仍由真实 API 决定。
- 新增 `truth-boundaries.spec.ts` 四条真 daemon/browser 测试：DEMO-02 静态 eligible=present 而三项运行 facet 仍 indeterminate；DEMO-03 产品排除不变成原生 absent、预算仍 unknown/null；DEMO-10 跨坐标不可形成基线、切换选择废弃结果并取消旧请求；比较列表离线恢复。四条针对性验证通过，并检查实际截图。
- 收口此前仅登记的 DSH 数字形状缺口：拒绝非规格化指数尾数、溢出/下溢、错误指数阈值。`10e+21`、`0.1e-7` 等实测不能原样由 JS serializer 输出；schema 正反例与 importer 集成反例通过。损坏 header 前已开始的 step 仍可列出，但不能产生精确 header digest 或 Claim。该过滤不是完整 IEEE-754 shortest-round-trip canonicalizer，不升级真实原生覆盖；正式 product JSON 仍拒绝浮点。

- 进一步跨 CLI/API 对账修复重复复制覆盖备份和回滚后 SBOM 残留：唯一 tx 由共享 assets 层生成，备份目录独占创建；复制前保存旧 provenance，回滚恢复旧记录或删除首次复制的记录。后续相同字节的复制也不能被旧事务越过，必须先回滚较新的事务。事务绑定项目摘要，跨项目回滚拒绝；缺少作用域/旧 provenance 的历史事务不推测恢复，明确拒绝并保留备份供核查。

尚待独立工作包：C-F03 全 mutation pipeline 接线、真实原生证据桥、E2EE/组织信任、真实 runner、其余 DEMO 子路径和发行矩阵。它们的前置合同、设备或外部环境没有因本地提交而满足。当前未发现必须在此次源码提交前继续扩张依赖或外部执行面的理由。

完整未提交更新作为同一 Git 候选核对；提交后在干净 HEAD 上运行 16 项 required gate 与 ui-e2e，再推送 `origin/main` 并检查该提交的 GitHub Actions。执行结果以对应 Git 提交及 CI、会话中真实门禁回执为准。
