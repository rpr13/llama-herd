#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use llama_herd::config::{
    is_safe_value, load_settings_from_ini, load_toml_safe, load_toml_silent, parse_ctx,
    parse_ctx_str,
};
use llama_herd::launcher::resolve_port;
use llama_herd::tui::theme::Theme;
use llama_herd::tui::{AppScreen, AppState, TuiEvent, handle_key_event};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_resolve_port_boundary_overflow() -> TestResult {
    // 1. Test parsing invalid ports
    assert!(resolve_port("invalid").is_err());
    assert!(resolve_port("-1").is_err());
    assert!(resolve_port("999999").is_err());

    // 2. Test port 65535 overflow behavior
    // If port 65535 is not available (we mock by binding or if it's already bound),
    // it must return Err (AddrInUse) and not panic.
    // First try to bind to 65535 to make it occupied (if possible on the host).
    let listener = std::net::TcpListener::bind(("127.0.0.1", 65535));
    let res = resolve_port("65535");
    if listener.is_ok() {
        assert!(res.is_err());
    } else {
        // If we can't bind because it's already occupied on the test machine, resolve_port should also fail safely.
        // If it was somehow free but we couldn't bind it, res could be Ok(65535) or Err.
        // The key check is that calling resolve_port does not panic.
        let _ = res;
    }

    Ok(())
}

#[test]
fn test_invalid_config_parsing_types() -> TestResult {
    let dir = tempdir()?;
    let path = dir.path().join("model.toml");

    // Write a configuration containing:
    // - NaN/Inf float values (which used to cause a panic due to Number::from_f64().unwrap())
    // - Invalid keys that start with a dash or contain underscore
    // - Valid nested tables and mixed arrays
    std::fs::write(
        &path,
        r#"
[llama-server-long]
temp = nan
top-p = inf
reasoning = "deepseek"
_invalid_key = 123
-invalid-key = "abc"
valid-key = [1, "abc", 3.15]

[llama-herd]
total-layers = 32
is-draft = true
"#,
    )?;

    // Test load_toml_safe
    let config = load_toml_safe(&path)?;

    // Verify NaN/Inf resolved to Null
    let long_opts = config
        .get("llama-server-long")
        .and_then(|v| v.as_object())
        .ok_or("Missing llama-server-long object")?;
    assert_eq!(long_opts.get("temp"), Some(&serde_json::Value::Null));
    assert_eq!(long_opts.get("top-p"), Some(&serde_json::Value::Null));
    assert_eq!(
        long_opts.get("reasoning"),
        Some(&serde_json::Value::String("deepseek".to_string()))
    );

    // Verify invalid keys are skipped
    assert!(long_opts.get("_invalid_key").is_none());
    assert!(long_opts.get("-invalid-key").is_none());

    // Verify valid keys are parsed
    assert_eq!(
        long_opts
            .get("valid-key")
            .and_then(|v| v.as_array())
            .unwrap()[2]
            .as_f64()
            .unwrap(),
        3.15
    );

    // Test load_toml_silent on a non-existent file
    let empty_config = load_toml_silent(&dir.path().join("missing.toml"));
    assert!(empty_config.is_empty());

    Ok(())
}

#[test]
fn test_parse_ctx_overflow_and_invalid() -> TestResult {
    // 1. Strings ending with 'k'/'K' that would multiply overflow usize::MAX
    let overflow_val = format!("{}k", usize::MAX);
    assert!(parse_ctx_str(&overflow_val).is_err());

    let overflow_val2 = format!("{}K", usize::MAX / 2);
    assert!(parse_ctx_str(&overflow_val2).is_err());

    // 2. String ending with 'k' but value starts with negative or invalid characters
    assert!(parse_ctx_str("-32k").is_err());
    assert!(parse_ctx_str("abcK").is_err());
    assert!(parse_ctx_str("32M").is_err()); // 'M' is not a valid suffix
    assert!(parse_ctx_str("32G").is_err()); // 'G' is not a valid suffix

    // 3. parse_ctx with various JSON values
    assert_eq!(parse_ctx(&json!("64K")).unwrap(), 65536);
    assert!(parse_ctx(&json!("-32k")).is_err());
    assert!(parse_ctx(&json!(18446744073709551615u64)).is_ok());

    Ok(())
}

#[test]
fn test_safe_values_robustness() {
    // 1. Test strings with options or injection
    assert!(!is_safe_value(&json!("--evil-flag")));
    assert!(!is_safe_value(&json!("-evil-flag")));
    assert!(!is_safe_value(&json!("  --hacked")));
    assert!(!is_safe_value(&json!("-e")));
    assert!(!is_safe_value(&json!("-"))); // empty dash is unsafe

    // 2. Test safe negative numbers
    assert!(is_safe_value(&json!("-1")));
    assert!(is_safe_value(&json!("-3.15")));
    assert!(is_safe_value(&json!("-0.005")));

    // 3. Test arbitrary safe strings
    assert!(is_safe_value(&json!("127.0.0.1")));
    assert!(is_safe_value(&json!("auto")));
    assert!(is_safe_value(&json!("q8_0")));

    // 4. Test complex values (Arrays & Objects)
    assert!(is_safe_value(&json!([
        "127.0.0.1",
        "auto",
        "-1.23",
        "q8_0"
    ])));
    assert!(!is_safe_value(&json!(["127.0.0.1", "--evil-flag"])));
    assert!(!is_safe_value(&json!({ "some_key": "some_value" }))); // Objects are not allowed
}

#[test]
fn test_load_settings_from_ini_corrupt() -> TestResult {
    let dir = tempdir()?;
    let path = dir.path().join("presets.ini");

    // 1. Completely empty or garbage file
    std::fs::write(&path, "garbage data that is not ini format at all")?;
    let settings = load_settings_from_ini("nonexistent", &path).unwrap_or_default();
    assert!(settings.is_empty());

    // 2. Valid INI format but sections don't match
    std::fs::write(
        &path,
        r#"
[different-preset]
ctx-size = 4096
n-gpu-layers = 12
"#,
    )?;
    let settings = load_settings_from_ini("target-preset", &path).unwrap_or_default();
    assert!(settings.is_empty());

    // 3. Malformed sections or duplicate key entries
    std::fs::write(
        &path,
        r#"
[target-preset]
ctx-size = 4096
ctx-size = 8192
n-gpu-layers = 24
[invalid-section
invalid-value = 1
"#,
    )?;
    let settings =
        load_settings_from_ini("target-preset", &path).ok_or("Failed to load settings option")?;
    assert_eq!(settings.get("ctx-size"), Some(&"8192".to_string()));
    assert_eq!(settings.get("n-gpu-layers"), Some(&"24".to_string()));

    Ok(())
}

#[test]
fn test_tui_input_handling_bounds() {
    let mut state = AppState::new(
        vec![],
        PathBuf::from("."),
        PathBuf::from("."),
        HashMap::new(),
        PathBuf::from("."),
        Theme::default(),
    );
    let (tx, _) = std::sync::mpsc::channel::<TuiEvent>();

    // 1. Test KeyCode::Backspace when input_buffer is empty
    state.input_buffer.clear();
    state.screen = AppScreen::EditingCtx;
    let backspace_key = KeyEvent {
        code: KeyCode::Backspace,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    handle_key_event(&mut state, backspace_key, &tx);
    assert!(state.input_buffer.is_empty()); // Assert it doesn't crash or underflow

    // 2. Test KeyCode::Char pushes character
    let char_key = KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    handle_key_event(&mut state, char_key, &tx);
    assert_eq!(state.input_buffer, "a");

    // 3. Test Enter key bounds validation in EditingCtx screen
    // Input is invalid: screen should remain EditingCtx
    state.input_buffer = "invalid_ctx".to_string();
    let enter_key = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    handle_key_event(&mut state, enter_key, &tx);
    assert_eq!(state.screen, AppScreen::EditingCtx);

    // Input is valid: screen should change to Dashboard
    state.input_buffer = "8192".to_string();
    handle_key_event(&mut state, enter_key, &tx);
    assert_eq!(state.screen, AppScreen::Dashboard);
    assert_eq!(state.ctx, 8192);

    // 4. Test Enter key bounds validation in EditingTotalLayers screen
    // Valid integer: should update total_layers and return to Dashboard
    state.screen = AppScreen::EditingTotalLayers;
    state.input_buffer = "32".to_string();
    handle_key_event(&mut state, enter_key, &tx);
    assert_eq!(state.screen, AppScreen::Dashboard);
    assert_eq!(state.total_layers, Some(32));

    // Invalid integer: should not update total_layers (leaves old value) and goes to Dashboard
    state.screen = AppScreen::EditingTotalLayers;
    state.input_buffer = "invalid_layers".to_string();
    handle_key_event(&mut state, enter_key, &tx);
    assert_eq!(state.screen, AppScreen::Dashboard);
    assert_eq!(state.total_layers, Some(32)); // Value unchanged
}
