use anyhow::{Context, Result};
use clap::Parser;

mod color;
mod pty;
mod proxy;
mod terminal;

#[derive(Parser)]
#[command(
    name = "gscreen",
    version = "0.1.0",
    about = "A true color command wrapper for terminal programs",
    author = "Gamunu Balagalla <gamunu@fastcode.io>"
)]
struct Args {
    /// The command to run
    #[arg(value_name = "COMMAND")]
    command: String,
    
    /// Arguments to pass to the command
    #[arg(value_name = "ARGS")]
    args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Validate that the command exists
    if which::which(&args.command).is_err() {
        anyhow::bail!("Command '{}' not found in PATH", args.command);
    }
    
    println!("Starting {} with true color support...", args.command);
    
    // Set up terminal for true color support
    terminal::setup_true_color_environment()?;
    
    // Spawn the command in a PTY
    let mut pty_pair = pty::create_pty_with_command(&args.command, &args.args)
        .context("Failed to create PTY")?;
    
    // Start bidirectional I/O proxy
    let result = proxy::run_proxy(&mut pty_pair)
        .await
        .context("I/O proxy failed");
    
    // Clean up terminal
    terminal::restore_terminal()?;
    
    result
}
