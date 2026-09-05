# ADR 0002：信任边界

> 状态：已接受（规范冻结；尚未实施产品运行时）
> 日期：2026-09-04

## 决策

| 主体 | 允许 | 默认拒绝 |
| --- | --- | --- |
| UI renderer | 调用 core 的显式 command/API | 直接读文件系统、拼 shell、任意 URL |
| Core | 用户授权的项目/配置 roots、声明 adapter 路径 | 扫描 `.git` 正文、依赖、`.env`、私钥、用户排除路径 |
| Bundled adapter | 声明路径与 capability | 网络、secret、整个 home |
| 第三方 adapter | 进程隔离或 WASI 的最小权限 | 网络、secret、home；出站走 egress allowlist |
| Sync engine | 标记为可同步的 redacted representation | 明文 secret、vault 解锁后的缓存正文 |
| LLM Advisor | 用户确认的脱敏 payload | 仓库/会话整包、未授权 egress |
| Runtime proxy | 独立高级 profile | 默认路径；主产品无它也完整 |
| 本地 API | loopback / Unix socket + 短期 credential | 非本机访问 |
| 管理员导出 | metadata、hash、policy status | 开发者 prompt 正文 |

## Reason code 与失败

越权或能力不足必须变成稳定 reason code 与 indeterminate/deny，而不是 toast。被动扫描不得执行 hook/MCP/plugin。

## 后果

Tauri command allowlist、CSP、路径重新解析、secret redaction-before-log 都是 WP-04/WP-11 的硬门禁。linter 是 defense-in-depth，不是 security boundary。
