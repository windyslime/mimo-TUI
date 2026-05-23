use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

pub struct SidebarPanel {
    pub title: String,
    pub content: Vec<String>,
    pub is_active: bool,
}

impl SidebarPanel {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            content: Vec::new(),
            is_active: false,
        }
    }

    pub fn with_content(mut self, content: Vec<String>) -> Self {
        self.content = content;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let border_color = if self.is_active {
            theme.accent
        } else {
            theme.muted
        };

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let text = if self.content.is_empty() {
            Text::from(Line::from(Span::styled(
                format!("No {}", self.title.to_lowercase()),
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::ITALIC),
            )))
        } else {
            let lines: Vec<Line> = self
                .content
                .iter()
                .map(|line| Line::from(Span::raw(line)))
                .collect();
            Text::from(lines)
        };

        f.render_widget(Paragraph::new(text), inner);
    }
}
