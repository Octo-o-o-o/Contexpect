# Security Policy

> 状态：规范（尚未实施产品运行时）

## 支持的版本

| 版本 | 支持 |
| --- | --- |
| 文档 / 验收合同 foundation | 接受针对合同、夹具泄露和仓库秘密的报告 |
| 尚未发布的 `ctxpect` 运行时 | 实现开始后按 release train 提供补丁；当前没有已发布二进制 |

## 报告漏洞

请不要在公开 issue 中报告可利用的漏洞，也不要在任何公开渠道粘贴 token、密钥、会话正文或客户源码。

**Canonical private channel**（已启用 GitHub Private Vulnerability Reporting）：

https://github.com/Octo-o-o-o/Contexpect/security/advisories/new

私有报告应包括：

- 受影响路径或规范章节
- 复现步骤（只使用合成夹具）
- 影响：机密性 / 完整性 / 可用性 / 证据假绿
- 是否已有公开讨论

**Public fallback**（当私有 advisory 表单不可用时）：打开

https://github.com/Octo-o-o-o/Contexpect/issues/new

标题使用 `Security contact request`，正文只写非敏感的高层次影响类别和“请开启私有讨论”的请求。不要在公开 issue 中描述利用细节、PoC、凭据或未脱敏日志。维护者会把对话转到 private advisory。

本项目不公布安全邮箱；不要猜测或编造邮件地址。

维护者目标：

- 3 个工作日内确认收到
- 14 个工作日内给出严重度与修复或缓解计划
- 修复发布后按 [release process](docs/process/release.md) 出具 advisory

## 项目特定安全边界

完整威胁模型见 [privacy-and-threat-model](docs/security/privacy-and-threat-model.md)。实现必须遵守：

- UI renderer 不得直接读取任意文件系统
- 第三方 adapter 默认无网络、无 secret
- secret 不得进入 Receipt、sync bundle、LLM payload、SARIF、日志或崩溃报告
- 被动扫描不得启动 MCP、hook、plugin 或动态 retrieval
- Unknown / indeterminate 不得自动变成 pass、0 或 absent
- 路径 containment 不等于执行 sandbox
- 本地 API 只绑定 loopback/Unix socket

## 安全关键夹具

`secret_literal`、`symlink_escape`、`hidden_unicode`、`path_containment_escape`、`archive_traversal`、`passive_scan_exec` 等规则在 sealed/live corpus 上零容错。已知错误不得发布。

## Secret 扫描

贡献不得包含真实 token。测试只使用文档化的 fixture token。发现误提交时视为泄露：轮换凭据、从历史中移除、写入 incident note。
