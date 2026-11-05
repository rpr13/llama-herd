#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

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

#[test]
fn test_grouped_models_variant_cycling() {
    use llama_herd::tui::app::{AppState, ModelVariant};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

    let mmproj_path = models_dir.join("mmproj-model.gguf");
    fs::write(&mmproj_path, "dummy").unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[test-preset]
model = main-model.gguf
ctx-size = 4096

[test-preset-draft]
model = main-model.gguf
model-draft = draft-model.gguf
gpu-layers-draft = 10

[test-preset-vision]
model = main-model.gguf
mmproj = mmproj-model.gguf

[test-preset-draft-vision]
model = main-model.gguf
model-draft = draft-model.gguf
mmproj = mmproj-model.gguf
gpu-layers-draft = 10
"#,
    )
    .unwrap();

    let presets = vec![
        ("test-preset".to_string(), model_path.clone()),
        ("test-preset-draft".to_string(), model_path.clone()),
        ("test-preset-vision".to_string(), model_path.clone()),
        ("test-preset-draft-vision".to_string(), model_path.clone()),
    ];

    let mut state = AppState::new(
        presets,
        models_dir,
        preset_path,
        HashMap::new(),
        PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    // Initial grouping checks
    assert_eq!(state.grouped_models.len(), 1);
    assert_eq!(state.selected_model_index, 0);

    let grouped = &state.grouped_models[0];
    assert_eq!(grouped.base_name, "main-model");
    assert_eq!(grouped.model_path, model_path);
    assert_eq!(grouped.variants.len(), 4);
    assert_eq!(grouped.selected_variant_index, 0);

    assert_eq!(grouped.variants[0].variant, ModelVariant::Base);
    assert_eq!(grouped.variants[0].preset_name, "test-preset");
    assert_eq!(grouped.variants[0].preset_index, 0);

    assert_eq!(grouped.variants[1].variant, ModelVariant::Draft);
    assert_eq!(grouped.variants[1].preset_name, "test-preset-draft");
    assert_eq!(grouped.variants[1].preset_index, 1);

    assert_eq!(grouped.variants[2].variant, ModelVariant::Vision);
    assert_eq!(grouped.variants[2].preset_name, "test-preset-vision");
    assert_eq!(grouped.variants[2].preset_index, 2);

    assert_eq!(grouped.variants[3].variant, ModelVariant::DraftVision);
    assert_eq!(grouped.variants[3].preset_name, "test-preset-draft-vision");
    assert_eq!(grouped.variants[3].preset_index, 3);

    // Initial preset_index should match the base variant
    assert_eq!(state.preset_index, 0);

    // Cycle forward: Base (0) -> Draft (1)
    state.cycle_variant_next();
    assert_eq!(state.grouped_models[0].selected_variant_index, 1);
    assert_eq!(state.preset_index, 1);
    assert_eq!(state.presets[state.preset_index].0, "test-preset-draft");
    assert_eq!(state.draft_ngl, "10");

    // Cycle forward: Draft (1) -> Vision (2)
    state.cycle_variant_next();
    assert_eq!(state.grouped_models[0].selected_variant_index, 2);
    assert_eq!(state.preset_index, 2);
    assert_eq!(state.presets[state.preset_index].0, "test-preset-vision");

    // Cycle forward: Vision (2) -> DraftVision (3)
    state.cycle_variant_next();
    assert_eq!(state.grouped_models[0].selected_variant_index, 3);
    assert_eq!(state.preset_index, 3);
    assert_eq!(
        state.presets[state.preset_index].0,
        "test-preset-draft-vision"
    );

    // Cycle forward wraps: DraftVision (3) -> Base (0)
    state.cycle_variant_next();
    assert_eq!(state.grouped_models[0].selected_variant_index, 0);
    assert_eq!(state.preset_index, 0);
    assert_eq!(state.presets[state.preset_index].0, "test-preset");

    // Cycle backward wraps: Base (0) -> DraftVision (3)
    state.cycle_variant_prev();
    assert_eq!(state.grouped_models[0].selected_variant_index, 3);
    assert_eq!(state.preset_index, 3);
    assert_eq!(
        state.presets[state.preset_index].0,
        "test-preset-draft-vision"
    );

    // Cycle backward: DraftVision (3) -> Vision (2)
    state.cycle_variant_prev();
    assert_eq!(state.grouped_models[0].selected_variant_index, 2);
    assert_eq!(state.preset_index, 2);
    assert_eq!(state.presets[state.preset_index].0, "test-preset-vision");

    // Test select_model
    state.select_model(0);
    assert_eq!(state.selected_model_index, 0);
    assert_eq!(state.grouped_models[0].selected_variant_index, 2);
    assert_eq!(state.preset_index, 2);

    // Verify badge_label
    assert_eq!(ModelVariant::Base.badge_label(), "B");
    assert_eq!(ModelVariant::Draft.badge_label(), "D");
    assert_eq!(ModelVariant::Vision.badge_label(), "V");
    assert_eq!(ModelVariant::DraftVision.badge_label(), "DV");
}

#[test]
fn test_grouped_models_fallback_detection_without_ini() {
    use llama_herd::tui::app::{AppState, ModelVariant};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("Qwen2.5-7B-Instruct-Q4_K_M.gguf");
    fs::write(&model_path, "dummy").unwrap();

    // INI does not exist
    let preset_path = dir.path().join("non-existent.ini");

    let presets = vec![
        ("qwen-base".to_string(), model_path.clone()),
        ("qwen-draft".to_string(), model_path.clone()),
        ("qwen-vision".to_string(), model_path.clone()),
        ("qwen-draft-vision".to_string(), model_path.clone()),
    ];

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        HashMap::new(),
        PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.grouped_models.len(), 1);
    let grouped = &state.grouped_models[0];
    assert_eq!(grouped.variants.len(), 4);
    assert_eq!(grouped.variants[0].variant, ModelVariant::Base);
    assert_eq!(grouped.variants[1].variant, ModelVariant::Draft);
    assert_eq!(grouped.variants[2].variant, ModelVariant::Vision);
    assert_eq!(grouped.variants[3].variant, ModelVariant::DraftVision);
}

#[test]
fn test_grouped_models_multi_model_selection() {
    use llama_herd::tui::app::AppState;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model1_path = models_dir.join("model1.gguf");
    fs::write(&model1_path, "dummy").unwrap();

    let model2_path = models_dir.join("model2.gguf");
    fs::write(&model2_path, "dummy").unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[model1-preset]
model = model1.gguf

[model1-preset-draft]
model = model1.gguf
model-draft = draft.gguf

[model2-preset]
model = model2.gguf
"#,
    )
    .unwrap();

    let presets = vec![
        ("model1-preset".to_string(), model1_path.clone()),
        ("model1-preset-draft".to_string(), model1_path.clone()),
        ("model2-preset".to_string(), model2_path.clone()),
    ];

    let mut state = AppState::new(
        presets,
        models_dir,
        preset_path,
        HashMap::new(),
        PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.grouped_models.len(), 2);
    assert_eq!(state.selected_model_index, 0);
    assert_eq!(state.grouped_models[0].base_name, "model1");
    assert_eq!(state.grouped_models[0].variants.len(), 2);
    assert_eq!(state.grouped_models[1].base_name, "model2");
    assert_eq!(state.grouped_models[1].variants.len(), 1);

    // Switch to Model 2
    state.select_model(1);
    assert_eq!(state.selected_model_index, 1);
    assert_eq!(state.preset_index, 2);
    assert_eq!(state.presets[state.preset_index].0, "model2-preset");

    // Switch back to Model 1
    state.select_model(0);
    assert_eq!(state.selected_model_index, 0);
    assert_eq!(state.preset_index, 0);
    assert_eq!(state.presets[state.preset_index].0, "model1-preset");
}

#[test]
fn test_grouped_models_never_groups_into_default() {
    use llama_herd::tui::app::AppState;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("gemma-12b.gguf");
    fs::write(&model_path, "dummy").unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[gemma-12b]
model = gemma-12b.gguf

[gemma-12b-draft]
model = gemma-12b.gguf
model-draft = draft.gguf
"#,
    )
    .unwrap();

    let presets = vec![
        ("gemma-12b".to_string(), model_path.clone()),
        ("gemma-12b-draft".to_string(), model_path.clone()),
    ];

    let state = AppState::new(
        presets,
        models_dir,
        preset_path,
        HashMap::new(),
        PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.grouped_models.len(), 1);
    assert_eq!(state.grouped_models[0].base_name, "gemma-12b");
    assert_eq!(state.grouped_models[0].variants.len(), 2);
    assert!(
        !state
            .grouped_models
            .iter()
            .any(|g| g.base_name == "default")
    );
}

#[test]
fn test_app_state_variant_custom_names_loading_cycling_and_saving() {
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

    let model_config_path = models_dir.join("main-model.toml");
    fs::write(
        &model_config_path,
        r#"
[llama-herd]
is-default = true

[llama-herd.custom-name]
base = "my-custom-base"
draft = "my-custom-draft"
"#,
    )
    .unwrap();

    let preset_path = dir.path().join("models-preset.ini");
    fs::write(
        &preset_path,
        r#"
[my-custom-base]
model = main-model.gguf

[my-custom-draft]
model = main-model.gguf
model-draft = draft-model.gguf
"#,
    )
    .unwrap();

    let presets = vec![
        ("my-custom-base".to_string(), model_path.clone()),
        ("my-custom-draft".to_string(), model_path.clone()),
    ];

    let mut state = AppState::new(
        presets,
        models_dir,
        preset_path,
        HashMap::new(),
        PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    // 1. load_current_preset_settings populates custom_names from [llama-herd.custom-name]
    assert_eq!(
        state.custom_names.get("base").map(String::as_str),
        Some("my-custom-base")
    );
    assert_eq!(
        state.custom_names.get("draft").map(String::as_str),
        Some("my-custom-draft")
    );
    assert_eq!(state.current_variant_key(), "base");
    assert_eq!(state.current_custom_name(), "my-custom-base");
    // GroupedModel base_name in left panel must always be the clean model ID:
    assert_eq!(state.grouped_models[0].base_name, "main-model");

    // 2. Cycling variants changes current_custom_name()
    state.cycle_variant_next();
    assert_eq!(state.current_variant_key(), "draft");
    assert_eq!(state.current_custom_name(), "my-custom-draft");

    // 3. Modifying custom name sets unsaved changes and save_current_preset_config writes it
    state.set_current_custom_name("new-draft-name".to_string());
    assert_eq!(state.current_custom_name(), "new-draft-name");
    assert!(state.has_unsaved_changes());

    state.save_current_preset_config(false).unwrap();

    // Verify file content: inactive variant (base) is preserved, active (draft) is updated, is-default removed
    let saved_toml = fs::read_to_string(&model_config_path).unwrap();
    let parsed: toml::Value = toml::from_str(&saved_toml).unwrap();
    let herd = parsed
        .get("llama-herd")
        .expect("llama-herd section must exist");
    assert!(
        herd.get("is-default").is_none(),
        "legacy is-default must be removed"
    );
    let custom_name_table = herd
        .get("custom-name")
        .expect("custom-name section must exist");
    assert_eq!(
        custom_name_table.get("base").and_then(|v| v.as_str()),
        Some("my-custom-base")
    );
    assert_eq!(
        custom_name_table.get("draft").and_then(|v| v.as_str()),
        Some("new-draft-name")
    );
}

#[test]
fn test_app_state_hidden_model_toggle_and_rebuild() {
    let temp_dir = tempfile::tempdir().unwrap();
    let models_dir = temp_dir.path();
    let gguf1 = models_dir.join("visible-model.gguf");
    let gguf2 = models_dir.join("hidden-model.gguf");
    let toml2 = models_dir.join("hidden-model.toml");

    std::fs::write(&gguf1, b"dummy").unwrap();
    std::fs::write(&gguf2, b"dummy").unwrap();
    std::fs::write(&toml2, b"[llama-herd]\nhidden = true\n").unwrap();

    let ini_path = models_dir.join("models-preset.ini");
    let global_config = std::collections::HashMap::new();
    let _ =
        llama_herd::discovery::generate_presets_ini(models_dir, &ini_path, &global_config).unwrap();
    let presets = llama_herd::discovery::discover_presets_from_ini(&ini_path);

    let mut state = llama_herd::tui::app::AppState::new(
        presets,
        models_dir.to_path_buf(),
        ini_path,
        global_config,
        std::path::PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    // Default: show_hidden is false, only visible model listed
    assert_eq!(state.grouped_models.len(), 1);
    assert_eq!(state.grouped_models[0].is_hidden, false);

    // Toggle show_hidden to true
    state.toggle_show_hidden();
    assert_eq!(state.show_hidden, true);
    assert_eq!(state.grouped_models.len(), 2);
    let hidden_idx = state
        .grouped_models
        .iter()
        .position(|m| m.is_hidden)
        .unwrap();
    assert_eq!(state.grouped_models[hidden_idx].base_name, "hidden-model");

    // Toggle back to false
    state.toggle_show_hidden();
    assert_eq!(state.show_hidden, false);
    assert_eq!(state.grouped_models.len(), 1);
}

#[test]
fn test_app_state_hidden_save_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let models_dir = temp_dir.path();
    let gguf = models_dir.join("test-model.gguf");
    let toml = models_dir.join("test-model.toml");

    std::fs::write(&gguf, b"dummy").unwrap();
    std::fs::write(&toml, b"[llama-herd]\nhidden = false\n").unwrap();

    let ini_path = models_dir.join("models-preset.ini");
    let global_config = std::collections::HashMap::new();
    let _ =
        llama_herd::discovery::generate_presets_ini(models_dir, &ini_path, &global_config).unwrap();
    let presets = llama_herd::discovery::discover_presets_from_ini(&ini_path);

    let mut state = llama_herd::tui::app::AppState::new(
        presets,
        models_dir.to_path_buf(),
        ini_path,
        global_config,
        std::path::PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.hidden, false);
    assert_eq!(state.original_hidden, false);
    assert!(!state.has_unsaved_changes());

    // Toggle hidden property in state
    state.hidden = true;
    assert!(state.has_unsaved_changes());

    // Save configuration
    state.save_current_preset_config(false).unwrap();

    let saved_toml = std::fs::read_to_string(&toml).unwrap();
    let parsed: toml::Value = toml::from_str(&saved_toml).unwrap();
    let herd = parsed
        .get("llama-herd")
        .expect("llama-herd section must exist");
    assert_eq!(herd.get("hidden").and_then(|v| v.as_bool()), Some(true));

    assert_eq!(state.hidden, true);
    assert_eq!(state.original_hidden, true);
    assert!(!state.has_unsaved_changes());
}

#[test]
fn test_load_current_preset_settings_precedence_and_override() {
    let temp_dir = tempfile::tempdir().unwrap();
    let models_dir = temp_dir.path();
    let model_file = models_dir.join("main-model.gguf");
    let default_toml = models_dir.join("main-model.toml");
    let override_toml = models_dir.join("custom-config.toml");

    std::fs::write(&model_file, b"dummy").unwrap();
    std::fs::write(
        &default_toml,
        b"[llama-herd]\ntotal-layers = 32\n[llama-server-long]\nctx-size = 4096\nngl = 16\n",
    )
    .unwrap();
    std::fs::write(
        &override_toml,
        b"[llama-herd]\ntotal-layers = 48\n[llama-server-long]\nctx-size = 8192\nngl = 64\n",
    )
    .unwrap();

    let ini_path = models_dir.join("models-preset.ini");
    std::fs::write(
        &ini_path,
        r#"
[main-model]
model = main-model.gguf
n-gpu-layers = 32
"#,
    )
    .unwrap();

    let presets = vec![("main-model".to_string(), model_file.clone())];
    let mut state = llama_herd::tui::app::AppState::new(
        presets,
        models_dir.to_path_buf(),
        ini_path.clone(),
        std::collections::HashMap::new(),
        std::path::PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    assert_eq!(state.grouped_models.len(), 1);
    assert_eq!(state.grouped_models[0].model_path, model_file);
    assert_eq!(state.ctx, 4096);
    assert_eq!(state.total_layers, Some(32));
    assert_eq!(state.ngl, "32");

    // Call load_current_preset_settings with toml_path_override
    state.load_current_preset_settings(Some(override_toml.clone()));

    // Verify: model_path and preset_name were still resolved from grouped_models[selected_model_index]
    // so ini_settings for "main-model" (n-gpu-layers = 32) are retained,
    // while assets.config was overridden by override_toml (ctx-size 8192, total-layers 48)
    assert_eq!(state.ngl, "32");
    assert_eq!(state.ctx, 8192);
    assert_eq!(state.total_layers, Some(48));
    assert_eq!(state.config_file_name, "custom-config.toml");

    // Now test fallback when both grouped_models and presets are empty
    let mut empty_state = llama_herd::tui::app::AppState::new(
        vec![],
        models_dir.to_path_buf(),
        ini_path,
        std::collections::HashMap::new(),
        std::path::PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );
    assert!(empty_state.grouped_models.is_empty());
    assert!(empty_state.presets.is_empty());

    // Call load_current_preset_settings with override path on empty state
    empty_state.load_current_preset_settings(Some(override_toml));
    assert_eq!(empty_state.ctx, 8192);
    assert_eq!(empty_state.total_layers, Some(48));
    assert_eq!(empty_state.config_file_name, "custom-config.toml");
}

#[test]
fn test_rebuild_grouped_models_hidden_path_matching() {
    let temp_dir = tempfile::tempdir().unwrap();
    let models_dir = temp_dir.path();
    let model_file = models_dir.join("special-model.gguf");
    let toml_file = models_dir.join("special-model.toml");

    std::fs::write(&model_file, b"dummy").unwrap();
    std::fs::write(&toml_file, b"[llama-herd]\nhidden = true\n").unwrap();

    let ini_path = models_dir.join("models-preset.ini");
    std::fs::write(
        &ini_path,
        r#"
[special-model]
model = special-model.gguf
"#,
    )
    .unwrap();

    let presets = vec![("special-model".to_string(), model_file.clone())];
    let mut state = llama_herd::tui::app::AppState::new(
        presets,
        models_dir.to_path_buf(),
        ini_path,
        std::collections::HashMap::new(),
        std::path::PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    // Toggle show_hidden to discover hidden models and mark existing entries
    state.toggle_show_hidden();
    assert_eq!(state.grouped_models.len(), 1);
    assert!(state.grouped_models[0].is_hidden);
    assert_eq!(
        state.grouped_models[0].model_path.file_name(),
        model_file.file_name()
    );
}

#[test]
fn test_current_effective_model_name_and_user_settings() {
    let temp_dir = tempfile::tempdir().unwrap();
    let models_dir = temp_dir.path();
    let model_file = models_dir.join("test-model.gguf");
    let ini_path = models_dir.join("models-preset.ini");

    std::fs::write(&model_file, b"dummy").unwrap();
    std::fs::write(
        &ini_path,
        r#"
[test-model]
model = test-model.gguf
"#,
    )
    .unwrap();

    let presets = vec![("test-model".to_string(), model_file.clone())];
    let mut state = llama_herd::tui::app::AppState::new(
        presets,
        models_dir.to_path_buf(),
        ini_path,
        std::collections::HashMap::new(),
        std::path::PathBuf::from("llama-server"),
        llama_herd::tui::theme::Theme::default(),
    );

    // 1. Without custom name, effective name is auto generated preset name
    assert_eq!(state.current_effective_model_name(), "test-model");
    let settings = state.get_user_settings();
    assert_eq!(settings.model_alias, Some("test-model".to_string()));

    // 2. With custom name set, effective name is the custom name
    state.set_current_custom_name("My Custom Display Name".to_string());
    assert_eq!(
        state.current_effective_model_name(),
        "My Custom Display Name"
    );
    let settings2 = state.get_user_settings();
    assert_eq!(
        settings2.model_alias,
        Some("My Custom Display Name".to_string())
    );
}
