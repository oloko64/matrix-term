//! Command-line interface. Parses flags and turns them into a `Config`.

use clap::{CommandFactory, Parser, ValueEnum, error::ErrorKind};

use crate::{
    charset::Charset,
    config::{Config, Rgb, defaults},
    glitch::{self, GlitchConfig},
};

/// Which glyphs the rain is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum CharsetChoice {
    /// Half-width katakana, digits and symbols, like the movie.
    #[default]
    Matrix,
    /// Printable ASCII only (letters, digits, symbols).
    Ascii,
}

impl From<CharsetChoice> for Charset {
    fn from(choice: CharsetChoice) -> Self {
        match choice {
            CharsetChoice::Matrix => Charset::matrix(),
            CharsetChoice::Ascii => Charset::ascii(),
        }
    }
}

/// Glitch effects for `--glitch-effects`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GlitchEffectArg {
    /// Horizontal bands of crackling noise.
    Noise,
    /// Glyphs flash red/white like an intrusion alert.
    Color,
    /// Glyphs change while they fall.
    Mutate,
    /// A blinking "SYSTEM BREACH"-style message.
    Message,
}

impl GlitchEffectArg {
    fn all() -> Vec<Self> {
        vec![Self::Noise, Self::Color, Self::Mutate, Self::Message]
    }
}

impl From<GlitchEffectArg> for glitch::Kind {
    fn from(arg: GlitchEffectArg) -> Self {
        match arg {
            GlitchEffectArg::Noise => glitch::Kind::Noise,
            GlitchEffectArg::Color => glitch::Kind::Color,
            GlitchEffectArg::Mutate => glitch::Kind::Mutate,
            GlitchEffectArg::Message => glitch::Kind::Message,
        }
    }
}

/// Matrix digital rain for your terminal.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Frames per second.
    #[arg(long, default_value_t = defaults::FPS,
          value_parser = clap::value_parser!(u32).range(1..=240))]
    pub fps: u32,

    /// Gap between rain columns (1 = densest).
    #[arg(short, long, default_value_t = defaults::COLUMN_SPACING,
          value_parser = clap::value_parser!(u16).range(1..))]
    pub spacing: u16,

    /// Shortest trail, in rows.
    #[arg(long, default_value_t = defaults::MIN_TRAIL,
          value_parser = clap::value_parser!(u16).range(1..))]
    pub min_trail: u16,

    /// Longest trail, in rows.
    #[arg(long, default_value_t = defaults::MAX_TRAIL,
          value_parser = clap::value_parser!(u16).range(1..))]
    pub max_trail: u16,

    /// Slowest fall speed, in rows per frame.
    #[arg(long, default_value_t = defaults::MIN_SPEED, value_parser = positive_f32)]
    pub min_speed: f32,

    /// Fastest fall speed, in rows per frame.
    #[arg(long, default_value_t = defaults::MAX_SPEED, value_parser = positive_f32)]
    pub max_speed: f32,

    /// Max frames a column waits before dropping again.
    #[arg(long, default_value_t = defaults::MAX_SPAWN_DELAY)]
    pub max_delay: u32,

    /// Trail color, as hex (e.g. #00ff46).
    #[arg(short, long, default_value_t = defaults::TRAIL_COLOR)]
    pub color: Rgb,

    /// Head (leading glyph) color, as hex.
    #[arg(long, default_value_t = defaults::HEAD_COLOR)]
    pub head_color: Rgb,

    /// Glyphs the rain is made of.
    #[arg(long, value_enum, default_value_t)]
    pub charset: CharsetChoice,

    /// Hacking mode: random glitch events while the rain falls.
    #[arg(short, long)]
    pub glitch: bool,

    /// Glitch strength and frequency, from 0.0 (rare) to 1.0 (chaos).
    #[arg(long, default_value_t = defaults::GLITCH_INTENSITY,
          value_parser = unit_f32, requires = "glitch")]
    pub glitch_intensity: f32,

    /// Glitch effects to rotate between (comma-separated).
    #[arg(long, value_enum, value_delimiter = ',',
          default_values_t = GlitchEffectArg::all(), requires = "glitch")]
    pub glitch_effects: Vec<GlitchEffectArg>,
}

impl Cli {
    /// Builds the config, exiting with a clap error on invalid combinations.
    pub fn into_config(self) -> Config {
        if self.min_trail > self.max_trail {
            fail("--min-trail must be <= --max-trail");
        }
        if self.min_speed > self.max_speed {
            fail("--min-speed must be <= --max-speed");
        }
        Config {
            fps: self.fps,
            column_spacing: self.spacing,
            trail_len: self.min_trail..=self.max_trail,
            speed: self.min_speed..=self.max_speed,
            max_spawn_delay: self.max_delay,
            head_color: self.head_color,
            trail_color: self.color,
            charset: self.charset.into(),
            glitch: self.glitch.then(|| {
                let mut effects: Vec<glitch::Kind> = Vec::new();
                for kind in self.glitch_effects.into_iter().map(Into::into) {
                    if !effects.contains(&kind) {
                        effects.push(kind);
                    }
                }
                GlitchConfig {
                    intensity: self.glitch_intensity,
                    effects,
                }
            }),
        }
    }
}

fn fail(msg: &str) -> ! {
    Cli::command()
        .error(ErrorKind::ArgumentConflict, msg)
        .exit()
}

fn positive_f32(s: &str) -> Result<f32, String> {
    let v: f32 = s.parse().map_err(|_| format!("`{s}` is not a number"))?;
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(format!("`{s}` must be a positive number"))
    }
}

fn unit_f32(s: &str) -> Result<f32, String> {
    let v: f32 = s.parse().map_err(|_| format!("`{s}` is not a number"))?;
    if (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!("`{s}` must be between 0.0 and 1.0"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn defaults_match_config_default() {
        let cfg = Cli::parse_from(["matrix-term"]).into_config();
        let def = Config::default();
        assert_eq!(cfg.fps, def.fps);
        assert_eq!(cfg.column_spacing, def.column_spacing);
        assert_eq!(cfg.trail_len, def.trail_len);
        assert_eq!(cfg.speed, def.speed);
        assert_eq!(cfg.trail_color, def.trail_color);
    }

    #[test]
    fn flags_override_defaults() {
        let cfg = Cli::parse_from(["matrix-term", "--fps", "60", "-s", "1", "-c", "#ff0000"])
            .into_config();
        assert_eq!(cfg.fps, 60);
        assert_eq!(cfg.column_spacing, 1);
        assert_eq!(cfg.trail_color, Rgb(255, 0, 0));
    }

    #[test]
    fn charset_defaults_to_matrix() {
        assert_eq!(
            Cli::parse_from(["matrix-term"]).charset,
            CharsetChoice::Matrix
        );
        let cli = Cli::parse_from(["matrix-term", "--charset", "ascii"]);
        assert_eq!(cli.charset, CharsetChoice::Ascii);
    }

    #[test]
    fn glitch_is_off_by_default() {
        assert!(
            Cli::parse_from(["matrix-term"])
                .into_config()
                .glitch
                .is_none()
        );
    }

    #[test]
    fn glitch_defaults_to_all_effects() {
        let cfg = Cli::parse_from(["matrix-term", "--glitch"]).into_config();
        let g = cfg.glitch.expect("glitch enabled");
        assert_eq!(g.effects, glitch::Kind::ALL.to_vec());
        assert_eq!(g.intensity, defaults::GLITCH_INTENSITY);
    }

    #[test]
    fn glitch_effects_can_be_selected() {
        let cfg = Cli::parse_from([
            "matrix-term",
            "--glitch",
            "--glitch-intensity",
            "0.9",
            "--glitch-effects",
            "noise,message,noise",
        ])
        .into_config();
        let g = cfg.glitch.expect("glitch enabled");
        assert_eq!(g.effects, vec![glitch::Kind::Noise, glitch::Kind::Message]);
        assert_eq!(g.intensity, 0.9);
    }

    #[test]
    fn glitch_options_require_glitch() {
        assert!(Cli::try_parse_from(["matrix-term", "--glitch-intensity", "0.5"]).is_err());
        assert!(Cli::try_parse_from(["matrix-term", "--glitch-effects", "noise"]).is_err());
    }

    #[test]
    fn rejects_bad_values() {
        assert!(Cli::try_parse_from(["matrix-term", "-g", "--glitch-intensity", "1.5"]).is_err());
        assert!(Cli::try_parse_from(["matrix-term", "-g", "--glitch-effects", "lasers"]).is_err());
        assert!(Cli::try_parse_from(["matrix-term", "--charset", "emoji"]).is_err());
        assert!(Cli::try_parse_from(["matrix-term", "--fps", "0"]).is_err());
        assert!(Cli::try_parse_from(["matrix-term", "--min-speed", "-1"]).is_err());
        assert!(Cli::try_parse_from(["matrix-term", "-c", "green"]).is_err());
    }
}
