use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal,
};
use std::io::{self};

static mut TERMINAL_STATE: Option<TerminalState> = None;

struct TerminalState {
    was_raw_mode: bool,
    was_alternate_screen: bool,
}

pub fn setup_true_color_environment() -> Result<()> {
    unsafe {
        if TERMINAL_STATE.is_some() {
            return Ok(()); // Already set up
        }
        
        TERMINAL_STATE = Some(TerminalState {
            was_raw_mode: false,
            was_alternate_screen: false,
        });
    }
    
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
        terminal::disable_raw_mode()
            .context("Failed to disable raw mode")?;
    }
    
    // We don't need to leave alternate screen since we never entered it
    // The child process should handle its own screen mode
    
    unsafe {
        TERMINAL_STATE = None;
    }
    
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

pub fn enable_mouse_support() -> Result<()> {
    execute!(
        io::stdout(),
        crossterm::event::EnableMouseCapture
    ).context("Failed to enable mouse capture")?;
    
    Ok(())
}

pub fn disable_mouse_support() -> Result<()> {
    execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture
    ).context("Failed to disable mouse capture")?;
    
    Ok(())
}

pub fn clear_screen() -> Result<()> {
    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    ).context("Failed to clear screen")?;
    
    Ok(())
}

// Test true color output
pub fn test_true_colors() -> Result<()> {
    println!("Testing true color output:");
    
    // Test RGB colors
    for i in 0..=255 {
        let r = i;
        let g = 255 - i;
        let b = i / 2;
        
        execute!(
            io::stdout(),
            crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb { r, g, b }),
            crossterm::style::Print("█"),
        )?;
        
        if i % 32 == 31 {
            execute!(
                io::stdout(),
                crossterm::style::ResetColor,
                crossterm::style::Print("\n")
            )?;
        }
    }
    
    execute!(
        io::stdout(),
        crossterm::style::ResetColor,
        crossterm::style::Print("\n")
    )?;
    
    Ok(())
}