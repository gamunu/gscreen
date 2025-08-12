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
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use portable_pty::PtyPair;
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use vte::Parser;

use crate::vte_handler::{InputVteHandler, VteHandler};

pub async fn run_proxy(pty_pair: &mut PtyPair, has_osc_support: bool) -> Result<()> {
    // Check if stdin is a TTY
    let stdin_is_tty = crossterm::tty::IsTty::is_tty(&std::io::stdin());

    // Enable raw mode only if stdin is a TTY
    if stdin_is_tty {
        let _ = crossterm::terminal::enable_raw_mode();
    }

    // Clone the reader for the background thread
    let mut reader = pty_pair
        .master
        .try_clone_reader()
        .context("Failed to clone PTY reader")?;

    // Get a writer handle
    let writer = pty_pair
        .master
        .take_writer()
        .context("Failed to get PTY writer")?;

    // Spawn a thread to handle PTY output -> stdout with VTE parsing
    let output_handle = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let stdout = std::io::stdout();

        // Create VTE parser and handler with capability info
        let mut parser = Parser::new();
        let mut vte_handler = VteHandler::new(Box::new(stdout), has_osc_support);

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    // PTY closed, child process exited
                    break;
                }
                Ok(n) => {
                    // Process bytes through VTE parser
                    for &byte in &buffer[..n] {
                        parser.advance(&mut vte_handler, byte);
                    }
                }
                Err(_) => {
                    // Read error, probably PTY closed
                    break;
                }
            }
        }
    });

    // Handle input differently based on whether stdin is a TTY
    if stdin_is_tty {
        // TTY mode: use crossterm event handling for interactive input
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
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    } else {
        // Non-TTY mode: copy stdin to PTY with VTE processing for terminal responses
        let writer = writer;
        let stdin_thread = thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buffer = [0u8; 4096];

            // Create VTE parser and input handler for processing terminal responses
            let mut parser = Parser::new();
            let mut input_handler = InputVteHandler::new(Box::new(writer));

            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        // Process bytes through input VTE parser to handle terminal responses
                        for &byte in &buffer[..n] {
                            parser.advance(&mut input_handler, byte);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Wait for either the output or stdin thread to finish
        loop {
            if output_handle.is_finished() || stdin_thread.is_finished() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let _ = stdin_thread.join();
    }

    // Wait for output thread to finish
    let _ = output_handle.join();

    Ok(())
}

async fn read_user_input() -> Result<Option<Vec<u8>>> {
    // Poll for events with faster response for better mouse performance
    if event::poll(Duration::from_millis(1)).context("Failed to poll for events")? {
        match event::read().context("Failed to read event")? {
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            }) => {
                // Handle special key combinations
                if modifiers.contains(event::KeyModifiers::CONTROL) {
                    match c {
                        'c' => return Ok(Some(vec![0x03])), // Ctrl+C
                        'd' => return Ok(Some(vec![0x04])), // Ctrl+D
                        'z' => return Ok(Some(vec![0x1a])), // Ctrl+Z
                        _ => {
                            // Other Ctrl combinations
                            let ctrl_char = (c as u8)
                                .to_ascii_lowercase()
                                .wrapping_sub(b'a')
                                .wrapping_add(1);
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
            Event::Mouse(mouse_event) => {
                // Handle mouse events - convert to appropriate escape sequences
                use crossterm::event::{MouseButton, MouseEventKind};

                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Mouse button down - send SGR mouse report
                        let sequence = format!(
                            "\x1b[<0;{};{}M",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        // Mouse button up
                        let sequence = format!(
                            "\x1b[<0;{};{}m",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Down(MouseButton::Right) => {
                        let sequence = format!(
                            "\x1b[<2;{};{}M",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Up(MouseButton::Right) => {
                        let sequence = format!(
                            "\x1b[<2;{};{}m",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Down(MouseButton::Middle) => {
                        let sequence = format!(
                            "\x1b[<1;{};{}M",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Up(MouseButton::Middle) => {
                        let sequence = format!(
                            "\x1b[<1;{};{}m",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        let sequence = format!(
                            "\x1b[<32;{};{}M",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::Moved => {
                        // Mouse movement without button pressed - don't send by default
                        // Most terminal applications only care about movement during drag
                        return Ok(None);
                    }
                    MouseEventKind::ScrollDown => {
                        let sequence = format!(
                            "\x1b[<65;{};{}M",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    MouseEventKind::ScrollUp => {
                        let sequence = format!(
                            "\x1b[<64;{};{}M",
                            mouse_event.column + 1,
                            mouse_event.row + 1
                        );
                        return Ok(Some(sequence.into_bytes()));
                    }
                    _ => {
                        // Other mouse events
                        return Ok(None);
                    }
                }
            }
            _ => {
                // Other events (resize, etc.)
                return Ok(None);
            }
        }
    }

    Ok(None)
}
