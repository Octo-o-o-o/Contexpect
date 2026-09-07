# CLI 参考

> 状态：规范（`ctxpect inspect` 开发切片以及 doctor/collect/receipt/diff/daemon 等命令已可构建；`--config`/`--privacy`/`--sarif` 等仍 fail-closed。完整产品运行时尚未实施。）
> 二进制 `ctxpect` 的 inspect 仍输出 `schema=dev-inspect-v0` / `receipt_kind=development-snapshot`。正式 Receipt 需 `migrate_dev_inspect_v0`。

机器字段默认英文。人类输出可本地化。JSON envelope 稳定、可版本化。

## 全局

```text
ctxpect [--json] [--offline] [--config <path>] [--project <dir>] [--cwd <dir>]
        [--harness <family>] [--surface <id>] [--version <ver>]
        [--privacy default|strict|metadata-only]
```

`--offline` 下核心 inspect/diff/doctor/receipt/history 必须可用。未知版本不能通过 `--force` 变成权威 pass。

## 命令树

| 命令 | 工作包 | 作用 |
| --- | --- | --- |
| `ctxpect inspect` | WP-02 | one-shot 探测 + development snapshot；`--store` 时显式迁移为正式 Receipt |
| `ctxpect collect` | WP-02 | 在目标环境采集签名/脱敏 Receipt，供离线导入 |
| `ctxpect doctor` | WP-02 | 确定性 Doctor；`--sarif` / `--fail-on` |
| `ctxpect diff` | WP-03 | 跨 harness/device/version/snapshot |
| `ctxpect receipt show\|verify\|export\|redact` | WP-03 | Receipt 生命周期 |
| `ctxpect inventory` | WP-02 | 资产目录，不含动态伪事件 |
| `ctxpect preflight` | WP-02 | task probe；生成可审计 argv 数组 |
| `ctxpect launch` | WP-02 | 用 structured process API 启动 harness |
| `ctxpect import` | WP-05 | 原生诊断/日志/stdin |
| `ctxpect sessions` | WP-05 | timeline；未暴露类型只报 capability unknown |
| `ctxpect daemon start\|stop\|status` | WP-05 | 可选 |
| `ctxpect sync preview\|apply\|status` | WP-07 | transport + reconciliation 分列 |
| `ctxpect intent validate\|show\|project\|preview` | WP-06 | CanonicalIntent；harness-native projection |
| `ctxpect apply` / `ctxpect rollback` | WP-06 | 唯一 authority；无合格 executor 则只导出 plan |
| `ctxpect standard validate\|publish\|preview\|adopt\|pin\|update\|status\|leave\|rollback\|revoke` | WP-07 | TeamContextStandard；git/file local-first |
| `ctxpect exception request\|approve\|reject\|revoke\|status` | WP-11 | 受控例外；过期 fail-closed。`--role`/`--actor` 仍是调用方自报、不构成授权；身份来自仓内已登记 principals（见「例外批准的身份源」） |
| `ctxpect assets [status\|list\|preview\|copy\|rollback\|sbom]` | WP-08 | catalog、资产复制 executor、lock 与 SBOM；无子命令时为概览。复制前先核验登记、许可证与内容摘要 |
| `ctxpect advisor` | WP-09 | 显式同意、payload preview |
| `ctxpect experiment` | WP-10 | Effect Lab |
| `ctxpect policy eval\|show` | WP-11 | 分层 context policy；detect-only 诚实性 |
| `ctxpect align status\|diff` | WP-06, WP-11 | 语义 drift / loss / unknown；不得把 byte-equality 当 pass |
| `ctxpect adapter test\|list` | WP-11 | SDK fixture runner |
| `ctxpect ci` | WP-11 | 稳定 exit code 的 CI 入口 |

## Exit codes

| Code | 含义 |
| --- | --- |
| 0 | policy pass：所有 required checks 确定通过 |
| 2 | 确定性 drift/finding failure（含可阻断 Doctor 规则） |
| 3 | required indeterminate / 未知版本 / 过期 policy / 无法刷新 |
| 1 | 用法/IO/内部错误 |

Unknown 不得自动映射为 0。组织可以把 indeterminate 降为非阻断，但必须写入 Receipt，且不改变 claim。

命令特定错误写入 JSON envelope 的 `error.code`（例如 `align.byte-equality-not-semantic`、`standard.unsigned`、`exception.expired`、`principal.secret_mismatch`、`exception.approver_role_required`），不新增第五种 exit code。`align` 把 byte/hash 相等当作语义通过时必须非 0。

`policy show`（无 `--from`）读取与 `apply` 相同的 store policy / exception 集合，并调用同一 `authorize_mutation` 判定。无 layers 时 `verdict=unknown`、`reason_code=policy.unknown`，不是 pass。`policy eval --from <file>` 只评价该文件，不替代 store 判定。

## 例外批准的身份源：仓内已登记 principals

> 选型理由、被否决的替代方案与 mutation 门禁边界见 [ADR 0005](../adr/0005-exception-identity-and-mutation-boundary.md)。

`ctxpect exception approve|reject|revoke` 不把调用方自报的 `--role` / `--actor` 当作授权证据。身份的唯一来源是**项目内受版本控制的 principals 登记**，配合调用方实际持有的登记密钥。

### 登记文件

`<project>/.ctxpect/principals.json`，随项目进版本控制，因此「谁可以申请、谁可以批准」与代码在同一处评审：

```json
{
  "schema": "ctxpect-principals-v1",
  "principals": [
    {
      "principal_id": "alice",
      "roles": ["requester"],
      "key_id": "k-alice",
      "key_digest": "<enrollment digest>"
    },
    {
      "principal_id": "carol",
      "roles": ["approver"],
      "key_id": "k-carol",
      "key_digest": "<enrollment digest>",
      "self_approval": false
    }
  ]
}
```

`roles` 只接受 `requester` 与 `approver`；出现其它取值时整条登记被拒（`principal.role_unknown`），不静默忽略。

### 登记摘要与密钥

`key_digest = HMAC-SHA256(secret, "ctxpect-principal-enrollment-v1:" + principal_id)`。摘要绑定到 `principal_id`，因此一份登记不能被挪用给另一个 principal。

密钥本身**不入库、不进 argv**，只经环境变量 `CTXPECT_PRINCIPAL_SECRET` 传入：argv 在进程列表里可见，也会进入 redaction roots，两处都不该出现 secret。

**登记摘要随项目公开**，因此 `secret` 必须是高熵随机材料；低熵口令可以由公开摘要离线暴力破解。

### 身份与授权

```bash
CTXPECT_PRINCIPAL_SECRET=<secret> ctxpect exception request \
  --project <dir> --store <dir> --id ex-1 --principal alice
```

- `request` 的授权来自 principals 登记，**不来自已批准例外**。否则创建第一条例外需要先有一条已批准例外，整条通道自锁。
- `approve` / `reject` / `revoke` 要求 `approver` 角色。
- 默认四眼：请求者不批准自己的请求，除非该 principal 在登记里显式写了 `"self_approval": true`。
- 状态机终态即终态：`requested → approved|rejected`、`approved → revoked`；已 `rejected`/`revoked` 的例外不能被复活（`exception.transition_invalid`）。
- 决策写入 `decided_by`，其中 `identity_kind = "enrolled-local-principal"`、`org_identity = false`。持有本地登记密钥不是组织身份，输出不会把它说成组织身份。

### daemon API 不提供此通道

`POST /api/v1/exceptions` 返回 `api.identity_required`。HTTP 请求不能安全携带登记密钥，因此 daemon 无法建立所需身份；它明确失败，而不是造一条 requester 为字面量 `user` 的例外。

## 资源限制哪些真的生效

`settings.resource_limits` 的两个字段处理方式不同，因为一个能执行、一个不能：

| 字段 | 状态 |
| --- | --- |
| `scan_files` | **生效**。`collect` 的目录遍历在达到该值时停止，输出随即带 `complete: false` 与 `reason_code: scan.file_limit_reached`。被截断的清单是**未知的**清单而不是更小的清单，因此它的 `inventory_digest` 与完整扫描不同，不能被当作完整 manifest 读。 |
| `daemon_rss_mb` | **不执行**。进程无法可移植地限制自身常驻内存——那需要 cgroups 或 setrlimit，本切片都不用。`GET /api/v1/settings/schema` 的 `unenforced` 字段公布这一点，Settings 界面据此在该字段旁标注「本切片不执行此项」。 |

两者此前都只是被存储与校验、从不被读取：配置它们没有任何效果。把能做的做了，做不到的说清楚，比让两个字段都看起来像可用开关要好。

## 资产复制 executor

`ctxpect assets preview|copy|rollback` 是资产的唯一复制 executor。它**不是包管理器**：不解析依赖、不下载、不理解版本区间。它只做一件事——把一份**仓内登记已声明**的文件复制到项目内声明的落点，作为可回滚的事务，并记录落地了什么。

### 资产登记

`<project>/.ctxpect/assets.json`（`schema: ctxpect-assets-v1`），随项目进版本控制：

```json
{
  "schema": "ctxpect-assets-v1",
  "assets": [
    {
      "asset_id": "skill-a",
      "origin": "project:vendor/skill-a.md",
      "license": "MIT",
      "digest": "<sha256 of the source bytes>",
      "target_rel": ".ctxpect/skills/skill-a.md"
    }
  ]
}
```

### 供应链门禁

复制前必须通过全部核验，任何一条不过都在 `preview` 阶段拒绝，因此**不会留下半个文件**：

| 情况 | reason code |
| --- | --- |
| asset_id 不在登记里 | `assets.not_registered` |
| 没有声明许可证 | `assets.license_unknown` |
| 没有声明来源 | `assets.origin_unknown` |
| 来源不是 `project:<rel>` | `assets.origin_unsupported` |
| 来源或落点含 `..` / 绝对路径 | `assets.path_escapes` |
| 源文件实际内容与登记摘要不符 | `assets.digest_mismatch` |

最后一条是冒名的防线：名字与许可证都对得上，但字节不是登记的那份，就不是那个资产。

### 事务与并发

`copy` 经唯一 authority `authorize_store_apply`，并在写入前重新核对两件事：源字节仍与核验时一致（否则 `assets.source_changed`），落点自 preview 以来未被他人改动（否则 `assets.concurrent_hash`，并发编辑得以保留）。复制成功后写入 `store` 的 lock 条目、审计记录，以及一份 **post-Receipt**——复制改变了项目，之后的状态要被观测而不是假定。

`rollback` 恢复先前字节；若这次复制是新建文件，则删除它。

### lock 与 SBOM

`ctxpect assets sbom` 从 lock 生成物料清单，含每个组件的来源、许可证与 sha256，并统计**没有许可证的组件数**而不是把它们从列表里略去。

该文档显式声明 `spdx_or_cyclonedx: "not-emitted"`：输出 SPDX 或 CycloneDX 意味着遵循其规范，一份仅仅顶着那个名字的近似文档会误导对合规性的判断。`scope` 同样写明它只覆盖本 executor 复制的资产，**不是** Cargo 或 npm 的依赖图。

## `standard` 各子命令的实际语义

`preview` / `adopt` / `pin` / `update` / `status` 是五个不同的操作，不是同一次读取的五个别名。采纳状态写在 store 的 `adoptions/<standard_id>.json`（`schema: ctxpect-adoption-v1`）。

| 子命令 | 写状态 | 前置条件 | 说明 |
| --- | --- | --- | --- |
| `status` | 否 | 无 | 读 standard 与本项目采纳状态；缺失即报 `absent` |
| `preview` | 否 | standard 存在且验签通过 | 报告 `adopt` 将写入什么（`would_write`），并标 `writes_state: false` |
| `adopt` | 是 | 未采纳 | 建立采纳，记录 `source_digest`；重复采纳报 `standard.already_adopted` |
| `pin` | 是 | 已采纳 | 固定到当前 `manifest_digest`，`state` 变 `pinned` |
| `update` | 是 | 已采纳且 standard 已变化 | 跟进到当前版本，返回 `from_digest`/`to_digest`；无变化报 `standard.already_current` |
| `leave` / `rollback` / `revoke` | 是 | standard 存在 | 破坏性删除；不存在时报 `standard.absent`，不谎报删除成功 |

`adopt` / `pin` / `update` / `publish` / `validate` / `leave` / `rollback` / `revoke` 都经过唯一 authority `authorize_store_apply`，并写入审计链。

## JSON envelope（计划）

```text
{
  "schema_version": 1,
  "command": "inspect",
  "exit_code": 3,
  "receipt_id": "...",
  "unknown": [{"reason_code": "not_installed", "family": "cline"}],
  "findings": [],
  "warnings": []
}
```

时间字段不参与 canonical digest。

## 安全

- 启动命令以参数数组生成，禁止字符串拼接后交给 shell
- 复制为 shell 文本时按目标 shell 严格 quoting
- secret 只通过 secret manager / 环境引用在执行边界注入
- `ctxpect collect` over SSH 调用系统 SSH，校验 known_hosts，不保存密码

## 当前可运行的不是 CLI

本阶段可运行的是 `python3 scripts/check_*.py`。不要把它们别名为 `ctxpect`。
