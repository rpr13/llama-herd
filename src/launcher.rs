use crate::config::{ModelAssets, UserSettings};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub fn kill_existing_servers() {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "llama-server.exe", "/T"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("pkill").args(["-9", "llama-server"]).output();
    }
}

pub fn build_launch_parameters(
    executable_path: &Path,
    model_path: &Path,
    assets: &ModelAssets,
    settings: &UserSettings,
    global_config: &HashMap<String, serde_json::Value>,
) -> Vec<String> {
    let mut params = Vec::new();
    params.push(executable_path.to_string_lossy().into_owned());

    params.push("-m".to_string());
    params.push(model_path.to_string_lossy().into_owned());

    let host = global_config
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0.0");
    params.push("--host".to_string());
    params.push(host.to_string());

    let port = global_config
        .get("port")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "8080".to_string());
    params.push("--port".to_string());
    params.push(port);

    params.push("--log-colors".to_string());
    params.push("on".to_string());

    params.push("-ngl".to_string());
    params.push(settings.ngl.clone());

    params.push("--ctx-size".to_string());
    params.push(settings.ctx.to_string());

    let flash_attn = global_config
        .get("flash-attn")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    params.push("--flash-attn".to_string());
    params.push(flash_attn.to_string());

    let cache_ram = global_config
        .get("cache-ram")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_string());
    params.push("--cache-ram".to_string());
    params.push(cache_ram);

    let np = global_config
        .get("np")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "1".to_string());
    params.push("-np".to_string());
    params.push(np);

    let threads = global_config
        .get("threads")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(crate::config::get_optimal_threads);
    params.push("-t".to_string());
    params.push(threads);

    params.push("--kv-unified".to_string());

    if let Some(bs) = global_config.get("batch-size").and_then(|v| v.as_i64()) {
        params.push("-b".to_string());
        params.push(bs.to_string());
    }
    if let Some(ubs) = global_config.get("ubatch-size").and_then(|v| v.as_i64()) {
        params.push("-ub".to_string());
        params.push(ubs.to_string());
    }
    if let Some(tools) = global_config.get("tools").and_then(|v| v.as_str()) {
        params.push("--tools".to_string());
        params.push(tools.to_string());
    }

    let kv_quant = global_config
        .get("lh-kv-quant")
        .or_else(|| global_config.get("kv-quant"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            assets
                .config
                .get("lh-kv-quant")
                .or_else(|| assets.config.get("kv-quant"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("q8_0");
    params.push("-ctk".to_string());
    params.push(kv_quant.to_string());
    params.push("-ctv".to_string());
    params.push(kv_quant.to_string());

    if !settings.ui {
        params.push("--no-ui".to_string());
    }

    let mut processed_config = assets.config.clone();
    if let Some(val) = processed_config.remove("lh-spec-type") {
        let spec_val = if val.as_str() == Some("mtp") {
            serde_json::Value::String("draft-mtp".to_string())
        } else {
            val
        };
        processed_config.insert("spec-type".to_string(), spec_val);
    }

    if let Some(spec_type) = processed_config.get_mut("spec-type")
        && spec_type.as_str() == Some("mtp")
    {
        *spec_type = serde_json::Value::String("draft-mtp".to_string());
    }

    let is_mtp = processed_config.get("spec-type").and_then(|v| v.as_str()) == Some("draft-mtp");

    if is_mtp
        && !processed_config.contains_key("lh-spec-draft-n-max")
        && !processed_config.contains_key("spec-draft-n-max")
    {
        processed_config.insert(
            "spec-draft-n-max".to_string(),
            serde_json::Value::Number(4.into()),
        );
    }

    for (key, val) in &processed_config {
        if crate::config::is_restricted_key(key) {
            continue;
        }
        if let Some(arg_name) = crate::config::format_arg_name(key) {
            if let Some(b) = val.as_bool() {
                if b {
                    params.push(arg_name);
                }
            } else if let Some(i) = val.as_i64() {
                params.push(arg_name);
                params.push(i.to_string());
            } else if let Some(f) = val.as_f64() {
                params.push(arg_name);
                params.push(f.to_string());
            } else if let Some(s) = val.as_str() {
                params.push(arg_name);
                params.push(s.to_string());
            } else if let Some(arr) = val.as_array() {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| v.to_string())
                    })
                    .collect();
                params.push(arg_name);
                params.push(items.join(","));
            }
        }
    }

    if let Some(ref mmproj) = settings.mmproj {
        params.push("--mmproj".to_string());
        params.push(mmproj.to_string_lossy().into_owned());
    }

    if let Some(ref draft) = settings.draft_model {
        params.push("-md".to_string());
        params.push(draft.to_string_lossy().into_owned());
        params.push("-ngld".to_string());
        params.push(settings.draft_ngl.clone());
    }

    if let Some(ref template) = assets.jinja_template {
        params.push("--jinja".to_string());
        params.push("--chat-template-file".to_string());
        params.push(template.to_string_lossy().into_owned());
    }

    params
}

pub fn build_router_launch_parameters(
    executable_path: &Path,
    preset_path: &Path,
    global_config: &HashMap<String, serde_json::Value>,
) -> Vec<String> {
    let mut params = Vec::new();
    params.push(executable_path.to_string_lossy().into_owned());

    params.push("--models-preset".to_string());
    params.push(preset_path.to_string_lossy().into_owned());

    let host = global_config
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0.0");
    params.push("--host".to_string());
    params.push(host.to_string());

    let port = global_config
        .get("port")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "8080".to_string());
    params.push("--port".to_string());
    params.push(port);

    params.push("--log-colors".to_string());
    params.push("on".to_string());

    let flash_attn = global_config
        .get("flash-attn")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    params.push("--flash-attn".to_string());
    params.push(flash_attn.to_string());

    let cache_ram = global_config
        .get("cache-ram")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_string());
    params.push("--cache-ram".to_string());
    params.push(cache_ram);

    let models_max = global_config
        .get("models-max")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "1".to_string());
    params.push("--models-max".to_string());
    params.push(models_max);

    let np = global_config
        .get("np")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "1".to_string());
    params.push("-np".to_string());
    params.push(np);

    let threads = global_config
        .get("threads")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(crate::config::get_optimal_threads);
    params.push("-t".to_string());
    params.push(threads);

    params.push("--props".to_string());

    if let Some(bs) = global_config.get("batch-size").and_then(|v| v.as_i64()) {
        params.push("-b".to_string());
        params.push(bs.to_string());
    }
    if let Some(ubs) = global_config.get("ubatch-size").and_then(|v| v.as_i64()) {
        params.push("-ub".to_string());
        params.push(ubs.to_string());
    }
    if let Some(tools) = global_config.get("tools").and_then(|v| v.as_str()) {
        params.push("--tools".to_string());
        params.push(tools.to_string());
    }

    let ui_enabled = global_config
        .get("ui")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !ui_enabled {
        params.push("--no-ui".to_string());
    }

    params
}
