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
    // Search state
    pub search_mode: bool,   // true when in search input mode
    pub search_active: bool, // true when search results are shown
    pub search_query: String,
    pub search_matches: Vec<(usize, usize, usize)>, // (line_index, start_col, end_col)
    pub current_match_index: Option<usize>,
    pub search_input_cursor: usize,
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
            search_mode: false,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match_index: None,
            search_input_cursor: 0,
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

    // Search methods
    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match_index = None;
        self.search_input_cursor = 0;
    }

    pub fn exit_search(&mut self) {
        self.search_mode = false;
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match_index = None;
    }

    pub fn update_search_query(&mut self, ch: char) {
        self.search_query.insert(self.search_input_cursor, ch);
        self.search_input_cursor += 1;
    }

    pub fn backspace_search(&mut self) {
        if self.search_input_cursor > 0 {
            self.search_input_cursor -= 1;
            self.search_query.remove(self.search_input_cursor);
        }
    }

    pub fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        self.search_matches.clear();
        self.current_match_index = None;

        // Search through the full file view
        if let Some(ref file) = self.current_file {
            if let Some(ref diff) = file.diff_content {
                let lines = diff.full_file_view.clone();
                self.find_matches(&lines);
            } else if let Some(ref patch) = file.patch {
                // Fallback: search in raw patch
                let lines: Vec<String> = patch.lines().map(|s| s.to_string()).collect();
                self.find_matches_in_strings(&lines);
            }
        }

        // If we found matches, select the first one
        if !self.search_matches.is_empty() {
            self.current_match_index = Some(0);
            self.scroll_to_current_match();
        }

        // Mark search as active when executed
        self.search_active = true;
        self.search_mode = false; // Exit input mode
    }

    fn find_matches(&mut self, lines: &[crate::github::models::DiffLine]) {
        let query_lower = self.search_query.to_lowercase();

        for (line_idx, line) in lines.iter().enumerate() {
            let content_lower = line.content.to_lowercase();
            let mut start_pos = 0;

            while let Some(match_pos) = content_lower[start_pos..].find(&query_lower) {
                let absolute_pos = start_pos + match_pos;
                self.search_matches.push((
                    line_idx,
                    absolute_pos,
                    absolute_pos + self.search_query.len(),
                ));
                start_pos = absolute_pos + 1; // Continue searching after this match
            }
        }
    }

    fn find_matches_in_strings(&mut self, lines: &[String]) {
        let query_lower = self.search_query.to_lowercase();

        for (line_idx, line) in lines.iter().enumerate() {
            // Skip the line number and prefix to get actual content
            let content = if line.len() > 1 {
                &line[1..] // Skip the +/- prefix
            } else {
                line
            };

            let content_lower = content.to_lowercase();
            let mut start_pos = 0;

            while let Some(match_pos) = content_lower[start_pos..].find(&query_lower) {
                let absolute_pos = start_pos + match_pos;
                self.search_matches.push((
                    line_idx,
                    absolute_pos + 1, // Account for the prefix we skipped
                    absolute_pos + 1 + self.search_query.len(),
                ));
                start_pos = absolute_pos + 1;
            }
        }
    }

    pub fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => {
                if idx + 1 >= self.search_matches.len() {
                    0 // Wrap around to beginning
                } else {
                    idx + 1
                }
            }
            None => 0,
        });

        self.scroll_to_current_match();
    }

    pub fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => {
                if idx == 0 {
                    self.search_matches.len() - 1 // Wrap around to end
                } else {
                    idx - 1
                }
            }
            None => self.search_matches.len() - 1,
        });

        self.scroll_to_current_match();
    }

    fn scroll_to_current_match(&mut self) {
        if let Some(idx) = self.current_match_index {
            if let Some(&(line_idx, _, _)) = self.search_matches.get(idx) {
                // Scroll to center the match in the viewport if possible
                let target_line = (line_idx as u16).saturating_sub(self.viewport_height / 2);
                self.scroll_offset = target_line.min(self.max_scroll);
            }
        }
    }

    #[allow(dead_code)] // Kept for potential future use
    pub fn clear_search(&mut self) {
        self.search_mode = false;
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match_index = None;
        self.search_input_cursor = 0;
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme, is_focused: bool) {
        // Adjust for search bar if in search mode or status line if search is active
        let (main_area, bottom_area) = if self.search_mode || self.search_active {
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(3),
                    ratatui::layout::Constraint::Length(3),
                ])
                .split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

        // Update viewport height
        self.viewport_height = main_area.height.saturating_sub(2);
        self.update_max_scroll();

        let content = self.generate_content(theme);
        let visible_height = main_area.height.saturating_sub(2) as usize;

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

        f.render_widget(paragraph, main_area);

        // Render scrollbar
        if self.max_scroll > 0 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .thumb_style(Style::default().fg(theme.scrollbar_thumb()))
                .track_style(Style::default().fg(theme.scrollbar()));

            let mut scrollbar_state =
                ScrollbarState::new(self.max_scroll as usize).position(self.scroll_offset as usize);

            f.render_stateful_widget(scrollbar, main_area, &mut scrollbar_state);
        }

        // Render search bar or search status
        if let Some(bottom_area) = bottom_area {
            if self.search_mode {
                self.render_search_bar(f, bottom_area, theme);
            } else if self.search_active {
                self.render_search_status(f, bottom_area, theme);
            }
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
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Search:",
                Style::default()
                    .fg(theme.subtitle())
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "    /           : Start search",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    n           : Next search result",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    N           : Previous search result",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(Span::styled(
                "    Esc         : Clear search (when searching)",
                Style::default().fg(theme.fg()),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "    q/Esc       : Quit",
                Style::default().fg(theme.fg()),
            )));
        }

        lines
    }

    fn apply_search_highlighting(
        &self,
        text: String,
        line_idx: usize,
        base_style: Style,
        theme: &Theme,
    ) -> Vec<Span<'static>> {
        // Check if we have any matches on this line
        let line_matches: Vec<_> = self
            .search_matches
            .iter()
            .filter(|(idx, _, _)| *idx == line_idx)
            .collect();

        if line_matches.is_empty() {
            return vec![Span::styled(text, base_style)];
        }

        let mut spans = Vec::new();
        let mut last_end = 0;

        // Sort matches by start position
        let mut sorted_matches = line_matches.clone();
        sorted_matches.sort_by_key(|(_, start, _)| *start);

        for &(_, start, end) in sorted_matches.iter() {
            // Add text before the match
            if last_end < *start && last_end < text.len() {
                let before = text[last_end..*start.min(&text.len())].to_string();
                if !before.is_empty() {
                    spans.push(Span::styled(before, base_style));
                }
            }

            // Add the matched text with highlight
            if *start < text.len() && *end <= text.len() && *start < *end {
                let matched = text[*start..*end].to_string();
                let is_current = self
                    .current_match_index
                    .map(|idx| {
                        self.search_matches
                            .get(idx)
                            .map(|(l, s, e)| l == &line_idx && s == start && e == end)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                let match_style = if is_current {
                    base_style
                        .bg(theme.search_current())
                        .add_modifier(Modifier::BOLD)
                } else {
                    base_style.bg(theme.search_match())
                };
                spans.push(Span::styled(matched, match_style));
                last_end = *end;
            }
        }

        // Add any remaining text after the last match
        if last_end < text.len() {
            let remaining = text[last_end..].to_string();
            if !remaining.is_empty() {
                spans.push(Span::styled(remaining, base_style));
            }
        }

        spans
    }

    fn render_search_bar(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let search_text = format!("/{}", self.search_query);
        let match_info = if !self.search_matches.is_empty() {
            if let Some(current) = self.current_match_index {
                format!(" [{}/{}]", current + 1, self.search_matches.len())
            } else {
                format!(" [0/{}]", self.search_matches.len())
            }
        } else if !self.search_query.is_empty() {
            " [No matches]".to_string()
        } else {
            String::new()
        };

        let full_text = format!("{search_text}{match_info}");

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_focused()))
            .title(" Search (Enter to search, Esc to cancel) ");

        let search_paragraph = Paragraph::new(full_text)
            .block(search_block)
            .style(Style::default().fg(theme.fg()));

        f.render_widget(search_paragraph, area);

        // Show cursor
        f.set_cursor_position((
            area.x + 1 + self.search_input_cursor as u16 + 1, // +1 for '/', +1 for border
            area.y + 1,
        ));
    }

    fn render_search_status(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let status_text = if !self.search_matches.is_empty() {
            let current = self.current_match_index.map(|i| i + 1).unwrap_or(0);
            format!(
                " Searching for: \"{}\" - {}/{} matches (n: next, N: previous, Esc: clear)",
                self.search_query,
                current,
                self.search_matches.len()
            )
        } else {
            format!(
                " Searching for: \"{}\" - No matches found (Esc: clear)",
                self.search_query
            )
        };

        let status_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .style(Style::default().bg(theme.bg()).fg(theme.info()));

        let status_paragraph = Paragraph::new(status_text)
            .block(status_block)
            .style(Style::default().fg(theme.info()));

        f.render_widget(status_paragraph, area);
    }

    fn render_full_file_diff(&self, diff: &DiffContent, theme: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Render the full file view with inline diff annotations
        for (line_idx, diff_line) in diff.full_file_view.iter().enumerate() {
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

            // Check if this line has search matches
            let has_search_match = !self.search_matches.is_empty()
                && self
                    .search_matches
                    .iter()
                    .any(|(idx, _, _)| *idx == line_idx);

            // Build the line with syntax highlighting if available
            let formatted_line = if matches!(diff_line.line_type, LineType::Header) {
                // Don't syntax highlight header lines
                let full_line = format!("{line_number_str}{prefix} {}", diff_line.content);
                if has_search_match {
                    // Apply search highlighting to the content part
                    let prefix_text = format!("{line_number_str}{prefix} ");
                    let mut spans = vec![Span::styled(prefix_text, base_style)];
                    spans.extend(self.apply_search_highlighting(
                        diff_line.content.clone(),
                        line_idx,
                        base_style,
                        theme,
                    ));
                    spans
                } else {
                    vec![Span::styled(full_line, base_style)]
                }
            } else if let Some(ref highlighter) = self.syntax_highlighter {
                let mut spans = Vec::new();

                // Add line numbers and prefix with base style
                spans.push(Span::styled(
                    format!("{line_number_str}{prefix} "),
                    base_style,
                ));

                // Apply syntax highlighting to the code content, or search highlighting if active
                if has_search_match {
                    // Apply search highlighting instead of syntax highlighting for simplicity
                    spans.extend(self.apply_search_highlighting(
                        diff_line.content.clone(),
                        line_idx,
                        if let Some(bg) = background_color {
                            base_style.bg(bg)
                        } else {
                            base_style
                        },
                        theme,
                    ));
                } else {
                    // Apply syntax highlighting
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
                }

                spans
            } else {
                // No syntax highlighting available, use the base style
                if has_search_match {
                    let prefix_text = format!("{line_number_str}{prefix} ");
                    let mut spans = vec![Span::styled(prefix_text, base_style)];
                    spans.extend(self.apply_search_highlighting(
                        diff_line.content.clone(),
                        line_idx,
                        if let Some(bg) = background_color {
                            base_style.bg(bg)
                        } else {
                            base_style
                        },
                        theme,
                    ));
                    spans
                } else {
                    vec![Span::styled(
                        format!("{line_number_str}{prefix} {}", diff_line.content),
                        if let Some(bg) = background_color {
                            base_style.bg(bg)
                        } else {
                            base_style
                        },
                    )]
                }
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

    #[test]
    fn test_search_functionality() {
        let mut diff_view = DiffView::new();

        // Create test content with searchable patterns
        let diff_content = DiffContent {
            hunks: vec![],
            full_file_view: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "fn hello_world() {".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "    println!(\"Hello, world!\");".to_string(),
                    old_line_no: None,
                    new_line_no: Some(2),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "    let world = \"Earth\";".to_string(),
                    old_line_no: Some(2),
                    new_line_no: Some(3),
                },
                DiffLine {
                    line_type: LineType::Deletion,
                    content: "    // Old world comment".to_string(),
                    old_line_no: Some(3),
                    new_line_no: None,
                },
            ],
        };

        let file_change = FileChange {
            filename: "test.rs".to_string(),
            status: FileStatus::Modified,
            additions: 1,
            deletions: 1,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Test search initialization
        diff_view.start_search();
        assert!(diff_view.search_mode);
        assert_eq!(diff_view.search_query, "");
        assert!(diff_view.search_matches.is_empty());

        // Test adding characters to search
        diff_view.update_search_query('w');
        diff_view.update_search_query('o');
        diff_view.update_search_query('r');
        diff_view.update_search_query('l');
        diff_view.update_search_query('d');
        assert_eq!(diff_view.search_query, "world");
        assert_eq!(diff_view.search_input_cursor, 5);

        // Test backspace
        diff_view.backspace_search();
        assert_eq!(diff_view.search_query, "worl");
        assert_eq!(diff_view.search_input_cursor, 4);

        // Complete the search query
        diff_view.update_search_query('d');

        // Execute search
        diff_view.execute_search();

        // Check matches found (case-insensitive)
        // Should match "world" in lines 0, 1, 2, and 3
        assert!(!diff_view.search_matches.is_empty());
        assert_eq!(diff_view.search_matches.len(), 4); // "world" appears in all 4 lines

        // Test navigation through matches
        let initial_match = diff_view.current_match_index.unwrap();
        diff_view.next_match();
        assert_ne!(diff_view.current_match_index.unwrap(), initial_match);

        // Test wrap-around
        for _ in 0..10 {
            diff_view.next_match();
        }
        // Should wrap around and still have a valid index
        assert!(diff_view.current_match_index.is_some());

        // Test previous match
        let current = diff_view.current_match_index.unwrap();
        diff_view.prev_match();
        assert_ne!(diff_view.current_match_index.unwrap(), current);

        // Test exit search
        diff_view.exit_search();
        assert!(!diff_view.search_mode);

        // Test clear search - search for "let" which exists in the content
        diff_view.start_search();
        diff_view.update_search_query('l');
        diff_view.update_search_query('e');
        diff_view.update_search_query('t');
        diff_view.execute_search();
        assert!(!diff_view.search_matches.is_empty());

        diff_view.clear_search();
        assert!(!diff_view.search_mode);
        assert_eq!(diff_view.search_query, "");
        assert!(diff_view.search_matches.is_empty());
        assert_eq!(diff_view.current_match_index, None);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut diff_view = DiffView::new();

        let diff_content = DiffContent {
            hunks: vec![],
            full_file_view: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "HELLO World".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "hello world".to_string(),
                    old_line_no: Some(2),
                    new_line_no: Some(2),
                },
                DiffLine {
                    line_type: LineType::Context,
                    content: "HeLLo WoRLd".to_string(),
                    old_line_no: Some(3),
                    new_line_no: Some(3),
                },
            ],
        };

        let file_change = FileChange {
            filename: "test.txt".to_string(),
            status: FileStatus::Modified,
            additions: 0,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Search for "hello" in lowercase
        diff_view.start_search();
        diff_view.update_search_query('h');
        diff_view.update_search_query('e');
        diff_view.update_search_query('l');
        diff_view.update_search_query('l');
        diff_view.update_search_query('o');
        diff_view.execute_search();

        // Should match all three lines (case-insensitive)
        assert_eq!(diff_view.search_matches.len(), 3);
    }

    #[test]
    fn test_search_with_no_matches() {
        let mut diff_view = DiffView::new();

        let diff_content = DiffContent {
            hunks: vec![],
            full_file_view: vec![DiffLine {
                line_type: LineType::Context,
                content: "some content".to_string(),
                old_line_no: Some(1),
                new_line_no: Some(1),
            }],
        };

        let file_change = FileChange {
            filename: "test.txt".to_string(),
            status: FileStatus::Modified,
            additions: 0,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Search for non-existent text
        diff_view.start_search();
        diff_view.search_query = "nonexistent".to_string();
        diff_view.execute_search();

        // No matches should be found
        assert!(diff_view.search_matches.is_empty());
        assert_eq!(diff_view.current_match_index, None);

        // Navigation should do nothing
        diff_view.next_match();
        assert_eq!(diff_view.current_match_index, None);

        diff_view.prev_match();
        assert_eq!(diff_view.current_match_index, None);
    }

    #[test]
    fn test_search_mode_transitions() {
        let mut diff_view = DiffView::new();

        // Create test content
        let diff_content = DiffContent {
            hunks: vec![],
            full_file_view: vec![DiffLine {
                line_type: LineType::Context,
                content: "test content".to_string(),
                old_line_no: Some(1),
                new_line_no: Some(1),
            }],
        };

        let file_change = FileChange {
            filename: "test.txt".to_string(),
            status: FileStatus::Modified,
            additions: 0,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Test initial state
        assert!(!diff_view.search_mode);
        assert!(!diff_view.search_active);

        // Start search
        diff_view.start_search();
        assert!(diff_view.search_mode);
        assert!(!diff_view.search_active);

        // Add search query
        diff_view.update_search_query('t');
        diff_view.update_search_query('e');
        diff_view.update_search_query('s');
        diff_view.update_search_query('t');

        // Execute search (should exit input mode but keep search active)
        diff_view.execute_search();
        assert!(!diff_view.search_mode); // Should exit input mode
        assert!(diff_view.search_active); // Should remain active
        assert!(!diff_view.search_matches.is_empty()); // Should have matches

        // Clear search
        diff_view.exit_search();
        assert!(!diff_view.search_mode);
        assert!(!diff_view.search_active);
        assert!(diff_view.search_query.is_empty());
    }

    #[test]
    fn test_escape_key_search_behavior() {
        let mut diff_view = DiffView::new();

        // Create test content
        let diff_content = DiffContent {
            hunks: vec![],
            full_file_view: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "test content".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "added line".to_string(),
                    old_line_no: None,
                    new_line_no: Some(2),
                },
            ],
        };

        let file_change = FileChange {
            filename: "test.txt".to_string(),
            status: FileStatus::Modified,
            additions: 1,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));

        // Test 1: Start search mode and verify states
        diff_view.start_search();
        assert!(diff_view.search_mode, "search_mode should be true");
        assert!(!diff_view.search_active, "search_active should be false");

        // Test 2: Execute search and verify states
        diff_view.update_search_query('t');
        diff_view.update_search_query('e');
        diff_view.update_search_query('s');
        diff_view.update_search_query('t');
        diff_view.execute_search();
        assert!(
            !diff_view.search_mode,
            "search_mode should be false after execute"
        );
        assert!(
            diff_view.search_active,
            "search_active should be true after execute"
        );
        assert!(!diff_view.search_matches.is_empty(), "should have matches");

        // Test 3: Clear search and verify all states are reset
        diff_view.clear_search();
        assert!(
            !diff_view.search_mode,
            "search_mode should be false after clear"
        );
        assert!(
            !diff_view.search_active,
            "search_active should be false after clear"
        );
        assert!(
            diff_view.search_query.is_empty(),
            "search_query should be empty"
        );
        assert!(
            diff_view.search_matches.is_empty(),
            "search_matches should be empty"
        );
        assert_eq!(
            diff_view.current_match_index, None,
            "current_match_index should be None"
        );

        // Test 4: Start search again and exit_search without executing
        diff_view.start_search();
        diff_view.update_search_query('a');
        assert!(diff_view.search_mode, "search_mode should be true");
        assert!(!diff_view.search_active, "search_active should be false");

        diff_view.exit_search();
        assert!(
            !diff_view.search_mode,
            "search_mode should be false after exit"
        );
        assert!(
            !diff_view.search_active,
            "search_active should be false after exit"
        );
        assert!(
            diff_view.search_query.is_empty(),
            "search_query should be cleared"
        );
    }

    #[test]
    fn test_search_scroll_to_match() {
        let mut diff_view = DiffView::new();
        diff_view.viewport_height = 5; // Small viewport

        // Create content with match at the bottom
        let mut lines = vec![];
        for i in 0..20 {
            lines.push(DiffLine {
                line_type: LineType::Context,
                content: format!("line {i}"),
                old_line_no: Some(i + 1),
                new_line_no: Some(i + 1),
            });
        }
        // Add a unique searchable line at the bottom
        lines.push(DiffLine {
            line_type: LineType::Addition,
            content: "unique_search_term".to_string(),
            old_line_no: None,
            new_line_no: Some(21),
        });

        let diff_content = DiffContent {
            hunks: vec![],
            full_file_view: lines,
        };

        let file_change = FileChange {
            filename: "test.txt".to_string(),
            status: FileStatus::Modified,
            additions: 1,
            deletions: 0,
            patch: None,
            raw_content: None,
            diff_content: Some(diff_content),
        };

        diff_view.set_file(Some(file_change));
        diff_view.update_max_scroll();

        // Initially at top
        diff_view.scroll_offset = 0;

        // Search for the unique term at bottom
        diff_view.start_search();
        diff_view.search_query = "unique_search_term".to_string();
        diff_view.execute_search();

        // Should have found one match
        assert_eq!(diff_view.search_matches.len(), 1);
        assert_eq!(diff_view.current_match_index, Some(0));

        // Should have scrolled to show the match
        // The match is at line 20, viewport is 5, so should center around line 20
        // Expected offset would be around 18 (20 - viewport_height/2)
        assert!(diff_view.scroll_offset > 15);
    }
}
