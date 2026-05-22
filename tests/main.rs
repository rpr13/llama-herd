use llama_herd::config;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_get_home_dir() -> TestResult {
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
    }
    if let Some(up) = original_userprofile {
        unsafe {
            std::env::set_var("USERPROFILE", up);
        }
    }
    Ok(())
}

#[test]
fn test_parse_args_logic() {
    use config::parse_args;

    // Help
    assert_eq!(
        parse_args(&["bin".into(), "-h".into()]),
        (false, true, false)
    );
    assert_eq!(
        parse_args(&["bin".into(), "--help".into()]),
        (false, true, false)
    );

    // CLI
    assert_eq!(
        parse_args(&["bin".into(), "-c".into()]),
        (true, false, false)
    );
    assert_eq!(
        parse_args(&["bin".into(), "--cli".into()]),
        (true, false, false)
    );

    // INI
    assert_eq!(
        parse_args(&["bin".into(), "--ini".into()]),
        (false, false, true)
    );

    // Default
    assert_eq!(parse_args(&["bin".into()]), (false, false, false));

    // Combined
    assert_eq!(
        parse_args(&["bin".into(), "-c".into(), "-h".into(), "--ini".into()]),
        (true, true, true)
    );
}

#[test]
fn test_get_server_executable() -> TestResult {
    let base = Path::new("/llama");
    let exe = config::get_server_executable(base);

    if cfg!(target_os = "windows") {
        assert_eq!(exe, PathBuf::from("/llama/llama-server.exe"));
    } else {
        assert_eq!(exe, PathBuf::from("/llama/llama-server"));
    }
    Ok(())
}

#[test]
fn test_resolve_base_dir_env_override() -> TestResult {
    let temp = tempdir()?;
    let models_path = temp.path().join("models");
    std::fs::create_dir(&models_path)?;

    let original_llama_path = std::env::var("LLAMA_PATH").ok();
    unsafe {
        std::env::set_var("LLAMA_PATH", temp.path());
    }

    let resolved = config::resolve_base_dir();
    assert_eq!(resolved, Some(temp.path().to_path_buf()));

    if let Some(lp) = original_llama_path {
        unsafe {
            std::env::set_var("LLAMA_PATH", lp);
        }
    } else {
        unsafe {
            std::env::remove_var("LLAMA_PATH");
        }
    }
    Ok(())
}
