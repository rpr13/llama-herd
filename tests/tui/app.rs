use llama_herd::tui::app::AppState;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_app_state_initialization() {
    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("test-model.gguf");
    fs::write(&model_path, "dummy").unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[*]
flash-attn = auto

[test-preset]
model = test-model.gguf
ctx-size = 8192
n-gpu-layers = 32
"#,
    )
    .unwrap();

    let presets = vec![("test-preset".to_string(), model_path)];
    let global_config = HashMap::new();
    let server_exe = PathBuf::from("llama-server");

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        global_config,
        server_exe,
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.ctx, 8192);
    assert_eq!(state.ngl, "32");
}

#[test]
fn test_app_state_mmproj_discovery() {
    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("test-model.gguf");
    fs::write(&model_path, "dummy").unwrap();

    let mmproj_path = models_dir.join("mmproj-model.gguf");
    fs::write(&mmproj_path, "dummy").unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[test-preset]
model = test-model.gguf
mmproj = mmproj-model.gguf
"#,
    )
    .unwrap();

    let presets = vec![("test-preset".to_string(), model_path)];
    let global_config = HashMap::new();
    let server_exe = PathBuf::from("llama-server");

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        global_config,
        server_exe,
        llama_herd::tui::theme::Theme::default(),
    );

    // mmproj_list should contain [None, Some(mmproj_path)]
    assert_eq!(state.mmproj_list.len(), 2);
    assert!(state.mmproj_list[1].is_some());
    assert_eq!(
        state.mmproj_list[state.mmproj_index]
            .as_ref()
            .unwrap()
            .file_name()
            .unwrap(),
        "mmproj-model.gguf"
    );
}

#[test]
fn test_app_state_draft_discovery() {
    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("main-model.gguf");
    fs::write(&model_path, "dummy").unwrap();

    let draft_path = models_dir.join("draft-model.gguf");
    fs::write(&draft_path, "dummy").unwrap();

    let draft_config_path = models_dir.join("draft-model.toml");
    fs::write(
        &draft_config_path,
        r#"
[llama-herd]
is-draft = true
"#,
    )
    .unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[test-preset-draft]
model = main-model.gguf
model-draft = draft-model.gguf
gpu-layers-draft = 10
"#,
    )
    .unwrap();

    let presets = vec![("test-preset-draft".to_string(), model_path)];
    let global_config = HashMap::new();
    let server_exe = PathBuf::from("llama-server");

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        global_config,
        server_exe,
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.draft_list.len(), 2);
    assert!(state.draft_list[1].is_some());
    assert_eq!(
        state.draft_list[state.draft_index]
            .as_ref()
            .unwrap()
            .file_name()
            .unwrap(),
        "draft-model.gguf"
    );
    assert_eq!(state.draft_ngl, "10");
}

#[test]
fn test_app_state_lh_draft_discovery() {
    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("main-model.gguf");
    fs::write(&model_path, "dummy").unwrap();

    let draft_path = models_dir.join("draft-model.gguf");
    fs::write(&draft_path, "dummy").unwrap();

    let draft_config_path = models_dir.join("draft-model.toml");
    fs::write(
        &draft_config_path,
        r#"
[llama-herd]
is-draft-only = true
"#,
    )
    .unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[test-preset-draft]
model = main-model.gguf
model-draft = draft-model.gguf
gpu-layers-draft = 10
"#,
    )
    .unwrap();

    let presets = vec![("test-preset-draft".to_string(), model_path)];
    let global_config = HashMap::new();
    let server_exe = PathBuf::from("llama-server");

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        global_config,
        server_exe,
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.draft_list.len(), 2);
    assert!(state.draft_list[1].is_some());
    assert_eq!(
        state.draft_list[state.draft_index]
            .as_ref()
            .unwrap()
            .file_name()
            .unwrap(),
        "draft-model.gguf"
    );
    assert_eq!(state.draft_ngl, "10");
}

#[test]
fn test_app_state_draft_heuristic() {
    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    // Main model: Llama-3-8B.gguf
    let model_path = models_dir.join("Llama-3-8B.gguf");
    fs::write(&model_path, "dummy").unwrap();

    // Draft model: Llama-3-Draft.gguf
    let draft_path = models_dir.join("Llama-3-Draft.gguf");
    fs::write(&draft_path, "dummy").unwrap();

    let draft_config_path = models_dir.join("Llama-3-Draft.toml");
    fs::write(
        &draft_config_path,
        r#"
[llama-herd]
is-draft = true
"#,
    )
    .unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[test-preset-draft]
model = Llama-3-8B.gguf
"#,
    )
    .unwrap();

    let presets = vec![("test-preset-draft".to_string(), model_path)];
    let global_config = HashMap::new();
    let server_exe = PathBuf::from("llama-server");

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        global_config,
        server_exe,
        llama_herd::tui::theme::Theme::default(),
    );

    // Heuristic should have selected the draft model
    assert_eq!(state.draft_list.len(), 2);
    assert_eq!(state.draft_index, 1);
    assert_eq!(
        state.draft_list[state.draft_index]
            .as_ref()
            .unwrap()
            .file_name()
            .unwrap(),
        "Llama-3-Draft.gguf"
    );
    assert_eq!(state.draft_ngl, "auto");
}

#[test]
fn test_app_state_get_user_settings() {
    use llama_herd::tui::app::AppState;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("test.gguf");
    fs::write(&model_path, "").unwrap();

    let preset_path = dir.path().join("presets.ini");
    fs::write(&preset_path, "[test]\nctx-size = 4096\n").unwrap();

    let presets = vec![("test".to_string(), model_path.clone())];

    let mut state = AppState::new(
        presets,
        models_dir.clone(),
        preset_path,
        HashMap::new(),
        PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    // Mutate state to simulate user interactions
    state.ctx = 8192;
    state.ngl = "32".to_string();

    // Add mock paths to list and select them
    let mmproj = models_dir.join("vision.gguf");
    let draft = models_dir.join("draft.gguf");
    state.mmproj_list = vec![None, Some(mmproj.clone())];
    state.mmproj_index = 1;
    state.draft_list = vec![None, Some(draft.clone())];
    state.draft_index = 1;
    state.draft_ngl = "auto".to_string();

    let settings = state.get_user_settings();

    assert_eq!(settings.ctx, 8192);
    assert_eq!(settings.ngl, "32");
    assert_eq!(settings.mmproj, Some(mmproj));
    assert_eq!(settings.draft_model, Some(draft));
    assert_eq!(settings.draft_ngl, "auto");
}
