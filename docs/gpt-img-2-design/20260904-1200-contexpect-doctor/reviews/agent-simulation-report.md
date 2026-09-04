# Local Agent Simulation Report

> 日期：2026-09-04  
> 模式：全部只读；没有让任何外部 agent 修改项目。  
> 被测对象：三轮后的 scope-13 candidate；之后依据结果做了 evidence-semantic patch，并在互联网/本机调研后做了 scope-18 patch。最终图已由当前会话逐区视觉复核，但没有伪称外部 agent 对最终 patch 重新跑过。

## 1. 总结

能实际读取图片的 Claude Code、Kimi Code 和 GitHub Copilot CLI 都能理解主流程：发现问题 → 区分证据 → Indeterminate 时先收集证据 → 再进入 Treatment/Receipt/Monitor。说明核心交互是成立的。

模拟也暴露了两个真实问题并已修正：

1. `Resolved` 容易被理解为“已修复”，绿色 check 又让静态解析/不可观察看起来像通过；最终统一改为蓝色 `Static resolution`，原生不可见改为中性 `Not exposed`。
2. 设计/文档的 agent 数量和 coverage taxonomy 不一致；最终定义 18 个 family，并显式分成 `Native evidence / Static resolution only / Needs connector`。

另外三次“无法进入评审”的失败不是无效噪音，而是产品要诊断的环境事实：sandbox unavailable、configured model unsupported、account/client rejected。Contexpect 必须把这些当 adapter health finding 保存，而不是只弹一个错误 toast。

## 2. 运行结果

| Harness | 真实结果 | 原型结论 | 发现 | 处理 |
|---|---|---|---|---|
| Claude Code 2.1.259 | exit 0，120.417s，图片可用 | PASS | 默认排序需 severity 优先；`Resolved` 歧义；Recheck 归属略隐晦 | 排序写入 spec；Resolved 已修；Recheck 保留为 evidence action + Receipt 路径 |
| Kimi Code 0.40.1 | 首次 flags 不兼容；修正命令后 exit 0，81.245s，图片可用 | PARTIAL | 两个 P1：静态/不可见状态用了 pass-like green check；`Resolved` 双关 | 已全部修正，并增加 plain-language diagnosis |
| GitHub Copilot CLI 1.0.82 | 首次不能附 Markdown；只附 PNG 后 exit 0，37.131s | PASS | coordinate 和 evidence chain 对偶发用户偏技术 | 已补 `The rule exists, but Cursor cannot prove the model received it.`；术语保留给详情层 |
| Qwen Code 0.18.0 | exit 0，102.342s；PNG 未实际呈现给模型 | PARTIAL（仅文档） | 12/13 名册冲突；Expected/Resolved/Observed/User-attested 词表冲突；coverage taxonomy 与 Unknown/Indeterminate 未定义 | 文档全部修正；最终扩为 18 family |
| Grok Build 1.0.13 | exit 1，1.003s | 未评审 | read-only sandbox 因 `/var/run/docker.sock` symlink 无法应用，CLI 拒绝在缺保护时启动 | 未绕过 sandbox；记录 `sandbox-unavailable` |
| OpenCode 1.18.21 | exit 1，5.023s | 未评审 | configured model `x-preview-f-free` unsupported，HTTP 401，non-retryable | 未擅自改用户模型；记录 `configured-model-unsupported` |
| Gemini CLI 0.55.1 | exit 41，8.032s | 未评审 | 当前账号/client 组合返回 `UNSUPPORTED_CLIENT`，建议该用户迁移 Antigravity | 不把单次失败推断为 Gemini CLI 全局废弃；记录 coordinate-specific auth/client finding |

## 3. 三类用户的模拟结论

### 多工具重度用户

可以从 finding row 进入四段 Evidence Chain，区分“规则存在”“resolver 能推导”“模型可见性未知”“原生 surface 未暴露”。最终的 plain-language diagnosis 降低了只看术语的成本。

### 偶尔打开的用户

`Describe what feels wrong…`、`Different on another device` 等 symptom 入口能把用户带到对应 finding；但不能要求其先理解 coordinate/evidence 术语。因此列表标题与右栏第一句保持自然语言，技术坐标作为第二层。

### 团队 DevEx / 长期开启用户

adapter coverage 三分组和顶部多轴摘要能快速回答“哪些可以建立 runtime 证据、哪些只能静态解析、哪些还没连接”。对这类用户，18 个 family 不是 logo wall，而是进入 Standards/conformance matrix 的入口。

## 4. 必须保留

1. 四段 Evidence Chain 及断点状态；
2. Indeterminate 时 Treatment 硬锁、Collect evidence 为主操作；
3. Confirmed/Suspected/Unknown 分轴展示，绝不收敛成健康总分；
4. adapter coverage 按证据能力分组，而不是按“在线/离线”分组；
5. 每次 adapter 启动失败也产生带 coordinate、reason code 和原始 receipt 的 finding。

## 5. 证据文件

- 使用时的冻结测试说明：`agent-simulation-prompt-used.md`
- Claude：`claude-simulation.log`、`claude-simulation.summary.json`
- Kimi：`kimi-simulation-retry.events.jsonl`、`kimi-simulation-retry.summary.json`
- Copilot：`copilot-simulation-retry.events.jsonl`、`copilot-simulation-retry.summary.json`
- Qwen：`qwen-simulation.log`、`qwen-simulation.summary.json`
- Grok：`grok-simulation.events.jsonl`、`grok-simulation.summary.json`
- OpenCode：`opencode-simulation.events.jsonl`、`opencode-simulation.summary.json`
- Gemini：`gemini-simulation.log`、`gemini-simulation.summary.json`

