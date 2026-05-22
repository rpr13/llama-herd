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
        ui: true,
        mmproj: None,
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    let global_config = HashMap::new();

    let params =
        build_launch_parameters(&exe_path, &model_path, &assets, &settings, &global_config);

    // Verify binary and model paths
    assert_eq!(params[0], "/bin/llama-server");

    // Model option
    let m_idx = params.iter().position(|r| r == "-m").unwrap();
    assert_eq!(params[m_idx + 1], "/models/gemma-2b.gguf");

    // Default host and port
    let host_idx = params.iter().position(|r| r == "--host").unwrap();
    assert_eq!(params[host_idx + 1], "0.0.0.0");
    let port_idx = params.iter().position(|r| r == "--port").unwrap();
    assert_eq!(params[port_idx + 1], "8080");

    // NGL and context size
    let ngl_idx = params.iter().position(|r| r == "-ngl").unwrap();
    assert_eq!(params[ngl_idx + 1], "32");
    let ctx_idx = params.iter().position(|r| r == "--ctx-size").unwrap();
    assert_eq!(params[ctx_idx + 1], "2048");

    // Unified KV cache is enabled
    assert!(params.contains(&"--kv-unified".to_string()));

    // Cache quantization options
    let ctk_idx = params.iter().position(|r| r == "-ctk").unwrap();
    assert_eq!(params[ctk_idx + 1], "q8_0");

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
    config_data.insert(
        "s-sps".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(0.8).unwrap()),
    );
    config_data.insert(
        "slot-prompt-similarity".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(0.95).unwrap()),
    );
    config_data.insert("lh-custom-ignored".to_string(), serde_json::json!(true));

    let assets = ModelAssets {
        config: config_data,
        jinja_template: Some(PathBuf::from("/models/gemma.jinja")),
    };

    // 2. Setup user settings including draft model, mmproj, and ui disabled
    let settings = UserSettings {
        ctx: 8192,
        ngl: "auto".to_string(),
        ui: false, // UI disabled
        mmproj: Some(PathBuf::from("/models/mmproj.gguf")),
        draft_model: Some(PathBuf::from("/models/gemma-draft.gguf")),
        draft_ngl: "16".to_string(),
    };

    // 3. Setup global overrides
    let mut global_config = HashMap::new();
    global_config.insert("host".to_string(), serde_json::json!("127.0.0.1"));
    global_config.insert("port".to_string(), serde_json::json!(9000));
    global_config.insert("lh-kv-quant".to_string(), serde_json::json!("q4_0"));
    global_config.insert("batch-size".to_string(), serde_json::json!(512));
    global_config.insert("ubatch-size".to_string(), serde_json::json!(256));
    global_config.insert("tools".to_string(), serde_json::json!("web-search"));

    let params =
        build_launch_parameters(&exe_path, &model_path, &assets, &settings, &global_config);

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

    // Verify custom "lh-" prefix keys are NOT passed directly to llama-server
    assert!(!params.contains(&"--lh-custom-ignored".to_string()));
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
    config_data.insert(
        "lh-spec-type".to_string(),
        serde_json::json!("draft-eagle3"),
    );

    let assets = ModelAssets {
        config: config_data,
        jinja_template: None,
    };
    let settings = UserSettings {
        ctx: 2048,
        ngl: "0".to_string(),
        ui: true,
        mmproj: None,
        draft_model: Some(PathBuf::from("draft.gguf")),
        draft_ngl: "0".to_string(),
    };

    let params =
        build_launch_parameters(&exe_path, &model_path, &assets, &settings, &HashMap::new());

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

    let params = build_router_launch_parameters(&exe_path, &preset_path, &global_config);

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
