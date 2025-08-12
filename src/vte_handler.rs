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

use std::io::{self, Write};
use vte::{Params, Perform};

use crate::color;

/// VTE Perform handler that processes terminal sequences and applies color conversion
pub struct VteHandler {
    writer: Box<dyn Write + Send>,
    has_osc_support: bool,
}

impl VteHandler {
    pub fn new(writer: Box<dyn Write + Send>, has_osc_support: bool) -> Self {
        Self {
            writer,
            has_osc_support,
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_bytes(s.as_bytes())
    }
}

impl Perform for VteHandler {
    fn print(&mut self, c: char) {
        let _ = self.write_string(&c.to_string());
    }

    fn execute(&mut self, byte: u8) {
        let _ = self.write_bytes(&[byte]);
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, c: char) {
        // DCS sequences - reconstruct and pass through
        let _ = self.write_string("\x1bP");
        self.write_params(params);
        for &intermediate in intermediates {
            let _ = self.write_bytes(&[intermediate]);
        }
        let _ = self.write_string(&c.to_string());
    }

    fn put(&mut self, byte: u8) {
        let _ = self.write_bytes(&[byte]);
    }

    fn unhook(&mut self) {
        // End of DCS sequence
        let _ = self.write_string("\x1b\\"); // ST terminator
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        if params.is_empty() {
            return;
        }

        let param_str = String::from_utf8_lossy(params[0]);

        // Handle OSC queries for terminals that don't support them
        if !self.has_osc_support {
            match param_str.as_ref() {
                "10" => {
                    // OSC 10: Foreground color query - respond with white
                    let _ = self.write_bytes(b"\x1b]10;rgb:ffff/ffff/ffff\x07");
                    return;
                }
                "11" => {
                    // OSC 11: Background color query - respond with black
                    let _ = self.write_bytes(b"\x1b]11;rgb:0000/0000/0000\x07");
                    return;
                }
                "12" => {
                    // OSC 12: Cursor color query - respond with white
                    let _ = self.write_bytes(b"\x1b]12;rgb:ffff/ffff/ffff\x07");
                    return;
                }
                _ => {
                    // For other OSC sequences, pass through normally
                }
            }
        }

        // For supported terminals or non-query OSC sequences, pass through
        let _ = self.write_bytes(b"\x1b]");

        // Write parameters with proper semicolon separation
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                let _ = self.write_bytes(b";");
            }
            let _ = self.write_bytes(param);
        }

        // Always use the terminator that was actually received
        if bell_terminated {
            let _ = self.write_bytes(b"\x07"); // BEL (^G)
        } else {
            let _ = self.write_bytes(b"\x1b\\"); // ST
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, c: char) {
        match c {
            'm' => {
                // SGR (Select Graphic Rendition) - handle colors specially
                self.handle_sgr_sequence(params);
            }
            _ => {
                // All other CSI sequences, pass through unchanged

                let _ = self.write_string("\x1b[");
                self.write_params(params);
                for &intermediate in intermediates {
                    let _ = self.write_bytes(&[intermediate]);
                }
                let _ = self.write_string(&c.to_string());
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        // ESC sequences - pass through
        let _ = self.write_bytes(b"\x1b");
        let _ = self.write_bytes(intermediates);
        let _ = self.write_bytes(&[byte]);
    }
}

impl VteHandler {
    fn write_params(&mut self, params: &Params) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                let _ = self.write_string(";");
            }
            for (j, &value) in param.iter().enumerate() {
                if j > 0 {
                    let _ = self.write_string(":");
                }
                let _ = self.write_string(&value.to_string());
            }
        }
    }

    fn handle_sgr_sequence(&mut self, params: &Params) {
        if params.is_empty() {
            // Reset
            let _ = self.write_string("\x1b[0m");
            return;
        }

        // Convert params to a vector for easier manipulation
        let param_vec: Vec<&[u16]> = params.iter().collect();
        let mut i = 0;

        while i < param_vec.len() {
            let param = param_vec[i];
            if param.is_empty() {
                i += 1;
                continue;
            }

            match param[0] {
                38 => {
                    // Foreground color
                    if let Some((converted, consumed)) =
                        self.handle_color_params_vec(&param_vec, i, false)
                    {
                        let _ = self.write_string(&converted);
                        i += consumed;
                    } else {
                        // Pass through unchanged
                        let _ = self.write_string("\x1b[38");
                        for param in param_vec.iter().skip(i + 1) {
                            if !param.is_empty() {
                                let _ = self.write_string(&format!(";{}", param[0]));
                            }
                        }
                        let _ = self.write_string("m");
                        return;
                    }
                }
                48 => {
                    // Background color
                    if let Some((converted, consumed)) =
                        self.handle_color_params_vec(&param_vec, i, true)
                    {
                        let _ = self.write_string(&converted);
                        i += consumed;
                    } else {
                        // Pass through unchanged
                        let _ = self.write_string("\x1b[48");
                        for param in param_vec.iter().skip(i + 1) {
                            if !param.is_empty() {
                                let _ = self.write_string(&format!(";{}", param[0]));
                            }
                        }
                        let _ = self.write_string("m");
                        return;
                    }
                }
                _ => {
                    // Other SGR parameters, pass through
                    let _ = self.write_string(&format!("\x1b[{}m", param[0]));
                    i += 1;
                }
            }
        }
    }

    fn handle_color_params_vec(
        &mut self,
        param_vec: &[&[u16]],
        start_idx: usize,
        is_background: bool,
    ) -> Option<(String, usize)> {
        if start_idx + 1 >= param_vec.len() {
            return None;
        }

        let color_type_param = param_vec[start_idx + 1];
        if color_type_param.is_empty() {
            return None;
        }

        match color_type_param[0] {
            2 => {
                // True color: 38;2;R;G;B or 48;2;R;G;B
                if start_idx + 4 < param_vec.len() {
                    let r_param = param_vec[start_idx + 2];
                    let g_param = param_vec[start_idx + 3];
                    let b_param = param_vec[start_idx + 4];

                    if !r_param.is_empty() && !g_param.is_empty() && !b_param.is_empty() {
                        let r = r_param[0];
                        let g = g_param[0];
                        let b = b_param[0];

                        if r <= 255 && g <= 255 && b <= 255 {
                            let color_idx = color::rgb_to_256color(r as u8, g as u8, b as u8);
                            let converted = if is_background {
                                format!("\x1b[48;5;{}m", color_idx)
                            } else {
                                format!("\x1b[38;5;{}m", color_idx)
                            };
                            return Some((converted, 5)); // Consumed 5 params: 38/48, 2, R, G, B
                        }
                    }
                }
                None
            }
            5 => {
                // 256-color: 38;5;N or 48;5;N - pass through unchanged
                if start_idx + 2 < param_vec.len() {
                    let color_param = param_vec[start_idx + 2];
                    if !color_param.is_empty() && color_param[0] <= 255 {
                        let converted = if is_background {
                            format!("\x1b[48;5;{}m", color_param[0])
                        } else {
                            format!("\x1b[38;5;{}m", color_param[0])
                        };
                        return Some((converted, 3)); // Consumed 3 params: 38/48, 5, N
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// InputVteHandler processes terminal responses (terminal -> application)
/// Unlike the output handler, this one passes sequences through without color conversion
pub struct InputVteHandler {
    writer: Box<dyn Write + Send>,
}

impl InputVteHandler {
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self { writer }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_bytes(s.as_bytes())
    }
}

impl Perform for InputVteHandler {
    fn print(&mut self, c: char) {
        let _ = self.write_string(&c.to_string());
    }

    fn execute(&mut self, byte: u8) {
        let _ = self.write_bytes(&[byte]);
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, c: char) {
        // DCS sequences - reconstruct and pass through unchanged
        let _ = self.write_string("\x1bP");
        self.write_params(params);
        for &intermediate in intermediates {
            let _ = self.write_bytes(&[intermediate]);
        }
        let _ = self.write_string(&c.to_string());
    }

    fn put(&mut self, byte: u8) {
        let _ = self.write_bytes(&[byte]);
    }

    fn unhook(&mut self) {
        // End of DCS sequence
        let _ = self.write_string("\x1b\\"); // ST terminator
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        // OSC sequences - pass through unchanged (no color conversion)
        let _ = self.write_bytes(b"\x1b]");

        // Write parameters with proper semicolon separation
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                let _ = self.write_bytes(b";");
            }
            let _ = self.write_bytes(param);
        }

        // Use the terminator that was actually received
        if bell_terminated {
            let _ = self.write_bytes(b"\x07"); // BEL (^G)
        } else {
            let _ = self.write_bytes(b"\x1b\\"); // ST
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, c: char) {
        // All CSI sequences pass through unchanged (no color processing for input)
        let _ = self.write_string("\x1b[");
        self.write_params(params);
        for &intermediate in intermediates {
            let _ = self.write_bytes(&[intermediate]);
        }
        let _ = self.write_string(&c.to_string());
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        // ESC sequences - pass through unchanged
        let _ = self.write_bytes(b"\x1b");
        let _ = self.write_bytes(intermediates);
        let _ = self.write_bytes(&[byte]);
    }
}

impl InputVteHandler {
    fn write_params(&mut self, params: &Params) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                let _ = self.write_string(";");
            }
            for (j, &value) in param.iter().enumerate() {
                if j > 0 {
                    let _ = self.write_string(":");
                }
                let _ = self.write_string(&value.to_string());
            }
        }
    }
}
