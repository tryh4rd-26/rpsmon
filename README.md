# rpsmon

rpsmon is a fast Rust TUI system monitor for macOS and Linux with live process insights, tree navigation, watchlists, and alerting.

Repository: https://github.com/tryh4rd-26/rpsmon

## Features

- Real-time process table with CPU, memory, threads, status, and disk I/O columns
- Lock selection mode (`l`) so the selected PID stays tracked across refresh/sort updates
- Tree mode (`t`) with per-node expand/collapse (`Left`, `Right`, `Space`)
- Watchlist support (`w`) to pin important processes
- Alert engine (`a`) with configurable CPU/memory thresholds and hold duration
- Detail view on selected process (`Enter`) with tab cycling (`Tab`)
- Interactive signal menu in detail mode (`s`) for custom signal dispatch
- Search/filter (`f` or `/`) and sort cycling (`s` in main list)
- Smooth rendering with decoupled ticks (responsive UI and 1 Hz data updates)
- Optimized release binary with LTO and stripping

## Installation

### From crates.io

```bash
cargo install rpsmon
```

### From source

```bash
git clone https://github.com/tryh4rd-26/rpsmon.git
cd rpsmon
cargo build --release
./target/release/rpsmon
```

## Keyboard Controls

| Key | Action |
|-----|--------|
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `g` / `G` | Jump to top / bottom |
| `PageUp` / `PageDown` | Page navigation |
| `Enter` | Toggle process details |
| `Tab` | Next detail tab |
| `f` or `/` | Filter/search mode |
| `s` | Sort column (main list) |
| `s` (detail view) | Open signal menu |
| `v` | Send TERM to selected process |
| `9` | Send KILL to selected process |
| `p` | Pause/resume selected process |
| `l` | Toggle lock selection |
| `t` | Toggle tree mode |
| `Left` / `Right` / `Space` | Collapse / expand / toggle node (tree mode) |
| `w` | Toggle watchlist for selected PID |
| `a` | Toggle alerts on/off |
| `q` / `Esc` / `Ctrl+C` | Quit |

## Configuration

Config path: `~/.config/rpsmon/config.toml`

Example:

```toml
theme = "nord"
sort_by = "cpu"
refresh_rate = 16

# watchlist + alerts
watchlist_pids = [1, 1234]
alerts_enabled = true
alert_cpu_pct = 85.0
alert_mem_pct = 25.0
alert_hold_secs = 3
```

## Build

```bash
cargo build --release
```

Binary path:

```bash
./target/release/rpsmon
```

## License

MIT
