use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::fs;
use std::path::PathBuf;

/// The operational mode for the file picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    /// Pick a single file.
    File,
    /// Pick a directory.
    Directory,
}

/// A single entry in the file/directory listing.
#[derive(Debug)]
pub struct PickerEntry {
    /// File or directory name.
    pub name: String,
    /// Flag indicating whether the entry is a directory.
    pub is_dir: bool,
}

/// Component that displays a scrollable folder tree picker overlay.
#[derive(Debug)]
pub struct FilePicker {
    /// Currently navigated path.
    pub current_path: PathBuf,
    /// Loaded sub-entries inside the current path.
    pub entries: Vec<PickerEntry>,
    /// Selected item index in the picker list.
    pub selected_index: usize,
    /// Active picker mode (File or Directory).
    pub mode: PickerMode,
}

impl FilePicker {
    /// Creates a new `FilePicker` starting at the specified path in the given mode.
    #[must_use]
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

    /// Refreshes the subdirectory/file listing based on the current navigated path.
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_index = 0;

        #[cfg(target_os = "windows")]
        let is_empty = self.current_path.to_string_lossy().is_empty();
        #[cfg(not(target_os = "windows"))]
        let is_empty = false;

        if self.mode == PickerMode::Directory && !is_empty {
            self.entries.push(PickerEntry {
                name: ".[Select current directory]".to_owned(),
                is_dir: true,
            });
        }

        #[cfg(target_os = "windows")]
        {
            if is_empty {
                for c in b'A'..=b'Z' {
                    let drive_str = format!("{}:\\", c as char);
                    if std::path::Path::new(&drive_str).exists() {
                        self.entries.push(PickerEntry {
                            name: drive_str,
                            is_dir: true,
                        });
                    }
                }
                return;
            }
        }

        let has_parent =
            self.current_path.parent().is_some() || (cfg!(target_os = "windows") && !is_empty);

        if has_parent {
            self.entries.push(PickerEntry {
                name: "..".to_owned(),
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

    /// Handles user key event inputs to navigate up/down or enter directories.
    pub fn handle_event(&mut self, key: KeyEvent) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let is_empty = self.current_path.to_string_lossy().is_empty();
        #[cfg(not(target_os = "windows"))]
        let is_empty = false;

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
                #[cfg(target_os = "windows")]
                {
                    if !is_empty {
                        if let Some(parent) = self.current_path.parent() {
                            self.current_path = parent.to_path_buf();
                        } else {
                            self.current_path = PathBuf::from("");
                        }
                        self.refresh();
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    if let Some(parent) = self.current_path.parent() {
                        self.current_path = parent.to_path_buf();
                        self.refresh();
                    }
                }
            }
            KeyCode::Enter => {
                if self.entries.is_empty() {
                    return None;
                }
                let selected = &self.entries[self.selected_index];

                if selected.name == ".[Select current directory]" {
                    if is_empty {
                        return None;
                    }
                    return Some(self.current_path.clone());
                } else if selected.name == ".." {
                    #[cfg(target_os = "windows")]
                    {
                        if let Some(parent) = self.current_path.parent() {
                            self.current_path = parent.to_path_buf();
                        } else {
                            self.current_path = PathBuf::from("");
                        }
                        self.refresh();
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        if let Some(parent) = self.current_path.parent() {
                            self.current_path = parent.to_path_buf();
                            self.refresh();
                        }
                    }
                } else if selected.is_dir {
                    #[cfg(target_os = "windows")]
                    {
                        if is_empty {
                            self.current_path = PathBuf::from(&selected.name);
                        } else {
                            self.current_path.push(&selected.name);
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        self.current_path.push(&selected.name);
                    }
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

    /// Renders the picker list view overlay on top of the frame.
    pub fn render(&self, f: &mut Frame<'_>, area: Rect, theme: &crate::tui::theme::Theme) {
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
        let display_path = if self.current_path.as_os_str().is_empty() {
            "Drives List".to_owned()
        } else {
            self.current_path.to_string_lossy().into_owned()
        };
        let breadcrumbs = Paragraph::new(format!("{breadcrumb_prefix}{display_path}"))
            .style(Style::default().fg(theme.primary));
        f.render_widget(breadcrumbs, chunks[0]);

        let items: Vec<ListItem<'_>> = self
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
                    format!("{prefix}Select this folder")
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
                let content = format!(" {icon} {display_name}{suffix}");

                ListItem::new(content).style(style)
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items);
        f.render_stateful_widget(list, chunks[2], &mut list_state);
    }
}
