# T07–T12 决策材料（2026-09-12 交接接续会话）

> 状态：决策材料，不是已批准的 ADR 变更、不是执行授权。
> 边界：本轮只产出决策材料与准入验证表。**不引入任何新依赖**；[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md) 未正式变更前，Rust workspace 零第三方 crate、`Cargo.lock` 无 registry 条目的不变量继续有效，不靠 sidecar / dev-dependency / 独立 workspace 绕行。
> 依据：交接包 `docs/handoff/contexpect-2026-09-11/` 的 09（T07–T12 任务单）与 10（Q01–Q16、停止条件）；本轮 T01–T06 的落地状态见各切片回执。

## T07 依赖政策与单项基础设施升级（P1）

### 现状证据
- 根 workspace 0 第三方 crate；16 条 required gate 离线全绿（2026-09-12 基线运行，退出码全 0，日志 `target/tmp/gate-baseline-2026-09-12.log`，该日志为运行状态记录、不入库）。
- 存储基线为 JSON ledger（ADR 0001 偏离，ADR 0006 决策二保留）。journal / index 修复 / 同机文件锁均已用 `std` 满足。
- `packages/ui` 唯一直接新增 dev 依赖为 `@playwright/test` 1.56.1（ADR 0006 决策三）。

### ADR 候选（建议编号 ADR 0007，未提交批准）
候选议题一：**是否继续维持零第三方依赖**。默认建议维持——本轮 T02/T06 全部切片均未遇到零依赖无法满足的功能需求，ADR 0006 决策二的条件 (a) 未触发。
候选议题二：**SQLite/FTS5 准入核对表**（仅当未来触发条件 (a) 时启用）：

| 准入项 | 验证方式 | 当前状态 |
| --- | --- | --- |
| 功能必要性 | 列出 std 无法满足的具体需求与反例 | 未触发 |
| 离线 vendoring | `cargo vendor` 进仓或本机 registry 镜像；16 条门禁离线复跑 | 未准备 |
| 版本与 features 固定 | lockfile + features 白名单 | 模板见下 |
| 许可证 | Apache-2.0 兼容性核对，NOTICE 更新 | 模板见下 |
| 迁移与回退 | JSON ledger→SQLite 对照读、不长期双写；旧 Receipt 字节不变回归（Q08） | 未实施 |
| 退出机制 | 新 ADR supersede 0006 决策二 + 同步 AGENTS.md | 待批准 |

候选议题三：**前端 lockfile 冻结与 ui-e2e 升级条件**。`ui-e2e` 进 required 表的前提：lockfile 冻结、CI 具备 Chromium 离线镜像、本机实跑记录（本轮 U05/U06 切片已产生本机实跑记录，见各回执）。

### 离线材料（仅登记需求，未获取）
若 ADR 0007 批准引入任何 crate，需先备：vendor 产物与 SHA-256 清单、许可证全文、advisory 时效核对日期、构建 features 清单。本轮未获取任何离线材料，因为条件 (a) 未触发。

## T08 外部报告与转换适配（P1）

### 决策点（待 owner/架构决策）
1. APM 等只读产物导入的合同：来源字段（producer/collected_at/quality）、退出码映射（外部进程未启动/非零/解析失败/partial/无命中分别处理，03 号文档 §8）、I/O 与网络边界（默认不外联）。
2. Rulesync 受限候选生成：显式隔离候选、禁止直接写真实目录、真实转换需执行授权。
3. 外部 `safe/verified/success/exact` 标签按字段重新解释（02 号文档 §4 五项整合细化），不继承其 home/网络/hooks/备份/log 默认行为。

### 前置状态
T02 已落地（本轮）；T05（Doctor 输入与规则收口）的声明校验标注已由 C-F04 切片部分落地（`declaration-validation` 标记）。真实转换执行授权未取得——本轮不做。

## T10 团队信任与加密同步（P1/P2）

### 决策点
- 密钥分用途：软件更新签名 / Receipt 签名 / 团队标准签名 / 加密密钥分立（10 号文档 §7）；本地 HMAC 只标 local-continuity，不冒充组织证明。
- bundle 合同：不可变、经授权；不传 hot DB/WAL、私钥、token、raw session、明文备份；验签解密顺序按具体封装合同，不预设通用顺序。
- 分叉协调：同父 revision 分叉不自动 LWW、不自动覆盖（Q11）；DEMO-11 的 UI 表达已有合同（07 号文档 V08）。
- 撤销材料新鲜度与例外 scope/期限分开验证。

### 前置状态
T06（安全变更端到端）核心链路本轮已被 Codex 纵向闭环切片覆盖（preview 冻结→授权重验→一次性消费→journal/备份→回滚→post-Receipt，product_loops.rs `codex_static_to_safe_write_loop_closes_end_to_end`）；T07 相关基础未触发。双设备实际验证未做——需要第二设备/环境授权。

## T11 历史、Advisor 与 Effect（P2）

### 决策点
- Advisor 最小发送：授权记录格式、发送前预览合同（/advisor 双确认已实现并本轮补 e2e）；LLM 建议不进入 Claim provenance/policy/CI/baseline（不变量 3）。
- 历史洞察：派生分析绑定 receipt_id、规则版本、评估时间（C-F05 已落地 as_of 机制）；删除传播的失效与 tombstone（C-F08）。
- Effect 真实 runner：ADR 0006 决策四明确"首次运行另行授权"；v2 估计器需独立数值与适用性验证（10 号文档 §6）；连续指标不得套二项判据。

### 前置状态
T04 证据桥部分落地（Codex 闭环，运行面保持 Unknown）；T07 未触发。runner 授权与付费上限未取得——本轮不做。

## T12 全矩阵与发行收口（P1/P2，持续准备）

### 准入验证表（发行前逐项核对，当前全部为"未执行"）

| 准入项 | 依据 | 当前状态 |
| --- | --- | --- |
| 冻结 required cell 全矩阵 | acceptance/ 八份工件 + compatibility-matrix | 未执行全矩阵适配 |
| 适用 OS lane | WP-12 | 仅 macOS 本机 |
| 恢复/安装/升级/卸载 | WP-12 | 未执行 |
| 72h soak | WP-12 | 未执行 |
| 人工验收与独立 readback | WP-12 | 未执行 |
| 发布包测试能力排除（驱动端口/mock/调试身份/后端旁路） | Q14 | 未执行；ui-e2e 种子仅在 `tests/` 下，发布构建排除需验证 |
| Release notes 准确性 | WP-12 | 未执行 |

## 本轮明确不做的事
- 不变更 ADR 0006；不引入任何新依赖；不获取离线 vendor 材料（条件未触发）。
- 不执行真实外部 runner、不部署组织身份、不做付费实验（各自需要授权）。
- 不把阶段状态写成"完整交付"；上表"未执行"项不得改标签消除缺口。
