use crate::theme::Theme;
use mimo_core::subagent::SubAgentStatus;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct AgentsView {
    pub selected_index: usize,
    pub show_detail: bool,
}

impl AgentsView {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            show_detail: false,
        }
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn move_down(&mut self, max: usize) {
        if self.selected_index + 1 < max {
            self.selected_index += 1;
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, agents: &[mimo_core::SubAgent]) {
        use ratatui::layout::{Constraint, Direction, Layout};

        let running = agents
            .iter()
            .filter(|a| matches!(a.status, SubAgentStatus::Running))
            .count();
        let completed = agents
            .iter()
            .filter(|a| matches!(a.status, SubAgentStatus::Completed))
            .count();

        let block = Block::default()
            .title(format!(
                " SubAgents ({} running, {} done, {} total) ",
                running,
                completed,
                agents.len()
            ))
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if agents.is_empty() {
            let msg = Paragraph::new("  No sub-agents active. Use agent_spawn to create one.")
                .style(Style::default().fg(theme.muted));
            f.render_widget(msg, inner);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(inner);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "  ID         Role          Status     Progress  Task",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        for (i, agent) in agents.iter().enumerate() {
            let (status_str, status_color) = match agent.status {
                SubAgentStatus::Running => ("▶ Running", theme.success),
                SubAgentStatus::Completed => ("✓ Done   ", theme.success),
                SubAgentStatus::Failed => ("✗ Failed ", theme.error),
                SubAgentStatus::Pending => ("⏳ Pending", theme.warning),
                SubAgentStatus::Interrupted => ("⏸ Interr ", theme.warning),
                SubAgentStatus::Cancelled => ("✕ Cancel ", theme.muted),
            };

            let role_str = format!("{:?}", agent.role);

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

            let short_id = &agent.id[..agent.id.len().min(8)];
            let prog_str = if agent.status == SubAgentStatus::Running {
                format!("{}%", agent.progress)
            } else {
                String::new()
            };
            let task_preview: String = agent.task.chars().take(40).collect();

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}{:<10}", prefix, short_id),
                    line_style.fg(theme.accent),
                ),
                Span::styled(format!("{:<12}", role_str), line_style.fg(theme.warning)),
                Span::styled(format!("{:<12}", status_str), line_style.fg(status_color)),
                Span::styled(format!("{:<8}", prog_str), line_style.fg(Color::Magenta)),
                Span::styled(task_preview, line_style),
            ]));

            if i == self.selected_index && self.show_detail {
                if let Some(ref msg) = agent.progress_message {
                    lines.push(Line::from(vec![Span::styled(
                        format!("      ⚡ {}", msg),
                        Style::default().fg(theme.muted),
                    )]));
                }
                if !agent.stream_content.is_empty() {
                    let preview: String = agent
                        .stream_content
                        .chars()
                        .rev()
                        .take(120)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    lines.push(Line::from(vec![Span::styled(
                        format!("      📝 {}", preview),
                        Style::default().fg(theme.muted),
                    )]));
                }
            } else if agent.status == SubAgentStatus::Running {
                if let Some(ref msg) = agent.progress_message {
                    lines.push(Line::from(vec![Span::styled(
                        format!("      {}", msg),
                        Style::default().fg(theme.muted),
                    )]));
                }
            }

            if let Some(ref result) = agent.result {
                if agent.status == SubAgentStatus::Completed
                    || agent.status == SubAgentStatus::Failed
                {
                    let summary: String = result.summary.chars().take(80).collect();
                    lines.push(Line::from(vec![Span::styled(
                        format!("      {}", summary),
                        Style::default().fg(theme.muted),
                    )]));
                }
            }
        }

        f.render_widget(Paragraph::new(lines), chunks[0]);

        let help = Line::from(vec![Span::styled(
            "  [j/k] Navigate  [d] Toggle detail  [r] Refresh  [c] Cancel selected  [q] Back to chat",
            Style::default().fg(theme.muted),
        )]);
        f.render_widget(Paragraph::new(help), chunks[1]);
    }
}

impl Default for AgentsView {
    fn default() -> Self {
        Self::new()
    }
}
