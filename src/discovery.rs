pub use crate::config::discover_assets;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static MODEL_ID_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"([b-zB-Z])(\d+)([a-zA-Z])").expect("Static regex is valid")
});
static MULTIPLE_DASHES_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"-+").expect("Static regex is valid"));
static VARIANT_SUFFIX_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"-([a-zA-Z0-9_]+)$").expect("Static regex is valid"));
static SIZE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b\d+(?:\.\d+)?(?:x\d+)?[bm]\b").expect("Static regex is valid")
});
static QUANT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b(?:q\d+(?:_?[k\d](?:_[sml])?)?|f16|fp16|bf16)\b")
        .expect("Static regex is valid")
});
static SPLIT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"[-._\s]+").expect("Static regex is valid"));

/// Normalizes a model path stem into a standard clean model ID string.
#[must_use]
pub fn clean_model_id(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let with_hyphens = stem.replace('.', "-");

    let formatted = MODEL_ID_RE.replace_all(&with_hyphens, "$1-$2$3");
    MULTIPLE_DASHES_RE.replace_all(&formatted, "-").into_owned()
}

/// Inserts a suffix (like 'vision' or 'draft') into a model ID before its final segment.
#[must_use]
pub fn insert_variant_suffix(name: &str, suffix: &str) -> String {
    let rep = format!("-{suffix}-${{1}}");
    VARIANT_SUFFIX_RE.replace(name, rep.as_str()).into_owned()
}

/// Finds the most compatible multimodal projector model (`mmproj`) for a given model.
#[must_use]
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
    None
}

/// Finds a compatible speculative draft model for a main model based on name token matching.
#[must_use]
pub fn find_matching_draft(model_path: &Path, draft_files: &[PathBuf]) -> Option<PathBuf> {
    let model_name_lower = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let clean_tokens = |name: &str| -> Vec<String> {
        let cleaned_size = SIZE_RE.replace_all(name, " ");
        let cleaned_quant = QUANT_RE.replace_all(&cleaned_size, " ");

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

        SPLIT_RE
            .split(&cleaned_quant)
            .filter(|&t| !t.is_empty() && !ignore_tokens.contains(&t))
            .map(str::to_owned)
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

/// Discovers active preset names and model file paths defined in a preset INI file.
#[must_use]
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
                if map.get("is-draft").map(String::as_str) == Some("true") {
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

/// Generates a `models-preset.ini` file dynamically by scanning the models directory.
///
/// # Errors
///
/// Returns an `std::io::Error` if the models directory cannot be read or if the output file cannot be written.
#[allow(clippy::too_many_lines)]
pub fn generate_presets_ini<S: std::hash::BuildHasher + Default>(
    models_dir: &Path,
    output_path: &Path,
    global_config: &HashMap<String, serde_json::Value, S>,
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
                    && (lh.get("is-draft").and_then(serde_json::Value::as_bool) == Some(true)
                        || lh.get("is-draft-only").and_then(serde_json::Value::as_bool)
                            == Some(true))
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
                    && lh.get("is-default").and_then(serde_json::Value::as_bool) == Some(true)
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
            .min_by_key(|m| std::fs::metadata(m).map_or(u64::MAX, |meta| meta.len()))
            .cloned()
    } else if !main_models.is_empty() {
        main_models
            .iter()
            .min_by_key(|m| std::fs::metadata(m).map_or(u64::MAX, |meta| meta.len()))
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
        v.as_str().map_or_else(
            || {
                #[allow(clippy::cast_possible_wrap)]
                v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
            },
            |s| s.parse::<i64>().ok(),
        )
    });

    let checkpoint_min_step = get_global_long("checkpoint-min-step").and_then(|v| {
        v.as_str().map_or_else(
            || {
                #[allow(clippy::cast_possible_wrap)]
                v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
            },
            |s| s.parse::<i64>().ok(),
        )
    });

    let no_mmap = get_global_long("no-mmap")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let mut lines = Vec::new();
    lines.push("version = 1".to_owned());
    lines.push("; Global settings shared across all presets".to_owned());
    lines.push("[*]".to_owned());
    lines.push("flash-attn = auto".to_owned());
    lines.push("jinja = true".to_owned());
    lines.push(format!("cache-type-k = {cache_type_k}"));
    lines.push(format!("cache-type-v = {cache_type_v}"));
    lines.push("kv-unified = true".to_owned());
    if let Some(checkpoints) = ctx_checkpoints {
        lines.push(format!("ctx-checkpoints = {checkpoints}"));
    }
    if let Some(step) = checkpoint_min_step {
        lines.push(format!("checkpoint-min-step = {step}"));
    }
    if no_mmap {
        lines.push("no-mmap = true".to_owned());
    }
    lines.push(String::new());

    let gpus = scan_gpu_topology();
    let auto_tensor_split = calculate_tensor_split(&gpus, 0.12);

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

        let default_ctx_val = serde_json::Value::String("131072".to_owned());
        let ctx_val = get_lh_val("ctx-size")
            .or_else(|| get_long_val("ctx-size"))
            .unwrap_or(&default_ctx_val)
            .clone();
        let ctx_size = crate::config::parse_ctx(&ctx_val).unwrap_or(131_072);

        let ngl_val = get_lh_val("ngl")
            .or_else(|| get_long_val("ngl"))
            .and_then(|v| {
                v.as_str()
                    .map_or_else(|| v.as_i64().map(|i| i.to_string()), |s| Some(s.to_owned()))
            })
            .unwrap_or_else(|| "auto".to_owned());
        let total_layers = get_lh_val("total-layers")
            .or_else(|| get_long_val("total-layers"))
            .and_then(|v| v.as_u64().and_then(|i| usize::try_from(i).ok()));
        let mut ngl = crate::config::calculate_ngl(&ngl_val, "auto", total_layers);
        if ngl == "auto"
            && let Some(total) = total_layers
        {
            ngl = total.to_string();
        }

        let temp = get_lh_val("temp")
            .or_else(|| get_long_val("temp"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.8);
        let top_p = get_lh_val("top-p")
            .or_else(|| get_long_val("top-p"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.95);
        let top_k = get_lh_val("top-k")
            .or_else(|| get_long_val("top-k"))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(40);
        let reasoning = get_lh_val("reasoning")
            .or_else(|| get_long_val("reasoning"))
            .and_then(|v| v.as_str())
            .unwrap_or("auto");

        let model_ctx_checkpoints = get_lh_val("ctx-checkpoints")
            .or_else(|| get_long_val("ctx-checkpoints"))
            .and_then(|v| {
                v.as_str().map_or_else(
                    || {
                        #[allow(clippy::cast_possible_wrap)]
                        v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                    },
                    |s| s.parse::<i64>().ok(),
                )
            });

        let model_checkpoint_min_step = get_lh_val("checkpoint-min-step")
            .or_else(|| get_long_val("checkpoint-min-step"))
            .and_then(|v| {
                v.as_str().map_or_else(
                    || {
                        #[allow(clippy::cast_possible_wrap)]
                        v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                    },
                    |s| s.parse::<i64>().ok(),
                )
            });

        let model_no_mmap = get_lh_val("no-mmap")
            .or_else(|| get_long_val("no-mmap"))
            .and_then(serde_json::Value::as_bool);

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
            let preset_name = preset_name.replace(['\n', '\r'], " ");
            let mut current_preset = Vec::new();
            current_preset.push(format!("; --- {preset_name} ---"));
            current_preset.push(format!("[{preset_name}]"));
            current_preset.push(format!(
                "model = {}",
                model_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .replace(['\n', '\r'], " ")
            ));

            if let Some(template) = &assets.jinja_template {
                current_preset.push(format!(
                    "chat-template-file = {}",
                    template
                        .to_string_lossy()
                        .replace('\\', "/")
                        .replace(['\n', '\r'], " ")
                ));
            }

            current_preset.push(format!("ctx-size = {ctx_size}"));
            current_preset.push(format!("n-gpu-layers = {ngl}"));
            current_preset.push(format!("temp = {temp}"));
            current_preset.push(format!("top-p = {top_p}"));
            current_preset.push(format!("top-k = {top_k}"));

            if let Some(checkpoints) = model_ctx_checkpoints {
                current_preset.push(format!("ctx-checkpoints = {checkpoints}"));
            }
            if let Some(step) = model_checkpoint_min_step {
                current_preset.push(format!("checkpoint-min-step = {step}"));
            }
            if let Some(mmap) = model_no_mmap {
                current_preset.push(format!("no-mmap = {mmap}"));
            }

            let config_ts = get_lh_val("tensor-split")
                .or_else(|| get_long_val("tensor-split"))
                .and_then(|v| v.as_str().map(ToOwned::to_owned));
            let tensor_split = config_ts.or_else(|| auto_tensor_split.clone());

            if let Some(ref ts) = tensor_split {
                if !ts.is_empty() && ts != "none" {
                    current_preset.push(format!("tensor-split = {ts}"));

                    let fit_val = get_lh_val("fit")
                        .or_else(|| get_long_val("fit"))
                        .and_then(|v| {
                            v.as_str().map_or_else(
                                || {
                                    v.as_bool()
                                        .map(|b| if b { "on".to_owned() } else { "off".to_owned() })
                                },
                                |s| Some(s.to_owned()),
                            )
                        })
                        .unwrap_or_else(|| "on".to_owned());

                    let fitt_target = get_lh_val("fitt")
                        .or_else(|| get_long_val("fitt"))
                        .and_then(|v| {
                            v.as_str().map_or_else(
                                || v.as_i64().map(|i| i.to_string()),
                                |s| Some(s.to_owned()),
                            )
                        })
                        .unwrap_or_else(|| "1024".to_owned());

                    current_preset.push(format!("fit = {fit_val}"));
                    current_preset.push(format!("fitt = {fitt_target}"));
                }
            }

            if reasoning != "auto" {
                current_preset.push(format!("reasoning = {reasoning}"));
                if reasoning == "on" {
                    current_preset.push("reasoning-format = deepseek".to_owned());
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
                        let sanitized = s.replace(['\n', '\r'], " ");
                        current_preset.push(format!("{ini_key} = {sanitized}"));
                    } else if let Some(b) = val.as_bool() {
                        current_preset.push(format!("{ini_key} = {b}"));
                    } else if let Some(n) = val.as_i64() {
                        current_preset.push(format!("{ini_key} = {n}"));
                    } else if let Some(f) = val.as_f64() {
                        current_preset.push(format!("{ini_key} = {f}"));
                    } else if let Some(arr) = val.as_array() {
                        let items: Vec<String> = arr
                            .iter()
                            .map(|v| v.as_str().map_or_else(|| v.to_string(), ToOwned::to_owned))
                            .collect();
                        current_preset.push(format!("{ini_key} = {}", items.join(",")));
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
                        let sanitized = s.replace(['\n', '\r'], " ");
                        current_preset.push(format!("{k} = {sanitized}"));
                    } else if let Some(b) = val.as_bool() {
                        current_preset.push(format!("{k} = {b}"));
                    } else if let Some(n) = val.as_i64() {
                        current_preset.push(format!("{k} = {n}"));
                    } else if let Some(f) = val.as_f64() {
                        current_preset.push(format!("{k} = {f}"));
                    }
                }
            }

            if use_vision && let Some(ref mm) = mmproj_file {
                current_preset.push(format!(
                    "mmproj = {}",
                    mm.to_string_lossy()
                        .replace('\\', "/")
                        .replace(['\n', '\r'], " ")
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
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(4);
                let spec_draft_p_min = get_draft_long("spec-draft-p-min")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                let get_draft_lh = |key: &str| -> Option<&serde_json::Value> {
                    draft_config.get("llama-herd").and_then(|lh| lh.get(key))
                };
                let get_draft_long_val = |key: &str| -> Option<&serde_json::Value> {
                    draft_config
                        .get("llama-server-long")
                        .and_then(|l| l.get(key))
                        .or_else(|| draft_config.get(key))
                };

                let draft_ngl_val = get_draft_lh("gpu-layers-draft")
                    .or_else(|| get_draft_long_val("gpu-layers-draft"))
                    .or_else(|| get_draft_lh("ngld"))
                    .or_else(|| get_draft_long_val("ngld"))
                    .or_else(|| get_draft_lh("n-gpu-layers"))
                    .or_else(|| get_draft_long_val("n-gpu-layers"))
                    .or_else(|| get_draft_lh("ngl"))
                    .or_else(|| get_draft_long_val("ngl"))
                    .and_then(|v| {
                        v.as_str()
                            .map(ToOwned::to_owned)
                            .or_else(|| v.as_i64().map(|i| i.to_string()))
                    });

                let draft_total_layers = get_draft_lh("total-layers")
                    .or_else(|| get_draft_long_val("total-layers"))
                    .and_then(|v| v.as_u64().and_then(|i| usize::try_from(i).ok()));

                let mut d_ngl = draft_ngl_val.as_ref().map_or_else(
                    || draft_total_layers.map_or_else(|| "auto".to_owned(), |t| t.to_string()),
                    |val_str| crate::config::calculate_ngl(val_str, "auto", draft_total_layers),
                );
                if d_ngl == "auto"
                    && let Some(total) = draft_total_layers
                {
                    d_ngl = total.to_string();
                }

                current_preset.push(format!(
                    "model-draft = {}",
                    df.to_string_lossy()
                        .replace('\\', "/")
                        .replace(['\n', '\r'], " ")
                ));
                current_preset.push(format!("spec-type = {spec_type}"));
                current_preset.push(format!("spec-draft-n-max = {spec_draft_n_max}"));
                current_preset.push(format!("spec-draft-p-min = {spec_draft_p_min}"));
                current_preset.push(format!("gpu-layers-draft = {d_ngl}"));
            }

            current_preset.push(String::new());

            if is_default && preset_name == clean_name {
                default_preset_lines = current_preset
                    .iter()
                    .map(|line| line.replace(&format!("[{clean_name}]"), "[default]"))
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

/// Represents the GPU driver/subsystem framework.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DriverType {
    /// NVIDIA CUDA driver interface.
    Cuda,
    /// AMD `ROCm` driver interface.
    Rocm,
    /// Windows Display Driver Model interface.
    Wddm,
    /// Unknown or unsupported driver.
    Unknown,
}

/// Represents details of a scanned GPU device in the system topology.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GpuDevice {
    /// Device index relative to driver/system enumeration.
    pub index: usize,
    /// Device product or model name.
    pub name: String,
    /// Total VRAM capacity in Megabytes (MiB).
    pub total_vram_mb: u64,
    /// Free/available VRAM capacity in Megabytes (MiB).
    pub free_vram_mb: u64,
    /// Detected driver framework type.
    pub driver_type: DriverType,
}

fn parse_vram_mb(s: &str) -> u64 {
    let s_clean = s.trim().to_lowercase();
    if s_clean.is_empty() {
        return 0;
    }
    let is_gb = s_clean.contains("gb") || s_clean.contains("gib");
    let num_str: String = s_clean
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    if let Ok(val) = num_str.parse::<f64>() {
        if is_gb {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            return (val * 1024.0) as u64;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let raw_val = val as u64;
        if raw_val > 100_000_000 {
            return raw_val / (1024 * 1024);
        }
        return raw_val;
    }
    0
}

/// Parses CSV output from `nvidia-smi`.
///
/// Expects lines formatted as: `index, name, memory.total, memory.free`
#[must_use]
pub fn parse_nvidia_smi_output(output: &str) -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() < 4 {
            continue;
        }
        let index_str = parts[0].trim();
        if index_str.to_lowercase().starts_with("index") {
            continue;
        }
        let Ok(index) = index_str.parse::<usize>() else {
            continue;
        };
        let name = parts[1].trim().to_owned();
        let total_vram_mb = parse_vram_mb(parts[2]);
        let free_vram_mb = parse_vram_mb(parts[3]);

        devices.push(GpuDevice {
            index,
            name,
            total_vram_mb,
            free_vram_mb,
            driver_type: DriverType::Cuda,
        });
    }
    devices
}

/// Parses CLI or CSV output from `rocm-smi`.
#[allow(
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::collapsible_if
)]
#[must_use]
pub fn parse_rocm_smi_output(output: &str) -> Vec<GpuDevice> {
    let mut csv_devices = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('=') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() >= 4 {
            let index_str = parts[0].trim();
            if index_str.to_lowercase().starts_with("gpu")
                || index_str.to_lowercase().starts_with("index")
            {
                if index_str.parse::<usize>().is_err() {
                    continue;
                }
            }
            if let Ok(index) = index_str.parse::<usize>() {
                let name = parts[1].trim().to_owned();
                let total_vram_mb = parse_vram_mb(parts[2]);
                let free_vram_mb = parse_vram_mb(parts[3]);
                csv_devices.push(GpuDevice {
                    index,
                    name,
                    total_vram_mb,
                    free_vram_mb,
                    driver_type: DriverType::Rocm,
                });
            }
        }
    }

    if !csv_devices.is_empty() {
        return csv_devices;
    }

    struct RocmEntry {
        name: Option<String>,
        total: Option<u64>,
        free: Option<u64>,
        used: Option<u64>,
    }

    let mut map: HashMap<usize, RocmEntry> = HashMap::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('=') {
            continue;
        }

        let key_val_split: Vec<&str> = trimmed.splitn(2, ':').collect();
        if key_val_split.len() < 2 {
            continue;
        }

        let prefix = key_val_split[0].trim();
        let val_str = key_val_split[1].trim();

        let mut gpu_idx = None;
        if let Some(start) = prefix.find("GPU[")
            && let Some(end) = prefix[start..].find(']')
        {
            if let Ok(idx) = prefix[start + 4..start + end].parse::<usize>() {
                gpu_idx = Some(idx);
            }
        } else if prefix.to_lowercase().starts_with("gpu") {
            let parts: Vec<&str> = prefix.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(idx) = parts[1]
                    .trim_matches(|c: char| !c.is_ascii_digit())
                    .parse::<usize>()
                {
                    gpu_idx = Some(idx);
                }
            }
        }

        let idx = match gpu_idx {
            Some(i) => i,
            None => continue,
        };

        let (sub_key, val) = if val_str.contains(':') {
            let parts: Vec<&str> = val_str.splitn(2, ':').collect();
            (parts[0].trim().to_lowercase(), parts[1].trim())
        } else {
            (prefix.to_lowercase(), val_str)
        };

        let entry = map.entry(idx).or_insert_with(|| RocmEntry {
            name: None,
            total: None,
            free: None,
            used: None,
        });

        if sub_key.contains("name") || sub_key.contains("series") {
            entry.name = Some(val.to_owned());
        } else if sub_key.contains("free") {
            entry.free = Some(parse_vram_mb(val));
        } else if sub_key.contains("used") {
            entry.used = Some(parse_vram_mb(val));
        } else if sub_key.contains("total") {
            entry.total = Some(parse_vram_mb(val));
        }
    }

    let mut indices: Vec<usize> = map.keys().copied().collect();
    indices.sort_unstable();

    let mut devices = Vec::new();
    for idx in indices {
        if let Some(entry) = map.remove(&idx) {
            let total = entry.total.unwrap_or(0);
            let free = entry
                .free
                .unwrap_or_else(|| total.saturating_sub(entry.used.unwrap_or(0)));
            devices.push(GpuDevice {
                index: idx,
                name: entry.name.unwrap_or_else(|| format!("ROCm GPU {idx}")),
                total_vram_mb: total,
                free_vram_mb: free,
                driver_type: DriverType::Rocm,
            });
        }
    }

    devices
}

/// Parses Windows WDDM video controller output (e.g. from PowerShell or WMIC).
#[allow(clippy::too_many_lines, clippy::manual_let_else)]
#[must_use]
pub fn parse_wddm_output(output: &str) -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let mut auto_idx = 0;

    for line in output.lines() {
        let trimmed = line.trim().trim_matches('"');
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let lower = trimmed.to_lowercase();
        if lower.starts_with("name") && lower.contains("adapterram") {
            continue;
        }

        let parts: Vec<&str> = trimmed
            .split(',')
            .map(|p| p.trim().trim_matches('"'))
            .collect();

        if parts.len() >= 4 {
            if let Ok(idx) = parts[0].parse::<usize>() {
                let name = parts[1].to_owned();
                let total = parse_vram_mb(parts[2]);
                let free = parse_vram_mb(parts[3]);
                devices.push(GpuDevice {
                    index: idx,
                    name,
                    total_vram_mb: total,
                    free_vram_mb: free,
                    driver_type: DriverType::Wddm,
                });
            }
        } else if parts.len() == 3 {
            if let Ok(idx) = parts[0].parse::<usize>() {
                let name = parts[1].to_owned();
                let total = parse_vram_mb(parts[2]);
                devices.push(GpuDevice {
                    index: idx,
                    name,
                    total_vram_mb: total,
                    free_vram_mb: total,
                    driver_type: DriverType::Wddm,
                });
            } else {
                let name = parts[0].to_owned();
                let total = parse_vram_mb(parts[1]);
                let free = parse_vram_mb(parts[2]);
                devices.push(GpuDevice {
                    index: auto_idx,
                    name,
                    total_vram_mb: total,
                    free_vram_mb: free,
                    driver_type: DriverType::Wddm,
                });
                auto_idx += 1;
            }
        } else if parts.len() == 2 {
            let name = parts[0].to_owned();
            let total = parse_vram_mb(parts[1]);
            if !name.is_empty() && (total > 0 || !name.to_lowercase().contains("name")) {
                devices.push(GpuDevice {
                    index: auto_idx,
                    name,
                    total_vram_mb: total,
                    free_vram_mb: total,
                    driver_type: DriverType::Wddm,
                });
                auto_idx += 1;
            }
        }
    }

    if !devices.is_empty() {
        return devices;
    }

    let mut current_name = None;
    let mut current_ram = None;
    let mut kv_idx = 0;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let Some(name) = current_name.take() {
                let total = current_ram.take().unwrap_or(0);
                devices.push(GpuDevice {
                    index: kv_idx,
                    name,
                    total_vram_mb: total,
                    free_vram_mb: total,
                    driver_type: DriverType::Wddm,
                });
                kv_idx += 1;
            }
            continue;
        }

        if let Some((key, val)) = trimmed.split_once(':') {
            let k = key.trim().to_lowercase();
            let v = val.trim();
            if k == "name" || k == "caption" {
                if let Some(name) = current_name.take() {
                    let total = current_ram.take().unwrap_or(0);
                    devices.push(GpuDevice {
                        index: kv_idx,
                        name,
                        total_vram_mb: total,
                        free_vram_mb: total,
                        driver_type: DriverType::Wddm,
                    });
                    kv_idx += 1;
                }
                current_name = Some(v.to_owned());
            } else if k == "adapterram" || k == "dedicatedvideomemory" {
                current_ram = Some(parse_vram_mb(v));
            }
        }
    }

    if let Some(name) = current_name {
        let total = current_ram.unwrap_or(0);
        devices.push(GpuDevice {
            index: kv_idx,
            name,
            total_vram_mb: total,
            free_vram_mb: total,
            driver_type: DriverType::Wddm,
        });
    }

    devices
}

/// Scans the system GPU topology by querying available hardware drivers (`CUDA`, `ROCm`, `WDDM`).
///
/// Order of attempts:
/// 1. `nvidia-smi`
/// 2. `rocm-smi`
/// 3. Windows WDDM (`Get-CimInstance Win32_VideoController`)
/// 4. Fallback (empty `Vec`)
#[must_use]
pub fn scan_gpu_topology() -> Vec<GpuDevice> {
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let devices = parse_nvidia_smi_output(&stdout);
            if !devices.is_empty() {
                return devices;
            }
        }
    }

    if let Ok(output) = std::process::Command::new("rocm-smi")
        .args(["--showid", "--showmeminfo"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let devices = parse_rocm_smi_output(&stdout);
            if !devices.is_empty() {
                return devices;
            }
        }
    }

    if cfg!(target_os = "windows") {
        if let Ok(output) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM | ConvertTo-Csv -NoTypeInformation",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let devices = parse_wddm_output(&stdout);
                if !devices.is_empty() {
                    return devices;
                }
            }
        }
    }

    Vec::new()
}

/// Calculates the multi-GPU tensor split ratio string based on effective VRAM and headroom percentage.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn calculate_tensor_split(gpus: &[GpuDevice], headroom_pct: f64) -> Option<String> {
    if gpus.len() < 2 {
        return None;
    }

    let headroom = if headroom_pct <= 0.0 || headroom_pct.is_nan() {
        0.12
    } else {
        headroom_pct.clamp(0.05, 0.25)
    };

    let effective_vrams: Vec<f64> = gpus
        .iter()
        .map(|g| (g.total_vram_mb as f64) * (1.0 - headroom))
        .collect();

    let min_vram = effective_vrams
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    if min_vram <= 0.0 || min_vram.is_infinite() {
        return None;
    }

    let weights: Vec<String> = effective_vrams
        .iter()
        .map(|&vram| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let w = (vram / min_vram).round() as u64;
            w.max(1).to_string()
        })
        .collect();

    Some(weights.join(","))
}
