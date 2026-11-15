//! Main loop: update -> render -> handle input until the next frame.

use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::terminal;

use crate::{
    config::Config,
    input::{self, Action},
    rain::Rain,
    render::Renderer,
};

pub fn run(cfg: &Config) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    let mut rain = Rain::new(width, height, cfg);
    let mut renderer = Renderer::new();
    renderer.clear()?;

    let frame = Duration::from_secs_f64(1.0 / cfg.fps.max(1) as f64);

    loop {
        let start = Instant::now();
        rain.update(cfg);
        renderer.draw(&rain, cfg)?;

        // Spend the rest of the frame waiting for input.
        while let Some(remaining) = frame.checked_sub(start.elapsed()) {
            match input::poll(remaining)? {
                Some(Action::Quit) => return Ok(()),
                Some(Action::Resize(w, h)) => {
                    rain.resize(w, h, cfg);
                    renderer.clear()?;
                }
                None => {}
            }
        }
    }
}
