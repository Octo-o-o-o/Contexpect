# Adapter 编写指南

> 状态：规范（尚未实施产品运行时）
> SDK crate 将在 WP-01/WP-02 落地；本文件先冻结作者合同。

## 你在实现什么

一个 adapter 不是“把某工具标成支持”。它必须为每个声明 cell 提供：

1. 静态 collector/resolver 或明确的 Unknown honesty
2. 若有原生 surface：importer + field-to-claim mapping
3. development fixtures（include/exclude/override/cap/ignore/conditional/progressive/legacy/fs-edge/negative）
4. 未知版本 fail-closed 测试
5. 仅当 oracle 可重复时：oracle recipes

## Manifest 最小字段

```text
family_id, display_name, version_range, surface, os
collectors[], resolvers[], importers[]
capabilities[] -> status, mapping, collector, importer
native_oracles[] -> command argv, coverage, repeatable
sources[] -> url, retrieved_at, digest, license
sandbox[] -> paths, net, secrets
```

`repeatable: true` 会把该 oracle 带入 ≥12 case 的验收分母。不要为了看起来完整而打开它。

## Field-to-claim 规则

- 原生字段不自动等于 ModelVisible
- “discovered configuration” 默认只证明 discoverable
- 覆盖范围必须写清；Codex prompt-input 不能标成完整 wire payload
- 每个字段给出 lifecycle_stage、claim_kind、coverage、最低 provenance

本仓库的映射草稿在 `acceptance/field-to-claim/`。它们是合同，不是已验证的运行时解析器。

## Fixture 作者规则

- 合成数据，Apache-2.0，记录 digest
- 不使用真实 secret；secret 规则使用文档化 fixture token
- 不把厂商不能再分发的会话正文放进 development corpus
- sealed 答案不得写入实现者可见树
- case id 在 development/sealed/live 之间不重叠
- `live_tested` 只有真正执行过声明命令且保存脱敏 evidence 时才能为 true

## 能力声明示例

Aider：`skills`/`plugins` 为 `not-applicable`。  
Codex/Grok：独立 `scoped-conditional-rules` primitive 为 `not-applicable`，投影必须显示 loss。  
跨 family 对齐必须输出 harness-native 产物（例如 Codex nested `AGENTS.md`、Claude path-scoped rules、Cursor `.cursor/rules`、Grok 项目指令）。不得把字节相同的一份文件当作 `native-equivalent`。  
Coze：大多数 prompt-parity 能力 `not-applicable`；实现 connector schema/status/JSON。

## 测试入口（未来 SDK）

实现后 adapter SDK 必须提供：

- `ctxpect adapter test --manifest <path>` 跑 fixture runner
- golden Receipt 比较（忽略时间字段）
- capability 完整性：taxonomy 类别不得无状态缺席
- sandbox：越权读 home / 出站网络必须失败

当前不要添加假的 SDK crate。

## 禁止

- 一个 `enabled: true` 吞掉 surface/version/install/auth
- 拦截 HTTPS 作为默认采集
- 解析不稳定的人类终端文本作为唯一接口
- 把 README 指标当作 Contexpect truth
- 为了让 corpus 变绿修改产品含义
