# gscreen

A true color command wrapper for terminal programs.

## About

gscreen is a lightweight wrapper that enables true color (24-bit RGB) support for any terminal program. Useful for applications like vim, neovim, htop, and other programs that benefit from enhanced color rendering.

## Features

- **True Color Support**: Automatically sets `COLORTERM=truecolor` and other environment variables
- **Universal Compatibility**: Works with any command-line program  
- **Transparent Operation**: Programs don't know they're being wrapped
- **Modern Architecture**: Built with async Rust and modern dependencies
- **Cross-Platform**: Works on macOS, Linux, and other Unix systems
- **Full I/O Support**: Complete keyboard input forwarding and output rendering

## Installation

### Download Release Binary

Download the latest release from [GitHub Releases](https://github.com/gamunu/gscreen/releases/latest):

```bash
# Download and extract for your platform
curl -L https://github.com/gamunu/gscreen/releases/download/v0.1.0/gscreen-macos-intel.tar.gz | tar -xz
chmod +x gscreen
sudo mv gscreen /usr/local/bin/
```

### Build from Source

```bash
git clone https://github.com/gamunu/gscreen.git
cd gscreen
cargo build --release
# Binary will be at ./target/release/gscreen
```

## Usage

```bash
# Basic usage
gscreen vim file.txt
gscreen nvim config.rs

# Works with command arguments
gscreen ls -la
gscreen git status
gscreen vim .

# Any terminal program
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