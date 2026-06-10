use crate::config::{ModelAssets, UserSettings};
use std::collections::HashMap;
use std::path::Path;

pub fn add_active_pid(pid: u32) {
    let lh_dir = crate::config::get_llama_herd_dir();
    let _ = std::fs::create_dir_all(&lh_dir);
    let pids_file = lh_dir.join("active_pids.txt");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&pids_file)
    {
        use std::io::Write;
        let _ = writeln!(file, "{}", pid);
    }
}

pub fn remove_active_pid(pid: u32) {
    let pids_file = crate::config::get_llama_herd_dir().join("active_pids.txt");
    if !pids_file.exists() {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(&pids_file) {
        let mut new_lines = Vec::new();
        for line in content.lines() {
            if let Ok(val) = line.trim().parse::<u32>()
                && val != pid
            {
                new_lines.push(line.to_string());
            }
        }
        let _ = std::fs::write(&pids_file, new_lines.join("\n"));
    }
}

pub fn kill_existing_servers() {
    let pids_file = crate::config::get_llama_herd_dir().join("active_pids.txt");
    if !pids_file.exists() {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(&pids_file) {
        use sysinfo::{Pid, System};
        let mut sys = System::new();
        sys.refresh_all();

        for line in content.lines() {
            if let Ok(pid_val) = line.trim().parse::<u32>() {
                let pid = Pid::from_u32(pid_val);
                if let Some(process) = sys.process(pid) {
                    let name = process.name().to_string_lossy().to_lowercase();
                    if name.contains("llama-server") {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("taskkill")
                                .args(["/F", "/PID", &pid_val.to_string(), "/T"])
                                .output();
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = process.kill();
                        }
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_file(pids_file);
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
        .unwrap_or("127.0.0.1");
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
        .unwrap_or_else(|| "-1".to_string());
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
        .unwrap_or_else(|| "-1".to_string());
    params.push("-t".to_string());
    params.push(threads);

    let kv_unified = get_global_long("kv-unified")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if kv_unified {
        params.push("--kv-unified".to_string());
    }

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

    let cache_type_k = get_global_long("cache-type-k")
        .or_else(|| get_lh_val("cache-type-k"))
        .or_else(|| get_long_val("cache-type-k"))
        .or_else(|| get_global_long("kv-quant"))
        .or_else(|| get_lh_val("kv-quant"))
        .or_else(|| get_long_val("kv-quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("f16");
    params.push("-ctk".to_string());
    params.push(cache_type_k.to_string());

    let cache_type_v = get_global_long("cache-type-v")
        .or_else(|| get_lh_val("cache-type-v"))
        .or_else(|| get_long_val("cache-type-v"))
        .or_else(|| get_global_long("kv-quant"))
        .or_else(|| get_lh_val("kv-quant"))
        .or_else(|| get_long_val("kv-quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("f16");
    params.push("-ctv".to_string());
    params.push(cache_type_v.to_string());

    let ui_enabled = global_config
        .get("llama-herd")
        .and_then(|lh| lh.get("ui"))
        .or_else(|| global_config.get("ui"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !ui_enabled {
        params.push("--no-ui".to_string());
    }

    let api_key = get_global_long("api-key")
        .and_then(|v| v.as_str())
        .unwrap_or("disabled");
    if api_key != "disabled" && !api_key.is_empty() {
        params.push("--api-key".to_string());
        params.push(api_key.to_string());
    }

    let metrics = get_global_long("metrics")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if metrics {
        params.push("--metrics".to_string());
    }

    let log_verbosity = get_global_long("log-verbosity")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "3".to_string());
    params.push("--log-verbosity".to_string());
    params.push(log_verbosity);

    // Helper to format and add a long parameter
    let add_long_param = |arg: &str, val: &serde_json::Value, params: &mut Vec<String>| {
        if crate::config::is_restricted_key(arg) {
            return;
        }
        if !crate::config::is_safe_value(val) {
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
            if !crate::config::is_safe_value(val) {
                continue;
            }
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

    // Temperature, Top P, Top K parameters mapping
    let temp = get_lh_val("temp")
        .or_else(|| get_long_val("temp"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<f64>().ok()
            } else if let Some(f) = v.as_f64() {
                Some(f)
            } else {
                v.as_i64().map(|i| i as f64)
            }
        });
    if let Some(t) = temp {
        params.push("--temp".to_string());
        params.push(t.to_string());
    }

    let top_p = get_lh_val("top-p")
        .or_else(|| get_long_val("top-p"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<f64>().ok()
            } else if let Some(f) = v.as_f64() {
                Some(f)
            } else {
                v.as_i64().map(|i| i as f64)
            }
        });
    if let Some(p) = top_p {
        params.push("--top-p".to_string());
        params.push(p.to_string());
    }

    let top_k = get_lh_val("top-k")
        .or_else(|| get_long_val("top-k"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<i64>().ok()
            } else if let Some(n) = v.as_u64() {
                Some(n as i64)
            } else {
                v.as_i64()
            }
        });
    if let Some(k) = top_k {
        params.push("--top-k".to_string());
        params.push(k.to_string());
    }

    // Context Checkpoints, Checkpoint Min Step, and Memory Mapping parameters mapping
    let ctx_checkpoints = get_global_long("ctx-checkpoints")
        .or_else(|| get_lh_val("ctx-checkpoints"))
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
    if let Some(checkpoints) = ctx_checkpoints {
        params.push("--ctx-checkpoints".to_string());
        params.push(checkpoints.to_string());
    }

    let checkpoint_min_step = get_global_long("checkpoint-min-step")
        .or_else(|| get_lh_val("checkpoint-min-step"))
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
    if let Some(step) = checkpoint_min_step {
        params.push("--checkpoint-min-step".to_string());
        params.push(step.to_string());
    }

    let no_mmap = get_global_long("no-mmap")
        .or_else(|| get_lh_val("no-mmap"))
        .or_else(|| get_long_val("no-mmap"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if no_mmap {
        params.push("--no-mmap".to_string());
    }

    if let Some(ref template) = assets.jinja_template {
        params.push("--jinja".to_string());
        params.push("--chat-template-file".to_string());
        params.push(template.to_string_lossy().into_owned());
    }

    // New Global settings
    let cache_prompt = get_global_long("cache-prompt")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !cache_prompt {
        params.push("--no-cache-prompt".to_string());
    }

    let context_shift = get_global_long("context-shift")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if context_shift {
        params.push("--context-shift".to_string());
    }

    let mlock = get_global_long("mlock")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if mlock {
        params.push("--mlock".to_string());
    }

    let numa = get_global_long("numa")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if numa != "none" && !numa.is_empty() {
        params.push("--numa".to_string());
        params.push(numa.to_string());
    }

    let split_mode = get_global_long("split-mode")
        .and_then(|v| v.as_str())
        .unwrap_or("layer");
    if !split_mode.is_empty() {
        params.push("--split-mode".to_string());
        params.push(split_mode.to_string());
    }

    let device = get_global_long("device")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if device != "none" && !device.is_empty() {
        params.push("--device".to_string());
        params.push(device.to_string());
    }

    let api_key_file = get_global_long("api-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if api_key_file != "none" && !api_key_file.is_empty() {
        params.push("--api-key-file".to_string());
        params.push(api_key_file.to_string());
    }

    let ssl_key_file = get_global_long("ssl-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_key_file != "none" && !ssl_key_file.is_empty() {
        params.push("--ssl-key-file".to_string());
        params.push(ssl_key_file.to_string());
    }

    let ssl_cert_file = get_global_long("ssl-cert-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_cert_file != "none" && !ssl_cert_file.is_empty() {
        params.push("--ssl-cert-file".to_string());
        params.push(ssl_cert_file.to_string());
    }

    // New Model-specific settings
    let min_p = get_lh_val("min-p")
        .or_else(|| get_long_val("min-p"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<f64>().ok()
            } else if let Some(f) = v.as_f64() {
                Some(f)
            } else {
                v.as_i64().map(|i| i as f64)
            }
        });
    if let Some(mp) = min_p {
        params.push("--min-p".to_string());
        params.push(mp.to_string());
    }

    let repeat_penalty = get_lh_val("repeat-penalty")
        .or_else(|| get_long_val("repeat-penalty"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<f64>().ok()
            } else if let Some(f) = v.as_f64() {
                Some(f)
            } else {
                v.as_i64().map(|i| i as f64)
            }
        });
    if let Some(rp) = repeat_penalty {
        params.push("--repeat-penalty".to_string());
        params.push(rp.to_string());
    }

    let repeat_last_n = get_lh_val("repeat-last-n")
        .or_else(|| get_long_val("repeat-last-n"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<i64>().ok()
            } else if let Some(n) = v.as_u64() {
                Some(n as i64)
            } else {
                v.as_i64()
            }
        });
    if let Some(rln) = repeat_last_n {
        params.push("--repeat-last-n".to_string());
        params.push(rln.to_string());
    }

    let reasoning = get_lh_val("reasoning")
        .or_else(|| get_long_val("reasoning"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(r) = reasoning {
        params.push("--reasoning".to_string());
        params.push(r);
    }

    let reasoning_format = get_lh_val("reasoning-format")
        .or_else(|| get_long_val("reasoning-format"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(rf) = reasoning_format {
        params.push("--reasoning-format".to_string());
        params.push(rf);
    }

    let reasoning_budget = get_lh_val("reasoning-budget")
        .or_else(|| get_long_val("reasoning-budget"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<i64>().ok()
            } else if let Some(n) = v.as_u64() {
                Some(n as i64)
            } else {
                v.as_i64()
            }
        });
    if let Some(rb) = reasoning_budget {
        params.push("--reasoning-budget".to_string());
        params.push(rb.to_string());
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
        .unwrap_or("127.0.0.1");
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
        .unwrap_or_else(|| "-1".to_string());
    params.push("-np".to_string());
    params.push(np);

    let threads = get_global_long("threads")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_string());
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

    let kv_unified = get_global_long("kv-unified")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if kv_unified {
        params.push("--kv-unified".to_string());
    }

    let api_key = get_global_long("api-key")
        .and_then(|v| v.as_str())
        .unwrap_or("disabled");
    if api_key != "disabled" && !api_key.is_empty() {
        params.push("--api-key".to_string());
        params.push(api_key.to_string());
    }

    let metrics = get_global_long("metrics")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if metrics {
        params.push("--metrics".to_string());
    }

    let log_verbosity = get_global_long("log-verbosity")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(i.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "3".to_string());
    params.push("--log-verbosity".to_string());
    params.push(log_verbosity);

    // New Global settings
    let cache_prompt = get_global_long("cache-prompt")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !cache_prompt {
        params.push("--no-cache-prompt".to_string());
    }

    let context_shift = get_global_long("context-shift")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if context_shift {
        params.push("--context-shift".to_string());
    }

    let mlock = get_global_long("mlock")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if mlock {
        params.push("--mlock".to_string());
    }

    let numa = get_global_long("numa")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if numa != "none" && !numa.is_empty() {
        params.push("--numa".to_string());
        params.push(numa.to_string());
    }

    let split_mode = get_global_long("split-mode")
        .and_then(|v| v.as_str())
        .unwrap_or("layer");
    if !split_mode.is_empty() {
        params.push("--split-mode".to_string());
        params.push(split_mode.to_string());
    }

    let device = get_global_long("device")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if device != "none" && !device.is_empty() {
        params.push("--device".to_string());
        params.push(device.to_string());
    }

    let api_key_file = get_global_long("api-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if api_key_file != "none" && !api_key_file.is_empty() {
        params.push("--api-key-file".to_string());
        params.push(api_key_file.to_string());
    }

    let ssl_key_file = get_global_long("ssl-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_key_file != "none" && !ssl_key_file.is_empty() {
        params.push("--ssl-key-file".to_string());
        params.push(ssl_key_file.to_string());
    }

    let ssl_cert_file = get_global_long("ssl-cert-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_cert_file != "none" && !ssl_cert_file.is_empty() {
        params.push("--ssl-cert-file".to_string());
        params.push(ssl_cert_file.to_string());
    }

    params
}

pub fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn resolve_port(port_str: &str) -> Result<u16, std::io::Error> {
    if port_str == "auto" {
        let mut port = 8080;
        while port < 65535 {
            if is_port_available(port) {
                return Ok(port);
            }
            port += 1;
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "No available ports found in range 8080-65535".to_string(),
        ))
    } else {
        let parsed: u16 = port_str.parse().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to parse port: {}", e),
            )
        })?;

        if is_port_available(parsed) {
            Ok(parsed)
        } else {
            let max_limit = parsed.saturating_add(10);
            let mut port = parsed + 1;
            while port <= max_limit && port < 65535 {
                if is_port_available(port) {
                    return Ok(port);
                }
                port += 1;
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!(
                    "Requested port {} and its subsequent retries (+10) are all occupied.",
                    parsed
                ),
            ))
        }
    }
}
