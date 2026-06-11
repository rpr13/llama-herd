#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

use llama_herd::config::{ModelAssets, UserSettings};
use llama_herd::launcher::{build_launch_parameters, build_router_launch_parameters};
use std::collections::HashMap;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_build_launch_parameters_defaults() -> TestResult {
    // Given default setup parameters for a single model launch
    // When calling build_launch_parameters
    // Then correct default CLI options (host, port, ngl, threads) are produced for llama-server

    let exe_path = PathBuf::from("/bin/llama-server");
    let model_path = PathBuf::from("/models/gemma-2b.gguf");

    let assets = ModelAssets {
        config: HashMap::new(),
        jinja_template: None,
    };

    let settings = UserSettings {
        ctx: 2048,
        ngl: "32".to_string(),
        mmproj: None,
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    let global_config = HashMap::new();

    let params = build_launch_parameters(
        &exe_path,
        &model_path,
        &assets,
        &settings,
        &global_config,
        8080,
    );

    // Verify binary and model paths
    assert_eq!(params[0], "/bin/llama-server");

    // Model option
    let m_idx = params.iter().position(|r| r == "-m").unwrap();
    assert_eq!(params[m_idx + 1], "/models/gemma-2b.gguf");

    // Default host and port
    let host_idx = params.iter().position(|r| r == "--host").unwrap();
    assert_eq!(params[host_idx + 1], "127.0.0.1");
    let port_idx = params.iter().position(|r| r == "--port").unwrap();
    assert_eq!(params[port_idx + 1], "8080");

    // NGL and context size
    let ngl_idx = params.iter().position(|r| r == "-ngl").unwrap();
    assert_eq!(params[ngl_idx + 1], "32");
    let ctx_idx = params.iter().position(|r| r == "--ctx-size").unwrap();
    assert_eq!(params[ctx_idx + 1], "2048");

    // Unified KV cache is enabled
    assert!(params.contains(&"--kv-unified".to_string()));

    // Log verbosity default
    let lv_idx = params.iter().position(|r| r == "--log-verbosity").unwrap();
    assert_eq!(params[lv_idx + 1], "3");

    // Cache quantization options
    let ctk_idx = params.iter().position(|r| r == "-ctk").unwrap();
    assert_eq!(params[ctk_idx + 1], "f16");

    // UI is enabled by default, so --no-ui should not be present
    assert!(!params.contains(&"--no-ui".to_string()));

    Ok(())
}

#[test]
fn test_build_launch_parameters_overrides() -> TestResult {
    // Given custom overrides, draft models, mmproj, and jinja templates
    // When calling build_launch_parameters
    // Then all corresponding option flags are populated correctly in the launcher arguments

    let exe_path = PathBuf::from("/bin/llama-server");
    let model_path = PathBuf::from("/models/gemma-2b.gguf");

    // 1. Setup custom config assets
    let mut config_data = HashMap::new();
    let mut short_map = serde_json::Map::new();
    short_map.insert(
        "sps".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(0.8).unwrap()),
    );
    config_data.insert(
        "llama-server-short".to_string(),
        serde_json::Value::Object(short_map),
    );

    config_data.insert(
        "slot-prompt-similarity".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(0.95).unwrap()),
    );

    let mut herd_map = serde_json::Map::new();
    herd_map.insert("custom-ignored".to_string(), serde_json::json!(true));
    config_data.insert(
        "llama-herd".to_string(),
        serde_json::Value::Object(herd_map),
    );

    let assets = ModelAssets {
        config: config_data,
        jinja_template: Some(PathBuf::from("/models/gemma.jinja")),
    };

    // 2. Setup user settings including draft model, mmproj
    let settings = UserSettings {
        ctx: 8192,
        ngl: "auto".to_string(),
        mmproj: Some(PathBuf::from("/models/mmproj.gguf")),
        draft_model: Some(PathBuf::from("/models/gemma-draft.gguf")),
        draft_ngl: "16".to_string(),
    };

    // 3. Setup global overrides
    let mut global_config = HashMap::new();
    global_config.insert("ui".to_string(), serde_json::json!(false));
    global_config.insert("host".to_string(), serde_json::json!("127.0.0.1"));
    global_config.insert("port".to_string(), serde_json::json!(9000));
    global_config.insert("kv-quant".to_string(), serde_json::json!("q4_0"));
    global_config.insert("batch-size".to_string(), serde_json::json!(512));
    global_config.insert("ubatch-size".to_string(), serde_json::json!(256));
    global_config.insert("tools".to_string(), serde_json::json!("web-search"));

    let params = build_launch_parameters(
        &exe_path,
        &model_path,
        &assets,
        &settings,
        &global_config,
        9000,
    );

    // Verify overrides
    let host_idx = params.iter().position(|r| r == "--host").unwrap();
    assert_eq!(params[host_idx + 1], "127.0.0.1");
    let port_idx = params.iter().position(|r| r == "--port").unwrap();
    assert_eq!(params[port_idx + 1], "9000");

    // UI disabled
    assert!(params.contains(&"--no-ui".to_string()));

    // Cache quantization override
    let ctk_idx = params.iter().position(|r| r == "-ctk").unwrap();
    assert_eq!(params[ctk_idx + 1], "q4_0");

    // Batch sizes & tools
    let b_idx = params.iter().position(|r| r == "-b").unwrap();
    assert_eq!(params[b_idx + 1], "512");
    let ub_idx = params.iter().position(|r| r == "-ub").unwrap();
    assert_eq!(params[ub_idx + 1], "256");
    let tools_idx = params.iter().position(|r| r == "--tools").unwrap();
    assert_eq!(params[tools_idx + 1], "web-search");

    // Custom key passthrough formatting
    let sps_idx = params.iter().position(|r| r == "-sps").unwrap();
    assert_eq!(params[sps_idx + 1], "0.8");
    let l_sps_idx = params
        .iter()
        .position(|r| r == "--slot-prompt-similarity")
        .unwrap();
    assert_eq!(params[l_sps_idx + 1], "0.95");

    // Verify custom "llama-herd" keys are NOT passed directly to llama-server
    assert!(!params.contains(&"--custom-ignored".to_string()));

    // Draft model arguments (-md and -ngld)
    let md_idx = params.iter().position(|r| r == "-md").unwrap();
    assert_eq!(params[md_idx + 1], "/models/gemma-draft.gguf");
    let ngld_idx = params.iter().position(|r| r == "-ngld").unwrap();
    assert_eq!(params[ngld_idx + 1], "16");

    // MMProj arguments
    let mmproj_idx = params.iter().position(|r| r == "--mmproj").unwrap();
    assert_eq!(params[mmproj_idx + 1], "/models/mmproj.gguf");

    // Jinja template arguments
    assert!(params.contains(&"--jinja".to_string()));
    let templ_idx = params
        .iter()
        .position(|r| r == "--chat-template-file")
        .unwrap();
    assert_eq!(params[templ_idx + 1], "/models/gemma.jinja");

    Ok(())
}

#[test]
fn test_build_launch_parameters_speculative_types() -> TestResult {
    let exe_path = PathBuf::from("llama-server");
    let model_path = PathBuf::from("model.gguf");

    let mut config_data = HashMap::new();
    let mut long_map = serde_json::Map::new();
    long_map.insert("spec-type".to_string(), serde_json::json!("draft-eagle3"));
    config_data.insert(
        "llama-server-long".to_string(),
        serde_json::Value::Object(long_map),
    );

    let assets = ModelAssets {
        config: config_data,
        jinja_template: None,
    };
    let settings = UserSettings {
        ctx: 2048,
        ngl: "0".to_string(),
        mmproj: None,
        draft_model: Some(PathBuf::from("draft.gguf")),
        draft_ngl: "0".to_string(),
    };

    let params = build_launch_parameters(
        &exe_path,
        &model_path,
        &assets,
        &settings,
        &HashMap::new(),
        8080,
    );

    let spec_idx = params.iter().position(|r| r == "--spec-type").unwrap();
    assert_eq!(params[spec_idx + 1], "draft-eagle3");

    Ok(())
}

#[test]
fn test_build_router_launch_parameters_logic() -> TestResult {
    // Given parameters for Router Mode loading
    // When calling build_router_launch_parameters
    // Then correct options like --models-preset, --models-max, and binding arguments are compiled

    let exe_path = PathBuf::from("/bin/llama-server");
    let preset_path = PathBuf::from("/base/models-preset.ini");

    let mut global_config = HashMap::new();
    global_config.insert("host".to_string(), serde_json::json!("0.0.0.0"));
    global_config.insert("port".to_string(), serde_json::json!(8000));
    global_config.insert("models-max".to_string(), serde_json::json!(3));
    global_config.insert("ui".to_string(), serde_json::json!(false));

    let params = build_router_launch_parameters(&exe_path, &preset_path, &global_config, 8000);

    // Verify base arguments
    assert_eq!(params[0], "/bin/llama-server");

    // Models preset INI path
    let pr_idx = params.iter().position(|r| r == "--models-preset").unwrap();
    assert_eq!(params[pr_idx + 1], "/base/models-preset.ini");

    // Max active models limit
    let max_idx = params.iter().position(|r| r == "--models-max").unwrap();
    assert_eq!(params[max_idx + 1], "3");

    // Binding overrides
    let host_idx = params.iter().position(|r| r == "--host").unwrap();
    assert_eq!(params[host_idx + 1], "0.0.0.0");
    let port_idx = params.iter().position(|r| r == "--port").unwrap();
    assert_eq!(params[port_idx + 1], "8000");

    // UI disable toggle mapped correctly
    assert!(params.contains(&"--no-ui".to_string()));

    Ok(())
}

#[test]
fn test_build_launch_parameters_draft_fallback_loading() -> TestResult {
    let dir = tempfile::tempdir()?;
    let models_dir = dir.path().join("models");
    std::fs::create_dir(&models_dir)?;

    let model_path = models_dir.join("main-model.gguf");
    std::fs::write(&model_path, "dummy").unwrap();

    let draft_path = models_dir.join("draft-model.gguf");
    std::fs::write(&draft_path, "dummy").unwrap();

    let draft_config_path = models_dir.join("draft-model.toml");
    std::fs::write(
        &draft_config_path,
        r#"
[llama-server-long]
spec-type = "draft-mtp"
spec-draft-n-max = 6
spec-draft-p-min = 0.85
"#,
    )
    .unwrap();

    let assets = ModelAssets {
        config: HashMap::new(),
        jinja_template: None,
    };

    let settings = UserSettings {
        ctx: 2048,
        ngl: "0".to_string(),
        mmproj: None,
        draft_model: Some(draft_path),
        draft_ngl: "0".to_string(),
    };

    let params = build_launch_parameters(
        &PathBuf::from("llama-server"),
        &model_path,
        &assets,
        &settings,
        &HashMap::new(),
        8080,
    );

    let spec_type_idx = params.iter().position(|r| r == "--spec-type").unwrap();
    assert_eq!(params[spec_type_idx + 1], "draft-mtp");

    let n_max_idx = params
        .iter()
        .position(|r| r == "--spec-draft-n-max")
        .unwrap();
    assert_eq!(params[n_max_idx + 1], "6");

    let p_min_idx = params
        .iter()
        .position(|r| r == "--spec-draft-p-min")
        .unwrap();
    assert_eq!(params[p_min_idx + 1], "0.85");

    Ok(())
}

#[test]
fn test_build_launch_parameters_new_rich_settings() -> TestResult {
    let exe_path = PathBuf::from("/bin/llama-server");
    let model_path = PathBuf::from("/models/gemma-2b.gguf");

    let assets = ModelAssets {
        config: HashMap::new(),
        jinja_template: None,
    };

    let settings = UserSettings {
        ctx: 2048,
        ngl: "32".to_string(),
        mmproj: None,
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    let mut global_config = HashMap::new();
    global_config.insert("cache-type-k".to_string(), serde_json::json!("q4_0"));
    global_config.insert("cache-type-v".to_string(), serde_json::json!("q4_1"));
    global_config.insert("kv-unified".to_string(), serde_json::json!(false));
    global_config.insert("api-key".to_string(), serde_json::json!("secret-token-123"));
    global_config.insert("metrics".to_string(), serde_json::json!(true));
    global_config.insert("log-verbosity".to_string(), serde_json::json!(5));

    let params = build_launch_parameters(
        &exe_path,
        &model_path,
        &assets,
        &settings,
        &global_config,
        8080,
    );

    // Verify cache type options are separate
    let ctk_idx = params.iter().position(|r| r == "-ctk").unwrap();
    assert_eq!(params[ctk_idx + 1], "q4_0");

    let ctv_idx = params.iter().position(|r| r == "-ctv").unwrap();
    assert_eq!(params[ctv_idx + 1], "q4_1");

    // Unified KV cache is disabled (since we passed false)
    assert!(!params.contains(&"--kv-unified".to_string()));

    // API key and metrics are present
    let api_idx = params.iter().position(|r| r == "--api-key").unwrap();
    assert_eq!(params[api_idx + 1], "secret-token-123");

    assert!(params.contains(&"--metrics".to_string()));

    let lv_idx = params.iter().position(|r| r == "--log-verbosity").unwrap();
    assert_eq!(params[lv_idx + 1], "5");

    // Now test router mode too
    let router_params = build_router_launch_parameters(
        &exe_path,
        &PathBuf::from("/models/presets.ini"),
        &global_config,
        8080,
    );

    assert!(!router_params.contains(&"--kv-unified".to_string()));

    let r_api_idx = router_params.iter().position(|r| r == "--api-key").unwrap();
    assert_eq!(router_params[r_api_idx + 1], "secret-token-123");

    assert!(router_params.contains(&"--metrics".to_string()));

    let r_lv_idx = router_params
        .iter()
        .position(|r| r == "--log-verbosity")
        .unwrap();
    assert_eq!(router_params[r_lv_idx + 1], "5");

    Ok(())
}

#[test]
fn test_build_launch_parameters_checkpointing_and_mmap() -> TestResult {
    let exe_path = PathBuf::from("/bin/llama-server");
    let model_path = PathBuf::from("/models/gemma-2b.gguf");

    let mut config_data = HashMap::new();
    let mut long_map = serde_json::Map::new();
    long_map.insert("temp".to_string(), serde_json::json!(1.0));
    long_map.insert("top-p".to_string(), serde_json::json!(0.95));
    long_map.insert("top-k".to_string(), serde_json::json!(64));
    config_data.insert(
        "llama-server-long".to_string(),
        serde_json::Value::Object(long_map),
    );

    let assets = ModelAssets {
        config: config_data,
        jinja_template: None,
    };

    let settings = UserSettings {
        ctx: 2048,
        ngl: "32".to_string(),
        mmproj: None,
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    let mut global_config = HashMap::new();
    global_config.insert("ctx-checkpoints".to_string(), serde_json::json!(128));
    global_config.insert("checkpoint-min-step".to_string(), serde_json::json!(2048));
    global_config.insert("no-mmap".to_string(), serde_json::json!(true));

    let params = build_launch_parameters(
        &exe_path,
        &model_path,
        &assets,
        &settings,
        &global_config,
        8080,
    );

    let temp_idx = params.iter().position(|r| r == "--temp").unwrap();
    assert_eq!(params[temp_idx + 1], "1");

    let top_p_idx = params.iter().position(|r| r == "--top-p").unwrap();
    assert_eq!(params[top_p_idx + 1], "0.95");

    let top_k_idx = params.iter().position(|r| r == "--top-k").unwrap();
    assert_eq!(params[top_k_idx + 1], "64");

    let checkpoints_idx = params
        .iter()
        .position(|r| r == "--ctx-checkpoints")
        .unwrap();
    assert_eq!(params[checkpoints_idx + 1], "128");

    let step_idx = params
        .iter()
        .position(|r| r == "--checkpoint-min-step")
        .unwrap();
    assert_eq!(params[step_idx + 1], "2048");

    assert!(params.contains(&"--no-mmap".to_string()));

    Ok(())
}
