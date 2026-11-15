//! Rain simulation: pure state, no terminal I/O.

use std::collections::VecDeque;

use rand::{Rng, RngExt, rngs::ThreadRng};

use crate::config::Config;

/// One falling trail of glyphs in a single column.
#[derive(Debug, Clone)]
pub struct Drop {
    pub x: u16,
    /// Fractional row of the head; the integer part is the drawn row.
    head: f32,
    /// Maximum trail length (including the head).
    pub len: u16,
    speed: f32,
    /// Ticks left before the drop starts falling.
    delay: u32,
    /// Glyphs from head (front) to tail (back). Each glyph stays fixed on
    /// screen while the drop moves past it.
    pub trail: VecDeque<char>,
    /// Rows the tail left during the last update (to be erased).
    pub vacated: Vec<i32>,
    /// Whether the head moved to a new row during the last update.
    pub moved: bool,
}

impl Drop {
    fn new<R: Rng + ?Sized>(x: u16, cfg: &Config, rng: &mut R) -> Self {
        let mut drop = Self {
            x,
            head: 0.0,
            len: 0,
            speed: 0.0,
            delay: 0,
            trail: VecDeque::new(),
            vacated: Vec::new(),
            moved: false,
        };
        drop.respawn(cfg, rng);
        drop
    }

    fn respawn<R: Rng + ?Sized>(&mut self, cfg: &Config, rng: &mut R) {
        self.head = 0.0;
        self.len = rng.random_range(cfg.trail_len.clone()).max(1);
        self.speed = rng.random_range(cfg.speed.clone());
        self.delay = rng.random_range(0..=cfg.max_spawn_delay);
        self.trail.clear();
        self.trail.push_front(cfg.charset.random(rng));
    }

    pub fn is_falling(&self) -> bool {
        self.delay == 0
    }

    pub fn head_row(&self) -> i32 {
        self.head as i32
    }

    pub fn tail_row(&self) -> i32 {
        self.head_row() - (self.trail.len() as i32 - 1)
    }

    /// Visible cells as `(row, distance_from_head, glyph)`.
    pub fn cells(&self) -> impl Iterator<Item = (i32, usize, char)> + '_ {
        let head = self.head_row();
        self.trail
            .iter()
            .enumerate()
            .map(move |(i, &ch)| (head - i as i32, i, ch))
    }

    fn update<R: Rng + ?Sized>(&mut self, height: u16, cfg: &Config, rng: &mut R) {
        self.vacated.clear();
        self.moved = false;

        if self.delay > 0 {
            self.delay -= 1;
            // Drawn for the first time when it starts falling.
            self.moved = self.delay == 0;
            return;
        }

        let before = self.head_row();
        self.head += self.speed;
        for _ in before..self.head_row() {
            self.moved = true;
            self.trail.push_front(cfg.charset.random(rng));
            if self.trail.len() > self.len as usize {
                self.trail.pop_back();
                self.vacated.push(self.tail_row() - 1);
            }
        }

        if self.tail_row() >= height as i32 {
            self.respawn(cfg, rng);
        }
    }
}

/// All drops on screen.
pub struct Rain {
    drops: Vec<Drop>,
    width: u16,
    height: u16,
    rng: ThreadRng,
}

impl Rain {
    pub fn new(width: u16, height: u16, cfg: &Config) -> Self {
        let mut rain = Self {
            drops: Vec::new(),
            width,
            height,
            rng: rand::rng(),
        };
        rain.resize(width, height, cfg);
        rain
    }

    /// Rebuilds every column for the new terminal size.
    pub fn resize(&mut self, width: u16, height: u16, cfg: &Config) {
        self.width = width;
        self.height = height;
        let step = cfg.column_spacing.max(1) as usize;
        self.drops = (0..width)
            .step_by(step)
            .map(|x| Drop::new(x, cfg, &mut self.rng))
            .collect();
    }

    pub fn update(&mut self, cfg: &Config) {
        for drop in &mut self.drops {
            drop.update(self.height, cfg, &mut self.rng);
        }
    }

    /// Randomly swaps glyphs inside the trails (glitch effect).
    pub fn mutate(&mut self, chance: f32, cfg: &Config) {
        let chance = f64::from(chance.clamp(0.0, 1.0));
        for drop in &mut self.drops {
            for glyph in &mut drop.trail {
                if self.rng.random_bool(chance) {
                    *glyph = cfg.charset.random(&mut self.rng);
                }
            }
        }
    }

    pub fn drops(&self) -> &[Drop] {
        &self.drops
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_drop_per_column_step() {
        let cfg = Config::default();
        let rain = Rain::new(80, 24, &cfg);
        assert_eq!(rain.drops().len(), 40);
        assert!(rain.drops().iter().all(|d| d.x < 80));
    }

    #[test]
    fn resize_rebuilds_columns() {
        let cfg = Config {
            column_spacing: 1,
            ..Config::default()
        };
        let mut rain = Rain::new(80, 24, &cfg);
        rain.resize(10, 5, &cfg);
        assert_eq!(rain.drops().len(), 10);
        assert!(rain.drops().iter().all(|d| d.x < 10));
    }

    #[test]
    fn mutate_keeps_trail_lengths() {
        let cfg = Config::default();
        let mut rain = Rain::new(40, 20, &cfg);
        for _ in 0..50 {
            rain.update(&cfg);
        }
        let lens: Vec<_> = rain.drops().iter().map(|d| d.trail.len()).collect();
        rain.mutate(1.0, &cfg);
        let after: Vec<_> = rain.drops().iter().map(|d| d.trail.len()).collect();
        assert_eq!(lens, after);
    }

    #[test]
    fn trail_never_exceeds_len_and_drop_respawns() {
        let cfg = Config::default();
        let mut rng = rand::rng();
        let mut drop = Drop::new(0, &cfg, &mut rng);
        let height = 10;
        let mut respawns = 0;
        let mut last_head = drop.head_row();
        for _ in 0..10_000 {
            drop.update(height, &cfg, &mut rng);
            assert!(drop.trail.len() <= drop.len as usize);
            assert!(drop.tail_row() < height as i32);
            if drop.head_row() < last_head {
                respawns += 1;
            }
            last_head = drop.head_row();
        }
        assert!(respawns > 0);
    }

    #[test]
    fn vacated_rows_are_just_behind_the_tail() {
        let cfg = Config {
            trail_len: 3..=3,
            speed: 1.0..=1.0,
            max_spawn_delay: 0,
            ..Config::default()
        };
        let mut rng = rand::rng();
        let mut drop = Drop::new(0, &cfg, &mut rng);
        // Head rows 1, 2 fill the trail; row 3 pushes row 0 out.
        drop.update(100, &cfg, &mut rng);
        drop.update(100, &cfg, &mut rng);
        assert!(drop.vacated.is_empty());
        drop.update(100, &cfg, &mut rng);
        assert_eq!(drop.vacated, vec![0]);
        assert_eq!(drop.tail_row(), 1);
    }
}
