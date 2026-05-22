use llama_herd::tui::ui::{centered_rect, truncate_middle};
use ratatui::layout::Rect;

#[test]
fn test_centered_rect_proportions() {
    let area = Rect::new(0, 0, 100, 100);
    let rect = centered_rect(60, 20, area);

    // Width should be 60% of 100 = 60
    assert_eq!(rect.width, 60);
    // Height should be 20% of 100 = 20
    assert_eq!(rect.height, 20);
    // X should be (100 - 60) / 2 = 20
    assert_eq!(rect.x, 20);
    // Y should be (100 - 20) / 2 = 40
    assert_eq!(rect.y, 40);
}

#[test]
fn test_centered_rect_clamping() {
    let area = Rect::new(0, 0, 10, 10);
    // Small area with large percentages
    let rect = centered_rect(100, 100, area);
    assert_eq!(rect.width, 10);
    assert_eq!(rect.height, 10);
    assert_eq!(rect.x, 0);
    assert_eq!(rect.y, 0);
}

#[test]
fn test_truncate_middle_basic() {
    assert_eq!(truncate_middle("hello", 10), "hello");
    assert_eq!(truncate_middle("hello", 5), "hello");
    assert_eq!(truncate_middle("hello world", 8), "he...rld");
    assert_eq!(
        truncate_middle("gemma-4-E-2B-it-heretic-arabic", 20),
        "gemma-4-...ic-arabic"
    );
    assert_eq!(truncate_middle("hello", 2), "...");
    assert_eq!(truncate_middle("你好，世界！", 5), "你...！");
}
