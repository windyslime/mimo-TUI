use crate::components::{ApprovalRequest, InputComponent, MessagesComponent};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub struct ChatView;

impl ChatView {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        messages: &MessagesComponent,
        input: &InputComponent,
        input_focused: bool,
        is_streaming: bool,
        approval: Option<&ApprovalRequest>,
    ) -> Option<Position> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(input_height(input) as u16),
            ])
            .split(area);

        messages.render(f, chunks[0], theme);

        if is_streaming {
            let status = Paragraph::new(Line::from(vec![Span::styled(
                " ◐ 正在思考...",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::ITALIC),
            )]));
            let status_area = Rect {
                x: chunks[1].x,
                y: chunks[1].y.saturating_sub(1),
                width: chunks[1].width,
                height: 1,
            };
            f.render_widget(status, status_area);
        }

        let cursor = input.render(f, chunks[1], theme, input_focused);

        if let Some(approval) = approval {
            approval.render(f, area, theme);
        }

        cursor
    }
}

fn input_height(input: &InputComponent) -> usize {
    let lines = input.value().lines().count();
    (lines + 2).max(3).min(10)
}

impl Default for ChatView {
    fn default() -> Self {
        Self::new()
    }
}
