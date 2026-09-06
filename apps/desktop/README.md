# Desktop shell

> 状态：规范与本地 API 已接上；Tauri 2 桌面壳尚未创建，因此也不在 Cargo workspace 内。产品运行时尚未完成全部 OS WebView 验收。

Truth path is the localhost API served by `ctxpect daemon start` (127.0.0.1 only). The React app in `packages/ui` consumes that API. Browser preview and a future Tauri WebView must not become a second claim authority.

Start:

```bash
pnpm --dir packages/ui build
cargo build -p ctxpect-cli
./target/debug/ctxpect daemon start \
  --project <temp-project> \
  --store <temp-store> \
  --listen 127.0.0.1:7420 \
  --ui-root packages/ui/dist
```

Then open `http://127.0.0.1:7420/doctor`.
