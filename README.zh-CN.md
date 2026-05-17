# MinionsCode

> 在终端里管理你机器上所有 Claude Code 会话的看板工具。

[English](README.md) · **简体中文**

```
┌─ MinionsCode ──── 49 sessions · 1 active · ▶ 2 tmux · $2916.45 total ──────┐
│ ▾ ~/Project/05_2026/MinionsCode                                ▶1  ●1   3  │
│    ▶ tmux busy   rust-tui notify integration   sonnet-4.6  $  2.41   2m   │
│    ● idle        refresh strategy notes         opus       $  0.47  14h   │
│ ▾ ~/Project/03_2026/Forecasting_Reasoning                              5  │
│    ▶ tmux idle   Q4 forecasting backtest        opus       $  0.75   3d   │
│    ○                清理 Zone 的无用 file        opus       $  0.12  17d   │
└────────────────────────────────────────────────────────────────────────────┘
```

一个 Rust 单文件。选中一行按 `Enter` 直接进入 `claude --resume`；按
`Ctrl-b d` 让它在后台继续跑，回到看板再选下一个，需要的时候再回来——
全程不用打开任何新窗口。

---

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/ChengAoShen/MinionsCode/main/install.sh | bash
```

就这一行。脚本会自动检测平台，从 GitHub Releases 下对应的预编译二进制，
装到 `~/.local/bin/minionscode`。**不需要 Rust 工具链**。

**支持的平台**：Linux x86_64、macOS Apple Silicon（M1 及以上）。
**Intel Mac / Windows**：[从源码编译](#从源码构建)（Windows 需要在 WSL 里编）。

如果 `~/.local/bin` 不在你的 `$PATH` 里，安装脚本会告诉你怎么加。

后续更新：再跑一遍上面的命令就行，它永远拿最新的 release。

## 第一次使用

```bash
minionscode
```

打开后你会看到这台机器上所有的 session——当前活跃的在最上面，最近用过
的在中间，30 天内的历史会话在下面。任何时候按 `?` 看完整快捷键。

最基本的几个键：

| 键 | 作用 |
|---|---|
| `↑` / `↓` | 移动选中 |
| `Enter` | 恢复当前选中的 session |
| `n` | 在当前 cwd 里开一个新的 `claude` |
| `s` | 在当前 cwd 里开个 shell |
| `/` | 按名字 / 路径过滤；找不到时按 `Enter` 会回退到 AI 搜索 |
| `q` | 退出 |

## 多会话——靠 tmux 实现

只要你装了 `tmux`（macOS: `brew install tmux`，Ubuntu: `sudo apt install
tmux`），MinionsCode 会自动把每个启动的 session 包到一个后台 tmux 里。
也就是说：

1. 在 session A 上按 `Enter` → 聊几句 → `Ctrl-b d`
2. TUI 重新出现。A 这一行变成 `▶`（后台还在跑）
3. 在 session B 上按 `Enter` → 切到另一个仓库、另一个模型聊 → 再
   `Ctrl-b d`
4. 两个都在后台跑。回到 A 按 `Enter` 直接接上原来的状态，**对话历史和
   位置都不丢**。来回切随便切。

要强制结束某个后台 session：选中它，按 `K`，确认。

**没装 tmux 也能用**，只是回到老的"一次一个" 模式——直接跑 claude，
正常 `/exit` 或 `Ctrl-D` 退回看板。

## 主要功能

- **实时状态**。每个 session 一种颜色：绿色 = 等输入，琥珀 = 正在跑，
  紫色 = 在思考，青色 `▶` = 后台 tmux 跑着。变化通常一秒内体现。
- **一眼看到花了多少钱**。每个 session 的 token 用量 + 美元成本，顶部
  有当天总和。
- **按目录分组**。同时开多个项目？每一行都归到自己的 cwd 下，不关心的
  组按 `space` 折叠掉。
- **AI 搜索**。`/` 是普通的字串过滤；找不到匹配时，按 `Enter` 会回退到
  一次 Haiku 调用做语义搜索。按 `\` 强制走 AI 搜索。
- **自动起名**。按 `A` 让 Haiku 给你那些还没命名的 session 取个有意义
  的短标题（基于聊天内容）。
- **完工通知**。后台 session 从 busy 跳回 idle 时（也就是 Claude 在等你
  回复），桌面会弹通知。按 `M` 静音。
- **启动选项**。按 `N` 弹一个表单，可以勾选 `--model`、
  `--dangerously-skip-permissions`、`--sandbox`、`--verbose`、`--add-dir`。

## 快捷键速查

**导航**

| 键 | 作用 |
|---|---|
| `↑` `↓` / `j` `k` | 移动 |
| `g` / `G` | 第一个 / 最后一个 |
| `space` / `tab` | 折叠 / 展开当前组 |
| `o` / `O` | 折叠不活跃的 / 展开全部 |
| `T` | 切换"按目录分组" |

**会话操作**

| 键 | 作用 |
|---|---|
| `Enter` | 恢复 / 重新接入选中的 session |
| `n` | 在 cwd 开新 claude（默认参数） |
| `N` | 开新 claude（带选项表单） |
| `s` | 在 cwd 开新 shell |
| `r` | 重命名 |
| `K` | 杀掉这一行后台的 tmux session |

**搜索**

| 键 | 作用 |
|---|---|
| `/` | 字符串过滤 |
| `\` | 强制 AI 搜索 |
| `A` | 自动命名 |

**维护**

| 键 | 作用 |
|---|---|
| `D` | 删除垃圾 session（tmp 目录 / 空对话） |
| `E` | 删除没消息的空 session |
| `M` | 切换通知 |
| `R` | 立刻刷新 |
| `?` | 帮助 |
| `q` / `Ctrl-C` | 退出 |

## 常见问题

**它从哪里读 session 数据？** 直接读 `~/.claude/sessions/`（活跃 PID）
和 `~/.claude/projects/<cwd>/*.jsonl`（对话历史）——这都是 Claude Code
自己写的文件。

**会不会很吃资源？** 不会。用文件监听器（Linux inotify / macOS
FSEvents），只重读真正变化的文件。空闲时 CPU 接近 0。

**会偷偷发请求给什么服务器吗？** 不会。只有你按 `\` 或 `A` 的时候才会
调一次 `claude --print --model haiku`——那次请求直接走 Anthropic 官方
API，跟 Claude Code 一样。看板本身完全在本地。

**能换 `claude` 二进制路径吗？** 能：
`CLAUDE_BIN=/path/to/claude minionscode`。它默认也会在
`/opt/homebrew/bin/claude`、`/usr/local/bin/claude`、
`~/.claude/local/bin/claude`、`~/.local/bin/claude`、`$PATH` 里自动找。

**怎么卸载？** `rm ~/.local/bin/minionscode`，就这样。

## 配置项

```bash
minionscode --days 7        # 只看最近 7 天（默认 30）
minionscode --list          # 不进 TUI，直接 print 出来
minionscode --version
INSTALL_DIR=/usr/local/bin VERSION=v0.2.0 bash install.sh
CLAUDE_BIN=/opt/homebrew/bin/claude minionscode
```

持久化数据：自定义的 session 名字会存到
`~/.minionscode/session-names.json`。这是 MinionsCode 唯一写入磁盘的
文件。

## 从源码构建

```bash
git clone https://github.com/ChengAoShen/MinionsCode.git
cd MinionsCode
./build.sh
```

需要 Rust 1.74+。脚本会编 release，装到 `~/.local/bin/minionscode`
（用 `PREFIX=/usr/local` 改装到别的地方）。

## 许可

MIT，见 [LICENSE](LICENSE)。
