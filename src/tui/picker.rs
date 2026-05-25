use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    File,
    Directory,
}

pub struct PickerEntry {
    pub name: String,
    pub is_dir: bool,
}

pub struct FilePicker {
    pub current_path: PathBuf,
    pub entries: Vec<PickerEntry>,
    pub selected_index: usize,
    pub mode: PickerMode,
}

impl FilePicker {
    pub fn new(initial_path: PathBuf, mode: PickerMode) -> Self {
        let mut picker = Self {
            current_path: initial_path,
            entries: Vec::new(),
            selected_index: 0,
            mode,
        };
        picker.refresh();
        picker
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_index = 0;

        if self.mode == PickerMode::Directory {
            self.entries.push(PickerEntry {
                name: ".[Select current directory]".to_string(),
                is_dir: true,
            });
        }

        if let Some(_parent) = self.current_path.parent() {
            self.entries.push(PickerEntry {
                name: "..".to_string(),
                is_dir: true,
            });
        }

        if let Ok(read_dir) = fs::read_dir(&self.current_path) {
            let mut sub_entries = Vec::new();
            for entry in read_dir.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let is_dir = metadata.is_dir();
                    if self.mode == PickerMode::Directory && !is_dir {
                        continue;
                    }
                    if let Ok(name) = entry.file_name().into_string() {
                        sub_entries.push(PickerEntry { name, is_dir });
                    }
                }
            }
            sub_entries.sort_by_key(|e| (!e.is_dir, e.name.to_lowercase()));
            self.entries.append(&mut sub_entries);
        }
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Option<PathBuf> {
        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                } else if !self.entries.is_empty() {
                    self.selected_index = self.entries.len() - 1;
                }
            }
            KeyCode::Down if !self.entries.is_empty() => {
                if self.selected_index < self.entries.len() - 1 {
                    self.selected_index += 1;
                } else {
                    self.selected_index = 0;
                }
            }
            KeyCode::Backspace => {
                if let Some(parent) = self.current_path.parent() {
                    self.current_path = parent.to_path_buf();
                    self.refresh();
                }
            }
            KeyCode::Enter => {
                if self.entries.is_empty() {
                    return None;
                }
                let selected = &self.entries[self.selected_index];

                if selected.name == ".[Select current directory]" {
                    return Some(self.current_path.clone());
                } else if selected.name == ".." {
                    if let Some(parent) = self.current_path.parent() {
                        self.current_path = parent.to_path_buf();
                        self.refresh();
                    }
                } else if selected.is_dir {
                    self.current_path.push(&selected.name);
                    self.refresh();
                } else if self.mode == PickerMode::File {
                    let mut path = self.current_path.clone();
                    path.push(&selected.name);
                    return Some(path);
                }
            }
            _ => {}
        }
        None
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &crate::tui::theme::Theme) {
        let title = match self.mode {
            PickerMode::File => " Select File ",
            PickerMode::Directory => " Select Folder ",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(theme.border_type)
            .style(Style::default().bg(theme.bg).fg(theme.fg));

        let inner_area = block.inner(area);
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Breadcrumbs
                Constraint::Length(1), // separator
                Constraint::Min(1),    // List
            ])
            .split(inner_area);

        let breadcrumb_prefix = if theme.show_emojis {
            " 📍 "
        } else {
            " Path: "
        };
        let breadcrumbs = Paragraph::new(format!(
            "{}{}",
            breadcrumb_prefix,
            self.current_path.display()
        ))
        .style(Style::default().fg(theme.primary));
        f.render_widget(breadcrumbs, chunks[0]);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_selected = i == self.selected_index;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.bg)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                let icon = if theme.show_emojis {
                    if entry.is_dir { "📁" } else { "📄" }
                } else {
                    if entry.is_dir { "[D]" } else { "[F]" }
                };

                let display_name = if entry.name == ".[Select current directory]" {
                    let prefix = if theme.show_emojis { "✅ " } else { "* " };
                    format!("{}Select this folder", prefix)
                } else {
                    entry.name.clone()
                };

                let suffix = if entry.is_dir
                    && entry.name != ".."
                    && entry.name != ".[Select current directory]"
                {
                    "/"
                } else {
                    ""
                };
                let content = format!(" {} {}{}", icon, display_name, suffix);

                ListItem::new(content).style(style)
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items);
        f.render_stateful_widget(list, chunks[2], &mut list_state);
    }
}
