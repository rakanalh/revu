use crate::events::Action;
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    #[serde(default = "default_prev_commit")]
    pub prev_commit: Vec<String>,
    #[serde(default = "default_next_commit")]
    pub next_commit: Vec<String>,
    #[serde(default = "default_navigate_up")]
    pub navigate_up: Vec<String>,
    #[serde(default = "default_navigate_down")]
    pub navigate_down: Vec<String>,
    #[serde(default = "default_scroll_up")]
    pub scroll_up: Vec<String>,
    #[serde(default = "default_scroll_down")]
    pub scroll_down: Vec<String>,
    #[serde(default = "default_page_up")]
    pub page_up: Vec<String>,
    #[serde(default = "default_page_down")]
    pub page_down: Vec<String>,
    #[serde(default = "default_go_to_top")]
    pub go_to_top: Vec<String>,
    #[serde(default = "default_go_to_bottom")]
    pub go_to_bottom: Vec<String>,
    #[serde(default = "default_quit")]
    pub quit: Vec<String>,
    #[serde(default = "default_toggle_focus")]
    pub toggle_focus: Vec<String>,
    #[serde(default = "default_cycle_theme")]
    pub cycle_theme: Vec<String>,
    #[serde(default = "default_refresh")]
    pub refresh: Vec<String>,
    #[serde(default = "default_next_hunk")]
    pub next_hunk: Vec<String>,
    #[serde(default = "default_prev_hunk")]
    pub prev_hunk: Vec<String>,
    #[serde(default = "default_start_search")]
    pub start_search: Vec<String>,
    #[serde(default = "default_next_match")]
    pub next_match: Vec<String>,
    #[serde(default = "default_prev_match")]
    pub prev_match: Vec<String>,
}

// Default key bindings - Vim-style with alternatives
fn default_prev_commit() -> Vec<String> {
    vec!["h".to_string(), "Left".to_string(), "p".to_string()]
}

fn default_next_commit() -> Vec<String> {
    vec!["l".to_string(), "Right".to_string(), "n".to_string()]
}

fn default_navigate_up() -> Vec<String> {
    vec!["k".to_string(), "Up".to_string()]
}

fn default_navigate_down() -> Vec<String> {
    vec!["j".to_string(), "Down".to_string()]
}

fn default_scroll_up() -> Vec<String> {
    vec!["u".to_string()]
}

fn default_scroll_down() -> Vec<String> {
    vec!["d".to_string()]
}

fn default_page_up() -> Vec<String> {
    vec!["b".to_string(), "PageUp".to_string()]
}

fn default_page_down() -> Vec<String> {
    vec!["Space".to_string(), "PageDown".to_string()]
}

fn default_go_to_top() -> Vec<String> {
    vec!["g".to_string(), "Home".to_string()]
}

fn default_go_to_bottom() -> Vec<String> {
    vec!["G".to_string(), "End".to_string()]
}

fn default_quit() -> Vec<String> {
    vec!["q".to_string(), "Esc".to_string(), "Ctrl+c".to_string()]
}

fn default_toggle_focus() -> Vec<String> {
    vec!["Tab".to_string()]
}

fn default_cycle_theme() -> Vec<String> {
    vec!["t".to_string(), "T".to_string()]
}

fn default_refresh() -> Vec<String> {
    vec!["r".to_string(), "F5".to_string()]
}

fn default_next_hunk() -> Vec<String> {
    vec!["]".to_string()]
}

fn default_prev_hunk() -> Vec<String> {
    vec!["[".to_string()]
}

fn default_start_search() -> Vec<String> {
    vec!["/".to_string()]
}

fn default_next_match() -> Vec<String> {
    vec!["n".to_string()]
}

fn default_prev_match() -> Vec<String> {
    vec!["N".to_string(), "Shift+n".to_string()]
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            prev_commit: default_prev_commit(),
            next_commit: default_next_commit(),
            navigate_up: default_navigate_up(),
            navigate_down: default_navigate_down(),
            scroll_up: default_scroll_up(),
            scroll_down: default_scroll_down(),
            page_up: default_page_up(),
            page_down: default_page_down(),
            go_to_top: default_go_to_top(),
            go_to_bottom: default_go_to_bottom(),
            quit: default_quit(),
            toggle_focus: default_toggle_focus(),
            cycle_theme: default_cycle_theme(),
            refresh: default_refresh(),
            next_hunk: default_next_hunk(),
            prev_hunk: default_prev_hunk(),
            start_search: default_start_search(),
            next_match: default_next_match(),
            prev_match: default_prev_match(),
        }
    }
}

impl KeyBindings {
    /// Create a mapping from KeyEvent to Action based on the configured bindings
    pub fn create_mapping(&self) -> Result<HashMap<KeyEvent, Action>> {
        let mut map = HashMap::new();

        // Helper closure to add mappings for a list of key strings
        let mut add_mappings = |keys: &[String], action: Action| -> Result<()> {
            for key_str in keys {
                let key_event = Self::parse_key(key_str)
                    .with_context(|| format!("Invalid key binding: {key_str}"))?;
                map.insert(key_event, action.clone());
            }
            Ok(())
        };

        // Map all configured keys to their actions
        add_mappings(&self.prev_commit, Action::PrevCommit)?;
        add_mappings(&self.next_commit, Action::NextCommit)?;
        add_mappings(&self.navigate_up, Action::NavigateUp)?;
        add_mappings(&self.navigate_down, Action::NavigateDown)?;
        add_mappings(&self.scroll_up, Action::ScrollUp)?;
        add_mappings(&self.scroll_down, Action::ScrollDown)?;
        add_mappings(&self.page_up, Action::PageUp)?;
        add_mappings(&self.page_down, Action::PageDown)?;
        add_mappings(&self.go_to_top, Action::Home)?;
        add_mappings(&self.go_to_bottom, Action::End)?;
        add_mappings(&self.quit, Action::Quit)?;
        add_mappings(&self.toggle_focus, Action::ToggleFocus)?;
        add_mappings(&self.cycle_theme, Action::CycleTheme)?;
        add_mappings(&self.refresh, Action::Refresh)?;
        add_mappings(&self.next_hunk, Action::NextHunk)?;
        add_mappings(&self.prev_hunk, Action::PrevHunk)?;
        add_mappings(&self.start_search, Action::StartSearch)?;
        add_mappings(&self.next_match, Action::NextMatch)?;
        add_mappings(&self.prev_match, Action::PrevMatch)?;

        Ok(map)
    }

    /// Parse a key string into a KeyEvent
    /// Supports formats like:
    /// - Single character: "a", "b", "1"
    /// - Special keys: "Tab", "Enter", "Esc", "Space", "Backspace"
    /// - Arrow keys: "Up", "Down", "Left", "Right"
    /// - Function keys: "F1", "F2", ..., "F12"
    /// - Page navigation: "PageUp", "PageDown", "Home", "End"
    /// - Modified keys: "Ctrl+c", "Alt+x", "Shift+A"
    fn parse_key(key_str: &str) -> Result<KeyEvent> {
        let parts: Vec<&str> = key_str.split('+').collect();

        let mut modifiers = KeyModifiers::empty();
        let key_part = if parts.len() > 1 {
            // Parse modifiers
            for modifier in &parts[..parts.len() - 1] {
                match modifier.to_lowercase().as_str() {
                    "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                    "alt" => modifiers |= KeyModifiers::ALT,
                    "shift" => modifiers |= KeyModifiers::SHIFT,
                    _ => anyhow::bail!("Unknown modifier: {modifier}"),
                }
            }
            parts.last().unwrap()
        } else {
            key_str
        };

        let code = match key_part {
            // Special keys
            "Tab" => KeyCode::Tab,
            "Enter" => KeyCode::Enter,
            "Esc" | "Escape" => KeyCode::Esc,
            "Space" => KeyCode::Char(' '),
            "Backspace" => KeyCode::Backspace,
            "Delete" => KeyCode::Delete,

            // Arrow keys
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,

            // Page navigation
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,

            // Function keys
            key if key.starts_with('F') && key.len() > 1 => {
                let num = key[1..]
                    .parse::<u8>()
                    .with_context(|| format!("Invalid function key: {key}"))?;
                if (1..=12).contains(&num) {
                    KeyCode::F(num)
                } else {
                    anyhow::bail!("Function key out of range: {key}")
                }
            }

            // Single character
            s if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                // If Shift modifier is explicitly specified, use uppercase
                // Otherwise, use the character as-is
                if modifiers.contains(KeyModifiers::SHIFT) && c.is_alphabetic() {
                    KeyCode::Char(c.to_ascii_uppercase())
                } else {
                    KeyCode::Char(c)
                }
            }

            _ => anyhow::bail!("Unknown key: {key_part}"),
        };

        Ok(KeyEvent::new(code, modifiers))
    }

    /// Get the first configured key for each action (for display purposes)
    pub fn get_display_keys(&self) -> KeyDisplays {
        KeyDisplays {
            prev_commit: self.prev_commit.first().cloned().unwrap_or_default(),
            next_commit: self.next_commit.first().cloned().unwrap_or_default(),
            navigate_up: self.navigate_up.first().cloned().unwrap_or_default(),
            navigate_down: self.navigate_down.first().cloned().unwrap_or_default(),
            go_to_top: self.go_to_top.first().cloned().unwrap_or_default(),
            go_to_bottom: self.go_to_bottom.first().cloned().unwrap_or_default(),
            toggle_focus: self.toggle_focus.first().cloned().unwrap_or_default(),
            quit: self.quit.first().cloned().unwrap_or_default(),
            next_hunk: self.next_hunk.first().cloned().unwrap_or_default(),
            prev_hunk: self.prev_hunk.first().cloned().unwrap_or_default(),
        }
    }
}

/// Structure holding display-friendly key strings for the UI
pub struct KeyDisplays {
    pub prev_commit: String,
    pub next_commit: String,
    pub navigate_up: String,
    pub navigate_down: String,
    pub go_to_top: String,
    pub go_to_bottom: String,
    pub toggle_focus: String,
    pub quit: String,
    pub next_hunk: String,
    pub prev_hunk: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_char() {
        let key = KeyBindings::parse_key("a").unwrap();
        assert_eq!(key.code, KeyCode::Char('a'));
        assert_eq!(key.modifiers, KeyModifiers::empty());
    }

    #[test]
    fn test_parse_special_keys() {
        let tab = KeyBindings::parse_key("Tab").unwrap();
        assert_eq!(tab.code, KeyCode::Tab);

        let space = KeyBindings::parse_key("Space").unwrap();
        assert_eq!(space.code, KeyCode::Char(' '));

        let esc = KeyBindings::parse_key("Esc").unwrap();
        assert_eq!(esc.code, KeyCode::Esc);
    }

    #[test]
    fn test_parse_arrow_keys() {
        let left = KeyBindings::parse_key("Left").unwrap();
        assert_eq!(left.code, KeyCode::Left);

        let up = KeyBindings::parse_key("Up").unwrap();
        assert_eq!(up.code, KeyCode::Up);
    }

    #[test]
    fn test_parse_modified_keys() {
        let ctrl_c = KeyBindings::parse_key("Ctrl+c").unwrap();
        assert_eq!(ctrl_c.code, KeyCode::Char('c'));
        assert!(ctrl_c.modifiers.contains(KeyModifiers::CONTROL));

        let alt_x = KeyBindings::parse_key("Alt+x").unwrap();
        assert_eq!(alt_x.code, KeyCode::Char('x'));
        assert!(alt_x.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn test_parse_function_keys() {
        let f1 = KeyBindings::parse_key("F1").unwrap();
        assert_eq!(f1.code, KeyCode::F(1));

        let f12 = KeyBindings::parse_key("F12").unwrap();
        assert_eq!(f12.code, KeyCode::F(12));
    }

    #[test]
    fn test_default_bindings() {
        let bindings = KeyBindings::default();
        let mapping = bindings.create_mapping().unwrap();

        // Test vim-style navigation
        let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
        assert_eq!(mapping.get(&h).cloned(), Some(Action::PrevCommit));

        let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty());
        assert_eq!(mapping.get(&l).cloned(), Some(Action::NextCommit));

        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        assert_eq!(mapping.get(&j).cloned(), Some(Action::NavigateDown));

        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty());
        assert_eq!(mapping.get(&k).cloned(), Some(Action::NavigateUp));
    }
}
