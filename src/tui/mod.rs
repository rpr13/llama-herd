#![allow(
    clippy::too_many_lines,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::assigning_clones,
    clippy::missing_errors_doc
)]
/// Application state and logic.
pub mod app;
/// Server logging and process management.
pub mod logs;
/// File and directory picker components.
pub mod picker;
/// Application themes and colors.
pub mod theme;
/// User interface layout drawing.
pub mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub use app::{AppScreen, AppState, DashboardFocus};
pub use logs::ActiveServer;

/// Events handled by the TUI event loop.
#[derive(Clone, Debug)]
pub enum TuiEvent {
    /// Keyboard input event.
    Input(KeyEvent),
    /// Periodic tick event for background processes.
    Tick,
    /// Log line received event.
    LogReceived,
    /// Models directory state changed.
    ModelsDirChanged(app::ModelsDirState),
    /// Models directory has become invalid / inaccessible.
    ModelsDirInvalid,
}

/// Handles TUI keyboard events and updates the application state.
#[allow(
    clippy::too_many_lines,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::assigning_clones
)]
pub fn handle_key_event(
    state: &mut AppState,
    key: KeyEvent,
    event_tx: &std::sync::mpsc::Sender<TuiEvent>,
) -> bool {
    let mut should_quit = false;

    match key.code {
        KeyCode::F(1) => {
            state.active_tab = 0;
            state.screen = AppScreen::Dashboard;
            return false;
        }
        KeyCode::F(2) => {
            state.active_tab = 1;
            state.screen = AppScreen::Settings;
            return false;
        }
        KeyCode::F(3) => {
            state.active_tab = 2;
            state.screen = AppScreen::Logs;
            return false;
        }
        KeyCode::F(4) => {
            if let Some(ref server) = state.active_server {
                let active_port = server.metrics.lock().ok().and_then(|m| m.active_port);
                if let Some(port) = active_port {
                    let host = state
                        .global_config
                        .get("host")
                        .and_then(|v| v.as_str())
                        .unwrap_or("127.0.0.1")
                        .to_owned();

                    std::thread::spawn(move || {
                        let _ = crate::control::cancel_active_generation(&host, port);
                    });

                    let msg =
                        "[CONTROL] F4 pressed: Interrupt signal sent to llama-server...".to_owned();
                    if let Ok(mut hist) = server.raw_history.lock() {
                        hist.push_back(msg.clone());
                    }
                    if let Ok(mut logs) = server.logs.lock() {
                        logs.push_back(logs::parse_ansi_line(&msg));
                    }
                }
            }
            return false;
        }
        _ => {}
    }

    match state.screen {
        AppScreen::Dashboard => match key.code {
            KeyCode::Char('q') => {
                should_quit = true;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if state.has_unsaved_changes() {
                    state.screen = AppScreen::ConfirmSaveConfig;
                    state.backup_config = true;
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                state.dashboard_focus = match state.dashboard_focus {
                    DashboardFocus::Left => DashboardFocus::Right,
                    DashboardFocus::Right => DashboardFocus::Left,
                };
            }
            KeyCode::Up => {
                if state.dashboard_focus == DashboardFocus::Right {
                    if state.dashboard_param_index == 0 {
                        state.dashboard_param_index = 23;
                    } else {
                        state.dashboard_param_index -= 1;
                    }
                } else if !state.presets.is_empty() {
                    let target_index = if state.preset_index == 0 {
                        state.presets.len() - 1
                    } else {
                        state.preset_index - 1
                    };
                    if state.has_unsaved_changes() {
                        state.pending_preset_index = Some(target_index);
                        state.screen = AppScreen::WarnDiscardChanges;
                    } else {
                        state.preset_index = target_index;
                        state.load_current_preset_settings(None);
                    }
                }
            }
            KeyCode::Down => {
                if state.dashboard_focus == DashboardFocus::Right {
                    state.dashboard_param_index = (state.dashboard_param_index + 1) % 24;
                } else if !state.presets.is_empty() {
                    let target_index = (state.preset_index + 1) % state.presets.len();
                    if state.has_unsaved_changes() {
                        state.pending_preset_index = Some(target_index);
                        state.screen = AppScreen::WarnDiscardChanges;
                    } else {
                        state.preset_index = target_index;
                        state.load_current_preset_settings(None);
                    }
                }
            }
            KeyCode::F(6) => {
                // Spawns router mode server
                crate::launcher::kill_existing_servers();
                let preset_ini_path = match crate::discovery::generate_presets_ini(
                    &state.models_dir,
                    &state.preset_path,
                    &state.global_config,
                ) {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                let port_str = state
                    .global_config
                    .get("port")
                    .and_then(|v| {
                        if let Some(i) = v.as_i64() {
                            Some(i.to_string())
                        } else {
                            v.as_str().map(ToOwned::to_owned)
                        }
                    })
                    .unwrap_or_else(|| "auto".to_owned());
                let resolved_port = match crate::launcher::resolve_port(&port_str) {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                let launch_args = crate::launcher::build_router_launch_parameters(
                    &state.server_exe,
                    &preset_ini_path,
                    &state.global_config,
                    resolved_port,
                );
                state.last_launch_args = launch_args;
                state.is_router_mode = true;
                let model_name = if state.presets.is_empty() {
                    None
                } else {
                    Some(state.presets[state.preset_index].0.clone())
                };

                match ActiveServer::spawn(
                    &state.last_launch_args,
                    &state.models_dir,
                    model_name,
                    Some(event_tx.clone()),
                ) {
                    Ok(server) => {
                        state.active_server = Some(server);
                        state.screen = AppScreen::Logs;
                        state.active_tab = 2;
                        state.logs_paused = false;
                        state.paused_logs_buffer.clear();
                        state.auto_scroll = true;
                        state.log_scroll_offset = 0;
                        state.log_scroll_x = 0;
                    }
                    Err(_e) => {}
                }
            }
            KeyCode::F(5) => {
                if !state.presets.is_empty() {
                    // Spawns preset server
                    crate::launcher::kill_existing_servers();
                    let (_preset_name, model_path) = &state.presets[state.preset_index];
                    let assets = crate::discovery::discover_assets(model_path, &state.models_dir);
                    let settings = state.get_user_settings();
                    let port_str = state
                        .global_config
                        .get("port")
                        .and_then(|v| {
                            if let Some(i) = v.as_i64() {
                                Some(i.to_string())
                            } else {
                                v.as_str().map(ToOwned::to_owned)
                            }
                        })
                        .unwrap_or_else(|| "auto".to_owned());
                    let resolved_port = match crate::launcher::resolve_port(&port_str) {
                        Ok(p) => p,
                        Err(_) => return false,
                    };
                    let launch_args = crate::launcher::build_launch_parameters(
                        &state.server_exe,
                        model_path,
                        &assets,
                        &settings,
                        &state.global_config,
                        resolved_port,
                    );
                    state.last_launch_args = launch_args;
                    state.is_router_mode = false;
                    let model_name = if state.presets.is_empty() {
                        None
                    } else {
                        Some(state.presets[state.preset_index].0.clone())
                    };

                    match ActiveServer::spawn(
                        &state.last_launch_args,
                        &state.models_dir,
                        model_name,
                        Some(event_tx.clone()),
                    ) {
                        Ok(server) => {
                            state.active_server = Some(server);
                            state.screen = AppScreen::Logs;
                            state.active_tab = 2;
                            state.logs_paused = false;
                            state.paused_logs_buffer.clear();
                            state.auto_scroll = true;
                            state.log_scroll_offset = 0;
                            state.log_scroll_x = 0;
                        }
                        Err(_e) => {}
                    }
                }
            }
            KeyCode::Enter if state.dashboard_focus == DashboardFocus::Right => {
                match state.dashboard_param_index {
                    0 => {
                        state.screen = AppScreen::EditingConfigFileName;
                        state.input_buffer = state.config_file_name.clone();
                        if state.presets.is_empty() {
                            state.similar_config_files.clear();
                            state.similar_config_index = None;
                        } else {
                            let (_, model_path) = &state.presets[state.preset_index];
                            state.similar_config_files = crate::config::find_similar_config_files(
                                model_path,
                                &state.models_dir,
                            );
                            state.similar_config_index = state
                                .similar_config_files
                                .iter()
                                .position(|f| f == &state.input_buffer);
                        }
                    }
                    1 => {
                        state.screen = AppScreen::EditingTotalLayers;
                        state.input_buffer = state
                            .total_layers
                            .map(|l| l.to_string())
                            .unwrap_or_default();
                    }
                    2 => {
                        state.screen = AppScreen::EditingCtx;
                        state.input_buffer = if state.ctx_str.is_empty() {
                            state.ctx.to_string()
                        } else {
                            state.ctx_str.clone()
                        };
                    }
                    3 => {
                        state.screen = AppScreen::EditingNgl;
                        state.input_buffer = state.ngl.clone();
                    }
                    4 => {
                        state.mmproj_index_backup = state.mmproj_index;
                        state.screen = AppScreen::SelectingMMProj;
                    }
                    5 => {
                        state.draft_index_backup = state.draft_index;
                        state.screen = AppScreen::SelectingDraftModel;
                    }
                    6 => {
                        state.screen = AppScreen::EditingDraftNgl;
                        state.input_buffer = state.draft_ngl.clone();
                    }
                    7 => {
                        state.spec_type_backup = state.spec_type_index;
                        state.screen = AppScreen::SelectingSpecType;
                    }
                    8 => {
                        state.screen = AppScreen::EditingSpecDraftNMax;
                        state.input_buffer = state.spec_draft_n_max.clone();
                    }
                    9 => {
                        state.screen = AppScreen::EditingSpecDraftPMin;
                        state.input_buffer = state.spec_draft_p_min.clone();
                    }
                    10 => {
                        state.screen = AppScreen::EditingTemp;
                        state.input_buffer = state.temp.clone();
                    }
                    11 => {
                        state.screen = AppScreen::EditingTopP;
                        state.input_buffer = state.top_p.clone();
                    }
                    12 => {
                        state.screen = AppScreen::EditingTopK;
                        state.input_buffer = state.top_k.clone();
                    }
                    13 => {
                        state.screen = AppScreen::EditingMinP;
                        state.input_buffer = state.min_p.clone();
                    }
                    14 => {
                        state.screen = AppScreen::EditingRepeatPenalty;
                        state.input_buffer = state.repeat_penalty.clone();
                    }
                    15 => {
                        state.screen = AppScreen::EditingRepeatLastN;
                        state.input_buffer = state.repeat_last_n.clone();
                    }
                    16 => {
                        state.screen = AppScreen::EditingDryMultiplier;
                        state.input_buffer = state.dry_multiplier.clone();
                    }
                    17 => {
                        state.screen = AppScreen::EditingDryBase;
                        state.input_buffer = state.dry_base.clone();
                    }
                    18 => {
                        state.screen = AppScreen::EditingDryAllowedLength;
                        state.input_buffer = state.dry_allowed_length.clone();
                    }
                    19 => {
                        state.screen = AppScreen::EditingDryPenaltyLastN;
                        state.input_buffer = state.dry_penalty_last_n.clone();
                    }
                    20 => {
                        state.screen = AppScreen::EditingDrySequenceBreaker;
                        state.input_buffer = state.dry_sequence_breaker.clone();
                    }
                    21 => {
                        state.reasoning_format_index_backup = state.reasoning_format_index;
                        state.screen = AppScreen::SelectingReasoningFormat;
                    }
                    22 => {
                        state.reasoning_index_backup = state.reasoning_index;
                        state.screen = AppScreen::SelectingReasoning;
                    }
                    23 => {
                        state.screen = AppScreen::EditingReasoningBudget;
                        state.input_buffer = state.reasoning_budget.clone();
                    }
                    _ => {}
                }
            }
            _ => {}
        },
        AppScreen::Settings => match key.code {
            KeyCode::Up => {
                if state.settings_index == 0 {
                    state.settings_index = ui::SETTINGS.len() - 1;
                } else {
                    state.settings_index -= 1;
                }
            }
            KeyCode::Down => {
                state.settings_index = (state.settings_index + 1) % ui::SETTINGS.len();
            }
            KeyCode::Enter => {
                let selected_item = &ui::SETTINGS[state.settings_index];
                match selected_item.key {
                    "llama-server" => {
                        state.screen = AppScreen::PickingServerPath;
                        let initial_path = if state.server_exe.as_os_str().is_empty() {
                            crate::config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
                        } else {
                            state
                                .server_exe
                                .parent()
                                .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf)
                        };
                        state.picker = Some(picker::FilePicker::new(
                            initial_path,
                            picker::PickerMode::File,
                        ));
                    }
                    "models-dir" => {
                        state.screen = AppScreen::PickingModelsDir;
                        let initial_path = if state.models_dir.as_os_str().is_empty() {
                            crate::config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
                        } else {
                            state.models_dir.clone()
                        };
                        state.picker = Some(picker::FilePicker::new(
                            initial_path,
                            picker::PickerMode::Directory,
                        ));
                    }
                    "flash-attn" | "cache-type-k" | "cache-type-v" | "log-verbosity" | "numa"
                    | "split-mode" => {
                        // Option selectors for flash-attn, cache-type-k, cache-type-v, log-verbosity, numa, split-mode
                        let option_list = match selected_item.key {
                            "flash-attn" => vec![
                                "auto".to_owned(),
                                "1".to_owned(),
                                "0".to_owned(),
                                "(Custom / Manual...)".to_owned(),
                            ],
                            "log-verbosity" => vec![
                                "0".to_owned(),
                                "1".to_owned(),
                                "2".to_owned(),
                                "3".to_owned(),
                                "4".to_owned(),
                                "5".to_owned(),
                                "(Custom / Manual...)".to_owned(),
                            ],
                            "numa" => vec![
                                "none".to_owned(),
                                "distribute".to_owned(),
                                "isolate".to_owned(),
                                "numactl".to_owned(),
                                "(Custom / Manual...)".to_owned(),
                            ],
                            "split-mode" => vec![
                                "layer".to_owned(),
                                "none".to_owned(),
                                "row".to_owned(),
                                "tensor".to_owned(),
                                "(Custom / Manual...)".to_owned(),
                            ],
                            _ => vec![
                                "f16".to_owned(),
                                "q8_0".to_owned(),
                                "q4_0".to_owned(),
                                "q4_1".to_owned(),
                                "iq4_nl".to_owned(),
                                "q5_0".to_owned(),
                                "q5_1".to_owned(),
                                "f32".to_owned(),
                                "bf16".to_owned(),
                                "(Custom / Manual...)".to_owned(),
                            ],
                        };
                        let val_str = crate::config::get_global_config_string(
                            &state.global_config,
                            selected_item.key,
                            selected_item.default_val,
                        );
                        let mut selected_idx = 0;
                        for (idx, opt) in option_list.iter().enumerate() {
                            if opt == &val_str {
                                selected_idx = idx;
                                break;
                            }
                        }
                        if selected_idx == 0 && val_str != option_list[0] {
                            selected_idx = option_list.len() - 1;
                        }
                        state.option_selector_index = selected_idx;
                        state.option_selector_list = option_list;
                        state.screen = AppScreen::SelectingGlobalSettingOption;
                    }
                    "kv-unified" | "metrics" | "ui" | "no-mmap" | "cache-prompt"
                    | "context-shift" | "mlock" => {
                        // Toggle boolean flags
                        let default_val =
                            matches!(selected_item.key, "kv-unified" | "ui" | "cache-prompt");
                        let current_val = state
                            .global_config
                            .get("llama-server-long")
                            .and_then(|l| l.get(selected_item.key))
                            .or_else(|| {
                                state
                                    .global_config
                                    .get("llama-herd")
                                    .and_then(|lh| lh.get(selected_item.key))
                            })
                            .or_else(|| state.global_config.get(selected_item.key))
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(default_val);
                        let next_val = !current_val;
                        if next_val == default_val {
                            crate::config::remove_global_config_value(
                                &mut state.global_config,
                                selected_item.key,
                            );
                        } else {
                            crate::config::update_global_config_value(
                                &mut state.global_config,
                                selected_item.key,
                                serde_json::Value::Bool(next_val),
                            );
                        }
                        let _ =
                            crate::config::save_config(&state.config_path, &state.global_config);
                    }
                    _ => {
                        let val_str = crate::config::get_global_config_string(
                            &state.global_config,
                            selected_item.key,
                            selected_item.default_val,
                        );
                        state.screen = AppScreen::EditingGlobalSetting;
                        state.input_buffer = val_str;
                    }
                }
            }
            KeyCode::Char('q') => {
                should_quit = true;
            }
            _ => {}
        },
        AppScreen::PickingServerPath | AppScreen::PickingModelsDir => {
            if let Some(picker) = &mut state.picker {
                if let Some(path) = picker.handle_event(key) {
                    if state.screen == AppScreen::PickingServerPath {
                        state.server_exe = path.clone();
                        state.server_version = crate::launcher::get_server_version(&path);
                        crate::config::update_global_config_value(
                            &mut state.global_config,
                            "llama-server",
                            serde_json::Value::String(path.to_string_lossy().to_string()),
                        );
                    } else {
                        state.models_dir = path.clone();
                        if let Ok(mut lock) = state.shared_models_dir.lock() {
                            *lock = path.clone();
                        }
                        crate::config::update_global_config_value(
                            &mut state.global_config,
                            "models-dir",
                            serde_json::Value::String(path.to_string_lossy().to_string()),
                        );
                        // Refresh presets list when models dir changes
                        let _ = crate::discovery::generate_presets_ini(
                            &state.models_dir,
                            &state.preset_path,
                            &state.global_config,
                        );
                        state.presets =
                            crate::discovery::discover_presets_from_ini(&state.preset_path);
                        state.preset_index = 0;
                        let new_state = app::get_models_dir_state(&state.models_dir);
                        state.last_models_dir_state = new_state.clone();
                        state.last_stable_models_dir_state = new_state;
                        state.load_current_preset_settings(None);
                    }

                    // Save config
                    let _ = crate::config::save_config(&state.config_path, &state.global_config);

                    state.screen = AppScreen::Settings;
                    state.picker = None;
                } else if key.code == KeyCode::Esc {
                    state.screen = AppScreen::Settings;
                    state.picker = None;
                }
            }
        }
        AppScreen::EditingCtx
        | AppScreen::EditingNgl
        | AppScreen::EditingDraftNgl
        | AppScreen::EditingTemp
        | AppScreen::EditingTopP
        | AppScreen::EditingTopK
        | AppScreen::EditingTotalLayers
        | AppScreen::EditingConfigFileName
        | AppScreen::EditingGlobalSetting
        | AppScreen::EditingMinP
        | AppScreen::EditingRepeatPenalty
        | AppScreen::EditingRepeatLastN
        | AppScreen::EditingReasoningBudget
        | AppScreen::EditingDryMultiplier
        | AppScreen::EditingDryBase
        | AppScreen::EditingDryAllowedLength
        | AppScreen::EditingDryPenaltyLastN
        | AppScreen::EditingDrySequenceBreaker
        | AppScreen::EditingSpecDraftNMax
        | AppScreen::EditingSpecDraftPMin => match key.code {
            KeyCode::Esc => {
                if state.screen == AppScreen::EditingGlobalSetting {
                    state.screen = AppScreen::Settings;
                } else {
                    state.screen = AppScreen::Dashboard;
                }
            }
            KeyCode::Enter => {
                match state.screen {
                    AppScreen::EditingCtx => {
                        let val = state.input_buffer.trim().to_owned();
                        match crate::config::parse_ctx_str(&val) {
                            Ok(parsed) => {
                                state.ctx_str = val;
                                state.ctx = parsed;
                                state.screen = AppScreen::Dashboard;
                            }
                            Err(_) => {
                                state.validation_error =
                                    Some("Invalid context size (e.g. 131072, 8k, 32k)".to_owned());
                            }
                        }
                    }
                    AppScreen::EditingNgl => {
                        let val = state.input_buffer.trim();
                        let is_valid = val.is_empty()
                            || val == "auto"
                            || val.parse::<usize>().is_ok()
                            || (val.starts_with("--") && val[2..].parse::<usize>().is_ok());
                        if is_valid {
                            state.ngl = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid layers count (e.g. auto, 32, --4)".to_owned());
                        }
                    }
                    AppScreen::EditingDraftNgl => {
                        let val = state.input_buffer.trim();
                        let is_valid = val.is_empty()
                            || val == "auto"
                            || val.parse::<usize>().is_ok()
                            || (val.starts_with("--") && val[2..].parse::<usize>().is_ok());
                        if is_valid {
                            state.draft_ngl = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid layers count (e.g. auto, 8, --1)".to_owned());
                        }
                    }
                    AppScreen::EditingTemp => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.temp = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid temperature (e.g. 0.8)".to_owned());
                        }
                    }
                    AppScreen::EditingTopP => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.top_p = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid Top P threshold (e.g. 0.95)".to_owned());
                        }
                    }
                    AppScreen::EditingTopK => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<i64>().is_ok() {
                            state.top_k = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid Top K count (e.g. 40)".to_owned());
                        }
                    }
                    AppScreen::EditingTotalLayers => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() {
                            state.total_layers = None;
                            state.screen = AppScreen::Dashboard;
                        } else if let Ok(num) = val.parse::<usize>() {
                            state.total_layers = Some(num);
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid total layers count (e.g. 33)".to_owned());
                        }
                    }
                    AppScreen::EditingConfigFileName => {
                        state.config_file_name = state.input_buffer.trim().to_owned();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingSpecDraftNMax => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<i64>().is_ok() {
                            state.spec_draft_n_max = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error = Some(
                                "Invalid speculative draft token predictions count (e.g. 4)"
                                    .to_owned(),
                            );
                        }
                    }
                    AppScreen::EditingSpecDraftPMin => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.spec_draft_p_min = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error = Some(
                                "Invalid minimum probability threshold (e.g. 0.85)".to_owned(),
                            );
                        }
                    }
                    AppScreen::EditingMinP => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.min_p = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid Min P threshold (e.g. 0.05)".to_owned());
                        }
                    }
                    AppScreen::EditingRepeatPenalty => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.repeat_penalty = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid repeat penalty (e.g. 1.1)".to_owned());
                        }
                    }
                    AppScreen::EditingRepeatLastN => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<i64>().is_ok() {
                            state.repeat_last_n = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid repeat last N count (e.g. 64)".to_owned());
                        }
                    }
                    AppScreen::EditingReasoningBudget => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<i64>().is_ok() {
                            state.reasoning_budget = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error = Some(
                                "Invalid token budget for thinking (e.g. -1, 1024)".to_owned(),
                            );
                        }
                    }
                    AppScreen::EditingDryMultiplier => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.dry_multiplier = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid DRY multiplier (e.g. 0.8)".to_owned());
                        }
                    }
                    AppScreen::EditingDryBase => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<f64>().is_ok() {
                            state.dry_base = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid DRY base (e.g. 1.75)".to_owned());
                        }
                    }
                    AppScreen::EditingDryAllowedLength => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<i64>().is_ok() {
                            state.dry_allowed_length = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid DRY allowed sequence length (e.g. 2)".to_owned());
                        }
                    }
                    AppScreen::EditingDryPenaltyLastN => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() || val.parse::<i64>().is_ok() {
                            state.dry_penalty_last_n = val.to_owned();
                            state.screen = AppScreen::Dashboard;
                        } else {
                            state.validation_error =
                                Some("Invalid DRY penalty last N count (e.g. -1)".to_owned());
                        }
                    }
                    AppScreen::EditingDrySequenceBreaker => {
                        state.dry_sequence_breaker = state.input_buffer.trim().to_owned();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingGlobalSetting => {
                        let val_str = state.input_buffer.trim().to_owned();
                        let selected_item = &ui::SETTINGS[state.settings_index];
                        let key_to_update = selected_item.key;

                        let is_valid = match selected_item.key {
                            "port" | "threads" => {
                                val_str == "auto" || val_str.parse::<i64>().is_ok()
                            }
                            "np"
                            | "batch-size"
                            | "ubatch-size"
                            | "models-max"
                            | "ctx-checkpoints"
                            | "checkpoint-min-step"
                            | "log-verbosity"
                            | "cache-ram"
                            | "dry-allowed-length"
                            | "dry-penalty-last-n"
                            | "spec-draft-n-max" => val_str.parse::<i64>().is_ok(),
                            "dry-multiplier" | "dry-base" | "spec-draft-p-min" => {
                                val_str.parse::<f64>().is_ok()
                            }
                            _ => true,
                        };

                        if !is_valid {
                            state.validation_error =
                                Some(format!("Invalid value for {}", selected_item.label));
                            return false;
                        }

                        if val_str == selected_item.default_val {
                            crate::config::remove_global_config_value(
                                &mut state.global_config,
                                key_to_update,
                            );
                        } else {
                            match selected_item.key {
                                "host"
                                | "flash-attn"
                                | "cache-type-k"
                                | "cache-type-v"
                                | "api-key"
                                | "device"
                                | "api-key-file"
                                | "ssl-key-file"
                                | "ssl-cert-file"
                                | "dry-sequence-breaker"
                                | "split-mode"
                                | "numa"
                                | "spec-type" => {
                                    crate::config::update_global_config_value(
                                        &mut state.global_config,
                                        key_to_update,
                                        serde_json::Value::String(val_str),
                                    );
                                }
                                "port" | "threads" => {
                                    let val = if val_str == "auto" {
                                        serde_json::Value::String(val_str)
                                    } else if let Ok(num) = val_str.parse::<i64>() {
                                        serde_json::Value::Number(num.into())
                                    } else {
                                        serde_json::Value::String(val_str)
                                    };
                                    crate::config::update_global_config_value(
                                        &mut state.global_config,
                                        key_to_update,
                                        val,
                                    );
                                }
                                "np"
                                | "batch-size"
                                | "ubatch-size"
                                | "models-max"
                                | "ctx-checkpoints"
                                | "checkpoint-min-step"
                                | "log-verbosity"
                                | "cache-ram"
                                | "dry-allowed-length"
                                | "dry-penalty-last-n"
                                | "spec-draft-n-max" => {
                                    if let Ok(num) = val_str.parse::<i64>() {
                                        crate::config::update_global_config_value(
                                            &mut state.global_config,
                                            key_to_update,
                                            serde_json::Value::Number(num.into()),
                                        );
                                    }
                                }
                                "dry-multiplier" | "dry-base" | "spec-draft-p-min" => {
                                    if let Ok(num) = val_str.parse::<f64>() {
                                        if let Some(n) = serde_json::Number::from_f64(num) {
                                            crate::config::update_global_config_value(
                                                &mut state.global_config,
                                                key_to_update,
                                                serde_json::Value::Number(n),
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Save config
                        let _ =
                            crate::config::save_config(&state.config_path, &state.global_config);

                        state.screen = AppScreen::Settings;
                    }
                    _ => {}
                }
            }
            KeyCode::Up => {
                if state.screen == AppScreen::EditingConfigFileName
                    && !state.similar_config_files.is_empty()
                {
                    let len = state.similar_config_files.len();
                    state.similar_config_index = Some(match state.similar_config_index {
                        Some(idx) => {
                            if idx == 0 {
                                len - 1
                            } else {
                                idx - 1
                            }
                        }
                        None => len - 1,
                    });
                    if let Some(idx) = state.similar_config_index {
                        state.input_buffer = state.similar_config_files[idx].clone();
                    }
                }
            }
            KeyCode::Down => {
                if state.screen == AppScreen::EditingConfigFileName
                    && !state.similar_config_files.is_empty()
                {
                    let len = state.similar_config_files.len();
                    state.similar_config_index = Some(match state.similar_config_index {
                        Some(idx) => (idx + 1) % len,
                        None => 0,
                    });
                    if let Some(idx) = state.similar_config_index {
                        state.input_buffer = state.similar_config_files[idx].clone();
                    }
                }
            }
            KeyCode::Backspace => {
                state.input_buffer.pop();
                state.validation_error = None;
                if state.screen == AppScreen::EditingConfigFileName {
                    state.similar_config_index = state
                        .similar_config_files
                        .iter()
                        .position(|f| f == &state.input_buffer);
                }
            }
            KeyCode::Char(c) => {
                state.input_buffer.push(c);
                state.validation_error = None;
                if state.screen == AppScreen::EditingConfigFileName {
                    state.similar_config_index = state
                        .similar_config_files
                        .iter()
                        .position(|f| f == &state.input_buffer);
                }
            }
            _ => {}
        },
        AppScreen::SelectingGlobalSettingOption => match key.code {
            KeyCode::Esc => {
                state.screen = AppScreen::Settings;
            }
            KeyCode::Up => {
                if state.option_selector_index == 0 {
                    state.option_selector_index = state.option_selector_list.len() - 1;
                } else {
                    state.option_selector_index -= 1;
                }
            }
            KeyCode::Down => {
                state.option_selector_index =
                    (state.option_selector_index + 1) % state.option_selector_list.len();
            }
            KeyCode::Enter => {
                let selected_opt = state.option_selector_list[state.option_selector_index].clone();
                let selected_item = &ui::SETTINGS[state.settings_index];
                let key_to_update = selected_item.key;

                if selected_opt == "(Custom / Manual...)" {
                    // Transition to manual entry
                    let val_str = crate::config::get_global_config_string(
                        &state.global_config,
                        key_to_update,
                        selected_item.default_val,
                    );
                    state.screen = AppScreen::EditingGlobalSetting;
                    state.input_buffer = val_str;
                } else {
                    // Selected standard option. Save it!
                    if selected_opt == selected_item.default_val {
                        crate::config::remove_global_config_value(
                            &mut state.global_config,
                            key_to_update,
                        );
                    } else {
                        let val = if selected_item.key == "log-verbosity" {
                            if let Ok(num) = selected_opt.parse::<i64>() {
                                serde_json::Value::Number(num.into())
                            } else {
                                serde_json::Value::String(selected_opt)
                            }
                        } else {
                            serde_json::Value::String(selected_opt)
                        };
                        crate::config::update_global_config_value(
                            &mut state.global_config,
                            key_to_update,
                            val,
                        );
                    }

                    // Save config
                    let _ = crate::config::save_config(&state.config_path, &state.global_config);

                    state.screen = AppScreen::Settings;
                }
            }
            _ => {}
        },
        AppScreen::SelectingMMProj => match key.code {
            KeyCode::Esc => {
                state.mmproj_index = state.mmproj_index_backup;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Up if !state.mmproj_list.is_empty() => {
                if state.mmproj_index == 0 {
                    state.mmproj_index = state.mmproj_list.len() - 1;
                } else {
                    state.mmproj_index -= 1;
                }
            }
            KeyCode::Down if !state.mmproj_list.is_empty() => {
                state.mmproj_index = (state.mmproj_index + 1) % state.mmproj_list.len();
            }
            KeyCode::Enter => {
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::SelectingDraftModel => match key.code {
            KeyCode::Esc => {
                state.draft_index = state.draft_index_backup;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Up if !state.draft_list.is_empty() => {
                if state.draft_index == 0 {
                    state.draft_index = state.draft_list.len() - 1;
                } else {
                    state.draft_index -= 1;
                }
            }
            KeyCode::Down if !state.draft_list.is_empty() => {
                state.draft_index = (state.draft_index + 1) % state.draft_list.len();
            }
            KeyCode::Enter => {
                if state.draft_list[state.draft_index].is_none() {
                    state.draft_ngl = String::new();
                } else if state.draft_ngl.is_empty() {
                    state.draft_ngl = "auto".to_owned();
                }
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::SelectingReasoning => match key.code {
            KeyCode::Esc => {
                state.reasoning_index = state.reasoning_index_backup;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Up if !state.reasoning_list.is_empty() => {
                if state.reasoning_index == 0 {
                    state.reasoning_index = state.reasoning_list.len() - 1;
                } else {
                    state.reasoning_index -= 1;
                }
            }
            KeyCode::Down if !state.reasoning_list.is_empty() => {
                state.reasoning_index = (state.reasoning_index + 1) % state.reasoning_list.len();
            }
            KeyCode::Enter => {
                state.reasoning = state.reasoning_list[state.reasoning_index].clone();
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::SelectingReasoningFormat => match key.code {
            KeyCode::Esc => {
                state.reasoning_format_index = state.reasoning_format_index_backup;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Up if !state.reasoning_format_list.is_empty() => {
                if state.reasoning_format_index == 0 {
                    state.reasoning_format_index = state.reasoning_format_list.len() - 1;
                } else {
                    state.reasoning_format_index -= 1;
                }
            }
            KeyCode::Down if !state.reasoning_format_list.is_empty() => {
                state.reasoning_format_index =
                    (state.reasoning_format_index + 1) % state.reasoning_format_list.len();
            }
            KeyCode::Enter => {
                state.reasoning_format =
                    state.reasoning_format_list[state.reasoning_format_index].clone();
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::SelectingSpecType => match key.code {
            KeyCode::Esc => {
                state.spec_type_index = state.spec_type_backup;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Up if !state.spec_type_list.is_empty() => {
                if state.spec_type_index == 0 {
                    state.spec_type_index = state.spec_type_list.len() - 1;
                } else {
                    state.spec_type_index -= 1;
                }
            }
            KeyCode::Down if !state.spec_type_list.is_empty() => {
                state.spec_type_index = (state.spec_type_index + 1) % state.spec_type_list.len();
            }
            KeyCode::Enter => {
                state.spec_type = state.spec_type_list[state.spec_type_index].clone();
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::ConfirmSaveConfig => match key.code {
            KeyCode::Esc => {
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Char(' ') => {
                state.backup_config = !state.backup_config;
            }
            KeyCode::Enter => {
                let _ = state.save_current_preset_config(state.backup_config);
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::WarnDiscardChanges => match key.code {
            KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                state.pending_preset_index = None;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                if let Some(target) = state.pending_preset_index.take() {
                    if target < state.presets.len() {
                        state.preset_index = target;
                    } else {
                        state.preset_index = 0;
                    }
                    state.load_current_preset_settings(None);
                }
                state.screen = AppScreen::Dashboard;
            }
            _ => {}
        },
        AppScreen::Logs => match key.code {
            KeyCode::Char('q') => {
                if let Some(mut server) = state.active_server.take() {
                    server.kill();
                }
                should_quit = true;
            }
            KeyCode::Char('s') => {
                if let Some(mut server) = state.active_server.take() {
                    server.kill();
                }
                state.screen = AppScreen::Dashboard;
                state.active_tab = 0;
            }
            KeyCode::Char('r') => {
                // Restart server
                if let Some(mut server) = state.active_server.take() {
                    server.kill();
                }
                let model_name = if state.presets.is_empty() {
                    None
                } else {
                    Some(state.presets[state.preset_index].0.clone())
                };
                match ActiveServer::spawn(
                    &state.last_launch_args,
                    &state.models_dir,
                    model_name,
                    Some(event_tx.clone()),
                ) {
                    Ok(server) => {
                        state.active_server = Some(server);
                        state.logs_paused = false;
                        state.paused_logs_buffer.clear();
                        state.auto_scroll = true;
                        state.log_scroll_offset = 0;
                        state.log_scroll_x = 0;
                    }
                    Err(_e) => {}
                }
            }
            KeyCode::Char('p') => {
                state.logs_paused = !state.logs_paused;
                if state.logs_paused {
                    if let Some(ref server) = state.active_server
                        && let Ok(l) = server.logs.lock()
                    {
                        state.paused_logs_buffer = l.clone();
                    }
                } else {
                    state.paused_logs_buffer.clear();
                }
            }
            KeyCode::Char('c') => {
                // Copy all logs to system clipboard
                if let Some(ref server) = state.active_server
                    && let Ok(hist) = server.raw_history.lock()
                {
                    let total_len: usize =
                        hist.iter().map(String::len).sum::<usize>() + hist.len().saturating_sub(1);
                    let mut full_text = String::with_capacity(total_len);
                    for (i, s) in hist.iter().enumerate() {
                        if i > 0 {
                            full_text.push('\n');
                        }
                        full_text.push_str(s);
                    }
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(full_text);
                    }
                }
            }
            KeyCode::Char('w') => {
                state.logs_wrap = !state.logs_wrap;
            }
            KeyCode::Char('a' | 'A' | ' ') => {
                state.auto_scroll = !state.auto_scroll;
            }
            KeyCode::Up => {
                state.auto_scroll = false;
                if state.log_scroll_offset > 0 {
                    state.log_scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                state.auto_scroll = false;
                state.log_scroll_offset += 1;
            }
            KeyCode::PageUp => {
                state.auto_scroll = false;
                if state.log_scroll_offset > 15 {
                    state.log_scroll_offset -= 15;
                } else {
                    state.log_scroll_offset = 0;
                }
            }
            KeyCode::PageDown => {
                state.auto_scroll = false;
                state.log_scroll_offset += 15;
            }
            KeyCode::Home => {
                state.auto_scroll = false;
                state.log_scroll_offset = 0;
            }
            KeyCode::End => {
                state.auto_scroll = true;
            }
            KeyCode::Left => {
                if state.log_scroll_x > 4 {
                    state.log_scroll_x -= 4;
                } else {
                    state.log_scroll_x = 0;
                }
            }
            KeyCode::Right => {
                state.log_scroll_x += 4;
            }
            _ => {}
        },
    }

    should_quit
}

/// Starts the TUI event loop and renders the dashboard.
///
/// # Errors
/// Returns an `io::Result` if terminal setup or the event loop fails.
pub fn run_tui(mut state: AppState) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (event_tx, event_rx) = std::sync::mpsc::channel::<TuiEvent>();

    // Spawn thread for user input events
    {
        let event_tx = event_tx.clone();
        std::thread::spawn(move || {
            loop {
                if matches!(event::poll(Duration::from_millis(100)), Ok(true)) {
                    match event::read() {
                        Ok(Event::Key(key))
                            if key.kind == event::KeyEventKind::Press
                                && event_tx.send(TuiEvent::Input(key)).is_err() =>
                        {
                            break;
                        }

                        _ => {}
                    }
                }
            }
        });
    }

    // Spawn thread for models directory changes monitoring
    {
        let event_tx = event_tx.clone();
        let models_dir_sharing = std::sync::Arc::clone(&state.shared_models_dir);
        std::thread::spawn(move || {
            let mut last_state: Option<app::ModelsDirState> = None;
            let mut last_stable_state: Option<app::ModelsDirState> = None;

            loop {
                std::thread::sleep(Duration::from_secs(1));

                let dir = {
                    if let Ok(lock) = models_dir_sharing.lock() {
                        lock.clone()
                    } else {
                        break;
                    }
                };

                if std::fs::read_dir(&dir).is_err() {
                    if event_tx.send(TuiEvent::ModelsDirInvalid).is_err() {
                        break;
                    }
                    continue;
                }

                let current_state = app::get_models_dir_state(&dir);
                if let Some(new_state) = current_state {
                    if let Some(ref prev_state) = last_state {
                        let mut is_stable = true;
                        for (path, mtime, size) in &new_state.files {
                            if let Some((_, prev_mtime, prev_size)) =
                                prev_state.files.iter().find(|(p, _, _)| p == path)
                            {
                                if prev_size != size || prev_mtime != mtime {
                                    is_stable = false;
                                    break;
                                }
                            } else {
                                is_stable = false;
                            }
                        }

                        if is_stable {
                            if let Some(ref stable_state) = last_stable_state {
                                if stable_state != &new_state {
                                    if event_tx
                                        .send(TuiEvent::ModelsDirChanged(new_state.clone()))
                                        .is_err()
                                    {
                                        break;
                                    }
                                    last_stable_state = Some(new_state.clone());
                                }
                            } else {
                                last_stable_state = Some(new_state.clone());
                            }
                        }
                    }
                    last_state = Some(new_state);
                }
            }
        });
    }

    // Spawn thread for periodic ticks
    {
        let event_tx = event_tx.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(250));
                if event_tx.send(TuiEvent::Tick).is_err() {
                    break;
                }
            }
        });
    }

    let mut should_quit = false;

    // Draw the initial screen before blocking on events
    terminal.draw(|f| ui::draw(f, &mut state))?;

    while !should_quit {
        if let Ok(first_event) = event_rx.recv() {
            let mut events = vec![first_event];
            // Coalesce / batch rapid subsequent events (e.g. multiple log lines)
            while let Ok(event) = event_rx.try_recv() {
                events.push(event);
            }

            for event in events {
                match event {
                    TuiEvent::Input(key) => {
                        should_quit = handle_key_event(&mut state, key, &event_tx);
                    }

                    TuiEvent::Tick => {
                        state.tick_count += 1;
                        if state.tick_count % 4 == 0
                            && state.models_dir_changed_dirty
                            && !state.has_unsaved_changes()
                        {
                            state.models_dir_changed_dirty = false;
                            state.load_current_preset_settings(None);
                        }
                    }
                    TuiEvent::ModelsDirChanged(new_state) => {
                        state.handle_models_dir_changed(new_state);
                    }
                    TuiEvent::ModelsDirInvalid => {
                        state.handle_models_dir_invalid();
                    }
                    TuiEvent::LogReceived => {}
                }
            }

            terminal.draw(|f| ui::draw(f, &mut state))?;
        } else {
            break; // Channel disconnected, exit loop
        }
    }

    // Clean up terminal raw mode and restore screen
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
