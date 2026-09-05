# 安全、隐私与威胁模型

> 状态：规范（尚未实施产品运行时）

本文冻结数据分类、威胁、vault、签名、egress 和 WebView 边界。同步细节见 [encrypted-sync-protocol](encrypted-sync-protocol.md)。进程边界见 [ADR 0002](../adr/0002-trust-boundaries.md)。

## 数据分类

| 级别 | 例子 | 默认 |
| --- | --- | --- |
| Public metadata | schema、adapter capability、公开包元数据 | 可分发 |
| Local metadata | 脱敏路径、hash、mtime、scope、findings | 可持久化 |
| Sensitive content | instructions、源码片段、session text、tool results | 仅用户选择后进入加密 vault |
| Secret | tokens、私钥、auth headers | 永不进入 Receipt、sync、LLM、SARIF、日志 |

## 威胁

必须防范 PRD §13.1 列出的全部项，包括：prompt injection 与隐藏 Unicode；hook/MCP 任意执行；symlink/TOCTOU/FIFO/压缩炸弹；secret 泄漏到 diff/截图/远端；恶意 registry；renderer 或 adapter 越权读 home；WebView XSS 与 Tauri bridge 提权；本地 API 被其他进程访问；SSH 假冒；历史/LLM 外发；管理员借治理读 prompt；rollback 删除未受管文件。

## Vault 生命周期

- metadata-only 模式不创建、不请求、不解锁 vault key
- 首次持久化敏感正文时生成随机 DEK，用 OS keystore 或用户选择的 age/SOPS recipient 包裹
- 不能安全存储时拒绝开启 vault
- 默认闲置 15 分钟锁定；锁屏/登出/休眠立即锁定
- wrapping-key rotation 只重包裹；content-key rotation 事务重加密并保留 checkpoint
- 备份默认不捆绑解密 key；丢失全部 wrapping key 会永久丢失正文，UI 必须预警
- Unix 私有目录 `0700`，密钥/密文 `0600`；Windows 使用当前用户 ACL。启动时拒绝宽权限 vault

不得声称在 APFS/SSD/备份上可保证物理安全擦除。敏感正文不写普通 OS temp。

## Signed Receipt

- 签名覆盖 canonical envelope：schema、coordinate、claim/evidence digest、unknown、redaction policy、附件 manifest、parent digest、signer id、签名时间
- canonicalization：RFC 8785 JCS；信封：DSSE 或等价公开格式
- SignerAdapter 复用 SSH/GPG/minisign/Sigstore/已有 trust store，不自建设备 PKI
- 本机连续性签名只证明同一安装连续性
- verification 区分 `trusted-valid` / `valid-untrusted-signer` / `revoked-signer` / `invalid` / `attachment-deleted` / `unsupported-schema`
- `signed_at` 默认是未受信本机时间
- 对外使用域隔离 keyed/redacted digest；低熵敏感文件不导出可离线猜测的 digest

## Egress

registry、update、webhook、LLM、sync provider、外部 adapter 共用 allowlist、SSRF/DNS rebinding 防护、redirect 重验、proxy/TLS 和目标认证。默认阻止 loopback 之外的本机地址、link-local、云 metadata 和解析后落入私网的 URL。

## WebView 与不可信内容

instructions、skills、路径、URL、tool result、会话正文全部视为不可信。默认纯文本 viewer。Markdown 预览禁用 raw HTML/script/iframe/危险 scheme。CSP：`default-src 'self'`，无 `unsafe-eval`。Tauri command 显式 allowlist。外链经确认后交给系统浏览器。`ctxpect://` deep link 必须有 versioned schema、上限、授权和 replay-safe nonce。未认证 link 最多打开只读页。

## Team Context Standard 与 leader 视图

Leader compliance 视图默认只含 standard version、compatibility、compliance、drift class、loss/Unknown、exception metadata、device freshness 与 redacted evidence references。默认不得包含 private prompt text、secrets、unrelated personal context 或 full session history。成员 report/upload 前必须先看到 preview/disclosure。TeamContextStandard bundle 不得携带 secret 值。

## 卸载与删除

卸载默认不删除用户原生配置。删除 Contexpect 数据必须可预览，并列出无法召回的外部 Receipt、系统备份和远端 bundle。删除 session/project/all 后，正文和派生 insight 必须不可查询。

## 实现尚未开始

本威胁模型用于指导 WP-01 之后的测试夹具与安全门禁。当前仓库没有 vault、没有本地 API、没有 WebView。
