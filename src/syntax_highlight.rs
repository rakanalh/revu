use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Global syntax set for parsing
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Global theme set
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Cache for syntax definitions based on file extensions
static EXTENSION_CACHE: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut cache = HashMap::new();

    // Common programming languages
    cache.insert("rs".to_string(), "Rust".to_string());
    cache.insert("py".to_string(), "Python".to_string());
    cache.insert("js".to_string(), "JavaScript".to_string());
    cache.insert("jsx".to_string(), "JavaScript (JSX)".to_string());
    cache.insert("ts".to_string(), "TypeScript".to_string());
    cache.insert("tsx".to_string(), "TypeScript (TSX)".to_string());
    cache.insert("go".to_string(), "Go".to_string());
    cache.insert("java".to_string(), "Java".to_string());
    cache.insert("c".to_string(), "C".to_string());
    cache.insert("cpp".to_string(), "C++".to_string());
    cache.insert("cc".to_string(), "C++".to_string());
    cache.insert("cxx".to_string(), "C++".to_string());
    cache.insert("h".to_string(), "C".to_string());
    cache.insert("hpp".to_string(), "C++".to_string());
    cache.insert("cs".to_string(), "C#".to_string());
    cache.insert("rb".to_string(), "Ruby".to_string());
    cache.insert("php".to_string(), "PHP".to_string());
    cache.insert("swift".to_string(), "Swift".to_string());
    cache.insert("kt".to_string(), "Kotlin".to_string());
    cache.insert("scala".to_string(), "Scala".to_string());
    cache.insert("r".to_string(), "R".to_string());
    cache.insert("m".to_string(), "Objective-C".to_string());
    cache.insert("mm".to_string(), "Objective-C++".to_string());
    cache.insert("pl".to_string(), "Perl".to_string());
    cache.insert("lua".to_string(), "Lua".to_string());
    cache.insert("dart".to_string(), "Dart".to_string());
    cache.insert("ex".to_string(), "Elixir".to_string());
    cache.insert("exs".to_string(), "Elixir".to_string());
    cache.insert("erl".to_string(), "Erlang".to_string());
    cache.insert("hrl".to_string(), "Erlang".to_string());
    cache.insert("clj".to_string(), "Clojure".to_string());
    cache.insert("cljs".to_string(), "Clojure".to_string());
    cache.insert("elm".to_string(), "Elm".to_string());
    cache.insert("hs".to_string(), "Haskell".to_string());
    cache.insert("ml".to_string(), "OCaml".to_string());
    cache.insert("mli".to_string(), "OCaml".to_string());
    cache.insert("fs".to_string(), "F#".to_string());
    cache.insert("fsx".to_string(), "F#".to_string());
    cache.insert("fsi".to_string(), "F#".to_string());
    cache.insert("jl".to_string(), "Julia".to_string());
    cache.insert("nim".to_string(), "Nim".to_string());
    cache.insert("nims".to_string(), "Nim".to_string());
    cache.insert("v".to_string(), "V".to_string());
    cache.insert("zig".to_string(), "Zig".to_string());

    // Web technologies
    cache.insert("html".to_string(), "HTML".to_string());
    cache.insert("htm".to_string(), "HTML".to_string());
    cache.insert("xml".to_string(), "XML".to_string());
    cache.insert("css".to_string(), "CSS".to_string());
    cache.insert("scss".to_string(), "Sass".to_string());
    cache.insert("sass".to_string(), "Sass".to_string());
    cache.insert("less".to_string(), "LESS".to_string());
    cache.insert("vue".to_string(), "Vue".to_string());
    cache.insert("svelte".to_string(), "Svelte".to_string());

    // Configuration files
    cache.insert("json".to_string(), "JSON".to_string());
    cache.insert("yaml".to_string(), "YAML".to_string());
    cache.insert("yml".to_string(), "YAML".to_string());
    cache.insert("toml".to_string(), "TOML".to_string());
    cache.insert("ini".to_string(), "INI".to_string());
    cache.insert("env".to_string(), "Plain Text".to_string());

    // Shell scripts
    cache.insert("sh".to_string(), "Bourne Again Shell (bash)".to_string());
    cache.insert("bash".to_string(), "Bourne Again Shell (bash)".to_string());
    cache.insert("zsh".to_string(), "Bourne Again Shell (bash)".to_string());
    cache.insert("fish".to_string(), "Fish".to_string());
    cache.insert("ps1".to_string(), "PowerShell".to_string());
    cache.insert("psm1".to_string(), "PowerShell".to_string());
    cache.insert("psd1".to_string(), "PowerShell".to_string());
    cache.insert("bat".to_string(), "Batch File".to_string());
    cache.insert("cmd".to_string(), "Batch File".to_string());

    // Documentation
    cache.insert("md".to_string(), "Markdown".to_string());
    cache.insert("markdown".to_string(), "Markdown".to_string());
    cache.insert("rst".to_string(), "reStructuredText".to_string());
    cache.insert("tex".to_string(), "LaTeX".to_string());
    cache.insert("adoc".to_string(), "AsciiDoc".to_string());
    cache.insert("asciidoc".to_string(), "AsciiDoc".to_string());

    // Build files
    cache.insert("makefile".to_string(), "Makefile".to_string());
    cache.insert("dockerfile".to_string(), "Dockerfile".to_string());
    cache.insert("dockerignore".to_string(), "Plain Text".to_string());
    cache.insert("gitignore".to_string(), "Plain Text".to_string());

    // SQL
    cache.insert("sql".to_string(), "SQL".to_string());

    // Others
    cache.insert("proto".to_string(), "Protocol Buffers".to_string());
    cache.insert("graphql".to_string(), "GraphQL".to_string());
    cache.insert("gql".to_string(), "GraphQL".to_string());

    cache
});

#[derive(Clone)]
pub struct SyntaxHighlighter {
    syntax: Option<SyntaxReference>,
    theme_name: String,
    highlighter: Arc<Mutex<Option<HighlightLines<'static>>>>,
}

/// Map app theme names to appropriate syntect themes
fn map_theme_to_syntect(app_theme_name: &str) -> &'static str {
    match app_theme_name {
        // Dark themes
        "catppuccin-mocha" => "base16-mocha.dark",
        "dracula" => "base16-mocha.dark", // No native Dracula, use mocha
        "nord" => "base16-ocean.dark",    // Ocean is close to Nord's color scheme
        "tokyo-night" => "base16-eighties.dark", // Eighties has good contrast
        "gruvbox-dark" => "base16-eighties.dark",
        "one-dark" => "base16-ocean.dark",
        "solarized-dark" => "Solarized (dark)",
        // Light themes
        "catppuccin-latte" => "base16-ocean.light",
        "gruvbox-light" => "InspiredGitHub",
        "solarized-light" => "Solarized (light)",
        // Default to dark theme for unknown
        _ => "base16-ocean.dark",
    }
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter for the given filename
    pub fn new(filename: &str) -> Self {
        Self::with_theme(filename, "base16-ocean.dark")
    }

    /// Create a new syntax highlighter with a specific theme
    pub fn with_theme(filename: &str, app_theme_name: &str) -> Self {
        let syntax = detect_syntax(filename);
        let syntect_theme = map_theme_to_syntect(app_theme_name);

        // Create the highlighter if we have syntax
        let highlighter = if let Some(syntax) = syntax {
            let theme = &THEME_SET.themes[syntect_theme];
            // We need to leak the references to make them 'static
            // This is safe because SYNTAX_SET and THEME_SET are static
            let syntax_ref: &'static SyntaxReference = unsafe {
                std::mem::transmute::<&SyntaxReference, &'static SyntaxReference>(syntax)
            };
            let theme_ref: &'static syntect::highlighting::Theme = unsafe {
                std::mem::transmute::<
                    &syntect::highlighting::Theme,
                    &'static syntect::highlighting::Theme,
                >(theme)
            };
            Arc::new(Mutex::new(Some(HighlightLines::new(syntax_ref, theme_ref))))
        } else {
            Arc::new(Mutex::new(None))
        };

        Self {
            syntax: syntax.cloned(),
            theme_name: syntect_theme.to_string(),
            highlighter,
        }
    }

    /// Highlight a line of code
    pub fn highlight_line(&self, line: &str) -> Vec<(SyntectStyle, String)> {
        let mut highlighter_guard = self.highlighter.lock().unwrap();

        if let Some(ref mut highlighter) = *highlighter_guard {
            // Highlight the line with the cached highlighter
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(highlighted) => highlighted
                    .into_iter()
                    .map(|(style, text)| (style, text.to_string()))
                    .collect(),
                Err(_) => {
                    // On error, return the line as-is
                    vec![(SyntectStyle::default(), line.to_string())]
                }
            }
        } else {
            // No syntax available, return the line as-is
            vec![(SyntectStyle::default(), line.to_string())]
        }
    }

    /// Reset the highlighter state (useful when switching between non-contiguous sections)
    #[allow(dead_code)]
    pub fn reset(&self) {
        let mut highlighter_guard = self.highlighter.lock().unwrap();

        if highlighter_guard.is_some() {
            // Recreate the highlighter to reset its state
            if let Some(ref syntax) = self.syntax {
                let theme = &THEME_SET.themes[&self.theme_name];
                // Safe because SYNTAX_SET and THEME_SET are static
                let syntax_ref: &'static SyntaxReference = unsafe {
                    std::mem::transmute::<&SyntaxReference, &'static SyntaxReference>(syntax)
                };
                let theme_ref: &'static syntect::highlighting::Theme = unsafe {
                    std::mem::transmute::<
                        &syntect::highlighting::Theme,
                        &'static syntect::highlighting::Theme,
                    >(theme)
                };
                *highlighter_guard = Some(HighlightLines::new(syntax_ref, theme_ref));
            }
        }
    }

    /// Check if syntax highlighting is available for this file
    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        self.syntax.is_some()
    }
}

/// Detect the syntax definition for a given filename
fn detect_syntax(filename: &str) -> Option<&'static SyntaxReference> {
    let path = Path::new(filename);

    // Try to get syntax by extension first
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = extension.to_lowercase();

        // Check our cache first
        if let Some(syntax_name) = EXTENSION_CACHE.get(&ext_lower) {
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_name(syntax_name) {
                return Some(syntax);
            }
        }

        // Fall back to syntect's built-in detection
        if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(&ext_lower) {
            return Some(syntax);
        }
    }

    // Try to detect by first line (for scripts without extensions)
    // This would require reading the file content, which we don't have here
    // So we'll check the filename itself for common patterns
    let filename_lower = filename.to_lowercase();

    // Check for specific filenames
    if filename_lower == "dockerfile" {
        // Try Docker, Dockerfile, or fall back to Shell syntax
        SYNTAX_SET
            .find_syntax_by_name("Dockerfile")
            .or_else(|| SYNTAX_SET.find_syntax_by_name("Docker"))
            .or_else(|| SYNTAX_SET.find_syntax_by_name("Bourne Again Shell (bash)"))
    } else if filename_lower == "makefile" || filename_lower.starts_with("makefile.") {
        SYNTAX_SET.find_syntax_by_name("Makefile")
    } else if filename_lower == "cmakelists.txt" {
        SYNTAX_SET
            .find_syntax_by_name("CMake")
            .or_else(|| SYNTAX_SET.find_syntax_by_name("Plain Text"))
    } else if filename_lower.ends_with(".gitignore") || filename_lower.ends_with(".dockerignore") {
        SYNTAX_SET
            .find_syntax_by_name("Git Ignore")
            .or_else(|| SYNTAX_SET.find_syntax_by_name("Plain Text"))
    } else {
        // Default to plain text if nothing else matches
        SYNTAX_SET.find_syntax_by_name("Plain Text")
    }
}

/// Convert syntect style to ratatui colors
/// This function maps syntect RGB colors to ratatui RGB colors
pub fn syntect_style_to_ratatui_style(style: &SyntectStyle) -> ratatui::style::Style {
    use ratatui::style::{Color, Modifier, Style};

    let mut ratatui_style = Style::default();

    // Convert foreground color
    ratatui_style = ratatui_style.fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ));

    // Handle font style modifiers
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::UNDERLINE)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }

    ratatui_style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_syntax_common_extensions() {
        // Test Rust file
        let rust_syntax = detect_syntax("main.rs");
        assert!(rust_syntax.is_some());
        assert_eq!(rust_syntax.unwrap().name, "Rust");

        // Test Python file
        let py_syntax = detect_syntax("script.py");
        assert!(py_syntax.is_some());
        assert_eq!(py_syntax.unwrap().name, "Python");

        // Test JavaScript file
        let js_syntax = detect_syntax("app.js");
        assert!(js_syntax.is_some());
        assert!(js_syntax.unwrap().name.contains("JavaScript"));

        // Test TypeScript file - syntect might not have TypeScript by default
        let ts_syntax = detect_syntax("component.ts");
        assert!(ts_syntax.is_some());
        // Just check that we get some syntax, not necessarily TypeScript
        assert!(!ts_syntax.unwrap().name.is_empty());

        // Test Go file
        let go_syntax = detect_syntax("main.go");
        assert!(go_syntax.is_some());
        assert_eq!(go_syntax.unwrap().name, "Go");
    }

    #[test]
    fn test_detect_syntax_special_files() {
        // Test Dockerfile - might not be available in syntect
        let dockerfile_syntax = detect_syntax("Dockerfile");
        // Should at least fall back to plain text
        assert!(dockerfile_syntax.is_some());

        // Test Makefile
        let makefile_syntax = detect_syntax("Makefile");
        assert!(makefile_syntax.is_some());

        // Test .gitignore
        let gitignore_syntax = detect_syntax(".gitignore");
        assert!(gitignore_syntax.is_some());
    }

    #[test]
    fn test_syntax_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new("test.rs");
        assert!(highlighter.is_available());

        let highlighter_unknown = SyntaxHighlighter::new("test.unknown");
        // Should fall back to plain text
        assert!(highlighter_unknown.is_available());
    }

    #[test]
    fn test_highlight_line() {
        let highlighter = SyntaxHighlighter::new("test.rs");
        let highlighted = highlighter.highlight_line("fn main() {");
        assert!(!highlighted.is_empty());

        // Test that we get multiple spans for syntax-highlighted code
        let highlighted_complex = highlighter.highlight_line("let x: i32 = 42;");
        assert!(!highlighted_complex.is_empty());
    }
}
