//! Terminal setup/teardown. The guard restores the terminal on drop, and a
//! panic hook does the same so a crash never leaves the shell broken.

use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    style::ResetColor,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        install_panic_hook();
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let mut out = io::stdout();
    let _ = execute!(out, ResetColor, Show, LeaveAlternateScreen);
    let _ = out.flush();
    let _ = terminal::disable_raw_mode();
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}
