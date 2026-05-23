use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

pub struct HeaderComponent;

impl HeaderComponent {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        model: &str,
        memory_enabled: bool,
        memory_count: usize,
        memory_usage: u32,
    ) {
        if area.width == 0 {
            return;
        }

        let mut left_spans = vec![Span::styled(
            "Agent ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )];

        if memory_enabled {
            left_spans.push(Span::styled(
                format!("· mem {} ({}%) ", memory_count, memory_usage),
                Style::default().fg(theme.muted),
            ));
        }

        let right_spans = vec![Span::styled(
            model.to_string(),
            Style::default().fg(Color::White),
        )];

        let left_line = Line::from(left_spans);
        let left_text = Text::from(vec![left_line]);
        f.render_widget(Paragraph::new(left_text), area);

        if area.width > 30 {
            let model_width = model.len() as u16 + 4;
            let right_area = Rect {
                x: area.x + area.width.saturating_sub(model_width),
                y: area.y,
                width: model_width.min(area.width),
                height: 1,
            };
            let right_text = Text::from(vec![Line::from(right_spans)]);
            f.render_widget(Paragraph::new(right_text), right_area);
        }
    }
}

pub struct FooterComponent;

impl FooterComponent {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        model: &str,
        mode: &str,
        turn_count: usize,
        input_tokens: usize,
        output_tokens: usize,
        tool_calls: usize,
        is_streaming: bool,
    ) {
        if area.width == 0 {
            return;
        }

        let bottom_area = if area.height > 1 {
            Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: 1,
            }
        } else {
            area
        };

        let mode_color = match mode {
            "Normal" => theme.muted,
            "Insert" => theme.success,
            "Command" => theme.warning,
            "Help" => theme.accent,
            "Tools" => theme.accent,
            "Memory" => Color::Magenta,
            "Approval" => theme.warning,
            _ => theme.muted,
        };

        let left_spans = vec![
            Span::styled(model.to_string(), Style::default().fg(theme.accent)),
            Span::styled(
                format!(" · turn {}", turn_count),
                Style::default().fg(theme.muted),
            ),
            if is_streaming {
                Span::styled(" ◐", Style::default().fg(theme.warning))
            } else {
                Span::styled("", Style::default())
            },
        ];

        let mode_spans = vec![Span::styled(
            format!(" {} ", mode),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        )];

        let total_tokens = input_tokens + output_tokens;
        let estimated_cost = estimate_cost(model, input_tokens, output_tokens);

        let right_spans = vec![
            Span::styled(
                format!("{}k tok", total_tokens / 1000),
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!(" ${:.2}", estimated_cost),
                Style::default().fg(theme.muted),
            ),
            if tool_calls > 0 {
                Span::styled(
                    format!(" ⚙{}", tool_calls),
                    Style::default().fg(theme.muted),
                )
            } else {
                Span::styled(String::new(), Style::default())
            },
        ];

        let left_line = Line::from(left_spans);
        f.render_widget(Paragraph::new(Text::from(vec![left_line])), area);

        let mode_line = Line::from(mode_spans);
        if area.width > 60 {
            let mode_area = Rect {
                x: area.x + area.width / 2 - 4,
                y: area.y,
                width: 10.min(area.width),
                height: 1,
            };
            f.render_widget(Paragraph::new(Text::from(vec![mode_line])), mode_area);
        }

        if area.width > 40 {
            let right_width = 20u16.min(area.width);
            let right_area = Rect {
                x: area.x + area.width.saturating_sub(right_width),
                y: bottom_area.y,
                width: right_width,
                height: 1,
            };
            let right_text = Text::from(vec![Line::from(right_spans)]);
            f.render_widget(Paragraph::new(right_text), right_area);
        }
    }
}

fn estimate_cost(model: &str, input_tokens: usize, output_tokens: usize) -> f64 {
    let (input_per_m, output_per_m) = if model.contains("pro") {
        (2.0, 8.0)
    } else if model.contains("flash") {
        (0.1, 0.4)
    } else {
        (1.0, 3.0)
    };
    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_per_m;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_per_m;
    input_cost + output_cost
}
