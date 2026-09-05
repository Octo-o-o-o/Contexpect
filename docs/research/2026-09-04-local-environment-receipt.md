# 2026-09-04 本机调研环境 Receipt

> 捕获时间：2026-09-04T12:33:58+08:00  
> 工作目录：`.`
> 性质：本会话只读探测的原始输出摘录；不是未来产品的正式 Context Receipt。

## OS 与架构

```text
$ sw_vers
ProductName:        macOS
ProductVersion:     27.0
BuildVersion:       26A5425a

$ uname -m
arm64
```

参考硬件探测：

```text
$ sysctl -n machdep.cpu.brand_string
Apple M5 Pro

$ sysctl -n hw.memsize
51539607552

$ sysctl -n hw.ncpu
18
```

## Harness 版本

```text
$ codex --version
codex-cli 0.147.0

$ claude --version
2.1.259 (Claude Code)

$ cursor --version
3.19.7
90de2327392570a5f5f625c656c6749d228e6430
arm64

$ cursor-agent --version
2026.08.25-3e8eec8

$ grok --version
grok 1.0.13 (5e9a58528b76)
```

Cursor app path：

```text
$ mdfind 'kMDItemCFBundleIdentifier == "com.todesktop.230313mzl4w4u92"' | head -5
/Applications/Cursor.app
```

## 原生检查能力

```text
$ codex debug --help
Debugging tools

Usage: codex debug [OPTIONS] <COMMAND>

Commands:
  models        Render the raw model catalog as JSON
  app-server    Tooling: helps debug the app server
  prompt-input  Render the model-visible prompt input list as JSON
  help          Print this message or the help of the given subcommand(s)
```

```text
$ grok inspect --help
Show the configuration Grok discovers for this directory

Usage: grok inspect [OPTIONS]

Options:
      --json                  Emit machine-readable JSON output
      --debug                 Enable debug logging
      --debug-file <FILE>     Write debug logs to FILE
  -h, --help                  Print help
      --leader-socket <PATH>  Use a custom leader socket path instead of the default `~/.grok/leader.sock`
```

## Codex 独立 Prompt Probe

命令只统计结构，未把 prompt 正文写入项目：

```text
$ codex debug prompt-input 'context inspection probe' | jq <structure-only-query>
{
  "messages": 5,
  "characters": 41460,
  "by_role": [
    {
      "role": "developer",
      "messages": 3,
      "characters": 23631
    },
    {
      "role": "user",
      "messages": 2,
      "characters": 17829
    }
  ]
}
```

证据边界：这是一个独立 Codex CLI probe 的 `prompt.input` 列表，不是当前 Codex Desktop 会话，也不是完整 provider wire payload。公开 issue #35706 说明当前命令未返回同一 core Prompt 的 base instructions 和 request-scoped tool schemas。

## 扩展 Agent 探测补充

在高保真原型完成后，按用户补充范围再次探测。只记录 path/version/目录存在性，不读取 credential 或配置正文。

```text
opencode       /opt/homebrew/bin/opencode                1.18.21
kimi           $HOME/.kimi-code/bin/kimi     0.40.1
kimi-cli       $HOME/.local/bin/kimi-cli     1.49.0
goose          /opt/homebrew/bin/goose                   1.37.0
gemini         /opt/homebrew/bin/gemini                  0.55.1
qwen           /opt/homebrew/bin/qwen                    0.18.0
copilot        /opt/homebrew/bin/copilot                 1.0.82
ZCode.app      /Applications/ZCode.app                   3.10.2
Kiro CLI.app   /Applications/Kiro CLI.app                2.9.0
```

Kiro App bundle 内实际发现 `kiro-cli`、`kiro-cli-chat`、`kiro-cli-term` 和 `kiro_cli_desktop` executable，但没有假设它们已加入 PATH。

以下 command 当前未找到：`dsh`、`zcode`、`coze`、`aider`、`cline`、`kiro`。其中：

- `~/.dsh` 与 `~/.zcode` 存在；ZCode 同时有可运行 App bundle，DSH 没有找到 executable。
- Coze 没有找到 executable、App bundle、`~/.coze` 或全局 npm package；只找到 `cn.coze.desktop` 的历史 Recent Documents 痕迹。
- `/Applications/Kiro CLI.app` 存在，因此 Kiro 是“App/executable bundle present, PATH command absent”，与 DSH/Coze 状态不同。

这组结果直接要求未来产品分别记录 executable、App、config residue、auth 和 connector readiness，不能只给一个 Installed/Active 布尔值。
