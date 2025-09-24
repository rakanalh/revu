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

    // Search colors (optional with defaults for backward compatibility)
    #[serde(default = "default_search_match")]
    pub search_match: String,
    #[serde(default = "default_search_current")]
    pub search_current: String,
}

// Default functions for optional fields
fn default_search_match() -> String {
    "#f9e2af".to_string() // Default to yellow/amber for search matches
}

fn default_search_current() -> String {
    "#cba6f7".to_string() // Default to purple/magenta for current search match
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
            fs::write(&path, content)?;
        } else {
            // Check if the existing theme file has search fields
            // If not, update it with the new embedded version
            if let Ok(existing_content) = fs::read_to_string(&path) {
                if !existing_content.contains("search_match")
                    || !existing_content.contains("search_current")
                {
                    // Update the theme file with new fields
                    fs::write(&path, content)?;
                }
            }
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

    pub fn search_match(&self) -> Color {
        self.parse_color(&self.colors.search_match)
    }

    pub fn search_current(&self) -> Color {
        self.parse_color(&self.colors.search_current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backward_compatibility_missing_search_fields() {
        // Create a theme without search fields (simulating old theme file)
        let old_theme_content = "# Test Theme without search fields\nbackground = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\nborder = \"#585b70\"\nborder_focused = \"#89b4fa\"\ntitle = \"#f5e0dc\"\nsubtitle = \"#a6adc8\"\nadded = \"#a6e3a1\"\nremoved = \"#f38ba8\"\nmodified = \"#fab387\"\ncontext = \"#6c7086\"\nheader = \"#89dceb\"\ninfo = \"#89b4fa\"\nwarning = \"#f9e2af\"\nerror = \"#f38ba8\"\nsuccess = \"#a6e3a1\"\nselection_bg = \"#313244\"\nselection_fg = \"#cdd6f4\"\ncursor = \"#f5e0dc\"\nnav_bg = \"#181825\"\nnav_fg = \"#bac2de\"\nnav_active = \"#cba6f7\"\nsidebar_bg = \"#11111b\"\nsidebar_fg = \"#a6adc8\"\nsidebar_selected = \"#45475a\"\nscrollbar = \"#313244\"\nscrollbar_thumb = \"#585b70\"";

        // Parse the old theme content without search fields
        let colors: ThemeColors = toml::from_str(old_theme_content).unwrap();

        // Verify that default values are used for search fields
        assert_eq!(colors.search_match, "#f9e2af");
        assert_eq!(colors.search_current, "#cba6f7");
    }

    #[test]
    fn test_theme_with_search_fields() {
        // Create a theme with search fields (new theme format)
        let new_theme_content = "background = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\nborder = \"#585b70\"\nborder_focused = \"#89b4fa\"\ntitle = \"#f5e0dc\"\nsubtitle = \"#a6adc8\"\nadded = \"#a6e3a1\"\nremoved = \"#f38ba8\"\nmodified = \"#fab387\"\ncontext = \"#6c7086\"\nheader = \"#89dceb\"\ninfo = \"#89b4fa\"\nwarning = \"#f9e2af\"\nerror = \"#f38ba8\"\nsuccess = \"#a6e3a1\"\nselection_bg = \"#313244\"\nselection_fg = \"#cdd6f4\"\ncursor = \"#f5e0dc\"\nnav_bg = \"#181825\"\nnav_fg = \"#bac2de\"\nnav_active = \"#cba6f7\"\nsidebar_bg = \"#11111b\"\nsidebar_fg = \"#a6adc8\"\nsidebar_selected = \"#45475a\"\nscrollbar = \"#313244\"\nscrollbar_thumb = \"#585b70\"\nsearch_match = \"#ff0000\"\nsearch_current = \"#00ff00\"";

        // Parse the theme content with search fields
        let colors: ThemeColors = toml::from_str(new_theme_content).unwrap();

        // Verify that provided values are used
        assert_eq!(colors.search_match, "#ff0000");
        assert_eq!(colors.search_current, "#00ff00");
    }

    #[test]
    fn test_theme_file_update() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let theme_path = temp_dir.path();

        // Create old theme file without search fields
        let old_content = "background = \"#000000\"\nforeground = \"#ffffff\"\nborder = \"#333333\"\nborder_focused = \"#666666\"\ntitle = \"#ffffff\"\nsubtitle = \"#cccccc\"\nadded = \"#00ff00\"\nremoved = \"#ff0000\"\nmodified = \"#ffff00\"\ncontext = \"#888888\"\nheader = \"#00ffff\"\ninfo = \"#0000ff\"\nwarning = \"#ffff00\"\nerror = \"#ff0000\"\nsuccess = \"#00ff00\"\nselection_bg = \"#444444\"\nselection_fg = \"#ffffff\"\ncursor = \"#ffffff\"\nnav_bg = \"#222222\"\nnav_fg = \"#dddddd\"\nnav_active = \"#ff00ff\"\nsidebar_bg = \"#111111\"\nsidebar_fg = \"#cccccc\"\nsidebar_selected = \"#555555\"\nscrollbar = \"#333333\"\nscrollbar_thumb = \"#666666\"";

        // New content with search fields
        let new_content =
            format!("{old_content}\nsearch_match = \"#ffff00\"\nsearch_current = \"#ff00ff\"");

        // Write old theme file
        Theme::write_theme_file_if_not_exists(theme_path, "test-theme", old_content)?;

        // Verify file was created
        let file_path = theme_path.join("test-theme.toml");
        assert!(file_path.exists());

        // Call again with new content (should update the file)
        Theme::write_theme_file_if_not_exists(theme_path, "test-theme", &new_content)?;

        // Read and verify the file was updated
        let content = fs::read_to_string(&file_path)?;
        assert!(content.contains("search_match"));
        assert!(content.contains("search_current"));

        Ok(())
    }

    #[test]
    fn test_parse_color_methods() {
        let theme = Theme {
            name: "test".to_string(),
            colors: ThemeColors {
                background: "#1e1e2e".to_string(),
                foreground: "#cdd6f4".to_string(),
                border: "#585b70".to_string(),
                border_focused: "#89b4fa".to_string(),
                title: "#f5e0dc".to_string(),
                subtitle: "#a6adc8".to_string(),
                added: "#a6e3a1".to_string(),
                removed: "#f38ba8".to_string(),
                modified: "#fab387".to_string(),
                context: "#6c7086".to_string(),
                header: "#89dceb".to_string(),
                info: "#89b4fa".to_string(),
                warning: "#f9e2af".to_string(),
                error: "#f38ba8".to_string(),
                success: "#a6e3a1".to_string(),
                selection_bg: "#313244".to_string(),
                selection_fg: "#cdd6f4".to_string(),
                cursor: "#f5e0dc".to_string(),
                nav_bg: "#181825".to_string(),
                nav_fg: "#bac2de".to_string(),
                nav_active: "#cba6f7".to_string(),
                sidebar_bg: "#11111b".to_string(),
                sidebar_fg: "#a6adc8".to_string(),
                sidebar_selected: "#45475a".to_string(),
                scrollbar: "#313244".to_string(),
                scrollbar_thumb: "#585b70".to_string(),
                search_match: "#f9e2af".to_string(),
                search_current: "#cba6f7".to_string(),
            },
        };

        // Test that search color methods work
        let search_match_color = theme.search_match();
        let search_current_color = theme.search_current();

        // Verify colors are parsed (not testing exact values as parse_color is private)
        match search_match_color {
            Color::Rgb(_, _, _) => {} // Expected
            _ => panic!("search_match should return RGB color"),
        }

        match search_current_color {
            Color::Rgb(_, _, _) => {} // Expected
            _ => panic!("search_current should return RGB color"),
        }
    }
}
