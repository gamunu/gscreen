use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use portable_pty::PtyPair;
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use crate::color;

pub async fn run_proxy(pty_pair: &mut PtyPair) -> Result<()> {
    // Enable raw mode for direct character input (ignore errors if not in a TTY)
    let _ = crossterm::terminal::enable_raw_mode();
    
    // Clone the reader for the background thread
    let mut reader = pty_pair.master.try_clone_reader()
        .context("Failed to clone PTY reader")?;
    
    // Get a writer handle
    let writer = pty_pair.master.take_writer()
        .context("Failed to get PTY writer")?;
    
    // Spawn a thread to handle PTY output -> stdout
    let output_handle = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stdout = std::io::stdout();
        
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    // PTY closed, child process exited
                    break;
                }
                Ok(n) => {
                    // Convert true colors to 256-color equivalents
                    let converted_output = color::convert_truecolor_to_256(&buffer[..n]);
                    
                    // Write converted output to stdout
                    if stdout.write_all(&converted_output).is_err() {
                        break;
                    }
                    if stdout.flush().is_err() {
                        break;
                    }
                }
                Err(_) => {
                    // Read error, probably PTY closed
                    break;
                }
            }
        }
    });
    
    // Main async loop for input handling
    let mut last_size = crossterm::terminal::size().unwrap_or((80, 24));
    let mut writer = writer;
    
    loop {
        // Check if output thread is still running
        if output_handle.is_finished() {
            break;
        }
        
        // Handle input events
        if let Ok(Some(input)) = read_user_input().await {
            // Write to PTY writer
            if writer.write_all(&input).is_err() {
                break;
            }
            if writer.flush().is_err() {
                break;
            }
        }
        
        // Handle window resize
        if let Ok(current_size) = crossterm::terminal::size() {
            if current_size != last_size {
                last_size = current_size;
                let size = portable_pty::PtySize {
                    rows: current_size.1,
                    cols: current_size.0,
                    pixel_width: 0,
                    pixel_height: 0,
                };
                let _ = pty_pair.master.resize(size);
            }
        }
        
        // Small delay to prevent busy waiting
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Wait for output thread to finish
    let _ = output_handle.join();
    
    Ok(())
}

async fn read_user_input() -> Result<Option<Vec<u8>>> {
    // Poll for events without blocking too long
    if event::poll(Duration::from_millis(10))
        .context("Failed to poll for events")? 
    {
        match event::read().context("Failed to read event")? {
            Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers, .. }) => {
                // Handle special key combinations
                if modifiers.contains(event::KeyModifiers::CONTROL) {
                    match c {
                        'c' => return Ok(Some(vec![0x03])), // Ctrl+C
                        'd' => return Ok(Some(vec![0x04])), // Ctrl+D
                        'z' => return Ok(Some(vec![0x1a])), // Ctrl+Z
                        _ => {
                            // Other Ctrl combinations
                            let ctrl_char = (c as u8).to_ascii_lowercase().wrapping_sub(b'a').wrapping_add(1);
                            return Ok(Some(vec![ctrl_char]));
                        }
                    }
                } else {
                    // Regular character
                    return Ok(Some(c.to_string().into_bytes()));
                }
            }
            Event::Key(KeyEvent { code, .. }) => {
                // Handle special keys
                let bytes = match code {
                    KeyCode::Enter => vec![b'\r'],
                    KeyCode::Tab => vec![b'\t'],
                    KeyCode::Backspace => vec![0x7f],
                    KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
                    KeyCode::Up => vec![0x1b, b'[', b'A'],
                    KeyCode::Down => vec![0x1b, b'[', b'B'],
                    KeyCode::Right => vec![0x1b, b'[', b'C'],
                    KeyCode::Left => vec![0x1b, b'[', b'D'],
                    KeyCode::Home => vec![0x1b, b'[', b'H'],
                    KeyCode::End => vec![0x1b, b'[', b'F'],
                    KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
                    KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
                    KeyCode::Esc => vec![0x1b],
                    KeyCode::F(n) => {
                        // Function keys F1-F12
                        match n {
                            1 => vec![0x1b, b'O', b'P'],
                            2 => vec![0x1b, b'O', b'Q'],
                            3 => vec![0x1b, b'O', b'R'],
                            4 => vec![0x1b, b'O', b'S'],
                            5 => vec![0x1b, b'[', b'1', b'5', b'~'],
                            6 => vec![0x1b, b'[', b'1', b'7', b'~'],
                            7 => vec![0x1b, b'[', b'1', b'8', b'~'],
                            8 => vec![0x1b, b'[', b'1', b'9', b'~'],
                            9 => vec![0x1b, b'[', b'2', b'0', b'~'],
                            10 => vec![0x1b, b'[', b'2', b'1', b'~'],
                            11 => vec![0x1b, b'[', b'2', b'3', b'~'],
                            12 => vec![0x1b, b'[', b'2', b'4', b'~'],
                            _ => return Ok(None),
                        }
                    }
                    _ => return Ok(None),
                };
                return Ok(Some(bytes));
            }
            _ => {
                // Ignore other events (mouse, resize, etc.)
                return Ok(None);
            }
        }
    }
    
    Ok(None)
}