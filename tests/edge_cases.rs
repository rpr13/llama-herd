#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(unix)]
#[test]
fn test_broken_config_security() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "host = '127.0.0.1'")?;

    // Remove read permissions
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))?;

    let res = llama_herd::config::load_toml_safe(&path);
    assert!(res.is_err());
    Ok(())
}

#[test]
fn test_invalid_syntax_config_security() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "invalid = { syntax = ")?;

    let res = llama_herd::config::load_toml_safe(&path);
    assert!(res.is_err());
    Ok(())
}

#[test]
fn test_port_saturation() -> Result<(), Box<dyn std::error::Error>> {
    let mut listeners = Vec::new();
    for port in 8080..=8090 {
        if let Ok(l) = std::net::TcpListener::bind(("127.0.0.1", port)) {
            listeners.push(l);
        }
    }

    let res = llama_herd::launcher::resolve_port("8080");
    assert!(res.is_err());
    Ok(())
}

#[test]
fn test_parse_ctx_str_validation() {
    use llama_herd::config::parse_ctx_str;

    assert!(parse_ctx_str("64M").is_err());
    assert!(parse_ctx_str("-100").is_err());
    assert!(parse_ctx_str("NaN").is_err());

    assert_eq!(parse_ctx_str("128k").unwrap(), 131072);
    assert_eq!(parse_ctx_str("32K").unwrap(), 32768);
    assert_eq!(parse_ctx_str("8192").unwrap(), 8192);
}

#[test]
fn test_option_injection_prevention() -> Result<(), Box<dyn std::error::Error>> {
    use llama_herd::config::{ModelAssets, UserSettings};
    use llama_herd::launcher::build_launch_parameters;

    let mut config_data = HashMap::new();
    let mut long_map = serde_json::Map::new();
    long_map.insert("port".to_string(), serde_json::json!("-1"));
    long_map.insert("host".to_string(), serde_json::json!("--hacked"));
    long_map.insert("custom-arg".to_string(), serde_json::json!("--evil"));
    long_map.insert("threads".to_string(), serde_json::json!("-1"));

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
        draft_model: None,
        draft_ngl: "".to_string(),
    };

    let params = build_launch_parameters(
        &PathBuf::from("llama-server"),
        &PathBuf::from("model.gguf"),
        &assets,
        &settings,
        &HashMap::new(),
        8080,
    );

    let port_idx = params.iter().position(|r| r == "--port").unwrap();
    assert_eq!(params[port_idx + 1], "8080");

    let host_idx = params.iter().position(|r| r == "--host").unwrap();
    assert_eq!(params[host_idx + 1], "127.0.0.1");

    assert!(!params.contains(&"--custom-arg".to_string()));
    assert!(!params.contains(&"--evil".to_string()));

    let threads_idx = params.iter().position(|r| r == "-t").unwrap();
    assert_eq!(params[threads_idx + 1], "-1");

    Ok(())
}

#[cfg(unix)]
#[test]
fn test_presets_write_failure() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let models_dir = dir.path().join("models");
    std::fs::create_dir(&models_dir)?;

    let output_path = models_dir.join("models-preset.ini");

    std::fs::set_permissions(&models_dir, std::fs::Permissions::from_mode(0o500))?;

    let res =
        llama_herd::discovery::generate_presets_ini(&models_dir, &output_path, &HashMap::new());

    assert!(res.is_err());

    std::fs::set_permissions(&models_dir, std::fs::Permissions::from_mode(0o700))?;

    Ok(())
}

#[test]
fn test_active_pid_tracking_file() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().to_path_buf();

    llama_herd::config::set_llama_herd_dir_override(Some(path));

    // Check that we can add pids
    llama_herd::launcher::add_active_pid(12345);
    llama_herd::launcher::add_active_pid(67890);

    let lh_dir = llama_herd::config::get_llama_herd_dir();
    let pids_file = lh_dir.join("active_pids.txt");

    assert!(pids_file.exists());
    let content = std::fs::read_to_string(&pids_file)?;
    assert!(content.contains("12345"));
    assert!(content.contains("67890"));

    // Check that we can remove a pid
    llama_herd::launcher::remove_active_pid(12345);
    let content_after = std::fs::read_to_string(&pids_file)?;
    assert!(!content_after.contains("12345"));
    assert!(content_after.contains("67890"));

    // Check remove_active_pid when non-existent
    llama_herd::launcher::remove_active_pid(11111);

    // Check kill_existing_servers on non-matching pids list
    // (Should not crash and should clean up the pids file)
    llama_herd::launcher::kill_existing_servers();
    assert!(!pids_file.exists());

    // Restore environment override
    llama_herd::config::set_llama_herd_dir_override(None);

    Ok(())
}

#[test]
fn test_port_fallback_robust() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to port 0 to let OS assign a free port
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    let local_addr = listener.local_addr()?;
    let occupied_port = local_addr.port();

    // Resolve port passing the occupied port as starting point.
    // It should find occupied_port + 1 (or another free port in range)
    let resolved = llama_herd::launcher::resolve_port(&occupied_port.to_string())?;
    assert!(resolved > occupied_port);
    assert!(resolved <= occupied_port + 10);

    Ok(())
}
