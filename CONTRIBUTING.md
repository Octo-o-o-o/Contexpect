# 贡献指南

> 状态：规范（尚未实施产品运行时）

感谢考虑为 Contexpect 做贡献。当前仓库有规范、验收合同与阶段实现（Rust workspace、`packages/ui`、独立 workspace 的 Tauri 壳）；完整产品运行时尚未实施，逐项状态见 [交付状态](docs/process/2026-09-08-delivery-status.md)。存在源码不代表已通过验收。

## 行为准则

参与即表示同意 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 开始之前

1. 阅读 [docs/README.md](docs/README.md) 与 [完整需求](docs/requirements/2026-09-04-contexpect-complete-product-requirements.md)。
2. 不要把范围裁成 MVP。F-01–F-18 与 WP-01–WP-12 都是完成合同。
3. 不要编造原生证据。未安装、未授权、无 oracle 的坐标必须保持 Unknown / not-installed / connector-required。
4. 不要把 LLM 建议写入 Claim provenance、policy、CI 或 baseline。

## 开发者环境

文档/合同门禁只需要 Python 3.9+ 标准库；Rust 门禁需要 stable toolchain（零第三方 crate，离线）；前端四条需要 Node 22 / pnpm。不要为门禁安装 pip 包。本阶段 16 条 required gate 的名称与命令以 [AGENTS.md](AGENTS.md) 为准，必须一起跑；前四条如下：

```bash
python3 scripts/check_docs.py
python3 scripts/check_acceptance.py --structure
python3 scripts/check_acceptance.py --traceability
python3 scripts/check_acceptance.py --corpus
```

验收 YAML 使用 JSON 兼容子集，以便零依赖校验。

## 变更类型

| 类型 | 要求 |
| --- | --- |
| 规范/文档 | 保持“尚未实施”诚实；更新 `docs/README.md` 索引 |
| 验收合同 | 新建 revision，说明原因，重跑 16 条门禁；不得为了让测试变绿而改产品含义 |
| 生成夹具 | 使用 `scripts/generate_acceptance.py`；记录 digest 与 Apache-2.0；`live_tested: false` |
| 运行时代码 | 必须跟随 [implementation-plan](docs/process/implementation-plan.md)、ADR 与 [AGENTS.md](AGENTS.md) 不变量；产品 crate 零第三方依赖、不执行 harness；每条修复先有转红负例 |

## 规范性语句

PRD 中带“必须 / 不得 / 不能 / 禁止 / 只有……才”的语句由 `scripts/check_acceptance.py --traceability` 抽取。新增或改写 PRD 后必须重新生成并核验 `acceptance/traceability.csv`。

## 许可与 provenance

- 贡献默认按 Apache-2.0 授权。
- 禁止提交无许可证、许可证不兼容或未保留 NOTICE 的代码。
- 厂商文档、真实会话、第三方配置进入 corpus 前必须记录来源、许可、脱敏状态和 digest。
- 依赖变更必须更新 [NOTICE](NOTICE)、SBOM 生成路径和 [dependency-and-provenance](docs/process/dependency-and-provenance.md)。

## 安全

不要在 issue、日志、Receipt 夹具或截图中放入真实 secret、绝对家目录或客户源码。漏洞披露走 [SECURITY.md](SECURITY.md)。

## Pull request

- 用简体中文写清做了什么以及为什么。
- 只包含与任务相关的文件。
- 贴出 16 条门禁各自的退出码；未跑或失败如实写明。
- 尚未实施的部分不要假装其测试已经存在。
