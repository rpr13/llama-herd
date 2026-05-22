use crate::config::UserSettings;
use crate::tui::logs::ActiveServer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AppScreen {
    Select,
    Running,
    EditingCtx,
    EditingNgl,
    EditingDraftNgl,
}

pub struct AppState {
    pub presets: Vec<(String, PathBuf)>,
    pub models_dir: PathBuf,
    pub base_dir: PathBuf,
    pub preset_path: PathBuf,
    pub global_config: HashMap<String, serde_json::Value>,
    pub server_exe: PathBuf,

    // UI state
    pub screen: AppScreen,
    pub preset_index: usize,

    // Config items for selected preset
    pub ctx: usize,
    pub ngl: String,
    pub ui: bool,

    // Dropdowns / Cycles
    pub mmproj_list: Vec<Option<PathBuf>>,
    pub mmproj_index: usize,
    pub draft_list: Vec<Option<PathBuf>>,
    pub draft_index: usize,
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
}

impl AppState {
    pub fn new(
        presets: Vec<(String, PathBuf)>,
        models_dir: PathBuf,
        base_dir: PathBuf,
        preset_path: PathBuf,
        global_config: HashMap<String, serde_json::Value>,
        server_exe: PathBuf,
    ) -> Self {
        let mut state = AppState {
            presets,
            models_dir,
            base_dir,
            preset_path,
            global_config,
            server_exe,
            screen: AppScreen::Select,
            preset_index: 0,
            ctx: 131072,
            ngl: "auto".to_string(),
            ui: true,
            mmproj_list: vec![None],
            mmproj_index: 0,
            draft_list: vec![None],
            draft_index: 0,
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
        };

        state.load_current_preset_settings();
        state
    }

    pub fn load_current_preset_settings(&mut self) {
        if self.presets.is_empty() {
            return;
        }

        let (preset_name, model_path) = &self.presets[self.preset_index];
        let ini_settings = crate::config::load_settings_from_ini(preset_name, &self.preset_path)
            .unwrap_or_default();

        let assets = crate::discovery::discover_assets(model_path, &self.models_dir);
        let total_layers = assets
            .config
            .get("total-layers")
            .and_then(|v| v.as_u64().map(|i| i as usize));

        // Context Size
        let ctx_val = ini_settings
            .get("ctx-size")
            .map(|s| serde_json::Value::String(s.clone()))
            .unwrap_or_else(|| {
                assets
                    .config
                    .get("ctx-size")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::String("131072".to_string()))
            });
        self.ctx = crate::config::parse_ctx(&ctx_val);

        // NGL
        self.ngl = ini_settings
            .get("n-gpu-layers")
            .cloned()
            .unwrap_or_else(|| {
                assets
                    .config
                    .get("ngl")
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

        // UI
        self.ui = !ini_settings.contains_key("no-ui")
            && ini_settings.get("no-ui").map(|s| s.as_str()) != Some("true");

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
                                    if cfg.get("is-draft").and_then(|v| v.as_bool()) == Some(true) {
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
            self.draft_ngl = "".to_string();
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

    pub fn get_user_settings(&self) -> UserSettings {
        UserSettings {
            ctx: self.ctx,
            ngl: self.ngl.clone(),
            ui: self.ui,
            mmproj: self.mmproj_list[self.mmproj_index].clone(),
            draft_model: self.draft_list[self.draft_index].clone(),
            draft_ngl: self.draft_ngl.clone(),
        }
    }
}
