use llama_herd::discovery::{
    clean_model_id, discover_presets_from_ini, find_matching_draft, find_matching_mmproj,
    generate_presets_ini, insert_variant_suffix,
};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_clean_model_id_formatting() -> TestResult {
    // Given various raw GGUF filenames (with dots, version names, size markers)
    // When clean_model_id is called
    // Then the names are simplified, dots replaced with dashes, and multiple hyphens collapsed

    // 1. File path with dots
    assert_eq!(
        clean_model_id(&PathBuf::from("/models/gemma-2.2b.it.gguf")),
        "gemma-2-2b-it"
    );

    // 2. File path with version size markers needing spacing
    // e.g. "mistral7b" -> "mistral-7b"
    assert_eq!(
        clean_model_id(&PathBuf::from("mistral7b.gguf")),
        "mistral-7b"
    );
    assert_eq!(
        clean_model_id(&PathBuf::from("mistral8b-instruct.gguf")),
        "mistral-8b-instruct"
    );

    // 3. Consecutive hyphens collapsed
    assert_eq!(
        clean_model_id(&PathBuf::from("my--model---name.gguf")),
        "my-model-name"
    );

    Ok(())
}

#[test]
fn test_insert_variant_suffix_logic() -> TestResult {
    // Given a cleaned model identifier and a variant suffix
    // When insert_variant_suffix is invoked
    // Then the suffix is injected right before the final component (e.g. before "-it" or "-instruct")

    // 1. Typical suffix replacement (e.g. insert "vision" before "-it")
    assert_eq!(
        insert_variant_suffix("gemma-2-9b-it", "vision"),
        "gemma-2-9b-vision-it"
    );

    // 2. Custom helper suffix (e.g. insert "draft" before "-instruct")
    assert_eq!(
        insert_variant_suffix("llama-3-8b-instruct", "draft"),
        "llama-3-8b-draft-instruct"
    );

    Ok(())
}

#[test]
fn test_find_matching_mmproj_heuristics() -> TestResult {
    // Given a main model and multiple candidate mmproj vision project files
    // When find_matching_mmproj is called
    // Then it returns the single available mmproj file or the token-matching one

    let mmproj_a = PathBuf::from("gemma-mmproj.gguf");
    let mmproj_b = PathBuf::from("llama-mmproj.gguf");
    let mmproj_files = vec![mmproj_a.clone(), mmproj_b.clone()];

    // 1. Standard token matching: "gemma-2-9b-it" should pair with "gemma-mmproj"
    let matched = find_matching_mmproj(&PathBuf::from("gemma-2-9b-it.gguf"), &mmproj_files);
    assert_eq!(matched, Some(mmproj_a));

    // 2. Standalone fallback: when only 1 mmproj file is present in the directory
    let single_list = vec![mmproj_b.clone()];
    let matched_single = find_matching_mmproj(&PathBuf::from("gemma-2-9b-it.gguf"), &single_list);
    assert_eq!(matched_single, Some(mmproj_b));

    // 3. Empty list returns None
    let empty_list = vec![];
    let matched_none = find_matching_mmproj(&PathBuf::from("gemma-2-9b-it.gguf"), &empty_list);
    assert_eq!(matched_none, None);

    Ok(())
}

#[test]
fn test_find_matching_draft_heuristics() -> TestResult {
    // Given a main model path and multiple draft model options
    // When find_matching_draft is called
    // Then helper tags (assistant, draft, mtp, etc.) are ignored and correct matching draft path is selected

    let draft_a = PathBuf::from("gemma-2-draft.gguf");
    let draft_b = PathBuf::from("llama-3-8b-assistant.gguf");
    let drafts = vec![draft_a.clone(), draft_b.clone()];

    // 1. Match based on remaining tokens: "gemma-2-9b-it" matches "gemma-2-draft"
    let matched = find_matching_draft(&PathBuf::from("gemma-2-9b-it.gguf"), &drafts);
    assert_eq!(matched, Some(draft_a));

    // 2. Unmatched returns None
    let matched_none = find_matching_draft(&PathBuf::from("mistral-7b.gguf"), &drafts);
    assert_eq!(matched_none, None);

    Ok(())
}

#[test]
fn test_discover_presets_from_ini_filtering() -> TestResult {
    // Given an INI configuration containing active models, draft configurations, and global section
    // When calling discover_presets_from_ini
    // Then presets are read and returned alphabetically, excluding draft-only presets and the global "*" section

    let dir = tempdir()?;
    let path = dir.path().join("models-preset.ini");

    let content = r#"
        [*]
        flash-attn = auto

        [gemma-2-9b]
        model = models/gemma-2-9b.gguf

        [gemma-2-draft]
        model = models/gemma-2-draft.gguf
        is-draft = true

        [llama-3-8b]
        model = models/llama-3-8b.gguf
    "#;

    File::create(&path)?.write_all(content.as_bytes())?;

    let presets = discover_presets_from_ini(&path);
    assert_eq!(presets.len(), 2);

    // Verify ordering is alphabetical and excludes draft and *
    assert_eq!(presets[0].0, "gemma-2-9b");
    assert_eq!(presets[0].1, PathBuf::from("models/gemma-2-9b.gguf"));
    assert_eq!(presets[1].0, "llama-3-8b");
    assert_eq!(presets[1].1, PathBuf::from("models/llama-3-8b.gguf"));

    // File missing check
    let missing_path = dir.path().join("missing-presets.ini");
    assert!(discover_presets_from_ini(&missing_path).is_empty());

    Ok(())
}

#[test]
fn test_generate_presets_ini_generation() -> TestResult {
    // Given a mock models folder structure containing GGUFs, custom TOML configs, draft models, and mmproj files
    // When generating a presets INI file using generate_presets_ini
    // Then a correct models-preset.ini file is generated containing merged parameters, formatted keys, draft and vision mappings

    let dir = tempdir()?;
    let models_dir = dir.path().join("models");
    fs::create_dir(&models_dir)?;

    // 1. Create main model
    let main_model_path = models_dir.join("gemma-2-9b-it.gguf");
    fs::write(&main_model_path, "main model binary content")?;

    // Create main model TOML config
    let main_toml_path = models_dir.join("gemma-2-9b-it.toml");
    let main_toml_content = r#"
        is-default = true
        ctx-size = "8k"
        total-layers = 42
        temp = 0.7
        lh-spec-type = "mtp"
        
        # Passthrough keys
        s-sps = 0.85
        slot-prompt-similarity = 0.9
    "#;
    fs::write(&main_toml_path, main_toml_content)?;

    // 2. Create draft model
    let draft_model_path = models_dir.join("gemma-draft.gguf");
    fs::write(&draft_model_path, "draft model binary content")?;

    // Create draft model TOML config specifying it is a draft
    let draft_toml_path = models_dir.join("gemma-draft.toml");
    let draft_toml_content = r#"
        is-draft = true
        total-layers = 8
        spec-type = "mtp"
        spec-draft-n-max = 5
    "#;
    fs::write(&draft_toml_path, draft_toml_content)?;

    // 3. Create mmproj vision model
    let mmproj_model_path = models_dir.join("gemma-mmproj.gguf");
    fs::write(&mmproj_model_path, "vision project binary content")?;

    // 4. Generate the preset
    let mut global_config = HashMap::new();
    global_config.insert("kv_quant".to_string(), serde_json::json!("q4_k"));

    let output_ini = generate_presets_ini(
        &models_dir,
        &dir.path().join("models-preset.ini"),
        &global_config,
    );
    assert!(output_ini.exists());

    let ini_content = fs::read_to_string(&output_ini)?;

    // Verify sections and values
    // Must contain global section
    assert!(ini_content.contains("[*]"));
    assert!(ini_content.contains("cache-type-k = q4_k"));
    assert!(ini_content.contains("cache-type-v = q4_k"));

    // Must contain the [default] section since it was marked is-default
    assert!(ini_content.contains("[default]"));

    // Must contain the clean preset section
    assert!(ini_content.contains("[gemma-2-9b-it]"));

    // Must contain the generated draft-vision variations because both mmproj and draft exist
    assert!(ini_content.contains("[gemma-2-9b-vision-it]"));
    assert!(ini_content.contains("[gemma-2-9b-draft-it]"));
    assert!(ini_content.contains("[gemma-2-9b-draft-vision-it]"));

    // Verify properties mapped to INI format
    assert!(ini_content.contains("ctx-size = 8192")); // parsed "8k" -> 8192
    assert!(ini_content.contains("n-gpu-layers = 42")); // total-layers used as fallback for auto NGL
    assert!(ini_content.contains("temp = 0.7"));

    // Verify passthrough formatting
    assert!(ini_content.contains("sps = 0.85")); // s-sps -> sps
    assert!(ini_content.contains("slot-prompt-similarity = 0.9"));

    // Verify mmproj vision path mapping
    assert!(ini_content.contains("mmproj ="));

    // Verify draft speculative decoding mapping
    assert!(ini_content.contains("model-draft ="));
    assert!(ini_content.contains("spec-type = draft-mtp")); // mtp mapped to draft-mtp
    assert!(ini_content.contains("gpu-layers-draft = 8"));
    assert!(ini_content.contains("spec-draft-n-max = 5"));

    Ok(())
}

#[test]
fn test_generate_presets_hyphenated_keys() {
    let dir = tempfile::tempdir().unwrap();
    let models_dir = dir.path().join("models");
    std::fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("test-model.gguf");
    std::fs::write(&model_path, "dummy").unwrap();

    let config_path = models_dir.join("test-model.toml");
    std::fs::write(
        &config_path,
        "is-default = true\nctx-size = \"8k\"\ntotal-layers = 32\n",
    )
    .unwrap();

    let global_config = std::collections::HashMap::new();
    let output_path = generate_presets_ini(
        &models_dir,
        &dir.path().join("models-preset.ini"),
        &global_config,
    );

    assert!(output_path.exists());
    let content = std::fs::read_to_string(output_path).unwrap();
    assert!(content.contains("[default]"));
    assert!(content.contains("ctx-size = 8192"));
    assert!(content.contains("n-gpu-layers = 32"));
}

#[test]
fn test_generate_presets_draft_hyphenated_keys() {
    let dir = tempfile::tempdir().unwrap();
    let models_dir = dir.path().join("models");
    std::fs::create_dir(&models_dir).unwrap();

    let model_path = models_dir.join("main-model.gguf");
    std::fs::write(&model_path, "dummy").unwrap();

    let draft_path = models_dir.join("draft-model.gguf");
    std::fs::write(&draft_path, "dummy").unwrap();

    let draft_config_path = models_dir.join("draft-model.toml");
    std::fs::write(
        &draft_config_path,
        "is-draft = true\ntotal-layers = 4\nspec-type = \"mtp\"\n",
    )
    .unwrap();

    let global_config = std::collections::HashMap::new();
    let output_path = generate_presets_ini(
        &models_dir,
        &dir.path().join("models-preset.ini"),
        &global_config,
    );

    let content = std::fs::read_to_string(output_path).unwrap();
    assert!(content.contains("model-draft ="));
    assert!(content.contains("spec-type = draft-mtp"));
    assert!(content.contains("gpu-layers-draft = 4"));
}
