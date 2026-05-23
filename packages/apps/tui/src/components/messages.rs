use crate::theme::Theme;
use chrono::{DateTime, Utc};
use mimo_core::session::MessageRole;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};
use std::time::Instant;
use uuid::Uuid;

pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    #[allow(dead_code)]
    pub timestamp: DateTime<Utc>,
    pub thinking: Option<String>,
    pub thinking_expanded: bool,
    pub thinking_start: Option<Instant>,
    pub thinking_duration: Option<Duration>,
    pub tool_calls: Option<Vec<ToolCallInfo>>,
}

use std::time::Duration;

pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub status: ToolCallStatus,
}

#[allow(dead_code)]
pub enum ToolCallStatus {
    Pending,
    Running,
    Success,
    Error(String),
}

pub struct MessagesComponent {
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
}

impl MessagesComponent {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) -> String {
        let id = Uuid::new_v4().to_string();
        self.messages.push(ChatMessage {
            id: id.clone(),
            role,
            content,
            timestamp: Utc::now(),
            thinking: None,
            thinking_expanded: false,
            thinking_start: None,
            thinking_duration: None,
            tool_calls: None,
        });
        self.scroll_to_bottom();
        id
    }

    pub fn add_thinking(&mut self, msg_id: &str, thinking: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            if msg.thinking_start.is_none() {
                msg.thinking_start = Some(Instant::now());
            }
            match &mut msg.thinking {
                Some(existing) => existing.push_str(thinking),
                None => msg.thinking = Some(thinking.to_string()),
            }
            msg.thinking_expanded = true;
        }
    }

    pub fn finish_thinking(&mut self, msg_id: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            if let Some(start) = msg.thinking_start.take() {
                msg.thinking_duration = Some(start.elapsed());
            }
        }
    }

    pub fn append_content(&mut self, msg_id: &str, delta: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.content.push_str(delta);
        }
    }

    pub fn add_tool_call(&mut self, msg_id: &str, tool_id: &str, name: &str, arguments: &str) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            let tc = ToolCallInfo {
                id: tool_id.to_string(),
                name: name.to_string(),
                arguments: arguments.to_string(),
                result: None,
                status: ToolCallStatus::Running,
            };
            match &mut msg.tool_calls {
                Some(list) => list.push(tc),
                None => msg.tool_calls = Some(vec![tc]),
            }
        }
    }

    pub fn update_tool_result(
        &mut self,
        msg_id: &str,
        tool_id: &str,
        result: String,
        success: bool,
    ) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            if let Some(tool_calls) = &mut msg.tool_calls {
                for tc in tool_calls.iter_mut() {
                    if tc.id == tool_id {
                        tc.status = if success {
                            tc.result = Some(result);
                            ToolCallStatus::Success
                        } else {
                            let err_msg = result.clone();
                            tc.result = Some(result);
                            ToolCallStatus::Error(err_msg)
                        };
                        break;
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn find_message_mut(&mut self, msg_id: &str) -> Option<&mut ChatMessage> {
        self.messages.iter_mut().find(|m| m.id == msg_id)
    }

    #[allow(dead_code)]
    pub fn last_message_mut(&mut self) -> Option<&mut ChatMessage> {
        self.messages.last_mut()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let inner = area;

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            lines.push(Line::from(""));
            lines.push(self.render_role_line(&msg.role, theme));

            if let Some(ref thinking) = msg.thinking {
                if msg.thinking_expanded {
                    let duration_str = if let Some(dur) = msg.thinking_duration {
                        format_duration(dur)
                    } else if let Some(start) = msg.thinking_start {
                        format_duration(start.elapsed())
                    } else {
                        "...".to_string()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("○ ", Style::default().fg(theme.muted)),
                        Span::styled(
                            "thinking",
                            Style::default()
                                .fg(theme.muted)
                                .add_modifier(Modifier::ITALIC),
                        ),
                        Span::styled(
                            format!(" · {}", duration_str),
                            Style::default().fg(theme.muted),
                        ),
                    ]));
                    for think_line in thinking.lines() {
                        lines.push(Line::from(vec![Span::styled(
                            format!(" │ {}", think_line),
                            Style::default().fg(theme.muted),
                        )]));
                    }
                }
            }

            for content_line in msg.content.lines() {
                lines.extend(self.render_markdown_line(content_line, theme));
            }

            if let Some(ref tool_calls) = msg.tool_calls {
                for tc in tool_calls {
                    let (status_icon, status_color) = match tc.status {
                        ToolCallStatus::Pending => ("⏳", theme.warning),
                        ToolCallStatus::Running => ("⚙", theme.accent),
                        ToolCallStatus::Success => ("✓", theme.success),
                        ToolCallStatus::Error(_) => ("✗", theme.error),
                    };
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {} Tool: {} ", status_icon, tc.name),
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    if !tc.arguments.is_empty() {
                        let args_preview: String = tc.arguments.chars().take(100).collect();
                        lines.push(Line::from(vec![Span::styled(
                            format!("     Args: {}", args_preview),
                            Style::default().fg(theme.muted),
                        )]));
                    }
                    if let Some(ref result) = tc.result {
                        let result_preview: String = result
                            .lines()
                            .next()
                            .unwrap_or("")
                            .chars()
                            .take(80)
                            .collect();
                        if !result_preview.is_empty() {
                            lines.push(Line::from(vec![Span::styled(
                                format!("     Result: {}", result_preview),
                                Style::default().fg(theme.muted),
                            )]));
                        }
                    }
                }
            }
        }

        if self.auto_scroll {
            let total_lines = lines.len() as u16;
            let skip = if total_lines > inner.height {
                total_lines.saturating_sub(inner.height) as usize
            } else {
                0
            };
            let start = skip.min(lines.len().saturating_sub(1));
            let end = lines.len();
            let visible_lines: Vec<Line> = lines[start..end].to_vec();
            f.render_widget(Paragraph::new(Text::from(visible_lines)), inner);
        } else {
            let skip = self.scroll_offset.min(lines.len().saturating_sub(1));
            let visible_lines: Vec<Line> = lines[skip..].to_vec();
            f.render_widget(Paragraph::new(Text::from(visible_lines)), inner);
        }
    }

    fn render_role_line(&self, role: &MessageRole, theme: &Theme) -> Line<'_> {
        match role {
            MessageRole::User => Line::from(vec![Span::styled(
                "You ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            MessageRole::Assistant => Line::from(vec![Span::styled(
                "Assistant ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            MessageRole::System => Line::from(vec![Span::styled(
                "System ",
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::ITALIC),
            )]),
            MessageRole::Tool => Line::from(vec![Span::styled(
                "Tool ",
                Style::default().fg(theme.warning),
            )]),
        }
    }

    fn render_markdown_line<'a>(&self, line: &'a str, theme: &Theme) -> Vec<Line<'a>> {
        if line.starts_with("```") {
            return vec![Line::from(vec![Span::styled(
                line,
                Style::default().fg(theme.warning),
            )])];
        }
        if line.starts_with("### ") {
            return vec![Line::from(vec![Span::styled(
                line,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )])];
        }
        if line.starts_with("## ") {
            return vec![Line::from(vec![Span::styled(
                line,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )])];
        }
        if line.starts_with("# ") {
            return vec![Line::from(vec![Span::styled(
                line,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )])];
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            return vec![Line::from(vec![Span::raw("  • "), Span::raw(&line[2..])])];
        }
        if let Some(num_dot) = line.find(". ") {
            if line[..num_dot].chars().all(|c| c.is_ascii_digit()) {
                return vec![Line::from(vec![Span::raw("  "), Span::raw(line)])];
            }
        }

        let mut result = Vec::new();
        let mut spans: Vec<Span> = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();

        while i < len {
            if chars[i] == '`' {
                let start = i;
                i += 1;
                while i < len && chars[i] != '`' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                let code: String = line[start..i.min(line.len())]
                    .chars()
                    .filter(|c| *c != '`')
                    .collect();
                spans.push(Span::styled(
                    code,
                    Style::default().fg(theme.warning).bg(theme.muted),
                ));
            } else if chars[i] == '*' && i + 1 < len && chars[i + 1] == '*' {
                i += 2;
                let mut bold_text = String::new();
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '*') {
                    bold_text.push(chars[i]);
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                }
                spans.push(Span::styled(
                    bold_text,
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            } else if chars[i] == '*' || chars[i] == '_' {
                let marker = chars[i];
                i += 1;
                let mut italic_text = String::new();
                while i < len && chars[i] != marker {
                    italic_text.push(chars[i]);
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                spans.push(Span::styled(
                    italic_text,
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
            } else {
                spans.push(Span::raw(chars[i].to_string()));
                i += 1;
            }
        }

        result.push(Line::from(spans));
        result
    }
}

impl Default for MessagesComponent {
    fn default() -> Self {
        Self::new()
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let ms = duration.subsec_millis();
    if secs >= 60 {
        let mins = secs / 60;
        let remain_secs = secs % 60;
        format!("{}m{}s", mins, remain_secs)
    } else if secs > 0 {
        if ms > 0 {
            format!("{}.{:01}s", secs, ms / 100)
        } else {
            format!("{}s", secs)
        }
    } else {
        format!("{}ms", ms)
    }
}
