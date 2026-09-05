# 支持

> 状态：规范（尚未实施产品运行时）

## 现在可以问什么

当前仓库没有可安装的 `ctxpect` 二进制或桌面应用。支持范围限于：

- 产品合同与文档如何阅读
- 验收件、夹具分类和门禁脚本如何运行
- 某条需求对应哪个工作包 / 测试门禁

请先搜索 [docs/README.md](docs/README.md) 和已有 issue。

## 以后运行时如何获得帮助

实现发布后，支持通道按以下顺序：

1. 文档：用户指南、CLI 参考、桌面 UI、运维指南
2. GitHub Discussions 或 issue（可公开的行为问题）
3. 附带脱敏 Context Receipt 的 bug 报告
4. 安全问题走 [SECURITY.md](SECURITY.md)，不要公开贴日志

## 报告缺陷时请附带

- coordinate：OS、harness、version、surface、cwd/project（脱敏）
- Receipt id 与 schema version
- 是 Expected、Observed 还是 Unknown 出了问题
- 是否可在 development corpus 的合成夹具上复现
- daemon / UI / CI 哪一个入口

不要发送：明文 secret、完整 prompt、客户源码、未脱敏绝对家目录。

## 商业支持

托管 E2EE 同步、私有 adapter SLA、团队 policy 与 air-gapped 包是可选商业能力，不是完成本地开源产品的前提。当前没有已上线的付费服务。

## 非目标

- 维护者不会在 issue 里帮你绕过 Unknown fail-closed。
- 维护者不会把一次模型登录失败解释成整个 family 废弃。
- 维护者不会接受“请当成 MVP 先做四个工具”作为完整交付的替代。
