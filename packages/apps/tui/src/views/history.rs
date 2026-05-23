use crate::theme::Theme;
use mimo_core::Session;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct HistoryView {
    pub selected_index: usize,
}

impl HistoryView {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn move_down(&mut self, max: usize) {
        if self.selected_index + 1 < max {
            self.selected_index += 1;
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, sessions: &[Session]) {
        let block = Block::default().title(" Sessions ").borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        if sessions.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  No sessions found",
                Style::default().fg(theme.muted),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "  Created         Preview                     Messages",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]));

            for (i, session) in sessions.iter().enumerate() {
                let time_str = session.created_at.format("%m-%d %H:%M").to_string();
                let preview = session.preview();
                let preview: String = preview.chars().take(45).collect();
                let msg_count = session.messages.len();

                let prefix = if i == self.selected_index {
                    "▶ "
                } else {
                    "  "
                };
                let line_style = if i == self.selected_index {
                    Style::default().bg(theme.muted)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}{:<16}", prefix, time_str),
                        line_style.fg(theme.accent),
                    ),
                    Span::styled(format!("{:<48}", preview), line_style.fg(theme.warning)),
                    Span::styled(format!("{}", msg_count), line_style.fg(theme.success)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  [Enter] Open  [d] Delete  [q] Back",
            Style::default().fg(theme.muted),
        )]));

        f.render_widget(Paragraph::new(lines), inner);
    }
}

impl Default for HistoryView {
    fn default() -> Self {
        Self::new()
    }
}
