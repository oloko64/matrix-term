mod app;
mod charset;
mod cli;
mod config;
mod glitch;
mod input;
mod rain;
mod render;
mod terminal;

use std::io;

use clap::Parser;

use cli::Cli;
use terminal::TerminalGuard;

fn main() -> io::Result<()> {
    // Parse before touching the terminal so --help / errors print normally.
    let cfg = Cli::parse().into_config();
    let _guard = TerminalGuard::new()?;
    app::run(&cfg)
}
