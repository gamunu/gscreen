use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, PtyPair, PtySize};
use std::collections::HashMap;

pub fn create_pty_with_command(command: &str, args: &[String]) -> Result<PtyPair> {
    // Create a new PTY with the actual terminal size
    let (cols, rows) = crossterm::terminal::size()
        .unwrap_or((80, 24)); // fallback to 80x24 if detection fails
        
    let pty_size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };
    
    let pty_system = portable_pty::native_pty_system();
    let pty_pair = pty_system
        .openpty(pty_size)
        .context("Failed to open PTY")?;
    
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
    
    // Set environment variables
    for (key, value) in env_vars {
        cmd_builder.env(&key, &value);
    }
    
    // Spawn the command in the PTY slave
    let _child = pty_pair
        .slave
        .spawn_command(cmd_builder)
        .context("Failed to spawn command in PTY")?;
    
    Ok(pty_pair)
}

pub fn get_terminal_size() -> Result<PtySize> {
    let (cols, rows) = crossterm::terminal::size()
        .context("Failed to get terminal size")?;
    
    Ok(PtySize {
        rows: rows,
        cols: cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

pub fn resize_pty(pty_pair: &PtyPair) -> Result<()> {
    let size = get_terminal_size()?;
    pty_pair
        .master
        .resize(size)
        .context("Failed to resize PTY")?;
    Ok(())
}