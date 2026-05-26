use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use llama_herd::tui::picker::{FilePicker, PickerMode};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_picker_file_mode() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("model1.gguf");
    let file2 = dir.path().join("model2.gguf");
    let subfolder = dir.path().join("folder1");

    fs::write(&file1, "a").unwrap();
    fs::write(&file2, "b").unwrap();
    fs::create_dir(&subfolder).unwrap();

    let picker = FilePicker::new(dir.path().to_path_buf(), PickerMode::File);

    assert!(
        picker
            .entries
            .iter()
            .any(|e| e.name == "model1.gguf" && !e.is_dir)
    );
    assert!(
        picker
            .entries
            .iter()
            .any(|e| e.name == "model2.gguf" && !e.is_dir)
    );
    assert!(
        picker
            .entries
            .iter()
            .any(|e| e.name == "folder1" && e.is_dir)
    );
}

#[test]
fn test_picker_dir_mode() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("model1.gguf");
    let subfolder = dir.path().join("folder1");

    fs::write(&file1, "a").unwrap();
    fs::create_dir(&subfolder).unwrap();

    let picker = FilePicker::new(dir.path().to_path_buf(), PickerMode::Directory);

    assert!(
        picker
            .entries
            .iter()
            .any(|e| e.name == ".[Select current directory]" && e.is_dir)
    );
    assert!(
        picker
            .entries
            .iter()
            .any(|e| e.name == "folder1" && e.is_dir)
    );
    assert!(!picker.entries.iter().any(|e| e.name == "model1.gguf"));
}

#[test]
fn test_picker_navigation() {
    let dir = tempdir().unwrap();
    let subfolder = dir.path().join("folder1");
    fs::create_dir(&subfolder).unwrap();

    let mut picker = FilePicker::new(dir.path().to_path_buf(), PickerMode::Directory);

    let folder_idx = picker
        .entries
        .iter()
        .position(|e| e.name == "folder1")
        .unwrap();
    picker.selected_index = folder_idx;

    let key_enter = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };

    let result = picker.handle_event(key_enter);
    assert!(result.is_none());
    assert_eq!(picker.current_path, subfolder);

    // Backspace to navigate back up
    let key_bs = KeyEvent {
        code: KeyCode::Backspace,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    picker.handle_event(key_bs);
    assert_eq!(picker.current_path, dir.path());
}

#[test]
fn test_picker_keyboard_up_down() {
    let dir = tempdir().unwrap();
    let mut picker = FilePicker::new(dir.path().to_path_buf(), PickerMode::Directory);

    // Mock entries manually to ensure predictable navigation state
    picker.entries = vec![
        llama_herd::tui::picker::PickerEntry {
            name: ".[Select current directory]".to_string(),
            is_dir: true,
        },
        llama_herd::tui::picker::PickerEntry {
            name: "folder1".to_string(),
            is_dir: true,
        },
    ];
    picker.selected_index = 0;

    let key_down = KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    let key_up = KeyEvent {
        code: KeyCode::Up,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };

    picker.handle_event(key_down);
    assert_eq!(picker.selected_index, 1);

    picker.handle_event(key_down); // Wraps around
    assert_eq!(picker.selected_index, 0);

    picker.handle_event(key_up); // Wraps back to end
    assert_eq!(picker.selected_index, 1);
}
