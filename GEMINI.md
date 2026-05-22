# GEMINI.md - LlamaHerd (`llama-herd`)

This file serves as the design description, architecture guidelines, configuration manual, and project mandates for developers and AI agents working on the Rust version of `llama-herd`.

## Project Overview

LlamaHerd is a high-performance, native Rust TUI and CLI wrapper designed as a self-contained multi-model server launcher, preset router, and TUI control center for `llama.cpp`'s `llama-server`. It simplifies the deployment of LLMs by automating model pairing (speculative draft models, vision projectors) and providing a rich, interactive dashboard for real-time monitoring and control.

### Key Architecture Components

- **Entry Point (`src/main.rs`)**: Handles environment resolution (`LLAMA_PATH`), command-line argument parsing (supporting flags like `--cli` and `--ini`), global configuration loading (`config.toml`), and switches between TUI, CLI, and early-exit modes.
- **Launcher Core (`src/launcher.rs`)**: Orchestrates the `llama-server` subprocess. Builds complex command-line arguments for both Single Model and Router modes.
- **Configuration & Discovery (`src/config.rs`, `src/discovery.rs`)**:
  - Automatically scans the `models/` subdirectory in `LLAMA_PATH` for GGUF files and corresponding TOML configurations.
  - Generates a `models-preset.ini` dynamically if it does not exist.
  - Implements heuristics for auto-pairing main models with compatible draft models or `mmproj` files.
- **Log Management (`src/tui/logs.rs`)**: Streams `stdout` and `stderr` concurrently into background threads. Features a native ANSI parser to convert escape sequences (like SGR color/style codes) into `ratatui::style::Style` spans, preserving native colored logs.
- **TUI Dashboard (`src/tui/`)**: Implemented with `ratatui` (0.30) and `crossterm` (0.29). Renders panels, lists, parameter override inputs, and a custom log viewer.

---

## Configuration System

### 1. Global Config (`config.toml`)

Placed directly in `LLAMA_PATH` to share parameters across all presets. Supported options include:

- `host` (default: `"0.0.0.0"`)
- `port` (default: `8080`)
- `flash-attn` (default: `"auto"`)
- `kv-quant` (default: `"q8_0"`)
- `models-max` (default: `1`) - Maximum active models loaded concurrently in Router Mode
- `batch-size` (default: `256`)
- `ubatch-size` (default: `256`)
- `threads` (default: auto-detected physical cores)
- `ui` (default: `true`) - Global Web UI enablement toggle

### 2. Model-Specific Configurations (`<model-name>.toml`)

Configured next to a `.gguf` file (e.g. `Qwen2.5-7B.toml` for `Qwen2.5-7B.gguf`).

> [!IMPORTANT]
> **Strict TOML Key Rules & Prefixes**:
>
> - **Strict Keys**: Keys in TOML configurations must not contain underscores (`_`) or start with a dash (`-`). Violating keys are rejected at parse/load time, with warning logs emitted in `load_toml_safe`.
> - **Prefix `lh-`**: Reserved for `llama-herd` custom settings (e.g. `lh-ctx-size`, `lh-is-draft`). These are handled internally and not passed to `llama-server`.
> - **Prefix `s-`**: Used for `llama-server` short option keys (e.g., `s-sps = 0.5`). When launching `llama-server` directly, these are formatted as a single dash (e.g. `-sps 0.5`). When generating `models-preset.ini`, the prefix is stripped (e.g. `sps = 0.5`).
> - **Normal keys (unprefixed)**: Treated as normal long keys. When launching `llama-server`, they are formatted with a double dash (e.g. `slot-prompt-similarity` -> `--slot-prompt-similarity`). When generating `models-preset.ini`, they are written as-is.

#### Supported `lh-` Custom Configuration Keys:

- **Heuristic & Discovery Controls**:
  - `lh-is-draft` / `lh-is-draft-only`: Flag this model as a draft (hides it from the main select list).
  - `lh-is-default`: Designates this model as the default startup selection in the TUI.
  - `lh-draft` / `lh-draft-model`: Specify a draft model filename (or `"none"` / `"false"` to disable draft pairing).
  - `lh-mmproj`: Explicitly define the vision projector model filename to pair with this model.
- **TUI Parameter & Launcher Overrides**:
  - `lh-ctx-size`: Override context size (supports formats like `"8k"` or raw numbers).
  - `lh-ngl`: Override GPU layers count. If set to `"auto"`, falls back to total layers if available.
  - `lh-total-layers`: Total number of layers for layers-based computations.
  - `lh-temp`: Default temperature setting (default: `0.8`).
  - `lh-top-p`: Default top-p setting (default: `0.95`).
  - `lh-top-k`: Default top-k setting (default: `40`).
  - `lh-reasoning`: Enable reasoning formatting (`"on"` maps format to deepseek format, `"off"`, or `"auto"`).
  - `lh-kv-quant`: Override KV cache quantization format (default: `"q8_0"`, maps to `-ctk` and `-ctv` arguments).
  - `lh-spec-type`: Override speculative decoding type (e.g., `"draft-mtp"`, `"draft-simple"`, `"draft-eagle3"`).
  - `lh-spec-draft-n-max`: Override maximum draft tokens to predict per slot (default: `4`).
  - `lh-spec-draft-p-min`: Override minimum speculative decoding probability (default: `0.0`).

---

## Project Mandates & Conventions

### 1. Mandatory Server Parameters

`llama-server` **must** always be launched with `--log-colors on`. This is hard-coded in the launcher (`src/launcher.rs`) to ensure the TUI receives color escape sequences to render.

### 2. Parameter Passthrough & Restrictions

To prevent conflicts with managed options, any key (long, short, prefixed with `s-`, or unprefixed) matching a managed parameter is **restricted** and ignored during the passthrough stage.

- **Restricted Long Option Keys**: `ctx-size`, `total-layers`, `n-gpu-layers`, `kv-quant`, `lh-kv-quant`, `kv-unified`, `lh-kv-unified`, `cache-type-k`, `cache-type-v`, `ngl`, `threads`, `ngld`, `gpu-layers-draft`, `spec-draft-ngl`, `model-draft`, `spec-draft-model`, `is-draft`, `is-default`, `is-draft-only`, `ui`, `webui`, `model`, `chat-template-file`, `mmproj`, `jinja`, `flash-attn`, `version`, `tools`, `batch-size`, `ubatch-size`, `log-colors`, `host`, `port`, `np`, `parallel`, `models-preset`, `models-max`, `models-autoload`, `props`, `temp`, `top-p`, `top-k`, `reasoning`, `reasoning-format`.
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
