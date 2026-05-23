use mimo_config::ThemeConfig;
use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub surface: Color,
    #[allow(dead_code)]
    pub text_primary: Color,
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        Self {
            accent: parse_color(&config.accent).unwrap_or(Color::Cyan),
            success: parse_color(&config.success).unwrap_or(Color::Green),
            warning: parse_color(&config.warning).unwrap_or(Color::Yellow),
            error: parse_color(&config.error).unwrap_or(Color::Red),
            muted: parse_color(&config.muted).unwrap_or(Color::DarkGray),
            surface: parse_color(&config.surface).unwrap_or(Color::Black),
            text_primary: parse_color(&config.text_primary).unwrap_or(Color::White),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            muted: Color::DarkGray,
            surface: Color::Black,
            text_primary: Color::White,
        }
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }

    match s.as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "white" => Some(Color::White),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        _ if s.starts_with("rgb(") || s.starts_with("rgba(") => parse_rgb_func(&s),
        _ => None,
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim();
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn parse_rgb_func(s: &str) -> Option<Color> {
    let inner = if let Some(stripped) = s.strip_prefix("rgba(") {
        stripped.strip_suffix(')')?
    } else if let Some(stripped) = s.strip_prefix("rgb(") {
        stripped.strip_suffix(')')?
    } else {
        return None;
    };

    let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parts[0].parse::<u8>().ok()?;
    let g = parts[1].parse::<u8>().ok()?;
    let b = parts[2].parse::<u8>().ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named_colors() {
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color("white"), Some(Color::White));
        assert_eq!(parse_color("black"), Some(Color::Black));
    }

    #[test]
    fn test_parse_hex_colors() {
        assert_eq!(parse_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("#0000ff"), Some(Color::Rgb(0, 0, 255)));
    }

    #[test]
    fn test_parse_rgb_func() {
        assert_eq!(parse_color("rgb(255, 0, 0)"), Some(Color::Rgb(255, 0, 0)));
    }
}
