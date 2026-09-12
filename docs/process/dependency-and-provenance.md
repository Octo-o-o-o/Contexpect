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

## 本副本已引入（前端，不进入 Cargo workspace）

Rust workspace 仍为零第三方 crate，十二条非前端 required gate（含 cargo 三条与 corpus/doctor/native conformance 三条）保持离线；crate 间新增的边只有 `ctxpect-projection → ctxpect-doctor` 与 `ctxpect-assets → ctxpect-doctor`（两个 executor 共用 secret 门）。下列 Node 包只用于 `packages/ui`，license 与版本以 lockfile 为准（安装后登记 digest）：

| 名称 | 版本 | SPDX | 来源 | lockfile integrity |
| --- | --- | --- | --- | --- |
| react | 18.3.1 | MIT | npm registry | `sha512-wS+hAgJShR0KhEvPJArfuPVN1+Hz1t0Y6n5jLrGQbkb4urgPE/0Rve+1kMB1v/oWgHgm4WIcV+i7F2pTVj+2iQ==` |
| react-dom | 18.3.1 | MIT | npm registry | `sha512-5m4nQKp+rZRb09LNH59GM4BxTh9251/ylbKIbpe7TpGxfJ+9kv6BLkLBXIjjspbgbnIBNqlI23tRnTWT0snUIw==` |
| react-router-dom | 6.30.1 | MIT | npm registry | `sha512-llKsgOkZdbPU1Eg3zK8lCn+sjD9wMRZZPuzmdWWX5SUs8OFkN5HnFVC0u5KMeMaC9aoancFI/KoLuKPqN+hxHw==` |
| vite | 6.0.11 | MIT | npm registry | `sha512-4VL9mQPKoHy4+FE0NnRE/kbY51TOfaknxAjt3fJbGJxhIpBZiqVzlZDEesWWsuREXHwNdAoOFZ9MkPEVXczHwg==` |
| typescript | 5.7.3 | Apache-2.0 | npm registry | `sha512-84MVSjMEHP+FQRPy3pX9sTVV/INIex71s9TL2Gm5FG/WG1SqXeKyZ0k7/blY/4FdOzI12CBy1vGc4og/eus0fw==` |
| @vitejs/plugin-react | 4.3.4 | MIT | npm registry | `sha512-SCCPBJtYLdE8PX/7ZQAs1QAZ8Jqwih+0VBLum1EGqmCCQal+MIUqLCzj3ZUy8ufbC0cAM4LRlSTm7IQJwWT4ug==` |
| @playwright/test（dev，可选门禁 `ui-e2e`，[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md)） | 1.56.1 | Apache-2.0 | npm registry | `sha512-vSMYtL/zOcFpvJCW71Q/OEGQb7KYBPAdKh35WNSkaZA75JlAO8ED8UN6GUNTm3drWomcbcqRPFqQbLae8yBTdg==` |

登记于 `pnpm-lock.yaml`（lockfileVersion 9.0）。pnpm 11 不再读取 `package.json` 的 `pnpm.onlyBuiltDependencies`；esbuild 的 lifecycle 许可写在 `pnpm-workspace.yaml` 的 `allowBuilds.esbuild: true`。`confirmModulesPurge: false`（同键也写在 `.npmrc` 的 `confirm-modules-purge=false`）避免无 TTY 时因重建 `node_modules` 而中止。传递依赖 `esbuild@0.24.2` SPDX MIT，lockfile integrity `sha512-+9egpBW8I3CD5XPe0n6BfT5fxLzxrlDzqydF3aviG+9ni1lDC/OvMHcxqEFV0+LANZG5R1bFMWfUrjVsdwxJvA==`；`@playwright/test` 的传递依赖 `playwright@1.56.1`（`sha512-aFi5B0WovBHTEvpM3DzXTUaeN6eN0qWnTkKx4NQaH4Wvcmc153PdaY2UBdSYKaGYw+UyWXSVyxDUg5DoPEttjw==`）与 `playwright-core@1.56.1`（`sha512-hutraynyn31F+Bifme+Ps9Vq59hKuUCz7H1kDOcBs+2oGguKkWTU50bBWrtz34OUWmIwpBTWDxaRPXrIXkgvmQ==`）均为 Apache-2.0；`ui-e2e` 所用 Chromium headless shell 由 Playwright 联网安装到用户缓存目录，不入库、不进发布物。lockfile 同时新增可选传递依赖 `fsevents@2.3.2`（MIT，仅 darwin，Playwright/vite 的文件监听），因此就 lockfile 而言本次新增 4 个包，就 `package.json` 直接依赖而言只新增 `@playwright/test`。Rust workspace 仍无 crates.io 依赖。

九项 integration（APM / Agentpack / agentsync / CtxWise / Scopeon / ctxray / ContextSpy / age-or-SOPS / SignerAdapter）的 `version_pin` 仍为 `evidence-backed-unavailable`。缺 pin 时：只读展示 / Unknown / 拒绝 vault-required sync；不编造 pin，不内置替代 authority。

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

## 2026-09-12 开发期外部运行 pin

[ADR 0007](../adr/0007-external-age-ssh-and-runner-adapters.md) 登记 age 1.3.2（BSD-3-Clause）官方包/二进制摘要与 OpenSSH SSHSIG 来源。它们由本地 profile pin，未作为 Rust crate 或随包工具分发；不追溯修改 cutoff 的 `evidence-backed-unavailable` 矩阵。Effect 的现有 `paired-exact-binomial-v2` 仍依据 ADR 0006；上文“统计未 pin 则不能 supported-*”不适用于该已登记实现。外部 runner 的 digest/协议与观察结果由每个冻结请求绑定。
