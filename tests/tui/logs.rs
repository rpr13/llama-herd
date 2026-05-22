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

#[test]
fn test_active_server_pid_termination() {
    use llama_herd::tui::logs::ActiveServer;
    use std::path::Path;

    // Spawn a dummy process that runs for a few seconds
    let params = if cfg!(target_os = "windows") {
        vec![
            "ping".to_string(),
            "127.0.0.1".to_string(),
            "-n".to_string(),
            "10".to_string(),
        ]
    } else {
        vec!["sleep".to_string(), "10".to_string()]
    };

    let mut server = ActiveServer::spawn(&params, Path::new("."), None).unwrap();

    // Verify it started running
    assert!(*server.is_running.lock().unwrap());

    // Terminate it via PID-based kill method
    server.kill();

    // Verify the child process is terminated (reaped exit status is some)
    let exit_status = server.child.try_wait().unwrap();
    assert!(
        exit_status.is_some(),
        "Spawning child process should be terminated after kill()"
    );
}

#[test]
fn test_active_server_ring_buffer_capacity() {
    use llama_herd::tui::logs::{ActiveServer, MAX_LOGS};
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    let params = if cfg!(target_os = "windows") {
        vec![
            "powershell.exe".to_string(),
            "-Command".to_string(),
            "1..6000 | ForEach-Object { Write-Output $_ }".to_string(),
        ]
    } else {
        vec!["seq".to_string(), "1".to_string(), "6000".to_string()]
    };

    let mut server = ActiveServer::spawn(&params, Path::new("."), None).unwrap();

    // Wait for the process to exit
    let status = server.child.wait().unwrap();
    assert!(status.success());

    // Give reader threads a moment to finish collecting the last lines
    thread::sleep(Duration::from_millis(500));

    // Get log counts
    let logs_count = server.logs.lock().unwrap().len();
    let history_count = server.raw_history.lock().unwrap().len();

    // Verify it is capped to MAX_LOGS
    assert_eq!(logs_count, MAX_LOGS);
    assert_eq!(history_count, MAX_LOGS);
}
