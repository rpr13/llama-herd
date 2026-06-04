use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

// --- CONFIGURATION STRUCTURES ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAssets {
    pub config: HashMap<String, serde_json::Value>,
    pub jinja_template: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct UserSettings {
    pub ctx: usize,
    pub ngl: String,
    pub mmproj: Option<PathBuf>,
    pub draft_model: Option<PathBuf>,
    pub draft_ngl: String,
}

// --- PURE PARSERS & HELPERS ---

pub fn parse_ctx(value: &serde_json::Value) -> Result<usize, String> {
    match value {
        serde_json::Value::Number(num) => {
            if let Some(i) = num.as_u64() {
                Ok(i as usize)
            } else if let Some(f) = num.as_f64() {
                if f < 0.0 {
                    Err(format!("Invalid negative context size: {}", f))
                } else {
                    Ok(f as usize)
                }
            } else {
                Err(format!("Invalid number format for context size: {}", num))
            }
        }
        serde_json::Value::String(s) => parse_ctx_str(s),
        _ => Err(format!("Invalid type for context size: {:?}", value)),
    }
}

pub fn parse_ctx_str(s: &str) -> Result<usize, String> {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return Err("Context size cannot be empty".to_string());
    }
    let s_lower = s_trimmed.to_lowercase();

    if s_lower.ends_with('k') {
        let val = s_lower[..s_lower.len() - 1]
            .trim()
            .parse::<usize>()
            .map_err(|e| format!("Failed to parse context size digits: {}", e))?;
        return Ok(val * 1024);
    }

    let val = s_trimmed
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse context size: {}", e))?;
    Ok(val)
}

pub fn is_safe_value(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.starts_with("--") {
                return false;
            }
            if let Some(after_dash) = trimmed.strip_prefix('-')
                && (after_dash.is_empty()
                    || !after_dash.chars().all(|c| c.is_ascii_digit() || c == '.'))
            {
                return false;
            }
            true
        }
        serde_json::Value::Array(arr) => arr.iter().all(is_safe_value),
        serde_json::Value::Object(_) => false,
        _ => true,
    }
}

pub fn get_optimal_threads() -> String {
    match std::thread::available_parallelism() {
        Ok(cores) => {
            let logical = cores.get();
            let physical = std::cmp::max(1, logical / 2);
            physical.to_string()
        }
        Err(_) => "4".to_string(),
    }
}

pub fn calculate_ngl(input_str: &str, default_val: &str, total_layers: Option<usize>) -> String {
    if let Some(stripped) = input_str.strip_prefix("--")
        && let Some(layers) = total_layers
        && let Ok(delta) = stripped.parse::<usize>()
    {
        return layers.saturating_sub(delta).to_string();
    }
    if input_str.is_empty() {
        default_val.to_string()
    } else {
        input_str.to_string()
    }
}

// --- TOML LOADERS ---

const RESTRICTED_LONG: &[&str] = &[
    "ctx-size",
    "total-layers",
    "n-gpu-layers",
    "kv-quant",
    "kv-unified",
    "cache-type-k",
    "cache-type-v",
    "ngl",
    "threads",
    "ngld",
    "gpu-layers-draft",
    "spec-draft-ngl",
    "model-draft",
    "spec-draft-model",
    "is-draft",
    "is-default",
    "is-draft-only",
    "ui",
    "webui",
    "model",
    "chat-template-file",
    "mmproj",
    "jinja",
    "flash-attn",
    "version",
    "tools",
    "batch-size",
    "ubatch-size",
    "log-colors",
    "host",
    "port",
    "np",
    "parallel",
    "models-preset",
    "models-max",
    "models-autoload",
    "props",
    "temp",
    "top-p",
    "top-k",
    "reasoning",
    "reasoning-format",
    "ctx-checkpoints",
    "checkpoint-min-step",
    "no-mmap",
];

const RESTRICTED_SHORT: &[&str] = &[
    "c", "ngl", "ngld", "t", "md", "m", "mm", "np", "b", "ub", "fa", "kvu", "h",
];

fn is_invalid_key(k: &str) -> Option<&'static str> {
    if k.starts_with('-') {
        Some("starts with a dash '-'")
    } else if k.contains('_') {
        Some("contains an underscore '_'")
    } else {
        None
    }
}

pub fn is_restricted_key(key: &str) -> bool {
    RESTRICTED_LONG.contains(&key)
}

pub fn is_restricted_short_key(key: &str) -> bool {
    RESTRICTED_SHORT.contains(&key)
}

pub fn format_arg_name(key: &str) -> Option<String> {
    Some(format!("--{}", key))
}

pub fn format_ini_key(key: &str) -> Option<String> {
    Some(key.to_string())
}

pub fn load_toml_silent(path: &Path) -> HashMap<String, serde_json::Value> {
    if let Ok(mut file) = File::open(path) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok()
            && let Ok(value) = toml::from_str::<toml::Value>(&contents)
        {
            return toml_to_json(value, false);
        }
    }
    HashMap::new()
}

pub fn load_toml_safe(path: &Path) -> Result<HashMap<String, serde_json::Value>, std::io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let value = toml::from_str::<toml::Value>(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    println!(
        "[*] Loaded parameters from: {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );

    Ok(toml_to_json(value, true))
}

fn toml_to_json(toml: toml::Value, warn: bool) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    if let toml::Value::Table(table) = toml {
        for (k, v) in table {
            if let Some(reason) = is_invalid_key(&k) {
                if warn {
                    println!(
                        "[!] Warning: Key '{}' is invalid ({}) and was skipped.",
                        k, reason
                    );
                }
                continue;
            }
            map.insert(k, convert_toml_val(v, warn));
        }
    }
    map
}

fn convert_toml_val(v: toml::Value, warn: bool) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::Number(i.into()),
        toml::Value::Float(f) => {
            serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap())
        }
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Array(arr) => serde_json::Value::Array(
            arr.into_iter()
                .map(|item| convert_toml_val(item, warn))
                .collect(),
        ),
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                if let Some(reason) = is_invalid_key(&k) {
                    if warn {
                        println!(
                            "[!] Warning: Key '{}' is invalid ({}) and was skipped.",
                            k, reason
                        );
                    }
                    continue;
                }
                map.insert(k, convert_toml_val(v, warn));
            }
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(_) => serde_json::Value::Null,
    }
}

// --- INI PARSING & MERGING ---

pub fn parse_settings_ini(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections = HashMap::new();
    let mut current_section_name: Option<String> = None;
    let mut current_section_map = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(sec) = current_section_name.take() {
                sections.insert(sec, current_section_map);
                current_section_map = HashMap::new();
            }
            let sec_name = line[1..line.len() - 1].trim().to_string();
            current_section_name = Some(sec_name);
        } else if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            current_section_map.insert(key, value);
        }
    }

    if let Some(sec) = current_section_name {
        sections.insert(sec, current_section_map);
    }

    sections
}

pub fn load_settings_from_ini(
    preset_name: &str,
    preset_path: &Path,
) -> Option<HashMap<String, String>> {
    if !preset_path.exists() {
        return None;
    }
    if let Ok(content) = std::fs::read_to_string(preset_path) {
        let sections = parse_settings_ini(&content);

        let mut merged = HashMap::new();
        if let Some(global) = sections.get("*") {
            merged.extend(global.clone());
        }
        if let Some(preset) = sections.get(preset_name) {
            merged.extend(preset.clone());
            return Some(merged);
        }
    }
    None
}

pub fn discover_assets(selected_model: &Path, models_dir: &Path) -> ModelAssets {
    let stem = selected_model
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut toml_files = Vec::new();
    let mut jinja_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "toml" {
                    toml_files.push(path);
                } else if ext_lower == "jinja" {
                    jinja_files.push(path);
                }
            }
        }
    }

    // Sort descending by length of file name
    toml_files.sort_by_key(|p| std::cmp::Reverse(p.file_name().unwrap_or_default().len()));
    jinja_files.sort_by_key(|p| std::cmp::Reverse(p.file_name().unwrap_or_default().len()));

    let mut config_data = HashMap::new();
    for f in toml_files {
        let f_stem = f
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if stem.starts_with(&f_stem) {
            config_data = load_toml_silent(&f);
            break;
        }
    }

    let mut jinja_template = None;
    for f in jinja_files {
        let f_stem = f
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if stem.starts_with(&f_stem) {
            jinja_template = Some(f);
            break;
        }
    }

    ModelAssets {
        config: config_data,
        jinja_template,
    }
}

pub fn resolve_toml_path(selected_model: &Path, models_dir: &Path) -> PathBuf {
    let stem = selected_model
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut toml_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str())
                && ext.to_lowercase() == "toml"
            {
                toml_files.push(path);
            }
        }
    }

    // Sort descending by length of file name
    toml_files.sort_by_key(|p| std::cmp::Reverse(p.file_name().unwrap_or_default().len()));

    for f in toml_files {
        let f_stem = f
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if stem.starts_with(&f_stem) {
            return f;
        }
    }

    // Default fallback: exact model name with .toml extension
    let file_name = selected_model
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.strip_suffix(".gguf").unwrap_or(s))
        .unwrap_or("model");
    models_dir.join(format!("{}.toml", file_name))
}

pub fn get_home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .ok()
}

pub fn get_llama_herd_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("llama-herd");
        }
    } else if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("llama-herd");
    }
    PathBuf::from(".")
}

pub fn save_config(
    path: &Path,
    config: &HashMap<String, serde_json::Value>,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let toml_string = toml::to_string(config)
        .map_err(|e| std::io::Error::other(format!("TOML serialization error: {}", e)))?;

    std::fs::write(path, toml_string)
}

pub fn resolve_server_executable(
    global_config: &HashMap<String, serde_json::Value>,
) -> Option<PathBuf> {
    // 1. Check config
    if let Some(s) = global_config
        .get("llama-herd")
        .and_then(|lh| lh.get("llama-server").or_else(|| lh.get("server-path")))
        .or_else(|| global_config.get("llama-server"))
        .or_else(|| global_config.get("server-path"))
        .and_then(|v| v.as_str())
    {
        let p = PathBuf::from(s);
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. Search PATH
    let bin_name = if cfg!(target_os = "windows") {
        "llama-server.exe"
    } else {
        "llama-server"
    };

    if let Ok(paths) = std::env::var("PATH") {
        for path in std::env::split_paths(&paths) {
            let full_path = path.join(bin_name);
            if full_path.is_file() {
                return Some(full_path);
            }
        }
    }

    None
}

pub fn resolve_models_dir(global_config: &HashMap<String, serde_json::Value>) -> Option<PathBuf> {
    // 1. Check config
    if let Some(s) = global_config
        .get("llama-herd")
        .and_then(|lh| lh.get("models-dir").or_else(|| lh.get("models-path")))
        .or_else(|| global_config.get("models-dir"))
        .or_else(|| global_config.get("models-path"))
        .and_then(|v| v.as_str())
    {
        let p = PathBuf::from(s);
        if p.is_dir() {
            return Some(p);
        }
    }

    // 2. Check current dir
    let local_models = PathBuf::from("models");
    if local_models.is_dir() {
        return Some(local_models);
    }

    None
}

pub fn parse_args(args: &[String]) -> (bool, bool) {
    let mut show_help = false;
    let mut generate_ini = false;

    for arg in args.iter().skip(1) {
        if arg == "-h" || arg == "--help" {
            show_help = true;
        }
        if arg == "--ini" {
            generate_ini = true;
        }
    }

    (show_help, generate_ini)
}

pub fn get_global_config_string(
    global_config: &HashMap<String, serde_json::Value>,
    key: &str,
    default_val: &str,
) -> String {
    let val = global_config
        .get("llama-server-long")
        .and_then(|obj| obj.get(key))
        .or_else(|| global_config.get("llama-herd").and_then(|obj| obj.get(key)))
        .or_else(|| global_config.get(key));

    match val {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => default_val.to_string(),
    }
}

pub fn update_global_config_value(
    global_config: &mut HashMap<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    if let Some(serde_json::Value::Object(long_obj)) = global_config.get_mut("llama-server-long")
        && long_obj.contains_key(key)
    {
        long_obj.insert(key.to_string(), value);
        return;
    }

    if let Some(serde_json::Value::Object(lh_obj)) = global_config.get_mut("llama-herd")
        && lh_obj.contains_key(key)
    {
        lh_obj.insert(key.to_string(), value);
        return;
    }

    global_config.insert(key.to_string(), value);
}

pub fn remove_global_config_value(
    global_config: &mut HashMap<String, serde_json::Value>,
    key: &str,
) {
    let mut remove_long = false;
    if let Some(serde_json::Value::Object(long_obj)) = global_config.get_mut("llama-server-long") {
        long_obj.remove(key);
        if long_obj.is_empty() {
            remove_long = true;
        }
    }
    if remove_long {
        global_config.remove("llama-server-long");
    }

    let mut remove_lh = false;
    if let Some(serde_json::Value::Object(lh_obj)) = global_config.get_mut("llama-herd") {
        lh_obj.remove(key);
        if lh_obj.is_empty() {
            remove_lh = true;
        }
    }
    if remove_lh {
        global_config.remove("llama-herd");
    }

    global_config.remove(key);
}

pub fn find_similar_config_files(model_path: &Path, models_dir: &Path) -> Vec<String> {
    let mut results = Vec::new();
    let stem = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Determine prefix by taking first hyphenated/underscored parts
    let parts: Vec<&str> = stem.split(['-', '_']).collect();
    let prefix = if parts.len() >= 2 {
        format!("{}-{}", parts[0], parts[1])
    } else if !parts.is_empty() {
        parts[0].to_string()
    } else {
        String::new()
    };

    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml")
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
            {
                let name_lower = name.to_lowercase();
                let toml_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if (!prefix.is_empty()
                    && (name_lower.starts_with(&prefix) || name_lower.contains(&prefix)))
                    || stem.starts_with(&toml_stem)
                {
                    results.push(name.to_string());
                }
            }
        }
    }
    results.sort();
    results
}
