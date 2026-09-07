# Contexpect 桌面壳

Tauri 2 窗口，承载本地 daemon 已经在提供的 UI。

真值路径是 `ctxpect daemon start` 提供的 localhost API（仅 127.0.0.1）；`packages/ui`
的 React 应用消费该 API。**浏览器预览与这个 Tauri WebView 都不得成为第二个 claim
权威**——它们呈现 daemon 已经判定的结果，不自行判定。

## 它刻意不做的事

**不注册任何 Tauri command。** UI 通过 daemon 的 loopback HTTP API 通信——它本来就是照着那个 API 写的。因此这个进程没有 IPC 面需要加固，页面内容也没有任何路径可以经由它触到文件系统（[ADR 0002](../../docs/adr/0002-trust-boundaries.md)）。`capabilities` 为空数组，不是省略。

**不启动 daemon。** 代为拉起它意味着为了便利而携带一个进程执行能力，而地址仍然需要被发现。要求调用方给出地址，信任边界就留在 ADR 划定的位置。

**不加载任意 URL。** 目标由 `CONTEXPECT_DAEMON_URL` 指定，默认 `http://127.0.0.1:7420`，且必须是 `http` 加回环主机名。主机名是**精确比较**，与 daemon 自身的校验同一理由：前缀比较会放行 `127.0.0.1.evil.com`。不合规就以退出码 2 拒绝启动并说明原因，不会退化成一个没有沙箱的浏览器窗口。

## 为什么是独立的 workspace

仓库根的 Rust workspace 零第三方 crate，13 条 required gate 据此离线运行。壳需要 Tauri 及其依赖树，所以它自带 `[workspace]`、不在根 `members` 里：根目录的 `cargo build --workspace` 永远看不到它，那条性质得以保留。

实测：加入壳之后，根 workspace 仍是 17 个本地 crate、**0 个第三方依赖**。

## 构建与运行

先构建 UI 与 CLI，再启动 daemon：

```bash
pnpm --dir packages/ui build
cargo build -p ctxpect-cli
./target/debug/ctxpect daemon start \
  --project <temp-project> \
  --store <temp-store> \
  --listen 127.0.0.1:7420 \
  --ui-root packages/ui/dist
```

此时已可用浏览器打开 `http://127.0.0.1:7420/doctor`。要用桌面壳：

```bash
cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml
CONTEXPECT_DAEMON_URL=http://127.0.0.1:7420 \
  apps/desktop/src-tauri/target/debug/contexpect-desktop
```

## 验证状态

| 项 | 状态 |
| --- | --- |
| macOS 构建与运行 | **已验**（2026-09-08，Darwin arm64）：窗口打开并完整渲染 UI——16 条导航、顶栏控件、18 个 family 徽章、诊断抽屉 |
| URL 校验 | **已验**：`evil.example.com`、`127.0.0.1.evil.com`、`https://`、`ftp://` 均以退出码 2 拒绝；两种合法回环形式进入事件循环 |
| Windows lane | **未验**——无该设备 |
| Linux lane | **未验**——无该设备 |
| 打包产物（dmg/msi/AppImage） | **未做**——发布与签名不在授权范围 |
| WebView a11y / 屏幕阅读器 / ≤2s 性能 | **未做**——需真实辅助技术与逐 lane 测量 |

按 C06，浏览器预览与 macOS 截图**不能**替代其余 OS lane。此处只声称 macOS 一个 lane。
