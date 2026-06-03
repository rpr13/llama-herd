use crate::config::UserSettings;
use crate::tui::logs::ActiveServer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AppScreen {
    Dashboard,
    Settings,
    Logs,
    EditingCtx,
    EditingNgl,
    EditingDraftNgl,
    PickingServerPath,
    PickingModelsDir,
    EditingGlobalSetting,
    SelectingGlobalSettingOption,
    SelectingMMProj,
    SelectingDraftModel,
    EditingTemp,
    EditingTopP,
    EditingTopK,
    EditingTotalLayers,
    EditingConfigFileName,
    ConfirmSaveConfig,
    WarnDiscardChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardFocus {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelsDirState {
    pub files: Vec<(PathBuf, std::time::SystemTime, u64)>,
}

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

pub struct AppState {
    pub dashboard_focus: DashboardFocus,
    pub dashboard_param_index: usize,
    pub presets: Vec<(String, PathBuf)>,
    pub models_dir: PathBuf,
    pub preset_path: PathBuf,
    pub global_config: HashMap<String, serde_json::Value>,
    pub server_exe: PathBuf,
    pub server_version: String,

    // UI state
    pub screen: AppScreen,
    pub active_tab: usize,
    pub preset_index: usize,
    pub settings_index: usize,
    pub picker: Option<crate::tui::picker::FilePicker>,

    // Config items for selected preset
    pub ctx: usize,
    pub ngl: String,
    pub total_layers: Option<usize>,

    // Dropdowns / Cycles
    pub mmproj_list: Vec<Option<PathBuf>>,
    pub mmproj_index: usize,
    pub mmproj_index_backup: usize,
    pub draft_list: Vec<Option<PathBuf>>,
    pub draft_index: usize,
    pub draft_index_backup: usize,
    pub draft_ngl: String,

    // Input prompts
    pub input_buffer: String,

    // Active running server
    pub active_server: Option<ActiveServer>,
    pub logs_paused: bool,
    pub logs_wrap: bool,
    pub paused_logs_buffer: std::collections::VecDeque<crate::tui::logs::LogLine>,
    pub log_scroll_offset: usize,
    pub log_scroll_x: usize,
    pub auto_scroll: bool,
    pub last_launch_args: Vec<String>,
    pub is_router_mode: bool,
    pub theme: crate::tui::theme::Theme,
    pub option_selector_index: usize,
    pub option_selector_list: Vec<String>,
    pub config_path: PathBuf,
    pub similar_config_files: Vec<String>,
    pub similar_config_index: Option<usize>,

    // Overrides / Editable parameters
    pub temp: String,
    pub top_p: String,
    pub top_k: String,
    pub config_file_name: String,
    pub ctx_str: String,
    pub backup_config: bool,
    pub pending_preset_index: Option<usize>,

    // Original values for comparison / diff
    pub original_ctx_str: String,
    pub original_ctx: usize,
    pub original_ngl: String,
    pub original_mmproj_index: usize,
    pub original_draft_index: usize,
    pub original_draft_ngl: String,
    pub original_temp: String,
    pub original_top_p: String,
    pub original_top_k: String,
    pub original_total_layers: Option<usize>,
    pub original_config_file_name: String,
    pub tick_count: u64,
    pub last_models_dir_state: Option<ModelsDirState>,
    pub last_stable_models_dir_state: Option<ModelsDirState>,
    pub models_dir_changed_dirty: bool,
    pub models_dir_invalid: bool,
}

impl AppState {
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

        let mut state = AppState {
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
            ctx: 131072,
            ngl: "auto".to_string(),
            total_layers: None,
            mmproj_list: vec![None],
            mmproj_index: 0,
            mmproj_index_backup: 0,
            draft_list: vec![None],
            draft_index: 0,
            draft_index_backup: 0,
            draft_ngl: "".to_string(),
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
            original_ctx_str: String::new(),
            original_ctx: 131072,
            original_ngl: "auto".to_string(),
            original_mmproj_index: 0,
            original_draft_index: 0,
            original_draft_ngl: "".to_string(),
            original_temp: String::new(),
            original_top_p: String::new(),
            original_top_k: String::new(),
            original_total_layers: None,
            original_config_file_name: String::new(),
            tick_count: 0,
            last_models_dir_state: last_models_dir_state.clone(),
            last_stable_models_dir_state: last_models_dir_state,
            models_dir_changed_dirty: false,
            models_dir_invalid,
        };

        state.load_current_preset_settings(None);
        state
    }

    pub fn load_current_preset_settings(&mut self, toml_path_override: Option<PathBuf>) {
        self.models_dir_changed_dirty = false;
        if self.presets.is_empty() {
            return;
        }

        let (preset_name, model_path) = &self.presets[self.preset_index];
        let ini_settings = crate::config::load_settings_from_ini(preset_name, &self.preset_path)
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
        let ctx_val = ini_settings
            .get("ctx-size")
            .map(|s| serde_json::Value::String(s.clone()))
            .unwrap_or_else(|| {
                get_lh_val("ctx-size")
                    .or_else(|| get_long_val("ctx-size"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::String("131072".to_string()))
            });
        self.ctx = crate::config::parse_ctx(&ctx_val).unwrap_or(131072);

        // NGL
        self.ngl = ini_settings
            .get("n-gpu-layers")
            .cloned()
            .unwrap_or_else(|| {
                get_lh_val("ngl")
                    .or_else(|| get_long_val("ngl"))
                    .and_then(|v| {
                        if let Some(s) = v.as_str() {
                            Some(s.to_string())
                        } else {
                            v.as_i64().map(|i| i.to_string())
                        }
                    })
                    .unwrap_or_else(|| {
                        total_layers
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "auto".to_string())
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
        if let Some(active_mmproj) = ini_settings.get("mmproj") {
            let active_name = Path::new(active_mmproj).file_name().unwrap_or_default();
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
                                        .and_then(|v| v.as_bool())
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
        self.draft_ngl = "".to_string();

        if preset_name.to_lowercase().contains("draft") {
            if let Some(active_draft) = ini_settings.get("model-draft") {
                let active_name = Path::new(active_draft).file_name().unwrap_or_default();
                for (idx, opt) in self.draft_list.iter().enumerate() {
                    if let Some(path) = opt
                        && path.file_name().unwrap_or_default() == active_name
                    {
                        self.draft_index = idx;
                        break;
                    }
                }
                self.draft_ngl = ini_settings
                    .get("gpu-layers-draft")
                    .cloned()
                    .unwrap_or_else(|| "auto".to_string());
            } else {
                // Automatically select draft if discovered by heuristic
                let heuristic_draft = crate::discovery::find_matching_draft(
                    model_path,
                    &self
                        .draft_list
                        .iter()
                        .filter_map(|x| x.clone())
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
                    self.draft_ngl = "auto".to_string();
                }
            }
        }

        let toml_path = toml_path_override
            .unwrap_or_else(|| crate::config::resolve_toml_path(model_path, &self.models_dir));
        let config_file_name = toml_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.toml")
            .to_string();

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
            _ => "131072".to_string(),
        };

        let temp_val = get_string_val(get_long_val("temp"));
        let top_p_val = get_string_val(get_long_val("top-p"));
        let top_k_val = get_string_val(get_long_val("top-k"));

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

        self.original_total_layers = self.total_layers;
    }

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
            format!("{}.toml", filename)
        };

        let target_path = self.models_dir.join(&filename);

        if create_backup && target_path.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let backup_path = self
                .models_dir
                .join(format!("{}.bak.{}", filename, timestamp));
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
        if !self.ctx_str.is_empty() {
            if let Ok(num) = self.ctx_str.parse::<i64>() {
                long_obj.insert(
                    "ctx-size".to_string(),
                    serde_json::Value::Number(num.into()),
                );
            } else {
                long_obj.insert(
                    "ctx-size".to_string(),
                    serde_json::Value::String(self.ctx_str.clone()),
                );
            }
        } else {
            long_obj.remove("ctx-size");
        }

        // 2. ngl
        if !self.ngl.is_empty() {
            if let Ok(num) = self.ngl.parse::<i64>() {
                long_obj.insert("ngl".to_string(), serde_json::Value::Number(num.into()));
            } else {
                long_obj.insert(
                    "ngl".to_string(),
                    serde_json::Value::String(self.ngl.clone()),
                );
            }
        } else {
            long_obj.remove("ngl");
        }

        // 3. temp
        if !self.temp.is_empty() {
            if let Ok(f) = self.temp.parse::<f64>()
                && let Some(num) = serde_json::Number::from_f64(f)
            {
                long_obj.insert("temp".to_string(), serde_json::Value::Number(num));
            }
        } else {
            long_obj.remove("temp");
        }

        // 4. top-p
        if !self.top_p.is_empty() {
            if let Ok(f) = self.top_p.parse::<f64>()
                && let Some(num) = serde_json::Number::from_f64(f)
            {
                long_obj.insert("top-p".to_string(), serde_json::Value::Number(num));
            }
        } else {
            long_obj.remove("top-p");
        }

        // 5. top-k
        if !self.top_k.is_empty() {
            if let Ok(num) = self.top_k.parse::<i64>() {
                long_obj.insert("top-k".to_string(), serde_json::Value::Number(num.into()));
            }
        } else {
            long_obj.remove("top-k");
        }

        // 6. total-layers
        if let Some(num) = self.total_layers {
            herd_obj.insert(
                "total-layers".to_string(),
                serde_json::Value::Number((num as i64).into()),
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
            herd_obj.insert("draft".to_string(), serde_json::Value::String(draft_val));
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
            herd_obj.insert("mmproj".to_string(), serde_json::Value::String(mmproj_val));
        } else {
            herd_obj.remove("mmproj");
        }

        if !herd_obj.is_empty() {
            current_config.insert(
                "llama-herd".to_string(),
                serde_json::Value::Object(herd_obj),
            );
        }
        if !long_obj.is_empty() {
            current_config.insert(
                "llama-server-long".to_string(),
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

    pub fn get_user_settings(&self) -> UserSettings {
        UserSettings {
            ctx: self.ctx,
            ngl: self.ngl.clone(),
            mmproj: self.mmproj_list[self.mmproj_index].clone(),
            draft_model: self.draft_list[self.draft_index].clone(),
            draft_ngl: self.draft_ngl.clone(),
        }
    }

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
    }

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
