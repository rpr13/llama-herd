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
    use llama_herd::tui::{AppScreen, AppState, DashboardFocus, TuiEvent, handle_key_event};
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

    #[test]
    fn test_handle_key_event_selecting_option() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.config_path = config_path;
        state.screen = AppScreen::Settings;
        let flash_attn_idx = llama_herd::tui::ui::SETTINGS
            .iter()
            .position(|item| item.key == "flash-attn")
            .unwrap();
        state.settings_index = flash_attn_idx;

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Enter key -> opens option list popup
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::SelectingGlobalSettingOption);
        assert_eq!(state.option_selector_list.len(), 4);
        assert_eq!(state.option_selector_list[0], "auto");
        assert_eq!(state.option_selector_list[1], "1");

        // 2. Down key -> moves selector to the next item
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.option_selector_index, 1); // Selected "1"

        // 3. Enter key -> saves standard option and returns to Settings
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Settings);
        assert_eq!(
            state
                .global_config
                .get("flash-attn")
                .unwrap()
                .as_str()
                .unwrap(),
            "1"
        );

        // 4. Open option list again
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::SelectingGlobalSettingOption);

        // 5. Select "(Custom / Manual...)" (which is the last item: index 3)
        state.option_selector_index = 3;

        // 6. Enter key -> transitions to text entry
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingGlobalSetting);
        assert_eq!(state.input_buffer, "1");
    }

    #[test]
    fn test_handle_key_event_selecting_mmproj() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.mmproj_list = vec![
            None,
            Some(PathBuf::from("mmproj-1.gguf")),
            Some(PathBuf::from("mmproj-2.gguf")),
        ];
        state.mmproj_index = 0;
        state.screen = AppScreen::Dashboard;

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Press 'v' -> enters MMProj selection popup
        let key_v = KeyEvent {
            code: KeyCode::Char('v'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_v, &tx);
        assert_eq!(state.screen, AppScreen::SelectingMMProj);

        // 2. Down key -> moves selection to index 1
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.mmproj_index, 1);

        // 3. Enter key -> saves & exits selection popup
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.mmproj_index, 1);

        // 4. Press 'v' again -> enters selection popup
        handle_key_event(&mut state, key_v, &tx);
        assert_eq!(state.screen, AppScreen::SelectingMMProj);
        assert_eq!(state.mmproj_index_backup, 1);

        // 5. Down key -> moves selection to index 2
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.mmproj_index, 2);

        // 6. Esc key -> cancels and resets to index 1
        let key_esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_esc, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.mmproj_index, 1);
    }

    #[test]
    fn test_handle_key_event_selecting_draft_model() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.draft_list = vec![None, Some(PathBuf::from("draft-1.gguf"))];
        state.draft_index = 0;
        state.draft_ngl = "".to_string();
        state.screen = AppScreen::Dashboard;

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Press 'd' -> enters Draft model selection popup
        let key_d = KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_d, &tx);
        assert_eq!(state.screen, AppScreen::SelectingDraftModel);

        // 2. Down key -> moves selection to index 1
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.draft_index, 1);

        // 3. Enter key -> saves & exits, sets draft_ngl to "auto" since it is empty
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.draft_index, 1);
        assert_eq!(state.draft_ngl, "auto");

        // 4. Press 'd' again -> enters selection popup
        handle_key_event(&mut state, key_d, &tx);
        assert_eq!(state.screen, AppScreen::SelectingDraftModel);
        assert_eq!(state.draft_index_backup, 1);

        // 5. Down key -> cycles selection back to index 0
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.draft_index, 0);

        // 6. Esc key -> cancels and resets to index 1
        let key_esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_esc, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.draft_index, 1);
    }

    #[test]
    fn test_model_config_loading_and_saving() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");

        // Create a fake model file
        let model_gguf = models_dir.join("test-model-7b.gguf");
        std::fs::write(&model_gguf, b"").unwrap();

        // Create a matching TOML config file
        let model_toml = models_dir.join("test-model-7b.toml");
        let toml_content = r#"
[llama-herd]
total-layers = 32
draft = "test-draft.gguf"

[llama-server-long]
ctx-size = 4096
ngl = "auto"
temp = 0.7
"#;
        std::fs::write(&model_toml, toml_content.as_bytes()).unwrap();

        // Generate the preset INI file first
        let _ =
            llama_herd::discovery::generate_presets_ini(&models_dir, &preset_path, &HashMap::new());

        let presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);
        assert_eq!(presets.len(), 2);

        let mut state = AppState::new(
            presets,
            models_dir.clone(),
            preset_path.clone(),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        // 1. Verify config is loaded automatically on AppState creation/preset selection
        assert_eq!(state.config_file_name, "test-model-7b.toml");
        assert_eq!(state.ctx_str, "4096");
        assert_eq!(state.ngl, "32");
        assert_eq!(state.temp, "0.7");
        assert_eq!(state.total_layers, Some(32));

        // 2. Modify values in State (simulating the inline editing inputs)
        state.temp = "0.9".to_string();
        state.ctx_str = "8192".to_string();
        state.top_p = "0.99".to_string();
        state.config_file_name = "shared-prefix-config.toml".to_string();

        // 3. Save config (simulating Enter on ConfirmSaveConfig screen with backup disabled)
        state.save_current_preset_config(false).unwrap();

        // Verify new TOML file is created
        let new_toml = models_dir.join("shared-prefix-config.toml");
        assert!(new_toml.exists());

        // Load new TOML file and check content
        let new_config = llama_herd::config::load_toml_silent(&new_toml);
        let long_opts = new_config
            .get("llama-server-long")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(long_opts.get("temp").unwrap().as_f64().unwrap(), 0.9);
        assert_eq!(long_opts.get("ctx-size").unwrap().as_i64().unwrap(), 8192);
        assert_eq!(long_opts.get("top-p").unwrap().as_f64().unwrap(), 0.99);

        // 4. Modify temp again and save with backup enabled to test backup generation
        state.temp = "0.95".to_string();
        state.save_current_preset_config(true).unwrap();

        // Verify that a backup file with suffix .bak.<timestamp> was created
        let backup_files: Vec<_> = std::fs::read_dir(&models_dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("shared-prefix-config.toml.bak.")
            })
            .collect();
        assert!(!backup_files.is_empty(), "Backup file should be created!");

        // Verify presets were regenerated
        let new_presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);
        assert_eq!(new_presets.len(), 2);
    }

    #[test]
    fn test_resolve_toml_path_prefix_matching() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();

        let model_gguf = models_dir.join("my-awesome-model-13b-q5_k_m.gguf");
        std::fs::write(&model_gguf, b"").unwrap();

        // 1. If no TOML exist, fallback path should be exact matching GGUF stem name with .toml extension
        let path = llama_herd::config::resolve_toml_path(&model_gguf, &models_dir);
        assert_eq!(path, models_dir.join("my-awesome-model-13b-q5_k_m.toml"));

        // 2. If a prefix matching TOML exists, use it
        let shared_toml = models_dir.join("my-awesome-model-13b.toml");
        std::fs::write(&shared_toml, b"").unwrap();

        let path = llama_herd::config::resolve_toml_path(&model_gguf, &models_dir);
        assert_eq!(path, shared_toml);

        // 3. If a more specific exact matching TOML exists, use it instead of prefix
        let exact_toml = models_dir.join("my-awesome-model-13b-q5_k_m.toml");
        std::fs::write(&exact_toml, b"").unwrap();

        let path = llama_herd::config::resolve_toml_path(&model_gguf, &models_dir);
        assert_eq!(path, exact_toml);
    }

    #[test]
    fn test_dashboard_tab_toggle_and_shortcuts() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        assert_eq!(state.dashboard_focus, DashboardFocus::Left);
        assert_eq!(state.dashboard_param_index, 0);

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Tab should toggle to right panel focus
        let key_tab = KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_tab, &tx);
        assert_eq!(state.dashboard_focus, DashboardFocus::Right);

        // 2. Down key should increment parameter index when focused on right panel
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.dashboard_param_index, 1);

        // 3. Tab should toggle back to left panel focus
        handle_key_event(&mut state, key_tab, &tx);
        assert_eq!(state.dashboard_focus, DashboardFocus::Left);

        // 4. Numeric key '2' should switch active tab to Settings
        let key_2 = KeyEvent {
            code: KeyCode::Char('2'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_2, &tx);
        assert_eq!(state.active_tab, 1);
        assert_eq!(state.screen, AppScreen::Settings);
    }

    #[test]
    fn test_config_filename_editing_suggestions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");

        // GGUF model
        let model_gguf = models_dir.join("my-model-7b-q4_0.gguf");
        std::fs::write(&model_gguf, b"").unwrap();

        // Similar TOMLs
        let toml1 = models_dir.join("my-model-7b.toml");
        std::fs::write(&toml1, b"").unwrap();
        let toml2 = models_dir.join("my-model-7b-q4_0.toml");
        std::fs::write(&toml2, b"").unwrap();

        let _ =
            llama_herd::discovery::generate_presets_ini(&models_dir, &preset_path, &HashMap::new());
        let presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);

        let mut state = AppState::new(
            presets,
            models_dir.clone(),
            preset_path.clone(),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // Trigger config filename edit screen
        let key_f = KeyEvent {
            code: KeyCode::Char('f'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_f, &tx);

        assert_eq!(state.screen, AppScreen::EditingConfigFileName);
        // Similar config files should contain our two TOMLs
        assert_eq!(state.similar_config_files.len(), 2);
        assert!(
            state
                .similar_config_files
                .contains(&"my-model-7b.toml".to_string())
        );
        assert!(
            state
                .similar_config_files
                .contains(&"my-model-7b-q4_0.toml".to_string())
        );

        // Press Down to cycle selection
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);

        assert!(state.similar_config_index.is_some());
        assert!(!state.input_buffer.is_empty());
    }

    #[test]
    fn test_unsaved_changes_preset_change_warning() {
        let mut state = AppState::new(
            vec![
                ("model-1".to_string(), PathBuf::from("model-1.gguf")),
                ("model-2".to_string(), PathBuf::from("model-2.gguf")),
            ],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        assert_eq!(state.preset_index, 0);
        assert_eq!(state.dashboard_focus, DashboardFocus::Left);

        // Make parameter change (dirty state)
        state.temp = "0.95".to_string();
        assert!(state.has_unsaved_changes());

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // Trigger preset selection change Down
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);

        // Verify warning screen is active and preset index has not changed yet
        assert_eq!(state.screen, AppScreen::WarnDiscardChanges);
        assert_eq!(state.preset_index, 0);
        assert_eq!(state.pending_preset_index, Some(1));

        // Press 'n' to cancel switching
        let key_n = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_n, &tx);

        // Verify back to dashboard, preset index still 0
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.preset_index, 0);
        assert_eq!(state.pending_preset_index, None);

        // Trigger down again
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.screen, AppScreen::WarnDiscardChanges);

        // Confirm switch using Enter
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);

        // Verify preset index successfully switched to 1
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.preset_index, 1);
        assert_eq!(state.pending_preset_index, None);
    }

    #[test]
    fn test_get_models_dir_state_and_stability() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();

        // Initially empty
        let state1 = llama_herd::tui::app::get_models_dir_state(&models_dir).unwrap();
        assert!(state1.files.is_empty());

        // Create a model file
        let gguf_path = models_dir.join("model1.gguf");
        std::fs::write(&gguf_path, b"hello").unwrap();

        let state2 = llama_herd::tui::app::get_models_dir_state(&models_dir).unwrap();
        assert_eq!(state2.files.len(), 1);
        assert_eq!(state2.files[0].0, gguf_path);
        assert_eq!(state2.files[0].2, 5); // size
    }

    #[test]
    fn test_check_models_dir_changes_invalidation() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("/non/existent/path/for/sure"),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        state.check_models_dir_changes();
        assert!(state.models_dir_invalid);
    }

    #[test]
    fn test_check_models_dir_changes_dirty_state() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");

        let gguf_path = models_dir.join("model1.gguf");
        std::fs::write(&gguf_path, b"test").unwrap();

        let _ =
            llama_herd::discovery::generate_presets_ini(&models_dir, &preset_path, &HashMap::new());
        let presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);

        let mut state = AppState::new(
            presets,
            models_dir.clone(),
            preset_path,
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        // Make state dirty
        state.temp = "0.95".to_string();
        assert!(state.has_unsaved_changes());

        // Introduce a new file and simulate ticks to settle
        let gguf_path2 = models_dir.join("model2.gguf");
        std::fs::write(&gguf_path2, b"test").unwrap();

        // First check sees the new file, directory is not stable (new file appearing)
        state.check_models_dir_changes();
        assert!(!state.models_dir_changed_dirty);

        // Second check sees file is now stable
        state.check_models_dir_changes();
        assert!(state.models_dir_changed_dirty);

        // Revert dirty state manually
        state.temp = "".to_string();
        state.check_models_dir_changes();
        assert!(!state.models_dir_changed_dirty);
    }

    #[test]
    fn test_handle_key_event_selecting_log_verbosity() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.config_path = config_path;
        state.screen = AppScreen::Settings;
        // Find index of log-verbosity in SETTINGS
        let log_verbosity_idx = llama_herd::tui::ui::SETTINGS
            .iter()
            .position(|item| item.key == "log-verbosity")
            .unwrap();
        state.settings_index = log_verbosity_idx;

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Enter key -> opens option list popup
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::SelectingGlobalSettingOption);
        assert_eq!(state.option_selector_list.len(), 7);
        assert_eq!(state.option_selector_list[0], "0");
        assert_eq!(state.option_selector_list[5], "5");

        // 2. Down key -> moves selector to the next item
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.option_selector_index, 4); // Selected "4" (since default is "3" at index 3)

        // 3. Enter key -> saves standard option as a Number (4) and returns to Settings
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Settings);
        assert_eq!(
            state
                .global_config
                .get("log-verbosity")
                .unwrap()
                .as_i64()
                .unwrap(),
            4
        );
    }

    #[test]
    fn test_mask_sensitive_args() {
        use llama_herd::tui::ui::mask_sensitive_args;

        // 1. Without sensitive args
        let args = vec![
            "llama-server".to_string(),
            "--model".to_string(),
            "model.gguf".to_string(),
        ];
        assert_eq!(
            mask_sensitive_args(&args),
            "llama-server --model model.gguf"
        );

        // 2. With api-key arg
        let args_with_key = vec![
            "llama-server".to_string(),
            "--api-key".to_string(),
            "secret-12345".to_string(),
            "--model".to_string(),
            "model.gguf".to_string(),
        ];
        assert_eq!(
            mask_sensitive_args(&args_with_key),
            "llama-server --api-key [MASKED] --model model.gguf"
        );

        // 3. Edgecase: api-key at the end of the argument list without a value
        let args_edge = vec!["llama-server".to_string(), "--api-key".to_string()];
        assert_eq!(mask_sensitive_args(&args_edge), "llama-server --api-key");
    }
}
