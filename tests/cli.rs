use llama_herd::cli::prompt_preset_selection_internal;
use std::io::Cursor;
use std::path::PathBuf;

#[test]
fn test_prompt_preset_selection_valid() {
    let presets = vec![
        ("model1".to_string(), PathBuf::from("m1.gguf")),
        ("model2".to_string(), PathBuf::from("m2.gguf")),
    ];
    let mut input = Cursor::new("2\n");
    let mut output = Vec::new();

    let (name, path) = prompt_preset_selection_internal(&presets, &mut input, &mut output);
    assert_eq!(name, "model2");
    assert_eq!(path, PathBuf::from("m2.gguf"));
}

#[test]
fn test_prompt_preset_selection_default() {
    let presets = vec![("model1".to_string(), PathBuf::from("m1.gguf"))];
    let mut input = Cursor::new("\n"); // Just press enter
    let mut output = Vec::new();

    let (name, _) = prompt_preset_selection_internal(&presets, &mut input, &mut output);
    assert_eq!(name, "model1");
}

#[test]
fn test_prompt_preset_selection_invalid_then_default() {
    let presets = vec![("model1".to_string(), PathBuf::from("m1.gguf"))];
    let mut input = Cursor::new("999\n"); // Out of bounds
    let mut output = Vec::new();

    let (name, _) = prompt_preset_selection_internal(&presets, &mut input, &mut output);
    assert_eq!(name, "model1");
}

#[test]
fn test_prompt_custom_settings_full_flow() {
    use llama_herd::cli::prompt_custom_settings_internal;
    use llama_herd::config::{ModelAssets, UserSettings};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    // Mock Main Model
    let main_model = models_dir.join("main.gguf");
    fs::write(&main_model, "dummy").unwrap();
    let mut config_data = HashMap::new();
    config_data.insert(
        "total-layers".to_string(),
        serde_json::Value::Number(serde_json::Number::from(32)),
    );
    let assets = ModelAssets {
        config: config_data,
        jinja_template: None,
    };

    // Mock Vision Module
    let vision_module = models_dir.join("vision-mmproj.gguf");
    fs::write(&vision_module, "dummy").unwrap();

    // Mock Draft Model
    let draft_model = models_dir.join("draft.gguf");
    fs::write(&draft_model, "dummy").unwrap();
    let draft_config = models_dir.join("draft.toml");
    fs::write(&draft_config, "is-draft = true\ntotal-layers = 4\n").unwrap();

    let default_settings = UserSettings {
        ctx: 131072,
        ngl: "auto".to_string(),
        ui: true,
        mmproj: None,
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    // Simulate User Input:
    // 1. Select vision module (index 1)
    // 2. Select MTP draft model (index 1)
    // 3. Draft GPU Layers (--4)
    // 4. GPU Layers (24)
    // 5. Context size (8k)
    // 6. Enable UI? (n)
    let input_str = "1\n1\n--4\n24\n8k\nn\n";
    let mut input = Cursor::new(input_str);
    let mut output = Vec::new();

    let result = prompt_custom_settings_internal(
        default_settings,
        &assets,
        &models_dir,
        &main_model,
        &mut input,
        &mut output,
    );

    assert_eq!(result.mmproj, Some(vision_module));
    assert_eq!(result.draft_model, Some(draft_model));
    assert_eq!(result.draft_ngl, "0"); // total-layers = 4, input = --4 -> saturating_sub = 0
    assert_eq!(result.ngl, "24");
    assert_eq!(result.ctx, 8192);
    assert!(!result.ui);
}

#[test]
fn test_prompt_custom_settings_defaults() {
    use llama_herd::cli::prompt_custom_settings_internal;
    use llama_herd::config::{ModelAssets, UserSettings};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir).unwrap();

    let main_model = models_dir.join("main.gguf");
    let assets = ModelAssets {
        config: HashMap::new(),
        jinja_template: None,
    };

    let default_settings = UserSettings {
        ctx: 4096,
        ngl: "99".to_string(),
        ui: false,
        mmproj: None,
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    // Simulate User Input: Just enter on all to accept defaults
    // Prompts: NGL, Context, UI (No vision/draft available)
    let input_str = "\n\n\n";
    let mut input = Cursor::new(input_str);
    let mut output = Vec::new();

    let result = prompt_custom_settings_internal(
        default_settings,
        &assets,
        &models_dir,
        &main_model,
        &mut input,
        &mut output,
    );

    assert_eq!(result.mmproj, None);
    assert_eq!(result.draft_model, None);
    assert_eq!(result.ngl, "99");
    assert_eq!(result.ctx, 4096);
    assert!(!result.ui); // Keeps original state if empty input
}
