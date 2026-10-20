# Contexpect × OctoWorkFlow 最终方案交叉 review

> 日期：2026-09-29；范围：方案和新会话实施 prompt，不是产品代码独立验收。
> 角色：用户明确要求的 1 个 subagent（cross_review）。初评与最终稿复核均为同一 agent，未派生其他 agent。
> 结论：最终复核未发现新增 P1 执行阻塞或实质内部矛盾；唯一 P2 状态句已按原建议修正。产品测试 NOT_RUN，不报告独立产品 GREEN。

## 1. 输入与方法

只读核对[前轮研究](../plan/2026-09-29-product-direction-and-octoworkflow.md)、[最终工程方案](../plan/2026-09-29-context-integration-final.md)、本机完整实施 prompt，以及两仓 AGENTS 与必要源码。重点检查授权终点、旧审批冲突、实际坐标与冻结范围、控制入口、关联存储、证据语义、候选保全和验收。

未改产品文件、未运行产品测试、未访问凭据或私人会话、未调用 Pro。本次独立性是不同 agent 的方案交叉检查，不宣称不同模型或未来实现已获独立验收。

## 2. 初评问题与宿主裁决

下表 P1 指“若原研究直接作为新施工合同，会实质阻断或误导本次授权执行”的方案风险，不是给既有产品新报 P1 漏洞。没有可达产品失败证据的存储/运行设计建议列 P2，不虚构事故。

| 项 | 级别 | 源码/方案依据 | 宿主裁决与最终处理 |
| --- | --- | --- | --- |
| 逐阶段再审批与完整实施冲突 | P1 合同风险 | 原研究 §5、原第93–107行 | 采纳；新 canonical 明确替代逐阶段审批；E01–E12 采纳即完整预授权，长期经营指标独立 |
| 当前工具坐标冒用冻结 oracle | P1 验收风险 | ContextView `native_oracle.rs:94–114,174`、AGENTS cutoff | 采纳；新增明确 development lane 及专项验收，旧 pin/golden/required scope 保留 |
| 以只读名义绕开执行权限 | P1 入口风险 | OWF `serve.py:119–127` | 采纳；默认只展示，触发走已有控制模式 POST/Bearer/Host/Origin；不让 GET 启动新诊断 |
| 任意 board state 字段重载丢失 | P2 合同完备性 | OWF `board_state.py:22–23,57–62` | 采纳；独立版本化 sidecar，单写者/锁/原子写/损坏告警；不改 task.json；这是机制风险，不声称已发生丢数据 |
| native/profile/HMAC 被升格 | P2 语义边界 | ContextView `native_oracle.rs:221–234` | 采纳；隔离 profile 与真实用户会话分开，静态/native/process 分轴，本地 HMAC 不做跨设备等价依据 |
| 调用、失效与重启语义不足 | P2 合同完备性 | 原方案仅概述进程与缓存边界 | 采纳；明确 run_id、去重/串行限制、超时取消清理、interrupted、stale/unavailable/integrity-failed |
| dirty 与 UI 验收不够可执行 | P2 交接完备性 | 两仓 dirty 状态、ContextView/OWF 门禁合同 | 采纳；两仓独立候选准确复制，源仓只读；真实engine→OWF→浏览器与双仓完整门禁 |
| schema 准入与责任界定 | P2 维护约定 | OWF AGENTS 规则准入 | 采纳；新增协议不改正式task治理字段；写理由/范围/收益并引用可复现源码事实，不捏造事故、不增加常驻规则/预算 |

## 3. 最终稿复核

subagent 最终报告支持本方案，确认以下条件明确且相容：

- E04 原生外部能力不足时继续其他独立包，但整体保留 PARTIAL/BLOCKED；不能以 Unknown 分支冒称原生验收。
- E02 当前静态支持缺依据时不可报告“可用集成”；实际 CLI/OWF 成功路径和 required gate 必须实测。
- 人工阶段审批点 0 不绕过真实平台权限、必要验收条件或无法安全处理的并发冲突。
- dirty 源仓只读、快照完整；旧冻结范围和新 development 专项分轨。
- 后续默认宿主 direct、自检；本次方案审不能替代未来有效 required 独立验收。
- 不包含 commit/push/全局安装/正式发布/历史迁移或长期自动监控。

最终提出一处 P2 语法歧义：prompt 将“E02可用当前静态坐标、实际CLI+OWF成功路径、关键门禁未过”压缩成不清楚的条件。宿主按 reviewer 给出的原句替换为：

> E02未实现当前静态坐标，或实际CLI+OWF成功路径未通过，或任一required/关键门禁未通过时，整体只能PARTIAL/BLOCKED；E04为BLOCKED/NOT_RUN时也不能报告E01–E12全部通过。

此修正只澄清已有条件，不扩大范围。无未解决 P1 方案问题；这不是产品门禁通过声明。

## 4. 最终产物状态

最终方案为本主题工程 canonical；旧研究加历史指针且保留原始裁决。完整 prompt 保存在本机 handoff，不把绝对用户路径写入发布文档。本轮未启动实施会话、未生成产品候选、未 commit。
