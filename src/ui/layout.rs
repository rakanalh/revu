use crate::{
    app::{LoadingStatus, LoadingStepStatus},
    theme::Theme,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub struct AppLayout;

impl AppLayout {
    pub fn split_main(area: Rect) -> (Rect, Rect, Rect) {
        // Create main vertical layout: main area + navigation bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Main content area
                Constraint::Length(3), // Navigation bar
            ])
            .split(area);

        // Split main content area into sidebar and diff view
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Sidebar (file list)
                Constraint::Percentage(70), // Diff view
            ])
            .split(main_chunks[0]);

        (content_chunks[0], content_chunks[1], main_chunks[1])
    }

    pub fn render_loading_checklist(
        f: &mut Frame,
        area: Rect,
        status: &LoadingStatus,
        theme: &Theme,
    ) {
        let block = Block::default()
            .title(" Loading ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.warning()))
            .style(Style::default().bg(theme.bg()));

        // Make the loading box smaller (40% width, 30% height)
        let loading_area = centered_rect(40, 30, area);
        f.render_widget(block.clone(), loading_area);

        // Create inner layout for content
        let inner_area = Rect {
            x: loading_area.x + 2,
            y: loading_area.y + 2,
            width: loading_area.width - 4,
            height: loading_area.height - 4,
        };

        // Split into title and checklist areas
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(5),    // Checklist
            ])
            .split(inner_area);

        // Render title - centered and bold for "Initializing..."
        let title_text = if status.current_message == "Initializing..." {
            vec![Span::styled(
                status.current_message.as_str(),
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
            )]
        } else {
            vec![Span::styled(
                status.current_message.as_str(),
                Style::default().fg(theme.fg()),
            )]
        };

        let title =
            Paragraph::new(Line::from(title_text)).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(title, chunks[0]);

        // Create checklist items
        let items: Vec<ListItem> = status
            .steps
            .iter()
            .map(|step| {
                let checkbox = match step.status {
                    LoadingStepStatus::Completed => "[✓]",
                    LoadingStepStatus::InProgress => "[⋯]",
                    LoadingStepStatus::Pending => "[ ]",
                };

                let style = match step.status {
                    LoadingStepStatus::Completed => Style::default().fg(theme.success()),
                    LoadingStepStatus::InProgress => Style::default().fg(theme.warning()),
                    LoadingStepStatus::Pending => Style::default().fg(theme.context()),
                };

                let line = Line::from(vec![
                    Span::styled(checkbox, style),
                    Span::raw(" "),
                    Span::styled(&step.name, style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let checklist = List::new(items).style(Style::default().bg(theme.bg()));

        f.render_widget(checklist, chunks[1]);
    }

    pub fn render_error(f: &mut Frame, area: Rect, error: &str, theme: &Theme) {
        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.error()))
            .style(Style::default().bg(theme.bg()));

        let error_area = centered_rect(60, 30, area);
        f.render_widget(block, error_area);

        // Render error message
        let text_area = Rect {
            x: error_area.x + 2,
            y: error_area.y + 2,
            width: error_area.width - 4,
            height: error_area.height - 4,
        };

        let paragraph = ratatui::widgets::Paragraph::new(error)
            .style(Style::default().fg(theme.error()))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, text_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
