use crate::config::{ModelAssets, UserSettings};
pub use crate::tui::logs::SupervisorConfig;
use std::collections::HashMap;
use std::path::Path;

use std::sync::{LazyLock, Mutex};

static PIDS_FILE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Appends a new server subprocess PID to the active PIDs tracking file.
///
/// # Panics
///
/// Panics if the internal file lock is poisoned.
pub fn add_active_pid(pid: u32) {
    let _guard = PIDS_FILE_LOCK.lock().expect("pids lock poisoned");
    let lh_dir = crate::config::get_llama_herd_dir();
    let _ = std::fs::create_dir_all(&lh_dir);
    let pids_file = lh_dir.join("active_pids.txt");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&pids_file)
    {
        use std::io::Write;
        let _ = writeln!(file, "{pid}");
    }
}

/// Removes a server subprocess PID from the active PIDs tracking file.
///
/// # Panics
///
/// Panics if the internal file lock is poisoned.
pub fn remove_active_pid(pid: u32) {
    let _guard = PIDS_FILE_LOCK.lock().expect("pids lock poisoned");
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
                new_lines.push(line.to_owned());
            }
        }
        let _ = std::fs::write(&pids_file, new_lines.join("\n"));
    }
}

/// Terminates any stray or active servers tracked in the active PIDs file.
///
/// # Panics
///
/// Panics if the internal file lock is poisoned.
pub fn kill_existing_servers() {
    let _guard = PIDS_FILE_LOCK.lock().expect("pids lock poisoned");
    let pids_file = crate::config::get_llama_herd_dir().join("active_pids.txt");
    if !pids_file.exists() {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(&pids_file) {
        use sysinfo::{Pid, ProcessesToUpdate, RefreshKind, System};
        let pids: Vec<Pid> = content
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok().map(Pid::from_u32))
            .collect();

        if !pids.is_empty()
            && let Ok(mut sys) = System::new_with_specifics(RefreshKind::nothing())
        {
            sys.refresh_processes(ProcessesToUpdate::Some(&pids), true);

            for pid in pids {
                if let Some(process) = sys.process(pid) {
                    let name = process.name().to_string_lossy().to_lowercase();
                    if name.contains("llama-server") {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("taskkill")
                                .args(["/F", "/PID", &pid.as_u32().to_string(), "/T"])
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

static BUILD_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(?:\bbuild[:=\s]+b?|\(b)(\d+)\b").expect("invalid build regex")
});

/// Parses the raw output from `llama-server --version` or `llama-cli --version` to extract the version and build number.
///
/// If both semver and build number are present (e.g. `"version: 0.4.0-dev (build 10894, commit d344123fe)"`),
/// formats as `"0.4.0-dev b10894"`. If only build number is present (e.g. `"version: 4567"`), formats as `"b4567"`.
///
/// # Panics
///
/// Panics if the internal regex pattern fails to compile.
#[must_use]
pub fn parse_server_version(raw: &str) -> String {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_version_line = line.starts_with("version:")
            || line.starts_with("llama version")
            || line.contains("build =");

        if is_version_line {
            let build_num = BUILD_RE
                .captures(line)
                .and_then(|caps| caps.get(1))
                .map(|m| format!("b{}", m.as_str()));

            // Extract the first token after "version:" or "llama version"
            let rest = line
                .strip_prefix("version:")
                .or_else(|| line.strip_prefix("llama version"))
                .unwrap_or("")
                .trim();

            let token = rest
                .split_whitespace()
                .next()
                .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '-'))
                .filter(|t| !t.is_empty());

            match (token, build_num) {
                (Some(ver), Some(build)) => {
                    // If the first token is already just the build number (e.g. "4567" or "b10894"),
                    // don't duplicate it.
                    if ver.chars().all(|c| c.is_ascii_digit())
                        || (ver.starts_with(['b', 'B'])
                            && ver.len() > 1
                            && ver[1..].chars().all(|c| c.is_ascii_digit()))
                    {
                        return build;
                    }
                    let clean_ver = ver.strip_prefix(['v', 'V']).unwrap_or(ver);
                    return format!("{clean_ver} {build}");
                }
                (Some(ver), None) => {
                    if ver.chars().all(|c| c.is_ascii_digit()) {
                        return format!("b{ver}");
                    }
                    if ver.starts_with(['b', 'B'])
                        && ver.len() > 1
                        && ver[1..].chars().all(|c| c.is_ascii_digit())
                    {
                        return ver.to_lowercase();
                    }
                    let clean_ver = ver.strip_prefix(['v', 'V']).unwrap_or(ver);
                    return clean_ver.to_owned();
                }
                (None, Some(build)) => return build,
                (None, None) => {}
            }
        }
    }

    // Global scan for build number across any line if not found in a specific version line
    if let Some(caps) = BUILD_RE.captures(raw)
        && let Some(m) = caps.get(1)
    {
        return format!("b{}", m.as_str());
    }

    // Fallback: take the first 20 chars of the first non-empty line
    if let Some(first_line) = raw.lines().find(|l| !l.trim().is_empty()) {
        let truncated: String = first_line.trim().chars().take(20).collect();
        return truncated;
    }

    "Unknown".to_owned()
}

/// Checks the server version by running the executable with the `--version` flag.
#[must_use]
pub fn get_server_version(executable_path: &Path) -> String {
    use std::process::Command;
    if let Ok(output) = Command::new(executable_path).arg("--version").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");
        return parse_server_version(&combined);
    }
    "Unknown".to_owned()
}

/// Builds command-line parameters to launch llama-server for a single model preset.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_launch_parameters<S: std::hash::BuildHasher>(
    executable_path: &Path,
    model_path: &Path,
    assets: &ModelAssets,
    settings: &UserSettings,
    global_config: &HashMap<String, serde_json::Value, S>,
    resolved_port: u16,
) -> Vec<String> {
    let mut params = Vec::new();
    params.push(executable_path.to_string_lossy().into_owned());

    params.push("-m".to_owned());
    params.push(model_path.to_string_lossy().into_owned());

    let get_global_long = |key: &str| -> Option<&serde_json::Value> {
        global_config
            .get("llama-server-long")
            .and_then(|l| l.get(key))
            .or_else(|| global_config.get(key))
    };

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

    let resolved_alias = settings.model_alias.clone().or_else(|| {
        let custom_names_val = get_lh_val("custom-name")
            .or_else(|| get_lh_val("custom-names"))
            .or_else(|| get_long_val("custom-name"))
            .or_else(|| get_long_val("custom-names"));
        let variant_key = match (settings.draft_model.is_some(), settings.mmproj.is_some()) {
            (true, true) => "draft-vision",
            (true, false) => "draft",
            (false, true) => "vision",
            (false, false) => "base",
        };
        let mut custom = None;
        if let Some(val) = custom_names_val {
            if let Some(obj) = val.as_object() {
                if let Some(s) = obj.get(variant_key).and_then(|v| v.as_str()) {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        custom = Some(trimmed.to_owned());
                    }
                }
            } else if variant_key == "base"
                && let Some(s) = val.as_str()
            {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    custom = Some(trimmed.to_owned());
                }
            }
        }
        custom.or_else(|| {
            let clean = crate::discovery::clean_model_id(model_path);
            let name = match variant_key {
                "draft" => crate::discovery::insert_variant_suffix(&clean, "draft"),
                "vision" => crate::discovery::insert_variant_suffix(&clean, "vision"),
                "draft-vision" => crate::discovery::insert_variant_suffix(&clean, "draft-vision"),
                _ => clean,
            };
            Some(name)
        })
    });

    if let Some(alias) = resolved_alias {
        params.push("--alias".to_owned());
        params.push(alias);
    }

    let host = get_global_long("host")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1");
    params.push("--host".to_owned());
    params.push(host.to_owned());

    params.push("--port".to_owned());
    params.push(resolved_port.to_string());

    params.push("--log-colors".to_owned());
    params.push("on".to_owned());

    let total_layers = get_lh_val("total-layers")
        .or_else(|| get_long_val("total-layers"))
        .and_then(|v| v.as_u64().and_then(|i| usize::try_from(i).ok()));
    let resolved_ngl = crate::config::calculate_ngl(&settings.ngl, "auto", total_layers);
    params.push("-ngl".to_owned());
    params.push(resolved_ngl);

    params.push("--ctx-size".to_owned());
    params.push(settings.ctx.to_string());

    let flash_attn = get_global_long("flash-attn")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    params.push("--flash-attn".to_owned());
    params.push(flash_attn.to_owned());

    let cache_ram = get_global_long("cache-ram")
        .and_then(|v| {
            v.as_i64().map_or_else(
                || v.as_str().map(ToOwned::to_owned),
                |i| Some(i.to_string()),
            )
        })
        .unwrap_or_else(|| "-1".to_owned());
    params.push("--cache-ram".to_owned());
    params.push(cache_ram);

    let np = get_global_long("np")
        .and_then(|v| {
            v.as_i64().map_or_else(
                || v.as_str().map(ToOwned::to_owned),
                |i| Some(i.to_string()),
            )
        })
        .unwrap_or_else(|| "-1".to_owned());
    params.push("-np".to_owned());
    params.push(np);

    let threads = get_global_long("threads")
        .and_then(|v| {
            v.as_i64().map_or_else(
                || v.as_str().map(ToOwned::to_owned),
                |i| Some(i.to_string()),
            )
        })
        .unwrap_or_else(|| "-1".to_owned());
    params.push("-t".to_owned());
    params.push(threads);

    let kv_unified = get_global_long("kv-unified")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if kv_unified {
        params.push("--kv-unified".to_owned());
    }

    if let Some(bs) = get_global_long("batch-size").and_then(serde_json::Value::as_i64) {
        params.push("-b".to_owned());
        params.push(bs.to_string());
    }
    if let Some(ubs) = get_global_long("ubatch-size").and_then(serde_json::Value::as_i64) {
        params.push("-ub".to_owned());
        params.push(ubs.to_string());
    }
    if let Some(tools) = get_global_long("tools").and_then(|v| v.as_str()) {
        params.push("--tools".to_owned());
        params.push(tools.to_owned());
    }

    let cache_type_k = get_global_long("cache-type-k")
        .or_else(|| get_lh_val("cache-type-k"))
        .or_else(|| get_long_val("cache-type-k"))
        .or_else(|| get_global_long("kv-quant"))
        .or_else(|| get_lh_val("kv-quant"))
        .or_else(|| get_long_val("kv-quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("f16");
    params.push("-ctk".to_owned());
    params.push(cache_type_k.to_owned());

    let cache_type_v = get_global_long("cache-type-v")
        .or_else(|| get_lh_val("cache-type-v"))
        .or_else(|| get_long_val("cache-type-v"))
        .or_else(|| get_global_long("kv-quant"))
        .or_else(|| get_lh_val("kv-quant"))
        .or_else(|| get_long_val("kv-quant"))
        .and_then(|v| v.as_str())
        .unwrap_or("f16");
    params.push("-ctv".to_owned());
    params.push(cache_type_v.to_owned());

    let ui_enabled = global_config
        .get("llama-herd")
        .and_then(|lh| lh.get("ui"))
        .or_else(|| global_config.get("ui"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !ui_enabled {
        params.push("--no-ui".to_owned());
    }

    let api_key = get_global_long("api-key")
        .and_then(|v| v.as_str())
        .unwrap_or("disabled");
    if api_key != "disabled" && !api_key.is_empty() {
        params.push("--api-key".to_owned());
        params.push(api_key.to_owned());
    }

    let metrics = get_global_long("metrics")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if metrics {
        params.push("--metrics".to_owned());
    }

    let log_verbosity = get_global_long("log-verbosity")
        .and_then(|v| {
            v.as_i64().map_or_else(
                || v.as_str().map(ToOwned::to_owned),
                |i| Some(i.to_string()),
            )
        })
        .unwrap_or_else(|| "3".to_owned());
    params.push("--log-verbosity".to_owned());
    params.push(log_verbosity);

    // Helper to format and add a long parameter
    let add_long_param = |arg: &str, val: &serde_json::Value, params: &mut Vec<String>| {
        if crate::config::is_restricted_key(arg) {
            return;
        }
        if !crate::config::is_safe_value(val) {
            return;
        }
        let arg_name = format!("--{arg}");
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
            params.push(s.to_owned());
        } else if let Some(arr) = val.as_array() {
            let items: Vec<String> = arr
                .iter()
                .map(|v| v.as_str().map_or_else(|| v.to_string(), ToOwned::to_owned))
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
            let arg_name = format!("-{k}");
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
                params.push(s.to_owned());
            }
        }
    }

    if let Some(ref mmproj) = settings.mmproj {
        params.push("--mmproj".to_owned());
        params.push(mmproj.to_string_lossy().into_owned());
    }

    if let Some(ref draft) = settings.draft_model {
        params.push("-md".to_owned());
        params.push(draft.to_string_lossy().into_owned());

        let (resolved_draft_ngl, draft_assets_opt) = draft.parent().map_or_else(
            || (settings.draft_ngl.clone(), None),
            |parent| {
                let draft_assets = crate::config::discover_assets(draft, parent);
                let get_draft_lh = |key: &str| -> Option<&serde_json::Value> {
                    draft_assets
                        .config
                        .get("llama-herd")
                        .and_then(|lh| lh.get(key))
                };
                let get_draft_long = |key: &str| -> Option<&serde_json::Value> {
                    draft_assets
                        .config
                        .get("llama-server-long")
                        .and_then(|l| l.get(key))
                        .or_else(|| draft_assets.config.get(key))
                };

                let draft_total_layers = get_draft_lh("total-layers")
                    .or_else(|| get_draft_long("total-layers"))
                    .and_then(|v| v.as_u64().and_then(|i| usize::try_from(i).ok()));

                let resolved =
                    crate::config::calculate_ngl(&settings.draft_ngl, "auto", draft_total_layers);
                (resolved, Some(draft_assets))
            },
        );

        params.push("-ngld".to_owned());
        params.push(resolved_draft_ngl);

        if let Some(draft_assets) = draft_assets_opt {
            let get_draft_long = |key: &str| -> Option<&serde_json::Value> {
                draft_assets
                    .config
                    .get("llama-server-long")
                    .and_then(|l| l.get(key))
                    .or_else(|| draft_assets.config.get(key))
            };

            let default_spec = crate::discovery::default_spec_type_for_draft(draft);
            let spec_type = get_long_val("spec-type")
                .or_else(|| get_draft_long("spec-type"))
                .and_then(|v| v.as_str())
                .unwrap_or(default_spec);

            let spec_draft_n_max = get_long_val("spec-draft-n-max")
                .or_else(|| get_draft_long("spec-draft-n-max"))
                .or_else(|| get_global_long("spec-draft-n-max"))
                .and_then(|v| {
                    v.as_str().map_or_else(
                        || {
                            v.as_u64().or_else(|| {
                                #[allow(clippy::cast_sign_loss)]
                                v.as_i64().map(|i| i as u64)
                            })
                        },
                        |s| s.parse::<u64>().ok(),
                    )
                })
                .unwrap_or(4);

            let spec_draft_p_min = get_long_val("spec-draft-p-min")
                .or_else(|| get_draft_long("spec-draft-p-min"))
                .or_else(|| get_global_long("spec-draft-p-min"))
                .and_then(|v| {
                    v.as_str().map_or_else(
                        || {
                            v.as_f64().or_else(|| {
                                #[allow(clippy::cast_precision_loss)]
                                v.as_i64().map(|i| i as f64)
                            })
                        },
                        |s| s.parse::<f64>().ok(),
                    )
                })
                .unwrap_or(0.0);

            params.push("--spec-type".to_owned());
            params.push(spec_type.to_owned());
            params.push("--spec-draft-n-max".to_owned());
            params.push(spec_draft_n_max.to_string());
            params.push("--spec-draft-p-min".to_owned());
            params.push(spec_draft_p_min.to_string());

            let spec_draft_n_ubatch = get_long_val("spec-draft-n-ubatch")
                .or_else(|| get_draft_long("spec-draft-n-ubatch"))
                .or_else(|| get_global_long("spec-draft-n-ubatch"))
                .and_then(|v| {
                    v.as_str().map_or_else(
                        || {
                            v.as_u64().or_else(|| {
                                #[allow(clippy::cast_sign_loss)]
                                v.as_i64().map(|i| i as u64)
                            })
                        },
                        |s| s.parse::<u64>().ok(),
                    )
                });
            if let Some(n_ubatch) = spec_draft_n_ubatch {
                params.push("--spec-draft-n-ubatch".to_owned());
                params.push(n_ubatch.to_string());
            }

            let spec_draft_sampling = get_long_val("spec-draft-sampling")
                .or_else(|| get_draft_long("spec-draft-sampling"))
                .or_else(|| get_global_long("spec-draft-sampling"))
                .and_then(|v| v.as_str());
            if let Some(sampling) = spec_draft_sampling {
                params.push("--spec-draft-sampling".to_owned());
                params.push(sampling.to_owned());
            }
        }
    }

    // Temperature, Top P, Top K parameters mapping
    let temp = get_lh_val("temp")
        .or_else(|| get_long_val("temp"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_f64().or_else(|| {
                        #[allow(clippy::cast_precision_loss)]
                        v.as_i64().map(|i| i as f64)
                    })
                },
                |s| s.parse::<f64>().ok(),
            )
        });
    if let Some(t) = temp {
        params.push("--temp".to_owned());
        params.push(t.to_string());
    }

    let top_p = get_lh_val("top-p")
        .or_else(|| get_long_val("top-p"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_f64().or_else(|| {
                        #[allow(clippy::cast_precision_loss)]
                        v.as_i64().map(|i| i as f64)
                    })
                },
                |s| s.parse::<f64>().ok(),
            )
        });
    if let Some(p) = top_p {
        params.push("--top-p".to_owned());
        params.push(p.to_string());
    }

    let top_k = get_lh_val("top-k")
        .or_else(|| get_long_val("top-k"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(k) = top_k {
        params.push("--top-k".to_owned());
        params.push(k.to_string());
    }

    // Context Checkpoints, Checkpoint Min Step, and Memory Mapping parameters mapping
    let ctx_checkpoints = get_global_long("ctx-checkpoints")
        .or_else(|| get_lh_val("ctx-checkpoints"))
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
    if let Some(checkpoints) = ctx_checkpoints {
        params.push("--ctx-checkpoints".to_owned());
        params.push(checkpoints.to_string());
    }

    let checkpoint_min_step = get_global_long("checkpoint-min-step")
        .or_else(|| get_lh_val("checkpoint-min-step"))
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
    if let Some(step) = checkpoint_min_step {
        params.push("--checkpoint-min-step".to_owned());
        params.push(step.to_string());
    }

    let explicit_load_mode = get_global_long("load-mode")
        .or_else(|| get_lh_val("load-mode"))
        .or_else(|| get_long_val("load-mode"))
        .and_then(|v| v.as_str());

    let mlock = get_global_long("mlock")
        .or_else(|| get_lh_val("mlock"))
        .or_else(|| get_long_val("mlock"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let no_mmap = get_global_long("no-mmap")
        .or_else(|| get_lh_val("no-mmap"))
        .or_else(|| get_long_val("no-mmap"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    if let Some(load_mode) = resolve_load_mode(explicit_load_mode, mlock, no_mmap) {
        params.push("--load-mode".to_owned());
        params.push(load_mode);
    }

    if let Some(ref template) = assets.jinja_template {
        params.push("--jinja".to_owned());
        params.push("--chat-template-file".to_owned());
        params.push(template.to_string_lossy().into_owned());
    }

    // New Global settings
    let cache_prompt = get_global_long("cache-prompt")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !cache_prompt {
        params.push("--no-cache-prompt".to_owned());
    }

    let context_shift = get_global_long("context-shift")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if context_shift {
        params.push("--context-shift".to_owned());
    }

    let sleep_idle_seconds = get_long_val("sleep-idle-seconds")
        .or_else(|| get_lh_val("sleep-idle-seconds"))
        .or_else(|| get_global_long("sleep-idle-seconds"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(secs) = sleep_idle_seconds {
        params.push("--sleep-idle-seconds".to_owned());
        params.push(secs.to_string());
    }

    let moe_cache_mib = get_lh_val("moe-cache-mib")
        .or_else(|| get_long_val("moe-cache-mib"))
        .or_else(|| get_global_long("moe-cache-mib"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_u64().or_else(|| {
                        #[allow(clippy::cast_sign_loss)]
                        v.as_i64().map(|i| i as u64)
                    })
                },
                |s| s.parse::<u64>().ok(),
            )
        });
    if let Some(mib) = moe_cache_mib {
        params.push("--moe-cache-mib".to_owned());
        params.push(mib.to_string());
    }

    let n_cpu_moe = get_lh_val("n-cpu-moe")
        .or_else(|| get_long_val("n-cpu-moe"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_u64().or_else(|| {
                        #[allow(clippy::cast_sign_loss)]
                        v.as_i64().map(|i| i as u64)
                    })
                },
                |s| s.parse::<u64>().ok(),
            )
        });
    if let Some(ncmoe) = n_cpu_moe {
        params.push("--n-cpu-moe".to_owned());
        params.push(ncmoe.to_string());
    }

    let numa = get_global_long("numa")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if numa != "none" && !numa.is_empty() {
        params.push("--numa".to_owned());
        params.push(numa.to_owned());
    }

    let split_mode = get_global_long("split-mode")
        .and_then(|v| v.as_str())
        .unwrap_or("layer");
    if !split_mode.is_empty() {
        params.push("--split-mode".to_owned());
        params.push(split_mode.to_owned());
    }

    let device = get_global_long("device")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if device != "none" && !device.is_empty() {
        params.push("--device".to_owned());
        params.push(device.to_owned());
    }

    let api_key_file = get_global_long("api-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if api_key_file != "none" && !api_key_file.is_empty() {
        params.push("--api-key-file".to_owned());
        params.push(api_key_file.to_owned());
    }

    let ssl_key_file = get_global_long("ssl-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_key_file != "none" && !ssl_key_file.is_empty() {
        params.push("--ssl-key-file".to_owned());
        params.push(ssl_key_file.to_owned());
    }

    let ssl_cert_file = get_global_long("ssl-cert-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_cert_file != "none" && !ssl_cert_file.is_empty() {
        params.push("--ssl-cert-file".to_owned());
        params.push(ssl_cert_file.to_owned());
    }

    // New Model-specific settings
    let min_p = get_lh_val("min-p")
        .or_else(|| get_long_val("min-p"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_f64().or_else(|| {
                        #[allow(clippy::cast_precision_loss)]
                        v.as_i64().map(|i| i as f64)
                    })
                },
                |s| s.parse::<f64>().ok(),
            )
        });
    if let Some(mp) = min_p {
        params.push("--min-p".to_owned());
        params.push(mp.to_string());
    }

    let repeat_penalty = get_lh_val("repeat-penalty")
        .or_else(|| get_long_val("repeat-penalty"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_f64().or_else(|| {
                        #[allow(clippy::cast_precision_loss)]
                        v.as_i64().map(|i| i as f64)
                    })
                },
                |s| s.parse::<f64>().ok(),
            )
        });
    if let Some(rp) = repeat_penalty {
        params.push("--repeat-penalty".to_owned());
        params.push(rp.to_string());
    }

    let repeat_last_n = get_lh_val("repeat-last-n")
        .or_else(|| get_long_val("repeat-last-n"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(rln) = repeat_last_n {
        params.push("--repeat-last-n".to_owned());
        params.push(rln.to_string());
    }

    let dry_multiplier = get_lh_val("dry-multiplier")
        .or_else(|| get_long_val("dry-multiplier"))
        .or_else(|| get_global_long("dry-multiplier"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_f64().or_else(|| {
                        #[allow(clippy::cast_precision_loss)]
                        v.as_i64().map(|i| i as f64)
                    })
                },
                |s| s.parse::<f64>().ok(),
            )
        });
    if let Some(dm) = dry_multiplier {
        params.push("--dry-multiplier".to_owned());
        params.push(dm.to_string());
    }

    let dry_base = get_lh_val("dry-base")
        .or_else(|| get_long_val("dry-base"))
        .or_else(|| get_global_long("dry-base"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    v.as_f64().or_else(|| {
                        #[allow(clippy::cast_precision_loss)]
                        v.as_i64().map(|i| i as f64)
                    })
                },
                |s| s.parse::<f64>().ok(),
            )
        });
    if let Some(db) = dry_base {
        params.push("--dry-base".to_owned());
        params.push(db.to_string());
    }

    let dry_allowed_length = get_lh_val("dry-allowed-length")
        .or_else(|| get_long_val("dry-allowed-length"))
        .or_else(|| get_global_long("dry-allowed-length"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(dal) = dry_allowed_length {
        params.push("--dry-allowed-length".to_owned());
        params.push(dal.to_string());
    }

    let dry_penalty_last_n = get_lh_val("dry-penalty-last-n")
        .or_else(|| get_long_val("dry-penalty-last-n"))
        .or_else(|| get_global_long("dry-penalty-last-n"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(dpn) = dry_penalty_last_n {
        params.push("--dry-penalty-last-n".to_owned());
        params.push(dpn.to_string());
    }

    let dry_sequence_breaker = get_lh_val("dry-sequence-breaker")
        .or_else(|| get_long_val("dry-sequence-breaker"))
        .or_else(|| get_global_long("dry-sequence-breaker"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if let Some(dsb) = dry_sequence_breaker {
        params.push("--dry-sequence-breaker".to_owned());
        params.push(dsb);
    }

    let reasoning = get_lh_val("reasoning")
        .or_else(|| get_long_val("reasoning"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if let Some(r) = reasoning {
        params.push("--reasoning".to_owned());
        params.push(r);
    }

    let reasoning_format = get_lh_val("reasoning-format")
        .or_else(|| get_long_val("reasoning-format"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if let Some(rf) = reasoning_format {
        params.push("--reasoning-format".to_owned());
        params.push(rf);
    }

    let reasoning_budget = get_lh_val("reasoning-budget")
        .or_else(|| get_long_val("reasoning-budget"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(rb) = reasoning_budget {
        params.push("--reasoning-budget".to_owned());
        params.push(rb.to_string());
    }

    let reasoning_effort = get_lh_val("reasoning-effort")
        .or_else(|| get_long_val("reasoning-effort"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if let Some(re) = reasoning_effort {
        params.push("--reasoning-effort".to_owned());
        params.push(re);
    }

    if let Some(rp_val) =
        get_lh_val("reasoning-preserve").or_else(|| get_long_val("reasoning-preserve"))
    {
        if let Some(b) = rp_val.as_bool() {
            params.push(if b {
                "--reasoning-preserve".to_owned()
            } else {
                "--no-reasoning-preserve".to_owned()
            });
        } else if let Some(s) = rp_val.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            match s.to_lowercase().as_str() {
                "true" | "on" | "1" => params.push("--reasoning-preserve".to_owned()),
                "false" | "off" | "0" => params.push("--no-reasoning-preserve".to_owned()),
                _ => {}
            }
        }
    }

    let config_ts = get_lh_val("tensor-split")
        .or_else(|| get_long_val("tensor-split"))
        .or_else(|| get_global_long("tensor-split"))
        .and_then(|v| v.as_str().map(|s| s.trim().to_owned()))
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"));

    let tensor_split = config_ts.or_else(|| {
        let gpus = crate::discovery::scan_gpu_topology();
        crate::discovery::calculate_tensor_split(&gpus, 0.12)
    });

    if let Some(ref ts) = tensor_split {
        if !ts.is_empty() && ts != "none" {
            params.push("--tensor-split".to_owned());
            params.push(ts.clone());

            let fit_val = get_lh_val("fit")
                .or_else(|| get_long_val("fit"))
                .or_else(|| get_global_long("fit"))
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
                .or_else(|| get_global_long("fitt"))
                .and_then(|v| {
                    v.as_str()
                        .map_or_else(|| v.as_i64().map(|i| i.to_string()), |s| Some(s.to_owned()))
                })
                .unwrap_or_else(|| "1024".to_owned());

            params.push("-fit".to_owned());
            params.push(fit_val);

            params.push("-fitt".to_owned());
            params.push(fitt_target);
        }
    } else if let Some(v) = get_lh_val("fit")
        .or_else(|| get_long_val("fit"))
        .or_else(|| get_global_long("fit"))
    {
        let fit_val = v
            .as_str()
            .map_or_else(
                || {
                    v.as_bool()
                        .map(|b| if b { "on".to_owned() } else { "off".to_owned() })
                },
                |s| Some(s.to_owned()),
            )
            .unwrap_or_else(|| "on".to_owned());

        params.push("-fit".to_owned());
        params.push(fit_val);

        let fitt_target = get_lh_val("fitt")
            .or_else(|| get_long_val("fitt"))
            .or_else(|| get_global_long("fitt"))
            .and_then(|v| {
                v.as_str()
                    .map_or_else(|| v.as_i64().map(|i| i.to_string()), |s| Some(s.to_owned()))
            })
            .unwrap_or_else(|| "1024".to_owned());

        params.push("-fitt".to_owned());
        params.push(fitt_target);
    }

    params
}

/// Builds command-line parameters to launch llama-server in Router Mode.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_router_launch_parameters<S: std::hash::BuildHasher>(
    executable_path: &Path,
    preset_path: &Path,
    global_config: &HashMap<String, serde_json::Value, S>,
    resolved_port: u16,
) -> Vec<String> {
    let mut params = Vec::new();
    params.push(executable_path.to_string_lossy().into_owned());

    params.push("--models-preset".to_owned());
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
    params.push("--host".to_owned());
    params.push(host.to_owned());

    params.push("--port".to_owned());
    params.push(resolved_port.to_string());

    params.push("--log-colors".to_owned());
    params.push("on".to_owned());

    let flash_attn = get_global_long("flash-attn")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    params.push("--flash-attn".to_owned());
    params.push(flash_attn.to_owned());

    let cache_ram = get_global_long("cache-ram")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_owned());
    params.push("--cache-ram".to_owned());
    params.push(cache_ram);

    let models_max = get_global_lh("models-max")
        .and_then(|v| {
            v.as_i64().map_or_else(
                || v.as_str().map(ToOwned::to_owned),
                |i| Some(i.to_string()),
            )
        })
        .unwrap_or_else(|| "1".to_owned());
    params.push("--models-max".to_owned());
    params.push(models_max);

    let models_memory_margin = get_global_long("models-memory-margin")
        .or_else(|| get_global_lh("models-memory-margin"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(margin) = models_memory_margin
        && margin > 0
    {
        params.push("--models-memory-margin".to_owned());
        params.push(margin.to_string());
    }

    let np = get_global_long("np")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_owned());
    params.push("-np".to_owned());
    params.push(np);

    let threads = get_global_long("threads")
        .and_then(|v| v.as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "-1".to_owned());
    params.push("-t".to_owned());
    params.push(threads);

    params.push("--props".to_owned());

    if let Some(bs) = get_global_long("batch-size").and_then(serde_json::Value::as_i64) {
        params.push("-b".to_owned());
        params.push(bs.to_string());
    }
    if let Some(ubs) = get_global_long("ubatch-size").and_then(serde_json::Value::as_i64) {
        params.push("-ub".to_owned());
        params.push(ubs.to_string());
    }
    if let Some(tools) = get_global_long("tools").and_then(|v| v.as_str()) {
        params.push("--tools".to_owned());
        params.push(tools.to_owned());
    }

    let ui_enabled = get_global_lh("ui")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !ui_enabled {
        params.push("--no-ui".to_owned());
    }

    let kv_unified = get_global_long("kv-unified")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if kv_unified {
        params.push("--kv-unified".to_owned());
    }

    let api_key = get_global_long("api-key")
        .and_then(|v| v.as_str())
        .unwrap_or("disabled");
    if api_key != "disabled" && !api_key.is_empty() {
        params.push("--api-key".to_owned());
        params.push(api_key.to_owned());
    }

    let metrics = get_global_long("metrics")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if metrics {
        params.push("--metrics".to_owned());
    }

    let log_verbosity = get_global_long("log-verbosity")
        .and_then(|v| {
            v.as_i64().map_or_else(
                || v.as_str().map(ToOwned::to_owned),
                |i| Some(i.to_string()),
            )
        })
        .unwrap_or_else(|| "3".to_owned());
    params.push("--log-verbosity".to_owned());
    params.push(log_verbosity);

    // New Global settings
    let cache_prompt = get_global_long("cache-prompt")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !cache_prompt {
        params.push("--no-cache-prompt".to_owned());
    }

    let context_shift = get_global_long("context-shift")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if context_shift {
        params.push("--context-shift".to_owned());
    }

    let sleep_idle_seconds = get_global_long("sleep-idle-seconds")
        .or_else(|| get_global_lh("sleep-idle-seconds"))
        .and_then(|v| {
            v.as_str().map_or_else(
                || {
                    #[allow(clippy::cast_possible_wrap)]
                    v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
                },
                |s| s.parse::<i64>().ok(),
            )
        });
    if let Some(secs) = sleep_idle_seconds {
        params.push("--sleep-idle-seconds".to_owned());
        params.push(secs.to_string());
    }

    let explicit_load_mode = get_global_long("load-mode").and_then(|v| v.as_str());

    let mlock = get_global_long("mlock")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let no_mmap = get_global_long("no-mmap")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    if let Some(load_mode) = resolve_load_mode(explicit_load_mode, mlock, no_mmap) {
        params.push("--load-mode".to_owned());
        params.push(load_mode);
    }

    let moe_cache_mib = get_global_long("moe-cache-mib").and_then(|v| {
        v.as_str().map_or_else(
            || {
                v.as_u64().or_else(|| {
                    #[allow(clippy::cast_sign_loss)]
                    v.as_i64().map(|i| i as u64)
                })
            },
            |s| s.parse::<u64>().ok(),
        )
    });
    if let Some(mib) = moe_cache_mib {
        params.push("--moe-cache-mib".to_owned());
        params.push(mib.to_string());
    }

    let numa = get_global_long("numa")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if numa != "none" && !numa.is_empty() {
        params.push("--numa".to_owned());
        params.push(numa.to_owned());
    }

    let split_mode = get_global_long("split-mode")
        .and_then(|v| v.as_str())
        .unwrap_or("layer");
    if !split_mode.is_empty() {
        params.push("--split-mode".to_owned());
        params.push(split_mode.to_owned());
    }

    let device = get_global_long("device")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if device != "none" && !device.is_empty() {
        params.push("--device".to_owned());
        params.push(device.to_owned());
    }

    let api_key_file = get_global_long("api-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if api_key_file != "none" && !api_key_file.is_empty() {
        params.push("--api-key-file".to_owned());
        params.push(api_key_file.to_owned());
    }

    let ssl_key_file = get_global_long("ssl-key-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_key_file != "none" && !ssl_key_file.is_empty() {
        params.push("--ssl-key-file".to_owned());
        params.push(ssl_key_file.to_owned());
    }

    let ssl_cert_file = get_global_long("ssl-cert-file")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if ssl_cert_file != "none" && !ssl_cert_file.is_empty() {
        params.push("--ssl-cert-file".to_owned());
        params.push(ssl_cert_file.to_owned());
    }

    let dry_multiplier = get_global_long("dry-multiplier").and_then(|v| {
        v.as_str().map_or_else(
            || {
                v.as_f64().or_else(|| {
                    #[allow(clippy::cast_precision_loss)]
                    v.as_i64().map(|i| i as f64)
                })
            },
            |s| s.parse::<f64>().ok(),
        )
    });
    if let Some(dm) = dry_multiplier {
        params.push("--dry-multiplier".to_owned());
        params.push(dm.to_string());
    }

    let dry_base = get_global_long("dry-base").and_then(|v| {
        v.as_str().map_or_else(
            || {
                v.as_f64().or_else(|| {
                    #[allow(clippy::cast_precision_loss)]
                    v.as_i64().map(|i| i as f64)
                })
            },
            |s| s.parse::<f64>().ok(),
        )
    });
    if let Some(db) = dry_base {
        params.push("--dry-base".to_owned());
        params.push(db.to_string());
    }

    let dry_allowed_length = get_global_long("dry-allowed-length").and_then(|v| {
        v.as_str().map_or_else(
            || {
                #[allow(clippy::cast_possible_wrap)]
                v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
            },
            |s| s.parse::<i64>().ok(),
        )
    });
    if let Some(dal) = dry_allowed_length {
        params.push("--dry-allowed-length".to_owned());
        params.push(dal.to_string());
    }

    let dry_penalty_last_n = get_global_long("dry-penalty-last-n").and_then(|v| {
        v.as_str().map_or_else(
            || {
                #[allow(clippy::cast_possible_wrap)]
                v.as_u64().map(|n| n as i64).or_else(|| v.as_i64())
            },
            |s| s.parse::<i64>().ok(),
        )
    });
    if let Some(dpn) = dry_penalty_last_n {
        params.push("--dry-penalty-last-n".to_owned());
        params.push(dpn.to_string());
    }

    let dry_sequence_breaker = get_global_long("dry-sequence-breaker")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if let Some(dsb) = dry_sequence_breaker {
        params.push("--dry-sequence-breaker".to_owned());
        params.push(dsb);
    }

    let config_ts = get_global_lh("tensor-split")
        .or_else(|| get_global_long("tensor-split"))
        .and_then(|v| v.as_str().map(|s| s.trim().to_owned()))
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"));

    let tensor_split = config_ts.or_else(|| {
        let gpus = crate::discovery::scan_gpu_topology();
        crate::discovery::calculate_tensor_split(&gpus, 0.12)
    });

    if let Some(ref ts) = tensor_split {
        if !ts.is_empty() && ts != "none" {
            params.push("--tensor-split".to_owned());
            params.push(ts.clone());

            let fit_val = get_global_lh("fit")
                .or_else(|| get_global_long("fit"))
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

            let fitt_target = get_global_lh("fitt")
                .or_else(|| get_global_long("fitt"))
                .and_then(|v| {
                    v.as_str()
                        .map_or_else(|| v.as_i64().map(|i| i.to_string()), |s| Some(s.to_owned()))
                })
                .unwrap_or_else(|| "1024".to_owned());

            params.push("-fit".to_owned());
            params.push(fit_val);

            params.push("-fitt".to_owned());
            params.push(fitt_target);
        }
    } else if let Some(v) = get_global_lh("fit").or_else(|| get_global_long("fit")) {
        let fit_val = v
            .as_str()
            .map_or_else(
                || {
                    v.as_bool()
                        .map(|b| if b { "on".to_owned() } else { "off".to_owned() })
                },
                |s| Some(s.to_owned()),
            )
            .unwrap_or_else(|| "on".to_owned());

        params.push("-fit".to_owned());
        params.push(fit_val);

        let fitt_target = get_global_lh("fitt")
            .or_else(|| get_global_long("fitt"))
            .and_then(|v| {
                v.as_str()
                    .map_or_else(|| v.as_i64().map(|i| i.to_string()), |s| Some(s.to_owned()))
            })
            .unwrap_or_else(|| "1024".to_owned());

        params.push("-fitt".to_owned());
        params.push(fitt_target);
    }

    params
}

/// Checks if a TCP port is available to bind locally.
#[must_use]
pub fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Resolves the port to use, supporting "auto" binding or fallback probing (+10 ports).
///
/// # Errors
///
/// Returns an error if the port string is invalid or if the requested port (and retries) are already in use.
pub fn resolve_port(port_str: &str) -> Result<u16, std::io::Error> {
    if port_str == "auto" {
        let mut port = 9931;
        while port < 65535 {
            if is_port_available(port) {
                return Ok(port);
            }
            port += 1;
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "No available ports found in range 9931-65535".to_owned(),
        ))
    } else {
        let parsed: u16 = port_str.parse().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to parse port: {e}"),
            )
        })?;

        if is_port_available(parsed) {
            Ok(parsed)
        } else {
            let max_limit = parsed.saturating_add(10);
            let mut port = parsed.saturating_add(1);
            while port <= max_limit && port < 65535 {
                if is_port_available(port) {
                    return Ok(port);
                }
                port += 1;
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!(
                    "Requested port {parsed} and its subsequent retries (+10) are all occupied."
                ),
            ))
        }
    }
}

/// Resolves the effective `--load-mode` value for `llama-server`.
///
/// Upstream `llama.cpp` deprecated and removed `--mlock`, `--mmap`, and `--no-mmap`
/// in favor of `-lm, --load-mode <auto|none|mmap|mlock|mmap+mlock|dio>`.
/// This helper respects explicit `load-mode` settings and translates legacy `mlock`
/// and `no-mmap` settings safely without passing invalid flags.
#[must_use]
pub fn resolve_load_mode(
    explicit_load_mode: Option<&str>,
    mlock: bool,
    no_mmap: bool,
) -> Option<String> {
    if let Some(mode) = explicit_load_mode {
        let trimmed = mode.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    match (mlock, no_mmap) {
        (true, _) => Some("mlock".to_owned()),
        (false, true) => Some("none".to_owned()),
        (false, false) => None,
    }
}
