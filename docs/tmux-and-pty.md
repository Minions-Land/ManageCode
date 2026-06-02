# tmux vs. embedded PTY — how launches run

ManageCode opens every session in an **embedded terminal pane** (a real PTY
parsed by `vt100` and drawn with `tui-term`). What runs *inside* that pane
depends on whether tmux is available.

## The two paths

| | **tmux path** (default, when available) | **plain PTY path** (fallback) |
|---|---|---|
| When | `prefer_tmux` is true **and** `tmux` is on `$PATH` **and** we're not already inside a tmux client | tmux missing / `prefer_tmux: false` / running inside tmux |
| What runs | a detached `tmux new-session` runs the command; the pane `tmux attach`es to it | the command runs directly as the pane's child |
| Detach (`Ctrl-a` to sidebar) | tmux session keeps running in the background (`▶`) | child gets SIGHUP and exits |
| Re-open (`Enter`) | re-attaches *exactly* where you left off | starts fresh (`claude --resume <id>` restores the transcript from disk, but not live scrollback) |
| Persistence | survives switching panes all day | only lives while its pane is open |
| `K` (kill background) | kills the `mc-<id>` session | nothing to kill |

The decision is made once per launch in `launcher::open_terminal_for`
(`use_tmux = config.prefer_tmux && tmux::available() && !tmux::inside_tmux()`).

## Why keep both

tmux is what makes the headline workflow possible — *open A, talk, flip to B,
flip back to A still running.* That statefulness is the product, so the default
leans into tmux. But tmux can't always be used (not installed, or we're already
nested inside a tmux client where wrapping again would be confusing), so the
plain-PTY path is a graceful fallback rather than a hard requirement.

## Lifecycle decisions

- **Within a run:** sessions persist across pane switches. Detaching never
  kills the background session; `close_terminal` deliberately leaves tmux alone.
- **On quit:** every `mc-*` session we created is killed (`tmux::kill_all_managed`),
  so the tmux server doesn't accumulate orphans. Opt out with
  `"cleanup_tmux_on_exit": false`.
- **Dead-session resume:** resuming a historical (non-live) session kills any
  stale `mc-<id>` backing session first, so `claude --resume <id>` genuinely
  re-runs instead of re-attaching to an exited pane (matters under a
  `remain-on-exit` tmux config).
- **No tmux:** a one-line startup hint suggests installing it for persistence.

## Config (`~/.managecode/config.json`)

```jsonc
{
  "prefer_tmux": true,           // false → always use the plain PTY path
  "cleanup_tmux_on_exit": true   // false → leave mc-* sessions running on quit
}
```
