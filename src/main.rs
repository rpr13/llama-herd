use llama_herd::tui::app::AppState;
use llama_herd::{cli, config, discovery, launcher, tui};
use std::collections::HashMap;
use std::path::PathBuf;

fn show_help() {
    println!(concat!(
        "🦙 LLAMA-HERD v",
        env!("CARGO_PKG_VERSION"),
        " - Rust Edition\n",
        "Native cross-platform server launcher for llama.cpp\n\n",
        "Usage:\n",
        "  llama-herd [options]\n\n",
        "Options:\n",
        "  -h, --help     Show this help documentation.\n",
        "  -c, --cli      Force classic interactive CLI terminal menus.\n",
        "  --ini          Generate models-preset.ini dynamically and exit.\n\n",
        "Environment:\n",
        "  LLAMA_PATH     Base directory containing llama-server and models/ subdirectory.\n",
        "                 Default fallbacks: d:/llama, c:/llama, or ~/llama.\n\n",
        "Global Configuration (config.toml):\n",
        "  Place in LLAMA_PATH to share parameters across all presets. Key settings:\n",
        "  host = \"0.0.0.0\"        # Host IP to bind the server\n",
        "  port = 8080             # Port to listen on\n",
        "  flash_attn = \"auto\"     # Enable flash attention (\"auto\", \"1\", \"0\")\n",
        "  kv_quant = \"q8_0\"       # KV cache quantization (\"q8_0\", \"f16\", etc.)\n",
        "  models_max = 1          # Max active models loaded concurrently in Router Mode\n",
        "  batch_size = 256        # Prompt processing batch size (-b)\n",
        "  ubatch_size = 256       # Prompt processing micro-batch size (-ub)\n",
        "  threads = 8             # Explicit CPU thread count override (-t)\n",
        "  ui = true                # Global Web UI enablement toggle\n"
    ));

    std::process::exit(0);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (use_cli, should_show_help, generate_ini) = config::parse_args(&args);

    if should_show_help {
        show_help();
    }

    let base_dir = match config::resolve_base_dir() {
        Some(dir) => match dir.canonicalize() {
            Ok(p) => p,
            Err(_) => dir,
        },
        None => {
            eprintln!(
                "Error: LLAMA_PATH environment variable is not set and no default base directory containing a 'models/' folder was found."
            );
            std::process::exit(1);
        }
    };

    let models_dir = base_dir.join("models");

    let global_config_path = base_dir.join("config.toml");
    let global_config = if global_config_path.exists() {
        config::load_toml_safe(&global_config_path)
    } else {
        HashMap::new()
    };

    if generate_ini {
        let preset_ini_path =
            discovery::generate_presets_ini(&models_dir, &base_dir, &global_config);
        println!(
            "Generated presets configuration at '{}'",
            preset_ini_path.to_string_lossy()
        );
        std::process::exit(0);
    }

    let server_exe = config::get_server_executable(&base_dir);

    if !server_exe.is_file() {
        eprintln!(
            "Error: llama-server executable not found at '{}'.",
            server_exe.to_string_lossy()
        );
        std::process::exit(1);
    }

    // Terminate any stray servers on startup
    launcher::kill_existing_servers();

    // Dynamically scan model directories and generate settings
    let preset_ini_path = discovery::generate_presets_ini(&models_dir, &base_dir, &global_config);

    // Retrieve active presets list
    let presets = discovery::discover_presets_from_ini(&preset_ini_path);
    if presets.is_empty() {
        eprintln!("Error: No valid presets discovered in models-preset.ini.");
        std::process::exit(1);
    }

    if use_cli {
        let title = format!(
            "LLAMA SERVER LAUNCHER v{} (Classic CLI)",
            env!("CARGO_PKG_VERSION")
        );
        let divider = "-".repeat(title.len());
        println!("{}", title);
        println!("{}", divider);
        println!("Select Server Mode:");
        println!("[1] Router Mode (Multi-model: dynamic load/unload)");
        println!("[2] Single Model Mode (Interactive selection)");

        print!("Select mode (1): ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut mode_choice = String::new();
        let _ = std::io::stdin().read_line(&mut mode_choice);
        let mode_choice = mode_choice.trim();

        let params = if mode_choice == "2" {
            let (preset_name, selected_model) = cli::prompt_preset_selection(&presets);
            let ini_settings = match config::load_settings_from_ini(&preset_name, &preset_ini_path)
            {
                Some(settings) => settings,
                None => {
                    eprintln!("Error: Preset settings for '{}' not found.", preset_name);
                    std::process::exit(1);
                }
            };

            let assets = discovery::discover_assets(&selected_model, &models_dir);

            // Build defaults from INI settings
            let ctx_val = ini_settings
                .get("ctx-size")
                .map(|s| serde_json::Value::String(s.clone()))
                .unwrap_or_else(|| {
                    assets
                        .config
                        .get("ctx_size")
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::String("131072".to_string()))
                });
            let ctx = config::parse_ctx(&ctx_val);

            let ngl = ini_settings
                .get("n-gpu-layers")
                .cloned()
                .unwrap_or_else(|| "auto".to_string());
            let ui = !ini_settings.contains_key("no-ui")
                && ini_settings.get("no-ui").map(|s| s.as_str()) != Some("true");

            let mmproj = ini_settings.get("mmproj").map(PathBuf::from);
            let draft_model = ini_settings.get("model-draft").map(PathBuf::from);
            let draft_ngl = ini_settings
                .get("gpu-layers-draft")
                .cloned()
                .unwrap_or_else(|| "".to_string());

            let base_settings = config::UserSettings {
                ctx,
                ngl,
                ui,
                mmproj,
                draft_model,
                draft_ngl,
            };

            print!(
                "Mode: Default ({}k, ngl:{}, ui:{}) or Custom? (D/c): ",
                base_settings.ctx / 1024,
                base_settings.ngl,
                if base_settings.ui {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let mut choice = String::new();
            let _ = std::io::stdin().read_line(&mut choice);
            let choice = choice.trim().to_lowercase();

            let final_settings = if choice == "c" {
                cli::prompt_custom_settings(base_settings, &assets, &models_dir, &selected_model)
            } else {
                base_settings
            };

            launcher::build_launch_parameters(
                &server_exe,
                &selected_model,
                &assets,
                &final_settings,
                &global_config,
            )
        } else {
            launcher::build_router_launch_parameters(&server_exe, &preset_ini_path, &global_config)
        };

        cli::run_cli_session(&params, &base_dir);
    } else {
        // Run Ratatui Dashboard
        let app_state = AppState::new(
            presets,
            models_dir,
            base_dir,
            preset_ini_path,
            global_config,
            server_exe,
        );

        if let Err(err) = tui::run_tui(app_state) {
            eprintln!("TUI Error: {}", err);
            launcher::kill_existing_servers();
            std::process::exit(1);
        }
    }

    // Clean up server on normal exit
    launcher::kill_existing_servers();
}
