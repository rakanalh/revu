use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize,
    Tick,
}

pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn poll(&self, timeout: Duration) -> Result<Option<Event>> {
        if event::poll(timeout)? {
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(Some(Event::Key(key))),
                CrosstermEvent::Mouse(mouse) => Ok(Some(Event::Mouse(mouse))),
                CrosstermEvent::Resize(_, _) => Ok(Some(Event::Resize)),
                _ => Ok(None),
            }
        } else {
            Ok(Some(Event::Tick))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Quit,
    NavigateUp,
    NavigateDown,
    NextCommit,
    PrevCommit,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    Home,
    End,
    ToggleFocus,
    Refresh,
    CycleTheme,
    NextHunk,
    PrevHunk,
}

impl Action {
    /// Get action from key event using the provided key mapping
    pub fn from_key_event(key: KeyEvent, key_mapping: &HashMap<KeyEvent, Action>) -> Option<Self> {
        key_mapping.get(&key).copied()
    }
}
