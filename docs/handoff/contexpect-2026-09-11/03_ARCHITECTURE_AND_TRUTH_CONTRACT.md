# 架构、真值合同与数据所有权

**状态：本会话最终架构建议的可执行摘要。** 字段以已有 schema 为准，本文不表示全部已实现；需扩展时合并现有对象，不创建平行协议。[^R2]

## 1. 逻辑架构

```text
显式授权文件 / 原生设置 / 原生导出 / 外部只读报告
                        │
                受控读取与输入校验
              ┌─────────┴──────────┐
              │                    │
         版本化 resolver       原生/外部 importer
         Expected + 依据       Observed + 覆盖/来源
              └─────────┬──────────┘
                        │
             Context IR / Claim / Receipt
                        │
              本地权威账本与证据索引
                        │
                Diff / Doctor / Policy
                        │
        意图/修复/标准候选 → 原生 renderer
                        │
            冻结 proposal / loss / 摘要
                        │
            身份与动作级权限 / 批准重验
                        │
              唯一 mutation 协调入口
                        │
          受控文件写入 / journal / 恢复
                        │
           重扫 → 按需取得新的原生证据
                        │
              新 Receipt / 明确未核验
```

CLI、daemon、UI 共用 application service。Advisor 是建议旁路，实验 runner 是受控结果输入，不向它们授予任意 Claim、策略或文件写入能力。20 个研究环节不是20个服务。[^R1][^R2]

## 2. 解析坐标

沿用项目的 device、environment、OS+arch、account alias、organization、policy snapshot、harness、version、surface、project/Git root、worktree、cwd、task probe、timestamp 等坐标。来自探测、用户声明或 Unknown 的字段分别标注。实例、session、request/turn 及采集时间要能与快照关联。[^truth][^R2]

`harness_version` 不是 `model_name`；同一工具的 CLI/IDE/cloud 不是同一个 surface。UI 缓存键和异步 generation 包含所有实际影响该查询的输入，不仅是一个项目名。不能把最新文件结果套回旧会话。

## 3. 六个独立证据面

| Facet | 问题 | 不能从什么直接推出 |
| --- | --- | --- |
| Installed | 文件、包或配置声明是否存在？ | 目录残留不证明可运行安装 |
| Discoverable | 对应原生规则会发现吗？ | Installed 不保证发现 |
| Eligible | 当前 cwd / task / files / policy 符合条件吗？ | Discoverable 不保证激活 |
| Model-visible | 相应原生运行字段是否证明模型可见？ | 静态解析、截图、包安装不证明 |
| UseEvidence | 观察到调用、引用或行为线索吗？ | 可见不等于内部实际采用 |
| Outcome-affecting | 有符合冻结合同的效果结果吗？ | 使用线索不等于因果改善 |

六面不可画成必然逐级点亮的进度条。`invocation-observed`、`reference-observed`、`behavior-consistent` 和 `internal-attribution` 不混为一个“使用了”。内部归因不由外部线索自动确证。[^truth]

## 4. Claim 的轴

| 轴 | 仓库合同中的取值 |
| --- | --- |
| claim_kind | resolved / observed / effect |
| lifecycle_stage | 六个 facet |
| truth_state | present / absent / indeterminate / not-applicable |
| provenance | native-runtime / native-log / harness-source / official-spec / heuristic / user-attested |
| coverage | full-declared-surface / partial-declared-surface / unknown |
| precision | exact / derived / estimated / not-applicable |
| knowledge_status | current / stale / conflicted / unknown |

LLM 不作为新增 provenance。Unknown 是证据不足的合法表达，不是严重程度，也不必再新增 `truth_state=unknown`。`absent` 要有足够覆盖；未读、无权限、未知版本和日志缺段都不是不存在。effect 的 present 表示有效结果对象存在，真正结论由 effect decision 表达。[^truth]

## 5. Receipt 与衍生分析

保留既有 one-shot、preflight、runtime、post-session、device baseline、CI 六类及正式 schema 的编码。旧开发快照必须经显式迁移，不能只改 schema 名称。正式 Receipt 的身份、原始签名字节、算法和历史时间不被新 parser 或 SQL 迁移隐式重算。[^truth]

衍生分析记录所用 Receipt、规则版本、评估时间和额外证据。删除 evidence 后保留最小 tombstone，并使相关分析/建议/索引失效。新的规则解释不是新的原生观察。导出策略按用途脱敏，不默认导出源代码、完整 prompt、工具结果或 secret。

本地 HMAC 表示 local-continuity，不是组织身份背书；用户能修改的原生格式日志也不是天然防篡改。签名、来源声明、采集通道可信度、覆盖和新鲜度需分别解释。[^R2]

## 6. 四类所有权与三种外部模式

| 所有权 | 规则 |
| --- | --- |
| 资产/意图源 | 每资产登记，可继续使用原生文件；不强制迁入自有 manifest |
| 投影规则 | 每目标坐标一个 authority，明确 overlay 与 loss |
| 实际文件写入 | Contexpect 默认协调；明确委托时不能双写 |
| 最终核对 | 核心校验并汇总，保留外部来源的未知和限制 |

外部工具先用只读产物导入；其次用显式隔离的候选生成；只有能约束目标版本的修改集合、权限和恢复时，才准入委托执行。外部报告、proposal、mutation、bundle、实验结果应合并进现有合同。[^R2]

## 7. 存储与同步

当前基线为 JSON ledger；SQLite+FTS5 是已有目标，但未完成准入与迁移不能写成已使用。每个本地信任域一个权威引擎，迁移可对照读但不长期双写。FTS 只能索引允许保存的数据。[^adr6][^R2]

设备之间传输不可变、经授权的 bundle，不传 hot DB/WAL、private keys、token、raw session 或明文备份。先检查结构与完整性、按格式完成验签解密、确认信任与版本谱系，再处理分叉，最后走正常采用/投影。无证据时不指定一个固定通用的“先验签还是先解密”顺序代替具体封装合同。

## 8. 错误、降级与出口

沿用现有命令级 `0/2/3/1` 合同与 error.code；不能发明一个所有工具共用的成功映射。外部进程未启动、非零退出、解析失败、partial 和无命中分别处理。只读路径可在安全范围内保留结果；与写入有关的前置条件未知时拒绝对应动作。

产物应分开提供：操作执行状态、静态核对状态、运行核对状态、效果结论和信任验证状态。禁止只返回一个 `success:true` 让 UI 猜测其含义。


## 本文依据

[^R2]: 本会话《最终建议与实施决策书》，[原文](originals/Contexpect_Final_Recommendations_2026-09-11.md)。这是最终咨询取舍，不是仓库已接受的新 ADR，也不是已完成的实现。

[^R1]: 本会话《分环节工程借鉴与整合决策报告》，[原文](originals/Contexpect_Engineering_Reference_and_Integration_Report_2026-09-11.md)。研究快照日期为 2026-09-11；本次仅归档，不重新声明外部项目的最新状态。

[^truth]: Contexpect，[数据与真值模型](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/architecture/data-and-truth-model.md)。用于六个独立 facet、Claim 多轴、Unknown、Receipt 不可变、等价、身份与本地连续性签名。本文第 4、9 章的扩展均为实施建议，不表示这些新增边界已经全部实现。

[^adr6]: Contexpect，[ADR 0006：第三方依赖政策、Effect Lab 估计器与存储偏离的结束条件](https://github.com/Octo-o-o-o/Contexpect/blob/257b7d4d82c0865a94b45d9b3fddcb3830658b32/docs/adr/0006-third-party-dependency-policy-and-estimator.md)。用于当前零第三方依赖决策、SQLite 退出条件、v2 估计器及 Playwright/真实 runner 边界。本文提出的是正式复议建议，不是对此 ADR 的已批准替换。
