# 用户提供方案的来源快照清单

> 日期：2026-09-04  
> 目的：为需求追踪固定两份用户附件的不可变身份。原文仍保存在 Codex attachment store；本项目不改写原文。

| Source ID | 标题/识别 | 原始路径 | 行数 | 字节 | SHA-256 |
|---|---|---|---:|---:|---|
| S1 | `AI Coding Context Control Plane` | `$HOME/.codex/attachments/09656809-67e1-4c5c-83bc-ebfe78e0518f/pasted-text.txt` | 2,806 | 38,248 | `e90f4b4ca4d77c4244faeadee5dceb6a4c384a231c55393a9616d191253a6f6d` |
| S2 | `织境 Loom：调研后的思考与项目需求` | `$HOME/.codex/attachments/91ed7277-4f35-4f01-a11a-a7137cdf702e/pasted-text.txt` | 2,240 | 31,002 | `1698aed2e7a905f1a3d3fd78ea7a5d7e04d7745a7ee500dc8bd5e42f83bc2ee1` |

校验命令：

```sh
shasum -a 256 <path>
wc -lc <path>
```

若附件存储未来不可用，以上 digest 只能证明已有副本是否同一，不能恢复原文。正式建仓前应在用户允许的情况下用普通文件复制机制归档原文；归档副本必须保持上述 SHA-256，不得用总结替代原文。

## 命题追踪

| 命题 ID | 来源 | 原始主张 | 处置 | 进入需求 |
|---|---|---|---|---|
| P01 | S1/S2 | 产品核心是可视化上下文管理 | 保留并收紧为 evidence-backed reconciliation | §1、F-05–F-07 |
| P02 | S1/S2 | 跨 Codex/Claude/Cursor/Grok 查找规则、skill、MCP | 保留，按 version/surface 实现 adapter | F-01–F-03、§11 |
| P03 | S1/S2 | 跨设备同步 | 保留，采用 encrypted bundle + 外部 transport | F-10、WP-07 |
| P04 | S1/S2 | LLM 判断是否合适、有价值 | 修正为 evidence-linked Advisor；效果由受控实验判断 | F-13、F-15 |
| P05 | S1/S2 | skill/MCP 管理和社区 | 保留 catalog/安全/import/export；市场和 package 生命周期集成 APM 等 | F-11、§15 |
| P06 | S1/S2 | 历史会话分析与建议 | 保留，限定已导入、可观察窗口和本地隐私 | F-14 |
| P07 | S1 | Context IR / Effective Context / Drift / Debt | 保留 IR、drift、debt；Effective 拆成六级 claim | §4、§10 |
| P08 | S1 | Canonical Intent + Native Overlay | 保留；明确 declared intent 不等于行为等价 | F-07、F-09 |
| P09 | S1 | Expected/Observed/Opaque | 保留并改成多轴 claim/evidence model | §4、§10 |
| P10 | S1 | Context Graph / 图数据库 | 只保留可视化；否决图数据库前置 | §9、§10 |
| P11 | S1 | Context Doctor | 保留 deterministic Doctor；LLM 分离 | F-08、F-13 |
| P12 | S1 | Semantic alignment percentage / task probability | 否决无校准数值 | §3、F-04、F-07、§19 |
| P13 | S1 | Runtime trace 能直接证明 usefulness | 修正：trace 证明 observed/used 的一部分，effect 需实验 | §4、F-12、F-15 |
| P14 | S1 | 云、社区和 marketplace 自建 | 否决自建通用生态；集成现有项目 | F-10、F-11、§15、§19 |
| P15 | S1 | ContextOps 命名 | 否决，已有同名项目/PyPI | §22 |
| P16 | S2 | always/conditional/progressive/runtime/opaque 加载态 | 保留为 activation/lifecycle 维度，不当作唯一真值轴 | §4、F-03、§9 |
| P17 | S2 | 静态约占三成 | 作为比喻丢弃，不展示为统计事实 | 研究台账 §3.2 |
| P18 | S2 | AGENTS.md 作为普通话 | 部分保留；是共享输入之一，不是唯一 canonical format | §2、F-09、F-11 |
| P19 | S2 | 项目 Git、机器云同步 | 修正为来源策略；execution environment 单独建模 | F-01、F-10 |
| P20 | S2 | 四家原生加载规则 | 保留为 adapter 研究输入；以官方资料、版本与 fixtures 核验 | 研究台账 §6、F-03、§11 |
| P21 | S2 | “织境 Loom”命名 | 中文意象保留，英文名因 Atlassian Loom 冲突而否决 | §22 |
| P22 | S2 | 分阶段只做观察、后做其他 | 否决范围裁剪；改为完整交付工作包依赖顺序 | §0、§16–§18 |
| P23 | S2 | 示范工作区 | 保留为 fixture corpus 场景，不当产品替代品 | WP-01、§17 |
| P24 | S2 | 静态扫描可精确给 token | 修正为 exact/estimated/unknown 多轴证据 | §4、§17 |
| P25 | S1 | “Where should this live?”：把事实、确定性约束、权限和按需流程放到合适载体 | 保留为 PlacementRecommendation，不由 LLM 自动迁移 | F-08、§17.6 |
| P26 | S1/S2 | commands/prompts、agent/subagent、@/retrieval、steering、tool search、MCP prompts/resources 等动态上下文也属于完整拼接面 | 保留为封闭 capability taxonomy；不可观察项进入 unknown-honesty | §4.6、F-02、F-12、§17.0–17.1 |

以上 26 条覆盖两份来源的核心产品命题；文件路径和 digest 允许后续 reviewer 回到原文逐段复核。若发现遗漏，新增命题 ID，不重排既有 ID。
