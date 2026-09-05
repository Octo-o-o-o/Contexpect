# 依赖、出处与 SBOM

> 状态：规范（尚未实施产品运行时）

## 原则

- 核心 scanner、resolver、Receipt、CLI、adapter SDK、local UI、deterministic Doctor 使用 Apache-2.0
- 引入依赖前核验 license、维护状态、security policy、release pin 和传递许可
- 无许可证或许可不兼容的代码不得进入 core
- Apache-2.0 第三方必须保留 NOTICE/版权行
- GPL-3.0 不得嵌入拟采用宽松许可的核心
- README 指标不是 Contexpect 的 truth

## 发布时 SBOM

每个产品发布和 adapter/corpus 更新包必须同时提供：

- CycloneDX
- SPDX reference
- 锁定 digest
- 生成器与其版本

CI 在 WP-11 将 `ctxpect assets` / 发布脚本与 APM SBOM 生命周期对齐。Package 域的 license/digest/SBOM **唯一 authority 是 APM**；Contexpect 只展示引用。

## provenance 记录

凡进入仓库或发布物的第三方材料，在 [NOTICE](../../NOTICE) 登记：

名称、版本、SPDX、URL、retrieved-at、digest、再分发许可、脱敏状态。

Corpus 中的厂商文档与真实会话尤其如此。不能公开再分发的样本只保存本地 recipe 或 metadata。

## 运行时依赖方向（尚未 pin）

| 能力 | 方向 | 未 pin 时 |
| --- | --- | --- |
| 包管理 | APM | 只读展示 / Unknown |
| 投影 executor | 冻结的 APM/Agentpack/agentsync 或 contexpect-native | 无合格 executor 则 export-only（required-write cell 除外，那些必须有 executor 否则验收失败） |
| Codex 专项 | CtxWise Apache-2.0 | bundled static resolver，标明 Observed 缺口 |
| 加密 | age 或 SOPS | 拒绝开启 sync/vault |
| 签名 | SSH/GPG/minisign/Sigstore | 仅本机连续性签名，且不冒充组织身份 |
| 统计 | 冻结的成熟库 | Effect Lab 不能给出 supported-* |

具体 pin 见 `acceptance/integration-contracts.yaml`。APM / Agentpack / agentsync / CtxWise / Scopeon / ctxray / ContextSpy / age-or-SOPS / SignerAdapter 在本机研究 Receipt 中只有 GitHub URL、没有 cutoff tag/digest，因此 version_pin 为 `evidence-backed-unavailable`，禁止编造 pin。

## 供应链安全

安装/更新锁定不可变 commit/digest。tag 漂移触发 drift。恶意 fixture 必须被阻止或清楚警告。无许可证内容不能复制进核心。
