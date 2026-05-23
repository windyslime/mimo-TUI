use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct HelpView;

impl HelpView {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let popup_width = area.width.min(55);
        let popup_height = area.height.min(18);
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(Clear, popup_area);

        let lines = vec![
            Line::from(vec![Span::styled(
                "MiMo-TUI Help",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation:",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Tab       Switch view (Chat/Agents/History)"),
            Line::from("  j/↑       Scroll up / Navigate up"),
            Line::from("  k/↓       Scroll down / Navigate down"),
            Line::from("  Ctrl+D    Page down"),
            Line::from("  Ctrl+U    Page up"),
            Line::from("  g         Jump to top"),
            Line::from("  G         Jump to bottom"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Input:",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  i         Enter Insert mode"),
            Line::from("  Esc       Cancel / Exit mode"),
            Line::from("  Ctrl+Enter Send message"),
            Line::from("  ↑/↓       Browse input history"),
            Line::from("  Ctrl+W    Delete previous word"),
            Line::from("  Ctrl+U    Delete to line start"),
            Line::from("  Ctrl+K    Delete to line end"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Commands:",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  /help     Show this help"),
            Line::from("  /memory   View/manage memory"),
            Line::from("  /tools    List available tools"),
            Line::from("  /sessions List sessions"),
            Line::from("  /clear    Clear conversation"),
            Line::from("  /quit     Exit application"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Press any key to close",
                Style::default().fg(theme.muted),
            )]),
        ];

        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .style(Style::default().bg(theme.surface));
        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, popup_area);
    }
}

impl Default for HelpView {
    fn default() -> Self {
        Self::new()
    }
}
