mod app;
mod charset;
mod config;
mod input;
mod rain;
mod render;
mod terminal;

use std::io;

use config::Config;
use terminal::TerminalGuard;

fn main() -> io::Result<()> {
    let cfg = Config::default();
    let _guard = TerminalGuard::new()?;
    app::run(&cfg)
}
