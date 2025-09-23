use crate::{
    github::models::{FileChange, FileStatus},
    theme::Theme,
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct Sidebar {
    pub state: ListState,
    pub files: Vec<FileChange>,
}

impl Sidebar {
    pub fn new(files: Vec<FileChange>) -> Self {
        let mut state = ListState::default();
        if !files.is_empty() {
            state.select(Some(0));
        }
        Self { state, files }
    }

    pub fn next(&mut self) {
        if self.files.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.files.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme, is_focused: bool) {
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|file| {
                let (symbol, color) = match file.status {
                    FileStatus::Added => ("+", theme.added()),
                    FileStatus::Modified => ("M", theme.modified()),
                    FileStatus::Deleted => ("-", theme.removed()),
                    FileStatus::Renamed => ("R", theme.info()),
                    FileStatus::Copied => ("C", theme.info()),
                };

                let stats = format!(" +{} -{}", file.additions, file.deletions);
                let content = format!("{} {}{}", symbol, file.filename, stats);

                ListItem::new(content).style(Style::default().fg(color))
            })
            .collect();

        // Use focused border style if this pane is focused
        let border_style = if is_focused {
            Style::default().fg(theme.border_focused())
        } else {
            Style::default().fg(theme.border())
        };

        let files_list = List::new(items)
            .block(
                Block::default()
                    .title(" Files ")
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(
                        Style::default()
                            .bg(theme.sidebar_bg())
                            .fg(theme.sidebar_fg()),
                    ),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.sidebar_selected())
                    .fg(theme.sidebar_fg())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("│ ");

        f.render_stateful_widget(files_list, area, &mut self.state);
    }

    pub fn get_selected_file(&self) -> Option<&FileChange> {
        self.state.selected().and_then(|i| self.files.get(i))
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn update_file(&mut self, index: usize, file: FileChange) {
        if index < self.files.len() {
            self.files[index] = file;
        }
    }

    pub fn update_files(&mut self, files: Vec<FileChange>) {
        self.files = files;
        // Preserve selection if possible
        if let Some(selected) = self.state.selected() {
            if selected >= self.files.len() {
                // Selection is out of bounds, select last file or none
                if !self.files.is_empty() {
                    self.state.select(Some(self.files.len() - 1));
                } else {
                    self.state.select(None);
                }
            }
            // Otherwise keep the current selection
        } else if !self.files.is_empty() {
            // No selection, select first file
            self.state.select(Some(0));
        }
    }
}
