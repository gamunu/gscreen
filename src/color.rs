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
        assert!((232..=255).contains(&mid_gray));
        assert!((232..=255).contains(&light_gray));
        assert!((232..=255).contains(&dark_gray));

        // Light gray should have higher index than dark gray
        assert!(light_gray > dark_gray);
    }
}
