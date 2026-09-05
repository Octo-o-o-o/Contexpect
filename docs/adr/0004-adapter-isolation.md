# ADR 0004：Adapter 隔离与扩展

> 状态：已接受（规范冻结；尚未实施产品运行时）
> 日期：2026-09-04

## 决策

- Bundled 18-family adapter 作为 workspace crate 模块，但仍只获得声明路径
- 第三方 adapter 默认 **独立进程或 WASI**，能力声明最小化
- Adapter SDK 必须包含 fixture runner、golden test、capability declaration、evidence quality、compatibility range
- 网络、LLM、registry、webhook 共用 egress 策略
- 高风险 proxy/CA 只作为高级 opt-in，单独 profile，可一键撤销

## 后果

一个 `enabled` 布尔值不能表示 family 状态。安装探测必须拆成 executable / app / config-residue / authenticated / connector-ready。
