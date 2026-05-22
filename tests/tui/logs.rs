use llama_herd::tui::logs::{apply_sgr, parse_ansi_line};
use ratatui::style::{Color, Modifier, Style};

#[test]
fn test_apply_sgr_bold() {
    let style = apply_sgr("1", Style::default());
    assert_eq!(style, Style::default().add_modifier(Modifier::BOLD));
}

#[test]
fn test_apply_sgr_fg() {
    let style = apply_sgr("31", Style::default());
    assert_eq!(style, Style::default().fg(Color::Red));
}

#[test]
fn test_apply_sgr_bg() {
    let style = apply_sgr("44", Style::default());
    assert_eq!(style, Style::default().bg(Color::Blue));
}

#[test]
fn test_apply_sgr_mixed() {
    let style = apply_sgr("1;32", Style::default());
    assert_eq!(
        style,
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Green)
    );
}

#[test]
fn test_apply_sgr_256_color() {
    let style_fg = apply_sgr("38;5;123", Style::default());
    assert_eq!(style_fg, Style::default().fg(Color::Indexed(123)));

    let style_bg = apply_sgr("48;5;201", Style::default());
    assert_eq!(style_bg, Style::default().bg(Color::Indexed(201)));
}

#[test]
fn test_parse_ansi_line_colored() {
    // "BoldRed" in bold and red, then "Normal"
    let input = "\x1b[1;31mBoldRed\x1b[0mNormal";
    let log_line = parse_ansi_line(input);

    assert_eq!(log_line.spans.len(), 2);

    assert_eq!(log_line.spans[0].text, "BoldRed");
    assert_eq!(
        log_line.spans[0].style,
        Style::default().add_modifier(Modifier::BOLD).fg(Color::Red)
    );

    assert_eq!(log_line.spans[1].text, "Normal");
    assert_eq!(log_line.spans[1].style, Style::default());
}

#[test]
fn test_parse_ansi_line_empty() {
    let log_line = parse_ansi_line("");
    assert_eq!(log_line.spans.len(), 1);
    assert_eq!(log_line.spans[0].text, "");
    assert_eq!(log_line.spans[0].style, Style::default());
}
