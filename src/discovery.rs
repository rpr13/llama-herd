pub use crate::config::discover_assets;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn clean_model_id(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let with_hyphens = stem.replace('.', "-");

    let re = regex::Regex::new(r"([b-zB-Z])(\d+)([a-zA-Z])").unwrap();
    let formatted = re.replace_all(&with_hyphens, "$1-$2$3");

    let re_multiple = regex::Regex::new(r"-+").unwrap();
    re_multiple.replace_all(&formatted, "-").into_owned()
}

pub fn insert_variant_suffix(name: &str, suffix: &str) -> String {
    let re = regex::Regex::new(r"-([a-zA-Z0-9_]+)$").unwrap();
    let rep = format!("-{}-${{1}}", suffix);
    re.replace(name, rep.as_str()).into_owned()
}

pub fn find_matching_mmproj(model_path: &Path, mmproj_files: &[PathBuf]) -> Option<PathBuf> {
    if mmproj_files.is_empty() {
        return None;
    }
    let model_name_lower = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    for mf in mmproj_files {
        let stem = mf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let tokens: Vec<&str> = stem
            .split('-')
            .filter(|&t| t != "mmproj" && t != "q8_0" && t != "f16" && t != "q4_k_m")
            .collect();
        if !tokens.is_empty() && tokens.iter().all(|&t| model_name_lower.contains(t)) {
            return Some(mf.clone());
        }
    }
    if mmproj_files.len() == 1 {
        return Some(mmproj_files[0].clone());
    }
    None
}

pub fn find_matching_draft(model_path: &Path, draft_files: &[PathBuf]) -> Option<PathBuf> {
    let model_name_lower = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let ignore_tokens = vec![
        "assistant",
        "draft",
        "mtp",
        "gguf",
        "it",
        "chat",
        "instruct",
        "q8_0",
        "f16",
        "q4_k_m",
        "q4_0",
        "q4_1",
        "q5_0",
        "q5_1",
        "q6_k",
    ];
    let re = regex::Regex::new(r"[-._]").unwrap();
    for df in draft_files {
        let df_stem_lower = df
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let draft_tokens: Vec<&str> = re
            .split(&df_stem_lower)
            .filter(|&t| !t.is_empty() && !ignore_tokens.contains(&t))
            .collect();
        if !draft_tokens.is_empty() && draft_tokens.iter().all(|&t| model_name_lower.contains(t)) {
            return Some(df.clone());
        }
    }
    None
}

pub fn discover_presets_from_ini(preset_path: &Path) -> Vec<(String, PathBuf)> {
    if !preset_path.exists() {
        return Vec::new();
    }
    if let Ok(content) = std::fs::read_to_string(preset_path) {
        let sections = crate::config::parse_settings_ini(&content);
        let mut presets = Vec::new();

        let mut sorted_keys: Vec<&String> = sections.keys().filter(|&k| k != "*").collect();
        sorted_keys.sort();

        for section in sorted_keys {
            if let Some(map) = sections.get(section) {
                if map.get("is-draft").map(|v| v.as_str()) == Some("true") {
                    continue;
                }
                if let Some(model_val) = map.get("model") {
                    presets.push((section.clone(), PathBuf::from(model_val)));
                }
            }
        }
        return presets;
    }
    Vec::new()
}

pub fn generate_presets_ini(
    models_dir: &Path,
    base_dir: &Path,
    global_config: &HashMap<String, serde_json::Value>,
) -> PathBuf {
    let mut all_ggufs = Vec::new();
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
            {
                all_ggufs.push(path);
            }
        }
    }
    all_ggufs.sort();

    let mut toml_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                toml_files.push(path);
            }
        }
    }
    toml_files.sort_by_key(|a| std::cmp::Reverse(a.file_name().unwrap_or_default().len()));

    let mut main_models = Vec::new();
    let mut draft_files = Vec::new();

    for model in &all_ggufs {
        let stem = model
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let mut is_draft = false;

        for js in &toml_files {
            let js_stem = js
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if stem.starts_with(&js_stem) {
                let cfg = crate::config::load_toml_silent(js);
                if cfg.get("lh-is-draft").and_then(|v| v.as_bool()) == Some(true)
                    || cfg.get("lh-is-draft-only").and_then(|v| v.as_bool()) == Some(true)
                    || cfg.get("is-draft").and_then(|v| v.as_bool()) == Some(true)
                    || cfg.get("is-draft-only").and_then(|v| v.as_bool()) == Some(true)
                {
                    is_draft = true;
                }
                break;
            }
        }

        if is_draft {
            draft_files.push(model.clone());
        } else {
            main_models.push(model.clone());
        }
    }

    let mut default_candidates = Vec::new();
    for model in &main_models {
        let stem = model
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        for js in &toml_files {
            let js_stem = js
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if stem.starts_with(&js_stem) {
                let cfg = crate::config::load_toml_silent(js);
                if cfg.get("lh-is-default").and_then(|v| v.as_bool()) == Some(true)
                    || cfg.get("is-default").and_then(|v| v.as_bool()) == Some(true)
                {
                    default_candidates.push(model.clone());
                }
                break;
            }
        }
    }

    let designated_default = if !default_candidates.is_empty() {
        default_candidates
            .iter()
            .min_by_key(|m| {
                std::fs::metadata(m)
                    .map(|meta| meta.len())
                    .unwrap_or(u64::MAX)
            })
            .cloned()
    } else if !main_models.is_empty() {
        main_models
            .iter()
            .min_by_key(|m| {
                std::fs::metadata(m)
                    .map(|meta| meta.len())
                    .unwrap_or(u64::MAX)
            })
            .cloned()
    } else {
        None
    };

    let mut mmproj_files = Vec::new();
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
                mmproj_files.push(path);
            }
        }
    }
    mmproj_files.sort();

    let kv_quant = global_config
        .get("kv_quant")
        .and_then(|v| v.as_str())
        .unwrap_or("q8_0");

    let mut lines = Vec::new();
    lines.push("version = 1".to_string());
    lines.push("; Global settings shared across all presets".to_string());
    lines.push("[*]".to_string());
    lines.push("flash-attn = auto".to_string());
    lines.push("jinja = true".to_string());
    lines.push(format!("cache-type-k = {}", kv_quant));
    lines.push(format!("cache-type-v = {}", kv_quant));
    lines.push("kv-unified = true".to_string());
    lines.push("".to_string());

    let mut default_preset_lines = Vec::new();

    for model_path in &main_models {
        let assets = discover_assets(model_path, models_dir);
        let clean_name = clean_model_id(model_path);
        let is_default = Some(model_path) == designated_default.as_ref();

        let ctx_val = assets
            .config
            .get("lh-ctx-size")
            .or_else(|| assets.config.get("ctx-size"))
            .unwrap_or(&serde_json::Value::String("131072".to_string()))
            .clone();
        let ctx_size = crate::config::parse_ctx(&ctx_val);

        let mut ngl = assets
            .config
            .get("lh-ngl")
            .or_else(|| assets.config.get("ngl"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else {
                    v.as_i64().map(|i| i.to_string())
                }
            })
            .unwrap_or_else(|| "auto".to_string());
        if ngl == "auto"
            && let Some(total) = assets
                .config
                .get("lh-total-layers")
                .or_else(|| assets.config.get("total-layers"))
                .and_then(|v| v.as_u64())
        {
            ngl = total.to_string();
        }

        let temp = assets
            .config
            .get("lh-temp")
            .or_else(|| assets.config.get("temp"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8);
        let top_p = assets
            .config
            .get("lh-top-p")
            .or_else(|| assets.config.get("top-p"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.95);
        let top_k = assets
            .config
            .get("lh-top-k")
            .or_else(|| assets.config.get("top-k"))
            .and_then(|v| v.as_i64())
            .unwrap_or(40);
        let reasoning = assets
            .config
            .get("lh-reasoning")
            .or_else(|| assets.config.get("reasoning"))
            .and_then(|v| v.as_str())
            .unwrap_or("auto");

        let mut mmproj_file = None;
        let mmproj_val = assets
            .config
            .get("lh-mmproj")
            .or_else(|| assets.config.get("mmproj"));
        if let Some(mmproj_cfg) = mmproj_val.and_then(|v| v.as_str()) {
            let mmproj_path = models_dir.join(mmproj_cfg);
            if mmproj_path.exists() {
                mmproj_file = Some(mmproj_path);
            } else {
                let direct_path = PathBuf::from(mmproj_cfg);
                if direct_path.exists() {
                    mmproj_file = Some(direct_path);
                }
            }
        }
        if mmproj_file.is_none() {
            mmproj_file = find_matching_mmproj(model_path, &mmproj_files);
        }

        let mut draft_file = None;
        let draft_val = assets
            .config
            .get("lh-draft")
            .or_else(|| assets.config.get("draft-model"));
        if let Some(draft_cfg) = draft_val.and_then(|v| v.as_str())
            && !draft_cfg.to_lowercase().eq("none")
            && !draft_cfg.to_lowercase().eq("false")
            && !draft_cfg.is_empty()
        {
            let draft_path = models_dir.join(draft_cfg);
            if draft_path.exists() {
                draft_file = Some(draft_path);
            } else {
                let direct_path = PathBuf::from(draft_cfg);
                if direct_path.exists() {
                    draft_file = Some(direct_path);
                }
            }
        }
        if !assets.config.contains_key("lh-draft")
            && !assets.config.contains_key("draft-model")
            && draft_file.is_none()
        {
            draft_file = find_matching_draft(model_path, &draft_files);
        }

        let mut presets_to_generate = vec![(clean_name.clone(), false, false)];
        if draft_file.is_some() && mmproj_file.is_some() {
            presets_to_generate.push((insert_variant_suffix(&clean_name, "vision"), false, true));
            presets_to_generate.push((insert_variant_suffix(&clean_name, "draft"), true, false));
            presets_to_generate.push((
                insert_variant_suffix(&clean_name, "draft-vision"),
                true,
                true,
            ));
        } else if draft_file.is_some() {
            presets_to_generate.push((insert_variant_suffix(&clean_name, "draft"), true, false));
        } else if mmproj_file.is_some() {
            presets_to_generate.push((insert_variant_suffix(&clean_name, "vision"), false, true));
        }

        for (preset_name, use_draft, use_vision) in presets_to_generate {
            let mut current_preset = Vec::new();
            current_preset.push(format!("; --- {} ---", preset_name));
            current_preset.push(format!("[{}]", preset_name));
            current_preset.push(format!(
                "model = {}",
                model_path.to_string_lossy().replace('\\', "/")
            ));

            if let Some(template) = &assets.jinja_template {
                current_preset.push(format!(
                    "chat-template-file = {}",
                    template.to_string_lossy().replace('\\', "/")
                ));
            }

            current_preset.push(format!("ctx-size = {}", ctx_size));
            current_preset.push(format!("n-gpu-layers = {}", ngl));
            current_preset.push(format!("temp = {}", temp));
            current_preset.push(format!("top-p = {}", top_p));
            current_preset.push(format!("top-k = {}", top_k));

            if reasoning != "auto" {
                current_preset.push(format!("reasoning = {}", reasoning));
                if reasoning == "on" {
                    current_preset.push("reasoning-format = deepseek".to_string());
                }
            }

            // --- Passthrough for other keys ---
            let mut sorted_cfg_keys: Vec<&String> = assets.config.keys().collect();
            sorted_cfg_keys.sort();
            for k in sorted_cfg_keys {
                if crate::config::is_restricted_key(k) {
                    continue;
                }
                if let Some(ini_key) = crate::config::format_ini_key(k) {
                    let val = &assets.config[k];
                    if let Some(s) = val.as_str() {
                        current_preset.push(format!("{} = {}", ini_key, s));
                    } else if let Some(b) = val.as_bool() {
                        current_preset.push(format!("{} = {}", ini_key, b));
                    } else if let Some(n) = val.as_i64() {
                        current_preset.push(format!("{} = {}", ini_key, n));
                    } else if let Some(f) = val.as_f64() {
                        current_preset.push(format!("{} = {}", ini_key, f));
                    } else if let Some(arr) = val.as_array() {
                        let items: Vec<String> = arr
                            .iter()
                            .map(|v| {
                                v.as_str()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| v.to_string())
                            })
                            .collect();
                        current_preset.push(format!("{} = {}", ini_key, items.join(",")));
                    }
                }
            }

            if use_vision && let Some(ref mm) = mmproj_file {
                current_preset.push(format!(
                    "mmproj = {}",
                    mm.to_string_lossy().replace('\\', "/")
                ));
            }

            if use_draft && let Some(ref df) = draft_file {
                let df_stem = df
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let mut draft_config = HashMap::new();
                for js in &toml_files {
                    let js_stem = js
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if df_stem.starts_with(&js_stem) {
                        draft_config = crate::config::load_toml_silent(js);
                        break;
                    }
                }

                let mut spec_type = draft_config
                    .get("lh-spec-type")
                    .or_else(|| draft_config.get("spec-type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("draft-mtp");
                if spec_type == "mtp" {
                    spec_type = "draft-mtp";
                }

                let spec_draft_n_max = draft_config
                    .get("lh-spec-draft-n-max")
                    .or_else(|| draft_config.get("spec-draft-n-max"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4);
                let spec_draft_p_min = draft_config
                    .get("lh-spec-draft-p-min")
                    .or_else(|| draft_config.get("spec-draft-p-min"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let d_ngl = draft_config
                    .get("lh-total-layers")
                    .or_else(|| draft_config.get("total-layers"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4);

                current_preset.push(format!(
                    "model-draft = {}",
                    df.to_string_lossy().replace('\\', "/")
                ));
                current_preset.push(format!("spec-type = {}", spec_type));
                current_preset.push(format!("spec-draft-n-max = {}", spec_draft_n_max));
                current_preset.push(format!("spec-draft-p-min = {}", spec_draft_p_min));
                current_preset.push(format!("gpu-layers-draft = {}", d_ngl));
            }

            current_preset.push("".to_string());

            if is_default && preset_name == clean_name {
                default_preset_lines = current_preset
                    .iter()
                    .map(|line| line.replace(&format!("[{}]", clean_name), "[default]"))
                    .collect();
            }

            lines.extend(current_preset);
        }
    }

    if !default_preset_lines.is_empty() {
        lines.extend(default_preset_lines);
    }

    let output_path = base_dir.join("models-preset.ini");
    if let Err(e) = std::fs::write(&output_path, lines.join("\n")) {
        println!("[!] Failed to write models-preset.ini: {}", e);
    } else {
        println!("[*] Dynamically generated router presets: models-preset.ini");
    }
    output_path
}
