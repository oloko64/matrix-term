//! Runtime settings. Everything tunable lives here; the CLI (or a future
//! config file) only needs to produce a `Config`.

use std::{fmt, ops::RangeInclusive, str::FromStr};

use crate::charset::Charset;

/// Default values, shared by `Config::default()` and the CLI help text.
pub mod defaults {
    use super::Rgb;

    pub const FPS: u32 = 30;
    pub const COLUMN_SPACING: u16 = 2;
    pub const MIN_TRAIL: u16 = 6;
    pub const MAX_TRAIL: u16 = 30;
    pub const MIN_SPEED: f32 = 0.3;
    pub const MAX_SPEED: f32 = 1.0;
    pub const MAX_SPAWN_DELAY: u32 = 90;
    pub const HEAD_COLOR: Rgb = Rgb(220, 255, 220);
    pub const TRAIL_COLOR: Rgb = Rgb(0, 255, 70);
}

/// Plain RGB color, independent of the terminal backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl FromStr for Rgb {
    type Err = String;

    /// Parses `#rrggbb` or `rrggbb`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        let err = || format!("invalid color `{s}`, expected hex like #00ff46");
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(err());
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| err());
        Ok(Rgb(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

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
        use defaults::*;
        Self {
            fps: FPS,
            column_spacing: COLUMN_SPACING,
            trail_len: MIN_TRAIL..=MAX_TRAIL,
            speed: MIN_SPEED..=MAX_SPEED,
            max_spawn_delay: MAX_SPAWN_DELAY,
            head_color: HEAD_COLOR,
            trail_color: TRAIL_COLOR,
            charset: Charset::matrix(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!("#00ff46".parse(), Ok(Rgb(0, 255, 70)));
        assert_eq!("DCFFDC".parse(), Ok(Rgb(220, 255, 220)));
        assert!("#00ff4".parse::<Rgb>().is_err());
        assert!("#zzzzzz".parse::<Rgb>().is_err());
        assert!("#ffé00".parse::<Rgb>().is_err());
    }

    #[test]
    fn display_round_trips() {
        let c = Rgb(0, 255, 70);
        assert_eq!(c.to_string().parse(), Ok(c));
    }
}
