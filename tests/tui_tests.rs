#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

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
            vec![("test".to_owned(), PathBuf::from("test.gguf"))],
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
        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 4; // Context Size
        let key = KeyEvent {
            code: KeyCode::Enter,
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
        state.input_buffer = "auto".to_owned();

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
    fn test_handle_key_event_edit_spec_draft_params() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.spec_draft_n_max = "4".to_owned();
        state.spec_draft_p_min = "0.0".to_owned();

        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 24; // Spec Draft N Max

        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // Go to spec-draft-n-max edit screen
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingSpecDraftNMax);
        assert_eq!(state.input_buffer, "4");

        // Save new val
        state.input_buffer = "8".to_owned();
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.spec_draft_n_max, "8");

        // Go to spec-draft-p-min edit screen (index 25)
        state.dashboard_param_index = 25;
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingSpecDraftPMin);
        assert_eq!(state.input_buffer, "0.0");

        // Save new val
        state.input_buffer = "0.85".to_owned();
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.spec_draft_p_min, "0.85");
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
    fn test_handle_key_event_editing_global_settings_manual() {
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
        state.screen = AppScreen::EditingGlobalSetting;

        let dry_mult_idx = llama_herd::tui::ui::SETTINGS
            .iter()
            .position(|item| item.key == "dry-multiplier")
            .unwrap();
        state.settings_index = dry_mult_idx;
        state.input_buffer = "1.5".to_owned();

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Settings);

        let saved_val = state
            .global_config
            .get("dry-multiplier")
            .and_then(|v| v.as_f64())
            .unwrap();
        assert_eq!(saved_val, 1.5);
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

        // 1. Enter key -> enters MMProj selection popup
        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 6; // MMProj
        let key_v = KeyEvent {
            code: KeyCode::Enter,
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
        state.draft_ngl = "".to_owned();
        state.screen = AppScreen::Dashboard;

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Enter key -> enters Draft model selection popup
        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 21; // Draft Model
        let key_d = KeyEvent {
            code: KeyCode::Enter,
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
        assert_eq!(presets.len(), 1);

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
        state.temp = "0.9".to_owned();
        state.ctx_str = "8192".to_owned();
        state.top_p = "0.99".to_owned();
        state.config_file_name = "shared-prefix-config.toml".to_owned();

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
        state.temp = "0.95".to_owned();
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
        assert_eq!(new_presets.len(), 1);
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

        // 4. F2 key should switch active tab to Settings
        let key_f2 = KeyEvent {
            code: KeyCode::F(2),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_f2, &tx);
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

        // Trigger config filename edit screen (index 1 on right pane)
        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 1; // Target Config File
        let key_f = KeyEvent {
            code: KeyCode::Enter,
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
                .contains(&"my-model-7b.toml".to_owned())
        );
        assert!(
            state
                .similar_config_files
                .contains(&"my-model-7b-q4_0.toml".to_owned())
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
                ("model-1".to_owned(), PathBuf::from("model-1.gguf")),
                ("model-2".to_owned(), PathBuf::from("model-2.gguf")),
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
        state.temp = "0.95".to_owned();
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
        state.temp = "0.95".to_owned();
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
        state.temp = "".to_owned();
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
            "llama-server".to_owned(),
            "--model".to_owned(),
            "model.gguf".to_owned(),
        ];
        assert_eq!(
            mask_sensitive_args(&args),
            "llama-server --model model.gguf"
        );

        // 2. With api-key arg
        let args_with_key = vec![
            "llama-server".to_owned(),
            "--api-key".to_owned(),
            "secret-12345".to_owned(),
            "--model".to_owned(),
            "model.gguf".to_owned(),
        ];
        assert_eq!(
            mask_sensitive_args(&args_with_key),
            "llama-server --api-key [MASKED] --model model.gguf"
        );

        // 3. Edgecase: api-key at the end of the argument list without a value
        let args_edge = vec!["llama-server".to_owned(), "--api-key".to_owned()];
        assert_eq!(mask_sensitive_args(&args_edge), "llama-server --api-key");
    }

    #[test]
    fn test_load_settings_from_draft_config_fallbacks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();

        let model_path = models_dir.join("main.gguf");
        std::fs::write(&model_path, b"main").unwrap();

        let draft_path = models_dir.join("draft.gguf");
        std::fs::write(&draft_path, b"draft").unwrap();

        // Draft config with spec-draft-n-max
        let draft_config_path = models_dir.join("draft.toml");
        std::fs::write(
            &draft_config_path,
            r#"[llama-herd]
total-layers = 4
is-draft-only = true

[llama-server-long]
spec-draft-n-max = 2
spec-draft-p-min = 0.85
spec-type = "draft-eagle3"
"#,
        )
        .unwrap();

        // Main config (no speculative settings overrides)
        let main_config_path = models_dir.join("main.toml");
        std::fs::write(&main_config_path, "").unwrap();

        let preset_path = temp_dir.path().join("models-preset.ini");
        std::fs::write(
            &preset_path,
            r#"[main]
model = main.gguf
model-draft = draft.gguf
gpu-layers-draft = 4
"#,
        )
        .unwrap();
        let presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);

        let mut state = AppState::new(
            presets,
            models_dir.clone(),
            preset_path,
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        // Load the settings
        state.load_current_preset_settings(None);

        assert_eq!(state.spec_draft_n_max, "2");
        assert_eq!(state.spec_draft_p_min, "0.85");
        assert_eq!(state.spec_type, "draft-eagle3");
    }

    #[test]
    fn test_tui_input_validation_temp() {
        let mut state = AppState::new(
            vec![("test".to_owned(), PathBuf::from("test.gguf"))],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.screen = AppScreen::EditingTemp;
        state.input_buffer = "invalid_temp".to_owned();

        let key = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        let _ = handle_key_event(&mut state, key, &tx);

        // Should refuse to close and set validation error
        assert_eq!(state.screen, AppScreen::EditingTemp);
        assert!(state.validation_error.is_some());

        // Typing a char should clear validation error
        let type_key = KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let _ = handle_key_event(&mut state, type_key, &tx);
        assert!(state.validation_error.is_none());
    }

    #[test]
    fn test_f4_keybinding_dispatches_control_signal() {
        use llama_herd::tui::logs::ActiveServer;
        use std::path::Path;

        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // Test 1: When active_server is None
        let key = KeyEvent {
            code: KeyCode::F(4),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let result = handle_key_event(&mut state, key, &tx);
        assert!(!result, "handle_key_event for F4 should return false");

        // Test 2: When active_server is Some
        let params = if cfg!(target_os = "windows") {
            vec![
                "ping".to_owned(),
                "127.0.0.1".to_owned(),
                "-n".to_owned(),
                "10".to_owned(),
            ]
        } else {
            vec!["sleep".to_owned(), "10".to_owned()]
        };

        if let Ok(server) = ActiveServer::spawn(&params, Path::new("."), None, None) {
            state.active_server = Some(server);
            let result = handle_key_event(&mut state, key, &tx);
            assert!(!result, "handle_key_event for F4 should return false");

            {
                let active_server = state.active_server.as_ref().unwrap();
                let raw_hist = active_server.raw_history.lock().unwrap();
                let has_msg = raw_hist
                    .iter()
                    .any(|line| line.contains("[CONTROL] F4 pressed"));
                assert!(has_msg, "raw_history should contain F4 control log line");

                let logs = active_server.logs.lock().unwrap();
                let has_log = logs.iter().any(|l| {
                    l.spans
                        .iter()
                        .any(|s| s.text.contains("[CONTROL] F4 pressed"))
                });
                assert!(has_log, "logs should contain F4 control log line");
            }

            if let Some(ref mut active_server) = state.active_server {
                active_server.kill();
            }
        }
    }

    #[test]
    fn test_autoscroll_toggle_keybinding() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.screen = AppScreen::Logs;
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        state.auto_scroll = true;

        let key_a = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_a, &tx);
        assert!(!state.auto_scroll);

        let key_cap_a = KeyEvent {
            code: KeyCode::Char('A'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_cap_a, &tx);
        assert!(state.auto_scroll);

        let key_space = KeyEvent {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_space, &tx);
        assert!(!state.auto_scroll);
    }

    #[test]
    fn test_draw_logs_screen_footer_and_metrics() {
        use llama_herd::tui::logs::ActiveServer;
        use ratatui::{Terminal, backend::TestBackend};
        use std::path::Path;

        let mut state = AppState::new(
            vec![("preset1".to_owned(), PathBuf::from("preset1.gguf"))],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.screen = AppScreen::Logs;

        // 1. Verify wide screen footer text and metrics
        {
            let backend = TestBackend::new(120, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            let params = if cfg!(target_os = "windows") {
                vec![
                    "ping".to_owned(),
                    "127.0.0.1".to_owned(),
                    "-n".to_owned(),
                    "10".to_owned(),
                ]
            } else {
                vec!["sleep".to_owned(), "10".to_owned()]
            };

            if let Ok(server) = ActiveServer::spawn(&params, Path::new("."), None, None) {
                if let Ok(mut m) = server.metrics.lock() {
                    m.status = "HEALTHY".to_owned();
                    m.pid = Some(9999);
                    m.active_model = Some("Qwen-7B.gguf".to_owned());
                    m.active_port = Some(8080);
                    m.ram_usage = Some((4096, 16384));
                    m.vram_usage = Some((8192, 12288));
                }
                state.active_server = Some(server);
            }

            terminal
                .draw(|f| {
                    llama_herd::tui::ui::draw(f, &mut state);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();

            // Check footer line 1 for updated keybindings
            let mut footer_str = String::new();
            for x in 0..120 {
                footer_str.push(buffer[(x, 28)].symbol().chars().next().unwrap_or(' '));
            }
            assert!(
                footer_str.contains("[A/Space] Auto-scroll"),
                "Footer line 1 should contain '[A/Space] Auto-scroll', got: '{}'",
                footer_str
            );
            assert!(
                footer_str.contains("[P] Pause"),
                "Footer line 1 should contain '[P] Pause', got: '{}'",
                footer_str
            );
            assert!(
                footer_str.contains("[W]"),
                "Footer line 1 should contain '[W]', got: '{}'",
                footer_str
            );
            assert!(
                footer_str.contains("[C] Copy"),
                "Footer line 1 should contain '[C] Copy', got: '{}'",
                footer_str
            );
            assert!(
                footer_str.contains("[F4] Cancel"),
                "Footer line 1 should contain '[F4] Cancel', got: '{}'",
                footer_str
            );

            // Verify status panel metrics output in buffer
            let mut buf_text = String::new();
            for y in 0..30 {
                for x in 0..120 {
                    buf_text.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
                }
                buf_text.push('\n');
            }
            assert!(buf_text.contains("HEALTHY"));
            assert!(buf_text.contains("9999"));
            assert!(buf_text.contains("Qwen-7B.gguf"));
            assert!(buf_text.contains("8080"));

            if let Some(ref mut server) = state.active_server {
                server.kill();
            }
        }

        // 2. Verify narrow screen footer text and metrics
        {
            let backend = TestBackend::new(90, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            let params = if cfg!(target_os = "windows") {
                vec![
                    "ping".to_owned(),
                    "127.0.0.1".to_owned(),
                    "-n".to_owned(),
                    "10".to_owned(),
                ]
            } else {
                vec!["sleep".to_owned(), "10".to_owned()]
            };

            if let Ok(server) = ActiveServer::spawn(&params, Path::new("."), None, None) {
                if let Ok(mut m) = server.metrics.lock() {
                    m.status = "RECOVERING".to_owned();
                    m.pid = Some(8888);
                    m.active_model = Some("Llama3-8B.gguf".to_owned());
                    m.active_port = Some(8081);
                }
                state.active_server = Some(server);
            }

            terminal
                .draw(|f| {
                    llama_herd::tui::ui::draw(f, &mut state);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();

            let mut footer_str = String::new();
            for x in 0..90 {
                footer_str.push(buffer[(x, 28)].symbol().chars().next().unwrap_or(' '));
            }
            assert!(footer_str.contains("[A/Space] Auto-scroll"));
            assert!(footer_str.contains("[P] Pause"));
            assert!(footer_str.contains("[W] Wrap"));
            assert!(footer_str.contains("[C] Copy"));
            assert!(footer_str.contains("[F4] Cancel"));

            let mut buf_text = String::new();
            for y in 0..30 {
                for x in 0..90 {
                    buf_text.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
                }
                buf_text.push('\n');
            }
            assert!(buf_text.contains("RECOVERING"));
            assert!(buf_text.contains("8888"));

            if let Some(ref mut server) = state.active_server {
                server.kill();
            }
        }
    }

    #[test]
    fn test_handle_key_event_left_right_variant_cycling() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");
        let model_path = models_dir.join("model1.gguf");

        std::fs::write(
            &preset_path,
            r#"
[model1]
model = model1.gguf

[model1-draft]
model = model1.gguf
model-draft = draft.gguf
"#,
        )
        .unwrap();

        let presets = vec![
            ("model1".to_string(), model_path.clone()),
            ("model1-draft".to_string(), model_path.clone()),
        ];

        let mut state = AppState::new(
            presets,
            models_dir,
            preset_path,
            HashMap::new(),
            PathBuf::from("llama-server"),
            Theme::default(),
        );

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        assert_eq!(state.grouped_models.len(), 1);
        assert_eq!(state.grouped_models[0].variants.len(), 2);
        assert_eq!(state.grouped_models[0].selected_variant_index, 0);
        assert_eq!(state.preset_index, 0);
        assert_eq!(state.dashboard_focus, DashboardFocus::Left);

        // KeyCode::Right -> cycles to variant 1 (Draft)
        let key_right = KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_right, &tx);
        assert_eq!(state.grouped_models[0].selected_variant_index, 1);
        assert_eq!(state.preset_index, 1);
        assert_eq!(state.presets[state.preset_index].0, "model1-draft");

        // KeyCode::Left -> cycles back to variant 0 (Base)
        let key_left = KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_left, &tx);
        assert_eq!(state.grouped_models[0].selected_variant_index, 0);
        assert_eq!(state.preset_index, 0);
        assert_eq!(state.presets[state.preset_index].0, "model1");

        // With unsaved changes, Right triggers WarnDiscardChanges
        state.temp = "0.99".to_string();
        assert!(state.has_unsaved_changes());
        handle_key_event(&mut state, key_right, &tx);
        assert_eq!(state.screen, AppScreen::WarnDiscardChanges);
        assert_eq!(state.pending_preset_index, Some(1));
        assert_eq!(state.preset_index, 0);

        // Discard changes with Enter
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.preset_index, 1);
        assert_eq!(state.grouped_models[0].selected_variant_index, 1);
        assert_eq!(state.selected_model_index, 0);
    }

    #[test]
    fn test_handle_key_event_up_down_model_navigation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");
        let model1_path = models_dir.join("model1.gguf");
        let model2_path = models_dir.join("model2.gguf");

        std::fs::write(
            &preset_path,
            r#"
[model1]
model = model1.gguf

[model2]
model = model2.gguf
"#,
        )
        .unwrap();

        let presets = vec![
            ("model1".to_string(), model1_path.clone()),
            ("model2".to_string(), model2_path.clone()),
        ];

        let mut state = AppState::new(
            presets,
            models_dir,
            preset_path,
            HashMap::new(),
            PathBuf::from("llama-server"),
            Theme::default(),
        );

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        assert_eq!(state.grouped_models.len(), 2);
        assert_eq!(state.selected_model_index, 0);
        assert_eq!(state.preset_index, 0);

        // KeyCode::Down -> moves to model 1
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.selected_model_index, 1);
        assert_eq!(state.preset_index, 1);
        assert_eq!(state.presets[state.preset_index].0, "model2");

        // KeyCode::Up -> moves back to model 0
        let key_up = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_up, &tx);
        assert_eq!(state.selected_model_index, 0);
        assert_eq!(state.preset_index, 0);
        assert_eq!(state.presets[state.preset_index].0, "model1");
    }

    #[test]
    fn test_ui_dashboard_renders_left_panel_with_badges() {
        use ratatui::{Terminal, backend::TestBackend};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");
        let model1_path = models_dir.join("model1.gguf");
        let model2_path = models_dir.join("model2.gguf");

        std::fs::write(
            &preset_path,
            r#"
[model1]
model = model1.gguf

[model1-draft]
model = model1.gguf
model-draft = draft.gguf

[model2]
model = model2.gguf
"#,
        )
        .unwrap();

        let presets = vec![
            ("model1".to_string(), model1_path.clone()),
            ("model1-draft".to_string(), model1_path.clone()),
            ("model2".to_string(), model2_path.clone()),
        ];

        let mut state = AppState::new(
            presets,
            models_dir,
            preset_path,
            HashMap::new(),
            PathBuf::from("llama-server"),
            Theme::default(),
        );

        terminal
            .draw(|f| {
                llama_herd::tui::ui::draw(f, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut buffer_text = String::new();
        for y in 0..40 {
            for x in 0..120 {
                buffer_text.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            buffer_text.push('\n');
        }

        assert!(buffer_text.contains("model1"));
        assert!(buffer_text.contains("[B]"));
        assert!(buffer_text.contains("[D]"));
        assert!(buffer_text.contains("model2"));
    }

    #[test]
    fn test_picking_models_dir_rebuilds_grouped_models() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir_1 = temp_dir.path().join("models1");
        let models_dir_2 = temp_dir.path().join("models2");
        std::fs::create_dir_all(&models_dir_1).unwrap();
        std::fs::create_dir_all(&models_dir_2).unwrap();

        let model1_path = models_dir_1.join("model1.gguf");
        let model2_path = models_dir_2.join("model2.gguf");
        std::fs::write(&model1_path, b"dummy1").unwrap();
        std::fs::write(&model2_path, b"dummy2").unwrap();

        let preset_path = temp_dir.path().join("models-preset.ini");
        let config_path = temp_dir.path().join("config.toml");

        // Generate initial presets for models_dir_1
        let mut global_config = HashMap::new();
        global_config.insert(
            "models-dir".to_string(),
            serde_json::Value::String(models_dir_1.to_string_lossy().to_string()),
        );

        let _ = llama_herd::discovery::generate_presets_ini(
            &models_dir_1,
            &preset_path,
            &global_config,
        );
        let initial_presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);

        let mut state = AppState::new(
            initial_presets,
            models_dir_1.clone(),
            preset_path,
            global_config,
            PathBuf::from("llama-server"),
            Theme::default(),
        );
        state.config_path = config_path;

        // Verify initial state has model1 in grouped_models
        assert_eq!(state.grouped_models.len(), 1);
        assert_eq!(state.grouped_models[0].base_name, "model1");
        assert_eq!(state.grouped_models[0].model_path, model1_path);
        assert!(
            state.grouped_models[0]
                .variants
                .iter()
                .any(|v| v.preset_name == "model1")
        );

        // Simulate opening the directory picker for models_dir_2
        state.screen = AppScreen::PickingModelsDir;
        state.picker = Some(llama_herd::tui::picker::FilePicker::new(
            models_dir_2.clone(),
            llama_herd::tui::picker::PickerMode::Directory,
        ));

        // Press Enter on ".[Select current directory]"
        let key = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        let quit = handle_key_event(&mut state, key, &tx);

        assert!(!quit);
        assert_eq!(state.screen, AppScreen::Settings);
        assert_eq!(state.models_dir, models_dir_2);

        // Verify grouped_models has been rebuilt to reflect model2
        assert_eq!(state.grouped_models.len(), 1);
        assert_eq!(state.grouped_models[0].base_name, "model2");
        assert_eq!(state.grouped_models[0].model_path, model2_path);
        assert!(
            state.grouped_models[0]
                .variants
                .iter()
                .any(|v| v.preset_name == "model2")
        );
    }

    #[test]
    fn test_dashboard_enter_and_esc_focus_switching() {
        let mut state = AppState::new(
            vec![("model1".to_string(), PathBuf::from("model1.gguf"))],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("llama-server"),
            Theme::default(),
        );

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // Initially Left is focused
        assert_eq!(state.dashboard_focus, DashboardFocus::Left);

        // Pressing Enter on Left panel switches focus to Right panel (Parameters)
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.dashboard_focus, DashboardFocus::Right);

        // Pressing Esc on Right panel switches focus back to Left panel (Models)
        let key_esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        handle_key_event(&mut state, key_esc, &tx);
        assert_eq!(state.dashboard_focus, DashboardFocus::Left);
    }

    #[test]
    fn test_handle_key_event_editing_variants() {
        let mut state = AppState::new(
            vec![],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );
        state.variants = "all".to_owned();
        state.original_variants = "all".to_owned();
        assert!(!state.has_unsaved_changes());

        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 3; // Variants

        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

        // 1. Enter key -> opens EditingVariants screen
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingVariants);
        assert_eq!(state.input_buffer, "all");

        // 2. Type new variants and submit
        state.input_buffer = "base, draft-vision".to_owned();
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.variants, "base, draft-vision");
        assert!(state.has_unsaved_changes());
    }

    #[test]
    fn test_save_variants_config_updates_presets_and_badges() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");

        // Create base model and draft model
        let model_gguf = models_dir.join("my-model.gguf");
        std::fs::write(&model_gguf, b"").unwrap();
        let draft_gguf = models_dir.join("my-model-draft.gguf");
        std::fs::write(&draft_gguf, b"").unwrap();

        // Initial config with draft model
        let model_toml = models_dir.join("my-model.toml");
        let toml_content = r#"
[llama-herd]
draft = "my-model-draft.gguf"
"#;
        std::fs::write(&model_toml, toml_content.as_bytes()).unwrap();

        let draft_toml = models_dir.join("my-model-draft.toml");
        std::fs::write(&draft_toml, b"[llama-herd]\nis-draft = true\n").unwrap();

        let _ =
            llama_herd::discovery::generate_presets_ini(&models_dir, &preset_path, &HashMap::new());
        let presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);
        // Generates my-model + my-model-draft = 2 presets
        assert_eq!(presets.len(), 2);

        let mut state = AppState::new(
            presets,
            models_dir.clone(),
            preset_path.clone(),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        // 1 grouped model: "my-model"
        assert_eq!(state.grouped_models.len(), 1);
        assert_eq!(state.grouped_models[0].base_name, "my-model");
        assert_eq!(state.grouped_models[0].variants.len(), 2);
        assert_eq!(state.variants, "all");

        // Select my-model (index 0)
        state.select_model(0);

        // Restrict variants to only "base"
        state.variants = "base".to_owned();
        assert!(state.has_unsaved_changes());

        // Save config
        state.save_current_preset_config(false).unwrap();
        assert!(!state.has_unsaved_changes());

        // Verify TOML file updated with variants = "base" in [llama-herd]
        let saved_config = llama_herd::config::load_toml_silent(&model_toml);
        let herd = saved_config.get("llama-herd").unwrap().as_object().unwrap();
        assert_eq!(herd.get("variants").unwrap().as_str().unwrap(), "base");

        // Verify presets and grouped models reloaded with only 1 variant for my-model
        assert_eq!(state.presets.len(), 1);
        assert_eq!(state.grouped_models.len(), 1);
        assert_eq!(state.grouped_models[0].variants.len(), 1);
        assert_eq!(
            state.grouped_models[0].variants[0].variant,
            llama_herd::tui::app::ModelVariant::Base
        );
    }

    #[test]
    fn test_save_variants_config_draft_vision_preserves_selection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let models_dir = temp_dir.path().to_path_buf();
        let preset_path = models_dir.join("models-preset.ini");

        let model_file = models_dir.join("gemma-4-12B-it-heretic.Q8_0.gguf");
        std::fs::write(&model_file, b"gemma content").unwrap();

        let model_toml = models_dir.join("gemma-4-12B-it-heretic.Q8_0.toml");
        std::fs::write(
            &model_toml,
            b"[llama-herd]\ndraft = \"mtp-gemma-4-12b-it-Q8_0.gguf\"\nvariants = \"all\"\n",
        )
        .unwrap();

        let draft_file = models_dir.join("mtp-gemma-4-12b-it-Q8_0.gguf");
        std::fs::write(&draft_file, b"draft content").unwrap();
        let draft_toml = models_dir.join("mtp-gemma-4-12b-it-Q8_0.toml");
        std::fs::write(&draft_toml, b"[llama-herd]\nis-draft = true\n").unwrap();

        let mmproj_file = models_dir.join("gemma-4-12b-it-mmproj-F32.gguf");
        std::fs::write(&mmproj_file, b"mmproj content").unwrap();

        let _ =
            llama_herd::discovery::generate_presets_ini(&models_dir, &preset_path, &HashMap::new());
        let presets = llama_herd::discovery::discover_presets_from_ini(&preset_path);
        // base + vision + draft + draft-vision = 4 presets
        assert_eq!(presets.len(), 4);

        let mut state = AppState::new(
            presets,
            models_dir.clone(),
            preset_path.clone(),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        // Select model 0 (gemma), and cycle to draft-vision variant
        state.select_model(0);
        while {
            let m = &state.grouped_models[0];
            m.variants[m.selected_variant_index].variant
                != llama_herd::tui::app::ModelVariant::DraftVision
        } {
            state.cycle_variant_next();
        }

        // Set variants = "draft-vision" and save
        state.variants = "draft-vision".to_owned();
        state.save_current_preset_config(false).unwrap();

        // After save, the selected variant should be DraftVision, NOT reset to Base or default
        assert_eq!(state.selected_model_index, 0);
        let current_model = &state.grouped_models[0];
        assert_eq!(
            current_model.variants[current_model.selected_variant_index].variant,
            llama_herd::tui::app::ModelVariant::DraftVision
        );
        let current_preset_name = &state.presets[state.preset_index].0;
        assert!(
            current_preset_name.contains("draft-vision"),
            "Expected current preset to contain draft-vision, got: {current_preset_name}"
        );
    }

    #[test]
    fn test_dashboard_param_navigation() {
        let mut state = AppState::new(
            vec![("test-preset".to_owned(), PathBuf::from("test-model.gguf"))],
            PathBuf::from("."),
            PathBuf::from("."),
            HashMap::new(),
            PathBuf::from("."),
            Theme::default(),
        );

        state.dashboard_focus = DashboardFocus::Right;
        state.dashboard_param_index = 0;

        let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();
        let key_enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let key_up = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let key_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let key_esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        // 1. Up from 0 wraps to 25
        handle_key_event(&mut state, key_up, &tx);
        assert_eq!(state.dashboard_param_index, 25);

        // 2. Down from 25 wraps to 0
        handle_key_event(&mut state, key_down, &tx);
        assert_eq!(state.dashboard_param_index, 0);

        // 3. Enter on index 0 opens EditingCustomName
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingCustomName);

        // Edit custom name and submit
        state.input_buffer = "My Custom Preset".to_owned();
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);
        assert_eq!(state.current_custom_name(), "My Custom Preset");
        assert!(state.has_unsaved_changes());

        // 4. Index 1 opens EditingConfigFileName
        state.dashboard_param_index = 1;
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingConfigFileName);
        handle_key_event(&mut state, key_esc, &tx);
        assert_eq!(state.screen, AppScreen::Dashboard);

        // 5. Index 2 opens EditingTotalLayers
        state.dashboard_param_index = 2;
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingTotalLayers);
        handle_key_event(&mut state, key_esc, &tx);

        // 6. Index 3 opens EditingVariants
        state.dashboard_param_index = 3;
        handle_key_event(&mut state, key_enter, &tx);
        assert_eq!(state.screen, AppScreen::EditingVariants);
        handle_key_event(&mut state, key_esc, &tx);
    }
}
