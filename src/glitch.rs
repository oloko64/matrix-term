//! "Hacking" glitch mode: a small state machine that, once per frame,
//! describes which glitch effects to apply. Pure logic, no terminal I/O.

use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use crate::charset::Charset;

/// The kinds of glitch events. Each event picks one at random.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Horizontal bands of crackling noise glyphs.
    Noise,
    /// Trail glyphs flash in alert colors.
    Color,
    /// Glyphs inside the trails change while falling.
    Mutate,
    /// A blinking "breach" message in the middle of the screen.
    Message,
}

impl Kind {
    #[cfg(test)]
    pub const ALL: [Kind; 4] = [Kind::Noise, Kind::Color, Kind::Mutate, Kind::Message];

    /// Event duration range, in seconds (before intensity scaling).
    fn duration_secs(self) -> (f32, f32) {
        match self {
            Kind::Noise => (0.15, 0.5),
            Kind::Color => (0.3, 0.8),
            Kind::Mutate => (0.5, 1.5),
            Kind::Message => (1.0, 2.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlitchConfig {
    /// 0.0 = rare and subtle, 1.0 = chaos.
    pub intensity: f32,
    /// Effects events are picked from. Must not be empty to do anything.
    pub effects: Vec<Kind>,
}

/// A horizontal run of noise glyphs.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseBand {
    pub row: u16,
    pub start: u16,
    pub glyphs: Vec<char>,
    pub bright: bool,
}

/// A message drawn on top of the rain.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub row: u16,
    pub col: u16,
    pub text: Vec<char>,
}

/// What to do this frame. `Frame::default()` means "no glitch".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    pub noise: Vec<NoiseBand>,
    /// Chance (0..=1) that a drawn trail cell uses an alert color.
    pub corrupt_chance: f32,
    /// Chance (0..=1) that each trail glyph changes this frame.
    pub mutate_chance: f32,
    pub message: Option<Message>,
    /// Rows to wipe before drawing (overlay leftovers).
    pub clear_rows: Vec<u16>,
    /// Repaint every falling trail, not just the ones that moved.
    pub force_redraw: bool,
}

const PHRASES: &[&str] = &[
    "SYSTEM BREACH",
    "ACCESS GRANTED",
    "INTRUSION DETECTED",
    "CONNECTION TRACED",
    "FIREWALL DOWN",
    "ROOT ACCESS OBTAINED",
    "DECRYPTING...",
];

struct Active {
    kind: Kind,
    ticks_left: u32,
    tick: u32,
    /// Screen rows this event draws overlays on (wiped when it ends).
    rows: Vec<u16>,
    phrase: &'static str,
}

pub struct Glitch {
    cfg: GlitchConfig,
    fps: u32,
    cooldown: u32,
    active: Option<Active>,
    rng: ThreadRng,
}

impl Glitch {
    pub fn new(mut cfg: GlitchConfig, fps: u32) -> Self {
        cfg.intensity = cfg.intensity.clamp(0.0, 1.0);
        let mut glitch = Self {
            cfg,
            fps: fps.max(1),
            cooldown: 0,
            active: None,
            rng: rand::rng(),
        };
        glitch.reset();
        glitch
    }

    /// Cancels any running event and starts a fresh cooldown.
    pub fn reset(&mut self) {
        self.active = None;
        self.cooldown = self.next_cooldown();
    }

    #[cfg(test)]
    pub fn active_kind(&self) -> Option<Kind> {
        self.active.as_ref().map(|a| a.kind)
    }

    /// Mean gap between events: ~8s at intensity 0, ~1s at intensity 1.
    fn mean_gap_secs(&self) -> f32 {
        8.0 - 7.0 * self.cfg.intensity
    }

    /// Longest possible cooldown, in frames.
    #[cfg(test)]
    pub fn max_cooldown(&self) -> u32 {
        self.ticks(self.mean_gap_secs() * 1.5)
    }

    fn ticks(&self, secs: f32) -> u32 {
        (secs * self.fps as f32).round().max(1.0) as u32
    }

    fn next_cooldown(&mut self) -> u32 {
        let mean = self.mean_gap_secs();
        let secs = self.rng.random_range(mean * 0.5..=mean * 1.5);
        self.ticks(secs)
    }

    /// Advances one frame and returns the effects to apply.
    pub fn update(&mut self, width: u16, height: u16, charset: &Charset) -> Frame {
        let mut fx = Frame::default();
        if width == 0 || height == 0 {
            return fx;
        }

        if self.active.is_none() {
            if self.cooldown > 0 {
                self.cooldown -= 1;
                return fx;
            }
            match self.start(height) {
                Some(active) => self.active = Some(active),
                None => {
                    self.cooldown = self.next_cooldown();
                    return fx;
                }
            }
        }

        let mut active = self.active.take().expect("checked above");
        if active.ticks_left == 0 {
            // Event over: repair the screen and go back to idle.
            fx.clear_rows = active.rows;
            fx.force_redraw = true;
            self.cooldown = self.next_cooldown();
            return fx;
        }
        active.ticks_left -= 1;
        active.tick += 1;
        self.fill(&active, &mut fx, width, height, charset);
        self.active = Some(active);
        fx
    }

    fn start(&mut self, height: u16) -> Option<Active> {
        let kind = *self.cfg.effects.choose(&mut self.rng)?;
        let intensity = self.cfg.intensity;

        let (lo, hi) = kind.duration_secs();
        let scale = 0.7 + 0.6 * intensity;
        let secs = self.rng.random_range(lo..=hi) * scale;
        let ticks_left = self.ticks(secs);

        let rows = match kind {
            Kind::Noise => {
                let bands = 1 + (intensity * 3.0).round() as usize;
                let mut rows: Vec<u16> = (0..bands)
                    .map(|_| self.rng.random_range(0..height))
                    .collect();
                rows.sort_unstable();
                rows.dedup();
                rows
            }
            Kind::Message => vec![height / 2],
            Kind::Color | Kind::Mutate => Vec::new(),
        };
        let phrase = PHRASES
            .choose(&mut self.rng)
            .copied()
            .unwrap_or("SYSTEM BREACH");

        Some(Active {
            kind,
            ticks_left,
            tick: 0,
            rows,
            phrase,
        })
    }

    fn fill(&mut self, a: &Active, fx: &mut Frame, width: u16, height: u16, charset: &Charset) {
        let intensity = self.cfg.intensity;
        match a.kind {
            Kind::Noise => {
                for &row in a.rows.iter().filter(|&&r| r < height) {
                    let start = self.rng.random_range(0..width);
                    let len = self.rng.random_range(1..=width - start);
                    fx.noise.push(NoiseBand {
                        row,
                        start,
                        glyphs: (0..len).map(|_| charset.random(&mut self.rng)).collect(),
                        bright: self.rng.random_bool(0.5),
                    });
                }
            }
            Kind::Color => {
                fx.corrupt_chance = 0.15 + 0.35 * intensity;
                fx.force_redraw = true;
            }
            Kind::Mutate => {
                fx.mutate_chance = 0.1 + 0.3 * intensity;
                fx.force_redraw = true;
            }
            Kind::Message => {
                let row = a.rows[0].min(height - 1);
                // Blink: visible 4 frames out of 6.
                if a.tick % 6 >= 4 {
                    fx.clear_rows.push(row);
                    fx.force_redraw = true;
                    return;
                }
                let noise = 0.05 + 0.1 * intensity as f64;
                let text: Vec<char> = format!(" {} ", a.phrase)
                    .chars()
                    .take(width as usize)
                    .map(|c| {
                        if c != ' ' && self.rng.random_bool(noise) {
                            charset.random(&mut self.rng)
                        } else {
                            c
                        }
                    })
                    .collect();
                let col = (width - text.len() as u16) / 2;
                fx.message = Some(Message { row, col, text });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glitch(intensity: f32, effects: &[Kind]) -> Glitch {
        let cfg = GlitchConfig {
            intensity,
            effects: effects.to_vec(),
        };
        Glitch::new(cfg, 30)
    }

    #[test]
    fn event_starts_within_max_cooldown() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &Kind::ALL);
        for _ in 0..=g.max_cooldown() {
            g.update(80, 24, &charset);
            if g.active_kind().is_some() {
                return;
            }
        }
        panic!("no glitch event started");
    }

    #[test]
    fn only_enabled_kinds_run() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &[Kind::Color]);
        let mut seen = false;
        for _ in 0..3000 {
            g.update(80, 24, &charset);
            if let Some(kind) = g.active_kind() {
                assert_eq!(kind, Kind::Color);
                seen = true;
            }
        }
        assert!(seen);
    }

    #[test]
    fn no_effects_means_no_events() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &[]);
        for _ in 0..3000 {
            assert_eq!(g.update(80, 24, &charset), Frame::default());
        }
    }

    #[test]
    fn noise_event_ends_by_clearing_its_rows() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &[Kind::Noise]);
        let mut noise_rows = Vec::new();
        for _ in 0..3000 {
            let was_active = g.active_kind().is_some();
            let fx = g.update(80, 24, &charset);
            noise_rows.extend(fx.noise.iter().map(|b| b.row));
            if was_active && g.active_kind().is_none() {
                assert!(fx.force_redraw);
                assert!(noise_rows.iter().all(|r| fx.clear_rows.contains(r)));
                return;
            }
        }
        panic!("noise event never ended");
    }

    #[test]
    fn overlays_stay_inside_the_screen() {
        let charset = Charset::matrix();
        for (w, h) in [(80, 24), (5, 3), (1, 1)] {
            let mut g = glitch(1.0, &[Kind::Noise, Kind::Message]);
            for _ in 0..2000 {
                let fx = g.update(w, h, &charset);
                for band in &fx.noise {
                    assert!(band.row < h);
                    assert!(band.start as usize + band.glyphs.len() <= w as usize);
                }
                if let Some(m) = &fx.message {
                    assert!(m.row < h);
                    assert!(m.col as usize + m.text.len() <= w as usize);
                }
                assert!(fx.clear_rows.iter().all(|&r| r < h));
            }
        }
    }
}
