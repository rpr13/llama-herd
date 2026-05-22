use llama_herd::config::{
    calculate_ngl, format_arg_name, format_ini_key, get_optimal_threads, is_restricted_key,
    load_settings_from_ini, load_toml_safe, load_toml_silent, parse_ctx, parse_ctx_str,
    parse_settings_ini,
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
    assert_eq!(parse_ctx(&json!(4096)), 4096);
    assert_eq!(parse_ctx(&json!(8192.0)), 8192);

    // 2. String representation with suffix 'k' or 'K'
    assert_eq!(parse_ctx(&json!("128k")), 131072);
    assert_eq!(parse_ctx(&json!("8K")), 8192);
    assert_eq!(parse_ctx(&json!("  4096  ")), 4096);

    // 3. Fallbacks for invalid/null types
    assert_eq!(parse_ctx(&json!(null)), 131072);
    assert_eq!(parse_ctx(&json!([])), 131072);
    assert_eq!(parse_ctx(&json!({})), 131072);

    Ok(())
}

#[test]
fn test_parse_ctx_str_edge_cases() -> TestResult {
    // Given edge case context size strings (empty, invalid format, overflows)
    // When calling parse_ctx_str
    // Then it returns either the parsed value or the fallback context size (131072)

    // 1. Empty and invalid inputs
    assert_eq!(parse_ctx_str(""), 131072);
    assert_eq!(parse_ctx_str("abc"), 131072);
    assert_eq!(parse_ctx_str("k"), 131072);

    // 2. Extraneous whitespaces & mixed casing
    assert_eq!(parse_ctx_str("  32K  "), 32768);
    assert_eq!(parse_ctx_str("\n64k\t"), 65536);

    // 3. Upper bounds/Overflows (saturating or fallback depending on parse outcome)
    // Very large string numbers that fail to parse as usize will trigger fallback
    assert_eq!(parse_ctx_str(&format!("{}00000000000", usize::MAX)), 131072);
    assert_eq!(parse_ctx_str(&format!("{}", usize::MAX)), usize::MAX);

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

    // 1. Custom settings prefix
    assert!(is_restricted_key("lh-ctx-size"));
    assert!(is_restricted_key("lh-anything"));

    // 2. Short forms
    assert!(is_restricted_key("s-ngl"));
    assert!(is_restricted_key("s-c"));
    assert!(is_restricted_key("ngl"));
    assert!(is_restricted_key("c"));

    // 3. Long forms
    assert!(is_restricted_key("ctx-size"));
    assert!(is_restricted_key("total-layers"));
    assert!(is_restricted_key("n-gpu-layers"));

    // 4. Pass-through keys
    assert!(!is_restricted_key("slot-prompt-similarity"));
    assert!(!is_restricted_key("s-sps")); // non-restricted short
    // Wait, is temp in RESTRICTED_LONG? Yes! Let's assert it is restricted:
    assert!(is_restricted_key("temp"));

    // Completely unmanaged keys should return false
    assert!(!is_restricted_key("slot-prompt-similarity"));

    Ok(())
}

#[test]
fn test_format_arg_name_mapping() -> TestResult {
    // Given user custom keys
    // When calling format_arg_name
    // Then custom prefixes return None, short prefixes translate to single dash, and others to double dashes

    // 1. Custom keys
    assert!(format_arg_name("lh-ctx-size").is_none());

    // 2. Short options
    assert_eq!(format_arg_name("s-sps"), Some("-sps".to_string()));
    assert_eq!(format_arg_name("s-t"), Some("-t".to_string()));

    // 3. Long options
    assert_eq!(
        format_arg_name("slot-prompt-similarity"),
        Some("--slot-prompt-similarity".to_string())
    );

    Ok(())
}

#[test]
fn test_format_ini_key_mapping() -> TestResult {
    // Given keys intended for preset INI formatting
    // When calling format_ini_key
    // Then custom prefixes return None, short prefixes strip the prefix, and others remain unchanged

    // 1. Custom keys
    assert!(format_ini_key("lh-ctx-size").is_none());

    // 2. Short options
    assert_eq!(format_ini_key("s-sps"), Some("sps".to_string()));

    // 3. Long options
    assert_eq!(
        format_ini_key("slot-prompt-similarity"),
        Some("slot-prompt-similarity".to_string())
    );

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
    // Given a temporary TOML file containing valid keys, underscored keys, and dashed prefix keys
    // When calling load_toml_silent or load_toml_safe
    // Then invalid keys containing underscores or leading dashes are skipped from the map

    let dir = tempdir()?;
    let path = dir.path().join("model.toml");

    let content = r#"
        host = "127.0.0.1"
        lh-ctx-size = 4096
        invalid_underscore_key = "value"
        -invalid-dash-start = 123
        nested-table = { valid-key = true, invalid_key = false }
    "#;

    File::create(&path)?.write_all(content.as_bytes())?;

    // 1. Using silent loader
    let loaded_silent = load_toml_silent(&path);
    assert_eq!(
        loaded_silent.get("host").unwrap().as_str().unwrap(),
        "127.0.0.1"
    );
    assert_eq!(
        loaded_silent.get("lh-ctx-size").unwrap().as_i64().unwrap(),
        4096
    );
    assert!(!loaded_silent.contains_key("invalid_underscore_key"));
    assert!(!loaded_silent.contains_key("-invalid-dash-start"));

    // 2. Using safe loader with stdout warnings (checks behavior is equivalent)
    let loaded_safe = load_toml_safe(&path);
    assert_eq!(
        loaded_safe.get("host").unwrap().as_str().unwrap(),
        "127.0.0.1"
    );
    assert!(!loaded_safe.contains_key("invalid_underscore_key"));
    assert!(!loaded_safe.contains_key("-invalid-dash-start"));

    // Non-existent file safety
    let missing_path = dir.path().join("missing.toml");
    assert!(load_toml_silent(&missing_path).is_empty());
    assert!(load_toml_safe(&missing_path).is_empty());

    Ok(())
}

#[test]
fn test_load_toml_silent_internal() {
    let test_path = std::path::Path::new("test_config_internal.toml");
    {
        let mut file = File::create(test_path).unwrap();
        std::io::Write::write_all(
            &mut file,
            b"host = \"127.0.0.1\"\nport = 8080\nis-draft = true\nthreads = 4\ntemp = 0.8\ninvalid_key = 123\n-invalid-dash = 456\n"
        ).unwrap();
    }

    let cfg = load_toml_silent(test_path);
    assert_eq!(cfg.get("host").unwrap().as_str().unwrap(), "127.0.0.1");
    assert_eq!(cfg.get("port").unwrap().as_i64().unwrap(), 8080);
    assert!(cfg.get("is-draft").unwrap().as_bool().unwrap());
    assert_eq!(cfg.get("threads").unwrap().as_i64().unwrap(), 4);
    assert_eq!(cfg.get("temp").unwrap().as_f64().unwrap(), 0.8);
    assert!(!cfg.contains_key("invalid_key"));
    assert!(!cfg.contains_key("-invalid-dash"));
    let _ = std::fs::remove_file(test_path);
}

#[test]
fn test_is_restricted_key_internal() {
    assert!(is_restricted_key("lh-ctx-size"));
    assert!(is_restricted_key("s-c"));
    assert!(is_restricted_key("c"));
    assert!(is_restricted_key("s-ctx-size"));
    assert!(is_restricted_key("ctx-size"));
    assert!(is_restricted_key("s-ngl"));
    assert!(is_restricted_key("ngl"));
    assert!(is_restricted_key("s-h"));

    assert!(!is_restricted_key("s-sps"));
    assert!(!is_restricted_key("s-rea"));
    assert!(!is_restricted_key("slot-prompt-similarity"));
}

#[test]
fn test_format_arg_name_internal() {
    assert_eq!(format_arg_name("lh-ctx-size"), None);
    assert_eq!(format_arg_name("s-sps"), Some("-sps".to_string()));
    assert_eq!(
        format_arg_name("slot-prompt-similarity"),
        Some("--slot-prompt-similarity".to_string())
    );
}

#[test]
fn test_format_ini_key_internal() {
    assert_eq!(format_ini_key("lh-ctx-size"), None);
    assert_eq!(format_ini_key("s-sps"), Some("sps".to_string()));
    assert_eq!(
        format_ini_key("slot-prompt-similarity"),
        Some("slot-prompt-similarity".to_string())
    );
}
