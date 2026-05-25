use llama_herd::config;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::tempdir;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_get_home_dir() -> TestResult {
    let _guard = ENV_MUTEX.lock().unwrap();
    let original_home = std::env::var("HOME").ok();
    let original_userprofile = std::env::var("USERPROFILE").ok();

    unsafe {
        std::env::set_var("HOME", "/dummy/home");
    }
    assert_eq!(config::get_home_dir(), Some(PathBuf::from("/dummy/home")));

    unsafe {
        std::env::remove_var("HOME");
        std::env::set_var("USERPROFILE", "/dummy/userprofile");
    }
    assert_eq!(
        config::get_home_dir(),
        Some(PathBuf::from("/dummy/userprofile"))
    );

    unsafe {
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
    }
    assert_eq!(config::get_home_dir(), None);

    if let Some(h) = original_home {
        unsafe {
            std::env::set_var("HOME", h);
        }
    } else {
        unsafe {
            std::env::remove_var("HOME");
        }
    }
    if let Some(up) = original_userprofile {
        unsafe {
            std::env::set_var("USERPROFILE", up);
        }
    } else {
        unsafe {
            std::env::remove_var("USERPROFILE");
        }
    }
    Ok(())
}

#[test]
fn test_parse_args_logic() {
    use config::parse_args;

    // Help
    assert_eq!(parse_args(&["bin".into(), "-h".into()]), (true, false));
    assert_eq!(parse_args(&["bin".into(), "--help".into()]), (true, false));

    // INI
    assert_eq!(parse_args(&["bin".into(), "--ini".into()]), (false, true));

    // Default
    assert_eq!(parse_args(&["bin".into()]), (false, false));

    // Combined
    assert_eq!(
        parse_args(&["bin".into(), "-h".into(), "--ini".into()]),
        (true, true)
    );
}

#[test]
fn test_get_llama_herd_dir() -> TestResult {
    let _guard = ENV_MUTEX.lock().unwrap();
    let original_home = std::env::var("HOME").ok();
    let original_appdata = std::env::var("APPDATA").ok();

    if cfg!(target_os = "windows") {
        unsafe {
            std::env::set_var("APPDATA", "/dummy/appdata");
        }
        assert_eq!(
            config::get_llama_herd_dir(),
            PathBuf::from("/dummy/appdata/llama-herd")
        );
    } else {
        unsafe {
            std::env::set_var("HOME", "/dummy/home");
        }
        assert_eq!(
            config::get_llama_herd_dir(),
            PathBuf::from("/dummy/home/.config/llama-herd")
        );
    }

    if let Some(h) = original_home {
        unsafe {
            std::env::set_var("HOME", h);
        }
    } else {
        unsafe {
            std::env::remove_var("HOME");
        }
    }
    if let Some(ad) = original_appdata {
        unsafe {
            std::env::set_var("APPDATA", ad);
        }
    } else {
        unsafe {
            std::env::remove_var("APPDATA");
        }
    }
    Ok(())
}

#[test]
fn test_resolve_server_executable() -> TestResult {
    use std::collections::HashMap;
    let mut config = HashMap::new();

    // Test from config
    let temp = tempdir()?;
    let dummy_exe = temp.path().join(if cfg!(target_os = "windows") {
        "llama-server.exe"
    } else {
        "llama-server"
    });
    std::fs::File::create(&dummy_exe)?;

    config.insert(
        "llama-server".to_string(),
        serde_json::Value::String(dummy_exe.to_string_lossy().to_string()),
    );
    assert_eq!(
        config::resolve_server_executable(&config),
        Some(dummy_exe.clone())
    );

    // Test from PATH (if possible, but hard to mock reliably without affecting other tests)
    Ok(())
}

#[test]
fn test_resolve_models_dir() -> TestResult {
    use std::collections::HashMap;
    let mut config = HashMap::new();

    let temp = tempdir()?;
    let models_path = temp.path().join("models");
    std::fs::create_dir(&models_path)?;

    // Test from config
    config.insert(
        "models-dir".to_string(),
        serde_json::Value::String(models_path.to_string_lossy().to_string()),
    );
    assert_eq!(
        config::resolve_models_dir(&config),
        Some(models_path.clone())
    );

    Ok(())
}
