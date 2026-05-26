pub mod tui {
    pub mod app;
    pub mod logs;
    pub mod picker;
    pub mod ui;
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use llama_herd::tui::theme::Theme;
    use llama_herd::tui::{AppScreen, AppState, TuiEvent, handle_key_event};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_handle_key_event_quit() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        let key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        assert!(handle_key_event(&mut state, key, &tx));
    }

    #[test]
    fn test_ui_header_displays_version() {
        use ratatui::{Terminal, backend::TestBackend};
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        terminal
            .draw(|f| {
                llama_herd::tui::ui::draw(f, &mut state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut row_str = String::new();
        for x in 0..80 {
            row_str.push(buffer[(x, 0)].symbol().chars().next().unwrap_or(' '));
        }
        let expected_version = env!("APP_VERSION");
        assert!(
            row_str.contains(expected_version),
            "Row 0 string '{}' did not contain version '{}'",
            row_str,
            expected_version
        );
    }

    #[test]
    fn test_ui_header_narrow_screen() {
        use ratatui::{Terminal, backend::TestBackend};
        let backend = TestBackend::new(30, 24); // Very narrow screen
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        terminal
            .draw(|f| {
                llama_herd::tui::ui::draw(f, &mut state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut row_str = String::new();
        for x in 0..30 {
            row_str.push(buffer[(x, 0)].symbol().chars().next().unwrap_or(' '));
        }

        // In narrow mode (< 60), logo should be just "🦙"
        assert!(row_str.contains('🦙'));
        assert!(!row_str.contains("LlamaHerd"));

        // In narrow mode (< 45), version should be hidden
        let expected_version = env!("APP_VERSION");
        assert!(!row_str.contains(expected_version));
    }

    #[test]
    fn test_ui_dashboard_responsive_layout() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut state = AppState::new(
            vec![("test".to_string(), PathBuf::from("test.gguf"))],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.screen = AppScreen::Dashboard;

        // 1. Wide screen (>= 110)
        {
            let backend = TestBackend::new(120, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    llama_herd::tui::ui::draw(f, &mut state);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();

            // Check footer for "Launch Preset" (full text)
            let mut footer_str = String::new();
            for x in 0..120 {
                footer_str.push(buffer[(x, 28)].symbol().chars().next().unwrap_or(' '));
            }
            assert!(footer_str.contains("Launch Preset"));
        }

        // 2. Narrow screen (< 110)
        {
            let backend = TestBackend::new(100, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    llama_herd::tui::ui::draw(f, &mut state);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();

            // Check footer for "Launch" (compact text)
            let mut footer_str = String::new();
            for x in 0..100 {
                footer_str.push(buffer[(x, 28)].symbol().chars().next().unwrap_or(' '));
            }
            assert!(footer_str.contains("Launch"));
            assert!(!footer_str.contains("Launch Preset"));
        }
    }

    #[test]
    fn test_handle_key_event_edit_ctx() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.ctx = 123;
        let key = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        handle_key_event(&mut state, key, &tx);
        assert_eq!(state.screen, AppScreen::EditingCtx);
        assert_eq!(state.input_buffer, "123");
    }

    #[test]
    fn test_handle_key_event_toggle_ui() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.ui = true;
        let key = KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        handle_key_event(&mut state, key, &tx);
        assert!(!state.ui);
        handle_key_event(&mut state, key, &tx);
        assert!(state.ui);
    }

    #[test]
    fn test_handle_key_event_editing_flow() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.screen = AppScreen::EditingNgl;
        state.input_buffer = "auto".to_string();

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // Type '1'
        let key_1 = KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_1, &tx);
        assert_eq!(state.input_buffer, "auto1");

        // Backspace
        let key_bs = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_bs, &tx);
        assert_eq!(state.input_buffer, "auto");

        // Enter
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.ngl, "auto");
    }
}
