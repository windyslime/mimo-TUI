use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::Span,
};
use std::collections::VecDeque;
use unicode_width::UnicodeWidthStr;

const MAX_HISTORY: usize = 50;

fn is_ctrl_or_cmd(mods: KeyModifiers) -> bool {
    mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::SUPER)
}

pub struct InputComponent {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    placeholder: String,
    history: VecDeque<String>,
    history_index: Option<usize>,
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

impl InputComponent {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            placeholder: "输入消息... (Ctrl+Enter 发送, Esc 取消)".to_string(),
            history: VecDeque::with_capacity(MAX_HISTORY),
            history_index: None,
        }
    }

    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.history_index = None;
    }

    pub fn set_placeholder(&mut self, text: String) {
        self.placeholder = text;
    }

    pub fn paste_text(&mut self, text: &str) {
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                let col = self.cursor_col;
                let rest =
                    self.current_line()[char_to_byte(self.current_line(), col)..].to_string();
                let current_line = self.current_line_mut();
                let byte_idx = char_to_byte(current_line, col);
                current_line.replace_range(byte_idx.., "");
                self.lines.insert(self.cursor_line + 1, rest);
                self.cursor_line += 1;
                self.cursor_col = 0;
            }
            for ch in line.chars() {
                if ch.is_control() {
                    continue;
                }
                let col = self.cursor_col;
                let current = self.current_line_mut();
                let chars: Vec<char> = current.chars().collect();
                let idx = col.min(chars.len());
                if let Some(byte_idx) = current.char_indices().nth(idx).map(|(i, _)| i) {
                    current.insert(byte_idx, ch);
                } else {
                    current.push(ch);
                }
                self.cursor_col += 1;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        if is_ctrl_or_cmd(key.modifiers) {
            match key.code {
                KeyCode::Char('a') => {
                    self.cursor_col = 0;
                    return InputAction::None;
                }
                KeyCode::Char('e') => {
                    self.cursor_col = self.current_line().chars().count();
                    return InputAction::None;
                }
                KeyCode::Char('w') => {
                    self.delete_prev_word();
                    return InputAction::None;
                }
                KeyCode::Char('u') => {
                    let col = self.cursor_col;
                    let line = self.current_line_mut();
                    let byte_idx = char_to_byte(line, col);
                    line.replace_range(..byte_idx, "");
                    self.cursor_col = 0;
                    return InputAction::None;
                }
                KeyCode::Char('k') => {
                    let col = self.cursor_col;
                    let line = self.current_line_mut();
                    let chars: Vec<char> = line.chars().collect();
                    if col < chars.len() {
                        let byte_idx = char_to_byte(line, col);
                        line.replace_range(byte_idx.., "");
                    }
                    return InputAction::None;
                }
                KeyCode::Enter | KeyCode::Char('j') => {
                    let value = self.value();
                    if !value.trim().is_empty() {
                        if self.history.len() >= MAX_HISTORY {
                            self.history.pop_front();
                        }
                        self.history.push_back(value.clone());
                    }
                    self.history_index = None;
                    return InputAction::Submit(value);
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Enter => {
                    let col = self.cursor_col;
                    let line = self.current_line().to_string();
                    let chars: Vec<char> = line.chars().collect();
                    let rest: String = chars[col.min(chars.len())..].iter().collect();
                    let current_line = self.current_line_mut();
                    let byte_idx = char_to_byte(current_line, col);
                    current_line.replace_range(byte_idx.., "");
                    self.lines.insert(self.cursor_line + 1, rest);
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                    return InputAction::None;
                }
                KeyCode::Esc => {
                    return InputAction::Cancel;
                }
                KeyCode::Backspace => {
                    if self.cursor_col > 0 {
                        let col = self.cursor_col;
                        let line = self.current_line_mut();
                        let chars: Vec<char> = line.chars().collect();
                        let idx = col.min(chars.len());
                        if idx > 0 {
                            let char_idx = line
                                .char_indices()
                                .nth(idx - 1)
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            line.remove(char_idx);
                            self.cursor_col -= 1;
                        }
                    } else if self.cursor_line > 0 {
                        let current = self.lines.remove(self.cursor_line);
                        self.cursor_line -= 1;
                        let prev_len = self.current_line().chars().count();
                        self.current_line_mut().push_str(&current);
                        self.cursor_col = prev_len;
                    }
                    return InputAction::None;
                }
                KeyCode::Delete => {
                    let col = self.cursor_col;
                    let line = self.current_line_mut();
                    let chars: Vec<char> = line.chars().collect();
                    if col < chars.len() {
                        let char_idx = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(0);
                        line.remove(char_idx);
                    } else if self.cursor_line + 1 < self.lines.len() {
                        let next = self.lines.remove(self.cursor_line + 1);
                        self.current_line_mut().push_str(&next);
                    }
                    return InputAction::None;
                }
                KeyCode::Left => {
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                    } else if self.cursor_line > 0 {
                        self.cursor_line -= 1;
                        self.cursor_col = self.current_line().chars().count();
                    }
                    return InputAction::None;
                }
                KeyCode::Right => {
                    let line_len = self.current_line().chars().count();
                    if self.cursor_col < line_len {
                        self.cursor_col += 1;
                    } else if self.cursor_line + 1 < self.lines.len() {
                        self.cursor_line += 1;
                        self.cursor_col = 0;
                    }
                    return InputAction::None;
                }
                KeyCode::Up => {
                    self.history_up();
                    return InputAction::None;
                }
                KeyCode::Down => {
                    self.history_down();
                    return InputAction::None;
                }
                KeyCode::Home => {
                    self.cursor_col = 0;
                    return InputAction::None;
                }
                KeyCode::End => {
                    self.cursor_col = self.current_line().chars().count();
                    return InputAction::None;
                }
                KeyCode::Char(c) => {
                    let col = self.cursor_col;
                    let line = self.current_line_mut();
                    let chars: Vec<char> = line.chars().collect();
                    let idx = col.min(chars.len());
                    if let Some(byte_idx) = line.char_indices().nth(idx).map(|(i, _)| i) {
                        line.insert(byte_idx, c);
                    } else {
                        line.push(c);
                    }
                    self.cursor_col += 1;
                    return InputAction::None;
                }
                _ => {}
            }
        }
        InputAction::None
    }

    fn current_line(&self) -> &str {
        &self.lines[self.cursor_line.min(self.lines.len().saturating_sub(1))]
    }

    fn current_line_mut(&mut self) -> &mut String {
        let idx = self.cursor_line.min(self.lines.len().saturating_sub(1));
        &mut self.lines[idx]
    }

    fn delete_prev_word(&mut self) {
        if self.cursor_col == 0 {
            return;
        }
        let col = self.cursor_col;
        let line = self.current_line_mut();
        let chars: Vec<char> = line.chars().collect();
        let start = col.min(chars.len());
        let mut new_col = start;
        while new_col > 0 && chars[new_col - 1].is_whitespace() {
            new_col -= 1;
        }
        while new_col > 0 && !chars[new_col - 1].is_whitespace() {
            new_col -= 1;
        }
        let start_byte = line
            .char_indices()
            .nth(new_col)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let end_byte = line
            .char_indices()
            .nth(start)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        line.replace_range(start_byte..end_byte, "");
        self.cursor_col = new_col;
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            None => {
                self.history_index = Some(self.history.len().saturating_sub(1));
                self.history.len().saturating_sub(1)
            }
            Some(i) if i > 0 => {
                self.history_index = Some(i - 1);
                i - 1
            }
            Some(i) => i,
        };
        if let Some(entry) = self.history.get(idx) {
            self.lines = entry.split('\n').map(|s| s.to_string()).collect();
            self.cursor_line = self.lines.len().saturating_sub(1);
            self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
        }
    }

    fn history_down(&mut self) {
        match self.history_index {
            None => return,
            Some(i) if i + 1 < self.history.len() => {
                self.history_index = Some(i + 1);
                if let Some(entry) = self.history.get(i + 1) {
                    self.lines = entry.split('\n').map(|s| s.to_string()).collect();
                    self.cursor_line = self.lines.len().saturating_sub(1);
                    self.cursor_col = self.lines.last().map(|l| l.chars().count()).unwrap_or(0);
                }
            }
            Some(_) => {
                self.history_index = None;
                self.lines = vec![String::new()];
                self.cursor_line = 0;
                self.cursor_col = 0;
            }
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, focused: bool) -> Option<Position> {
        if area.width == 0 || area.height == 0 {
            return None;
        }

        let title_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };

        let input_area = if area.height > 1 {
            Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: area.height - 1,
            }
        } else {
            area
        };

        use ratatui::text::{Line, Text};
        use ratatui::widgets::Paragraph;

        let title_line = Line::from(vec![Span::styled(
            "Composer",
            Style::default().fg(theme.muted),
        )]);
        f.render_widget(Paragraph::new(Text::from(vec![title_line])), title_area);

        if self.lines.is_empty() || (self.lines.len() == 1 && self.lines[0].is_empty()) {
            let placeholder_text = vec![Line::from(vec![
                Span::styled("▌", Style::default().fg(theme.muted)),
                Span::styled(
                    &self.placeholder,
                    Style::default()
                        .fg(theme.muted)
                        .add_modifier(Modifier::ITALIC),
                ),
            ])];
            let placeholder = Paragraph::new(Text::from(placeholder_text));
            f.render_widget(placeholder, input_area);
            None
        } else {
            let text_lines: Vec<Line> = self
                .lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    let prefix = if i == 0 {
                        Span::styled("▌ ", Style::default().fg(theme.muted))
                    } else {
                        Span::styled("  ", Style::default().fg(theme.muted))
                    };
                    Line::from(vec![prefix, Span::raw(line.as_str())])
                })
                .collect();

            let cursor_pos = if focused && self.cursor_line < self.lines.len() {
                let line = &self.lines[self.cursor_line];
                let prefix_len = 2u16;
                let char_idx: String = line.chars().take(self.cursor_col).collect();
                let cursor_x = prefix_len.saturating_add(char_idx.width() as u16);
                Some(Position::new(
                    input_area.x.saturating_add(cursor_x),
                    input_area.y.saturating_add(self.cursor_line as u16),
                ))
            } else {
                None
            };

            let paragraph =
                Paragraph::new(Text::from(text_lines)).wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(paragraph, input_area);
            cursor_pos
        }
    }
}

pub enum InputAction {
    None,
    Submit(String),
    Cancel,
}

impl Default for InputComponent {
    fn default() -> Self {
        Self::new()
    }
}
