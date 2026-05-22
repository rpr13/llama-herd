use crate::tui::app::{AppScreen, AppState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap,
    },
};

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let size = f.area();

    // Global background/layout structure
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Content
            Constraint::Length(4), // Footer / Hotkeys
        ])
        .split(size);

    // --- 1. HEADER PANEL ---
    let header_text = vec![Line::from(vec![
        Span::styled(
            concat!(" 🦙 LLAMA-HERD v", env!("CARGO_PKG_VERSION"), " "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Rust Edition - Native Server Launcher ",
            Style::default().fg(Color::DarkGray),
        ),
    ])];
    let header_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Cyan));
    let header = Paragraph::new(header_text)
        .block(header_block)
        .alignment(Alignment::Left);
    f.render_widget(header, main_layout[0]);

    // --- 2. MAIN CONTENT AREA & EDIT POPUPS ---
    match state.screen {
        AppScreen::Select
        | AppScreen::EditingCtx
        | AppScreen::EditingNgl
        | AppScreen::EditingDraftNgl => {
            // Split Content Area into Left (Presets List) and Right (Preset Parameters Details)
            let content_layout = if size.width < 110 {
                let presets_height = (state.presets.len() as u16 + 2).clamp(5, 8);
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(presets_height), Constraint::Min(5)])
                    .split(main_layout[1])
            } else {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                    .split(main_layout[1])
            };

            // LEFT Panel: Presets List
            let list_val_width = content_layout[0].width.saturating_sub(6) as usize;
            let items: Vec<ListItem> = state
                .presets
                .iter()
                .enumerate()
                .map(|(idx, (name, _))| {
                    let display_name = truncate_middle(name, list_val_width);
                    if idx == state.preset_index {
                        ListItem::new(format!(" ➤ {} ", display_name)).style(
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ListItem::new(format!("   {} ", display_name))
                            .style(Style::default().fg(Color::White))
                    }
                })
                .collect();

            let presets_borders = if size.width < 110 {
                Borders::TOP
            } else {
                Borders::TOP | Borders::RIGHT
            };

            let presets_block = Block::default()
                .borders(presets_borders)
                .title("── Presets ")
                .border_style(Style::default().fg(Color::Cyan));
            let presets_list = List::new(items).block(presets_block);
            f.render_widget(presets_list, content_layout[0]);

            // RIGHT Panel: Parameters Details
            let right_block = Block::default()
                .borders(Borders::TOP)
                .title("── Preset Details & Parameters ")
                .border_style(Style::default().fg(Color::Cyan));

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

            let draft_val = match &state.draft_list[state.draft_index] {
                Some(p) => p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "None".to_string()),
                None => "None (Disabled)".to_string(),
            };

            let val_width = content_layout[1].width.saturating_sub(26) as usize;
            let display_preset_name = truncate_middle(&preset_name, val_width);
            let display_model_name = truncate_middle(&model_name, val_width);
            let display_mmproj_val = truncate_middle(&mmproj_val, val_width);
            let display_draft_val = truncate_middle(&draft_val, val_width);

            let rows = vec![
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(Span::styled(
                        "Preset Name",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(display_preset_name).style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(Span::styled(
                        "Model File",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(display_model_name).style(Style::default().fg(Color::LightCyan)),
                ]),
                Row::new(vec![
                    Cell::from(Span::styled(
                        "[c]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "Context Size",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(format!("{}", state.ctx)).style(Style::default().fg(Color::Green)),
                ]),
                Row::new(vec![
                    Cell::from(Span::styled(
                        "[n]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "GPU Layers",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(state.ngl.clone()).style(Style::default().fg(Color::Green)),
                ]),
                Row::new(vec![
                    Cell::from(Span::styled(
                        "[v]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "MMProj (Vision)",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(display_mmproj_val).style(Style::default().fg(Color::Yellow)),
                ]),
                Row::new(vec![
                    Cell::from(Span::styled(
                        "[d]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "Draft Model",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(display_draft_val).style(Style::default().fg(Color::Yellow)),
                ]),
                Row::new(vec![
                    Cell::from(Span::styled(
                        "[g]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        "Draft GPU Layers",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(if state.draft_ngl.is_empty() {
                        "N/A".to_string()
                    } else {
                        state.draft_ngl.clone()
                    })
                    .style(Style::default().fg(Color::Yellow)),
                ]),
                Row::new(vec![
                    Cell::from(Span::styled(
                        "[u]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled("Web UI", Style::default().fg(Color::DarkGray))),
                    Cell::from(if state.ui { "ON" } else { "OFF" }).style(
                        Style::default()
                            .fg(if state.ui { Color::Green } else { Color::Red })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];

            let table = Table::new(
                rows,
                [
                    Constraint::Length(4),
                    Constraint::Length(20),
                    Constraint::Min(20),
                ],
            )
            .block(right_block);
            f.render_widget(table, content_layout[1]);

            // Render Input Prompt Overlays
            if state.screen != AppScreen::Select {
                let (title, prompt) = match state.screen {
                    AppScreen::EditingCtx => (
                        " Edit Context Size ",
                        "Enter new context size (e.g. 131072, 8k, 32k):",
                    ),
                    AppScreen::EditingNgl => (
                        " Edit GPU Layers ",
                        "Enter N-GPU-layers (e.g. auto, 0, 32, --4):",
                    ),
                    AppScreen::EditingDraftNgl => (
                        " Edit Draft GPU Layers ",
                        "Enter draft N-GPU-layers (e.g. auto, 0, 8):",
                    ),
                    _ => ("", ""),
                };

                let popup_area = centered_rect(60, 20, main_layout[1]);
                f.render_widget(Clear, popup_area); // clears the background

                let popup_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(title)
                    .border_style(Style::default().fg(Color::Magenta));

                let popup_text = vec![
                    Line::from(prompt),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(&state.input_buffer, Style::default().fg(Color::White)),
                        Span::styled(
                            "_",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::RAPID_BLINK),
                        ),
                    ]),
                ];

                let popup_para = Paragraph::new(popup_text)
                    .block(popup_block)
                    .alignment(Alignment::Center);
                f.render_widget(popup_para, popup_area);
            }
        }
        AppScreen::Running => {
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
                .unwrap_or("0.0.0.0");
            let port = state
                .global_config
                .get("port")
                .and_then(|v| {
                    if let Some(i) = v.as_i64() {
                        Some(i.to_string())
                    } else {
                        v.as_str().map(|s| s.to_string())
                    }
                })
                .unwrap_or_else(|| "8080".to_string());

            let status_span = if state.logs_paused {
                Span::styled(
                    " PAUSED (LOGS BUFFERED) ",
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Rgb(50, 50, 0))
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " RUNNING ",
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::Rgb(0, 50, 0))
                        .add_modifier(Modifier::BOLD),
                )
            };

            let server_info = if size.width < 110 {
                Line::from(vec![
                    Span::styled("Preset: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{} ", truncate_middle(&preset_name, 15)),
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" | URL: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("http://{}:{} ", host, port),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                    status_span,
                ])
            } else {
                Line::from(vec![
                    Span::styled("Server Preset: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{} ", preset_name),
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" |  Address: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("http://{}:{} ", host, port),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(" |  Status: ", Style::default().fg(Color::DarkGray)),
                    status_span,
                ])
            };

            let mut raw_logs = Vec::new();
            if let Some(ref server) = state.active_server {
                if state.logs_paused {
                    raw_logs = state.paused_logs_buffer.clone();
                } else if let Ok(l) = server.logs.lock() {
                    raw_logs = l.clone();
                }
            }

            let log_block_title = if state.logs_wrap {
                "── Server Logs (Wrap Enabled) "
            } else {
                "── Server Logs (Wrap Disabled - Left/Right arrows to scroll horizontally) "
            };

            let logs_block = Block::default()
                .borders(Borders::TOP)
                .title(log_block_title)
                .border_style(Style::default().fg(Color::Cyan));

            let full_command = state.last_launch_args.join(" ");
            let content_width = main_layout[1].width as usize;
            let cmd_height = if content_width > 0 {
                (9 + full_command.len()).div_ceil(content_width).min(4) as u16
            } else {
                1
            };

            // Split Running view into Server Info Header + Full Command + Logs Scroll Pane
            let running_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),          // Server Info line
                    Constraint::Length(cmd_height), // Full Command line
                    Constraint::Min(2),             // Logs block
                ])
                .split(main_layout[1]);

            f.render_widget(Paragraph::new(server_info), running_layout[0]);

            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Command: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(full_command, Style::default().fg(Color::Yellow)),
                ]))
                .wrap(Wrap { trim: true }),
                running_layout[1],
            );

            // Calculate inner logs scroll height
            let logs_rect = running_layout[2];
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
                                        spans_out.push(Span::styled(
                                            std::mem::take(&mut cur_text),
                                            cur_style,
                                        ));
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

            f.render_widget(paragraph, running_layout[2]);
        }
    }

    // --- 3. FOOTER HINTS PANEL ---
    let footer_text = match state.screen {
        AppScreen::Select => {
            if size.width < 110 {
                vec![
                    Line::from(vec![
                        Span::styled(
                            " [Enter]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Launch  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [Ctrl+R]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Router  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [c]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Context  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [n]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" GPU Layers  ", Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " [v]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" MMProj  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [d]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Draft  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [g]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Draft NGL  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [u]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Web UI  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [q]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Quit", Style::default().fg(Color::White)),
                    ]),
                ]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled(
                            " [Enter]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Launch Preset  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [Ctrl+R]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Launch Router  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [c]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Edit Context  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [n]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Edit GPU Layers  ", Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " [v]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Cycle MMProj  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [d]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Cycle Draft  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [g]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Edit Draft NGL  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [u]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Toggle Web UI  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [q]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Quit", Style::default().fg(Color::White)),
                    ]),
                ]
            }
        }
        AppScreen::EditingCtx | AppScreen::EditingNgl | AppScreen::EditingDraftNgl => {
            vec![Line::from(vec![
                Span::styled(
                    " [Enter]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Save Settings  ", Style::default().fg(Color::White)),
                Span::styled(
                    " [Esc]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Cancel and Go Back", Style::default().fg(Color::White)),
            ])]
        }
        AppScreen::Running => {
            if size.width < 110 {
                vec![
                    Line::from(vec![
                        Span::styled(
                            " [r]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Restart  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [p]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Pause  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [c]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Copy  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [w]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Wrap  ", Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " [Up/Dn/PgUp/PgDn]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll V  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [Left/Right]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll H  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [s]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Stop  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [q]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Quit", Style::default().fg(Color::White)),
                    ]),
                ]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled(
                            " [r]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Restart Server  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [p]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Pause Logs  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [c]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Copy Logs  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [w]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Toggle Wrap  ", Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " [Up/Down/PgUp/PgDn]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            " Scroll Logs Vertically  ",
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(
                            " [Left/Right]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Scroll Horizontally  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [s]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Stop Server  ", Style::default().fg(Color::White)),
                        Span::styled(
                            " [q]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" Quit", Style::default().fg(Color::White)),
                    ]),
                ]
            }
        }
    };

    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Cyan));
    let footer = Paragraph::new(footer_text)
        .block(footer_block)
        .alignment(Alignment::Center);
    f.render_widget(footer, main_layout[2]);
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
