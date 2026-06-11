//! `LlamaHerd` binary entrypoint.
//! Handles command-line arguments, global configuration loading, and starts the TUI or setup wizard.

#![forbid(unsafe_code)]
#![allow(clippy::multiple_crate_versions)]
use llama_herd::tui::app::AppState;
use llama_herd::{config, discovery, launcher, tui};

use std::collections::HashMap;

#[allow(clippy::exit)]
fn show_help() {
    println!(concat!(
        "🦙 LLAMA-HERD ",
        env!("APP_VERSION"),
        " - Rust Edition\n",
        "Native cross-platform server launcher for llama.cpp\n\n",
        "Usage:\n",
        "  llama-herd [options]\n\n",
        "Options:\n",
        "  -h, --help     Show this help documentation.\n",
        "  --ini          Generate models-preset.ini dynamically inside the models directory and exit.\n\n",
        "Configuration (config.toml):\n",
        "  Configuration is stored in a platform-specific directory:\n",
        "  - Unix: ~/.config/llama-herd/config.toml\n",
        "  - Windows: %APPDATA%\\llama-herd\\config.toml\n\n",
        "  The 'llama-server' path and 'models-dir' can be configured there.\n",
        "  If missing, an interactive setup wizard will guide you on startup.\n\n",
        "Global Settings in config.toml:\n",
        "  host = \"127.0.0.1\"      # Host IP to bind the server\n",
        "  port = 8080             # Port to listen on\n",
        "  flash_attn = \"auto\"     # Enable flash attention (\"auto\", \"1\", \"0\")\n",
        "  cache_type_k = \"f16\"    # KV cache key quantization (\"f16\", \"q8_0\", etc.)\n",
        "  cache_type_v = \"f16\"    # KV cache value quantization (\"f16\", \"q8_0\", etc.)\n",
        "  kv_unified = true       # Enable unified KV cache (\"true\", \"false\")\n",
        "  models_max = 1          # Max active models loaded concurrently in Router Mode\n",
        "  batch_size = 2048       # Prompt processing batch size (-b)\n",
        "  ubatch_size = 512       # Prompt processing micro-batch size (-ub)\n",
        "  threads = -1            # Explicit CPU thread count override (-t), -1 = auto\n",
        "  api_key = \"disabled\"    # API key for authorization (\"disabled\" or key)\n",
        "  metrics = false         # Enable Prometheus metrics endpoint (\"true\", \"false\")\n",
        "  ui = true               # Global Web UI enablement toggle\n"
    ));

    std::process::exit(0);
}

#[allow(clippy::exit)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (should_show_help, generate_ini) = config::parse_args(&args);

    if should_show_help {
        show_help();
    }

    let lh_dir = config::get_llama_herd_dir();
    if !lh_dir.exists() {
        let _ = std::fs::create_dir_all(&lh_dir);
    }

    let global_config_path = lh_dir.join("config.toml");
    let global_config = if global_config_path.exists() {
        match config::load_toml_safe(&global_config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "CRITICAL: Failed to load config.toml: {e}. Aborting to prevent unsafe defaults."
                );
                std::process::exit(1);
            }
        }
    } else {
        HashMap::new()
    };

    let (server_exe, models_dir, global_config) = if let (Some(exe), Some(dir)) = (
        config::resolve_server_executable(&global_config),
        config::resolve_models_dir(&global_config),
    ) {
        (exe, dir, global_config)
    } else {
        llama_herd::setup::run_wizard(&lh_dir, global_config).unwrap_or_else(|| {
            eprintln!(
                "Setup cancelled or failed. Please configure 'config.toml' manually or run the wizard again."
            );
            std::process::exit(0);
        })
    };

    let models_dir = std::path::absolute(&models_dir).unwrap_or(models_dir);

    let preset_ini_path = models_dir.join("models-preset.ini");

    if generate_ini {
        if let Err(e) =
            discovery::generate_presets_ini(&models_dir, &preset_ini_path, &global_config)
        {
            eprintln!("CRITICAL: Failed to generate presets configuration: {e}.");
            std::process::exit(1);
        }
        println!(
            "Generated presets configuration at '{}'",
            preset_ini_path.to_string_lossy()
        );
        std::process::exit(0);
    }

    // Terminate any stray servers on startup
    launcher::kill_existing_servers();

    // Dynamically scan model directories and generate settings
    if let Err(e) = discovery::generate_presets_ini(&models_dir, &preset_ini_path, &global_config) {
        eprintln!("CRITICAL: Failed to generate presets configuration: {e}.");
        std::process::exit(1);
    }

    // Retrieve active presets list
    let presets = discovery::discover_presets_from_ini(&preset_ini_path);
    if presets.is_empty() {
        eprintln!("Error: No valid presets discovered in models-preset.ini.");
        std::process::exit(1);
    }

    // Load Theme
    let theme_path = lh_dir.join("theme.toml");
    let theme = if theme_path.exists() {
        tui::theme::Theme::load(&theme_path)
    } else {
        tui::theme::Theme::default()
    };

    // Run Ratatui Dashboard
    let app_state = AppState::new(
        presets,
        models_dir,
        preset_ini_path,
        global_config,
        server_exe,
        theme,
    );

    if let Err(err) = tui::run_tui(app_state) {
        eprintln!("TUI Error: {err}");
        launcher::kill_existing_servers();
        std::process::exit(1);
    }

    // Clean up server on normal exit
    launcher::kill_existing_servers();
}
