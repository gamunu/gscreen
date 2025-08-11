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
use crossterm::terminal;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_true_color_environment() -> Result<()> {
    INIT.call_once(|| {
        // Environment setup is done here - this runs only once
    });

    // Check if terminal supports true colors
    detect_and_report_color_support();

    // Set environment variables for the current process
    // (these will be inherited by child processes)
    std::env::set_var("COLORTERM", "truecolor");
    std::env::set_var("TERM", "xterm-256color");
    std::env::set_var("FORCE_COLOR", "1");
    std::env::set_var("CLICOLOR_FORCE", "1");

    Ok(())
}

pub fn restore_terminal() -> Result<()> {
    // Disable raw mode if it was enabled
    if terminal::is_raw_mode_enabled().unwrap_or(false) {
        terminal::disable_raw_mode().context("Failed to disable raw mode")?;
    }

    // We don't need to leave alternate screen since we never entered it
    // The child process should handle its own screen mode

    Ok(())
}

fn detect_and_report_color_support() {
    // Check various environment variables that indicate color support
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

    let has_truecolor = colorterm == "truecolor"
        || colorterm == "24bit"
        || term.contains("256color")
        || term_program == "iTerm.app"
        || term_program == "Apple_Terminal";

    if has_truecolor {
        eprintln!("✓ True color support detected");
    } else {
        eprintln!("⚠ True color support not detected, but will be forced");
    }

    // Report current terminal info
    eprintln!("Terminal info:");
    eprintln!("  TERM: {}", term);
    eprintln!("  COLORTERM: {}", colorterm);
    if !term_program.is_empty() {
        eprintln!("  TERM_PROGRAM: {}", term_program);
    }
}
