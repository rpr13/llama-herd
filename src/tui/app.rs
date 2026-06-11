use crate::config::UserSettings;
use crate::tui::logs::ActiveServer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The different screen views and input modal dialog states.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AppScreen {
    /// Main dashboard view.
    Dashboard,
    /// Settings management view.
    Settings,
    /// Active server logs streaming view.
    Logs,
    /// Editing context size input mode.
    EditingCtx,
    /// Editing GPU layers (ngl) input mode.
    EditingNgl,
    /// Editing draft model GPU layers input mode.
    EditingDraftNgl,
    /// File picker for selecting the llama-server executable path.
    PickingServerPath,
    /// File picker for selecting the GGUF models directory.
    PickingModelsDir,
    /// Text editing modal for global configurations.
    EditingGlobalSetting,
    /// Selection option list modal for global configurations.
    SelectingGlobalSettingOption,
    /// Dropdown list for selecting a vision projector.
    SelectingMMProj,
    /// Dropdown list for selecting a speculative draft model.
    SelectingDraftModel,
    /// Editing temperature parameter input mode.
    EditingTemp,
    /// Editing top-p parameter input mode.
    EditingTopP,
    /// Editing top-k parameter input mode.
    EditingTopK,
    /// Editing total model layers parameter input mode.
    EditingTotalLayers,
    /// Editing TOML configuration filename input mode.
    EditingConfigFileName,
    /// Confirmation dialog before saving configuration changes.
    ConfirmSaveConfig,
    /// Warning dialog before discarding unsaved configuration changes.
    WarnDiscardChanges,
    /// Editing min-p parameter input mode.
    EditingMinP,
    /// Editing repeat penalty parameter input mode.
    EditingRepeatPenalty,
    /// Editing repeat last N parameter input mode.
    EditingRepeatLastN,
    /// Selecting reasoning format dropdown mode.
    SelectingReasoningFormat,
    /// Selecting reasoning setting dropdown mode.
    SelectingReasoning,
    /// Editing reasoning budget parameter input mode.
    EditingReasoningBudget,
}

/// The focus panels on the TUI dashboard tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardFocus {
    /// Left preset list panel focus.
    Left,
    /// Right parameter overrides panel focus.
    Right,
}

/// Representation of the state/contents of the GGUF models directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelsDirState {
    /// List of paths, modification times, and file sizes.
    pub files: Vec<(PathBuf, std::time::SystemTime, u64)>,
}

/// Helper function to retrieve the current file state of the GGUF models directory.
#[must_use]
pub fn get_models_dir_state(models_dir: &Path) -> Option<ModelsDirState> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_lower = ext.to_lowercase();
                if (ext_lower == "gguf" || ext_lower == "toml")
                    && let Ok(meta) = path.metadata()
                {
                    let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let size = meta.len();
                    files.push((path, mtime, size));
                }
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Some(ModelsDirState { files })
}

/// Main application state for the Ratatui dashboard.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct AppState {
    /// Current panel focus on the dashboard.
    pub dashboard_focus: DashboardFocus,
    /// Selected index in the parameter overrides list.
    pub dashboard_param_index: usize,
    /// Discovered list of model presets.
    pub presets: Vec<(String, PathBuf)>,
    /// Configured GGUF models directory path.
    pub models_dir: PathBuf,
    /// Path to the preset INI file.
    pub preset_path: PathBuf,
    /// Merged global configuration.
    pub global_config: HashMap<String, serde_json::Value>,
    /// Path to the llama-server executable.
    pub server_exe: PathBuf,
    /// Cached version string of the server executable.
    pub server_version: String,

    /// Active view/screen mode.
    pub screen: AppScreen,
    /// Currently selected tab index.
    pub active_tab: usize,
    /// Currently selected preset index.
    pub preset_index: usize,
    /// Currently selected global settings index.
    pub settings_index: usize,
    /// Active file picker instance.
    pub picker: Option<crate::tui::picker::FilePicker>,

    /// Override context size in tokens.
    pub ctx: usize,
    /// Override GPU offload layers.
    pub ngl: String,
    /// Total layers in the model (read from configuration).
    pub total_layers: Option<usize>,

    /// List of discovered vision projectors.
    pub mmproj_list: Vec<Option<PathBuf>>,
    /// Selected mmproj index.
    pub mmproj_index: usize,
    /// Backup of the selected mmproj index (for Esc/cancel actions).
    pub mmproj_index_backup: usize,
    /// List of discovered draft models.
    pub draft_list: Vec<Option<PathBuf>>,
    /// Selected draft model index.
    pub draft_index: usize,
    /// Backup of the selected draft model index (for Esc/cancel actions).
    pub draft_index_backup: usize,
    /// Override draft model GPU offload layers.
    pub draft_ngl: String,

    /// Temporary keyboard input text buffer.
    pub input_buffer: String,

    /// Active llama-server subprocess manager wrapper.
    pub active_server: Option<ActiveServer>,
    /// Flag indicating whether log streaming is paused.
    pub logs_paused: bool,
    /// Flag indicating whether logs should wrap to the next line.
    pub logs_wrap: bool,
    /// Ring buffer storing recent logs when streaming is paused.
    pub paused_logs_buffer: std::collections::VecDeque<crate::tui::logs::LogLine>,
    /// Vertical scroll offset in the logs panel.
    pub log_scroll_offset: usize,
    /// Horizontal scroll offset in the logs panel.
    pub log_scroll_x: usize,
    /// Flag indicating whether log panel autoscrolls to the bottom.
    pub auto_scroll: bool,
    /// Arguments used to launch the current active subprocess.
    pub last_launch_args: Vec<String>,
    /// Flag indicating whether the server is launched in Router Mode.
    pub is_router_mode: bool,
    /// Styling theme.
    pub theme: crate::tui::theme::Theme,
    /// Selected option index in a dropdown menu.
    pub option_selector_index: usize,
    /// List of options in a dropdown menu.
    pub option_selector_list: Vec<String>,
    /// Path to the configuration TOML file of the selected model.
    pub config_path: PathBuf,
    /// List of similar configuration files suggested.
    pub similar_config_files: Vec<String>,
    /// Selected index in similar configuration files suggestions.
    pub similar_config_index: Option<usize>,

    /// Override temperature.
    pub temp: String,
    /// Override top-p.
    pub top_p: String,
    /// Override top-k.
    pub top_k: String,
    /// Configuration filename target.
    pub config_file_name: String,
    /// Raw context size input string.
    pub ctx_str: String,
    /// Flag indicating whether to create a backup file when saving.
    pub backup_config: bool,
    /// Pending preset index to switch to after warning confirmation.
    pub pending_preset_index: Option<usize>,

    /// Override min-p.
    pub min_p: String,
    /// Override repeat penalty.
    pub repeat_penalty: String,
    /// Override repeat last N tokens.
    pub repeat_last_n: String,
    /// Override reasoning setting.
    pub reasoning: String,
    /// Override reasoning format.
    pub reasoning_format: String,
    /// Override reasoning budget.
    pub reasoning_budget: String,

    /// List of reasoning settings options.
    pub reasoning_list: Vec<String>,
    /// Selected reasoning index.
    pub reasoning_index: usize,
    /// Backup of the selected reasoning index.
    pub reasoning_index_backup: usize,

    /// List of reasoning formats options.
    pub reasoning_format_list: Vec<String>,
    /// Selected reasoning format index.
    pub reasoning_format_index: usize,
    /// Backup of the selected reasoning format index.
    pub reasoning_format_index_backup: usize,

    /// Original context size input string (to check for edits).
    pub original_ctx_str: String,
    /// Original context size (to check for edits).
    pub original_ctx: usize,
    /// Original GPU layers (to check for edits).
    pub original_ngl: String,
    /// Original mmproj index (to check for edits).
    pub original_mmproj_index: usize,
    /// Original draft model index (to check for edits).
    pub original_draft_index: usize,
    /// Original draft GPU layers (to check for edits).
    pub original_draft_ngl: String,
    /// Original temperature (to check for edits).
    pub original_temp: String,
    /// Original top-p (to check for edits).
    pub original_top_p: String,
    /// Original top-k (to check for edits).
    pub original_top_k: String,
    /// Original total layers (to check for edits).
    pub original_total_layers: Option<usize>,
    /// Original configuration filename target (to check for edits).
    pub original_config_file_name: String,

    /// Original min-p (to check for edits).
    pub original_min_p: String,
    /// Original repeat penalty (to check for edits).
    pub original_repeat_penalty: String,
    /// Original repeat last N tokens (to check for edits).
    pub original_repeat_last_n: String,
    /// Original reasoning setting (to check for edits).
    pub original_reasoning: String,
    /// Original reasoning format (to check for edits).
    pub original_reasoning_format: String,
    /// Original reasoning budget (to check for edits).
    pub original_reasoning_budget: String,
    /// Original reasoning index (to check for edits).
    pub original_reasoning_index: usize,
    /// Original reasoning format index (to check for edits).
    pub original_reasoning_format_index: usize,

    /// Number of clock ticks (useful for flash animations).
    pub tick_count: u64,
    /// Last recorded file state of the GGUF models directory.
    pub last_models_dir_state: Option<ModelsDirState>,
    /// Last stable recorded file state of the GGUF models directory.
    pub last_stable_models_dir_state: Option<ModelsDirState>,
    /// Flag indicating if background models directory scan has pending changes.
    pub models_dir_changed_dirty: bool,
    /// Flag indicating if GGUF models directory is inaccessible or deleted.
    pub models_dir_invalid: bool,
}

impl AppState {
    /// Creates a new `AppState` instance with the provided presets, paths, configurations, and theme.
    #[must_use]
    pub fn new(
        presets: Vec<(String, PathBuf)>,
        models_dir: PathBuf,
        preset_path: PathBuf,
        global_config: HashMap<String, serde_json::Value>,
        server_exe: PathBuf,
        theme: crate::tui::theme::Theme,
    ) -> Self {
        let server_version = crate::launcher::get_server_version(&server_exe);
        let last_models_dir_state = get_models_dir_state(&models_dir);
        let models_dir_invalid = std::fs::read_dir(&models_dir).is_err();

        let mut state = Self {
            dashboard_focus: DashboardFocus::Left,
            dashboard_param_index: 0,
            presets,
            models_dir,
            preset_path,
            global_config,
            server_exe,
            server_version,
            screen: AppScreen::Dashboard,
            active_tab: 0,
            preset_index: 0,
            settings_index: 0,
            picker: None,
            ctx: 131_072,
            ngl: "auto".to_owned(),
            total_layers: None,
            mmproj_list: vec![None],
            mmproj_index: 0,
            mmproj_index_backup: 0,
            draft_list: vec![None],
            draft_index: 0,
            draft_index_backup: 0,
            draft_ngl: String::new(),
            input_buffer: String::new(),
            active_server: None,
            logs_paused: false,
            logs_wrap: false,
            paused_logs_buffer: std::collections::VecDeque::new(),
            log_scroll_offset: 0,
            log_scroll_x: 0,
            auto_scroll: true,
            last_launch_args: Vec::new(),
            is_router_mode: false,
            theme,
            option_selector_index: 0,
            option_selector_list: Vec::new(),
            config_path: crate::config::get_llama_herd_dir().join("config.toml"),
            similar_config_files: Vec::new(),
            similar_config_index: None,
            temp: String::new(),
            top_p: String::new(),
            top_k: String::new(),
            config_file_name: String::new(),
            ctx_str: String::new(),
            backup_config: true,
            pending_preset_index: None,
            min_p: String::new(),
            repeat_penalty: String::new(),
            repeat_last_n: String::new(),
            reasoning: String::new(),
            reasoning_format: String::new(),
            reasoning_budget: String::new(),
            reasoning_list: vec![
                String::new(),
                "auto".to_owned(),
                "on".to_owned(),
                "off".to_owned(),
            ],
            reasoning_index: 0,
            reasoning_index_backup: 0,
            reasoning_format_list: vec![
                String::new(),
                "auto".to_owned(),
                "none".to_owned(),
                "deepseek".to_owned(),
                "deepseek-legacy".to_owned(),
            ],
            reasoning_format_index: 0,
            reasoning_format_index_backup: 0,
            original_ctx_str: String::new(),
            original_ctx: 131_072,
            original_ngl: "auto".to_owned(),
            original_mmproj_index: 0,
            original_draft_index: 0,
            original_draft_ngl: String::new(),
            original_temp: String::new(),
            original_top_p: String::new(),
            original_top_k: String::new(),
            original_total_layers: None,
            original_config_file_name: String::new(),
            original_min_p: String::new(),
            original_repeat_penalty: String::new(),
            original_repeat_last_n: String::new(),
            original_reasoning: String::new(),
            original_reasoning_format: String::new(),
            original_reasoning_budget: String::new(),
            original_reasoning_index: 0,
            original_reasoning_format_index: 0,
            tick_count: 0,
            last_models_dir_state: last_models_dir_state.clone(),
            last_stable_models_dir_state: last_models_dir_state,
            models_dir_changed_dirty: false,
            models_dir_invalid,
        };

        state.load_current_preset_settings(None);
        state
    }

    /// Loads the configuration settings for the currently selected preset, with an optional path override.
    #[allow(
        clippy::similar_names,
        clippy::too_many_lines,
        clippy::cast_possible_truncation
    )]
    pub fn load_current_preset_settings(&mut self, toml_path_override: Option<PathBuf>) {
        self.models_dir_changed_dirty = false;
        if self.presets.is_empty() {
            return;
        }

        let (preset_name, model_path) = &self.presets[self.preset_index];
        let mut ini_settings =
            crate::config::load_settings_from_ini(preset_name, &self.preset_path)
                .unwrap_or_default();

        let mut assets = crate::discovery::discover_assets(model_path, &self.models_dir);
        if let Some(ref path) = toml_path_override {
            assets.config = crate::config::load_toml_silent(path);
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

        let total_layers = get_lh_val("total-layers")
            .or_else(|| get_long_val("total-layers"))
            .and_then(|v| v.as_u64().map(|i| i as usize));
        self.total_layers = total_layers;

        // Context Size
        let ctx_val = ini_settings.remove("ctx-size").map_or_else(
            || {
                get_lh_val("ctx-size")
                    .or_else(|| get_long_val("ctx-size"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::String("131072".to_owned()))
            },
            serde_json::Value::String,
        );
        self.ctx = crate::config::parse_ctx(&ctx_val).unwrap_or(131_072);

        // NGL
        self.ngl = ini_settings.remove("n-gpu-layers").unwrap_or_else(|| {
            get_lh_val("ngl")
                .or_else(|| get_long_val("ngl"))
                .and_then(|v| {
                    v.as_str()
                        .map(ToOwned::to_owned)
                        .or_else(|| v.as_i64().map(|i| i.to_string()))
                })
                .unwrap_or_else(|| {
                    total_layers.map_or_else(|| "auto".to_owned(), |t| t.to_string())
                })
        });

        // Populate mmproj list
        let mut mmproj_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
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

        self.mmproj_list = vec![None];
        for f in mmproj_files {
            self.mmproj_list.push(Some(f));
        }

        // Find active mmproj
        self.mmproj_index = 0;
        if let Some(active_mmproj) = ini_settings.remove("mmproj") {
            let active_name = Path::new(&active_mmproj).file_name().unwrap_or_default();
            for (idx, opt) in self.mmproj_list.iter().enumerate() {
                if let Some(path) = opt
                    && path.file_name().unwrap_or_default() == active_name
                {
                    self.mmproj_index = idx;
                    break;
                }
            }
        }

        // Populate draft list
        let mut draft_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("gguf")
                    && !path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains("mmproj")
                    && &path != model_path
                {
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let mut is_draft = false;
                    if let Ok(sub_entries) = std::fs::read_dir(&self.models_dir) {
                        for se in sub_entries.flatten() {
                            let sp = se.path();
                            if sp.extension().and_then(|s| s.to_str()) == Some("toml") {
                                let js_stem = sp
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();
                                if stem.starts_with(&js_stem) {
                                    let cfg = crate::config::load_toml_silent(&sp);
                                    let is_d = cfg
                                        .get("llama-herd")
                                        .and_then(|lh| {
                                            lh.get("is-draft").or_else(|| lh.get("is-draft-only"))
                                        })
                                        .and_then(serde_json::Value::as_bool)
                                        == Some(true);
                                    if is_d {
                                        is_draft = true;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    if is_draft {
                        draft_files.push(path);
                    }
                }
            }
        }
        draft_files.sort();

        self.draft_list = vec![None];
        for f in draft_files {
            self.draft_list.push(Some(f));
        }

        self.draft_index = 0;
        self.draft_ngl = String::new();

        if preset_name.to_lowercase().contains("draft") {
            if let Some(active_draft) = ini_settings.remove("model-draft") {
                let active_name = Path::new(&active_draft).file_name().unwrap_or_default();
                for (idx, opt) in self.draft_list.iter().enumerate() {
                    if let Some(path) = opt
                        && path.file_name().unwrap_or_default() == active_name
                    {
                        self.draft_index = idx;
                        break;
                    }
                }
                self.draft_ngl = ini_settings
                    .remove("gpu-layers-draft")
                    .unwrap_or_else(|| "auto".to_owned());
            } else {
                // Automatically select draft if discovered by heuristic
                let heuristic_draft = crate::discovery::find_matching_draft(
                    model_path,
                    &self
                        .draft_list
                        .iter()
                        .filter_map(Clone::clone)
                        .collect::<Vec<_>>(),
                );
                if let Some(hd) = heuristic_draft {
                    for (idx, opt) in self.draft_list.iter().enumerate() {
                        if let Some(path) = opt
                            && path == &hd
                        {
                            self.draft_index = idx;
                            break;
                        }
                    }
                    "auto".clone_into(&mut self.draft_ngl);
                }
            }
        }

        let toml_path = toml_path_override
            .unwrap_or_else(|| crate::config::resolve_toml_path(model_path, &self.models_dir));
        let config_file_name = toml_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.toml")
            .to_owned();

        let config = crate::config::load_toml_silent(&toml_path);
        let get_long_val = |key: &str| -> Option<&serde_json::Value> {
            config
                .get("llama-server-long")
                .and_then(|l| l.get(key))
                .or_else(|| config.get(key))
        };
        let get_string_val = |v: Option<&serde_json::Value>| -> String {
            match v {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Number(n)) => n.to_string(),
                Some(serde_json::Value::Bool(b)) => b.to_string(),
                _ => String::new(),
            }
        };

        // Context representation
        let ctx_str_val = match &ctx_val {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => "131072".to_owned(),
        };

        let temp_val = get_string_val(get_long_val("temp"));
        let top_p_val = get_string_val(get_long_val("top-p"));
        let top_k_val = get_string_val(get_long_val("top-k"));
        let min_p_val = get_string_val(get_long_val("min-p"));
        let repeat_penalty_val = get_string_val(get_long_val("repeat-penalty"));
        let repeat_last_n_val = get_string_val(get_long_val("repeat-last-n"));
        let reasoning_val = get_string_val(get_long_val("reasoning"));
        let reasoning_format_val = get_string_val(get_long_val("reasoning-format"));
        let reasoning_budget_val = get_string_val(get_long_val("reasoning-budget"));

        self.config_file_name = config_file_name.clone();
        self.original_config_file_name = config_file_name;

        self.ctx_str = ctx_str_val.clone();
        self.original_ctx_str = ctx_str_val;
        self.original_ctx = self.ctx;

        self.original_ngl = self.ngl.clone();

        self.original_mmproj_index = self.mmproj_index;
        self.original_draft_index = self.draft_index;
        self.original_draft_ngl = self.draft_ngl.clone();

        self.temp = temp_val.clone();
        self.original_temp = temp_val;

        self.top_p = top_p_val.clone();
        self.original_top_p = top_p_val;

        self.top_k = top_k_val.clone();
        self.original_top_k = top_k_val;

        self.min_p = min_p_val.clone();
        self.original_min_p = min_p_val;

        self.repeat_penalty = repeat_penalty_val.clone();
        self.original_repeat_penalty = repeat_penalty_val;

        self.repeat_last_n = repeat_last_n_val.clone();
        self.original_repeat_last_n = repeat_last_n_val;

        self.reasoning.clone_from(&reasoning_val);
        self.original_reasoning.clone_from(&reasoning_val);
        self.reasoning_index = self
            .reasoning_list
            .iter()
            .position(|r| r == &reasoning_val)
            .unwrap_or(0);
        self.original_reasoning_index = self.reasoning_index;

        self.reasoning_format.clone_from(&reasoning_format_val);
        self.original_reasoning_format
            .clone_from(&reasoning_format_val);
        self.reasoning_format_index = self
            .reasoning_format_list
            .iter()
            .position(|r| r == &reasoning_format_val)
            .unwrap_or(0);
        self.original_reasoning_format_index = self.reasoning_format_index;

        self.reasoning_budget = reasoning_budget_val.clone();
        self.original_reasoning_budget = reasoning_budget_val;

        self.original_total_layers = self.total_layers;
    }

    /// Saves the current configuration of the selected preset, optionally generating a backup file first.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if the configuration cannot be saved or if the backup file cannot be created.
    #[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
    pub fn save_current_preset_config(
        &mut self,
        create_backup: bool,
    ) -> Result<(), std::io::Error> {
        self.models_dir_changed_dirty = false;
        let filename = self.config_file_name.clone();
        if filename.trim().is_empty() {
            return Err(std::io::Error::other("Config file name cannot be empty"));
        }
        let filename = if filename.to_lowercase().ends_with(".toml") {
            filename
        } else {
            format!("{filename}.toml")
        };

        let target_path = self.models_dir.join(&filename);

        if create_backup && target_path.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let backup_path = self.models_dir.join(format!("{filename}.bak.{timestamp}"));
            let _ = std::fs::copy(&target_path, &backup_path);
        }

        let mut current_config = crate::config::load_toml_silent(&target_path);

        let mut herd_obj = current_config
            .remove("llama-herd")
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        let mut long_obj = current_config
            .remove("llama-server-long")
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        // 1. ctx-size
        if self.ctx_str.is_empty() {
            long_obj.remove("ctx-size");
        } else if let Ok(num) = self.ctx_str.parse::<i64>() {
            long_obj.insert("ctx-size".to_owned(), serde_json::Value::Number(num.into()));
        } else {
            long_obj.insert(
                "ctx-size".to_owned(),
                serde_json::Value::String(self.ctx_str.clone()),
            );
        }

        // 2. ngl
        if self.ngl.is_empty() {
            long_obj.remove("ngl");
        } else if let Ok(num) = self.ngl.parse::<i64>() {
            long_obj.insert("ngl".to_owned(), serde_json::Value::Number(num.into()));
        } else {
            long_obj.insert(
                "ngl".to_owned(),
                serde_json::Value::String(self.ngl.clone()),
            );
        }

        // 3. temp
        update_float_setting(&mut long_obj, "temp", &self.temp);

        // 4. top-p
        update_float_setting(&mut long_obj, "top-p", &self.top_p);

        // 5. top-k
        if self.top_k.is_empty() {
            long_obj.remove("top-k");
        } else if let Ok(num) = self.top_k.parse::<i64>() {
            long_obj.insert("top-k".to_owned(), serde_json::Value::Number(num.into()));
        }

        // min-p
        update_float_setting(&mut long_obj, "min-p", &self.min_p);

        // repeat-penalty
        update_float_setting(&mut long_obj, "repeat-penalty", &self.repeat_penalty);

        // repeat-last-n
        if self.repeat_last_n.is_empty() {
            long_obj.remove("repeat-last-n");
        } else if let Ok(num) = self.repeat_last_n.parse::<i64>() {
            long_obj.insert(
                "repeat-last-n".to_owned(),
                serde_json::Value::Number(num.into()),
            );
        }

        // reasoning
        if self.reasoning.is_empty() {
            long_obj.remove("reasoning");
        } else {
            long_obj.insert(
                "reasoning".to_owned(),
                serde_json::Value::String(self.reasoning.clone()),
            );
        }

        // reasoning-format
        if self.reasoning_format.is_empty() {
            long_obj.remove("reasoning-format");
        } else {
            long_obj.insert(
                "reasoning-format".to_owned(),
                serde_json::Value::String(self.reasoning_format.clone()),
            );
        }

        // reasoning-budget
        if self.reasoning_budget.is_empty() {
            long_obj.remove("reasoning-budget");
        } else if let Ok(num) = self.reasoning_budget.parse::<i64>() {
            long_obj.insert(
                "reasoning-budget".to_owned(),
                serde_json::Value::Number(num.into()),
            );
        }

        // 6. total-layers
        if let Some(num) = self.total_layers {
            herd_obj.insert(
                "total-layers".to_owned(),
                serde_json::Value::Number(
                    #[allow(clippy::cast_possible_wrap)]
                    {
                        (num as i64).into()
                    },
                ),
            );
        } else {
            herd_obj.remove("total-layers");
        }

        // 7. draft
        let draft_val = match self.draft_list.get(self.draft_index) {
            Some(Some(path)) => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            _ => String::new(),
        };
        if !draft_val.is_empty() && draft_val != "None (Disabled)" {
            herd_obj.insert("draft".to_owned(), serde_json::Value::String(draft_val));
        } else {
            herd_obj.remove("draft");
        }

        // 8. mmproj
        let mmproj_val = match self.mmproj_list.get(self.mmproj_index) {
            Some(Some(path)) => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            _ => String::new(),
        };
        if !mmproj_val.is_empty() && mmproj_val != "None (Disabled)" {
            herd_obj.insert("mmproj".to_owned(), serde_json::Value::String(mmproj_val));
        } else {
            herd_obj.remove("mmproj");
        }

        if !herd_obj.is_empty() {
            current_config.insert("llama-herd".to_owned(), serde_json::Value::Object(herd_obj));
        }
        if !long_obj.is_empty() {
            current_config.insert(
                "llama-server-long".to_owned(),
                serde_json::Value::Object(long_obj),
            );
        }

        let current_model_path = if self.presets.is_empty() {
            None
        } else {
            Some(self.presets[self.preset_index].1.clone())
        };

        crate::config::save_config(&target_path, &current_config)?;

        // Regenerate presets and reload list
        crate::discovery::generate_presets_ini(
            &self.models_dir,
            &self.preset_path,
            &self.global_config,
        )?;
        self.presets = crate::discovery::discover_presets_from_ini(&self.preset_path);

        if let Some(ref model_path) = current_model_path {
            if let Some(idx) = self.presets.iter().position(|(_, path)| path == model_path) {
                self.preset_index = idx;
            } else {
                self.preset_index = 0;
            }
        } else {
            self.preset_index = 0;
        }

        self.load_current_preset_settings(Some(target_path));
        Ok(())
    }

    /// Resolves and returns the customized user settings for model deployment.
    #[must_use]
    pub fn get_user_settings(&self) -> UserSettings {
        UserSettings {
            ctx: self.ctx,
            ngl: self.ngl.clone(),
            mmproj: self.mmproj_list.get(self.mmproj_index).cloned().flatten(),
            draft_model: self.draft_list.get(self.draft_index).cloned().flatten(),
            draft_ngl: self.draft_ngl.clone(),
        }
    }

    /// Checks if there are unsaved parameter configuration overrides in the dashboard.
    #[must_use]
    pub fn has_unsaved_changes(&self) -> bool {
        self.ctx_str != self.original_ctx_str
            || self.ngl != self.original_ngl
            || self.mmproj_index != self.original_mmproj_index
            || self.draft_index != self.original_draft_index
            || self.draft_ngl != self.original_draft_ngl
            || self.temp != self.original_temp
            || self.top_p != self.original_top_p
            || self.top_k != self.original_top_k
            || self.total_layers != self.original_total_layers
            || self.config_file_name != self.original_config_file_name
            || self.min_p != self.original_min_p
            || self.repeat_penalty != self.original_repeat_penalty
            || self.repeat_last_n != self.original_repeat_last_n
            || self.reasoning != self.original_reasoning
            || self.reasoning_format != self.original_reasoning_format
            || self.reasoning_budget != self.original_reasoning_budget
    }

    /// Periodically scans the models directory in the background and invalidates/updates presets when file changes settle.
    pub fn check_models_dir_changes(&mut self) {
        if self.models_dir_changed_dirty && !self.has_unsaved_changes() {
            self.models_dir_changed_dirty = false;
            self.load_current_preset_settings(None);
        }

        if let Ok(_entries) = std::fs::read_dir(&self.models_dir) {
            self.models_dir_invalid = false;
        } else {
            self.models_dir_invalid = true;
            return;
        }

        let current_state = get_models_dir_state(&self.models_dir);
        if let Some(new_state) = current_state {
            if let Some(ref prev_state) = self.last_models_dir_state {
                // Check if the directory is stable (i.e. no file sizes or mtimes changed since the last check)
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
                        // Brand new file in this check interval - wait for next tick to see if it stabilizes
                        is_stable = false;
                    }
                }

                if is_stable {
                    if let Some(ref stable_state) = self.last_stable_models_dir_state {
                        if stable_state != &new_state {
                            if self.has_unsaved_changes() {
                                self.models_dir_changed_dirty = true;
                            }
                            let _ = self.regenerate_and_reload_presets();
                            self.last_stable_models_dir_state = Some(new_state.clone());
                        }
                    } else {
                        self.last_stable_models_dir_state = Some(new_state.clone());
                    }
                }
            }
            self.last_models_dir_state = Some(new_state);
        }
    }

    /// Regenerates the presets INI file and reloads the active list of presets into TUI memory.
    #[allow(clippy::missing_errors_doc)]
    pub fn regenerate_and_reload_presets(&mut self) -> Result<(), std::io::Error> {
        let current_preset_name = if self.presets.is_empty() {
            None
        } else {
            Some(self.presets[self.preset_index].0.clone())
        };

        crate::discovery::generate_presets_ini(
            &self.models_dir,
            &self.preset_path,
            &self.global_config,
        )?;

        let new_presets = crate::discovery::discover_presets_from_ini(&self.preset_path);
        self.presets = new_presets;

        let mut should_reload = true;
        if let Some(ref name) = current_preset_name {
            if let Some(pos) = self.presets.iter().position(|(p_name, _)| p_name == name) {
                self.preset_index = pos;
                if self.has_unsaved_changes() {
                    should_reload = false;
                }
            } else {
                self.preset_index = 0;
            }
        } else {
            self.preset_index = 0;
        }

        if should_reload {
            self.load_current_preset_settings(None);
        }
        Ok(())
    }
}

fn update_float_setting(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    val_str: &str,
) {
    if !val_str.is_empty()
        && let Ok(f) = val_str.parse::<f64>()
        && let Some(num) = serde_json::Number::from_f64(f)
    {
        obj.insert(key.to_owned(), serde_json::Value::Number(num));
        return;
    }
    obj.remove(key);
}
