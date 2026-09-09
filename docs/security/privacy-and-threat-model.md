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

## 本地 store 的信任边界（已实施部分）

当前 ledger 是 std-only JSON 文档库（ADR 0001 记录的偏离），其信任边界如下，与 `crates/ctxpect-store/src/lib.rs` 的实现同步：

- **连续性密钥**：`store/keys/continuity.key` 是明文 `0600` 文件，与 ledger 同目录。Receipt 的 `local-continuity` 签名与审计链 MAC 都用它。持有该 store 完整本地访问权的人可以**重写整条审计链并重签 Receipt**；这条保证只检测不持有该密钥者的篡改，从不表示组织级 attestation（`org_identity: false`）。外部 SignerAdapter 引入后再升级。
- **审计链**：`audit/events.jsonl` 每条带 `seq`、`prev`、`mac`；编辑、中段删除、重排破坏链接（`audit.mac_mismatch` / `audit.link_broken` / `audit.sequence_broken`）；**尾部截断**由 `audit/head.json` 锚点检出（`audit.truncated`）——锚点记录最后一条的 `seq` 与 `mac` 并自带 MAC，缺失报 `audit.head_missing`、伪造报 `audit.head_mismatch`。被截断或锚点无效的日志**拒绝再写**（`store.audit_truncated` / `store.audit_anchor`），撕裂尾行也拒写（`store.audit_torn`）直到 `store repair` 把尾行移入 `audit/torn-*.bin`。链建立前的旧条目报 `audit.unverifiable_legacy_present`，既不算有效也不算被篡改。边界：持有 store 密钥与完整本地访问权者可整体重写，包括删除锚点后让 `repair` 重锚；这与下一条同一边界。
- **随机源与密钥文件**：密钥只从平台安全随机源生成；源不可用时拒绝生成（`store.random_unavailable`），不退化为可预测值。非 Unix lane 目前没有接线，会 fail-closed——那意味着整个 store 在非 Unix 上不可写（审计需要密钥）。`keys/continuity.key` 以 0600 **独占创建**（没有先按 umask 写再 chmod 的窗口）；读取时若对组/其他用户可读则拒绝使用（`store.key_permissions`）。
- **daemon 无 token、无 Unix socket**：`ctxpect daemon` 是单线程阻塞的 127.0.0.1 HTTP 服务，只靠 loopback 绑定、`Host`/`Origin` 精确匹配与 `X-Ctxpect-Client` 头；同机任何进程都能访问它，也能经 `POST /inspect` 的 `project` / `codex_home` 让它观测任意本机目录并落 Receipt（响应只含摘要）。这与 F-17「随机 token 或 Unix socket」的差距是已知的，身份通道随 ADR 0005 后续决定一起处理。一个慢请求仍会阻塞全部请求，但单次 socket 读有 5 s 超时（空闲连接、preconnect 不再无限期占住 daemon），请求头 64 KiB / 请求体 4 MiB 封顶，请求体按 `Content-Length` 收满后才路由。
- **授权绑定**：每个 mutation 经 `authorize_store_apply(action, target)`，例外记录绑定 action / project digest / target，policy 来源超过 30 天未刷新即 stale；`*` 目标不覆盖控制路径（`.git/`、`.ctxpect/`、store 所在路径 → `projection.control_path`），否则一条 apply 例外能自写 rollback 例外或 git hook；预览持久化并**绑定 project、有消费状态、每次唯一**（跨项目 → `projection.preview_scope`，重放 → `projection.tx_consumed`，再次 preview 不覆盖旧事务的 backup）、`tx.json` 带 `project_digest` 并先于目标落盘、rollback 校验项目与 `after_digest`、`tx_id` 必须匹配 `tx_<16 hex>` 才拼路径；desired 与 backup 内容过 secret 门；导入 metadata-only 不存正文。
- **文件系统写入**：所有写入（projection、assets、store）经**独占创建**的临时文件（`O_EXCL`，pid + 计数命名）+ fsync + rename；目标或临时名上预置的 symlink 被拒绝而不是被跟随（此前固定临时名 + `fs::write` 会跟随敌意仓库里提交的 `.AGENTS.md.ctxpect-tmp -> /任意路径`，把 desired 写到项目外并让目标变成仓外链接）；事务目录及其父目录若是 symlink 一并拒绝；目标是 symlink（含悬空）、FIFO/socket、目录 → `projection.not_a_file`，读取前用 `symlink_metadata` 判定，因此 FIFO 不能挂死进程。
- **secret 门的范围**：`secret_literal` 是**窄的凭据形状文法**（PEM 单行 armor、`AKIA…`、`aws_secret_access_key` 赋值、`ghp_/gho_/ghs_/github_pat_`、`xox*`、含数字的 `sk-` 体、`Authorization: Bearer`），支持 `.env` / YAML / JSON 的引号写法，把 `<your-key>`、`${ENV}`、`{{template}}`、`****`、`YOUR_TOKEN_HERE` 之类占位形状排除在外；`password=…`、数据库 URL 里的口令、64 位 hex token **不在**文法内，不报也不算 Unknown。不可解码为 UTF-8 的字节被替换后仍参与判定（此前一个 Latin-1 字节即可让整个文件跳过全部内容规则）。这些是 §17.3 红线的当前落地，见 [cli-reference「授权绑定」](../guides/cli-reference.md#授权绑定action--project--target)。
- **Receipt 签名覆盖时间**：MAC 覆盖 `manifest.digest + created_at + signed_at`；`manifest.digest` 按 schema `x-display-only` 路径清单（`created_at`、`signature/signed_at`）剥离显示字段，不按键名。`signed_at` / `created_at` 是未受信的本机时钟读数（与上文「Signed Receipt」的 `signed_at` 规则一致）；没有 `signed_at` 的旧格式 Receipt 验签得到 `receipt.signature_legacy`，`signed_at` 存在但不是字符串则是被改过的信封（`receipt.signature_mismatch`），两者都不放行；tombstone 复制原签名，永远不能作为已签 Receipt 验签（`receipt.tombstoned`）。MAC 输入用换行拼接、无域分隔符，同一密钥也用于审计链与标准文档 MAC——三种用途靠输入形状区分，是已记录的 P3 边界。
- **三写与 journal**：Receipt 文件、index、审计链三写之前先落 `store/journal/<op>-<id>.json`，`Store::open` **只在持锁时**重放（能完成的完成、不能判定的列为 in-doubt 并由 `store status` 读出；另一存活进程持锁时不动它正在写的记录）；index 损坏报 `store.index_corrupt`、有 Receipt 而 index 缺失报 `store.index_missing`，都不是空 ledger，`store repair` 从 `receipts/` + `tombstones/` 确定性重建。pending 的 projection 事务只判定不改写（`apply status`）。威胁模型上这意味着：崩溃或部分写入不会让一份 Receipt「消失」在 index 之外，也不会让重试产生重复审计条目；但 journal 与 index 与密钥同目录，持有完整本地访问权者仍可整体改写（同上一条的边界）。
- **同机 advisory lock**：`store/lock` 上的 OS 文件锁（std `File::try_lock`，`flock` 语义）；store 的每个写入方法自己取锁（可重入），mutation 在外层持锁到结束，另一存活进程持锁即 `store.busy`，持有者退出时内核释放，不需要 pid 存活探测（产品 crate 不创建进程）。它防止同机 CLI 与 daemon 交错写坏 index / 审计链，**不是**强制锁，不跨主机或网络文件系统（NFS 上 flock 语义不可靠），也不防同一用户手工删除锁文件（删除后另一进程会在新 inode 上取锁，两者互不感知——已记录的边界）。
- **原生会话导入**：DSH 原生 JSONL 只接受 pinned 版本的未压缩、未打包编码（其它编码拒绝而非部分读）；持久化记录只有类型 / seq / 长度 / digest / 区间与证据 id，`reason` 限于 DSH 词表、header 与 usage 的数值字段只保留整数（此前 `createdAt` / `delegationDepth` / `usage.*` / `reason` 原样透传，畸形输入可借它们把正文带进 store），正文与 `cwd` 等路径只以摘要出现，UI 也不展示正文。解析侧：JSON 嵌套深度上限 128（此前一万层 `[` 会栈溢出 abort daemon）、`sourceEventSeqs` 区间在展开前封顶（此前 `[[0, 2×10^8]]` 分配 1.6 GB）。

## 卸载与删除

卸载默认不删除用户原生配置。删除 Contexpect 数据必须可预览，并列出无法召回的外部 Receipt、系统备份和远端 bundle。删除 session/project/all 后，正文和派生 insight 必须不可查询。

## 实现状态

本威胁模型指导测试夹具与安全门禁。vault、E2EE、外部 signer、daemon 身份通道尚未实施；本地 API、路径包含性、脱敏边界、Doctor 的 secret/hidden-unicode/路径逃逸规则与上一节的 store 边界已有阶段实现（见 [交付状态](../process/2026-09-08-delivery-status.md)）。
