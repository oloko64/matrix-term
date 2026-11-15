//! Draws the rain state with crossterm. Only cells that changed are touched,
//! so the screen is never fully cleared mid-animation (no flicker).

use std::io::{self, BufWriter, Stdout, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::{
    config::{Config, Rgb},
    rain::{Drop, Rain},
};

pub struct Renderer {
    out: BufWriter<Stdout>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            out: BufWriter::with_capacity(1 << 16, io::stdout()),
        }
    }

    pub fn clear(&mut self) -> io::Result<()> {
        queue!(self.out, ResetColor, Clear(ClearType::All))?;
        self.out.flush()
    }

    pub fn draw(&mut self, rain: &Rain, cfg: &Config) -> io::Result<()> {
        let height = rain.height() as i32;
        for drop in rain.drops() {
            for &row in &drop.vacated {
                if (0..height).contains(&row) {
                    queue!(self.out, MoveTo(drop.x, row as u16), Print(' '))?;
                }
            }
            if !drop.moved || !drop.is_falling() {
                continue;
            }
            for (row, dist, ch) in drop.cells() {
                if !(0..height).contains(&row) {
                    continue;
                }
                let color = to_color(cell_color(drop, dist, cfg));
                queue!(
                    self.out,
                    MoveTo(drop.x, row as u16),
                    SetForegroundColor(color),
                    Print(ch)
                )?;
            }
        }
        queue!(self.out, ResetColor)?;
        self.out.flush()
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
