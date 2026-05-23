use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct ApprovalRequest {
    pub tool_name: String,
    pub arguments: String,
    pub risk_level: RiskLevel,
}

#[allow(dead_code)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[allow(dead_code)]
pub enum ApprovalAction {
    Approve,
    Reject,
    ApproveAll,
    Cancel,
}

impl ApprovalRequest {
    #[allow(dead_code)]
    pub fn new(tool_name: String, arguments: String) -> Self {
        let risk = if arguments.contains("rm -rf")
            || arguments.contains("sudo")
            || arguments.contains("delete")
        {
            RiskLevel::High
        } else if arguments.contains("write") || arguments.contains(">") || arguments.contains("mv")
        {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        Self {
            tool_name,
            arguments,
            risk_level: risk,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let popup_width = area.width.min(60);
        let popup_height = 10;
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(Clear, popup_area);

        let (risk_text, risk_color) = match self.risk_level {
            RiskLevel::Low => ("🟢 Low", theme.success),
            RiskLevel::Medium => ("🟡 Medium", theme.warning),
            RiskLevel::High => ("🔴 High (destructive)", theme.error),
        };

        let lines = vec![
            Line::from(vec![Span::styled(
                "Tool Approval Required",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("  Tool: {}", self.tool_name),
                Style::default().fg(theme.accent),
            )]),
            Line::from(vec![Span::styled(
                "  Arguments:",
                Style::default().fg(theme.muted),
            )]),
            Line::from(vec![Span::styled(
                format!("    {}", self.arguments),
                Style::default().fg(theme.warning),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("  Risk: {}", risk_text),
                Style::default().fg(risk_color).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  [a] Approve  [r] Reject  [A] Approve All  [q] Cancel",
                Style::default().fg(theme.muted),
            )]),
        ];

        let block = Block::default()
            .title(" Tool Approval ")
            .borders(Borders::ALL)
            .style(Style::default().bg(theme.surface));
        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, popup_area);
    }
}
