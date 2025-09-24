use crate::{
    github::models::{DiffContent, FileChange, LineType},
    syntax_highlight::{syntect_style_to_ratatui_style, SyntaxHighlighter},
    theme::Theme,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::HashMap;

pub struct DiffView {
    pub scroll_offset: u16,
    pub max_scroll: u16,
    pub current_file: Option<FileChange>,
    pub viewport_height: u16,
    pub total_lines: usize,
    hunk_positions: Vec<usize>,
    syntax_highlighter: Option<SyntaxHighlighter>,
    theme_name: Option<String>,
    /// Cache of syntax highlighters per file extension
    highlighter_cache: HashMap<String, SyntaxHighlighter>,
    /// Cache of hunk positions per file
    hunk_cache: HashMap<String, Vec<usize>>,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            max_scroll: 0,
            current_file: None,
            viewport_height: 20,
            total_lines: 0,
            hunk_positions: Vec::new(),
            syntax_highlighter: None,
            theme_name: None,
            highlighter_cache: HashMap::new(),
            hunk_cache: HashMap::new(),
        }
    }

    pub fn set_file(&mut self, file: Option<FileChange>) {
        if let Some(ref f) = file {
            // Get file extension for caching
            let ext = f.filename.rsplit('.').next().unwrap_or("").to_string();

            // Check cache first, create new highlighter only if needed
            if !self.highlighter_cache.contains_key(&ext) && !ext.is_empty() {
                let highlighter = if let Some(ref theme_name) = self.theme_name {
                    SyntaxHighlighter::with_theme(&f.filename, theme_name)
                } else {
                    SyntaxHighlighter::new(&f.filename)
                };
                self.highlighter_cache.insert(ext.clone(), highlighter);
            }

            // Use cached highlighter
            self.syntax_highlighter = self.highlighter_cache.get(&ext).cloned();

            // Check if we have cached hunk positions for this file
            let file_key = f.filename.clone();
            if !self.hunk_cache.contains_key(&file_key) {
                // Calculate and cache hunk positions
                self.current_file = file.clone();
                self.update_hunk_positions();
                self.hunk_cache
                    .insert(file_key.clone(), self.hunk_positions.clone());
            } else {
                // Use cached hunk positions
                self.hunk_positions = self.hunk_cache.get(&file_key).cloned().unwrap_or_default();
            }
        } else {
            self.syntax_highlighter = None;
            self.hunk_positions.clear();
        }

        self.current_file = file;
        self.scroll_offset = 0;
        self.update_max_scroll();
        self.scroll_to_first_change();
    }

    pub fn set_theme(&mut self, theme_name: &str) {
        self.theme_name = Some(theme_name.to_string());
        // Clear highlighter cache to force recreation with new theme
        self.highlighter_cache.clear();
        // Recreate syntax highlighter with new theme if a file is loaded
        if let Some(ref file) = self.current_file {
            let ext = file.filename.rsplit('.').next().unwrap_or("").to_string();
            if !ext.is_empty() {
                let highlighter = SyntaxHighlighter::with_theme(&file.filename, theme_name);
                self.highlighter_cache
                    .insert(ext.clone(), highlighter.clone());
                self.syntax_highlighter = Some(highlighter);
            }
        }
    }

    fn update_max_scroll(&mut self) {
        if let Some(ref file) = self.current_file {
            if let Some(ref diff) = file.diff_content {
                // Use the full file view line count
                self.total_lines = diff.full_file_view.len();
            } else if let Some(ref patch) = file.patch {
                self.total_lines = patch.lines().count();
            } else {
                self.total_lines = 0;
            }
            self.max_scroll = self
                .total_lines
                .saturating_sub(self.viewport_height as usize) as u16;
        } else {
            self.total_lines = 0;
            self.max_scroll = 0;
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self
            .scroll_offset
            .saturating_add(amount)
            .min(self.max_scroll);
    }

    pub fn page_up(&mut self) {
        let page_size = self.viewport_height.saturating_sub(2);
        self.scroll_up(page_size);
    }

    pub fn page_down(&mut self) {
        let page_size = self.viewport_height.saturating_sub(2);
        self.scroll_down(page_size);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll;
    }

    /// Jump to the next hunk in the diff
    pub fn next_hunk(&mut self) {
        if self.hunk_positions.is_empty() {
            return;
        }

        let current_line = self.scroll_offset as usize;

        // Find the next hunk position after the current scroll position
        // We need to check if the next hunk's adjusted position (with context) is different from current
        for &hunk_pos in &self.hunk_positions {
            let target_line = hunk_pos.saturating_sub(2);
            if target_line > current_line {
                // Jump to this hunk, but don't exceed max_scroll if content fits in viewport
                self.scroll_offset = if self.total_lines > self.viewport_height as usize {
                    (target_line as u16).min(self.max_scroll)
                } else {
                    target_line as u16
                };
                return;
            }
        }

        // If we're at or past the last hunk, wrap around to the first
        if !self.hunk_positions.is_empty() {
            let first_hunk = self.hunk_positions[0];
            let target_line = first_hunk.saturating_sub(2);
            self.scroll_offset = if self.total_lines > self.viewport_height as usize {
                (target_line as u16).min(self.max_scroll)
            } else {
                target_line as u16
            };
        }
    }

    /// Jump to the previous hunk in the diff
    pub fn prev_hunk(&mut self) {
        if self.hunk_positions.is_empty() {
            return;
        }

        let current_line = self.scroll_offset as usize;

        // Find the previous hunk position before the current scroll position
        // We check against the adjusted target position to find the previous visible hunk
        for &hunk_pos in self.hunk_positions.iter().rev() {
            let target_line = hunk_pos.saturating_sub(2);
            if target_line < current_line {
                // Jump to this hunk, but don't exceed max_scroll if content fits in viewport
                self.scroll_offset = if self.total_lines > self.viewport_height as usize {
                    (target_line as u16).min(self.max_scroll)
                } else {
                    target_line as u16
                };
                return;
            }
        }

        // If we're at or before the first hunk, optionally wrap around to the last
        if !self.hunk_positions.is_empty() {
            let last_hunk = *self.hunk_positions.last().unwrap();
            let target_line = last_hunk.saturating_sub(2);
            self.scroll_offset = if self.total_lines > self.viewport_height as usize {
                (target_line as u16).min(self.max_scroll)
            } else {
                target_line as u16
            };
        }
    }

    /// Update the positions of hunks (where changes start)
    fn update_hunk_positions(&mut self) {
        self.hunk_positions.clear();

        if let Some(ref file) = self.current_file {
            if let Some(ref diff) = file.diff_content {
                let mut in_hunk = false;

                for (index, line) in diff.full_file_view.iter().enumerate() {
                    match line.line_type {
                        LineType::Addition | LineType::Deletion => {
                            if !in_hunk {
                                // This is the start of a new hunk
                                self.hunk_positions.push(index);
                                in_hunk = true;
                            }
                        }
                        LineType::Context | LineType::Header => {
                            // We've left a hunk
                            in_hunk = false;
                        }
                    }
                }
            } else if let Some(ref patch) = file.patch {
                // Fallback for patch format: find lines starting with + or - that are not consecutive
                let mut in_hunk = false;

                for (index, line) in patch.lines().enumerate() {
                    let is_change = line.starts_with('+') || line.starts_with('-');

                    if is_change && !in_hunk {
                        // Start of a new hunk
                        self.hunk_positions.push(index);
                        in_hunk = true;
                    } else if !is_change && in_hunk {
                        // End of hunk
                        in_hunk = false;
                    }
                }
            }
        }
    }

    fn scroll_to_first_change(&mut self) {
        // Auto-scroll to the first change (addition or deletion) in the diff
        if let Some(ref file) = self.current_file {
            if let Some(ref diff) = file.diff_content {
                // Find the index of the first change line (not context or header)
                for (index, line) in diff.full_file_view.iter().enumerate() {
                    if matches!(line.line_type, LineType::Addition | LineType::Deletion) {
                        // Scroll to this line, with a small offset to show some context
                        let target_line = index.saturating_sub(2); // Show 2 lines of context before if possible
                        self.scroll_offset = (target_line as u16).min(self.max_scroll);
                        break;
                    }
                }
            } else if let Some(ref patch) = file.patch {
                // Fallback for patch format: find first + or - line
                for (index, line) in patch.lines().enumerate() {
                    if line.starts_with('+') || line.starts_with('-') {
                        let target_line = index.saturating_sub(2);
                        self.scroll_offset = (target_line as u16).min(self.max_scroll);
                        break;
                    }
                }
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme, is_focused: bool) {
        // Update viewport height
        self.viewport_height = area.height.saturating_sub(2);
        self.update_max_scroll();

        let content = self.generate_content(theme);
        let visible_height = area.height.saturating_sub(2) as usize;

        // Get visible lines
        let lines: Vec<Line> = content
            .into_iter()
            .skip(self.scroll_offset as usize)
            .take(visible_height)
            .collect();

        // Build title with scroll position indicator
        let title = if let Some(ref file) = self.current_file {
            let scroll_info = if self.total_lines > 0 {
                let current_line = self.scroll_offset as usize + 1;
                let end_line = (self.scroll_offset as usize + visible_height).min(self.total_lines);
                format!(
                    " {} [L{}-{}/{}] ",
                    file.filename, current_line, end_line, self.total_lines
                )
            } else {
                format!(" {} ", file.filename)
            };
            scroll_info
        } else {
            " Select a file to view diff ".to_string()
        };

        // Use focused border style if this pane is focused
        let border_style = if is_focused {
            Style::default().fg(theme.border_focused())
        } else {
            Style::default().fg(theme.border())
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().bg(theme.bg()).fg(theme.fg())),
            )
            .style(Style::default().fg(theme.fg()));

        f.render_widget(paragraph, area);

        // Render scrollbar
        if self.max_scroll > 0 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .thumb_style(Style::default().fg(theme.scrollbar_thumb()))
                .track_style(Style::default().fg(theme.scrollbar()));

            let mut scrollbar_state =
                ScrollbarState::new(self.max_scroll as usize).position(self.scroll_offset as usize);

            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn generate_content(&self, theme: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if let Some(ref file) = self.current_file {
            if let Some(ref diff) = file.diff_content {
                // Show full file with changes highlighted
                lines.extend(self.render_full_file_diff(diff, theme));
            } else if let Some(ref patch) = file.patch {
                // Fallback to raw patch with syntax highlighting
                for line in patch.lines() {
                    let formatted_line = if line.starts_with("@@") {
                        // Header line - no syntax highlighting
                        vec![Span::styled(
                            line.to_string(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )]
                    } else if line.starts_with('+') || line.starts_with('-') {
                        // Addition or deletion with syntax highlighting
                        let (base_style, background_color) = if line.starts_with('+') {
                            (
                                Style::default().fg(theme.added()),
                                Some(Color::Rgb(0, 40, 0)),
                            )
                        } else {
                            (
                                Style::default().fg(theme.removed()),
                                Some(Color::Rgb(40, 0, 0)),
                            )
                        };

                        if let Some(ref highlighter) = self.syntax_highlighter {
                            let mut spans = Vec::new();
                            // Add the +/- prefix
                            spans.push(Span::styled(line[0..1].to_string(), base_style));

                            // Syntax highlight the rest if there's content after the prefix
                            if line.len() > 1 {
                                let code_content = &line[1..];
                                let highlighted_spans = highlighter.highlight_line(code_content);

                                for (syntax_style, text) in highlighted_spans {
                                    let mut span_style =
                                        syntect_style_to_ratatui_style(&syntax_style);
                                    if let Some(bg) = background_color {
                                        span_style = span_style.bg(bg);
                                    }
                                    spans.push(Span::styled(text, span_style));
                                }
                            }
                            spans
                        } else {
                            // No syntax highlighting
                            vec![Span::styled(
                                line.to_string(),
                                if let Some(bg) = background_color {
                                    base_style.bg(bg)
                                } else {
                                    base_style
                                },
                            )]
                        }
                    } else {
                        // Context line with syntax highlighting
                        if let Some(ref highlighter) = self.syntax_highlighter {
                            let highlighted_spans = highlighter.highlight_line(line);
                            highlighted_spans
                                .into_iter()
                                .map(|(syntax_style, text)| {
                                    Span::styled(
                                        text,
                                        syntect_style_to_ratatui_style(&syntax_style),
                                    )
                                })
                                .collect()
                        } else {
                            vec![Span::styled(
                                line.to_string(),
                                Style::default().fg(theme.context()),
                            )]
                        }
                    };
                    lines.push(Line::from(formatted_line));
                }
            } else {
                lines.push(Line::from("No diff available for this file"));
            }
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Select a file from the sidebar to view its changes",
                Style::default().fg(theme.info()),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Keyboard shortcuts:",
                Style::default()
                    .fg(theme.subtitle())
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "    ↑/↓ or j/k  : Navigate files",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    n/p         : Next/Previous commit",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    Space/PgDn  : Scroll down",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    b/PgUp      : Scroll up",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    ]           : Jump to next hunk",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    [           : Jump to previous hunk",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    q/Esc       : Quit",
                Style::default().fg(theme.fg()),
            )));
        }

        lines
    }

    fn render_full_file_diff(&self, diff: &DiffContent, theme: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Render the full file view with inline diff annotations
        for diff_line in &diff.full_file_view {
            // Format line numbers - show both old and new line numbers for context lines,
            // only the relevant one for additions/deletions
            let line_number_str = match diff_line.line_type {
                LineType::Context => {
                    // Show both line numbers for unchanged lines
                    if let (Some(old), Some(new)) = (diff_line.old_line_no, diff_line.new_line_no) {
                        format!("{old:5} {new:5} ")
                    } else {
                        format!("{:11} ", " ")
                    }
                }
                LineType::Deletion => {
                    // Show only old line number for deletions
                    if let Some(old) = diff_line.old_line_no {
                        format!("{:5} {:5} ", old, "-")
                    } else {
                        format!("{:11} ", " ")
                    }
                }
                LineType::Addition => {
                    // Show only new line number for additions
                    if let Some(new) = diff_line.new_line_no {
                        format!("{:5} {:5} ", "+", new)
                    } else {
                        format!("{:11} ", " ")
                    }
                }
                LineType::Header => {
                    format!("{:11} ", " ")
                }
            };

            // Determine the prefix character and base style based on the line type
            let (prefix, base_style, background_color) = match diff_line.line_type {
                LineType::Addition => (
                    "+",
                    Style::default().fg(theme.added()),
                    Some(Color::Rgb(0, 40, 0)), // Subtle green background
                ),
                LineType::Deletion => (
                    "-",
                    Style::default().fg(theme.removed()),
                    Some(Color::Rgb(40, 0, 0)), // Subtle red background
                ),
                LineType::Context => (" ", Style::default().fg(theme.context()), None),
                LineType::Header => (
                    "@",
                    Style::default()
                        .fg(theme.header())
                        .add_modifier(Modifier::BOLD),
                    None,
                ),
            };

            // Build the line with syntax highlighting if available
            let formatted_line = if matches!(diff_line.line_type, LineType::Header) {
                // Don't syntax highlight header lines
                vec![Span::styled(
                    format!("{line_number_str}{prefix} {}", diff_line.content),
                    base_style,
                )]
            } else if let Some(ref highlighter) = self.syntax_highlighter {
                let mut spans = Vec::new();

                // Add line numbers and prefix with base style
                spans.push(Span::styled(
                    format!("{line_number_str}{prefix} "),
                    base_style,
                ));

                // Apply syntax highlighting to the code content
                let highlighted_spans = highlighter.highlight_line(&diff_line.content);

                for (syntax_style, text) in highlighted_spans {
                    // Convert syntect style to ratatui style
                    let mut span_style = syntect_style_to_ratatui_style(&syntax_style);

                    // Apply the diff background color if present
                    if let Some(bg) = background_color {
                        span_style = span_style.bg(bg);
                    }

                    // For additions and deletions, blend the syntax highlighting with diff colors
                    if matches!(diff_line.line_type, LineType::Addition | LineType::Deletion) {
                        // Keep the syntax highlighting foreground but make it slightly brighter/dimmer
                        // based on whether it's an addition or deletion
                        if diff_line.line_type == LineType::Addition {
                            // Make additions slightly brighter
                            if let Color::Rgb(r, g, b) = span_style.fg.unwrap_or(theme.fg()) {
                                span_style = span_style.fg(Color::Rgb(
                                    (r as u16 + 20).min(255) as u8,
                                    (g as u16 + 30).min(255) as u8,
                                    (b as u16 + 20).min(255) as u8,
                                ));
                            }
                        }
                    }

                    spans.push(Span::styled(text, span_style));
                }

                spans
            } else {
                // No syntax highlighting available, use the base style
                vec![Span::styled(
                    format!("{line_number_str}{prefix} {}", diff_line.content),
                    if let Some(bg) = background_color {
                        base_style.bg(bg)
                    } else {
                        base_style
                    },
                )]
            };

            lines.push(Line::from(formatted_line));
        }

        if lines.is_empty() {
            lines.push(Line::from("No changes in this file"));
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::models::{DiffContent, DiffLine, FileStatus, LineType};

    #[test]
    fn test_hunk_navigation_with_multiple_hunks() {
        let mut diff_view = DiffView::new();

        // Create a test file with multiple hunks
        let diff_content = DiffContent {
            hunks: vec![], // Not used in our navigation logic
            full_file_view: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 1".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 2".to_string(),
                    old_line_no: Some(2),
                    new_line_no: Some(2),
                },
                // First hunk starts at index 2
                DiffLine {
                    line_type: LineType::Addition,
                    content: "added line 1".to_string(),
                    old_line_no: None,
                    new_line_no: Some(3),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "added line 2".to_string(),
                    old_line_no: None,
                    new_line_no: Some(4),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 3".to_string(),
                    old_line_no: Some(3),
                    new_line_no: Some(5),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 4".to_string(),
                    old_line_no: Some(4),
                    new_line_no: Some(6),
                },
                // Second hunk starts at index 6
                DiffLine {
                    line_type: LineType::Deletion,
                    content: "deleted line".to_string(),
                    old_line_no: Some(5),
                    new_line_no: None,
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 5".to_string(),
                    old_line_no: Some(6),
                    new_line_no: Some(7),
                },
                // Third hunk starts at index 8
                DiffLine {
                    line_type: LineType::Addition,
                    content: "final addition".to_string(),
                    old_line_no: None,
                    new_line_no: Some(8),
                },
            ],
        };

        let file_change = FileChange {
            filename: "test.rs".to_string(),
            status: FileStatus::Modified,
            additions: 3,
            deletions: 1,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Check that hunks were correctly identified
        assert_eq!(diff_view.hunk_positions, vec![2, 6, 8]);

        // Test navigation from the beginning
        diff_view.scroll_offset = 0;
        diff_view.next_hunk();
        assert_eq!(diff_view.scroll_offset, 4); // From position 0 (viewing first hunk), jump to second hunk at position 4

        // Navigate to third hunk
        diff_view.next_hunk();
        assert_eq!(diff_view.scroll_offset, 6); // Third hunk at 8, with 2 context lines = 6

        // Wrap around to first hunk
        diff_view.next_hunk();
        assert_eq!(diff_view.scroll_offset, 0);

        // Test prev_hunk
        diff_view.scroll_offset = 7;
        diff_view.prev_hunk();
        assert_eq!(diff_view.scroll_offset, 6); // Should jump to third hunk (at position 8, target 6)

        diff_view.prev_hunk();
        assert_eq!(diff_view.scroll_offset, 4); // Should jump to second hunk

        diff_view.prev_hunk();
        assert_eq!(diff_view.scroll_offset, 0); // Should jump to first hunk

        // Wrap around to last hunk
        diff_view.prev_hunk();
        assert_eq!(diff_view.scroll_offset, 6); // Should wrap to third hunk
    }

    #[test]
    fn test_hunk_navigation_with_no_hunks() {
        let mut diff_view = DiffView::new();

        // Create a file with no changes
        let diff_content = DiffContent {
            hunks: vec![], // Not used in our navigation logic
            full_file_view: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 1".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 2".to_string(),
                    old_line_no: Some(2),
                    new_line_no: Some(2),
                },
            ],
        };

        let file_change = FileChange {
            filename: "test.rs".to_string(),
            status: FileStatus::Modified,
            additions: 0,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Check that no hunks were identified
        assert_eq!(diff_view.hunk_positions, Vec::<usize>::new());

        // Navigation should not change position
        let initial_offset = diff_view.scroll_offset;
        diff_view.next_hunk();
        assert_eq!(diff_view.scroll_offset, initial_offset);

        diff_view.prev_hunk();
        assert_eq!(diff_view.scroll_offset, initial_offset);
    }

    #[test]
    fn test_hunk_navigation_single_hunk() {
        let mut diff_view = DiffView::new();

        // Create a file with a single hunk
        let diff_content = DiffContent {
            hunks: vec![], // Not used in our navigation logic
            full_file_view: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 1".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "added line".to_string(),
                    old_line_no: None,
                    new_line_no: Some(2),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "line 2".to_string(),
                    old_line_no: Some(2),
                    new_line_no: Some(3),
                },
            ],
        };

        let file_change = FileChange {
            filename: "test.rs".to_string(),
            status: FileStatus::Modified,
            additions: 1,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Check that one hunk was identified
        assert_eq!(diff_view.hunk_positions, vec![1]);

        // Both next and prev should jump to the same hunk
        diff_view.scroll_offset = 5;
        diff_view.next_hunk();
        assert_eq!(diff_view.scroll_offset, 0); // Jump to the single hunk

        diff_view.scroll_offset = 5;
        diff_view.prev_hunk();
        assert_eq!(diff_view.scroll_offset, 0); // Jump to the single hunk
    }
}
