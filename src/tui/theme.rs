use ratatui::style::Color;
use ratatui::widgets::BorderType;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub selection: Color,
    pub bg: Color,
    pub fg: Color,
    pub header_bg: Color,
    pub footer_bg: Color,
    pub show_emojis: bool,
    pub border_type: BorderType,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ThemeConfig {
    pub palette: Option<PaletteConfig>,
    pub ui: Option<UiConfig>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PaletteConfig {
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub accent: Option<String>,
    pub success: Option<String>,
    pub error: Option<String>,
    pub selection: Option<String>,
    pub bg: Option<String>,
    pub fg: Option<String>,
    pub header_bg: Option<String>,
    pub footer_bg: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UiConfig {
    pub show_emojis: Option<bool>,
    pub border_type: Option<String>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Gray,
            accent: Color::Yellow,
            success: Color::Green,
            error: Color::Red,
            selection: Color::Magenta,
            bg: Color::Black,
            fg: Color::White,
            header_bg: Color::Indexed(234), // Subtle dark gray
            footer_bg: Color::Indexed(234), // Subtle dark gray
            show_emojis: true,
            border_type: BorderType::Plain,
        }
    }
}

impl Theme {
    pub fn load(path: &Path) -> Self {
        if let Ok(content) = std::fs::read_to_string(path) {
            match toml::from_str::<ThemeConfig>(&content) {
                Ok(config) => return Self::from_config(config),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse theme file '{}': {}",
                        path.display(),
                        e
                    );
                }
            }
        }
        Self::default()
    }

    pub fn from_config(config: ThemeConfig) -> Self {
        let mut theme = Self::default();

        if let Some(palette) = config.palette {
            if let Some(c) = palette.primary.as_deref().and_then(parse_color) {
                theme.primary = c;
            }
            if let Some(c) = palette.secondary.as_deref().and_then(parse_color) {
                theme.secondary = c;
            }
            if let Some(c) = palette.accent.as_deref().and_then(parse_color) {
                theme.accent = c;
            }
            if let Some(c) = palette.success.as_deref().and_then(parse_color) {
                theme.success = c;
            }
            if let Some(c) = palette.error.as_deref().and_then(parse_color) {
                theme.error = c;
            }
            if let Some(c) = palette.selection.as_deref().and_then(parse_color) {
                theme.selection = c;
            }
            if let Some(c) = palette.bg.as_deref().and_then(parse_color) {
                theme.bg = c;
            }
            if let Some(c) = palette.fg.as_deref().and_then(parse_color) {
                theme.fg = c;
            }
            if let Some(c) = palette.header_bg.as_deref().and_then(parse_color) {
                theme.header_bg = c;
            }
            if let Some(c) = palette.footer_bg.as_deref().and_then(parse_color) {
                theme.footer_bg = c;
            }
        }

        if let Some(ui) = config.ui {
            if let Some(show) = ui.show_emojis {
                theme.show_emojis = show;
            }
            if let Some(bt) = ui.border_type.as_deref().and_then(parse_border) {
                theme.border_type = bt;
            }
        }

        theme
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" => Some(Color::Gray),
        "white" => Some(Color::White),
        "dark-gray" | "dark_gray" => Some(Color::DarkGray),
        "light-red" | "light_red" => Some(Color::LightRed),
        "light-green" | "light_green" => Some(Color::LightGreen),
        "light-yellow" | "light_yellow" => Some(Color::LightYellow),
        "light-blue" | "light_blue" => Some(Color::LightBlue),
        "light-magenta" | "light_magenta" => Some(Color::LightMagenta),
        "light-cyan" | "light_cyan" => Some(Color::LightCyan),
        "reset" => Some(Color::Reset),
        _ => {
            if (s.starts_with("indexed(") || s.starts_with("color(")) && s.ends_with(')') {
                let start = if s.starts_with("indexed(") { 8 } else { 6 };
                let num_str = &s[start..s.len() - 1];
                if let Ok(n) = num_str.parse::<u8>() {
                    return Some(Color::Indexed(n));
                }
            }

            let hex = s.strip_prefix('#').unwrap_or(s);
            match hex.len() {
                3 => {
                    let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                    let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                    let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                    Some(Color::Rgb(r * 17, g * 17, b * 17))
                }
                6 => {
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    Some(Color::Rgb(r, g, b))
                }
                _ => {
                    eprintln!("Warning: Unknown color format '{}'", s);
                    None
                }
            }
        }
    }
}

fn parse_border(s: &str) -> Option<BorderType> {
    match s.to_lowercase().as_str() {
        "plain" => Some(BorderType::Plain),
        "rounded" => Some(BorderType::Rounded),
        "thick" => Some(BorderType::Thick),
        "double" => Some(BorderType::Double),
        "none" => Some(BorderType::Plain),
        _ => {
            eprintln!("Warning: Unknown border type '{}'", s);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_color_named() {
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("dark-gray"), Some(Color::DarkGray));
        assert_eq!(parse_color("Light_Green"), Some(Color::LightGreen));
        assert_eq!(parse_color("black"), Some(Color::Black));
        assert_eq!(parse_color("green"), Some(Color::Green));
        assert_eq!(parse_color("yellow"), Some(Color::Yellow));
        assert_eq!(parse_color("blue"), Some(Color::Blue));
        assert_eq!(parse_color("magenta"), Some(Color::Magenta));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("gray"), Some(Color::Gray));
        assert_eq!(parse_color("white"), Some(Color::White));
        assert_eq!(parse_color("light-red"), Some(Color::LightRed));
        assert_eq!(parse_color("light-yellow"), Some(Color::LightYellow));
        assert_eq!(parse_color("light-blue"), Some(Color::LightBlue));
        assert_eq!(parse_color("light-magenta"), Some(Color::LightMagenta));
        assert_eq!(parse_color("light-cyan"), Some(Color::LightCyan));
        assert_eq!(parse_color("reset"), Some(Color::Reset));
    }

    #[test]
    fn test_parse_color_hex() {
        assert_eq!(parse_color("#ffffff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_color("000000"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(parse_color("#123"), Some(Color::Rgb(0x11, 0x22, 0x33)));
        assert_eq!(parse_color("abc"), Some(Color::Rgb(0xaa, 0xbb, 0xcc)));
    }

    #[test]
    fn test_parse_color_indexed() {
        assert_eq!(parse_color("indexed(234)"), Some(Color::Indexed(234)));
        assert_eq!(parse_color("color(10)"), Some(Color::Indexed(10)));
    }

    #[test]
    fn test_parse_color_invalid() {
        assert_eq!(parse_color(""), None);
        assert_eq!(parse_color("not-a-color"), None);
        assert_eq!(parse_color("#12"), None);
        assert_eq!(parse_color("#12345"), None);
    }

    #[test]
    fn test_parse_border_options() {
        assert_eq!(parse_border("plain"), Some(BorderType::Plain));
        assert_eq!(parse_border("rounded"), Some(BorderType::Rounded));
        assert_eq!(parse_border("thick"), Some(BorderType::Thick));
        assert_eq!(parse_border("double"), Some(BorderType::Double));
        assert_eq!(parse_border("none"), Some(BorderType::Plain));
        assert_eq!(parse_border("unknown"), None);
    }

    #[test]
    fn test_theme_config_kebab_case() {
        let toml_str = r#"
            [ui]
            show-emojis = false
            border-type = "rounded"
        "#;
        let config: ThemeConfig = toml::from_str(toml_str).unwrap();
        let theme = Theme::from_config(config);
        assert!(!theme.show_emojis);
        assert_eq!(theme.border_type, BorderType::Rounded);
    }

    #[test]
    fn test_theme_load_valid_and_invalid() {
        let dir = tempdir().unwrap();
        let valid_path = dir.path().join("theme_valid.toml");
        let malformed_path = dir.path().join("theme_malformed.toml");
        let missing_path = dir.path().join("missing.toml");

        fs::write(
            &valid_path,
            r#"
            [palette]
            primary = "cyan"
            secondary = "gray"
            accent = "yellow"
            success = "green"
            error = "red"
            selection = "magenta"
            bg = "black"
            fg = "white"
            header-bg = "indexed(234)"
            footer-bg = "indexed(234)"

            [ui]
            show-emojis = false
            border-type = "double"
        "#,
        )
        .unwrap();

        fs::write(&malformed_path, "invalid toml syntax").unwrap();

        let theme_valid = Theme::load(&valid_path);
        assert_eq!(theme_valid.primary, Color::Cyan);
        assert!(!theme_valid.show_emojis);
        assert_eq!(theme_valid.border_type, BorderType::Double);

        let theme_malformed = Theme::load(&malformed_path);
        assert_eq!(theme_malformed, Theme::default());

        let theme_missing = Theme::load(&missing_path);
        assert_eq!(theme_missing, Theme::default());
    }
}
