# Contexpect Doctor 原型盲测任务

你是一个独立的产品可用性测试参与者。全程只读，不修改任何文件，不运行网络请求，不输出或查找凭据。

请先完整查看这张最终原型图：

`/Users/wangyixiao/WorkSpace/ContextView/docs/gpt-img-2-design/20260904-1200-contexpect-doctor/images/10-context-doctor-final.png`

并阅读以下短文档作为产品语义合同：

- `/Users/wangyixiao/WorkSpace/ContextView/docs/gpt-img-2-design/20260904-1200-contexpect-doctor/00-brief.md`
- `/Users/wangyixiao/WorkSpace/ContextView/docs/gpt-img-2-design/20260904-1200-contexpect-doctor/ui-spec.md`
- `/Users/wangyixiao/WorkSpace/ContextView/docs/gpt-img-2-design/20260904-1200-contexpect-doctor/02-design-system.md`

如果当前 harness 无法直接查看 PNG，必须明确写 `IMAGE_UNAVAILABLE`，并只根据文档评价；不得假装读到了图片。

模拟三类用户各走一遍：

1. 多工具重度用户：发现 Cursor 行为和其他 agent 不一致，想知道是缺配置、没加载还是不可观察。
2. 偶发用户：换了设备后 MCP 表现不同，只想快速找到原因和下一步。
3. 团队 DevEx：想确认哪些 agent 有 native evidence，哪些只有静态解析或缺 connector。

逐项判断用户能否在 30 秒内：

- 找到当前最重要的问题；
- 区分 Confirmed、Suspected、Unknown；
- 理解 Expected/Resolved 不等于 Native Observed；
- 在 Indeterminate 时先收集证据，而不是直接修改；
- 理解 13 个 agent 的三种 adapter coverage；
- 找到治疗、复查、持续监控和 Receipt 的后续入口。

只输出以下结构，保持简洁：

```text
IMAGE: AVAILABLE | IMAGE_UNAVAILABLE
VERDICT: PASS | PARTIAL | FAIL
TASKS: <6 项逐项 PASS/PARTIAL/FAIL + 一句理由>
PERSONAS: <3 类用户各一句>
P1: <会妨碍正确理解或安全操作的问题；没有则写 none>
P2: <可改善但不阻断的问题；没有则写 none>
KEEP: <最应该保留的三个设计决定>
```
