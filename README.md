# Revu - Terminal UI for GitHub PR Reviews

[![CI](https://github.com/rakan/GithubReview/actions/workflows/ci.yml/badge.svg)](https://github.com/rakan/GithubReview/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-%23dea584.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Crates.io](https://img.shields.io/crates/v/revu.svg)](https://crates.io/crates/revu)

A fast, keyboard-driven terminal user interface for reviewing GitHub Pull Requests in the exact steps the PR author took to author the PR. Experience code changes with syntax highlighting, diff viewing, and customizable themes.

![Revu Screenshot](docs/screenshot.png)

## Features

- **Full-featured PR review interface**: Navigate through commits, files, and diffs seamlessly
- **Syntax-highlighted diffs**: View code changes with proper syntax highlighting
- **Vim-style navigation**: Familiar key bindings for efficient navigation
- **Multiple built-in themes**: Choose from Catppuccin, Dracula, Nord, Tokyo Night, and more
- **Custom theme support**: Create and use your own color themes
- **Real-time loading progress**: Visual checklist showing loading status
- **Commit-by-commit review**: Step through individual commits or view all changes
- **Mouse support**: Optional mouse scrolling and interaction
- **Configurable key bindings**: Customize shortcuts to match your workflow

## Milestones

- [ ] Load multiple PRs and switch between them
- [ ] Comment on a line
- [ ] Approve a PR
- [x] Syntax highlighting
- [ ] Search within the diff
- [ ] Resolve conflicts

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo (comes with Rust)
- GitHub personal access token (for private repositories and to avoid rate limits)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/revu
cd revu

# Build the application
cargo build --release

# The binary will be available at ./target/release/revu
```

### Install via Cargo

```bash
cargo install revu
```

### Binary Installation

Download the latest binary from the [releases page](https://github.com/yourusername/revu/releases) and add it to your PATH.

## Usage

### Basic Usage

```bash
# Review a PR using its full URL
revu https://github.com/owner/repo/pull/123

# Review a PR using just the number (requires environment variables)
revu 123
```

### Authentication

Revu supports multiple authentication methods, checked in this order:

#### 1. Command Line (highest priority)
```bash
revu --token YOUR_TOKEN https://github.com/owner/repo/pull/123
```

#### 2. Authinfo File (recommended)

Create a `~/.authinfo` or `~/.netrc` file with the following format:

```
machine api.github.com login YOUR_USERNAME^revu password YOUR_GITHUB_TOKEN
```

For security, ensure the file has proper permissions:
```bash
chmod 600 ~/.authinfo
```

Example:
```
machine api.github.com login johndoe^revu password ghp_1234567890abcdef
```

**Note:** The `^revu` suffix in the login field is required to identify tokens specifically for this application.

#### 3. Environment Variable (fallback)
```bash
export GITHUB_TOKEN="your-personal-access-token"
```

### Environment Variables

```bash
# Optional: Set default repository (for using PR number only)
export GITHUB_OWNER="repository-owner"
export GITHUB_REPO="repository-name"
```

### Command Line Options

```bash
revu [OPTIONS] <PR>

Arguments:
  <PR>  GitHub PR URL or PR number

Options:
  -t, --token <TOKEN>    GitHub personal access token (overrides GITHUB_TOKEN env var)
  -o, --owner <OWNER>    Repository owner (overrides GITHUB_OWNER env var)
  -r, --repo <REPO>      Repository name (overrides GITHUB_REPO env var)
  -h, --help             Print help information
```

### Keyboard Shortcuts

#### Navigation

| Key | Action | Description |
|-----|--------|-------------|
| `Tab` | Toggle focus | Switch between sidebar and diff view |
| `j` / `↓` | Navigate down | Move down in current pane |
| `k` / `↑` | Navigate up | Move up in current pane |
| `n` | Next commit | Go to next commit |
| `p` | Previous commit | Go to previous commit |
| `g` | Go to top | Jump to beginning |
| `G` | Go to bottom | Jump to end |

#### Scrolling (Diff View)

| Key | Action | Description |
|-----|--------|-------------|
| `h` / `←` | Scroll left | Scroll diff view left |
| `l` / `→` | Scroll right | Scroll diff view right |
| `Ctrl+d` | Page down | Scroll down by half page |
| `Ctrl+u` | Page up | Scroll up by half page |
| `Page Down` | Page down | Scroll down by full page |
| `Page Up` | Page up | Scroll up by full page |

#### Other

| Key | Action | Description |
|-----|--------|-------------|
| `r` | Refresh | Reload PR data |
| `t` | Cycle theme | Switch to next theme |
| `q` | Quit | Exit the application |
| `Esc` | Quit | Exit the application |

### Mouse Support

- **Scroll wheel**: Scroll through diff content
- **Click**: Select items in sidebar (coming soon)

## Configuration

### Configuration File Location

The configuration file is located at:
- Linux/Mac: `~/.config/revu/config.toml`
- Windows: `%APPDATA%\revu\config.toml`

### Example Configuration

```toml
# ~/.config/revu/config.toml

# Theme settings
theme = "catppuccin-mocha"  # Default theme to use

# Key bindings (all keys are optional, defaults shown)
[keybindings]
quit = ["q", "Esc"]
toggle_focus = ["Tab"]
navigate_up = ["k", "Up"]
navigate_down = ["j", "Down"]
next_commit = ["n"]
prev_commit = ["p"]
scroll_up = ["h", "Left"]
scroll_down = ["l", "Right"]
page_up = ["PageUp", "Ctrl+u"]
page_down = ["PageDown", "Ctrl+d"]
home = ["g", "Home"]
end = ["G", "End"]
refresh = ["r", "F5"]
cycle_theme = ["t"]
```

### Customizing Key Bindings

You can customize any key binding by modifying the configuration file. Keys can be specified as:
- Single characters: `"a"`, `"b"`, `"1"`
- Special keys: `"Enter"`, `"Tab"`, `"Esc"`, `"Space"`
- Arrow keys: `"Up"`, `"Down"`, `"Left"`, `"Right"`
- Function keys: `"F1"`, `"F2"`, ..., `"F12"`
- Modified keys: `"Ctrl+a"`, `"Alt+x"`, `"Shift+Tab"`
- Page navigation: `"PageUp"`, `"PageDown"`, `"Home"`, `"End"`

Multiple keys can be bound to the same action by providing an array of keys.

## Themes

### Built-in Themes

Revu comes with several built-in themes:

- **catppuccin-mocha**: Soothing pastel theme (dark)
- **catppuccin-latte**: Soothing pastel theme (light)
- **dracula**: Dark theme with vibrant colors
- **nord**: Arctic, north-bluish color palette
- **tokyo-night**: Clean, dark theme inspired by Tokyo night lights
- **gruvbox-dark**: Retro groove color scheme (dark)
- **gruvbox-light**: Retro groove color scheme (light)
- **onedark**: Atom One Dark inspired theme
- **solarized-dark**: Precision colors for machines and people (dark)
- **solarized-light**: Precision colors for machines and people (light)

### Changing Themes

You can change themes in three ways:

1. **Configuration file**: Set `theme = "theme-name"` in `~/.config/revu/config.toml`
2. **Runtime**: Press `t` while running to cycle through available themes
3. **Custom theme**: Create your own theme file (see below)

### Creating Custom Themes

Custom themes are defined in TOML files placed in `~/.config/revu/themes/`.

#### Theme File Location

- Linux/Mac: `~/.config/revu/themes/my-theme.toml`
- Windows: `%APPDATA%\revu\themes\my-theme.toml`

#### Theme File Format

Create a new file with your theme name (e.g., `my-custom-theme.toml`):

```toml
# ~/.config/revu/themes/my-custom-theme.toml

name = "my-custom-theme"
description = "My custom color theme"

[colors]
# Basic colors
background = "#1e1e2e"      # Main background
foreground = "#cdd6f4"      # Main text color

# UI elements
border = "#45475a"          # Normal border color
border_focused = "#89dceb"  # Focused border color
title = "#f38ba8"          # Title text color

# Diff colors
added = "#a6e3a1"          # Added lines (green)
removed = "#f38ba8"        # Removed lines (red)
modified = "#f9e2af"       # Modified lines (yellow)
context = "#6c7086"        # Context lines (gray)

# Status colors
info = "#89b4fa"           # Info messages (blue)
warning = "#f9e2af"        # Warning messages (yellow)
error = "#f38ba8"          # Error messages (red)
success = "#a6e3a1"        # Success messages (green)

# Selection colors
selection_bg = "#45475a"    # Selected item background
selection_fg = "#cdd6f4"    # Selected item text

# Navigation bar
nav_bg = "#181825"         # Navigation bar background
nav_fg = "#bac2de"         # Navigation bar text
nav_active = "#89dceb"     # Active item in nav bar

# Sidebar
sidebar_bg = "#1e1e2e"     # Sidebar background
sidebar_fg = "#cdd6f4"     # Sidebar text
sidebar_selected = "#45475a" # Selected item in sidebar

# Headers and special text
header = "#cba6f7"         # Header text (purple)
subtitle = "#94e2d5"       # Subtitle text (teal)

# Other UI elements
cursor = "#f5e0dc"         # Cursor color
scrollbar = "#45475a"      # Scrollbar track
scrollbar_thumb = "#585b70" # Scrollbar thumb
```

#### Color Format Options

Colors can be specified in several formats:

1. **Hex colors**: `"#ff5733"`, `"#fff"`
2. **RGB colors**: `"rgb(255, 87, 51)"`
3. **Named colors**: `"red"`, `"blue"`, `"green"`, `"yellow"`, `"magenta"`, `"cyan"`, `"white"`, `"black"`, `"gray"`, `"darkgray"`, `"lightred"`, `"lightgreen"`, `"lightyellow"`, `"lightblue"`, `"lightmagenta"`, `"lightcyan"`

#### Using Your Custom Theme

Once you've created your theme file:

1. Save it in `~/.config/revu/themes/my-custom-theme.toml`
2. Either:
   - Set it in config: `theme = "my-custom-theme"`
   - Or cycle to it using the `t` key while running revu

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

```bash
# Clone the repo
git clone https://github.com/yourusername/revu
cd revu

# Build in debug mode
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- https://github.com/owner/repo/pull/123
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- Themes inspired by popular color schemes from the community
- GitHub API integration using [Octocrab](https://github.com/XAMPPRocky/octocrab)

## Troubleshooting

### Common Issues

**Q: I get a "rate limit exceeded" error**
A: You need to provide authentication. The recommended method is to create a `~/.authinfo` file:
```
machine api.github.com login YOUR_USERNAME^revu password YOUR_GITHUB_TOKEN
```
Alternatively, you can use the `--token` flag or set the `GITHUB_TOKEN` environment variable.

**Q: The TUI doesn't display correctly**
A: Ensure your terminal supports Unicode and 256 colors. Try a different terminal emulator if issues persist.

**Q: Custom themes aren't loading**
A: Check that your theme file is valid TOML and located in the correct directory (`~/.config/revu/themes/`).

**Q: Key bindings don't work as expected**
A: Some key combinations might be captured by your terminal emulator. Check your terminal's key binding settings.

For more help, please open an issue on the [GitHub repository](https://github.com/yourusername/revu/issues).