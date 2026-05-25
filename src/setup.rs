use crate::config;
use crate::tui::picker::{FilePicker, PickerMode};
use crate::tui::theme::Theme;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

pub struct SetupState {
    pub server_path: String,
    pub models_dir: String,
    pub picker: FilePicker,
    pub theme: Theme,
    pub current_step: usize, // 0: Server, 1: Models, 2: Done
    pub error_message: Option<String>,
    pub should_exit: bool,
}

pub fn run_wizard(
    lh_dir: &Path,
    mut global_config: HashMap<String, serde_json::Value>,
) -> Option<(PathBuf, PathBuf, HashMap<String, serde_json::Value>)> {
    // Initial values from config if present
    let initial_server = config::resolve_server_executable(&global_config)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let initial_models = config::resolve_models_dir(&global_config)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let theme = Theme::default();

    // Determine starting step
    let mut current_step = 0;
    if !initial_server.is_empty() && Path::new(&initial_server).is_file() {
        current_step = 1;
    }
    if current_step == 1 && !initial_models.is_empty() && Path::new(&initial_models).is_dir() {
        current_step = 2;
    }

    let picker = if current_step == 0 {
        let initial_path = if !initial_server.is_empty() {
            PathBuf::from(&initial_server)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| config::get_home_dir().unwrap_or_else(|| PathBuf::from(".")))
        } else {
            config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
        };
        FilePicker::new(initial_path, PickerMode::File)
    } else {
        let initial_path = if !initial_models.is_empty() {
            PathBuf::from(&initial_models)
        } else {
            config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
        };
        FilePicker::new(initial_path, PickerMode::Directory)
    };

    let mut state = SetupState {
        server_path: initial_server,
        models_dir: initial_models,
        picker,
        theme,
        current_step,
        error_message: None,
        should_exit: false,
    };

    // Terminal setup
    if enable_raw_mode().is_err() {
        return None;
    }
    let mut stdout = io::stdout();
    if execute!(stdout, EnterAlternateScreen).is_err() {
        let _ = disable_raw_mode();
        return None;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return None;
        }
    };

    while !state.should_exit && state.current_step < 2 {
        if terminal.draw(|f| render_wizard(f, &state)).is_err() {
            break;
        }

        if let Ok(ev) = event::read() {
            handle_event(&mut state, ev);
        }
    }

    // Restore terminal
    let _ = terminal.show_cursor();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = disable_raw_mode();

    if state.should_exit || state.current_step < 2 {
        return None;
    }

    let server_exe = PathBuf::from(state.server_path.trim());
    let models_dir = PathBuf::from(state.models_dir.trim());

    global_config.insert(
        "llama-server".to_string(),
        serde_json::Value::String(server_exe.to_string_lossy().to_string()),
    );
    global_config.insert(
        "models-dir".to_string(),
        serde_json::Value::String(models_dir.to_string_lossy().to_string()),
    );

    let config_path = lh_dir.join("config.toml");
    if let Err(e) = config::save_config(&config_path, &global_config) {
        eprintln!("Warning: Failed to save config: {}", e);
    }

    Some((server_exe, models_dir, global_config))
}

fn handle_event(state: &mut SetupState, event: Event) {
    if let Event::Key(key) = event
        && key.kind == event::KeyEventKind::Press
    {
        match key.code {
            KeyCode::Esc => {
                state.should_exit = true;
            }
            _ => {
                if let Some(path) = state.picker.handle_event(key) {
                    state.error_message = None;
                    if state.current_step == 0 {
                        if path.is_file() {
                            state.server_path = path.to_string_lossy().into_owned();
                            state.current_step = 1;
                            let initial_models_path = if !state.models_dir.is_empty() {
                                PathBuf::from(&state.models_dir)
                            } else {
                                path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
                                    config::get_home_dir().unwrap_or_else(|| PathBuf::from("."))
                                })
                            };
                            state.picker =
                                FilePicker::new(initial_models_path, PickerMode::Directory);

                            // Auto-skip if next step is already valid
                            if !state.models_dir.is_empty() && Path::new(&state.models_dir).is_dir()
                            {
                                state.current_step = 2;
                            }
                        } else {
                            state.error_message = Some("Selected path is not a file".to_string());
                        }
                    } else if state.current_step == 1 {
                        if path.is_dir() {
                            state.models_dir = path.to_string_lossy().into_owned();
                            state.current_step = 2;
                        } else {
                            state.error_message =
                                Some("Selected path is not a directory".to_string());
                        }
                    }
                }
            }
        }
    }
}

fn render_wizard(f: &mut Frame, state: &SetupState) {
    let theme = &state.theme;
    let size = f.area();

    // --- 0. GLOBAL BACKGROUND ---
    // Fills the entire screen with the theme's background color
    f.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg)),
        size,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🦙 LlamaHerd Setup Wizard ")
        .title_alignment(Alignment::Center)
        .border_type(theme.border_type)
        .border_style(Style::default().fg(theme.primary));

    let area = centered_rect(80, 80, size);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let step_info = if state.current_step == 0 {
        "(Step 1/2): 🚀 Llama Server"
    } else {
        "(Step 2/2): 📂 Models Directory"
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Step title
            Constraint::Length(1), // spacing
            Constraint::Min(1),    // Picker
            Constraint::Length(1), // Error
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            step_info,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )])),
        chunks[0],
    );

    state.picker.render(f, chunks[2], theme);

    if let Some(ref err) = state.error_message {
        f.render_widget(
            Paragraph::new(format!("[!] {}", err)).style(Style::default().fg(theme.error)),
            chunks[3],
        );
    }

    f.render_widget(
        Paragraph::new("Use arrows to navigate, Enter to select, Esc to exit")
            .style(Style::default().fg(theme.secondary))
            .alignment(Alignment::Center),
        chunks[4],
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
