use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub enum Command {
    Help,
    Memory(MemorySubCommand),
    Tools,
    Sessions,
    SwitchSession(String),
    Clear,
    Compact,
    ModelSwitch(String),
    Models,
    Quit,
    Unknown(String),
}

pub enum MemorySubCommand {
    Show,
    Add(String),
    Clear,
}

pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    let parts: Vec<&str> = input.split_whitespace().collect();

    match parts.first().copied() {
        Some("/help") | Some("/h") => Command::Help,
        Some("/memory") | Some("/mem") => parse_memory_command(&parts),
        Some("/tools") | Some("/t") => Command::Tools,
        Some("/sessions") | Some("/s") => Command::Sessions,
        Some("/session") => {
            if parts.len() > 1 {
                Command::SwitchSession(parts[1].to_string())
            } else {
                Command::Sessions
            }
        }
        Some("/clear") => Command::Clear,
        Some("/compact") => Command::Compact,
        Some("/model") => {
            if parts.len() > 1 {
                Command::ModelSwitch(parts[1..].join(" "))
            } else {
                Command::Models
            }
        }
        Some("/models") => Command::Models,
        Some("/quit") | Some("/exit") | Some("/q") => Command::Quit,
        Some(cmd) if cmd.starts_with('/') => Command::Unknown(cmd.to_string()),
        _ => Command::Unknown(input.to_string()),
    }
}

fn parse_memory_command(parts: &[&str]) -> Command {
    match parts.get(1).copied() {
        None | Some("show") => Command::Memory(MemorySubCommand::Show),
        Some("add") => {
            let content = parts[2..].join(" ");
            if content.is_empty() {
                Command::Memory(MemorySubCommand::Show)
            } else {
                Command::Memory(MemorySubCommand::Add(content))
            }
        }
        Some("clear") => Command::Memory(MemorySubCommand::Clear),
        _ => Command::Memory(MemorySubCommand::Show),
    }
}

pub fn get_command_completions(
    input: &str,
    model_ids: &[String],
    session_ids: &[String],
) -> Vec<String> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return Vec::new();
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    if parts.len() == 1 {
        let prefix = parts[0].to_lowercase();
        let all_commands = [
            "/help",
            "/memory",
            "/tools",
            "/sessions",
            "/session",
            "/clear",
            "/compact",
            "/model",
            "/models",
            "/quit",
        ];
        all_commands
            .iter()
            .filter(|c| c.to_lowercase().starts_with(&prefix))
            .map(|c| c.to_string())
            .collect()
    } else {
        match parts[0] {
            "/memory" | "/mem" if parts.len() == 2 => {
                let sub_prefix = parts[1].to_lowercase();
                let subs = ["show", "add", "clear"];
                subs.iter()
                    .filter(|s| s.to_lowercase().starts_with(&sub_prefix))
                    .map(|s| format!("{} {}", parts[0], s))
                    .collect()
            }
            "/model" => {
                let arg_prefix = parts[1..].join(" ").to_lowercase();
                model_ids
                    .iter()
                    .filter(|m| m.to_lowercase().starts_with(&arg_prefix))
                    .map(|m| format!("/model {}", m))
                    .collect()
            }
            "/session" => {
                let arg_prefix = parts.get(1).copied().unwrap_or("").to_lowercase();
                session_ids
                    .iter()
                    .filter(|s| s.to_lowercase().starts_with(&arg_prefix))
                    .map(|s| format!("/session {}", s))
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

pub fn is_quick_memory(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.starts_with('#') && trimmed.len() > 1 {
        Some(trimmed[1..].trim().to_string())
    } else {
        None
    }
}

pub fn render_tools_popup(f: &mut Frame, area: Rect, theme: &Theme, tools: &[String]) {
    let popup_width = area.width.min(50);
    let popup_height = area.height.min(tools.len() as u16 + 4);
    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Available Tools",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for tool in tools {
        lines.push(Line::from(vec![Span::styled(
            format!("  • {}", tool),
            Style::default().fg(theme.accent),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  Press any key to close",
        Style::default().fg(theme.muted),
    )]));

    let block = Block::default()
        .title(" Tools ")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.surface));
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn render_memory_popup(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    entries: &[String],
    total_chars: usize,
    limit_chars: usize,
) {
    let popup_width = area.width.min(60);
    let popup_height = area.height.min(entries.len() as u16 + 6);
    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            format!(
                "Memory ({} entries | {} / {} chars)",
                entries.len(),
                total_chars,
                limit_chars
            ),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for entry in entries {
        lines.push(Line::from(vec![Span::styled(
            format!("  - {}", entry),
            Style::default().fg(theme.accent),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  [q] Close  [c] Clear  [a] Add entry",
        Style::default().fg(theme.muted),
    )]));

    let block = Block::default()
        .title(" Memory ")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.surface));
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}
