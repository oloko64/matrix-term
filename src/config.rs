//! Runtime settings. Everything tunable lives here so future CLI flags /
//! config files only need to produce a `Config`.

use std::ops::RangeInclusive;

use crate::charset::Charset;

/// Plain RGB color, independent of the terminal backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone)]
pub struct Config {
    /// Frames (simulation ticks) per second.
    pub fps: u32,
    /// Distance between rain columns (1 = every terminal column).
    pub column_spacing: u16,
    /// Trail length range, in rows.
    pub trail_len: RangeInclusive<u16>,
    /// Fall speed range, in rows per tick.
    pub speed: RangeInclusive<f32>,
    /// Maximum ticks a column waits before (re)spawning a drop.
    pub max_spawn_delay: u32,
    /// Color of the leading glyph.
    pub head_color: Rgb,
    /// Color of the trail right behind the head; fades towards the tail.
    pub trail_color: Rgb,
    /// Glyphs the rain is made of.
    pub charset: Charset,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fps: 30,
            column_spacing: 2,
            trail_len: 6..=30,
            speed: 0.3..=1.0,
            max_spawn_delay: 90,
            head_color: Rgb(220, 255, 220),
            trail_color: Rgb(0, 255, 70),
            charset: Charset::matrix(),
        }
    }
}
