# GEMINI.md - LlamaHerd (`llama-herd`)

This file serves as the design description, architecture guidelines, configuration manual, and project mandates for developers and AI agents working on the Rust version of `llama-herd`.

## Project Overview

LlamaHerd is a high-performance, native Rust TUI designed as a self-contained multi-model server launcher, preset router, and TUI control center for `llama.cpp`'s `llama-server`. It simplifies the deployment of LLMs by automating model pairing (speculative draft models, vision projectors) and providing a rich, interactive dashboard for real-time monitoring and control.

### Key Architecture Components

- **Entry Point (`src/main.rs`)**: Handles command-line argument parsing (supporting flags like `--ini`), global configuration loading from platform-specific directories, and switches between TUI and early-exit modes.
- **Launcher Core (`src/launcher.rs`)**: Orchestrates the `llama-server` subprocess. Builds complex command-line arguments for both Single Model and Router modes.
- **Configuration & Discovery (`src/config.rs`, `src/discovery.rs`)**:
  - Automatically scans the configured `models_dir` for GGUF files and corresponding TOML configurations.
  - Generates a `models-preset.ini` dynamically if it does not exist.
  - Implements heuristics for auto-pairing main models with compatible draft models or `mmproj` files.
- **Setup Wizard (`src/setup.rs`)**:
  - Provides an interactive TUI-based setup flow for first-time initialization or missing path resolution.
- **Log Management (`src/tui/logs.rs`)**: Streams `stdout` and `stderr` concurrently into background threads. Features a native ANSI parser to convert escape sequences (like SGR color/style codes) into `ratatui::style::Style` spans, preserving native colored logs.
- **TUI Dashboard (`src/tui/`)**: Implemented with `ratatui` (0.30) and `crossterm` (0.29). Renders panels, lists, parameter override inputs, and a custom log viewer.

---

## Configuration System

### 1. Global Config (`config.toml`)

Stored in a platform-specific configuration directory:
- **Linux/Unix**: `~/.config/llama-herd/config.toml`
- **Windows**: `%APPDATA%\llama-herd\config.toml`
- **macOS**: `~/Library/Application Support/llama-herd/config.toml`

The `llama-server` executable path and the `models-dir` (where your models are located) MUST be defined here. If they are missing or invalid, LlamaHerd will launch an interactive **Setup Wizard** to help you configure them.

#### Supported Global Options:
- `llama-server` (Path to the `llama-server` executable)
- `models-dir` (Path to the directory containing your GGUF models)
- `host` (default: `"0.0.0.0"`)
- `port` (default: `"auto"`) - Binds to 8080 or the first free port sequentially if auto or occupied.
- `flash-attn` (default: `"auto"`)
- `kv-quant` (default: `"q8_0"`)
- `models-max` (default: `1`) - Maximum active models loaded concurrently in Router Mode
- `batch-size` (default: `256`)
- `ubatch-size` (default: `256`)
- `threads` (default: auto-detected physical cores)
- `ui` (default: `true`) - Global Web UI enablement toggle

### 2. Model-Specific Configurations (`<model-name>.toml`)

Configured next to a `.gguf` file (e.g. `Qwen2.5-7B.toml` for `Qwen2.5-7B.gguf`).

- **Prefix-Based Matching Hierarchy**: LlamaHerd resolves configuration paths by searching first for an exact GGUF stem match, then falling back to prefix matching (e.g. `model-name.toml` matching `model-name-q4_0.gguf` and `model-name-q5_0.gguf`), enabling shared config files across different model quantizations.

> [!IMPORTANT]
> **Strict TOML Key Rules & Categories**:
>
> - **Strict Keys**: Keys in TOML configurations must not contain underscores (`_`) or start with a dash (`-`). Violating keys are rejected at parse/load time, with warning logs emitted in `load_toml_safe`.
> - **Table `[llama-herd]`**: Reserved for `llama-herd` custom settings (e.g. `is-default`, `is-draft`). These are handled internally and not passed to `llama-server`.
> - **Table `[llama-server-long]`**: Mapped directly to double-dash long options for `llama-server`. For example, `ctx-size = "32k"` maps to `--ctx-size 32768`.
> - **Table `[llama-server-short]`**: Mapped directly to single-dash short options for `llama-server`. For example, `sps = 0.5` maps to `-sps 0.5`.
> - **Root level**: Treated as long options, maintaining backward compatibility.

#### Supported `[llama-herd]` Custom Configuration Keys:

- **Heuristic & Discovery Controls**:
  - `is-draft` / `is-draft-only`: Flag this model as a draft (hides it from the main select list).
  - `is-default`: Designates this model as the default startup selection in the TUI.
  - `draft` / `draft-model`: Specify a draft model filename (or `"none"` / `"false"` to disable draft pairing).
  - `mmproj`: Explicitly define the vision projector model filename to pair with this model.
  - `total-layers`: Total number of layers for layers-based computations.

### 3. Dynamic Configuration Scanning & Active Writes Settling
- **Background Scanner**: The TUI runs a background check on event ticks to scan the `models-dir` files every 1 second.
- **Settle Logic**: When a GGUF or TOML file size or mtime is actively changing (e.g. during model copy/download), preset regeneration is deferred to prevent partial preset loading and lock contention. Presets are only regenerated when the directory state stabilizes.
- **Dirty State Indicator**: If a background change is detected while the user has unsaved dashboard parameter overrides, settings reloading is skipped to protect active edits, and a TUI warning notification bar is rendered.
- **Invalid Directory Status**: If the directory is removed or becomes inaccessible, the TUI displays a warning bar and suspends scanning until resolved.

---

## Theme System (`theme.toml`)

The TUI utilizes a Hybrid Theme System (Functional Palette + Procedural UI). It is fully customizable via a `theme.toml` file in the global configuration directory.

- **Functional Palette**: Uses semantic keys (`primary`, `selection`, `accent`, `success`, `error`, etc.) to map colors across the entire UI.
- **Procedural UI**: Controls aesthetic behaviors like `show-emojis` and `border-type`.

If `theme.toml` is missing, LlamaHerd defaults to an internal **"Borderless Simple"** skin.

See `docs/theming.md` for the full schema, design principles, and examples.

---

## Project Mandates & Conventions

### 1. Mandatory Theme Adherence

Every UI component in LlamaHerd **must** use the theme system. 
- **NO HARDCODED COLORS**: Hardcoding colors like `Color::Cyan` or `Color::White` in `src/tui/ui.rs` or any other UI module is strictly prohibited.
- Always use `state.theme.<property>` or pass a reference to the `Theme` struct to rendering functions.
- All new visual elements must be mapped to an appropriate semantic field in the `Theme` struct.

### 2. Mandatory Server Parameters

`llama-server` **must** always be launched with `--log-colors on`. This is hard-coded in the launcher (`src/launcher.rs`) to ensure the TUI receives color escape sequences to render.

### 2. Parameter Passthrough & Restrictions

To prevent conflicts with managed options, any key (long, short, prefixed with `s-`, or unprefixed) matching a managed parameter is **restricted** and ignored during the passthrough stage.

- **Restricted Long Option Keys**: `ctx-size`, `total-layers`, `n-gpu-layers`, `kv-quant`, `kv-unified`, `cache-type-k`, `cache-type-v`, `ngl`, `threads`, `ngld`, `gpu-layers-draft`, `spec-draft-ngl`, `model-draft`, `spec-draft-model`, `is-draft`, `is-default`, `is-draft-only`, `ui`, `webui`, `model`, `chat-template-file`, `mmproj`, `jinja`, `flash-attn`, `version`, `tools`, `batch-size`, `ubatch-size`, `log-colors`, `host`, `port`, `np`, `parallel`, `models-preset`, `models-max`, `models-autoload`, `props`, `temp`, `top-p`, `top-k`, `reasoning`, `reasoning-format`.
- **Restricted Short Option Keys**: `c`, `ngl`, `ngld`, `t`, `md`, `m`, `mm`, `np`, `b`, `ub`, `fa`, `kvu`, `h`.

### 3. Automated Quality Gates

The project enforces code quality via Git pre-commit hooks managed by `cargo-husky`.

- **Pre-commit Checks**: Runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- Avoid bypassing these hooks unless absolutely necessary.

### 4. Code Quality & Formatting

- **ANSI Support**: The log parser in `src/tui/logs.rs` must correctly parse SGR (Select Graphic Rendition) color codes and ignore other escape sequences.
- **Prettier & ESLint**: Ensure that configuration files, formatting configs, and markdown are well-formatted.
- **Type Safety**: Leverage Rust's type system to ensure parameter building and configurations are safe, cleanly mapping option keys between hyphens (`-`) and underscores (`_`) correctly.

### 5. Documentation Maintenance

- **AI Agents**: When implementing code changes, the AI agent must always check `README.md` and all files in the `docs/` directory to identify and perform any necessary updates, ensuring that documentation never goes out of sync with code modifications.

### 6. Local Process Metrics & Orchestrator Status

The TUI must not use network-scraping or HTTP requests to poll performance metrics/slots from `llama-server`. Instead, status tracking is handled locally:
- Subprocess `stdout`/`stderr` output streams are parsed dynamically to extract runtime info (e.g. startup completion, sub-instance routing details, port mapping).
- Subprocess state and PID are monitored directly via local OS child processes check (e.g. `try_wait`).
