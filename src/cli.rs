use crate::config::{ModelAssets, UserSettings, calculate_ngl, parse_ctx_str};
use crate::discovery::discover_assets;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn clear_screen() {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd").args(["/c", "cls"]).status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("clear").status();
    }
}

pub fn prompt_preset_selection(presets: &[(String, PathBuf)]) -> (String, PathBuf) {
    let mut input = io::stdin().lock();
    let mut output = io::stdout();
    prompt_preset_selection_internal(presets, &mut input, &mut output)
}

pub fn prompt_preset_selection_internal<R: io::BufRead, W: io::Write>(
    presets: &[(String, PathBuf)],
    reader: &mut R,
    writer: &mut W,
) -> (String, PathBuf) {
    clear_screen();
    for (i, (name, _)) in presets.iter().enumerate() {
        let _ = writeln!(writer, "[{}] {}", i + 1, name);
    }
    let _ = write!(writer, "Select model (1): ");
    let _ = writer.flush();
    let mut input = String::new();
    let _ = reader.read_line(&mut input);
    let input = input.trim();
    if let Ok(num) = input.parse::<usize>()
        && num >= 1
        && num <= presets.len()
    {
        return presets[num - 1].clone();
    }
    presets[0].clone()
}

pub fn prompt_custom_settings(
    default_settings: UserSettings,
    assets: &ModelAssets,
    models_dir: &Path,
    selected_model: &Path,
) -> UserSettings {
    let mut input = io::stdin().lock();
    let mut output = io::stdout();
    prompt_custom_settings_internal(
        default_settings,
        assets,
        models_dir,
        selected_model,
        &mut input,
        &mut output,
    )
}

pub fn prompt_custom_settings_internal<R: io::BufRead, W: io::Write>(
    default_settings: UserSettings,
    assets: &ModelAssets,
    models_dir: &Path,
    selected_model: &Path,
    reader: &mut R,
    writer: &mut W,
) -> UserSettings {
    let total_layers = assets
        .config
        .get("total-layers")
        .and_then(|v| v.as_u64().map(|i| i as usize));

    let mut current_settings = default_settings;

    // mmproj selection
    let mut vision_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gguf")
                && path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("mmproj")
            {
                vision_files.push(path);
            }
        }
    }
    vision_files.sort();

    if !vision_files.is_empty() && current_settings.mmproj.is_none() {
        let _ = writeln!(writer, "Available vision modules:");
        for (i, vf) in vision_files.iter().enumerate() {
            let _ = writeln!(
                writer,
                "[{}] {}",
                i + 1,
                vf.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        let _ = write!(writer, "Select vision module (none): ");
        let _ = writer.flush();
        let mut input = String::new();
        let _ = reader.read_line(&mut input);
        let input = input.trim();
        if let Ok(num) = input.parse::<usize>()
            && num >= 1
            && num <= vision_files.len()
        {
            current_settings.mmproj = Some(vision_files[num - 1].clone());
        }
    }

    // Draft Model (MTP) selection
    let mut draft_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gguf")
                && !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("mmproj")
                && path != selected_model
            {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let mut is_draft = false;
                if let Ok(sub_entries) = std::fs::read_dir(models_dir) {
                    for se in sub_entries.flatten() {
                        let sp = se.path();
                        if sp.extension().and_then(|s| s.to_str()) == Some("toml") {
                            let js_stem = sp
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if stem.starts_with(&js_stem) {
                                let cfg = crate::config::load_toml_silent(&sp);
                                if cfg.get("is-draft").and_then(|v| v.as_bool()) == Some(true) {
                                    is_draft = true;
                                }
                                break;
                            }
                        }
                    }
                }
                if is_draft {
                    draft_files.push(path);
                }
            }
        }
    }
    draft_files.sort();

    if !draft_files.is_empty() && current_settings.draft_model.is_none() {
        let _ = writeln!(writer, "-----------------");
        for (i, df) in draft_files.iter().enumerate() {
            let _ = writeln!(
                writer,
                "[{}] {}",
                i + 1,
                df.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        let _ = write!(writer, "Select MTP draft model (none): ");
        let _ = writer.flush();
        let mut input = String::new();
        let _ = reader.read_line(&mut input);
        let input = input.trim();
        if let Ok(num) = input.parse::<usize>()
            && num >= 1
            && num <= draft_files.len()
        {
            let selected_draft = draft_files[num - 1].clone();
            current_settings.draft_model = Some(selected_draft.clone());

            let draft_assets = discover_assets(&selected_draft, models_dir);
            let d_total_layers = draft_assets
                .config
                .get("total-layers")
                .and_then(|v| v.as_u64().map(|i| i as usize));

            let d_ngl_hint = match d_total_layers {
                Some(layers) => format!("subtract from {}", layers),
                None => "total-layers missing".to_string(),
            };

            let _ = write!(
                writer,
                "Draft GPU Layers (auto, use --X to {}): ",
                d_ngl_hint
            );
            let _ = writer.flush();
            let mut draft_ngl_in = String::new();
            let _ = reader.read_line(&mut draft_ngl_in);
            let draft_ngl_in = draft_ngl_in.trim();
            current_settings.draft_ngl = calculate_ngl(draft_ngl_in, "auto", d_total_layers);
        }
    }

    // GPU Layers NGL
    let ngl_hint = match total_layers {
        Some(layers) => format!("subtract from {}", layers),
        None => "total-layers missing".to_string(),
    };
    let _ = write!(
        writer,
        "GPU Layers ({}, use --X to {}): ",
        current_settings.ngl, ngl_hint
    );
    let _ = writer.flush();
    let mut ngl_in = String::new();
    let _ = reader.read_line(&mut ngl_in);
    let ngl_in = ngl_in.trim();
    current_settings.ngl = calculate_ngl(ngl_in, &current_settings.ngl, total_layers);

    // Context
    let _ = write!(writer, "Context size ({}k): ", current_settings.ctx / 1024);
    let _ = writer.flush();
    let mut ctx_in = String::new();
    let _ = reader.read_line(&mut ctx_in);
    let ctx_in = ctx_in.trim();
    if !ctx_in.is_empty() {
        current_settings.ctx = parse_ctx_str(ctx_in);
    }

    // UI
    let prompt_ui = if current_settings.ui { "Y/n" } else { "y/N" };
    let _ = write!(writer, "Enable UI? ({}): ", prompt_ui);
    let _ = writer.flush();
    let mut ui_in = String::new();
    let _ = reader.read_line(&mut ui_in);
    let ui_in = ui_in.trim().to_lowercase();
    if !ui_in.is_empty() {
        current_settings.ui = ui_in.starts_with('y') || ui_in.is_empty();
    }

    current_settings
}

pub fn run_cli_session(params: &[String], base_dir: &Path) {
    clear_screen();
    let safe_cmd_str = params.join(" ");
    println!("Command: {}\n------------------------------", safe_cmd_str);

    let mut cmd = Command::new(&params[0]);
    cmd.args(&params[1..]).current_dir(base_dir);

    crate::launcher::kill_existing_servers();

    match cmd.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => {
            eprintln!("Execution failed: {}", e);
        }
    }
}
