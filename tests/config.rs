use llama_herd::config::{
    calculate_ngl, format_arg_name, format_ini_key, get_global_config_string, get_optimal_threads,
    is_restricted_key, load_settings_from_ini, load_toml_safe, load_toml_silent, parse_ctx,
    parse_ctx_str, parse_settings_ini, remove_global_config_value, update_global_config_value,
};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_parse_ctx_values() -> TestResult {
    // Given different JSON types representing context sizes
    // When calling parse_ctx
    // Then correct context size integers are returned

    // 1. Valid numbers
    assert_eq!(parse_ctx(&json!(4096)).unwrap(), 4096);
    assert_eq!(parse_ctx(&json!(8192.0)).unwrap(), 8192);

    // 2. String representation with suffix 'k' or 'K'
    assert_eq!(parse_ctx(&json!("128k")).unwrap(), 131072);
    assert_eq!(parse_ctx(&json!("8K")).unwrap(), 8192);
    assert_eq!(parse_ctx(&json!("  4096  ")).unwrap(), 4096);

    // 3. Fallbacks for invalid/null types
    assert!(parse_ctx(&json!(null)).is_err());
    assert!(parse_ctx(&json!([])).is_err());
    assert!(parse_ctx(&json!({})).is_err());

    Ok(())
}

#[test]
fn test_parse_ctx_str_edge_cases() -> TestResult {
    // Given edge case context size strings (empty, invalid format, overflows)
    // When calling parse_ctx_str
    // Then it returns either the parsed value or the fallback context size (131072)

    // 1. Empty and invalid inputs
    assert!(parse_ctx_str("").is_err());
    assert!(parse_ctx_str("abc").is_err());
    assert!(parse_ctx_str("k").is_err());

    // 2. Extraneous whitespaces & mixed casing
    assert_eq!(parse_ctx_str("  32K  ").unwrap(), 32768);
    assert_eq!(parse_ctx_str("\n64k\t").unwrap(), 65536);

    // 3. Upper bounds/Overflows (saturating or fallback depending on parse outcome)
    // Very large string numbers that fail to parse as usize will trigger fallback
    assert!(parse_ctx_str(&format!("{}00000000000", usize::MAX)).is_err());
    assert_eq!(
        parse_ctx_str(&format!("{}", usize::MAX)).unwrap(),
        usize::MAX
    );

    Ok(())
}

#[test]
fn test_get_optimal_threads_format() -> TestResult {
    // Given the CPU core count querying helper
    // When calling get_optimal_threads
    // Then it returns a valid integer string representation that is greater than or equal to 1

    let threads_str = get_optimal_threads();
    assert!(!threads_str.is_empty());

    let parsed_val = threads_str.parse::<usize>()?;
    assert!(parsed_val >= 1);

    Ok(())
}

#[test]
fn test_calculate_ngl_logic() -> TestResult {
    // Given inputs for calculating number of GPU layers (ngl)
    // When calling calculate_ngl with layer counts and subtraction tokens
    // Then it performs correct saturating arithmetic or fallback selection

    // 1. Subtraction format "--N" (saturating subtract from total layers)
    assert_eq!(calculate_ngl("--4", "auto", Some(32)), "28");
    // Underflow / saturating check
    assert_eq!(calculate_ngl("--40", "auto", Some(32)), "0");

    // 2. Fallbacks when total layers are unknown
    assert_eq!(calculate_ngl("--4", "auto", None), "--4");

    // 3. Empty input defaults
    assert_eq!(calculate_ngl("", "auto", Some(32)), "auto");
    assert_eq!(calculate_ngl("", "32", None), "32");

    // 4. Custom integer values passed directly
    assert_eq!(calculate_ngl("16", "auto", Some(32)), "16");

    Ok(())
}

#[test]
fn test_is_restricted_key_checks() -> TestResult {
    // Given configuration keys
    // When checked via is_restricted_key
    // Then restricted options return true, and customizable pass-through keys return false

    // 1. Long forms
    assert!(is_restricted_key("ctx-size"));
    assert!(is_restricted_key("total-layers"));
    assert!(is_restricted_key("n-gpu-layers"));
    assert!(is_restricted_key("temp"));

    // 2. Pass-through keys
    assert!(!is_restricted_key("slot-prompt-similarity"));
    assert!(!is_restricted_key("sps"));

    Ok(())
}

#[test]
fn test_format_arg_name_mapping() -> TestResult {
    // Given user custom keys
    // When calling format_arg_name
    // Then it formats to double dashes
    assert_eq!(
        format_arg_name("slot-prompt-similarity"),
        Some("--slot-prompt-similarity".to_string())
    );
    assert_eq!(format_arg_name("sps"), Some("--sps".to_string()));

    Ok(())
}

#[test]
fn test_format_ini_key_mapping() -> TestResult {
    // Given keys intended for preset INI formatting
    // When calling format_ini_key
    // Then it returns the key unchanged
    assert_eq!(
        format_ini_key("slot-prompt-similarity"),
        Some("slot-prompt-similarity".to_string())
    );
    assert_eq!(format_ini_key("sps"), Some("sps".to_string()));

    Ok(())
}

#[test]
fn test_parse_settings_ini_content() -> TestResult {
    // Given an INI string containing comments, sections, global section, and keys
    // When calling parse_settings_ini
    // Then a nested Map structure of sections to key-value pairs is correctly constructed

    let content = r#"
        # Comment line
        ; Another comment
        
        [*]
        flash-attn = auto
        kv-unified = true

        [gemma-2]
        model = gemma-2b.gguf
        ctx-size = 8192
    "#;

    let parsed = parse_settings_ini(content);

    // Assert section exists
    assert!(parsed.contains_key("*"));
    assert!(parsed.contains_key("gemma-2"));

    // Assert values
    let global = parsed.get("*").unwrap();
    assert_eq!(global.get("flash-attn").unwrap(), "auto");
    assert_eq!(global.get("kv-unified").unwrap(), "true");

    let preset = parsed.get("gemma-2").unwrap();
    assert_eq!(preset.get("model").unwrap(), "gemma-2b.gguf");
    assert_eq!(preset.get("ctx-size").unwrap(), "8192");

    Ok(())
}

#[test]
fn test_load_settings_from_ini_merge() -> TestResult {
    // Given a temporary INI file with default and specific preset values
    // When load_settings_from_ini is called
    // Then global defaults are merged and overwritten by specific preset values

    let dir = tempdir()?;
    let path = dir.path().join("preset.ini");

    let content = r#"
        [*]
        flash-attn = auto
        temp = 0.8

        [my-preset]
        model = test.gguf
        temp = 0.5
    "#;

    File::create(&path)?.write_all(content.as_bytes())?;

    // Load non-existing file should return None
    assert!(load_settings_from_ini("my-preset", &dir.path().join("nonexistent.ini")).is_none());

    // Load existing preset
    let loaded = load_settings_from_ini("my-preset", &path).unwrap();
    assert_eq!(loaded.get("flash-attn").unwrap(), "auto"); // from global *
    assert_eq!(loaded.get("model").unwrap(), "test.gguf"); // from my-preset
    assert_eq!(loaded.get("temp").unwrap(), "0.5"); // overridden from my-preset

    // Load non-existing preset within existing file should return None
    assert!(load_settings_from_ini("missing-preset", &path).is_none());

    Ok(())
}

#[test]
fn test_load_toml_skips_invalid_keys() -> TestResult {
    // Given a temporary TOML file containing tables and keys
    // When calling load_toml_silent or load_toml_safe
    // Then invalid keys containing underscores or leading dashes are skipped from the map

    let dir = tempdir()?;
    let path = dir.path().join("model.toml");

    let content = r#"
        host = "127.0.0.1"
        invalid_underscore_key = "value"
        -invalid-dash-start = 123
        [llama-herd]
        is-draft = true
        [llama-server-short]
        sps = 0.85
        [llama-server-long]
        ctx-size = "128k"
    "#;

    File::create(&path)?.write_all(content.as_bytes())?;

    // 1. Using silent loader
    let loaded_silent = load_toml_silent(&path);
    assert_eq!(
        loaded_silent.get("host").unwrap().as_str().unwrap(),
        "127.0.0.1"
    );
    let lh = loaded_silent
        .get("llama-herd")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(lh.get("is-draft").unwrap().as_bool().unwrap());

    let short = loaded_silent
        .get("llama-server-short")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(short.get("sps").unwrap().as_f64().unwrap(), 0.85);

    let long = loaded_silent
        .get("llama-server-long")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(long.get("ctx-size").unwrap().as_str().unwrap(), "128k");

    assert!(!loaded_silent.contains_key("invalid_underscore_key"));
    assert!(!loaded_silent.contains_key("-invalid-dash-start"));

    // 2. Using safe loader
    let loaded_safe = load_toml_safe(&path).unwrap();
    assert_eq!(
        loaded_safe.get("host").unwrap().as_str().unwrap(),
        "127.0.0.1"
    );
    assert!(!loaded_safe.contains_key("invalid_underscore_key"));
    assert!(!loaded_safe.contains_key("-invalid-dash-start"));

    // Non-existent file safety
    let missing_path = dir.path().join("missing.toml");
    assert!(load_toml_silent(&missing_path).is_empty());
    assert!(load_toml_safe(&missing_path).is_err());

    Ok(())
}

#[test]
fn test_load_toml_silent_internal() {
    let test_path = std::path::Path::new("test_config_internal.toml");
    {
        let mut file = File::create(test_path).unwrap();
        std::io::Write::write_all(
            &mut file,
            b"host = \"127.0.0.1\"\nport = 8080\nthreads = 4\ntemp = 0.8\ninvalid_key = 123\n-invalid-dash = 456\n"
        ).unwrap();
    }

    let cfg = load_toml_silent(test_path);
    assert_eq!(cfg.get("host").unwrap().as_str().unwrap(), "127.0.0.1");
    assert_eq!(cfg.get("port").unwrap().as_i64().unwrap(), 8080);
    assert_eq!(cfg.get("threads").unwrap().as_i64().unwrap(), 4);
    assert_eq!(cfg.get("temp").unwrap().as_f64().unwrap(), 0.8);
    assert!(!cfg.contains_key("invalid_key"));
    assert!(!cfg.contains_key("-invalid-dash"));
    let _ = std::fs::remove_file(test_path);
}

#[test]
fn test_is_restricted_key_internal() {
    assert!(is_restricted_key("ctx-size"));
    assert!(is_restricted_key("total-layers"));
    assert!(is_restricted_key("n-gpu-layers"));
    assert!(is_restricted_key("temp"));

    assert!(!is_restricted_key("slot-prompt-similarity"));
}

#[test]
fn test_format_arg_name_internal() {
    assert_eq!(
        format_arg_name("slot-prompt-similarity"),
        Some("--slot-prompt-similarity".to_string())
    );
}

#[test]
fn test_format_ini_key_internal() {
    assert_eq!(
        format_ini_key("slot-prompt-similarity"),
        Some("slot-prompt-similarity".to_string())
    );
}

#[test]
fn test_get_global_config_string() -> TestResult {
    use std::collections::HashMap;

    let mut config = HashMap::new();
    config.insert("host".to_string(), serde_json::json!("127.0.0.1"));
    config.insert("port".to_string(), serde_json::json!(8080));
    config.insert("ui".to_string(), serde_json::json!(true));

    // Test direct root lookup
    assert_eq!(
        get_global_config_string(&config, "host", "0.0.0.0"),
        "127.0.0.1"
    );
    assert_eq!(get_global_config_string(&config, "port", "auto"), "8080");
    assert_eq!(get_global_config_string(&config, "ui", "false"), "true");

    // Test fallback
    assert_eq!(
        get_global_config_string(&config, "missing-key", "fallback-val"),
        "fallback-val"
    );

    // Test nesting llama-herd
    let mut herd_table = serde_json::Map::new();
    herd_table.insert("ui".to_string(), serde_json::json!(false));
    config.insert(
        "llama-herd".to_string(),
        serde_json::Value::Object(herd_table),
    );

    // Nested llama-herd has priority over root
    assert_eq!(get_global_config_string(&config, "ui", "true"), "false");

    // Test nesting llama-server-long
    let mut long_table = serde_json::Map::new();
    long_table.insert("host".to_string(), serde_json::json!("192.168.1.1"));
    config.insert(
        "llama-server-long".to_string(),
        serde_json::Value::Object(long_table),
    );

    // Nested llama-server-long has priority over llama-herd and root
    assert_eq!(
        get_global_config_string(&config, "host", "0.0.0.0"),
        "192.168.1.1"
    );

    Ok(())
}

#[test]
fn test_update_global_config_value() -> TestResult {
    use std::collections::HashMap;

    let mut config = HashMap::new();

    // 1. Initial insert (should go to root)
    update_global_config_value(&mut config, "host", serde_json::json!("127.0.0.1"));
    assert_eq!(config.get("host").unwrap().as_str().unwrap(), "127.0.0.1");

    // 2. Insert with llama-herd present but key not in llama-herd (should still go to root)
    let herd_table = serde_json::Map::new();
    config.insert(
        "llama-herd".to_string(),
        serde_json::Value::Object(herd_table),
    );
    update_global_config_value(&mut config, "port", serde_json::json!(8080));
    assert_eq!(config.get("port").unwrap().as_i64().unwrap(), 8080);

    // 3. Update key that is already inside llama-herd
    let mut herd_table = serde_json::Map::new();
    herd_table.insert("ui".to_string(), serde_json::json!(true));
    config.insert(
        "llama-herd".to_string(),
        serde_json::Value::Object(herd_table),
    );

    update_global_config_value(&mut config, "ui", serde_json::json!(false));
    let updated_herd = config.get("llama-herd").unwrap().as_object().unwrap();
    assert!(!updated_herd.get("ui").unwrap().as_bool().unwrap());
    assert!(!config.contains_key("ui")); // should not be at root

    // 4. Update key that is already inside llama-server-long
    let mut long_table = serde_json::Map::new();
    long_table.insert("host".to_string(), serde_json::json!("127.0.0.1"));
    config.insert(
        "llama-server-long".to_string(),
        serde_json::Value::Object(long_table),
    );

    update_global_config_value(&mut config, "host", serde_json::json!("0.0.0.0"));
    let updated_long = config
        .get("llama-server-long")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        updated_long.get("host").unwrap().as_str().unwrap(),
        "0.0.0.0"
    );
    // root "host" is unchanged
    assert_eq!(config.get("host").unwrap().as_str().unwrap(), "127.0.0.1");

    Ok(())
}

#[test]
fn test_remove_global_config_value() -> TestResult {
    use std::collections::HashMap;

    let mut config = HashMap::new();
    config.insert("host".to_string(), serde_json::json!("127.0.0.1"));

    // 1. Remove root value
    remove_global_config_value(&mut config, "host");
    assert!(!config.contains_key("host"));

    // 2. Remove nested value, verifying table cleanup
    let mut herd_table = serde_json::Map::new();
    herd_table.insert("ui".to_string(), serde_json::json!(false));
    herd_table.insert("models-max".to_string(), serde_json::json!(2));
    config.insert(
        "llama-herd".to_string(),
        serde_json::Value::Object(herd_table),
    );

    // Remove one nested key (table still has models-max, so table should remain)
    remove_global_config_value(&mut config, "ui");
    assert!(config.contains_key("llama-herd"));
    let updated_herd = config.get("llama-herd").unwrap().as_object().unwrap();
    assert!(!updated_herd.contains_key("ui"));
    assert_eq!(updated_herd.get("models-max").unwrap().as_i64().unwrap(), 2);

    // Remove the last nested key (table is now empty, so table should be removed)
    remove_global_config_value(&mut config, "models-max");
    assert!(!config.contains_key("llama-herd"));

    Ok(())
}
