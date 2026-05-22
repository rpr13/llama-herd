pub mod tui {
    pub mod app;
    pub mod logs;
    pub mod ui;
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use llama_herd::tui::{AppScreen, AppState, handle_key_event};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_handle_key_event_quit() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
        );
        let key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert!(handle_key_event(&mut state, key));
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
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
        );
        terminal
            .draw(|f| {
                llama_herd::tui::ui::draw(f, &mut state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut row_str = String::new();
        for x in 0..80 {
            row_str.push(buffer[(x, 1)].symbol().chars().next().unwrap_or(' '));
        }
        let expected_version = env!("CARGO_PKG_VERSION");
        assert!(
            row_str.contains(&format!("v{}", expected_version)),
            "Row 1 string '{}' did not contain version 'v{}'",
            row_str,
            expected_version
        );
    }

    #[test]
    fn test_handle_key_event_edit_ctx() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
        );
        state.ctx = 123;
        let key = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key);
        assert_eq!(state.screen, AppScreen::EditingCtx);
        assert_eq!(state.input_buffer, "123");
    }

    #[test]
    fn test_handle_key_event_toggle_ui() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
        );
        state.ui = true;
        let key = KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key);
        assert!(!state.ui);
        handle_key_event(&mut state, key);
        assert!(state.ui);
    }

    #[test]
    fn test_handle_key_event_editing_flow() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
        );
        state.screen = AppScreen::EditingNgl;
        state.input_buffer = "auto".to_string();

        // Type '1'
        let key_1 = KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_1);
        assert_eq!(state.input_buffer, "auto1");

        // Backspace
        let key_bs = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_bs);
        assert_eq!(state.input_buffer, "auto");

        // Enter
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter);
        assert_eq!(state.screen, AppScreen::Select);
        assert_eq!(state.ngl, "auto");
    }
}
