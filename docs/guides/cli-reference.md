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
| `ctxpect exception request\|approve\|reject\|revoke\|status` | WP-11 | 受控例外；过期 fail-closed。`approve`/`reject`/`revoke` 拒绝调用方自报 `--role`/`--actor`；本切片无外部身份源，reason code `exception.identity_source_uncovered` |
| `ctxpect assets` | WP-08 | catalog / validate / 展示 APM lock |
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

命令特定错误写入 JSON envelope 的 `error.code`（例如 `align.byte-equality-not-semantic`、`standard.unsigned`、`exception.expired`、`exception.identity_source_uncovered`），不新增第五种 exit code。`align` 把 byte/hash 相等当作语义通过时必须非 0。

`policy show`（无 `--from`）读取与 `apply` 相同的 store policy / exception 集合，并调用同一 `authorize_mutation` 判定。无 layers 时 `verdict=unknown`、`reason_code=policy.unknown`，不是 pass。`policy eval --from <file>` 只评价该文件，不替代 store 判定。

## 未覆盖：例外批准的身份源

`ctxpect exception approve|reject|revoke` 不把调用方自报的 `--role` / `--actor` 当作授权证据。本切片没有 IdP、OS 用户绑定或已登记 principals。因此批准路径 fail-closed，`error.code=exception.identity_source_uncovered`。把已批准例外写入 store 只能由 store 外的运营动作完成；产品命令不能自批自用。

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
