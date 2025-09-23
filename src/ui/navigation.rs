use crate::{app::FocusedPane, github::models::Commit, keybindings::KeyBindings, theme::Theme};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Navigation {
    pub current_commit: usize,
    pub total_commits: usize,
    pub commits: Vec<Commit>,
}

impl Navigation {
    pub fn new(commits: Vec<Commit>) -> Self {
        let total = commits.len();
        Self {
            current_commit: if total > 0 { 1 } else { 0 },
            total_commits: total,
            commits,
        }
    }

    pub fn next_commit(&mut self) -> bool {
        if self.current_commit < self.total_commits {
            self.current_commit += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_commit(&mut self) -> bool {
        if self.current_commit > 1 {
            self.current_commit -= 1;
            true
        } else {
            false
        }
    }

    pub fn get_current_commit(&self) -> Option<&Commit> {
        if self.current_commit > 0 && self.current_commit <= self.commits.len() {
            self.commits.get(self.current_commit - 1)
        } else {
            None
        }
    }

    pub fn get_current_index(&self) -> usize {
        if self.current_commit > 0 {
            self.current_commit - 1
        } else {
            0
        }
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        focused_pane: FocusedPane,
        keybindings: &KeyBindings,
    ) {
        let commit_info = if let Some(commit) = self.get_current_commit() {
            let short_sha = &commit.sha[..7];
            let message = commit
                .commit
                .message
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(50)
                .collect::<String>();
            format!(
                " [{}/{}] {} - {} ",
                self.current_commit, self.total_commits, short_sha, message
            )
        } else if self.total_commits == 0 {
            " No commits in this PR ".to_string()
        } else {
            format!(" Commit {}/{} ", self.current_commit, self.total_commits)
        };

        // Add focus indicator
        let focus_indicator = match focused_pane {
            FocusedPane::Sidebar => " [Focus: Sidebar] ",
            FocusedPane::DiffView => " [Focus: Diff] ",
        };

        // Get display keys from keybindings
        let display_keys = keybindings.get_display_keys();

        // Format navigation keys for display
        let nav_up_down = format!(
            "{}/{}",
            display_keys.navigate_up, display_keys.navigate_down
        );
        let top_bottom = format!("{}/{}", display_keys.go_to_top, display_keys.go_to_bottom);
        let hunks = format!("{}/{}", display_keys.prev_hunk, display_keys.next_hunk);

        let nav_controls = vec![Line::from(vec![
            Span::raw(" "),
            Span::styled(
                &display_keys.toggle_focus,
                Style::default()
                    .fg(theme.nav_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Toggle Focus  ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                &display_keys.prev_commit,
                Style::default()
                    .fg(theme.nav_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Prev  ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                &display_keys.next_commit,
                Style::default()
                    .fg(theme.nav_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Next  ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                nav_up_down,
                Style::default()
                    .fg(theme.nav_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Nav  ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                top_bottom,
                Style::default()
                    .fg(theme.nav_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Top/Bottom  ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                hunks,
                Style::default()
                    .fg(theme.nav_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Hunks  ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                &display_keys.quit,
                Style::default()
                    .fg(theme.error())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quit ", Style::default().fg(theme.nav_fg())),
            Span::styled(
                focus_indicator,
                Style::default()
                    .fg(theme.info())
                    .add_modifier(Modifier::BOLD),
            ),
        ])];

        let paragraph = Paragraph::new(nav_controls)
            .block(
                Block::default()
                    .title(commit_info)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border()))
                    .style(Style::default().bg(theme.nav_bg()).fg(theme.nav_fg())),
            )
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}
