# 治理

> 状态：规范（尚未实施产品运行时）

Contexpect 是 Apache-2.0 开源项目。本文件描述维护、决策和验收合同修订方式。它不建立多租户公司控制面。

## 角色

| 角色 | 职责 |
| --- | --- |
| Contributor | 提交 PR、issue、夹具与文档 |
| Maintainer | 合并变更、发布、安全响应、合同 revision |
| Adapter reviewer | 审核 family/version/surface 清单与 Unknown honesty |
| Release manager | 签名发布、SBOM、revocation denylist |

项目尚无独立基金会。初始 maintainers 为仓库所有者；公开发布前应在本文件列出至少两名联系人。

## 决策

日常技术决策通过 PR 讨论。下列事项需要书面 ADR 或 acceptance-contract revision：

- 运行时栈变更（已冻结：Rust workspace + Tauri 2 + React/TypeScript + SQLite/FTS5）
- 信任边界、加密格式、签名信封
- 新增 adapter family 或把 smoke 版本升级为权威 baseline
- 把 required-write projection cell 改为其他 authority
- 把 required-unknown-honesty 改成 required-supported

不得用 code freeze 日期悄悄扩大 cutoff 之后的 harness/OS 范围。Acceptance cutoff 为 `2026-09-04T23:59:59+08:00`。

## 验收合同

八份 §17.0 工件是后续工作包的硬前置。修订流程：

1. 新建 contract revision 号并说明原因。
2. 更新 `acceptance/` 与 `acceptance/traceability.csv`。
3. 重跑四条 foundation 门禁及所有受影响后续 gate。
4. 独立 reviewer 核对；实现者不得为消除失败而改答案。

## 适配器与 corpus

- 18 个 family 是完整交付声明范围，不是每个 surface 都有 Native evidence。
- development / sealed / live 三套 corpus 的 case id 不得重叠。
- sealed 答案在 RC 冻结前不对实现者开放。
- 实现代码不得对 sealed/live 答案写特判。

## 行为与安全

[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) 与 [SECURITY.md](SECURITY.md) 优先于本文件的社交约定。CoC 执行与安全升级由 maintainers 负责。

- Canonical private channel：https://github.com/Octo-o-o-o/Contexpect/security/advisories/new
- Public fallback：https://github.com/Octo-o-o-o/Contexpect/issues/new ，标题 `Security contact request` 或 `Conduct report request`，只写非敏感摘要并请求私有讨论。不要在公开 issue 中披露 secret、利用细节或未脱敏日志。

本项目不使用安全邮箱作为上报渠道。

## 商标

`Contexpect`、`ctxpect` 与 “Expected. Observed. Reconciled.” 在公开发布前必须完成 GitHub、crates.io、npm、PyPI、域名和商标清查。本文件不构成商标授权。
