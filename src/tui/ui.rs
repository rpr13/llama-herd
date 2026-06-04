use crate::tui::app::{AppScreen, AppState, DashboardFocus};
use crate::tui::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};

pub struct SettingItem {
    pub label: &'static str,
    pub key: &'static str,
    pub default_val: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
}

pub const SETTINGS: &[SettingItem] = &[
    SettingItem {
        label: "Llama Server Path",
        key: "llama-server",
        default_val: "(System PATH)",
        emoji: "🚀",
        description: "The absolute path to the llama-server executable. If not set, LlamaHerd will search your system PATH.",
    },
    SettingItem {
        label: "Models Directory",
        key: "models-dir",
        default_val: "./models",
        emoji: "📂",
        description: "The directory where your GGUF models are located. LlamaHerd will automatically discover models and presets in this folder.",
    },
    SettingItem {
        label: "Server Host IP",
        key: "host",
        default_val: "127.0.0.1",
        emoji: "🌐",
        description: "The host IP address that llama-server binds to. Defaults to '127.0.0.1' for local-only access.",
    },
    SettingItem {
        label: "Server Port",
        key: "port",
        default_val: "8080",
        emoji: "🔌",
        description: "The port number for llama-server. Defaults to '8080'. If set to 'auto', it binds to the first sequentially free port.",
    },
    SettingItem {
        label: "CPU Threads",
        key: "threads",
        default_val: "-1",
        emoji: "🧠",
        description: "Number of CPU threads to use for generation. Defaults to '-1' for auto-detection.",
    },
    SettingItem {
        label: "Flash Attention",
        key: "flash-attn",
        default_val: "auto",
        emoji: "⚡",
        description: "Enable flash attention for faster inference. Options: 'auto', '1' (enable), '0' (disable).",
    },
    SettingItem {
        label: "Cache Type K",
        key: "cache-type-k",
        default_val: "f16",
        emoji: "🔑",
        description: "Quantization format for the KV cache keys (e.g. 'f16', 'q8_0', 'q4_0'). Lower values save VRAM.",
    },
    SettingItem {
        label: "Cache Type V",
        key: "cache-type-v",
        default_val: "f16",
        emoji: "📦",
        description: "Quantization format for the KV cache values (e.g. 'f16', 'q8_0', 'q4_0'). Lower values save VRAM.",
    },
    SettingItem {
        label: "Unified KV Cache",
        key: "kv-unified",
        default_val: "true",
        emoji: "🔗",
        description: "Enable unified KV cache for keys and values. Maps to llama-server --kv-unified flag.",
    },
    SettingItem {
        label: "Context Checkpoints",
        key: "ctx-checkpoints",
        default_val: "32",
        emoji: "🔄",
        description: "Number of context checkpoints to keep in memory (for caching). Defaults to 32. Higher values prevent prompt re-processing on long chats.",
    },
    SettingItem {
        label: "Checkpoint Min Step",
        key: "checkpoint-min-step",
        default_val: "256",
        emoji: "📏",
        description: "Minimum spacing between context checkpoints in tokens. Defaults to 256. Lower values speed up cache matching but use more system RAM.",
    },
    SettingItem {
        label: "Disable Memory Map (no-mmap)",
        key: "no-mmap",
        default_val: "false",
        emoji: "💾",
        description: "Disable memory-mapping (mmap) for model loading, loading everything directly into physical RAM. Helps stabilize memory allocation issues.",
    },
    SettingItem {
        label: "Parallel Slots (np)",
        key: "np",
        default_val: "-1",
        emoji: "👥",
        description: "Number of parallel slots/requests to support simultaneously. Defaults to '-1' (auto).",
    },
    SettingItem {
        label: "Prompt Batch Size",
        key: "batch-size",
        default_val: "2048",
        emoji: "📊",
        description: "The logical batch size used for prompt processing. Maps to llama-server --batch-size flag.",
    },
    SettingItem {
        label: "Prompt Micro-Batch",
        key: "ubatch-size",
        default_val: "512",
        emoji: "📉",
        description: "The physical batch size used for prompt processing. Maps to llama-server --ubatch-size flag.",
    },
    SettingItem {
        label: "Max Active Models",
        key: "models-max",
        default_val: "1",
        emoji: "🔀",
        description: "The maximum number of active models loaded concurrently when running in Router Mode.",
    },
    SettingItem {
        label: "API Key",
        key: "api-key",
        default_val: "disabled",
        emoji: "🔑",
        description: "Set a static API key for server authorization to secure the HTTP endpoints. Use 'disabled' to turn off.",
    },
    SettingItem {
        label: "Enable Metrics",
        key: "metrics",
        default_val: "false",
        emoji: "📈",
        description: "Enable the /metrics Prometheus endpoint on llama-server. Maps to --metrics when enabled.",
    },
    SettingItem {
        label: "Enable Web UI",
        key: "ui",
        default_val: "true",
        emoji: "💻",
        description: "Enable/disable the built-in HTML/web chat interface provided by llama-server. Maps to --no-ui when disabled.",
    },
];

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let size = f.area();
    let theme = &state.theme;

    // --- 0. GLOBAL BACKGROUND ---
    // Force clear the buffer to prevent ghost characters from previous frames
    f.render_widget(Clear, size);
    f.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg)),
        size,
    );

    let show_dir_warning = state.models_dir_invalid;
    let show_dirty_warning = state.models_dir_changed_dirty;
    let has_warning = show_dir_warning || show_dirty_warning;

    // Global background/layout structure
    let main_layout = if has_warning {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(5),    // Content
                Constraint::Length(1), // Warning Bar
                Constraint::Length(2), // Footer / Hotkeys
            ])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(5),    // Content
                Constraint::Length(2), // Footer / Hotkeys
            ])
            .split(size)
    };

    // --- 1. HEADER PANEL ---
    render_mc_header(f, state, main_layout[0]);

    // --- 2. MAIN CONTENT AREA ---
    match state.screen {
        AppScreen::Dashboard
        | AppScreen::EditingCtx
        | AppScreen::EditingNgl
        | AppScreen::EditingDraftNgl
        | AppScreen::SelectingMMProj
        | AppScreen::SelectingDraftModel
        | AppScreen::EditingTemp
        | AppScreen::EditingTopP
        | AppScreen::EditingTopK
        | AppScreen::EditingTotalLayers
        | AppScreen::EditingConfigFileName
        | AppScreen::ConfirmSaveConfig
        | AppScreen::WarnDiscardChanges => {
            render_dashboard(f, state, main_layout[1]);
        }
        AppScreen::Settings
        | AppScreen::PickingServerPath
        | AppScreen::PickingModelsDir
        | AppScreen::EditingGlobalSetting
        | AppScreen::SelectingGlobalSettingOption => {
            render_settings_tab(f, state, main_layout[1]);
        }
        AppScreen::Logs => {
            render_logs(f, state, main_layout[1]);
        }
    }

    // --- 3. FOOTER HINTS PANEL ---
    let theme = &state.theme;
    let footer_text = match state.screen {
        AppScreen::Dashboard => {
            let config_changed = state.ctx_str != state.original_ctx_str
                || state.ngl != state.original_ngl
                || state.mmproj_index != state.original_mmproj_index
                || state.draft_index != state.original_draft_index
                || state.draft_ngl != state.original_draft_ngl
                || state.temp != state.original_temp
                || state.top_p != state.original_top_p
                || state.top_k != state.original_top_k
                || state.total_layers != state.original_total_layers
                || state.config_file_name != state.original_config_file_name;

            let mut second_line_spans = vec![
                Span::styled(
                    " [t]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Temp  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [p]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Top P  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [k]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Top K  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [l]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Layers  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [f]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Config File  ", Style::default().fg(theme.fg)),
            ];

            if config_changed {
                second_line_spans.push(Span::styled(
                    " [Ctrl+S]",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ));
                second_line_spans.push(Span::styled(
                    " Save Config  ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            second_line_spans.push(Span::styled(
                " [q]",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ));
            second_line_spans.push(Span::styled(" Quit", Style::default().fg(theme.fg)));

            let launch_preset_label = if size.width < 110 {
                " Launch  "
            } else {
                " Launch Preset  "
            };

            let launch_router_label = if size.width < 110 {
                " Router  "
            } else {
                " Launch Router  "
            };

            let context_label = if size.width < 110 {
                " Context  "
            } else {
                " Edit Context  "
            };

            let layers_label = if size.width < 110 {
                " GPU Layers  "
            } else {
                " Edit GPU Layers  "
            };

            let draft_ngl_label = if size.width < 110 {
                " Draft NGL  "
            } else {
                " Edit Draft NGL  "
            };

            let mmproj_label = if size.width < 110 {
                " MMProj  "
            } else {
                " Cycle MMProj  "
            };

            let draft_label = if size.width < 110 {
                " Draft  "
            } else {
                " Cycle Draft  "
            };

            let focus_label = if state.dashboard_focus == DashboardFocus::Left {
                " Focus Params  "
            } else {
                " Focus List  "
            };

            let mut first_line_spans = vec![
                Span::styled(
                    " [Tab]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(focus_label, Style::default().fg(theme.fg)),
            ];

            if state.dashboard_focus == DashboardFocus::Left {
                first_line_spans.extend(vec![
                    Span::styled(
                        " [Enter]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(launch_preset_label, Style::default().fg(theme.fg)),
                    Span::styled(
                        " [Ctrl+R]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(launch_router_label, Style::default().fg(theme.fg)),
                    Span::styled(
                        " [c]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(context_label, Style::default().fg(theme.fg)),
                    Span::styled(
                        " [n]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(layers_label, Style::default().fg(theme.fg)),
                    Span::styled(
                        " [v]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(mmproj_label, Style::default().fg(theme.fg)),
                    Span::styled(
                        " [d]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(draft_label, Style::default().fg(theme.fg)),
                    Span::styled(
                        " [g]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(draft_ngl_label, Style::default().fg(theme.fg)),
                ]);
            } else {
                first_line_spans.extend(vec![
                    Span::styled(
                        " [Up/Down]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Select Param  ", Style::default().fg(theme.fg)),
                    Span::styled(
                        " [Enter]",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Edit Selected  ", Style::default().fg(theme.fg)),
                ]);
            }

            vec![Line::from(first_line_spans), Line::from(second_line_spans)]
        }
        AppScreen::Settings => {
            vec![Line::from(vec![
                Span::styled(
                    " [Up/Down]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Navigate  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Enter]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Edit Setting  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Tab]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Switch Tabs  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [q]",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Quit", Style::default().fg(theme.fg)),
            ])]
        }
        AppScreen::PickingServerPath | AppScreen::PickingModelsDir => {
            vec![Line::from(vec![
                Span::styled(
                    " [Up/Down]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Navigate  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Enter]",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Select  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Esc]",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Cancel  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Backspace]",
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Parent Dir ", Style::default().fg(theme.fg)),
            ])]
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
        | AppScreen::SelectingGlobalSettingOption
        | AppScreen::SelectingMMProj
        | AppScreen::SelectingDraftModel => {
            vec![Line::from(vec![
                Span::styled(
                    " [Enter]",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Save Setting  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Esc]",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Cancel and Go Back", Style::default().fg(theme.fg)),
            ])]
        }
        AppScreen::ConfirmSaveConfig => {
            vec![Line::from(vec![
                Span::styled(
                    " [Space]",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Toggle Backup  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Enter]",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Save Config  ", Style::default().fg(theme.fg)),
                Span::styled(
                    " [Esc]",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Cancel and Go Back", Style::default().fg(theme.fg)),
            ])]
        }
        AppScreen::WarnDiscardChanges => {
            vec![Line::from(vec![
                Span::styled(
                    " [y] / [Enter]",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " Discard changes & switch preset  ",
                    Style::default().fg(theme.fg),
                ),
                Span::styled(
                    " [n] / [Esc]",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " Keep current preset & edit parameters",
                    Style::default().fg(theme.fg),
                ),
            ])]
        }
        AppScreen::Logs => {
            if size.width < 110 {
                vec![
                    Line::from(vec![
                        Span::styled(
                            " [r]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Restart  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [p]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Pause  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [c]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Copy  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [w]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Wrap  ", Style::default().fg(theme.fg)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " [Up/Dn/PgUp/PgDn]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll V  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [Left/Right]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll H  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [s]",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Stop  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [q]",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Quit", Style::default().fg(theme.fg)),
                    ]),
                ]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled(
                            " [r]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Restart Server  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [p]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Pause Logs  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [c]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Copy Logs  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [w]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Toggle Wrap  ", Style::default().fg(theme.fg)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " [Up/Down/PgUp/PgDn]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll Logs Vertically  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [Left/Right]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll Horizontally  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [s]",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Stop Server  ", Style::default().fg(theme.fg)),
                        Span::styled(
                            " [q]",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Quit", Style::default().fg(theme.fg)),
                    ]),
                ]
            }
        }
    };

    let footer_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(theme.footer_bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.primary));
    let footer = Paragraph::new(footer_text)
        .block(footer_block)
        .alignment(Alignment::Center)
        .style(Style::default().bg(theme.footer_bg));
    if show_dir_warning || show_dirty_warning {
        let warning_text = if show_dir_warning {
            if theme.show_emojis {
                "⚠️  WARNING: Models directory is invalid or inaccessible!"
            } else {
                "WARNING: Models directory is invalid or inaccessible!"
            }
        } else {
            if theme.show_emojis {
                "⚠️  NOTICE: Models folder changed (unsaved edits override automatic reload)"
            } else {
                "NOTICE: Models folder changed (unsaved edits override automatic reload)"
            }
        };
        let warning_paragraph = Paragraph::new(Span::styled(
            warning_text,
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .style(Style::default().bg(theme.header_bg));
        f.render_widget(warning_paragraph, main_layout[2]);
    }

    f.render_widget(footer, main_layout[main_layout.len() - 1]);
}

fn render_mc_header(f: &mut Frame, state: &AppState, area: Rect) {
    let theme = &state.theme;
    let logo_full = if theme.show_emojis {
        " 🦙 LlamaHerd "
    } else {
        " LlamaHerd "
    };
    let logo_short = if theme.show_emojis { " 🦙 " } else { " LH " };

    let version_str = if area.width >= 90
        && !state.server_version.is_empty()
        && state.server_version != "Unknown"
    {
        format!("{} (core: {}) ", env!("APP_VERSION"), state.server_version)
    } else {
        format!("{} ", env!("APP_VERSION"))
    };

    // Fill the background of the entire header area
    f.render_widget(
        Block::default().style(Style::default().bg(theme.header_bg).fg(theme.fg)),
        area,
    );

    // Higher breakpoints to ensure center tabs (40 chars) have enough room
    let show_full_logo = area.width >= 75;
    let show_version = area.width >= 55;

    let logo_len = if show_full_logo {
        logo_full.len() as u16
    } else {
        logo_short.len() as u16
    };
    let version_len = if show_version {
        version_str.len() as u16
    } else {
        0
    };

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(logo_len),
            Constraint::Min(10),
            Constraint::Length(version_len),
        ])
        .split(area);

    // --- LEFT: LOGO ---
    let logo_str = if show_full_logo {
        logo_full
    } else {
        logo_short
    };
    let logo = Span::styled(
        logo_str,
        Style::default()
            .fg(theme.primary)
            .bg(theme.header_bg)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(Paragraph::new(Line::from(logo)), header_layout[0]);

    // --- CENTER: TABS ---
    let tabs_text = if area.width >= 75 {
        vec![
            Span::styled(
                "[ ",
                Style::default().fg(theme.secondary).bg(theme.header_bg),
            ),
            render_header_tab(
                if theme.show_emojis {
                    "[1] 📊 Dashboard"
                } else {
                    "[1] Dashboard"
                },
                state.active_tab == 0,
                theme,
            ),
            Span::styled(
                " | ",
                Style::default().fg(theme.secondary).bg(theme.header_bg),
            ),
            render_header_tab(
                if theme.show_emojis {
                    "[2] ⚙ Settings"
                } else {
                    "[2] Settings"
                },
                state.active_tab == 1,
                theme,
            ),
            Span::styled(
                " | ",
                Style::default().fg(theme.secondary).bg(theme.header_bg),
            ),
            render_header_tab(
                if theme.show_emojis {
                    "[3] 📜 Logs"
                } else {
                    "[3] Logs"
                },
                state.active_tab == 2,
                theme,
            ),
            Span::styled(
                " ]",
                Style::default().fg(theme.secondary).bg(theme.header_bg),
            ),
        ]
    } else {
        vec![
            render_header_tab(
                if area.width >= 45 {
                    if theme.show_emojis {
                        "[1] 📊 Dash"
                    } else {
                        "[1] Dash"
                    }
                } else if theme.show_emojis {
                    "[1] 📊"
                } else {
                    "[1] D"
                },
                state.active_tab == 0,
                theme,
            ),
            Span::styled("  ", Style::default().bg(theme.header_bg)),
            render_header_tab(
                if area.width >= 45 {
                    if theme.show_emojis {
                        "[2] ⚙ Set"
                    } else {
                        "[2] Set"
                    }
                } else if theme.show_emojis {
                    "[2] ⚙"
                } else {
                    "[2] S"
                },
                state.active_tab == 1,
                theme,
            ),
            Span::styled("  ", Style::default().bg(theme.header_bg)),
            render_header_tab(
                if area.width >= 45 {
                    if theme.show_emojis {
                        "[3] 📜 Logs"
                    } else {
                        "[3] Logs"
                    }
                } else if theme.show_emojis {
                    "[3] 📜"
                } else {
                    "[3] L"
                },
                state.active_tab == 2,
                theme,
            ),
        ]
    };
    let tabs = Paragraph::new(Line::from(tabs_text))
        .style(Style::default().bg(theme.header_bg))
        .alignment(Alignment::Center);
    f.render_widget(tabs, header_layout[1]);

    // --- RIGHT: VERSION ---
    if show_version {
        let version = Span::styled(
            version_str,
            Style::default().fg(theme.secondary).bg(theme.header_bg),
        );
        let version_para = Paragraph::new(Line::from(version))
            .style(Style::default().bg(theme.header_bg))
            .alignment(Alignment::Right);
        f.render_widget(version_para, header_layout[2]);
    }
}

fn render_header_tab<'a>(title: &'a str, is_active: bool, theme: &Theme) -> Span<'a> {
    if is_active {
        Span::styled(
            title,
            Style::default()
                .fg(theme.primary)
                .bg(theme.header_bg)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            title,
            Style::default().fg(theme.secondary).bg(theme.header_bg),
        )
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn truncate_middle(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return "...".to_string();
    }
    let keep = (max_len - 3) / 2;
    let start: String = chars[..keep].iter().collect();
    let end: String = chars[chars.len() - (max_len - 3 - keep)..].iter().collect();
    format!("{}...{}", start, end)
}

fn render_settings_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let theme = &state.theme;
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .title(" Global Settings ")
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.primary));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split Content Area into Left (Settings Table) and Right (Details Pane)
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner_area);

    let mut rows = Vec::new();
    for (idx, item) in SETTINGS.iter().enumerate() {
        let is_selected = idx == state.settings_index;

        let label_str = if theme.show_emojis {
            format!("{} {}", item.emoji, item.label)
        } else {
            item.label.to_string()
        };

        let val_str = match idx {
            0 => state.server_exe.to_string_lossy().to_string(),
            1 => state.models_dir.to_string_lossy().to_string(),
            _ => crate::config::get_global_config_string(
                &state.global_config,
                item.key,
                item.default_val,
            ),
        };

        let truncate_len = (content_layout[0].width as usize)
            .saturating_sub(30)
            .max(10);
        let display_val = truncate_middle(&val_str, truncate_len);

        let cell_indicator = if is_selected {
            Cell::from(" ➤ ").style(
                Style::default()
                    .fg(theme.selection)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Cell::from("   ")
        };

        let cell_label = Cell::from(label_str).style(if is_selected {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg)
        });

        let cell_value = Cell::from(display_val).style(if is_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.secondary)
        });

        rows.push(Row::new(vec![cell_indicator, cell_label, cell_value]));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(25),
            Constraint::Min(10),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.secondary))
            .border_type(theme.border_type),
    );
    f.render_widget(table, content_layout[0]);

    // Right Column: Details Card
    let selected_item = &SETTINGS[state.settings_index];
    let selected_val = match selected_item.key {
        "llama-server" => state.server_exe.to_string_lossy().to_string(),
        "models-dir" => state.models_dir.to_string_lossy().to_string(),
        _ => crate::config::get_global_config_string(
            &state.global_config,
            selected_item.key,
            selected_item.default_val,
        ),
    };

    let detail_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    let detail_area = detail_block.inner(content_layout[1]);
    f.render_widget(detail_block, content_layout[1]);

    let detail_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title
            Constraint::Length(1), // Key
            Constraint::Length(1), // Default
            Constraint::Length(1), // Current Value
            Constraint::Length(1), // Spacer
            Constraint::Min(5),    // Description
        ])
        .split(detail_area);

    let title_span = Span::styled(
        if theme.show_emojis {
            format!("{} {}", selected_item.emoji, selected_item.label)
        } else {
            selected_item.label.to_string()
        },
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(Paragraph::new(Line::from(title_span)), detail_layout[0]);

    let key_line = Line::from(vec![
        Span::styled(" TOML Key:      ", Style::default().fg(theme.secondary)),
        Span::styled(selected_item.key, Style::default().fg(theme.accent)),
    ]);
    f.render_widget(Paragraph::new(key_line), detail_layout[1]);

    let default_line = Line::from(vec![
        Span::styled(" Default Value: ", Style::default().fg(theme.secondary)),
        Span::styled(selected_item.default_val, Style::default().fg(theme.fg)),
    ]);
    f.render_widget(Paragraph::new(default_line), detail_layout[2]);

    let val_line = Line::from(vec![
        Span::styled(" Current Value: ", Style::default().fg(theme.secondary)),
        Span::styled(
            truncate_middle(
                &selected_val,
                (detail_layout[3].width as usize).saturating_sub(18).max(10),
            ),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(val_line), detail_layout[3]);

    let desc_para = Paragraph::new(selected_item.description)
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::TOP)
                .title(" Description ")
                .border_style(Style::default().fg(theme.secondary))
                .border_type(theme.border_type),
        );
    f.render_widget(desc_para, detail_layout[5]);

    // Render Picker Modal if active
    if let Some(picker) = &state.picker {
        let popup_area = centered_rect(80, 80, f.area());
        f.render_widget(Clear, popup_area);
        picker.render(f, popup_area, theme);
    }

    // Render text input popup overlay for EditingGlobalSetting
    if state.screen == AppScreen::EditingGlobalSetting {
        let title = format!(" Edit {} ", selected_item.label);
        let prompt = format!(
            "Enter new value for {} (Default: {}):",
            selected_item.label, selected_item.default_val
        );

        let popup_area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, popup_area); // clears the background

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .title(title)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.selection));

        let popup_text = vec![
            Line::from(prompt),
            Line::from(""),
            Line::from(vec![
                Span::styled(&state.input_buffer, Style::default().fg(theme.fg)),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(theme.selection)
                        .add_modifier(Modifier::RAPID_BLINK),
                ),
            ]),
        ];

        let popup_para = Paragraph::new(popup_text)
            .block(popup_block)
            .alignment(Alignment::Center);
        f.render_widget(popup_para, popup_area);
    }

    // Render option list selector popup overlay for SelectingGlobalSettingOption
    if state.screen == AppScreen::SelectingGlobalSettingOption {
        let title = format!(" Select {} ", selected_item.label);
        let popup_area = centered_rect(50, 40, f.area());
        f.render_widget(Clear, popup_area); // clears the background

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .title(title)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.selection));

        // Draw items as paragraph spans
        let mut list_spans = Vec::new();
        list_spans.push(Line::from(""));
        for (i, opt) in state.option_selector_list.iter().enumerate() {
            if i == state.option_selector_index {
                list_spans.push(Line::from(vec![
                    Span::styled(
                        " 👉 ",
                        Style::default()
                            .fg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        opt,
                        Style::default()
                            .fg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                list_spans.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(opt, Style::default().fg(theme.fg)),
                ]));
            }
        }
        list_spans.push(Line::from(""));

        let popup_para = Paragraph::new(list_spans)
            .block(popup_block)
            .alignment(Alignment::Left);
        f.render_widget(popup_para, popup_area);
    }
}

fn render_dashboard(f: &mut Frame, state: &mut AppState, area: Rect) {
    let theme = &state.theme;
    let size = f.area();
    // Split Content Area into Left (Presets List) and Right (Preset Parameters Details)
    let content_layout = if size.width < 110 {
        let presets_height = (state.presets.len() as u16 + 2).clamp(5, 8);
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(presets_height), Constraint::Min(5)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(area)
    };

    // LEFT Panel: Presets List
    let list_val_width = content_layout[0].width.saturating_sub(6) as usize;
    let is_left_focused = state.dashboard_focus == DashboardFocus::Left;
    let items: Vec<ListItem> = state
        .presets
        .iter()
        .enumerate()
        .map(|(idx, (name, _))| {
            let display_name = truncate_middle(name, list_val_width);
            if idx == state.preset_index {
                if is_left_focused {
                    ListItem::new(format!(" ➤ {} ", display_name)).style(
                        Style::default()
                            .fg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(format!(" • {} ", display_name)).style(
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    )
                }
            } else {
                ListItem::new(format!("   {} ", display_name)).style(Style::default().fg(theme.fg))
            }
        })
        .collect();

    let presets_borders = if size.width < 110 {
        Borders::TOP | Borders::BOTTOM
    } else {
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM
    };

    let presets_border_color = if state.dashboard_focus == DashboardFocus::Left {
        theme.primary
    } else {
        theme.secondary
    };

    let presets_block = Block::default()
        .borders(presets_borders)
        .title(" Presets ")
        .border_type(theme.border_type)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(presets_border_color));
    let presets_list = List::new(items).block(presets_block);
    f.render_widget(presets_list, content_layout[0]);

    // RIGHT Panel: Parameters Details
    let config_changed = state.ctx_str != state.original_ctx_str
        || state.ngl != state.original_ngl
        || state.mmproj_index != state.original_mmproj_index
        || state.draft_index != state.original_draft_index
        || state.draft_ngl != state.original_draft_ngl
        || state.temp != state.original_temp
        || state.top_p != state.original_top_p
        || state.top_k != state.original_top_k
        || state.total_layers != state.original_total_layers
        || state.config_file_name != state.original_config_file_name;

    let right_title = if config_changed {
        " Preset Details & Parameters (Unsaved Changes: Ctrl+S to save) ".to_string()
    } else {
        " Preset Details & Parameters ".to_string()
    };

    let right_border_color = if state.dashboard_focus == DashboardFocus::Right {
        theme.primary
    } else {
        theme.secondary
    };

    let right_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .title(right_title)
        .border_type(theme.border_type)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(right_border_color));

    let preset_name = if state.presets.is_empty() {
        "None".to_string()
    } else {
        state.presets[state.preset_index].0.clone()
    };

    let model_name = if state.presets.is_empty() {
        "None".to_string()
    } else {
        state.presets[state.preset_index]
            .1
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    let mmproj_val = match &state.mmproj_list[state.mmproj_index] {
        Some(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "None".to_string()),
        None => "None (Disabled)".to_string(),
    };
    let original_mmproj_val = match &state.mmproj_list[state.original_mmproj_index] {
        Some(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "None".to_string()),
        None => "None (Disabled)".to_string(),
    };

    let draft_val = match &state.draft_list[state.draft_index] {
        Some(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "None".to_string()),
        None => "None (Disabled)".to_string(),
    };
    let original_draft_val = match &state.draft_list[state.original_draft_index] {
        Some(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "None".to_string()),
        None => "None (Disabled)".to_string(),
    };

    let val_width = content_layout[1].width.saturating_sub(26) as usize;
    let display_preset_name = truncate_middle(&preset_name, val_width);
    let display_model_name = truncate_middle(&model_name, val_width);

    let make_val_cell = |orig: &str, curr: &str, default_style: Style| -> Cell<'static> {
        let display_orig = if orig.is_empty() { "none" } else { orig };
        let display_curr = if curr.is_empty() { "none" } else { curr };
        if display_orig == display_curr {
            Cell::from(Span::styled(display_curr.to_string(), default_style))
        } else {
            Cell::from(Line::from(vec![
                Span::styled(
                    display_orig.to_string(),
                    Style::default().fg(theme.secondary),
                ),
                Span::styled(
                    " -> ",
                    Style::default()
                        .fg(theme.selection)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    display_curr.to_string(),
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
        }
    };

    let make_param_cells =
        |param_idx: usize, key_char: &str, label: &str| -> (Cell<'static>, Cell<'static>) {
            let is_selected = state.dashboard_focus == DashboardFocus::Right
                && state.dashboard_param_index == param_idx;
            let is_last_selected = state.dashboard_focus != DashboardFocus::Right
                && state.dashboard_param_index == param_idx;

            let prompt_cell = if is_selected {
                Cell::from(Span::styled(
                    format!(" ➤ [{}]", key_char),
                    Style::default()
                        .fg(theme.selection)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if is_last_selected {
                Cell::from(Span::styled(
                    format!(" • [{}]", key_char),
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Cell::from(Span::styled(
                    format!("    [{}]", key_char),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ))
            };

            let label_cell = if is_selected {
                Cell::from(Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(theme.selection)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if is_last_selected {
                Cell::from(Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Cell::from(Span::styled(
                    label.to_string(),
                    Style::default().fg(theme.secondary),
                ))
            };
            (prompt_cell, label_cell)
        };

    let (f_prompt, f_label) = make_param_cells(0, "f", "Target Config File");
    let (c_prompt, c_label) = make_param_cells(1, "c", "Context Size");
    let (n_prompt, n_label) = make_param_cells(2, "n", "GPU Layers");
    let (v_prompt, v_label) = make_param_cells(3, "v", "MMProj (Vision)");
    let (d_prompt, d_label) = make_param_cells(4, "d", "Draft Model");
    let (g_prompt, g_label) = make_param_cells(5, "g", "Draft GPU Layers");
    let (t_prompt, t_label) = make_param_cells(6, "t", "Temperature");
    let (p_prompt, p_label) = make_param_cells(7, "p", "Top P");
    let (k_prompt, k_label) = make_param_cells(8, "k", "Top K");
    let (l_prompt, l_label) = make_param_cells(9, "l", "Total Layers");

    let rows = vec![
        Row::new(vec![
            Cell::from(""),
            Cell::from(Span::styled(
                "Preset Name",
                Style::default().fg(theme.secondary),
            )),
            Cell::from(display_preset_name)
                .style(Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from(""),
            Cell::from(Span::styled(
                "Model File",
                Style::default().fg(theme.secondary),
            )),
            Cell::from(display_model_name).style(Style::default().fg(theme.primary)),
        ]),
        Row::new(vec![
            f_prompt,
            f_label,
            make_val_cell(
                &state.original_config_file_name,
                &state.config_file_name,
                Style::default().fg(theme.primary),
            ),
        ]),
        Row::new(vec![
            c_prompt,
            c_label,
            make_val_cell(
                &state.original_ctx_str,
                &state.ctx_str,
                Style::default().fg(theme.success),
            ),
        ]),
        Row::new(vec![
            n_prompt,
            n_label,
            make_val_cell(
                &state.original_ngl,
                &state.ngl,
                Style::default().fg(theme.success),
            ),
        ]),
        Row::new(vec![
            v_prompt,
            v_label,
            make_val_cell(
                &original_mmproj_val,
                &mmproj_val,
                Style::default().fg(theme.accent),
            ),
        ]),
        Row::new(vec![
            d_prompt,
            d_label,
            make_val_cell(
                &original_draft_val,
                &draft_val,
                Style::default().fg(theme.accent),
            ),
        ]),
        Row::new(vec![
            g_prompt,
            g_label,
            make_val_cell(
                &state.original_draft_ngl,
                &state.draft_ngl,
                Style::default().fg(theme.accent),
            ),
        ]),
        Row::new(vec![
            t_prompt,
            t_label,
            make_val_cell(
                &state.original_temp,
                &state.temp,
                Style::default().fg(theme.success),
            ),
        ]),
        Row::new(vec![
            p_prompt,
            p_label,
            make_val_cell(
                &state.original_top_p,
                &state.top_p,
                Style::default().fg(theme.success),
            ),
        ]),
        Row::new(vec![
            k_prompt,
            k_label,
            make_val_cell(
                &state.original_top_k,
                &state.top_k,
                Style::default().fg(theme.success),
            ),
        ]),
        Row::new(vec![
            l_prompt,
            l_label,
            make_val_cell(
                &state
                    .original_total_layers
                    .map(|l| l.to_string())
                    .unwrap_or_default(),
                &state
                    .total_layers
                    .map(|l| l.to_string())
                    .unwrap_or_default(),
                Style::default().fg(theme.success),
            ),
        ]),
    ];

    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(22),
            Constraint::Min(20),
        ],
    )
    .block(right_block);
    f.render_widget(table, content_layout[1]);

    // Render Input Prompt Overlays
    if state.screen != AppScreen::Dashboard
        && state.screen != AppScreen::ConfirmSaveConfig
        && state.screen != AppScreen::WarnDiscardChanges
        && state.screen != AppScreen::SelectingMMProj
        && state.screen != AppScreen::SelectingDraftModel
    {
        let (title, prompt) = match state.screen {
            AppScreen::EditingCtx => (
                " Edit Context Size ",
                "Enter new context size (e.g. 131072, 8k, 32k):",
            ),
            AppScreen::EditingNgl => {
                let prompt = if state.total_layers.is_some() {
                    "Enter N-GPU-layers (e.g. auto, 0, 32, --4):"
                } else {
                    "Enter N-GPU-layers (e.g. auto, 0, 32):"
                };
                (" Edit GPU Layers ", prompt)
            }
            AppScreen::EditingDraftNgl => (
                " Edit Draft GPU Layers ",
                "Enter draft N-GPU-layers (e.g. auto, 0, 8):",
            ),
            AppScreen::EditingTemp => (
                " Edit Temperature ",
                "Enter new inference temperature (e.g. 0.8):",
            ),
            AppScreen::EditingTopP => (" Edit Top P ", "Enter new Top P threshold (e.g. 0.95):"),
            AppScreen::EditingTopK => (" Edit Top K ", "Enter new Top K count (e.g. 40):"),
            AppScreen::EditingTotalLayers => (
                " Edit Total Model Layers ",
                "Enter total model layers (e.g. 33):",
            ),
            AppScreen::EditingConfigFileName => (
                " Edit Target Config File ",
                "Enter config TOML filename (e.g. model.toml):",
            ),
            _ => ("", ""),
        };

        let height_pct = if state.screen == AppScreen::EditingConfigFileName {
            45
        } else {
            20
        };
        let popup_area = centered_rect(60, height_pct, area);
        f.render_widget(Clear, popup_area); // clears the background

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .title(title)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.selection));

        let mut popup_text = vec![
            Line::from(prompt),
            Line::from(""),
            Line::from(vec![
                Span::styled(&state.input_buffer, Style::default().fg(theme.fg)),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(theme.selection)
                        .add_modifier(Modifier::RAPID_BLINK),
                ),
            ]),
        ];

        if state.screen == AppScreen::EditingConfigFileName
            && !state.similar_config_files.is_empty()
        {
            popup_text.push(Line::from(""));
            popup_text.push(Line::from(Span::styled(
                "Similar Config Files:",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            popup_text.push(Line::from(""));
            for (idx, filename) in state.similar_config_files.iter().enumerate() {
                let is_selected = state.similar_config_index == Some(idx);
                if is_selected {
                    popup_text.push(Line::from(Span::styled(
                        format!(" ➤ {} ", filename),
                        Style::default()
                            .fg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    )));
                } else {
                    popup_text.push(Line::from(Span::styled(
                        format!("    {} ", filename),
                        Style::default().fg(theme.fg),
                    )));
                }
            }
            popup_text.push(Line::from(""));
            popup_text.push(Line::from(Span::styled(
                " (Use [Up/Down] to select a file) ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::ITALIC),
            )));
        }

        let popup_para = Paragraph::new(popup_text)
            .block(popup_block)
            .alignment(Alignment::Center);
        f.render_widget(popup_para, popup_area);
    }

    if state.screen == AppScreen::SelectingMMProj || state.screen == AppScreen::SelectingDraftModel
    {
        let (title, options_list, current_idx) = if state.screen == AppScreen::SelectingMMProj {
            let opts: Vec<String> = state
                .mmproj_list
                .iter()
                .map(|opt| match opt {
                    Some(p) => p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    None => "None (Disabled)".to_string(),
                })
                .collect();
            (" Select MMProj (Vision) ", opts, state.mmproj_index)
        } else {
            let opts: Vec<String> = state
                .draft_list
                .iter()
                .map(|opt| match opt {
                    Some(p) => p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    None => "None (Disabled)".to_string(),
                })
                .collect();
            (" Select Draft Model ", opts, state.draft_index)
        };

        let popup_area = centered_rect(50, 40, area);
        f.render_widget(Clear, popup_area);

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .title(title)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.selection));

        let mut list_spans = Vec::new();
        list_spans.push(Line::from(""));
        for (i, opt) in options_list.iter().enumerate() {
            if i == current_idx {
                list_spans.push(Line::from(vec![
                    Span::styled(
                        " 👉 ",
                        Style::default()
                            .fg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        opt,
                        Style::default()
                            .fg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                list_spans.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(opt, Style::default().fg(theme.fg)),
                ]));
            }
        }
        list_spans.push(Line::from(""));

        let popup_para = Paragraph::new(list_spans)
            .block(popup_block)
            .alignment(Alignment::Left);
        f.render_widget(popup_para, popup_area);
    }

    if state.screen == AppScreen::ConfirmSaveConfig {
        let popup_area = centered_rect(65, 55, area);
        f.render_widget(Clear, popup_area);

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .title(" Confirm Save Configuration ")
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.selection));

        let mut changes = Vec::new();
        let mmproj_val = match &state.mmproj_list[state.mmproj_index] {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "None".to_string()),
            None => "None (Disabled)".to_string(),
        };
        let original_mmproj_val = match &state.mmproj_list[state.original_mmproj_index] {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "None".to_string()),
            None => "None (Disabled)".to_string(),
        };
        let draft_val = match &state.draft_list[state.draft_index] {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "None".to_string()),
            None => "None (Disabled)".to_string(),
        };
        let original_draft_val = match &state.draft_list[state.original_draft_index] {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "None".to_string()),
            None => "None (Disabled)".to_string(),
        };

        if state.config_file_name != state.original_config_file_name {
            changes.push((
                "Target Config File",
                state.original_config_file_name.clone(),
                state.config_file_name.clone(),
            ));
        }
        if state.ctx_str != state.original_ctx_str {
            changes.push((
                "Context Size",
                state.original_ctx_str.clone(),
                state.ctx_str.clone(),
            ));
        }
        if state.ngl != state.original_ngl {
            changes.push(("GPU Layers", state.original_ngl.clone(), state.ngl.clone()));
        }
        if mmproj_val != original_mmproj_val {
            changes.push(("MMProj (Vision)", original_mmproj_val, mmproj_val));
        }
        if draft_val != original_draft_val {
            changes.push(("Draft Model", original_draft_val, draft_val));
        }
        if state.draft_ngl != state.original_draft_ngl {
            changes.push((
                "Draft GPU Layers",
                state.original_draft_ngl.clone(),
                state.draft_ngl.clone(),
            ));
        }
        if state.temp != state.original_temp {
            changes.push((
                "Temperature",
                state.original_temp.clone(),
                state.temp.clone(),
            ));
        }
        if state.top_p != state.original_top_p {
            changes.push(("Top P", state.original_top_p.clone(), state.top_p.clone()));
        }
        if state.top_k != state.original_top_k {
            changes.push(("Top K", state.original_top_k.clone(), state.top_k.clone()));
        }
        let original_total_layers_str = state
            .original_total_layers
            .map(|l| l.to_string())
            .unwrap_or_default();
        let total_layers_str = state
            .total_layers
            .map(|l| l.to_string())
            .unwrap_or_default();
        if total_layers_str != original_total_layers_str {
            changes.push(("Total Layers", original_total_layers_str, total_layers_str));
        }

        let mut text_lines = Vec::new();
        text_lines.push(Line::from(""));
        text_lines.push(Line::from(Span::styled(
            " The following configuration properties have changed:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        text_lines.push(Line::from(""));

        for (label, old, new) in changes {
            let display_old = if old.is_empty() { "none" } else { &old };
            let display_new = if new.is_empty() { "none" } else { &new };
            text_lines.push(Line::from(vec![
                Span::styled(format!("  • {}: ", label), Style::default().fg(theme.fg)),
                Span::styled(
                    display_old.to_string(),
                    Style::default().fg(theme.secondary),
                ),
                Span::styled(
                    " -> ",
                    Style::default()
                        .fg(theme.selection)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    display_new.to_string(),
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(""));

        let backup_check = if state.backup_config {
            if theme.show_emojis {
                " 🔘 [x] Backup previous config (.bak.<timestamp>) "
            } else {
                " [x] Backup previous config (.bak.<timestamp>) "
            }
        } else {
            if theme.show_emojis {
                " ⚪ [ ] Backup previous config (.bak.<timestamp>) "
            } else {
                " [ ] Backup previous config (.bak.<timestamp>) "
            }
        };
        let backup_style = if state.backup_config {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.secondary)
        };
        text_lines.push(Line::from(Span::styled(backup_check, backup_style)));
        text_lines.push(Line::from(""));
        text_lines.push(Line::from(Span::styled(
            "   (Press [Space] to toggle backup option)",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::ITALIC),
        )));

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(vec![
            Span::styled("   Press ", Style::default().fg(theme.fg)),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to save to ", Style::default().fg(theme.fg)),
            Span::styled(
                format!("{}.toml", state.config_file_name),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" or ", Style::default().fg(theme.fg)),
            Span::styled(
                "[Esc]",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to cancel.", Style::default().fg(theme.fg)),
        ]));

        let popup_para = Paragraph::new(text_lines)
            .block(popup_block)
            .alignment(Alignment::Left);
        f.render_widget(popup_para, popup_area);
    }

    if state.screen == AppScreen::WarnDiscardChanges {
        let popup_area = centered_rect(60, 25, area);
        f.render_widget(Clear, popup_area);

        let title = if theme.show_emojis {
            " ⚠️  Unsaved Parameter Changes "
        } else {
            " Unsaved Parameter Changes "
        };

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .title(title)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.error));

        let mut text_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                " You have unsaved configuration changes in the right panel.",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Moving to another model/preset will discard all temporary values.",
                Style::default().fg(theme.secondary),
            )),
            Line::from(""),
            Line::from(""),
        ];

        let confirm_spans = vec![
            Span::styled("   Press ", Style::default().fg(theme.fg)),
            Span::styled(
                "[Enter] / [y]",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to discard changes and switch model.",
                Style::default().fg(theme.fg),
            ),
        ];
        text_lines.push(Line::from(confirm_spans));

        let cancel_spans = vec![
            Span::styled("   Press ", Style::default().fg(theme.fg)),
            Span::styled(
                "[Esc] / [n]",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to return to editing parameters.",
                Style::default().fg(theme.fg),
            ),
        ];
        text_lines.push(Line::from(cancel_spans));
        text_lines.push(Line::from(""));

        let popup_para = Paragraph::new(text_lines)
            .block(popup_block)
            .alignment(Alignment::Left);
        f.render_widget(popup_para, popup_area);
    }
}

fn render_logs(f: &mut Frame, state: &mut AppState, area: Rect) {
    let theme = &state.theme;
    let size = f.area();
    // Running logs viewer view
    let preset_name = if state.presets.is_empty() {
        "None".to_string()
    } else {
        state.presets[state.preset_index].0.clone()
    };

    let host = state
        .global_config
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1");
    let port = if let Some(ref _server) = state.active_server {
        let mut p = "8080".to_string();
        let mut idx = 0;
        while idx < state.last_launch_args.len() {
            if state.last_launch_args[idx] == "--port" && idx + 1 < state.last_launch_args.len() {
                p = state.last_launch_args[idx + 1].clone();
                break;
            }
            idx += 1;
        }
        p
    } else {
        state
            .global_config
            .get("port")
            .and_then(|v| {
                if let Some(i) = v.as_i64() {
                    Some(i.to_string())
                } else {
                    v.as_str().map(|s| s.to_string())
                }
            })
            .unwrap_or_else(|| "auto".to_string())
    };

    let status_span = if state.logs_paused {
        Span::styled(
            " PAUSED (LOGS BUFFERED) ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " RUNNING ",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        )
    };

    let (label, display_name) = if state.is_router_mode {
        (
            if size.width < 110 {
                "Mode: "
            } else {
                "Server Mode: "
            },
            "Router".to_string(),
        )
    } else {
        (
            if size.width < 110 {
                "Preset: "
            } else {
                "Server Preset: "
            },
            preset_name,
        )
    };

    let server_info = if size.width < 110 {
        Line::from(vec![
            Span::styled(label, Style::default().fg(theme.secondary)),
            Span::styled(
                format!("{} ", truncate_middle(&display_name, 15)),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | URL: ", Style::default().fg(theme.secondary)),
            Span::styled(
                format!("http://{}:{} ", host, port),
                Style::default().fg(theme.primary),
            ),
            Span::styled(" | ", Style::default().fg(theme.secondary)),
            status_span,
        ])
    } else {
        Line::from(vec![
            Span::styled(label, Style::default().fg(theme.secondary)),
            Span::styled(
                format!("{} ", display_name),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" |  Address: ", Style::default().fg(theme.secondary)),
            Span::styled(
                format!("http://{}:{} ", host, port),
                Style::default().fg(theme.primary),
            ),
            Span::styled(" |  Status: ", Style::default().fg(theme.secondary)),
            status_span,
        ])
    };

    let mut raw_logs = std::collections::VecDeque::new();
    if let Some(ref server) = state.active_server {
        if state.logs_paused {
            raw_logs = state.paused_logs_buffer.clone();
        } else if let Ok(l) = server.logs.lock() {
            raw_logs = l.clone();
        }
    }

    let log_block_title = if state.logs_wrap {
        " Server Logs (Wrap Enabled) "
    } else {
        " Server Logs (Wrap Disabled - Left/Right arrows to scroll horizontally) "
    };

    let logs_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .title(log_block_title)
        .border_type(theme.border_type)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.primary));

    let full_command = state.last_launch_args.join(" ");
    let content_width = area.width as usize;
    let cmd_height = if content_width > 0 {
        (9 + full_command.len()).div_ceil(content_width).min(4) as u16
    } else {
        1
    };

    // Split Running view into Server Info Header + Full Command + Metrics + Logs Scroll Pane
    let running_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),          // Server Info line
            Constraint::Length(cmd_height), // Full Command line
            Constraint::Length(3),          // Metrics Panel
            Constraint::Min(2),             // Logs block
        ])
        .split(area);

    f.render_widget(Paragraph::new(server_info), running_layout[0]);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(theme.secondary)),
            Span::styled(full_command, Style::default().fg(theme.accent)),
        ]))
        .wrap(Wrap { trim: true }),
        running_layout[1],
    );

    // Fetch and render server metrics
    let mut server_metrics = crate::tui::logs::ServerMetrics::default();
    if let Some(ref server) = state.active_server {
        if let Ok(m) = server.metrics.lock() {
            server_metrics = m.clone();
        }
    } else {
        server_metrics.status = "OFFLINE".to_string();
    }

    let metrics_block = Block::default()
        .borders(Borders::TOP)
        .title(" Orchestrator Process & Routing Status ")
        .border_type(theme.border_type)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.primary));

    f.render_widget(metrics_block.clone(), running_layout[2]);
    let metrics_area = metrics_block.inner(running_layout[2]);
    let metrics_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Column 1: Process Status
            Constraint::Percentage(25), // Column 2: Server Mode
            Constraint::Percentage(25), // Column 3: Routing Details
            Constraint::Percentage(25), // Column 4: Memory Usage
        ])
        .split(metrics_area);

    // Column 1: Process Status & PID
    let status_color = match server_metrics.status.as_str() {
        "RUNNING" => theme.success,
        "LOADING" => theme.accent,
        "ERROR" => theme.error,
        _ => theme.secondary,
    };
    let col1_text = vec![
        Line::from(vec![
            Span::styled(" Status: ", Style::default().fg(theme.secondary)),
            Span::styled(
                server_metrics.status.clone(),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" PID:    ", Style::default().fg(theme.secondary)),
            Span::styled(
                server_metrics
                    .pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                Style::default().fg(theme.fg),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(col1_text), metrics_cols[0]);

    // Column 2: Server Mode & Max Models
    let mode_str = if server_metrics.is_router {
        "Router"
    } else {
        "Single Model"
    };
    let max_models_str = if server_metrics.is_router {
        server_metrics
            .max_models
            .map(|m| m.to_string())
            .unwrap_or_else(|| "1".to_string())
    } else {
        "N/A".to_string()
    };
    let col2_text = vec![
        Line::from(vec![
            Span::styled(" Mode:       ", Style::default().fg(theme.secondary)),
            Span::styled(
                mode_str,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Max Active: ", Style::default().fg(theme.secondary)),
            Span::styled(max_models_str, Style::default().fg(theme.fg)),
        ]),
    ];
    f.render_widget(Paragraph::new(col2_text), metrics_cols[1]);

    // Column 3: Active Model & Port
    let active_model_str = server_metrics.active_model.as_deref().unwrap_or("None");
    let active_port_str = server_metrics
        .active_port
        .map(|p| p.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let active_model_truncated = truncate_middle(
        active_model_str,
        metrics_cols[2].width.saturating_sub(16) as usize,
    );
    let col3_text = vec![
        Line::from(vec![
            Span::styled(" Active Model: ", Style::default().fg(theme.secondary)),
            Span::styled(
                active_model_truncated,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Active Port:  ", Style::default().fg(theme.secondary)),
            Span::styled(active_port_str, Style::default().fg(theme.fg)),
        ]),
    ];
    f.render_widget(Paragraph::new(col3_text), metrics_cols[2]);

    // Column 4: Memory Usage (RAM & VRAM)
    let col4_width = metrics_cols[3].width as usize;
    let bar_width = if col4_width > 22 {
        (col4_width - 20).min(8)
    } else {
        0
    };

    let mut col4_text = Vec::new();
    if let Some((used, total)) = server_metrics.ram_usage {
        let pct = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let bar = if bar_width > 0 {
            format!(" {}", get_vram_bar(Some((used, total)), bar_width))
        } else {
            "".to_string()
        };
        col4_text.push(Line::from(vec![
            Span::styled(" RAM:  ", Style::default().fg(theme.secondary)),
            Span::styled(
                format!(
                    "{:.1}/{:.1}G ({:.0}%)",
                    used as f64 / 1024.0,
                    total as f64 / 1024.0,
                    pct
                ),
                Style::default().fg(theme.success),
            ),
            Span::styled(bar, Style::default().fg(theme.accent)),
        ]));
    } else {
        col4_text.push(Line::from(vec![
            Span::styled(" RAM:  ", Style::default().fg(theme.secondary)),
            Span::styled("N/A", Style::default().fg(theme.secondary)),
        ]));
    }

    if let Some((used, total)) = server_metrics.vram_usage {
        let pct = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let bar = if bar_width > 0 {
            format!(" {}", get_vram_bar(Some((used, total)), bar_width))
        } else {
            "".to_string()
        };
        col4_text.push(Line::from(vec![
            Span::styled(" VRAM: ", Style::default().fg(theme.secondary)),
            Span::styled(
                format!(
                    "{:.1}/{:.1}G ({:.0}%)",
                    used as f64 / 1024.0,
                    total as f64 / 1024.0,
                    pct
                ),
                Style::default().fg(theme.success),
            ),
            Span::styled(bar, Style::default().fg(theme.accent)),
        ]));
    } else {
        col4_text.push(Line::from(vec![
            Span::styled(" VRAM: ", Style::default().fg(theme.secondary)),
            Span::styled("N/A (No GPU)", Style::default().fg(theme.secondary)),
        ]));
    }
    f.render_widget(Paragraph::new(col4_text), metrics_cols[3]);

    // Calculate inner logs scroll height
    let logs_rect = running_layout[3];
    let inner_height = if logs_rect.height > 2 {
        logs_rect.height - 2
    } else {
        1
    } as usize;
    let width = if logs_rect.width > 2 {
        logs_rect.width - 2
    } else {
        1
    } as usize;

    // Compile logs styling and wrapping
    let mut rendered_lines: Vec<Line> = Vec::new();
    for line in &raw_logs {
        if state.logs_wrap {
            // Build a flat list of (char, style) to wrap at width boundaries
            let mut chars: Vec<(char, Style)> = Vec::new();
            for span in &line.spans {
                for ch in span.text.chars() {
                    chars.push((ch, span.style));
                }
            }

            if chars.is_empty() {
                rendered_lines.push(Line::from(""));
            } else {
                for chunk in chars.chunks(width.max(1)) {
                    let mut spans_out: Vec<Span> = Vec::new();
                    let mut cur_text = String::new();
                    let mut cur_style = chunk[0].1;

                    for &(ch, st) in chunk {
                        if st != cur_style {
                            if !cur_text.is_empty() {
                                spans_out
                                    .push(Span::styled(std::mem::take(&mut cur_text), cur_style));
                            }
                            cur_style = st;
                        }
                        cur_text.push(ch);
                    }
                    if !cur_text.is_empty() {
                        spans_out.push(Span::styled(cur_text, cur_style));
                    }
                    rendered_lines.push(Line::from(spans_out));
                }
            }
        } else {
            let spans_out: Vec<Span> = line
                .spans
                .iter()
                .map(|s| Span::styled(s.text.clone(), s.style))
                .collect();
            rendered_lines.push(Line::from(spans_out));
        }
    }

    // Auto-scroll logic clamp
    if state.auto_scroll && !state.logs_paused && rendered_lines.len() > inner_height {
        state.log_scroll_offset = rendered_lines.len() - inner_height;
    }

    // Clamping scroll offsets
    let max_scroll_y = if rendered_lines.len() > inner_height {
        rendered_lines.len() - inner_height
    } else {
        0
    };
    if state.log_scroll_offset >= max_scroll_y {
        state.log_scroll_offset = max_scroll_y;
        if !state.auto_scroll {
            state.auto_scroll = true;
        }
    }

    let text = Text {
        lines: rendered_lines,
        ..Default::default()
    };

    let paragraph = Paragraph::new(text)
        .block(logs_block)
        .scroll((state.log_scroll_offset as u16, state.log_scroll_x as u16));

    f.render_widget(paragraph, running_layout[3]);
}

fn get_vram_bar(vram: Option<(u64, u64)>, bar_width: usize) -> String {
    let Some((used, total)) = vram else {
        return "-".repeat(bar_width);
    };
    if total == 0 {
        return "-".repeat(bar_width);
    }
    let pct = (used as f64 / total as f64).clamp(0.0, 1.0);
    let filled = (pct * bar_width as f64).round() as usize;
    let mut bar = String::new();
    for i in 0..bar_width {
        if i < filled {
            bar.push('█');
        } else {
            bar.push('░');
        }
    }
    bar
}
