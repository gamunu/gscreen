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
use clap::Parser;

mod color;
mod proxy;
mod pty;
mod terminal;
mod vte_handler;

#[derive(Parser)]
#[command(
    name = "gscreen",
    version = "0.2.0",
    about = "A true color command wrapper for terminal programs",
    author = "Gamunu Balagalla <gamunu@fastcode.io>",
    trailing_var_arg = true
)]
struct Args {
    /// The command to run
    #[arg(value_name = "COMMAND")]
    command: String,

    /// Arguments to pass to the command
    #[arg(value_name = "ARGS", num_args = 0.., allow_hyphen_values = true)]
    args: Vec<String>,

    /// Enable debug output
    #[arg(long, short, help = "Enable debug output")]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Validate that the command exists
    if which::which(&args.command).is_err() {
        anyhow::bail!("Command '{}' not found in PATH", args.command);
    }

    if args.debug {
        println!("Starting {} with true color support...", args.command);
    }

    // Set up terminal for true color support and get capabilities
    let has_osc_support = terminal::setup_true_color_environment(args.debug)?;

    // Spawn the command in a PTY
    let (mut pty_pair, child) =
        pty::create_pty_with_command(&args.command, &args.args).context("Failed to create PTY")?;

    // Start bidirectional I/O proxy with capability info and get exit status
    let exit_status = proxy::run_proxy(&mut pty_pair, child, has_osc_support).await?;

    // Clean up terminal
    terminal::restore_terminal()?;

    // Exit with the child's exit code if it's non-zero
    if !exit_status.success() {
        std::process::exit(exit_status.exit_code() as i32);
    }

    Ok(())
}
