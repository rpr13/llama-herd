#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

use llama_herd::tui::logs::{LogStream, SupervisorConfig};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[test]
fn test_supervisor_config_parameter_immutability() {
    let original_params = vec![
        "llama-server".to_owned(),
        "-m".to_owned(),
        "/models/llama-3.gguf".to_owned(),
        "--host".to_owned(),
        "127.0.0.1".to_owned(),
        "--port".to_owned(),
        "8085".to_owned(),
        "-ngl".to_owned(),
        "99".to_owned(),
        "--ctx-size".to_owned(),
        "4096".to_owned(),
    ];
    let cwd = PathBuf::from("/test/cwd");
    let model_name = Some("llama-3".to_owned());

    let config = SupervisorConfig::new(original_params.clone(), cwd.clone(), model_name.clone());

    // CRITICAL: Ensure exact parameter immutability - no flags or context sizes mutated!
    assert_eq!(config.params, original_params);
    assert_eq!(config.cwd, cwd);
    assert_eq!(config.model_name, model_name);
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8085);
}

#[test]
fn test_auto_recovery_on_unexpected_termination() {
    let params = if cfg!(target_os = "windows") {
        vec![
            "ping".to_owned(),
            "127.0.0.1".to_owned(),
            "-n".to_owned(),
            "30".to_owned(),
        ]
    } else {
        vec!["sleep".to_owned(), "30".to_owned()]
    };

    let config = SupervisorConfig::new(params.clone(), PathBuf::from("."), None);
    let mut server = LogStream::spawn_supervised(config, None).expect("Failed to spawn process");

    let pid1 = server.child.lock().unwrap().id();
    assert!(*server.is_running.lock().unwrap());
    assert!(!*server.manual_stop.lock().unwrap());

    // Simulate process crash by directly terminating child process WITHOUT setting manual_stop
    let _ = server.child.lock().unwrap().kill();

    // Give monitor_status thread time to detect crash, update status, and re-spawn
    thread::sleep(Duration::from_millis(2500));

    // Verify supervisor logged crash line
    let crash_logged = {
        let history = server.raw_history.lock().unwrap();
        history
            .iter()
            .any(|line| line.contains("[SUPERVISOR] Process crash detected. Auto-restarting"))
    };
    assert!(
        crash_logged,
        "Supervisor crash detection log should be present in raw_history"
    );

    // Verify child process was re-spawned with a NEW PID
    let pid2 = server.child.lock().unwrap().id();
    assert_ne!(
        pid1, pid2,
        "Auto-recovery should re-spawn process with a new PID"
    );

    // Verify parameters remained strictly identical
    assert_eq!(server.config.params, params);

    // Verify newly re-spawned child process is actively running
    let status = server.child.lock().unwrap().try_wait().unwrap();
    assert!(
        status.is_none(),
        "Re-spawned child process should be currently running"
    );

    // Clean up
    server.kill();
}

#[test]
fn test_manual_kill_prevents_auto_restart() {
    let params = if cfg!(target_os = "windows") {
        vec![
            "ping".to_owned(),
            "127.0.0.1".to_owned(),
            "-n".to_owned(),
            "30".to_owned(),
        ]
    } else {
        vec!["sleep".to_owned(), "30".to_owned()]
    };

    let mut server =
        LogStream::spawn(&params, Path::new("."), None, None).expect("Failed to spawn process");

    let initial_pid = server.child.lock().unwrap().id();

    // Call kill() which sets manual_stop = true
    server.kill();
    assert!(*server.manual_stop.lock().unwrap());

    // Wait for monitoring loop to exit
    thread::sleep(Duration::from_millis(1500));

    // Verify child is terminated and PID did not change (no auto-restart)
    let current_pid = server.child.lock().unwrap().id();
    assert_eq!(
        initial_pid, current_pid,
        "Manual kill should not trigger process re-spawn"
    );

    assert_eq!(server.metrics.lock().unwrap().status, "STOPPED");
}
