# Development 项目指令核对

> 状态：本地工程候选；不改变 frozen cutoff、旧 golden 或完整产品验收。

独立 CLI 使用 `ctxpect integration`，从 stdin 读取一个 JSON 请求（最多 64 KiB），输出一个 JSON 结果。协议 `schema_major=1` 与正式 Receipt schema 分开；其他 major 拒绝。`action=capabilities` 从 engine 的 development registry 返回支持坐标，不在 OWF 复制能力表。

当前坐标为 Codex CLI **0.153.3 / cli / macos-27-arm64**，声明 profile 为 `isolated-default-trusted-instructions-v1`：默认文件名、32768 项目字节上限、可信项目、空全局指令。它不读取实际用户 config/auth/session，也不声称自定义配置或其他版本受支持。未知版本与 OS 返回 Unknown。

依据为 [0.153.3 原生源码](https://github.com/openai/codex/blob/rust-v0.153.3/codex-rs/core/src/agents_md.rs)（原始文件 SHA-256 `8bbaf068c099fdeeaf4fe49076d398da7671f20c9a962fde5c4eb7653008fed4`）和 [官方指令说明](https://developers.openai.com/codex/guides/agents-md)。规则按 cwd 最近 `.git` 向下发现；无 marker 只检查 cwd；同层 override 优先，空白正文不构成采用。项目 cap 不扣除全局指令。当前可用 profile 不提供全局指令；其他能力保持 Unknown。`.ctxpect-ignore` 是 Contexpect 观察排除，不是原生 absence。

```json
{"schema_major":1,"action":"inspect","request_id":"request-1","run_id":"run-1","project_id":"example","project":"/absolute/project","cwd":".","store":"/absolute/evidence/store","profile":"isolated-default-trusted-instructions-v1","version":"0.153.3","os_lane":"macos-27-arm64","native":false,"harness_tool":{"path":"/absolute/native-codex","sha256":"actual-binary-sha256"}}
```

```bash
cargo build --release -p ctxpect-cli
target/release/ctxpect integration < request.json
python3 scripts/check_context_integration.py --bin target/release/ctxpect --native-tool /absolute/native-codex --output /absolute/private/native-evidence.json
```

请求仅由本机操作者或受控 bridge 构造。网页只能选择已注册项目，不能提交路径、argv 或 profile。源文件只读，store 必须在被检项目外单独配置。相关文件及缺席文件都有 manifest；扫描前后比较，不稳定时最多重试一次。formal Receipt 通过原迁移、持久化和 local-continuity 验证链产生；重复内容保持 first-write-wins，`observed_at` 不替代 Receipt 首次 `created_at`。

`native=true` 显式执行真实原生诊断：只复制允许的指令文件和 root marker 到隔离 profile，使用固定摘要的 native binary，env 清空，macOS sandbox 禁止网络，stdin 关闭，限时且输出有上限。不复制项目配置、hook、MCP、skills 或 Git 配置，不启动模型。原文只在内存，输出角色/字节数与本地 HMAC；HMAC 不跨设备比较。浮点元数据只在导入解析中保留，不改正式 Receipt 数字合同。原生观察不升级静态 Claim。

结果分为 process、diagnostic、native、profile、formal_receipt 与 freshness 输入。process exit 0 可以同时伴随 diagnostic exit 2/3 或 native Unknown；不得合成“全部生效”。诊断仅覆盖上下文 instructions，不运行全仓 Doctor 安全扫描。

测试包括当前坐标、未知版本/OS、override 差异、空白和嵌套根、schema/profile/路径拒绝、symlink/FIFO、输出脱敏、重复 Receipt 及显式原生诊断。专项不替代现行 16 条 required gate 与 ui-e2e。

## A/B/C 观察记录

| 事件 | A 原生方法 | B 独立 CLI | C OWF 入口 | 解释正确性 | 人工耗时 | 复用/维护成本 |
| --- | --- | --- | --- | --- | --- | --- |
| 合成 override 差异 | 诊断构造可观察正文 | 给出 overridden-by 与 Receipt | 同一引擎结果关联和过期提示 | 由专项测试核对 | NOT_RUN | NOT_RUN |
| 真实需求事件（待记录） | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN |

不把合成故障或本机工程通过当作长期需求/收益证据。没有启动长期监控；独立产品验收与正式发布仍另列。
