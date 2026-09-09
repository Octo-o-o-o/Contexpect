# 发布流程

> 状态：规范（尚未实施产品运行时）

完整产品只有在一次全产品集成门禁绿 **和** 一次独立 readback 绿之后才能宣布交付。中间工作包不能单独称为产品完成。

## 发布对象（计划）

| Artifact | 说明 |
| --- | --- |
| `ctxpect` CLI | macOS/Linux/Windows 签名二进制 |
| Desktop app | Tauri 2 包，同样三条 OS lane |
| Container/CI image | 可选，不含用户正文 |
| SBOM | CycloneDX 与 SPDX reference |
| NOTICE / LICENSE | 必须随包 |
| adapter/corpus 更新包 | 签名 + pinned digest |
| schema | Receipt / manifest JSON Schema，带 migration |

## 版本

- 产品 semver
- Receipt schema major 进入 EquivalenceProfile
- Adapter 按 family+version range 独立版本
- Acceptance cutoff 与产品版本分开；升级 required matrix 必须走 contract revision

## 发布前检查

1. development、sealed、live 三套结果分别过线
2. security-critical / secret / path containment / Unknown 假绿 / CI exit 2 在 sealed/live 上零已知错误
3. 许可与 SBOM 审计
4. 未知版本 fail-closed 仍然有效
5. 卸载不删除用户原生配置的演练
6. 签名与 revocation denylist 更新

## 签名与更新

Core、adapter、corpus、executor 更新必须来自签名发布或固定 digest。失败不得破坏当前已验证版本。被撤销版本停止产生权威 claim。

## 本阶段

没有已发布、已签名的二进制或安装包：`ctxpect` 与桌面壳可在开发树构建，但不是发布物；发布前检查 6 项也没有对应脚本（见 [交付状态](2026-09-08-delivery-status.md)）。当前交付物是文档、`acceptance/`、离线门禁与阶段实现源码。
