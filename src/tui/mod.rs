pub mod app;
pub mod logs;
pub mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;

pub use app::{AppScreen, AppState};
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

    match state.screen {
        AppScreen::Select => match key.code {
            KeyCode::Char('q') => {
                should_quit = true;
            }
            KeyCode::Char('c') => {
                state.screen = AppScreen::EditingCtx;
                state.input_buffer = state.ctx.to_string();
            }
            KeyCode::Char('n') => {
                state.screen = AppScreen::EditingNgl;
                state.input_buffer = state.ngl.clone();
            }
            KeyCode::Char('g') => {
                state.screen = AppScreen::EditingDraftNgl;
                state.input_buffer = state.draft_ngl.clone();
            }
            KeyCode::Char('u') => {
                state.ui = !state.ui;
            }
            KeyCode::Char('v') if !state.mmproj_list.is_empty() => {
                state.mmproj_index = (state.mmproj_index + 1) % state.mmproj_list.len();
            }
            KeyCode::Char('d') if !state.draft_list.is_empty() => {
                state.draft_index = (state.draft_index + 1) % state.draft_list.len();
                if state.draft_list[state.draft_index].is_none() {
                    state.draft_ngl = "".to_string();
                } else if state.draft_ngl.is_empty() {
                    state.draft_ngl = "auto".to_string();
                }
            }
            KeyCode::Up if !state.presets.is_empty() => {
                if state.preset_index == 0 {
                    state.preset_index = state.presets.len() - 1;
                } else {
                    state.preset_index -= 1;
                }
                state.load_current_preset_settings();
            }
            KeyCode::Down if !state.presets.is_empty() => {
                state.preset_index = (state.preset_index + 1) % state.presets.len();
                state.load_current_preset_settings();
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Spawns router mode server
                crate::launcher::kill_existing_servers();
                let preset_ini_path = crate::discovery::generate_presets_ini(
                    &state.models_dir,
                    &state.base_dir,
                    &state.global_config,
                );
                let launch_args = crate::launcher::build_router_launch_parameters(
                    &state.server_exe,
                    &preset_ini_path,
                    &state.global_config,
                );
                state.last_launch_args = launch_args.clone();
                state.is_router_mode = true;

                match ActiveServer::spawn(&launch_args, &state.models_dir, Some(event_tx.clone())) {
                    Ok(server) => {
                        state.active_server = Some(server);
                        state.screen = AppScreen::Running;
                        state.logs_paused = false;
                        state.paused_logs_buffer.clear();
                        state.auto_scroll = true;
                        state.log_scroll_offset = 0;
                        state.log_scroll_x = 0;
                    }
                    Err(_e) => {}
                }
            }
            KeyCode::Enter if !state.presets.is_empty() => {
                // Spawns preset server
                crate::launcher::kill_existing_servers();
                let (_preset_name, model_path) = &state.presets[state.preset_index];
                let assets = crate::discovery::discover_assets(model_path, &state.models_dir);
                let settings = state.get_user_settings();
                let launch_args = crate::launcher::build_launch_parameters(
                    &state.server_exe,
                    model_path,
                    &assets,
                    &settings,
                    &state.global_config,
                );
                state.last_launch_args = launch_args.clone();
                state.is_router_mode = false;

                match ActiveServer::spawn(&launch_args, &state.models_dir, Some(event_tx.clone())) {
                    Ok(server) => {
                        state.active_server = Some(server);
                        state.screen = AppScreen::Running;
                        state.logs_paused = false;
                        state.paused_logs_buffer.clear();
                        state.auto_scroll = true;
                        state.log_scroll_offset = 0;
                        state.log_scroll_x = 0;
                    }
                    Err(_e) => {}
                }
            }
            _ => {}
        },
        AppScreen::EditingCtx | AppScreen::EditingNgl | AppScreen::EditingDraftNgl => {
            match key.code {
                KeyCode::Esc => {
                    state.screen = AppScreen::Select;
                }
                KeyCode::Enter => {
                    match state.screen {
                        AppScreen::EditingCtx => {
                            state.ctx = crate::config::parse_ctx_str(&state.input_buffer);
                        }
                        AppScreen::EditingNgl => {
                            state.ngl = state.input_buffer.trim().to_string();
                        }
                        AppScreen::EditingDraftNgl => {
                            state.draft_ngl = state.input_buffer.trim().to_string();
                        }
                        _ => {}
                    }
                    state.screen = AppScreen::Select;
                }
                KeyCode::Backspace => {
                    state.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    state.input_buffer.push(c);
                }
                _ => {}
            }
        }
        AppScreen::Running => match key.code {
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
                state.screen = AppScreen::Select;
            }
            KeyCode::Char('r') => {
                // Restart server
                if let Some(mut server) = state.active_server.take() {
                    server.kill();
                }
                match ActiveServer::spawn(
                    &state.last_launch_args,
                    &state.models_dir,
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
                if let Ok(true) = crossterm::event::poll(Duration::from_millis(100))
                    && let Ok(Event::Key(key)) = crossterm::event::read()
                    && key.kind == event::KeyEventKind::Press
                    && event_tx.send(TuiEvent::Input(key)).is_err()
                {
                    break;
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
                    TuiEvent::Tick => {}
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
