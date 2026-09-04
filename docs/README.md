# Contexpect 文档索引

> 当前仓库仍处于产品定义阶段；尚未开始代码实现，也未初始化 Git。

## 主文档

- [完整产品需求方案](requirements/2026-09-04-contexpect-complete-product-requirements.md)：产品边界、用户与使用节奏、功能合同、架构、安全、工作包和验收。
- [研究台账](research/2026-09-04-context-management-research-ledger.md)：互联网调研、竞品/开源复用判断、错误路线、被否决的设计和命名结论。
- [用户方案来源快照](research/source-snapshots.md)：两份附件的路径、SHA-256 与 26 条命题追踪。
- [本机环境 Receipt](research/2026-09-04-local-environment-receipt.md)：本次需求基线使用的 OS、硬件、四个工具版本与实测命令输出。
- [独立评审记录](review/2026-09-04-contexpect-requirements-review.md)：三轮只读评审、RED 结论、修订映射和最终状态。
- [评审输入合同](plan/2026-09-04-requirements-review-prompt.md)：独立 reviewer 使用的范围与检查口径。

## 阅读顺序

先读完整产品需求方案；需要核实依据或避免重复踩坑时读研究台账和来源快照；进入实施前先读评审记录，并按需求 §17.0 生成、冻结和独立评审 acceptance artifacts。

## 当前判定

需求已根据第三轮评审完成最后一轮修订和机械一致性检查，但没有第四次独立复审，因此不能声称获得独立 GREEN。项目只有在需求所定义的一次全产品集成门禁绿与一次独立 readback 绿后，才能宣布完整交付。
