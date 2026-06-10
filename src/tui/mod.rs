pub mod app;
pub mod logs;
pub mod picker;
pub mod theme;
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

#[derive(Clone, Debug)]
pub enum TuiEvent {
    Input(KeyEvent),
    Tick,
    LogReceived,
}

pub fn handle_key_event(
    state: &mut AppState,
    key: KeyEvent,
    event_tx: &std::sync::mpsc::Sender<TuiEvent>,
) -> bool {
    let mut should_quit = false;

    if !matches!(
        state.screen,
        AppScreen::EditingCtx
            | AppScreen::EditingNgl
            | AppScreen::EditingDraftNgl
            | AppScreen::PickingServerPath
            | AppScreen::PickingModelsDir
            | AppScreen::EditingGlobalSetting
            | AppScreen::SelectingGlobalSettingOption
            | AppScreen::SelectingMMProj
            | AppScreen::SelectingDraftModel
            | AppScreen::EditingTemp
            | AppScreen::EditingTopP
            | AppScreen::EditingTopK
            | AppScreen::EditingTotalLayers
            | AppScreen::EditingConfigFileName
            | AppScreen::ConfirmSaveConfig
            | AppScreen::WarnDiscardChanges
            | AppScreen::EditingMinP
            | AppScreen::EditingRepeatPenalty
            | AppScreen::EditingRepeatLastN
            | AppScreen::SelectingReasoningFormat
            | AppScreen::SelectingReasoning
            | AppScreen::EditingReasoningBudget
    ) {
        match key.code {
            KeyCode::Char('1') => {
                state.active_tab = 0;
                state.screen = AppScreen::Dashboard;
                return false;
            }
            KeyCode::Char('2') => {
                state.active_tab = 1;
                state.screen = AppScreen::Settings;
                return false;
            }
            KeyCode::Char('3') => {
                state.active_tab = 2;
                state.screen = AppScreen::Logs;
                return false;
            }
            _ => {}
        }
    }

    match state.screen {
        AppScreen::Dashboard => match key.code {
            KeyCode::Char('q') => {
                should_quit = true;
            }
            KeyCode::Char('c') => {
                state.screen = AppScreen::EditingCtx;
                state.input_buffer = if state.ctx_str.is_empty() {
                    state.ctx.to_string()
                } else {
                    state.ctx_str.clone()
                };
            }
            KeyCode::Char('n') => {
                state.screen = AppScreen::EditingNgl;
                state.input_buffer = state.ngl.clone();
            }
            KeyCode::Char('g') => {
                state.screen = AppScreen::EditingDraftNgl;
                state.input_buffer = state.draft_ngl.clone();
            }
            KeyCode::Char('v') => {
                state.mmproj_index_backup = state.mmproj_index;
                state.screen = AppScreen::SelectingMMProj;
            }
            KeyCode::Char('d') => {
                state.draft_index_backup = state.draft_index;
                state.screen = AppScreen::SelectingDraftModel;
            }
            KeyCode::Char('t') => {
                state.screen = AppScreen::EditingTemp;
                state.input_buffer = state.temp.clone();
            }
            KeyCode::Char('p') => {
                state.screen = AppScreen::EditingTopP;
                state.input_buffer = state.top_p.clone();
            }
            KeyCode::Char('k') => {
                state.screen = AppScreen::EditingTopK;
                state.input_buffer = state.top_k.clone();
            }
            KeyCode::Char('l') => {
                state.screen = AppScreen::EditingTotalLayers;
                state.input_buffer = state
                    .total_layers
                    .map(|l| l.to_string())
                    .unwrap_or_default();
            }
            KeyCode::Char('f') => {
                state.screen = AppScreen::EditingConfigFileName;
                state.input_buffer = state.config_file_name.clone();
                if !state.presets.is_empty() {
                    let (_, model_path) = &state.presets[state.preset_index];
                    state.similar_config_files =
                        crate::config::find_similar_config_files(model_path, &state.models_dir);
                    state.similar_config_index = state
                        .similar_config_files
                        .iter()
                        .position(|f| f == &state.input_buffer);
                } else {
                    state.similar_config_files.clear();
                    state.similar_config_index = None;
                }
            }
            KeyCode::Char('m') => {
                state.screen = AppScreen::EditingMinP;
                state.input_buffer = state.min_p.clone();
            }
            KeyCode::Char('e') => {
                state.screen = AppScreen::EditingRepeatPenalty;
                state.input_buffer = state.repeat_penalty.clone();
            }
            KeyCode::Char('a') => {
                state.screen = AppScreen::EditingRepeatLastN;
                state.input_buffer = state.repeat_last_n.clone();
            }
            KeyCode::Char('o') => {
                state.reasoning_format_index_backup = state.reasoning_format_index;
                state.screen = AppScreen::SelectingReasoningFormat;
            }
            KeyCode::Char('u') => {
                state.reasoning_index_backup = state.reasoning_index;
                state.screen = AppScreen::SelectingReasoning;
            }
            KeyCode::Char('b') => {
                state.screen = AppScreen::EditingReasoningBudget;
                state.input_buffer = state.reasoning_budget.clone();
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
                        state.dashboard_param_index = 15;
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
                    state.dashboard_param_index = (state.dashboard_param_index + 1) % 16;
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
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
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
                            v.as_str().map(|s| s.to_string())
                        }
                    })
                    .unwrap_or_else(|| "auto".to_string());
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
                state.last_launch_args = launch_args.clone();
                state.is_router_mode = true;
                let model_name = if state.presets.is_empty() {
                    None
                } else {
                    Some(state.presets[state.preset_index].0.clone())
                };

                match ActiveServer::spawn(
                    &launch_args,
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
            KeyCode::Enter => {
                if state.dashboard_focus == DashboardFocus::Right {
                    match state.dashboard_param_index {
                        0 => {
                            state.screen = AppScreen::EditingConfigFileName;
                            state.input_buffer = state.config_file_name.clone();
                            if !state.presets.is_empty() {
                                let (_, model_path) = &state.presets[state.preset_index];
                                state.similar_config_files =
                                    crate::config::find_similar_config_files(
                                        model_path,
                                        &state.models_dir,
                                    );
                                state.similar_config_index = state
                                    .similar_config_files
                                    .iter()
                                    .position(|f| f == &state.input_buffer);
                            } else {
                                state.similar_config_files.clear();
                                state.similar_config_index = None;
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
                            state.screen = AppScreen::EditingTemp;
                            state.input_buffer = state.temp.clone();
                        }
                        8 => {
                            state.screen = AppScreen::EditingTopP;
                            state.input_buffer = state.top_p.clone();
                        }
                        9 => {
                            state.screen = AppScreen::EditingTopK;
                            state.input_buffer = state.top_k.clone();
                        }
                        10 => {
                            state.screen = AppScreen::EditingMinP;
                            state.input_buffer = state.min_p.clone();
                        }
                        11 => {
                            state.screen = AppScreen::EditingRepeatPenalty;
                            state.input_buffer = state.repeat_penalty.clone();
                        }
                        12 => {
                            state.screen = AppScreen::EditingRepeatLastN;
                            state.input_buffer = state.repeat_last_n.clone();
                        }
                        13 => {
                            state.reasoning_format_index_backup = state.reasoning_format_index;
                            state.screen = AppScreen::SelectingReasoningFormat;
                        }
                        14 => {
                            state.reasoning_index_backup = state.reasoning_index;
                            state.screen = AppScreen::SelectingReasoning;
                        }
                        15 => {
                            state.screen = AppScreen::EditingReasoningBudget;
                            state.input_buffer = state.reasoning_budget.clone();
                        }
                        _ => {}
                    }
                } else if !state.presets.is_empty() {
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
                                v.as_str().map(|s| s.to_string())
                            }
                        })
                        .unwrap_or_else(|| "auto".to_string());
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
                    state.last_launch_args = launch_args.clone();
                    state.is_router_mode = false;
                    let model_name = if state.presets.is_empty() {
                        None
                    } else {
                        Some(state.presets[state.preset_index].0.clone())
                    };

                    match ActiveServer::spawn(
                        &launch_args,
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
            _ => {}
        },
        AppScreen::Settings => match key.code {
            KeyCode::Up => {
                if state.settings_index == 0 {
                    state.settings_index = crate::tui::ui::SETTINGS.len() - 1;
                } else {
                    state.settings_index -= 1;
                }
            }
            KeyCode::Down => {
                state.settings_index = (state.settings_index + 1) % crate::tui::ui::SETTINGS.len();
            }
            KeyCode::Enter => {
                let selected_item = &crate::tui::ui::SETTINGS[state.settings_index];
                match selected_item.key {
                    "llama-server" => {
                        state.screen = AppScreen::PickingServerPath;
                        let initial_path = if state.server_exe.as_os_str().is_empty() {
                            crate::config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
                        } else {
                            state
                                .server_exe
                                .parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| PathBuf::from("."))
                        };
                        state.picker = Some(crate::tui::picker::FilePicker::new(
                            initial_path,
                            crate::tui::picker::PickerMode::File,
                        ));
                    }
                    "models-dir" => {
                        state.screen = AppScreen::PickingModelsDir;
                        let initial_path = if state.models_dir.as_os_str().is_empty() {
                            crate::config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
                        } else {
                            state.models_dir.clone()
                        };
                        state.picker = Some(crate::tui::picker::FilePicker::new(
                            initial_path,
                            crate::tui::picker::PickerMode::Directory,
                        ));
                    }
                    "flash-attn" | "cache-type-k" | "cache-type-v" | "log-verbosity" | "numa"
                    | "split-mode" => {
                        // Option selectors for flash-attn, cache-type-k, cache-type-v, log-verbosity, numa, split-mode
                        let option_list = match selected_item.key {
                            "flash-attn" => vec![
                                "auto".to_string(),
                                "1".to_string(),
                                "0".to_string(),
                                "(Custom / Manual...)".to_string(),
                            ],
                            "log-verbosity" => vec![
                                "0".to_string(),
                                "1".to_string(),
                                "2".to_string(),
                                "3".to_string(),
                                "4".to_string(),
                                "5".to_string(),
                                "(Custom / Manual...)".to_string(),
                            ],
                            "numa" => vec![
                                "none".to_string(),
                                "distribute".to_string(),
                                "isolate".to_string(),
                                "numactl".to_string(),
                                "(Custom / Manual...)".to_string(),
                            ],
                            "split-mode" => vec![
                                "layer".to_string(),
                                "none".to_string(),
                                "row".to_string(),
                                "tensor".to_string(),
                                "(Custom / Manual...)".to_string(),
                            ],
                            _ => vec![
                                "f16".to_string(),
                                "q8_0".to_string(),
                                "q4_0".to_string(),
                                "q4_1".to_string(),
                                "iq4_nl".to_string(),
                                "q5_0".to_string(),
                                "q5_1".to_string(),
                                "f32".to_string(),
                                "bf16".to_string(),
                                "(Custom / Manual...)".to_string(),
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
                            .and_then(|v| v.as_bool())
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
                        state.global_config.insert(
                            "llama-server".to_string(),
                            serde_json::Value::String(path.to_string_lossy().to_string()),
                        );
                    } else {
                        state.models_dir = path.clone();
                        state.global_config.insert(
                            "models-dir".to_string(),
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
                        let new_state = crate::tui::app::get_models_dir_state(&state.models_dir);
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
        | AppScreen::EditingReasoningBudget => match key.code {
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
                        let val = state.input_buffer.trim().to_string();
                        if let Ok(parsed) = crate::config::parse_ctx_str(&val) {
                            state.ctx_str = val;
                            state.ctx = parsed;
                            state.screen = AppScreen::Dashboard;
                        }
                    }
                    AppScreen::EditingNgl => {
                        state.ngl = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingDraftNgl => {
                        state.draft_ngl = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingTemp => {
                        state.temp = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingTopP => {
                        state.top_p = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingTopK => {
                        state.top_k = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingTotalLayers => {
                        let val = state.input_buffer.trim();
                        if val.is_empty() {
                            state.total_layers = None;
                        } else if let Ok(num) = val.parse::<usize>() {
                            state.total_layers = Some(num);
                        }
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingConfigFileName => {
                        state.config_file_name = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingMinP => {
                        state.min_p = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingRepeatPenalty => {
                        state.repeat_penalty = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingRepeatLastN => {
                        state.repeat_last_n = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingReasoningBudget => {
                        state.reasoning_budget = state.input_buffer.trim().to_string();
                        state.screen = AppScreen::Dashboard;
                    }
                    AppScreen::EditingGlobalSetting => {
                        let val_str = state.input_buffer.trim().to_string();
                        let selected_item = &crate::tui::ui::SETTINGS[state.settings_index];
                        let key_to_update = selected_item.key;

                        if val_str == selected_item.default_val {
                            crate::config::remove_global_config_value(
                                &mut state.global_config,
                                key_to_update,
                            );
                        } else {
                            match selected_item.key {
                                "host" | "flash-attn" | "cache-type-k" | "cache-type-v"
                                | "api-key" | "device" | "api-key-file" | "ssl-key-file"
                                | "ssl-cert-file" => {
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
                                | "cache-ram" => {
                                    if let Ok(num) = val_str.parse::<i64>() {
                                        crate::config::update_global_config_value(
                                            &mut state.global_config,
                                            key_to_update,
                                            serde_json::Value::Number(num.into()),
                                        );
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
                if state.screen == AppScreen::EditingConfigFileName {
                    state.similar_config_index = state
                        .similar_config_files
                        .iter()
                        .position(|f| f == &state.input_buffer);
                }
            }
            KeyCode::Char(c) => {
                state.input_buffer.push(c);
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
                let selected_item = &crate::tui::ui::SETTINGS[state.settings_index];
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
                    state.draft_ngl = "".to_string();
                } else if state.draft_ngl.is_empty() {
                    state.draft_ngl = "auto".to_string();
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
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.pending_preset_index = None;
                state.screen = AppScreen::Dashboard;
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(target) = state.pending_preset_index.take() {
                    state.preset_index = target;
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
                    let full_text = hist
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                        .join("\n");
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(full_text);
                    }
                }
            }
            KeyCode::Char('w') => {
                state.logs_wrap = !state.logs_wrap;
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
                if let Ok(true) = crossterm::event::poll(Duration::from_millis(100)) {
                    match crossterm::event::read() {
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
                        if state.tick_count.is_multiple_of(4) {
                            state.check_models_dir_changes();
                        }
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
