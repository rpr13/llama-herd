#![allow(
    missing_docs,
    unused_qualifications,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

#[test]
fn test_lib_modules_reachable() {
    // Ensuring all public modules are reachable and symbols are exported correctly
    let _ = llama_herd::config::get_optimal_threads();
    let _ = llama_herd::discovery::clean_model_id(std::path::Path::new("test.gguf"));
    llama_herd::launcher::kill_existing_servers();
    // TUI symbols
    let _ = llama_herd::tui::app::AppScreen::Dashboard;
}
