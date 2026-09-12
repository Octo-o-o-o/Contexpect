# 加密同步协议

> 状态：规范；2026-09-12 已有外部 age / SSHSIG 的 CLI transport 切片，完整 WP-07 尚未实施完毕、未验收。
> 算法选择见 [ADR 0003](../adr/0003-encrypted-sync-and-signing.md)。

## 目标

在不自建 Contexpect 云账号的前提下，把 allowlist 内的非秘密资产以端到端加密 bundle 在设备间移动，并区分 **transport success** 与 **semantic reconciliation success**。

## 传输中立

Core 定义 bundle 格式与 provider contract。完整交付内置：

- 本地文件夹 provider：可放入用户已有的 iCloud Drive、Dropbox、Syncthing 目录
- Git-backed provider：直接使用 Git 的 commit/merge/signing

其他云/WebDAV/S3 只能通过 provider adapter。Core 不实现通用传输客户端。

## Bundle 内容

同步按 asset allowlist。分类：`local-only` / `project-shared` / `device-group` / `never-sync`。Secret 只保存引用。项目资产优先引用 Git source，不得把 Git 已管理正文无理由复制进第二套库。

远端只能看到 opaque blob、最少 metadata 和版本。

## 加密与真实性

- 加密复用 age 或 SOPS（ADR 0003 选定的成熟可互操作格式）
- recipient 私钥由 OS keystore、用户现有 secret manager 或外部工具管理
- 不发明密钥交换、设备 enrollment 或恢复码协议
- confidentiality 由 AEAD 提供；sender authenticity 与 metadata 完整性由 SignerAdapter 对 envelope 签名提供
- header、generation、parent digest、recipient set、schema、ciphertext digest 均在签名覆盖内

## 版本与分叉

每台设备保存最近接受的 signed head 和单调 generation。必须拒绝或冲突化：

- 旧 generation（replay）
- 父链断裂
- 未知/不可信 signer
- 已从 recipient set 移除的写入

无透明日志时不承诺阻止服务器 equivocation。观察到同一父节点的不同 signed heads 必须显示分叉，禁止静默选择。

## Provider 语义

| Provider | 历史 / merge | 冲突 |
| --- | --- | --- |
| Git-backed | 完全委托 Git | Git merge/signing |
| Folder | 只维护单调 generation、parent digest、latest pointer | 同父分叉进入 conflict；用户选择 head、导出双方或转交 Git/external authority；禁止自动 merge |

文件夹 provider 不实现自定义 DAG、CRDT 或三方 merge。

## Recipient 生命周期

- 加入：新 recipient 默认只能解密加入后的 bundle；历史 re-encrypt 必须显式选择
- 移除：轮换后续 bundle key；被移除设备不能解密未来 bundle
- 已经下载的历史不能被收回；产品不得声称远程擦除
- 端点失陷可暴露当时可访问的数据；不宣称对已失陷端点提供前向保密
- 丢失全部 recipient key 可能永久失去数据，UI 必须预警

## 同步流程

1. 设备 A 选择非秘密资产，生成加密 bundle 或 Git-backed manifest
2. 设备 B preview：新增、冲突、有损、无法支持
3. 用户选择 resolution；原子应用并保留备份
4. 在设备 B 用目标 harness 生成 Receipt，按 EquivalenceProfile 做语义核对
5. UI/CLI 分别显示 transport 结果与 `verified` / `structural-only` / `indeterminate` / `failed`

## 失败语义

| 事件 | 结果 |
| --- | --- |
| transport 成功但 Observed 不可得 | `structural-only`，不得显示跨设备完全一致 |
| 缺 resolver/policy/identity/version | `indeterminate` |
| 未批准 loss 或 transaction digest 不匹配 | `failed` |
| 本地 keyed digest 不可比较 | `indeterminate`，禁止用正文相似度替代 |

## TeamContextStandard 传输

签名的 Team Context Standard 必须能经 git/file 本地分发。把同一 bundle 放进已有加密云/文件夹 provider 只是可选传输，不得成为语义依赖，也不得把 transport success 写成 semantic reconciliation success。

## 当前 CLI transport 切片

`sync seal|preview|apply|status --adapter age-ssh-v1 --profile <local-profile.json>` 使用 [ADR 0007](../adr/0007-external-age-ssh-and-runner-adapters.md) 的外部 age / SSHSIG。`seal` 只接受 `ctxpect-sync-assets-v1` 中显式 `device-group` 的 instruction 资产；需 `sync.seal` policy grant。导出文件要求新路径，不覆盖已有文件，也不推进发送端 head。发送端须先 preview/apply 自己的导出，才能继续下一代。

接收端验证完整签名、当前 recipient/epoch、信任有效期及撤销，再在内存解密。`preview` 给出 15 分钟的 `tx_id`，绑定 profile、当前 head 和输入；`apply --tx <id>` 需 `sync.apply` grant，并重新核验后原子存储密文 head 与资产大小/id 元数据，保存上一代密文。正文不写入 store。双 profile、未来 recipient 撤销、篡改、分叉、过期预览测试在 `scripts/check_secure_sync.py`。

本地 profile 显式指定 group、epoch、trust_valid_until、recipients、signers（principal→recipient）、sender、identity/signing_key/allowed_signers/revoked_signers 路径，以及 age/ssh_keygen 的绝对路径和 SHA-256。profile 不从远端导入；临时测试 profile 的完整构造见上述脚本，不应把测试授权复制到生产。

## 保留工作

该切片未完成 native projection、跨设备语义 Receipt、Git provider、组织 enrollment、OS keystore 接入或完整资产类型。Web Sync 页展示接收状态和边界，加密操作仍走 CLI。收取密文只算 transport，不是完整 WP-07 验收。
