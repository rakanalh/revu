use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // UI colors
    pub background: String,
    pub foreground: String,
    pub border: String,
    pub border_focused: String,
    pub title: String,
    pub subtitle: String,

    // Diff colors
    pub added: String,
    pub removed: String,
    pub modified: String,
    pub context: String,
    pub header: String,

    // Status colors
    pub info: String,
    pub warning: String,
    pub error: String,
    pub success: String,

    // Selection colors
    pub selection_bg: String,
    pub selection_fg: String,
    pub cursor: String,

    // Navigation colors
    pub nav_bg: String,
    pub nav_fg: String,
    pub nav_active: String,

    // Sidebar colors
    pub sidebar_bg: String,
    pub sidebar_fg: String,
    pub sidebar_selected: String,

    // Scrollbar
    pub scrollbar: String,
    pub scrollbar_thumb: String,
}

#[derive(Debug, Clone)]
pub struct Theme {
    #[allow(dead_code)]
    pub name: String,
    colors: ThemeColors,
}

impl Theme {
    pub fn load(theme_name: &str) -> Result<Self> {
        // Ensure default themes exist
        Self::create_default_themes()?;

        // Load from theme directory
        let theme_path = Self::theme_path(theme_name)?;
        if theme_path.exists() {
            return Self::load_from_file(&theme_path, theme_name);
        }

        // If theme not found, return error
        anyhow::bail!("Theme '{theme_name}' not found")
    }

    fn load_from_file(path: &Path, name: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read theme file: {path:?}"))?;

        let colors: ThemeColors = toml::from_str(&content)
            .with_context(|| format!("Failed to parse theme file: {path:?}"))?;

        Ok(Self {
            name: name.to_string(),
            colors,
        })
    }

    pub fn theme_path(theme_name: &str) -> Result<PathBuf> {
        let config_dir = Self::config_dir()?;
        Ok(config_dir.join("themes").join(format!("{theme_name}.toml")))
    }

    pub fn config_dir() -> Result<PathBuf> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            PathBuf::from(".")
        };
        Ok(config_dir.join("revu"))
    }

    pub fn list_available_themes() -> Result<Vec<String>> {
        // Ensure default themes exist
        Self::create_default_themes()?;

        let mut themes = Vec::new();
        let themes_dir = Self::config_dir()?.join("themes");

        if themes_dir.exists() {
            for entry in fs::read_dir(themes_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        themes.push(name.to_string());
                    }
                }
            }
        }

        themes.sort();
        Ok(themes)
    }

    pub fn create_default_themes() -> Result<()> {
        let themes_dir = Self::config_dir()?.join("themes");
        fs::create_dir_all(&themes_dir)?;

        // Embed theme files at compile time
        const CATPPUCCIN_MOCHA: &str = include_str!("../themes/catppuccin-mocha.toml");
        const CATPPUCCIN_LATTE: &str = include_str!("../themes/catppuccin-latte.toml");
        const DRACULA: &str = include_str!("../themes/dracula.toml");
        const TOKYO_NIGHT: &str = include_str!("../themes/tokyo-night.toml");
        const GRUVBOX_DARK: &str = include_str!("../themes/gruvbox-dark.toml");
        const GRUVBOX_LIGHT: &str = include_str!("../themes/gruvbox-light.toml");
        const ONE_DARK: &str = include_str!("../themes/one-dark.toml");
        const SOLARIZED_DARK: &str = include_str!("../themes/solarized-dark.toml");
        const SOLARIZED_LIGHT: &str = include_str!("../themes/solarized-light.toml");
        const NORD: &str = include_str!("../themes/nord.toml");

        // Create all default theme files only if they don't exist
        Self::write_theme_file_if_not_exists(&themes_dir, "catppuccin-mocha", CATPPUCCIN_MOCHA)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "catppuccin-latte", CATPPUCCIN_LATTE)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "dracula", DRACULA)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "tokyo-night", TOKYO_NIGHT)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "gruvbox-dark", GRUVBOX_DARK)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "gruvbox-light", GRUVBOX_LIGHT)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "one-dark", ONE_DARK)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "solarized-dark", SOLARIZED_DARK)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "solarized-light", SOLARIZED_LIGHT)?;
        Self::write_theme_file_if_not_exists(&themes_dir, "nord", NORD)?;

        Ok(())
    }

    fn write_theme_file_if_not_exists(dir: &Path, name: &str, content: &str) -> Result<()> {
        let path = dir.join(format!("{name}.toml"));
        if !path.exists() {
            fs::write(path, content)?;
        }
        Ok(())
    }

    // Color parsing helper
    fn parse_color(&self, color_str: &str) -> Color {
        if color_str.starts_with('#') {
            // Parse hex color
            let hex = color_str.trim_start_matches('#');
            if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                return Color::Rgb(r, g, b);
            }
        } else if color_str.starts_with("rgb(") && color_str.ends_with(')') {
            // Parse rgb(r, g, b) format
            let rgb = color_str.trim_start_matches("rgb(").trim_end_matches(')');
            let parts: Vec<&str> = rgb.split(',').collect();
            if parts.len() == 3 {
                let r = parts[0].trim().parse().unwrap_or(0);
                let g = parts[1].trim().parse().unwrap_or(0);
                let b = parts[2].trim().parse().unwrap_or(0);
                return Color::Rgb(r, g, b);
            }
        } else {
            // Try to parse as named color
            match color_str.to_lowercase().as_str() {
                "black" => return Color::Black,
                "red" => return Color::Red,
                "green" => return Color::Green,
                "yellow" => return Color::Yellow,
                "blue" => return Color::Blue,
                "magenta" => return Color::Magenta,
                "cyan" => return Color::Cyan,
                "gray" | "grey" => return Color::Gray,
                "darkgray" | "darkgrey" => return Color::DarkGray,
                "lightred" => return Color::LightRed,
                "lightgreen" => return Color::LightGreen,
                "lightyellow" => return Color::LightYellow,
                "lightblue" => return Color::LightBlue,
                "lightmagenta" => return Color::LightMagenta,
                "lightcyan" => return Color::LightCyan,
                "white" => return Color::White,
                _ => {}
            }
        }

        // Default to white if parsing fails
        Color::White
    }

    // Getters for colors
    pub fn bg(&self) -> Color {
        self.parse_color(&self.colors.background)
    }

    pub fn fg(&self) -> Color {
        self.parse_color(&self.colors.foreground)
    }

    pub fn border(&self) -> Color {
        self.parse_color(&self.colors.border)
    }

    pub fn border_focused(&self) -> Color {
        self.parse_color(&self.colors.border_focused)
    }

    pub fn added(&self) -> Color {
        self.parse_color(&self.colors.added)
    }

    pub fn removed(&self) -> Color {
        self.parse_color(&self.colors.removed)
    }

    pub fn modified(&self) -> Color {
        self.parse_color(&self.colors.modified)
    }

    pub fn context(&self) -> Color {
        self.parse_color(&self.colors.context)
    }

    pub fn info(&self) -> Color {
        self.parse_color(&self.colors.info)
    }

    pub fn warning(&self) -> Color {
        self.parse_color(&self.colors.warning)
    }

    pub fn error(&self) -> Color {
        self.parse_color(&self.colors.error)
    }

    pub fn success(&self) -> Color {
        self.parse_color(&self.colors.success)
    }

    pub fn nav_bg(&self) -> Color {
        self.parse_color(&self.colors.nav_bg)
    }

    pub fn nav_fg(&self) -> Color {
        self.parse_color(&self.colors.nav_fg)
    }

    pub fn sidebar_bg(&self) -> Color {
        self.parse_color(&self.colors.sidebar_bg)
    }

    pub fn sidebar_fg(&self) -> Color {
        self.parse_color(&self.colors.sidebar_fg)
    }

    pub fn sidebar_selected(&self) -> Color {
        self.parse_color(&self.colors.sidebar_selected)
    }

    pub fn header(&self) -> Color {
        self.parse_color(&self.colors.header)
    }

    pub fn subtitle(&self) -> Color {
        self.parse_color(&self.colors.subtitle)
    }

    pub fn nav_active(&self) -> Color {
        self.parse_color(&self.colors.nav_active)
    }

    pub fn scrollbar(&self) -> Color {
        self.parse_color(&self.colors.scrollbar)
    }

    pub fn scrollbar_thumb(&self) -> Color {
        self.parse_color(&self.colors.scrollbar_thumb)
    }
}
