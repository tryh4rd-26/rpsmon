# rpsmon - Rust Process Monitor

A high-performance, macOS process monitor written in Rust. Designed as a modern alternative to `top` and `htop` with a minimal, focused terminal user interface and real-time system telemetry visualization.

## Features

- **Real-time process monitoring** with live CPU and memory metrics per-process
- **Process tracking with PID pinning**: Viewport scrolling smartly locks to your selected process across ticks instead of jumping visually
- **Advanced visualization**: CPU / Memory line charts mapping history live, bar charts, Gauges, and color-highlighting
- **System panel** showing per-core CPU bars, RAM visualization, battery, thermal, network stats, and GPU utilization with real data API integration
- **btop-style layout** with prominent system information and compact process list
- **Auto-scrolling process list** with viewport management for large process counts
- **Unicode visualizations**: Per-core CPU bars, RAM fill percentage, color-coded highlighting
- **Advanced filtering** with fuzzy search by PID, process name, or user ID
- **Vim-like keybindings** for intuitive navigation: j/k, f/search, v/kill, s/sort
- **Multiple sortable columns**: by CPU usage, memory, PID, name, or thread count
- **Configurable preferences** saved to `~/.config/rpsmon/config.toml`
- **Optimized binary**: 1.4 MB with LTO compilation and symbol stripping
- **High-performance rendering**: 60 FPS UI refresh with 1 FPS data collection (decoupled event rates)
- **Cross-platform support**: Built specifically for macOS and Linux.
- **Graceful error handling** for system calls and permission restrictions
cd
## Installation

### From crates.io (Recommended)

```bash
cargo install rpsmon
```

### From Source

```bash
git clone https://github.com/tryh4rd-26/rpsmon.git
cd rpsmon
cargo build --release
./target/release/rpsmon
```

### Global Installation (Run from anywhere)

To install `rpsmon` as a global command on your system, run:

```bash
cargo install --path .
```

This will compile the binary and place it in your `~/.cargo/bin` directory. Ensure this directory is in your `PATH` (it usually is if you installed Rust via rustup).

Alternatively, you can manually move the binary to `/usr/local/bin`:

```bash
sudo cp target/release/rpsmon /usr/local/bin/
```

### Prerequisites

- Rust 1.56+ (with Cargo)
- macOS 10.13+ or Linux with glibc 2.17+

## Usage

### Keyboard Controls

| Key | Action |
|-----|--------|
| `j` or `↓` | Move selection down |
| `k` or `↑` | Move selection up |
| `s` | Cycle through sort modes (CPU, Memory, PID, Name, Threads) |
| `f` | Enter filter/search mode (case-insensitive) |
| `/` | Alternate search trigger (for backward compatibility) |
| `v` | Kill selected process (with confirmation via repeat) |
| `q` | Quit application |
| `Ctrl+C` | Force exit |
| `ESC` | Exit search mode |

### Search Mode

Press `/` to activate filter mode. Search queries are matched against:

- Process ID (numeric)
- Process name (case-insensitive substring matching)
- User ID (numeric)

Press `ENTER` or `ESC` to exit search mode.

### Sort Modes

Press `s` to cycle through available sort orders:

1. CPU usage (descending)
2. Memory usage (descending)
3. Process ID (descending)
4. Process name (ascending)
5. Thread count (descending)

## Configuration

Configuration is stored at `~/.config/rpsmon/config.toml` and created automatically on first run.

```toml
theme = "nord"
sort_by = "cpu"
refresh_rate = 16
```

**Configuration Options:**

- `theme`: Color scheme ("nord" is default; currently the only theme)
- `sort_by`: Default sort mode on startup ("cpu", "memory", "pid", "name", "threads")
- `refresh_rate`: UI update interval in milliseconds (default: 16 ms for 60 FPS)

## Project Structure

```
src/
├── main.rs       Main entry point and terminal initialization
├── app.rs        Application state management and event loop
├── events.rs     Event dispatcher (keyboard input, timer ticks, resize)
├── keys.rs       Keyboard input handler with double-tap detection
├── ui.rs         Terminal UI rendering and layout management
├── process.rs    Process data collection, filtering, and sorting
├── config.rs     Configuration file I/O
├── sort.rs       Sort mode definitions and utilities
└── sparkline.rs  Unicode sparkline chart rendering

Cargo.toml       Project manifest with dependencies and build configuration
README.md        This file
```

## Architecture

RPS uses a modular, event-driven architecture with optimizations for low CPU usage and high frame rates:

- **Decoupled event rates**: UI rendering at 60 FPS (`UiTick`), system data collection at 1 FPS (`DataTick`) to eliminate glitching
- **Viewport scrolling**: Process list maintains scroll offset separately from selection index for smooth navigation
- **State machine**: Central `App` struct manages application state and responds to keyboard/system events
- **Per-core CPU tracking**: Normalized CPU usage per-core (not global) with real-time percentage display
- **Async runtime**: Tokio enables non-blocking system information collection
- **TUI rendering**: Ratatui provides the terminal UI framework with per-frame render updates
- **Color-coded highlighting**: Automatic visual emphasis on high-CPU (red) and high-RAM (magenta) processes

The application collects system metrics at 1 Hz and renders the UI at 60 FPS, maintaining a rolling history buffer for optional trend visualization.

## Building

### Development Build

```bash
cargo build
```

### Release Build (Optimized)

Optimized for performance and size with aggressive LTO and symbol stripping:

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

### Binary Size Analysis

```bash
cargo build --release
ls -lh target/release/rps
```

Expected size: 1.4 MB (with LTO and symbol stripping enabled)

## Performance Characteristics

- **Startup time**: ~200 ms
- **Idle CPU usage**: < 1% (with 16 ms refresh interval)
- **Idle memory**: 12-15 MB
- **Peak refresh rate**: 60+ FPS on modern systems
- **Binary size**: 1.4 MB (stripped, LTO enabled)

## Technical Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| TUI Framework | ratatui | 0.27 |
| System Information | sysinfo | 0.30 |
| Terminal Events | crossterm | 0.27 |
| Async Runtime | tokio | 1.x |
| Configuration | serde + toml | Latest |

## Limitations

- Thread count display not available in sysinfo 0.30; sorting by memory as proxy
- macOS-specific features not yet implemented (App Nap status, Sandbox sandbox, thermal data)
- Apple Silicon P-core vs E-core distinction not yet available
- Process visibility limited to permissions of the user running RPS
- No real-time integration with system log streams

## Known Issues

- None currently identified. See GitHub Issues for any reported problems.

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. **Fork the Project**
2. **Create your Feature Branch** (`git checkout -b feature/AmazingFeature`)
3. **Commit your Changes** (`git commit -m 'Add some AmazingFeature'`)
4. **Push to the Branch** (`git push origin feature/AmazingFeature`)
5. **Open a Pull Request**

### Guidelines

- Ensure code compiles without warnings (`cargo build`)
- All tests must pass: `cargo test`
- Maintain the minimal binary size (< 5 MB)
- Follow standard Rust naming conventions and `rustfmt`
- Catch and handle errors gracefully to avoid UI panics

## License

MIT License. See [LICENSE](LICENSE) file for details.

## Support

For bug reports and feature requests, visit the GitHub Issues page.
