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
    assert_eq!(grouped.base_name, "test-preset");
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
    assert_eq!(state.grouped_models[0].base_name, "model1-preset");
    assert_eq!(state.grouped_models[0].variants.len(), 2);
    assert_eq!(state.grouped_models[1].base_name, "model2-preset");
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
[default]
model = gemma-12b.gguf

[gemma-12b]
model = gemma-12b.gguf

[gemma-12b-draft]
model = gemma-12b.gguf
model-draft = draft.gguf
"#,
    )
    .unwrap();

    let presets = vec![
        ("default".to_string(), model_path.clone()),
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

    assert_eq!(state.grouped_models.len(), 2);
    assert_eq!(state.grouped_models[0].base_name, "default");
    assert_eq!(state.grouped_models[0].variants.len(), 1);
    assert_eq!(state.grouped_models[0].variants[0].preset_name, "default");

    assert_eq!(state.grouped_models[1].base_name, "gemma-12b");
    assert_eq!(state.grouped_models[1].variants.len(), 2);
    assert!(
        !state.grouped_models[1]
            .variants
            .iter()
            .any(|v| v.preset_name == "default")
    );
}
