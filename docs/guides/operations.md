# 运维、Daemon 与 CI

> 状态：规范（尚未实施产品运行时）

## CLI 是一等公民

核心扫描、diff、Doctor、Receipt、历史查询在没有 daemon、UI、账号、网络时必须可用。运维不得把“先启动后台服务”写成前置。

## Daemon

可选增强，不是产品成立条件。

职责：

- 监听配置和版本变化
- 去重事件，生成增量 snapshot
- 为 UI/IDE 提供只绑定 localhost 的 API（随机 token 或 Unix socket）
- 资源上限：空闲 30 分钟窗口平均 CPU <1%、P95 <3%，RSS P95 <150MB；超限可见并可单独禁用 importer

通知：本地桌面、终端、webhook adapter。默认只通知 actionable change，不发送正文。同一 digest 不重复通知。

## Scheduler

支持 daily/weekly/monthly policy，并集成 cron/launchd/Task Scheduler。不自造任务调度云。

## CI

`ctxpect ci` 输出 JSON、SARIF、Markdown summary。

| Exit | 含义 |
| --- | --- |
| 0 | 全部 required policy checks 确定通过 |
| 2 | 确定性 drift 或可阻断 Doctor finding |
| 3 | required indeterminate、未知版本、policy 过期/无法验证 |

默认只使用 deterministic rules 和 metadata。需要 secret/runtime/account 的检查应 skip/unknown，而不是假绿。

未知 harness version 不得建立新 baseline。

CI 对 Team Context Standard 只评价语义 drift/loss/Unknown 与 policy 分层结果。把四份字节相同的 MD 当成 pass 必须失败。无托管通道的 harness 只能 `detect-only`，不得假绿。

## Remote / container / CI collect

默认在目标环境运行版本匹配的 `ctxpect collect`，生成签名/脱敏 Receipt 后离线导入。Host UI 不通过共享文件系统猜测远端 home。

可选 SSH collector：系统 SSH、校验 known_hosts、不保存密码、显式授权远端 roots、schema/version negotiation。失败则受影响 claim 为 indeterminate。

## 安装、升级、卸载（WP-11）

- 更新来自签名发布 / pinned digest，有回退保护与 revoked-version denylist
- 更新失败不破坏当前已验证版本
- 卸载不删除用户原生配置
- 删除产品数据单独、可预览

## 备份

SQLite/vault 支持完整删除、备份、迁移。backup 遵循与源数据相同或更强的加密/保留策略。派生索引在删除后失效。

## 本阶段

没有 daemon unit、没有 CI action 可安装。可运行的只有文档门禁脚本。
