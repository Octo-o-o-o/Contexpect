# ADR 0003：加密同步与签名

> 状态：已接受（规范冻结；尚未实施产品运行时）
> 日期：2026-09-04

## 决策

- Bundle 加密直接复用 **age 或 SOPS** 中在 WP-07 开工前选定并 pin 的一个成熟可互操作格式
- Recipient 私钥留在 OS keystore / 用户已有 secret manager / 外部工具
- 不自建设备 PKI、恢复码协议、自定义版本图或 CRDT
- Receipt 与 sync envelope 使用 RFC 8785 JCS canonicalization，以及 DSSE 或等价公开签名信封
- SignerAdapter 复用 SSH signing、GPG、minisign、Sigstore 或组织已有 trust store
- Git provider 委托 Git history/merge/signing；folder provider 只用单调 generation + parent digest + latest pointer

在 age 与 SOPS 之间的最终库 pin 属于 WP-07 开工检查项，但不能改成自研算法。

## 后果

没有可安全存储 wrapping key 时必须拒绝 vault 与 sync。移除 recipient 只限制未来访问。已下载历史不能远程擦除。
