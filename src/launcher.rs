use crate::config::{ModelAssets, UserSettings};
use std::collections::HashMap;
use std::path::Path;

pub fn kill_existing_servers() {
    // Left empty to prevent killing unrelated global llama-server instances.
    // LlamaHerd now manages child processes isolated by PID.
}

pub fn get_server_version(executable_path: &Path) -> String {
    use std::process::Command;
    if let Ok(output) = Command::new(executable_path).arg("--version").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        for line in combined.lines() {
            let line = line.trim();
            if !line.is_empty() {
                if let Some(stripped) = line.strip_prefix("version: ") {
                    return stripped
                        .split_whitespace()
                        .next()
                        .unwrap_or(stripped)
                        .to_string();
                } else if let Some(stripped) = line.strip_prefix("llama version ") {
                    return stripped
                        .split_whitespace()
                        .next()
                        .unwrap_or(stripped)
                        .to_string();
                }
            }
        }

        // Fallback: take the first 20 chars of the first non-empty line
        if let Some(first_line) = combined.lines().find(|l| !l.trim().is_empty()) {
            let truncated: String = first_line.trim().chars().take(20).collect();
            return truncated;
        }
    }
    "Unknown".to_string()
}

pub fn build_launch_parameters(
    executable_path: &Path,
    model_path: &Path,
    assets: &ModelAssets,
    settings: &UserSettings,
    global_config: &HashMap<String, serde_json::Value>,
    resolved_port: u16,
) -> Vec<String> {
    let mut params = Vec::new();
    params.push(executable_path.to_string_lossy().into_owned());

    params.push("-m".to_string());
    params.push(model_path.to_string_lossy().into_owned());

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

    let host = get_global_long("host")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0.0");
    params.push("--host".to_string());
    params.push(host.to_string());

    params.push("--port".to_string());
    params.push(resolved_port.to_string());

    params.push("--log-colors".to_string());
    params.push("on".to_string());

    params.push("-ngl".to_string());
    params.push(settings.ngl.clone());

    params.push("--ctx-size".to_string());
    params.push(settings.ctx.to_string());

    let flash_attn = get_global_long("flash-attn")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    params.push("--flash-attn".to_string());
    params.push(flash_attn.to_string());

    let cache_ram = get_global_long("cache-ram")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "-1".to_string());
    params.push("--cache-ram".to_string());
    params.push(cache_ram);

    let np = get_global_long("np")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "1".to_string());
    params.push("-np".to_string());
    params.push(np);

    let threads = get_global_long("threads")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(crate::config::get_optimal_threads);
    params.push("-t".to_string());
    params.push(threads);

    params.push("--kv-unified".to_string());

    if let Some(bs) = get_global_long("batch-size").and_then(|v| v.as_i64()) {
        params.push("-b".to_string());
        params.push(bs.to_string());
    }
    if let Some(ubs) = get_global_long("ubatch-size").and_then(|v| v.as_i64()) {
        params.push("-ub".to_string());
        params.push(ubs.to_string());
    }
    if let Some(tools) = get_global_long("tools").and_then(|v| v.as_str()) {
        params.push("--tools".to_string());
        params.push(tools.to_string());
    }

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

    let kv_quant = get_global_long("kv-quant")
        .or_else(|| get_lh_val("kv-quant"))
        .or_else(|| get_long_val("kv-quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("q8_0");
    params.push("-ctk".to_string());
    params.push(kv_quant.to_string());
    params.push("-ctv".to_string());
    params.push(kv_quant.to_string());

    if !settings.ui {
        params.push("--no-ui".to_string());
    }

    // Helper to format and add a long parameter
    let add_long_param = |arg: &str, val: &serde_json::Value, params: &mut Vec<String>| {
        if crate::config::is_restricted_key(arg) {
            return;
        }
        let arg_name = format!("--{}", arg);
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
    };

    // 1. Process root level long options
    let mut sorted_root_keys: Vec<&String> = assets.config.keys().collect();
    sorted_root_keys.sort();
    for k in sorted_root_keys {
        if k == "llama-herd" || k == "llama-server-short" || k == "llama-server-long" {
            continue;
        }
        add_long_param(k, &assets.config[k], &mut params);
    }

    // 2. Process llama-server-long table options
    if let Some(long_obj) = assets
        .config
        .get("llama-server-long")
        .and_then(|v| v.as_object())
    {
        let mut sorted_long_keys: Vec<&String> = long_obj.keys().collect();
        sorted_long_keys.sort();
        for k in sorted_long_keys {
            add_long_param(k, &long_obj[k], &mut params);
        }
    }

    // 3. Process llama-server-short table options
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
            let arg_name = format!("-{}", k);
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

        if let Some(parent) = draft.parent() {
            let draft_assets = crate::config::discover_assets(draft, parent);

            let get_draft_long = |key: &str| -> Option<&serde_json::Value> {
                draft_assets
                    .config
                    .get("llama-server-long")
                    .and_then(|l| l.get(key))
                    .or_else(|| draft_assets.config.get(key))
            };

            let spec_type = get_long_val("spec-type")
                .or_else(|| get_draft_long("spec-type"))
                .and_then(|v| v.as_str())
                .unwrap_or("draft-mtp");

            let spec_draft_n_max = get_long_val("spec-draft-n-max")
                .or_else(|| get_draft_long("spec-draft-n-max"))
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        s.parse::<u64>().ok()
                    } else if let Some(n) = v.as_u64() {
                        Some(n)
                    } else {
                        v.as_i64().map(|i| i as u64)
                    }
                })
                .unwrap_or(4);

            let spec_draft_p_min = get_long_val("spec-draft-p-min")
                .or_else(|| get_draft_long("spec-draft-p-min"))
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        s.parse::<f64>().ok()
                    } else if let Some(f) = v.as_f64() {
                        Some(f)
                    } else {
                        v.as_i64().map(|i| i as f64)
                    }
                })
                .unwrap_or(0.0);

            let has_main_spec_type = assets.config.contains_key("spec-type")
                || assets
                    .config
                    .get("llama-server-long")
                    .and_then(|l| l.get("spec-type"))
                    .is_some();
            let has_main_n_max = assets.config.contains_key("spec-draft-n-max")
                || assets
                    .config
                    .get("llama-server-long")
                    .and_then(|l| l.get("spec-draft-n-max"))
                    .is_some();
            let has_main_p_min = assets.config.contains_key("spec-draft-p-min")
                || assets
                    .config
                    .get("llama-server-long")
                    .and_then(|l| l.get("spec-draft-p-min"))
                    .is_some();

            if !has_main_spec_type {
                params.push("--spec-type".to_string());
                params.push(spec_type.to_string());
            }
            if !has_main_n_max {
                params.push("--spec-draft-n-max".to_string());
                params.push(spec_draft_n_max.to_string());
            }
            if !has_main_p_min {
                params.push("--spec-draft-p-min".to_string());
                params.push(spec_draft_p_min.to_string());
            }
        }
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
    resolved_port: u16,
) -> Vec<String> {
    let mut params = Vec::new();
    params.push(executable_path.to_string_lossy().into_owned());

    params.push("--models-preset".to_string());
    params.push(preset_path.to_string_lossy().into_owned());

    let get_global_lh = |key: &str| -> Option<&serde_json::Value> {
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

    let host = get_global_long("host")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0.0");
    params.push("--host".to_string());
    params.push(host.to_string());

    params.push("--port".to_string());
    params.push(resolved_port.to_string());

    params.push("--log-colors".to_string());
    params.push("on".to_string());

    let flash_attn = get_global_long("flash-attn")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    params.push("--flash-attn".to_string());
    params.push(flash_attn.to_string());

    let cache_ram = get_global_long("cache-ram")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_string());
    params.push("--cache-ram".to_string());
    params.push(cache_ram);

    let models_max = get_global_lh("models-max")
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

    let np = get_global_long("np")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "1".to_string());
    params.push("-np".to_string());
    params.push(np);

    let threads = get_global_long("threads")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(crate::config::get_optimal_threads);
    params.push("-t".to_string());
    params.push(threads);

    params.push("--props".to_string());

    if let Some(bs) = get_global_long("batch-size").and_then(|v| v.as_i64()) {
        params.push("-b".to_string());
        params.push(bs.to_string());
    }
    if let Some(ubs) = get_global_long("ubatch-size").and_then(|v| v.as_i64()) {
        params.push("-ub".to_string());
        params.push(ubs.to_string());
    }
    if let Some(tools) = get_global_long("tools").and_then(|v| v.as_str()) {
        params.push("--tools".to_string());
        params.push(tools.to_string());
    }

    let ui_enabled = get_global_lh("ui")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !ui_enabled {
        params.push("--no-ui".to_string());
    }

    params
}

pub fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn resolve_port(port_str: &str) -> u16 {
    if port_str == "auto" {
        let mut port = 8080;
        while port < 65535 {
            if is_port_available(port) {
                return port;
            }
            port += 1;
        }
        8080
    } else {
        let parsed: u16 = port_str.parse().unwrap_or(8080);
        if is_port_available(parsed) {
            parsed
        } else {
            let mut port = parsed + 1;
            while port < 65535 {
                if is_port_available(port) {
                    return port;
                }
                port += 1;
            }
            parsed
        }
    }
}
