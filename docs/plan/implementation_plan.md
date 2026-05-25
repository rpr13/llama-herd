# Upgrading llama.cpp Path Configuration & Dropping CLI

We will upgrade the path resolution system of `llama-herd` to fully decouple configuration storage, model file storage, and the `llama-server` binary path. Additionally, we will drop the classic CLI interface (`src/cli.rs`), making `llama-herd` a pure TUI and preset generator tool.

## Relocating Configuration & Presets
- `config.toml` is moved to a dedicated user config folder (`~/.config/llama-herd` on Linux/macOS, `%APPDATA%/llama-herd` on Windows).
- `models-preset.ini` is moved directly into the GGUF `models` directory.
- `LLAMA_PATH` env variable check is dropped.

## System PATH Binary Default
- If `llama-server` is already available in the system `PATH`, we will automatically set it as the default binary and bypass the server setup prompt.

## Setup Wizard
- If paths to `llama-server` or the models directory are missing on startup, an interactive setup wizard will prompt the user to input the missing paths in the terminal, then save them to `config.toml`.

## Proposed Changes

### Clean Up CLI Module
#### [DELETE] [cli.rs](file:///home/rpr/dev/llama-herd/src/cli.rs)
- Remove `src/cli.rs`.
#### [DELETE] [cli.rs](file:///home/rpr/dev/llama-herd/tests/cli.rs)
- Remove `tests/cli.rs` unit tests.
#### [MODIFY] [lib.rs](file:///home/rpr/dev/llama-herd/src/lib.rs)
- Remove `pub mod cli;`.

### Config and Resolution Module

#### [MODIFY] [config.rs](file:///home/rpr/dev/llama-herd/src/config.rs)
- Remove `resolve_base_dir()`.
- Implement `get_llama_herd_dir() -> PathBuf` to resolve:
  - Unix: `~/.config/llama-herd`
  - Windows: `~AppData/Roaming/llama-herd`
  - Fallback: Current working directory (`.`) if home directory cannot be resolved.
- Implement `save_config(path: &Path, config: &HashMap<String, serde_json::Value>) -> Result<(), std::io::Error>`:
  - Converts JSON values to TOML values and serializes them using the `toml` crate.
  - Ensures the parent directory exists before writing.
- Implement path resolution helpers:
  - `resolve_server_executable(global_config: &HashMap<String, serde_json::Value>) -> Option<PathBuf>`:
    - 1. Checks `llama-server` or `server-path` in `config.toml`.
    - 2. Searches system `PATH` environment variable for `llama-server` / `llama-server.exe` and returns its absolute path if found.
  - `resolve_models_dir(global_config: &HashMap<String, serde_json::Value>) -> Option<PathBuf>`:
    - 1. Checks `models-dir` or `models-path` in `config.toml`.
    - 2. Checks if a folder named `models` exists in the current working directory.
- Update `parse_args(args: &[String]) -> (bool, bool)`:
  - Remove CLI-related flag parsing (`-c`, `--cli`).
  - Return `(show_help, generate_ini)`.

### Preset Generation

#### [MODIFY] [discovery.rs](file:///home/rpr/dev/llama-herd/src/discovery.rs)
- Update `generate_presets_ini`:
  - Drop the `base_dir: &Path` parameter from the signature.
  - Generate/write `models-preset.ini` directly inside the `models_dir` path: `models_dir.join("models-preset.ini")`.

### Application Bootstrap and Interactive Setup Wizard

#### [MODIFY] [main.rs](file:///home/rpr/dev/llama-herd/src/main.rs)
- Resolve `llama_herd_dir` and read `config.toml` from `llama_herd_dir.join("config.toml")`.
- Resolve `server_exe` and `models_dir` paths.
- If either path is missing, print a setup wizard to prompt the user:
  - Loop and ask for the absolute path to `llama-server` until a valid file is provided. (Bypassed if auto-detected in system `PATH`).
  - Loop and ask for the path to the models directory until a valid directory is provided.
  - Save these paths to `config.toml` under `llama-herd` config directory.
- Update signature calls:
  - Call `generate_presets_ini(&models_dir, &global_config)`.
  - Initialize TUI `AppState` without `base_dir` parameters.
  - Remove all CLI interaction flows and classic menu modes.

### TUI App State

#### [MODIFY] [tui/app.rs](file:///home/rpr/dev/llama-herd/src/tui/app.rs)
- Remove `base_dir: PathBuf` field from `AppState` struct and its `AppState::new()` signature.

#### [MODIFY] [tui/mod.rs](file:///home/rpr/dev/llama-herd/src/tui/mod.rs)
- Update calls to `generate_presets_ini` to omit the `base_dir` parameter.
- Update `AppState::new()` invocation to omit the `base_dir` parameter.

## Verification Plan

### Automated Tests
- Update/add unit tests in `tests/config.rs`, `tests/main.rs`, `tests/discovery.rs`, and `tests/launcher.rs`:
  - Remove/disable CLI-related test checks.
  - Update `test_generate_presets_ini_generation` to pass the updated signatures.
  - Test config loader, parser, and setup path resolutions.
  - Run `cargo test` to ensure all checks pass.
