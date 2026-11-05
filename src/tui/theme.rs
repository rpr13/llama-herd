use ratatui::style::Color;
use ratatui::widgets::BorderType;
use serde::Deserialize;
use std::path::Path;

/// The resolved application theme with parsed terminal colors and styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Color for primary highlights.
    pub primary: Color,
    /// Color for secondary elements.
    pub secondary: Color,
    /// Color for accents/warnings.
    pub accent: Color,
    /// Color for success indicators.
    pub success: Color,
    /// Color for error indicators.
    pub error: Color,
    /// Color for selection highlights.
    pub selection: Color,
    /// Background color.
    pub bg: Color,
    /// Foreground color.
    pub fg: Color,
    /// Background color for the headers.
    pub header_bg: Color,
    /// Background color for the footers.
    pub footer_bg: Color,
    /// Flag indicating whether to show emojis in the UI.
    pub show_emojis: bool,
    /// Border style type.
    pub border_type: BorderType,
}

/// Built-in curated theme presets and custom theme configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    /// Default Borderless Simple skin with transparent background.
    #[default]
    Default,
    /// Warm retro dark theme by Pavel Pertsev (MIT).
    GruvboxDark,
    /// Classic modern dark theme by Microsoft (MIT).
    VsCodeDark,
    /// Flagship dark theme by `JetBrains` (Apache 2.0).
    IntellijDark,
    /// Soothing pastel dark theme by Catppuccin Org (MIT).
    CatppuccinMocha,
    /// Elegant dark neon theme by enkia (MIT).
    TokyoNight,
    /// Arctic, north-bluish clean theme by Arctic Ice Studio (MIT).
    Nord,
    /// Iconic dark syntax theme from Atom (MIT).
    OneDark,
    /// User-customized theme loaded from theme.toml.
    Custom,
}

impl ThemePreset {
    /// Returns the unique machine-readable identifier for this theme preset.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::GruvboxDark => "gruvbox-dark",
            Self::VsCodeDark => "vscode-dark",
            Self::IntellijDark => "intellij-dark",
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::TokyoNight => "tokyo-night",
            Self::Nord => "nord",
            Self::OneDark => "one-dark",
            Self::Custom => "custom",
        }
    }

    /// Returns the friendly human-readable display label for TUI selectors.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Default => "Default (Borderless Simple)",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::VsCodeDark => "VS Code Dark+",
            Self::IntellijDark => "IntelliJ Dark (Darcula)",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::TokyoNight => "Tokyo Night",
            Self::Nord => "Nord",
            Self::OneDark => "One Dark",
            Self::Custom => "Custom (theme.toml)",
        }
    }

    /// Returns the open-source license under which this color scheme is distributed.
    #[must_use]
    pub const fn license(&self) -> &'static str {
        match self {
            Self::IntellijDark => "Apache 2.0",
            Self::Custom => "User Defined",
            _ => "MIT",
        }
    }

    /// Returns a concise description of the theme's palette and aesthetic.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Default => {
                "Clean minimal borderless interface with native terminal transparency and high-contrast accents."
            }
            Self::GruvboxDark => {
                "Warm retro groove palette by Pavel Pertsev featuring earthy tones, warm yellows, and distinct contrast."
            }
            Self::VsCodeDark => {
                "Official dark palette from Visual Studio Code with signature editor blues, soft grays, and balanced syntax accents."
            }
            Self::IntellijDark => {
                "Classic Darcula dark palette from JetBrains IDEs with muted teals, soft ambers, and comfortable syntax hues."
            }
            Self::CatppuccinMocha => {
                "Soothing, highly accessible dark pastel palette with rich purples, soft blues, and comforting contrast."
            }
            Self::TokyoNight => {
                "Celebrated deep blue/indigo night theme with vibrant neon blues, purples, and warm amber highlights."
            }
            Self::Nord => {
                "Iconic arctic, north-bluish palette by Arctic Ice Studio focused on clean readability and cold pastel accents."
            }
            Self::OneDark => {
                "The timeless dark syntax theme from Atom editor featuring balanced blues, purples, greens, and soft stone gray."
            }
            Self::Custom => {
                "User-defined color palette loaded directly from %APPDATA%\\llama-herd\\theme.toml."
            }
        }
    }

    /// Resolves a string identifier to a known `ThemePreset`, falling back to `Default`.
    #[must_use]
    pub fn from_id(id: &str) -> Self {
        match id.trim().to_ascii_lowercase().as_str() {
            "gruvbox-dark" | "gruvbox" => Self::GruvboxDark,
            "vscode-dark" | "vscode" | "vs-code" => Self::VsCodeDark,
            "intellij-dark" | "intellij" | "darcula" => Self::IntellijDark,
            "catppuccin-mocha" | "catppuccin" => Self::CatppuccinMocha,
            "tokyo-night" | "tokyo" => Self::TokyoNight,
            "nord" => Self::Nord,
            "one-dark" | "onedark" | "atom" => Self::OneDark,
            "custom" => Self::Custom,
            _ => Self::Default,
        }
    }

    /// Returns a slice containing all available theme presets in display order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Default,
            Self::GruvboxDark,
            Self::VsCodeDark,
            Self::IntellijDark,
            Self::CatppuccinMocha,
            Self::TokyoNight,
            Self::Nord,
            Self::OneDark,
            Self::Custom,
        ]
    }

    /// Builds a fully-resolved `Theme` struct according to this preset,
    /// with optional transparency override.
    #[must_use]
    pub fn build_theme(&self, custom_path: Option<&Path>, transparent: bool) -> Theme {
        let mut theme = match self {
            Self::Default => Theme::default(),
            Self::GruvboxDark => Theme {
                primary: Color::Rgb(0x83, 0xa5, 0x98),
                secondary: Color::Rgb(0x92, 0x83, 0x74),
                accent: Color::Rgb(0xfa, 0xbd, 0x2f),
                success: Color::Rgb(0xb8, 0xbb, 0x26),
                error: Color::Rgb(0xfb, 0x49, 0x34),
                selection: Color::Rgb(0xd3, 0x86, 0x9b),
                bg: Color::Rgb(0x28, 0x28, 0x28),
                fg: Color::Rgb(0xeb, 0xdb, 0xb2),
                header_bg: Color::Rgb(0x32, 0x30, 0x2f),
                footer_bg: Color::Rgb(0x32, 0x30, 0x2f),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::VsCodeDark => Theme {
                primary: Color::Rgb(0x00, 0x7a, 0xcc),
                secondary: Color::Rgb(0x80, 0x80, 0x80),
                accent: Color::Rgb(0xdc, 0xdc, 0xaa),
                success: Color::Rgb(0x6a, 0x99, 0x55),
                error: Color::Rgb(0xf4, 0x47, 0x47),
                selection: Color::Rgb(0xc5, 0x86, 0xc0),
                bg: Color::Rgb(0x1e, 0x1e, 0x1e),
                fg: Color::Rgb(0xd4, 0xd4, 0xd4),
                header_bg: Color::Rgb(0x25, 0x25, 0x26),
                footer_bg: Color::Rgb(0x25, 0x25, 0x26),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::IntellijDark => Theme {
                primary: Color::Rgb(0x29, 0x99, 0x99),
                secondary: Color::Rgb(0x80, 0x80, 0x80),
                accent: Color::Rgb(0xbb, 0xb5, 0x29),
                success: Color::Rgb(0x62, 0x97, 0x55),
                error: Color::Rgb(0xff, 0x6b, 0x68),
                selection: Color::Rgb(0x98, 0x76, 0xaa),
                bg: Color::Rgb(0x2b, 0x2b, 0x2b),
                fg: Color::Rgb(0xa9, 0xb7, 0xc6),
                header_bg: Color::Rgb(0x31, 0x33, 0x35),
                footer_bg: Color::Rgb(0x31, 0x33, 0x35),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::CatppuccinMocha => Theme {
                primary: Color::Rgb(0x89, 0xb4, 0xfa),
                secondary: Color::Rgb(0xa6, 0xad, 0xc8),
                accent: Color::Rgb(0xf9, 0xe2, 0xaf),
                success: Color::Rgb(0xa6, 0xe3, 0xa1),
                error: Color::Rgb(0xf3, 0x8b, 0xa8),
                selection: Color::Rgb(0xcb, 0xa6, 0xf7),
                bg: Color::Rgb(0x1e, 0x1e, 0x2e),
                fg: Color::Rgb(0xcd, 0xd6, 0xf4),
                header_bg: Color::Rgb(0x18, 0x18, 0x25),
                footer_bg: Color::Rgb(0x18, 0x18, 0x25),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::TokyoNight => Theme {
                primary: Color::Rgb(0x7a, 0xa2, 0xf7),
                secondary: Color::Rgb(0x56, 0x5f, 0x89),
                accent: Color::Rgb(0xe0, 0xaf, 0x68),
                success: Color::Rgb(0x9e, 0xce, 0x6a),
                error: Color::Rgb(0xf7, 0x76, 0x8e),
                selection: Color::Rgb(0xbb, 0x9a, 0xf7),
                bg: Color::Rgb(0x1a, 0x1b, 0x26),
                fg: Color::Rgb(0xc0, 0xca, 0xf5),
                header_bg: Color::Rgb(0x16, 0x16, 0x1e),
                footer_bg: Color::Rgb(0x16, 0x16, 0x1e),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::Nord => Theme {
                primary: Color::Rgb(0x88, 0xc0, 0xd0),
                secondary: Color::Rgb(0x4c, 0x56, 0x6a),
                accent: Color::Rgb(0xeb, 0xcb, 0x8b),
                success: Color::Rgb(0xa3, 0xbe, 0x8c),
                error: Color::Rgb(0xbf, 0x61, 0x6a),
                selection: Color::Rgb(0xb4, 0x8e, 0xad),
                bg: Color::Rgb(0x2e, 0x34, 0x40),
                fg: Color::Rgb(0xec, 0xef, 0xf4),
                header_bg: Color::Rgb(0x24, 0x29, 0x33),
                footer_bg: Color::Rgb(0x24, 0x29, 0x33),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::OneDark => Theme {
                primary: Color::Rgb(0x61, 0xaf, 0xef),
                secondary: Color::Rgb(0x5c, 0x63, 0x70),
                accent: Color::Rgb(0xe5, 0xc0, 0x7b),
                success: Color::Rgb(0x98, 0xc3, 0x79),
                error: Color::Rgb(0xe0, 0x6c, 0x75),
                selection: Color::Rgb(0xc6, 0x78, 0xdd),
                bg: Color::Rgb(0x28, 0x2c, 0x34),
                fg: Color::Rgb(0xab, 0xb2, 0xbf),
                header_bg: Color::Rgb(0x21, 0x25, 0x2b),
                footer_bg: Color::Rgb(0x21, 0x25, 0x2b),
                show_emojis: true,
                border_type: BorderType::Plain,
            },
            Self::Custom => {
                if let Some(path) = custom_path {
                    Theme::load(path)
                } else {
                    Theme::default()
                }
            }
        };

        if transparent {
            theme.bg = Color::Reset;
            theme.header_bg = Color::Reset;
            theme.footer_bg = Color::Reset;
        }

        theme
    }
}

/// Parsed raw configuration for styling the TUI application.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ThemeConfig {
    /// Optional color palette configuration.
    pub palette: Option<PaletteConfig>,
    /// Optional UI layout config (emojis, borders).
    pub ui: Option<UiConfig>,
}

/// Color palette options defined in the theme TOML file.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct PaletteConfig {
    /// Primary highlight color string.
    pub primary: Option<String>,
    /// Secondary elements color string.
    pub secondary: Option<String>,
    /// Accent color string.
    pub accent: Option<String>,
    /// Success indicator color string.
    pub success: Option<String>,
    /// Error indicator color string.
    pub error: Option<String>,
    /// Selection highlight color string.
    pub selection: Option<String>,
    /// Background color string.
    pub bg: Option<String>,
    /// Foreground color string.
    pub fg: Option<String>,
    /// Header panel background color string.
    pub header_bg: Option<String>,
    /// Footer panel background color string.
    pub footer_bg: Option<String>,
}

/// General UI aesthetic preferences defined in the theme TOML file.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct UiConfig {
    /// Flag indicating whether emojis should be displayed.
    pub show_emojis: Option<bool>,
    /// Border type name.
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
            bg: Color::Reset,
            fg: Color::White,
            header_bg: Color::Reset,
            footer_bg: Color::Reset,
            show_emojis: true,
            border_type: BorderType::Plain,
        }
    }
}

impl Theme {
    /// Loads a theme TOML file, falling back to default theme on error.
    #[must_use]
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

    /// Converts parsed raw configuration into resolved terminal colors and styles.
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
        "reset" | "default" | "none" | "transparent" => Some(Color::Reset),
        _ => {
            if (s.starts_with("indexed(") || s.starts_with("color(")) && s.ends_with(')') {
                let start = if s.starts_with("indexed(") { 8 } else { 6 };
                let num_str = &s[start..s.len() - 1];
                if let Ok(n) = num_str.parse::<u8>() {
                    return Some(Color::Indexed(n));
                }
            }

            if s.starts_with("rgb(") && s.ends_with(')') {
                let inner = &s[4..s.len() - 1];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() == 3 {
                    let r = parts[0].trim().parse::<u8>().ok()?;
                    let g = parts[1].trim().parse::<u8>().ok()?;
                    let b = parts[2].trim().parse::<u8>().ok()?;
                    return Some(Color::Rgb(r, g, b));
                }
            }

            let hex = s.strip_prefix('#').unwrap_or(s);
            if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
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
                    eprintln!("Warning: Unknown color format '{s}'");
                    None
                }
            }
        }
    }
}

fn parse_border(s: &str) -> Option<BorderType> {
    match s.to_lowercase().as_str() {
        "plain" | "none" => Some(BorderType::Plain),
        "rounded" => Some(BorderType::Rounded),
        "thick" => Some(BorderType::Thick),
        "double" => Some(BorderType::Double),
        _ => {
            eprintln!("Warning: Unknown border type '{s}'");
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
        assert_eq!(parse_color("default"), Some(Color::Reset));
        assert_eq!(parse_color("none"), Some(Color::Reset));
        assert_eq!(parse_color("transparent"), Some(Color::Reset));
    }

    #[test]
    fn test_parse_color_rgb() {
        assert_eq!(parse_color("rgb(10, 20, 30)"), Some(Color::Rgb(10, 20, 30)));
        assert_eq!(parse_color("rgb(255,0,128)"), Some(Color::Rgb(255, 0, 128)));
        assert_eq!(parse_color("rgb(256, 0, 0)"), None);
        assert_eq!(parse_color("rgb(1, 2)"), None);
    }

    #[test]
    fn test_theme_default_transparency() {
        let theme = Theme::default();
        assert_eq!(theme.bg, Color::Reset);
        assert_eq!(theme.header_bg, Color::Reset);
        assert_eq!(theme.footer_bg, Color::Reset);
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
        let config: ThemeConfig = toml::from_str(toml_str).expect("Failed to parse valid TOML");
        let theme = Theme::from_config(config);
        assert!(!theme.show_emojis);
        assert_eq!(theme.border_type, BorderType::Rounded);
    }

    #[test]
    fn test_theme_load_valid_and_invalid() {
        let dir = tempdir().expect("Failed to create temp directory");
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
        .expect("Failed to write valid theme file");

        fs::write(&malformed_path, "invalid toml syntax")
            .expect("Failed to write malformed theme file");

        let theme_valid = Theme::load(&valid_path);
        assert_eq!(theme_valid.primary, Color::Cyan);
        assert!(!theme_valid.show_emojis);
        assert_eq!(theme_valid.border_type, BorderType::Double);

        let theme_malformed = Theme::load(&malformed_path);
        assert_eq!(theme_malformed, Theme::default());

        let theme_missing = Theme::load(&missing_path);
        assert_eq!(theme_missing, Theme::default());
    }

    #[test]
    fn test_theme_presets_serialization_and_fallback() {
        assert_eq!(
            ThemePreset::from_id("gruvbox-dark"),
            ThemePreset::GruvboxDark
        );
        assert_eq!(ThemePreset::from_id("vscode-dark"), ThemePreset::VsCodeDark);
        assert_eq!(
            ThemePreset::from_id("intellij-dark"),
            ThemePreset::IntellijDark
        );
        assert_eq!(
            ThemePreset::from_id("catppuccin-mocha"),
            ThemePreset::CatppuccinMocha
        );
        assert_eq!(ThemePreset::from_id("tokyo-night"), ThemePreset::TokyoNight);
        assert_eq!(ThemePreset::from_id("nord"), ThemePreset::Nord);
        assert_eq!(ThemePreset::from_id("one-dark"), ThemePreset::OneDark);
        assert_eq!(ThemePreset::from_id("custom"), ThemePreset::Custom);
        assert_eq!(ThemePreset::from_id("invalid-xyz"), ThemePreset::Default);
    }

    #[test]
    fn test_theme_preset_build_theme_opaque_and_transparent() {
        let gruvbox_opaque = ThemePreset::GruvboxDark.build_theme(None, false);
        assert_ne!(gruvbox_opaque.bg, Color::Reset);
        assert_eq!(gruvbox_opaque.bg, Color::Rgb(0x28, 0x28, 0x28));

        let gruvbox_trans = ThemePreset::GruvboxDark.build_theme(None, true);
        assert_eq!(gruvbox_trans.bg, Color::Reset);
        assert_eq!(gruvbox_trans.header_bg, Color::Reset);
        assert_eq!(gruvbox_trans.footer_bg, Color::Reset);
        assert_eq!(gruvbox_trans.primary, gruvbox_opaque.primary);
    }

    #[test]
    fn test_live_theme_switch_and_transparency_toggle() {
        let mut theme = ThemePreset::GruvboxDark.build_theme(None, false);
        assert_ne!(theme.bg, Color::Reset);

        // Switch to transparent
        theme = ThemePreset::GruvboxDark.build_theme(None, true);
        assert_eq!(theme.bg, Color::Reset);

        // Switch to VS Code Dark
        theme = ThemePreset::VsCodeDark.build_theme(None, false);
        assert_eq!(theme.bg, Color::Rgb(0x1e, 0x1e, 0x1e));
    }
}
