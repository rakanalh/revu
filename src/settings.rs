use crate::keybindings::KeyBindings;
use crate::theme::Theme;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_show_line_numbers")]
    pub show_line_numbers: bool,
    #[serde(default)]
    pub vim_mode: bool,
    #[serde(default)]
    pub keybindings: KeyBindings,
}

fn default_theme() -> String {
    "catppuccin-mocha".to_string()
}

fn default_show_line_numbers() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            show_line_numbers: default_show_line_numbers(),
            vim_mode: false,
            keybindings: KeyBindings::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // Create default settings if file doesn't exist
            let settings = Self::default();
            settings.save()?;
            return Ok(settings);
        }

        let content = fs::read_to_string(&config_path).context("Failed to read settings file")?;

        let settings: Self = toml::from_str(&content).context("Failed to parse settings file")?;

        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize settings")?;

        fs::write(&config_path, content).context("Failed to write settings file")?;

        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            PathBuf::from(".")
        };

        Ok(config_dir.join("revu").join("config.toml"))
    }

    pub fn get_theme(&self) -> Result<Theme> {
        Theme::load(&self.theme)
    }

    pub fn cycle_theme(&mut self) -> Result<()> {
        let themes = Theme::list_available_themes()?;
        if themes.is_empty() {
            return Ok(());
        }

        let current_index = themes.iter().position(|t| t == &self.theme).unwrap_or(0);
        let next_index = (current_index + 1) % themes.len();
        self.theme = themes[next_index].clone();
        self.save()
    }

    #[allow(dead_code)]
    pub fn set_theme(&mut self, theme: String) -> Result<()> {
        // Validate that the theme exists
        let _ = Theme::load(&theme)?;
        self.theme = theme;
        self.save()
    }
}
