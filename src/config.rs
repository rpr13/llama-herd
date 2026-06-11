//! Configuration structures and helpers for parsing GGUF/TOML config files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// --- CONFIGURATION STRUCTURES ---

/// Represents assets associated with a specific model, including its configuration TOML and optional template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAssets {
    /// Parsed model configuration.
    pub config: HashMap<String, serde_json::Value>,
    /// Optional path to a Jinja template.
    pub jinja_template: Option<PathBuf>,
}

/// Dynamic settings overridden by the user on the dashboard.
#[derive(Debug, Clone)]
pub struct UserSettings {
    /// Context size in tokens.
    pub ctx: usize,
    /// GPU offload layer specification.
    pub ngl: String,
    /// Optional path to the vision projector model.
    pub mmproj: Option<PathBuf>,
    /// Optional path to the speculative draft model.
    pub draft_model: Option<PathBuf>,
    /// GPU offload layer specification for the draft model.
    pub draft_ngl: String,
}

// --- PURE PARSERS & HELPERS ---

/// Parses context size from a JSON value, supporting strings with 'k'/'K' suffixes and numbers.
///
/// # Errors
///
/// Returns an error if the value is not a string or number, or if the number is negative or unparseable.
pub fn parse_ctx(value: &serde_json::Value) -> Result<usize, String> {
    match value {
        serde_json::Value::Number(num) => num.as_u64().map_or_else(
            || {
                num.as_f64().map_or_else(
                    || Err(format!("Invalid number format for context size: {num}")),
                    |f| {
                        if f < 0.0 {
                            Err(format!("Invalid negative context size: {f}"))
                        } else {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            Ok(f as usize)
                        }
                    },
                )
            },
            |i| usize::try_from(i).map_err(|e| format!("Context size overflow: {e}")),
        ),
        serde_json::Value::String(s) => parse_ctx_str(s),
        _ => Err(format!("Invalid type for context size: {value:?}")),
    }
}

/// Parses context size from a string, allowing optional 'k' or 'K' suffix for multipliers.
///
/// # Errors
///
/// Returns an error if the string is empty, contains invalid digits, or if the resulting context size overflows.
pub fn parse_ctx_str(s: &str) -> Result<usize, String> {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return Err("Context size cannot be empty".to_owned());
    }
    let s_lower = s_trimmed.to_lowercase();

    if s_lower.ends_with('k') {
        let val = s_lower[..s_lower.len() - 1]
            .trim()
            .parse::<usize>()
            .map_err(|e| format!("Failed to parse context size digits: {e}"))?;
        return val
            .checked_mul(1024)
            .ok_or_else(|| "Context size overflowed".to_owned());
    }

    let val = s_trimmed
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse context size: {e}"))?;
    Ok(val)
}

/// Validates that a setting value does not contain injection attempts (e.g., options starting with '--').
#[must_use]
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

/// Returns the optimal CPU thread count based on physical cores.
#[must_use]
pub fn get_optimal_threads() -> String {
    std::thread::available_parallelism().map_or_else(
        |_| "4".to_owned(),
        |cores| {
            let logical = cores.get();
            let physical = std::cmp::max(1, logical / 2);
            physical.to_string()
        },
    )
}

/// Computes the GPU offload layers if delta syntax (e.g., `--4`) is provided.
#[must_use]
pub fn calculate_ngl(input_str: &str, default_val: &str, total_layers: Option<usize>) -> String {
    if let Some(stripped) = input_str.strip_prefix("--")
        && let Some(layers) = total_layers
        && let Ok(delta) = stripped.parse::<usize>()
    {
        return layers.saturating_sub(delta).to_string();
    }
    if input_str.is_empty() {
        default_val.to_owned()
    } else {
        input_str.to_owned()
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
    "log-verbosity",
    "verbosity",
    "lv",
    "min-p",
    "repeat-penalty",
    "repeat-last-n",
    "reasoning-budget",
    "cache-prompt",
    "no-cache-prompt",
    "context-shift",
    "no-context-shift",
    "mlock",
    "numa",
    "split-mode",
    "device",
    "api-key-file",
    "ssl-key-file",
    "ssl-cert-file",
];

const RESTRICTED_SHORT: &[&str] = &[
    "c", "ngl", "ngld", "t", "md", "m", "mm", "np", "b", "ub", "fa", "kvu", "h", "lv",
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

/// Checks if a long option key is restricted.
#[must_use]
pub fn is_restricted_key(key: &str) -> bool {
    RESTRICTED_LONG.contains(&key)
}

/// Checks if a short option key is restricted.
#[must_use]
pub fn is_restricted_short_key(key: &str) -> bool {
    RESTRICTED_SHORT.contains(&key)
}

/// Formats a key name to double-dash CLI option style.
#[must_use]
pub fn format_arg_name(key: &str) -> Option<String> {
    Some(format!("--{key}"))
}

/// Formats a key name to INI format.
#[must_use]
pub fn format_ini_key(key: &str) -> Option<String> {
    Some(key.to_owned())
}

/// Loads a TOML configuration silently, returning an empty `HashMap` on failure.
#[must_use]
pub fn load_toml_silent(path: &Path) -> HashMap<String, serde_json::Value> {
    if let Ok(contents) = std::fs::read_to_string(path)
        && let Ok(value) = toml::from_str::<toml::Value>(&contents)
    {
        return toml_to_json(value, false);
    }
    HashMap::new()
}

/// Custom error enum representing orchestration and config errors in llama-herd.
#[derive(thiserror::Error, Debug)]
pub enum HerdError {
    /// An input/output operation failed.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// The file path where the error occurred.
        path: PathBuf,
        /// The underlying I/O error source.
        source: std::io::Error,
    },
    /// A configuration file parsing error.
    #[error("TOML parsing error in {path}: {source}")]
    TomlParse {
        /// The file path where the error occurred.
        path: PathBuf,
        /// The underlying TOML deserialization error.
        source: toml::de::Error,
    },
}

/// Safely loads a TOML configuration from the filesystem, validating keys
/// and converting TOML values to generic JSON values.
///
/// # Errors
///
/// Returns a `HerdError` if the file cannot be opened, read, or if the TOML content is invalid.
pub fn load_toml_safe(path: &Path) -> Result<HashMap<String, serde_json::Value>, HerdError> {
    let contents = std::fs::read_to_string(path).map_err(|e| HerdError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let value = toml::from_str::<toml::Value>(&contents).map_err(|e| HerdError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })?;

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
                    println!("[!] Warning: Key '{k}' is invalid ({reason}) and was skipped.");
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
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
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
                        println!("[!] Warning: Key '{k}' is invalid ({reason}) and was skipped.");
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

/// Parses a raw INI file string into nested section maps.
#[must_use]
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
            let sec_name = line[1..line.len() - 1].trim().to_owned();
            current_section_name = Some(sec_name);
        } else if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_owned();
            let value = line[pos + 1..].trim().to_owned();
            current_section_map.insert(key, value);
        }
    }

    if let Some(sec) = current_section_name {
        sections.insert(sec, current_section_map);
    }

    sections
}

/// Loads and merges the preset-specific and global configurations from a preset INI file.
#[must_use]
pub fn load_settings_from_ini(
    preset_name: &str,
    preset_path: &Path,
) -> Option<HashMap<String, String>> {
    if !preset_path.exists() {
        return None;
    }
    if let Ok(content) = std::fs::read_to_string(preset_path) {
        let mut sections = parse_settings_ini(&content);

        let mut merged = HashMap::new();
        if let Some(global) = sections.remove("*") {
            merged.extend(global);
        }
        if let Some(preset) = sections.remove(preset_name) {
            merged.extend(preset);
            return Some(merged);
        }
    }
    None
}

/// Scans the models directory for configuration TOML files matching the chosen GGUF model name.
#[must_use]
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

/// Resolves the file path of the TOML configuration file corresponding to a selected model.
#[must_use]
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
        .map_or("model", |s| s.strip_suffix(".gguf").unwrap_or(s));
    models_dir.join(format!("{file_name}.toml"))
}

use std::cell::RefCell;

thread_local! {
    static LLAMA_HERD_DIR_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static HOME_DIR_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Sets a thread-local override path for the llama-herd application directory.
/// Useful for isolating filesystem path resolution during integration testing.
pub fn set_llama_herd_dir_override(path: Option<PathBuf>) {
    LLAMA_HERD_DIR_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = path;
    });
}

/// Sets a thread-local override path for the user's home directory.
/// Useful for isolating user home path checks during integration testing.
pub fn set_home_dir_override(path: Option<PathBuf>) {
    HOME_DIR_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = path;
    });
}

/// Returns the path to the user's home directory.
#[must_use]
pub fn get_home_dir() -> Option<PathBuf> {
    if let Some(path) = HOME_DIR_OVERRIDE.with(|cell| cell.borrow().clone()) {
        return Some(path);
    }

    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

/// Returns the configuration directory path for llama-herd.
#[must_use]
pub fn get_llama_herd_dir() -> PathBuf {
    if let Some(path) = LLAMA_HERD_DIR_OVERRIDE.with(|cell| cell.borrow().clone()) {
        return path;
    }
    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("llama-herd");
        }
    } else if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("llama-herd");
    }
    PathBuf::from(".")
}

/// Saves the global configuration back to its TOML file.
///
/// # Errors
///
/// Returns an `std::io::Error` if the directory cannot be created or if the file cannot be written.
pub fn save_config<S: std::hash::BuildHasher>(
    path: &Path,
    config: &HashMap<String, serde_json::Value, S>,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let toml_string = toml::to_string(config)
        .map_err(|e| std::io::Error::other(format!("TOML serialization error: {e}")))?;

    std::fs::write(path, toml_string)
}

/// Locates the llama-server executable path, checking both config and search paths.
#[must_use]
pub fn resolve_server_executable<S: std::hash::BuildHasher>(
    global_config: &HashMap<String, serde_json::Value, S>,
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

/// Resolves the models directory path.
#[must_use]
pub fn resolve_models_dir<S: std::hash::BuildHasher>(
    global_config: &HashMap<String, serde_json::Value, S>,
) -> Option<PathBuf> {
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

/// Parses command-line arguments to check for help flag or ini-generation requests.
#[must_use]
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

/// Helper function to retrieve a configuration value as a String from config hierarchy.
#[must_use]
pub fn get_global_config_string<S: std::hash::BuildHasher>(
    global_config: &HashMap<String, serde_json::Value, S>,
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
                "true".to_owned()
            } else {
                "false".to_owned()
            }
        }
        _ => default_val.to_owned(),
    }
}

/// Updates a configuration value in the global config nested maps.
pub fn update_global_config_value<S: std::hash::BuildHasher>(
    global_config: &mut HashMap<String, serde_json::Value, S>,
    key: &str,
    value: serde_json::Value,
) {
    if let Some(serde_json::Value::Object(long_obj)) = global_config.get_mut("llama-server-long")
        && long_obj.contains_key(key)
    {
        long_obj.insert(key.to_owned(), value);
        return;
    }

    if let Some(serde_json::Value::Object(lh_obj)) = global_config.get_mut("llama-herd")
        && lh_obj.contains_key(key)
    {
        lh_obj.insert(key.to_owned(), value);
        return;
    }

    global_config.insert(key.to_owned(), value);
}

/// Removes a configuration value from the global configuration nested maps.
pub fn remove_global_config_value<S: std::hash::BuildHasher>(
    global_config: &mut HashMap<String, serde_json::Value, S>,
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

/// Finds files that are named similarly to the specified model inside the models directory.
#[must_use]
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
        parts[0].to_owned()
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
                    results.push(name.to_owned());
                }
            }
        }
    }
    results.sort();
    results
}
