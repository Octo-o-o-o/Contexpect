# ADR 0007：外部 age / SSH 与 runner adapter

> 状态：实施中的决策；依据 owner 对 WP-07 / WP-10 继续实施和运行的授权。
> 日期：2026-09-12

## 决策

延续 ADR 0003 的外部工具路径：加密采用 age 1.3.2 格式；发送者真实性使用 OpenSSH SSHSIG 的独立命名空间 `ctxpect-sync-v1`。不实现密码学原语，不引入第三方 Rust crate，不复用本地 HMAC 作为远端信任。ADR 0006 的 Cargo 零第三方依赖约束保持；这里没有以 sidecar 包装自研密码学或规避依赖准入。

- age 官方 darwin-arm64 包 SHA-256：`e2020b073c44f692685a24d6abc378817eb81ffaaf49fd0531ef8565f767f2f5`；其 age 可执行文件 SHA-256：`4012dfc2725883beafb710894af4f599b7a94f8c8e0f51f02cc96ab8df33915e`。
- age 版本为 cutoff 后的开发期接入 pin，不改写冻结 integration/oracle 矩阵，不声称它已通过全部 required OS lane。
- 工具路径、二进制摘要、版本、recipient 集合和 allowed-signers/revoked-signers 由本地 profile 明确指定；远端 bundle 不得指定命令、密钥文件或信任文件。
- recipient 私钥与 SSH signing key 只保存路径引用，由外部工具管理；生产 profile 必须使用用户认可的密钥托管方式。验收用临时生成的测试密钥，不读取用户密钥。
- SSHSIG 覆盖完整的纯字符串/整数 envelope canonical JSON（schema、generation、parent、recipient、sender、ciphertext digest 和 ciphertext）。该受限数据模型避开 IEEE754 数字规范化的未实现范围。
- 同步 transport 与静态/原生语义核对分别记账；收取密文不产生 native Claim。

Effect 采用 versioned 外部 command adapter 协议；Contexpect 只管冻结输入、调用边界、budget/timeout、结果校验与现有估计器，不实现 agent 或 sandbox。adapter 必须自己提供隔离环境/门禁与 executor receipt；仅指定 cwd 不得宣称操作系统 sandbox。

## 进程边界

此前全仓禁止进程创建的静态护栏收窄为：仅 `ctxpect-cli/src/tool_process.rs` 可创建进程，调用者只准 CLI dispatch 及三种显式执行 adapter。其余产品源文件仍禁止进程创建及引用 executor，`ctxpect-collect/tests/passive_scan.rs` 全量遍历执行此检查。普通 inspect/静态 collector 不调用外部进程；新增执行路径要求显式参数与 policy 授权。受控 executor 清空环境、限制输入/输出、超时并清理 Unix 进程组，不能作为不可信程序的 OS sandbox。

## 来源与许可

[age 1.3.2 官方发布](https://github.com/FiloSottile/age/releases/tag/v1.3.2)、[age 格式与 CLI](https://github.com/FiloSottile/age)、[OpenSSH ssh-keygen 手册](https://man.openbsd.org/ssh-keygen)。age 为 BSD-3-Clause；本仓不再分发下载的可执行文件，发布打包若携带它必须补充其 LICENSE 与平台 pin。OpenSSH 来自系统，不重新打包。

## 验收边界

运行工具缺失、摘要变化、签名不可信、recipient 撤销、协议漂移必须拒绝；不能退回明文同步。测试 profile 的双端运行证明软件链路，不代替组织身份、真实多机、独立安全审查或正式发行签名。
