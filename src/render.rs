//! Draws the rain state with crossterm. Only cells that changed are touched,
//! so the screen is never fully cleared mid-animation (no flicker).

use std::io::{self, BufWriter, Stdout, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};
use rand::{RngExt, rngs::ThreadRng};

use crate::{
    config::{Config, Rgb},
    glitch::Frame,
    rain::{Drop, Rain},
};

const ALERT_RED: Rgb = Rgb(255, 40, 60);
const ALERT_WHITE: Rgb = Rgb(255, 255, 255);

pub struct Renderer {
    out: BufWriter<Stdout>,
    rng: ThreadRng,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            out: BufWriter::with_capacity(1 << 16, io::stdout()),
            rng: rand::rng(),
        }
    }

    pub fn clear(&mut self) -> io::Result<()> {
        queue!(self.out, ResetColor, Clear(ClearType::All))?;
        self.out.flush()
    }

    /// Draws one frame: wipes glitch leftovers, draws the rain, then the
    /// glitch overlays on top. `Frame::default()` = plain rain.
    pub fn draw(&mut self, rain: &Rain, cfg: &Config, fx: &Frame) -> io::Result<()> {
        let height = rain.height() as i32;

        for &row in &fx.clear_rows {
            queue!(self.out, MoveTo(0, row), Clear(ClearType::CurrentLine))?;
        }

        for drop in rain.drops() {
            for &row in &drop.vacated {
                if (0..height).contains(&row) {
                    queue!(self.out, MoveTo(drop.x, row as u16), Print(' '))?;
                }
            }
            if !drop.is_falling() || !(drop.moved || fx.force_redraw) {
                continue;
            }
            for (row, dist, ch) in drop.cells() {
                if !(0..height).contains(&row) {
                    continue;
                }
                let color = self.corrupt(cell_color(drop, dist, cfg), fx.corrupt_chance);
                queue!(
                    self.out,
                    MoveTo(drop.x, row as u16),
                    SetForegroundColor(to_color(color)),
                    Print(ch)
                )?;
            }
        }

        for span in &fx.spans {
            let text: String = span.text.iter().collect();
            queue!(
                self.out,
                MoveTo(span.col, span.row),
                SetForegroundColor(to_color(span.style.fg))
            )?;
            if let Some(bg) = span.style.bg {
                queue!(self.out, SetBackgroundColor(to_color(bg)))?;
            }
            if span.style.bold {
                queue!(self.out, SetAttribute(Attribute::Bold))?;
            }
            queue!(self.out, Print(text), SetAttribute(Attribute::Reset))?;
        }

        queue!(self.out, ResetColor)?;
        self.out.flush()
    }

    /// With probability `chance`, swaps the color for a red/white alert.
    fn corrupt(&mut self, color: Rgb, chance: f32) -> Rgb {
        if chance > 0.0 && self.rng.random_bool(f64::from(chance.min(1.0))) {
            if self.rng.random_bool(0.7) {
                ALERT_RED
            } else {
                ALERT_WHITE
            }
        } else {
            color
        }
    }
}

/// Head gets `head_color`; the trail fades from `trail_color` to ~15%.
fn cell_color(drop: &Drop, dist: usize, cfg: &Config) -> Rgb {
    if dist == 0 {
        return cfg.head_color;
    }
    let t = (dist - 1) as f32 / drop.len.max(1) as f32;
    let k = 1.0 - 0.85 * t;
    let Rgb(r, g, b) = cfg.trail_color;
    let scale = |c: u8| (c as f32 * k) as u8;
    Rgb(scale(r), scale(g), scale(b))
}

fn to_color(Rgb(r, g, b): Rgb) -> Color {
    Color::Rgb { r, g, b }
}
