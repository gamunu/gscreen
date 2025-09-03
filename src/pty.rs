/*
 * gscreen - A true color command wrapper for terminal programs
 * Copyright (C) 2025 Gamunu Balagalla <gamunu@fastcode.io>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, PtyPair, PtySize};
use std::collections::HashMap;

pub fn create_pty_with_command(command: &str, args: &[String]) -> Result<(PtyPair, Box<dyn Child>)> {
    // Create a new PTY with the actual terminal size
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24)); // fallback to 80x24 if detection fails

    let pty_size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pty_system = portable_pty::native_pty_system();
    let pty_pair = pty_system.openpty(pty_size).context("Failed to open PTY")?;

    // Set up environment variables for true color support
    let mut env_vars = HashMap::new();

    // Copy current environment
    for (key, value) in std::env::vars() {
        env_vars.insert(key, value);
    }

    // Override specific variables for true color support
    env_vars.insert("COLORTERM".to_string(), "truecolor".to_string());
    env_vars.insert("TERM".to_string(), "xterm-256color".to_string());

    // Force true color support
    env_vars.insert("FORCE_COLOR".to_string(), "1".to_string());
    env_vars.insert("CLICOLOR_FORCE".to_string(), "1".to_string());

    // Build the command with arguments
    let mut cmd_builder = CommandBuilder::new(command);
    cmd_builder.args(args);

    // Set the current working directory to match the shell's cwd
    if let Ok(current_dir) = std::env::current_dir() {
        cmd_builder.cwd(current_dir);
    }

    // Set environment variables
    for (key, value) in env_vars {
        cmd_builder.env(&key, &value);
    }

    // Spawn the command in the PTY slave
    let child = pty_pair
        .slave
        .spawn_command(cmd_builder)
        .context("Failed to spawn command in PTY")?;

    Ok((pty_pair, child))
}
