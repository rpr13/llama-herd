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

    let clean_tokens = |name: &str| -> Vec<String> {
        let size_re = regex::Regex::new(r"\b\d+(?:\.\d+)?(?:x\d+)?[bm]\b").unwrap();
        let quant_re =
            regex::Regex::new(r"\b(?:q\d+(?:_?[k\d](?:_[sml])?)?|f16|fp16|bf16)\b").unwrap();

        let cleaned_size = size_re.replace_all(name, " ");
        let cleaned_quant = quant_re.replace_all(&cleaned_size, " ");

        let ignore_tokens = [
            "assistant",
            "draft",
            "mtp",
            "gguf",
            "it",
            "chat",
            "instruct",
            "vision",
        ];

        let split_re = regex::Regex::new(r"[-._\s]+").unwrap();
        split_re
            .split(&cleaned_quant)
            .filter(|&t| !t.is_empty() && !ignore_tokens.contains(&t))
            .map(|t| t.to_string())
            .collect()
    };

    let main_tokens = clean_tokens(&model_name_lower);

    for df in draft_files {
        let df_stem_lower = df
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let draft_tokens = clean_tokens(&df_stem_lower);

        if !draft_tokens.is_empty() && draft_tokens.iter().all(|t| main_tokens.contains(t)) {
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
    output_path: &Path,
    global_config: &HashMap<String, serde_json::Value>,
) -> Result<PathBuf, std::io::Error> {
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
                if let Some(lh) = cfg.get("llama-herd")
                    && (lh.get("is-draft").and_then(|v| v.as_bool()) == Some(true)
                        || lh.get("is-draft-only").and_then(|v| v.as_bool()) == Some(true))
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
                if let Some(lh) = cfg.get("llama-herd")
                    && lh.get("is-default").and_then(|v| v.as_bool()) == Some(true)
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

    let _get_global_lh = |key: &str| -> Option<&serde_json::Value> {
        global_config
            .get("llama-herd")
            .and_then(|lh| lh.get(key))
            .or_else(|| global_config.get(key))
    };
    let get_global_long = |key: &str| -> Option<&serde_json::Value> {
        global_config
            .get("llama-server-long")
            .and_then(|l| l.get(key))
            .or_else(|| global_config.get(key))
    };

    let cache_type_k = get_global_long("cache-type-k")
        .or_else(|| get_global_long("kv-quant"))
        .or_else(|| get_global_long("kv_quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("f16");

    let cache_type_v = get_global_long("cache-type-v")
        .or_else(|| get_global_long("kv-quant"))
        .or_else(|| get_global_long("kv_quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("f16");

    let ctx_checkpoints = get_global_long("ctx-checkpoints").and_then(|v| {
        if let Some(s) = v.as_str() {
            s.parse::<i64>().ok()
        } else if let Some(n) = v.as_u64() {
            Some(n as i64)
        } else {
            v.as_i64()
        }
    });

    let checkpoint_min_step = get_global_long("checkpoint-min-step").and_then(|v| {
        if let Some(s) = v.as_str() {
            s.parse::<i64>().ok()
        } else if let Some(n) = v.as_u64() {
            Some(n as i64)
        } else {
            v.as_i64()
        }
    });

    let no_mmap = get_global_long("no-mmap")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut lines = Vec::new();
    lines.push("version = 1".to_string());
    lines.push("; Global settings shared across all presets".to_string());
    lines.push("[*]".to_string());
    lines.push("flash-attn = auto".to_string());
    lines.push("jinja = true".to_string());
    lines.push(format!("cache-type-k = {}", cache_type_k));
    lines.push(format!("cache-type-v = {}", cache_type_v));
    lines.push("kv-unified = true".to_string());
    if let Some(checkpoints) = ctx_checkpoints {
        lines.push(format!("ctx-checkpoints = {}", checkpoints));
    }
    if let Some(step) = checkpoint_min_step {
        lines.push(format!("checkpoint-min-step = {}", step));
    }
    if no_mmap {
        lines.push("no-mmap = true".to_string());
    }
    lines.push("".to_string());

    let mut default_preset_lines = Vec::new();

    for model_path in &main_models {
        let assets = discover_assets(model_path, models_dir);
        let clean_name = clean_model_id(model_path);
        let is_default = Some(model_path) == designated_default.as_ref();

        let get_lh_val = |key: &str| -> Option<&serde_json::Value> {
            assets.config.get("llama-herd").and_then(|lh| lh.get(key))
        };
        let get_long_val = |key: &str| -> Option<&serde_json::Value> {
            assets
                .config
                .get("llama-server-long")
                .and_then(|l| l.get(key))
                .or_else(|| assets.config.get(key))
        };

        let ctx_val = get_lh_val("ctx-size")
            .or_else(|| get_long_val("ctx-size"))
            .unwrap_or(&serde_json::Value::String("131072".to_string()))
            .clone();
        let ctx_size = crate::config::parse_ctx(&ctx_val).unwrap_or(131072);

        let mut ngl = get_lh_val("ngl")
            .or_else(|| get_long_val("ngl"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else {
                    v.as_i64().map(|i| i.to_string())
                }
            })
            .unwrap_or_else(|| "auto".to_string());
        if ngl == "auto"
            && let Some(total) = get_lh_val("total-layers").and_then(|v| v.as_u64())
        {
            ngl = total.to_string();
        }

        let temp = get_lh_val("temp")
            .or_else(|| get_long_val("temp"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8);
        let top_p = get_lh_val("top-p")
            .or_else(|| get_long_val("top-p"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.95);
        let top_k = get_lh_val("top-k")
            .or_else(|| get_long_val("top-k"))
            .and_then(|v| v.as_i64())
            .unwrap_or(40);
        let reasoning = get_lh_val("reasoning")
            .or_else(|| get_long_val("reasoning"))
            .and_then(|v| v.as_str())
            .unwrap_or("auto");

        let model_ctx_checkpoints = get_lh_val("ctx-checkpoints")
            .or_else(|| get_long_val("ctx-checkpoints"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    s.parse::<i64>().ok()
                } else if let Some(n) = v.as_u64() {
                    Some(n as i64)
                } else {
                    v.as_i64()
                }
            });

        let model_checkpoint_min_step = get_lh_val("checkpoint-min-step")
            .or_else(|| get_long_val("checkpoint-min-step"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    s.parse::<i64>().ok()
                } else if let Some(n) = v.as_u64() {
                    Some(n as i64)
                } else {
                    v.as_i64()
                }
            });

        let model_no_mmap = get_lh_val("no-mmap")
            .or_else(|| get_long_val("no-mmap"))
            .and_then(|v| v.as_bool());

        let mut mmproj_file = None;
        if let Some(mmproj_cfg) = get_lh_val("mmproj")
            .or_else(|| get_long_val("mmproj"))
            .and_then(|v| v.as_str())
        {
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
        let draft_val = get_lh_val("draft").or_else(|| get_long_val("draft"));
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
        let draft_in_lh = assets
            .config
            .get("llama-herd")
            .and_then(|lh| lh.get("draft"))
            .is_some();
        let draft_in_root = assets.config.contains_key("draft");

        if !draft_in_lh && !draft_in_root && draft_file.is_none() {
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

            if let Some(checkpoints) = model_ctx_checkpoints {
                current_preset.push(format!("ctx-checkpoints = {}", checkpoints));
            }
            if let Some(step) = model_checkpoint_min_step {
                current_preset.push(format!("checkpoint-min-step = {}", step));
            }
            if let Some(mmap) = model_no_mmap {
                current_preset.push(format!("no-mmap = {}", mmap));
            }

            if reasoning != "auto" {
                current_preset.push(format!("reasoning = {}", reasoning));
                if reasoning == "on" {
                    current_preset.push("reasoning-format = deepseek".to_string());
                }
            }

            // Helper to check if a key is restricted and format/write it
            let write_long_option =
                |k: &str, val: &serde_json::Value, current_preset: &mut Vec<String>| {
                    if crate::config::is_restricted_key(k) {
                        return;
                    }
                    let ini_key = k;
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
                };

            // 1. Process root level passthrough keys
            let mut sorted_root_keys: Vec<&String> = assets.config.keys().collect();
            sorted_root_keys.sort();
            for k in sorted_root_keys {
                if k == "llama-herd" || k == "llama-server-short" || k == "llama-server-long" {
                    continue;
                }
                write_long_option(k, &assets.config[k], &mut current_preset);
            }

            // 2. Process [llama-server-long] table passthrough keys
            if let Some(long_obj) = assets
                .config
                .get("llama-server-long")
                .and_then(|v| v.as_object())
            {
                let mut sorted_long_keys: Vec<&String> = long_obj.keys().collect();
                sorted_long_keys.sort();
                for k in sorted_long_keys {
                    write_long_option(k, &long_obj[k], &mut current_preset);
                }
            }

            // 3. Process [llama-server-short] table keys
            if let Some(short_obj) = assets
                .config
                .get("llama-server-short")
                .and_then(|v| v.as_object())
            {
                let mut sorted_short_keys: Vec<&String> = short_obj.keys().collect();
                sorted_short_keys.sort();
                for k in sorted_short_keys {
                    if crate::config::is_restricted_short_key(k) {
                        continue;
                    }
                    let val = &short_obj[k];
                    if let Some(s) = val.as_str() {
                        current_preset.push(format!("{} = {}", k, s));
                    } else if let Some(b) = val.as_bool() {
                        current_preset.push(format!("{} = {}", k, b));
                    } else if let Some(n) = val.as_i64() {
                        current_preset.push(format!("{} = {}", k, n));
                    } else if let Some(f) = val.as_f64() {
                        current_preset.push(format!("{} = {}", k, f));
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

                let get_draft_lh = |key: &str| -> Option<&serde_json::Value> {
                    draft_config.get("llama-herd").and_then(|lh| lh.get(key))
                };
                let get_draft_long = |key: &str| -> Option<&serde_json::Value> {
                    draft_config
                        .get("llama-server-long")
                        .and_then(|l| l.get(key))
                        .or_else(|| draft_config.get(key))
                };

                let mut spec_type = get_draft_long("spec-type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("draft-mtp");
                if spec_type == "mtp" {
                    spec_type = "draft-mtp";
                }

                let spec_draft_n_max = get_draft_long("spec-draft-n-max")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4);
                let spec_draft_p_min = get_draft_long("spec-draft-p-min")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let d_ngl = get_draft_lh("total-layers")
                    .or_else(|| get_draft_long("total-layers"))
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

    std::fs::write(output_path, lines.join("\n"))?;
    Ok(output_path.to_path_buf())
}
