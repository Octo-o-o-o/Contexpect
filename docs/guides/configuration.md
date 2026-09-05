# 配置参考

> 状态：规范（尚未实施产品运行时）

## 查找顺序（计划）

1. 命令行 flag
2. 环境变量 `CTXPECT_*`
3. 项目文件 `.ctxpect/config.toml`（或文档化的等价路径）
4. 用户配置（XDG 或平台约定，**不是**扫描整个 home 除非用户授权）
5. 内置默认

覆盖来源必须写入 Receipt。显式 `--override-path` 同样记录。

## 配置域

| 域 | 例子 | 默认 |
| --- | --- | --- |
| privacy | `metadata-only` / vault / retention | metadata-only |
| scan | 排除 `.git`、依赖、构建产物、`.env`、私钥、用户路径 | 排除上述正文 |
| harness | family、version pin、自定义 roots | 自动探测 |
| daemon | 启用、CPU/RSS 上限、通知阈值 | 关闭 |
| sync | provider、allowlist、recipient | 关闭 |
| advisor | AnalysisAdapter command/endpoint | 关闭，默认不联网 |
| egress | allowlist、proxy、TLS | 拒绝未授权出站 |
| ci | fail-on、indeterminate policy | exit 3 非通过 |
| policy | 绑定的 context policy bundle；TeamContextStandard pin | 无则不定宽 |
| standard | git/file 路径、channel、pin revision | local-first；云可选 |

## 禁止放入配置的内容

- API key、token、私钥明文
- 客户源码
- 把 LLM 建议提升为权威的开关

Secret 只保存 OS keystore / 现有 secret manager 的引用。

## Receipt 与 redaction policy

`redaction_policy` 本身是 Receipt 字段。分享包默认 metadata/hash/规则解释。逐项授权才能附带片段。

## 未知版本

没有“把未知版本当最近兼容”的默认配置。探索预览必须显式开启，且不能改变权威 Receipt。非阻断例外必须是显式 policy 并写入 Receipt。

## 文件格式

实现时使用 versioned TOML/JSON Schema。本阶段不提供可加载的用户配置文件，避免被误认为产品已可运行。
