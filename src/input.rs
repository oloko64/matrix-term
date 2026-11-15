//! Keyboard / terminal events mapped to app actions.

use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Resize(u16, u16),
}

/// Waits up to `timeout` for an event; returns an action if it maps to one.
pub fn poll(timeout: Duration) -> io::Result<Option<Action>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let action = match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::Quit)
            }
            _ => None,
        },
        Event::Resize(w, h) => Some(Action::Resize(w, h)),
        _ => None,
    };
    Ok(action)
}
