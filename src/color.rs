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

/// Color conversion utilities for translating 24-bit RGB to 256-color palette
use std::cmp;

/// Convert 24-bit RGB values to the closest 256-color palette index
pub fn rgb_to_256color(r: u8, g: u8, b: u8) -> u8 {
    // The 256-color palette consists of:
    // - Colors 0-15: Standard 16 ANSI colors
    // - Colors 16-231: 6x6x6 RGB color cube
    // - Colors 232-255: 24 grayscale colors

    // Check if it's a grayscale color (when R, G, B are very close)
    let max_diff = cmp::max(
        cmp::max((r as i16 - g as i16).abs(), (g as i16 - b as i16).abs()),
        (r as i16 - b as i16).abs(),
    );

    if max_diff < 8 {
        // It's grayscale, use the grayscale palette (colors 232-255)
        let gray_avg = ((r as u16 + g as u16 + b as u16) / 3) as u8;
        if gray_avg < 8 {
            return 16; // Black from the color cube
        } else if gray_avg > 238 {
            return 231; // White from the color cube
        } else {
            // Map to grayscale colors 232-255 (24 levels)
            let scaled = (gray_avg.saturating_sub(8) as u16 * 23 / 230) as u8;
            return 232 + scaled.min(23);
        }
    }

    // Convert to 6x6x6 RGB color cube (colors 16-231)
    let r6 = (r as u16 * 5 / 255) as u8;
    let g6 = (g as u16 * 5 / 255) as u8;
    let b6 = (b as u16 * 5 / 255) as u8;

    16 + (36 * r6) + (6 * g6) + b6
}

/// Convert true color ANSI escape sequences to 256-color equivalents
pub fn convert_truecolor_to_256(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        // Look for exact true color sequence patterns only
        if let Some(match_result) = try_match_truecolor_sequence(input, i) {
            match match_result {
                TrueColorMatch::Converted(converted_bytes, end_pos) => {
                    // Successfully converted true color sequence
                    output.extend_from_slice(&converted_bytes);
                    i = end_pos;
                    continue;
                }
                TrueColorMatch::NotTrueColor => {
                    // Not a true color sequence, just copy the current byte
                }
            }
        }

        // Copy character as-is
        output.push(input[i]);
        i += 1;
    }

    output
}

#[derive(Debug)]
enum TrueColorMatch {
    Converted(Vec<u8>, usize), // (converted bytes, new position)
    NotTrueColor,
}

/// Try to match and convert a true color sequence at the given position
fn try_match_truecolor_sequence(input: &[u8], start: usize) -> Option<TrueColorMatch> {
    // Need at least enough bytes for the shortest true color sequence
    if start + 12 > input.len() {
        // ESC[38;2;0;0;0m minimum 12 bytes
        return Some(TrueColorMatch::NotTrueColor);
    }

    // Must start with ESC[
    if input[start] != 0x1b || input[start + 1] != b'[' {
        return Some(TrueColorMatch::NotTrueColor);
    }

    // Check for exact true color pattern: ESC[38;2; or ESC[48;2;
    let pattern_38 = b"\x1b[38;2;";
    let pattern_48 = b"\x1b[48;2;";

    let is_fg = input[start..].starts_with(pattern_38);
    let is_bg = input[start..].starts_with(pattern_48);

    if !is_fg && !is_bg {
        return Some(TrueColorMatch::NotTrueColor);
    }

    let prefix_len = 7; // Length of "ESC[38;2;" or "ESC[48;2;"
    let mut pos = start + prefix_len;
    let mut rgb_values = Vec::new();
    let mut current_num = String::new();

    // Parse exactly 3 RGB values
    while pos < input.len() && rgb_values.len() < 3 {
        let c = input[pos] as char;

        if c.is_ascii_digit() {
            current_num.push(c);
        } else if c == ';' || c == 'm' {
            if !current_num.is_empty() {
                if let Ok(num) = current_num.parse::<u16>() {
                    if num <= 255 {
                        rgb_values.push(num);
                    } else {
                        // Invalid RGB value
                        return Some(TrueColorMatch::NotTrueColor);
                    }
                }
                current_num.clear();
            }

            if c == 'm' {
                break;
            }
        } else {
            // Invalid character in RGB sequence
            return Some(TrueColorMatch::NotTrueColor);
        }

        pos += 1;
    }

    // Must have exactly 3 RGB values and end with 'm'
    if rgb_values.len() == 3 && pos < input.len() && input[pos] == b'm' {
        let r = rgb_values[0] as u8;
        let g = rgb_values[1] as u8;
        let b = rgb_values[2] as u8;
        let color_idx = rgb_to_256color(r, g, b);

        let converted = if is_fg {
            format!("\x1b[38;5;{}m", color_idx).into_bytes()
        } else {
            format!("\x1b[48;5;{}m", color_idx).into_bytes()
        };

        return Some(TrueColorMatch::Converted(converted, pos + 1));
    }

    Some(TrueColorMatch::NotTrueColor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_256color_basic() {
        // Test pure colors
        assert_eq!(rgb_to_256color(255, 0, 0), 196); // Pure red
        assert_eq!(rgb_to_256color(0, 255, 0), 46); // Pure green
        assert_eq!(rgb_to_256color(0, 0, 255), 21); // Pure blue
        assert_eq!(rgb_to_256color(255, 255, 255), 231); // White
        assert_eq!(rgb_to_256color(0, 0, 0), 16); // Black
    }

    #[test]
    fn test_grayscale_detection() {
        // Test that grayscale colors get mapped to grayscale palette
        let mid_gray = rgb_to_256color(128, 128, 128);
        let light_gray = rgb_to_256color(200, 200, 200);
        let dark_gray = rgb_to_256color(50, 50, 50);

        // These should all be in the grayscale range (232-255)
        assert!(mid_gray >= 232 && mid_gray <= 255);
        assert!(light_gray >= 232 && light_gray <= 255);
        assert!(dark_gray >= 232 && dark_gray <= 255);

        // Light gray should have higher index than dark gray
        assert!(light_gray > dark_gray);
    }

    #[test]
    fn test_true_color_conversion() {
        let input = b"\x1b[38;2;255;128;64m";
        let output = convert_truecolor_to_256(input);
        let expected_color = rgb_to_256color(255, 128, 64);
        let expected = format!("\x1b[38;5;{}m", expected_color);
        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn test_non_color_sequences_preserved() {
        // Test that non-color ANSI sequences are preserved
        let input = b"\x1b[?25h\x1b[?1049h\x1b[?2004h";
        let output = convert_truecolor_to_256(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_complex_sequences_preserved() {
        // Test more complex ANSI sequences that might be corrupted
        let input = b"\x1b[40;44H\x1b[28;2c\x1b[282c";
        let output = convert_truecolor_to_256(input);
        assert_eq!(output, input);
    }
}
