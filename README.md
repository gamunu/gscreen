# gscreen

A true color command wrapper for terminal programs.

## About

gscreen is a lightweight wrapper that enables true color (24-bit RGB) support for any terminal program. It's particularly useful for applications like vim, neovim, htop, and other terminal programs that benefit from enhanced color rendering but might not detect true color support correctly.

## Features

- ✅ **True Color Support**: Automatically sets `COLORTERM=truecolor` and other environment variables
- ✅ **Universal Compatibility**: Works with any command-line program  
- ✅ **Transparent Operation**: Programs don't know they're being wrapped
- ✅ **Modern Architecture**: Built with async Rust and modern dependencies
- ✅ **Cross-Platform**: Works on macOS, Linux, and other Unix systems
- ✅ **Full I/O Support**: Complete keyboard input forwarding and output rendering

## Installation

```bash
# Build from source
git clone https://github.com/gamunu/gscreen.git
cd gscreen
cargo build --release

# The binary will be at ./target/release/gscreen
```

## Usage

Use gscreen as a prefix to any terminal command:

```bash
# Launch vim with true colors
gscreen vim file.txt

# Launch neovim with enhanced colors
gscreen nvim config.rs

# Use with any terminal program
gscreen htop
gscreen ranger
gscreen bat README.md
```

## How It Works

gscreen creates a pseudo-terminal (PTY) for the target program and:

1. **Sets true color environment variables**:
   - `COLORTERM=truecolor`
   - `TERM=xterm-256color`
   - `FORCE_COLOR=1`
   - `CLICOLOR_FORCE=1`

2. **Provides transparent I/O proxying**:
   - Forwards all keyboard input to the child process
   - Streams output directly to your terminal
   - Handles special keys (arrows, function keys, Ctrl combinations)
   - Supports window resizing

3. **Maintains compatibility**:
   - Programs run exactly as if called directly
   - Exit codes are preserved
   - Signal handling works correctly

## Technical Details

- **Language**: Rust (edition 2021)
- **Runtime**: Tokio async runtime
- **PTY Handling**: portable-pty for cross-platform compatibility
- **Terminal Control**: crossterm for true color and raw mode support
- **CLI**: clap 4.0 for modern command-line parsing

## Development

```bash
# Run in development mode
cargo run -- vim test.txt

# Build optimized release
cargo build --release

# Run tests
cargo test
```

## License

GPL-2.0

## Author

Gamunu Balagalla <gamunu@fastcode.io>