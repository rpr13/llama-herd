# AGENTS.md - Instructions & Conventions for AI Coding Agents

This document defines architectural boundaries, coding standards, quality gates, and operational mandates for AI agents (and human contributors) working on the `llama-herd` codebase.

---

## 1. Project Identity & Architecture Boundaries

**LlamaHerd** is a native Rust TUI & CLI companion for orchestrating, pairing, and routing local LLM services driven by `llama.cpp`'s `llama-server`. It manages process lifecycles, health monitoring, auto-recovery, dynamic preset generation (`models-preset.ini`), and ANSI log streaming.

### Explicit Architectural Boundaries:
- **Direct Client Connection (No Proxy)**: Clients connect directly to `llama-server` (or its router gateway). LlamaHerd does NOT act as an in-line HTTP reverse proxy for inference tokens.
- **No Parameter Mutation on Auto-Recovery**: If `llama-server` crashes, the supervisor restarts it with **identical, immutable launch parameters** (`SupervisorConfig`). It must NEVER alter VRAM allocation flags, context size, or `--n-gpu-layers` during recovery.
- **Low-Frequency Polling**: Host RAM and GPU VRAM stats are polled at low frequency (every 2 seconds) to avoid CPU spikes and lock contention.
- **Local Metrics Only**: Status tracking is local (subprocess PID checks, stdout/stderr stream parsing, REST `/health` checks). Do not scrape or poll heavy telemetry or sparklines.
- **No LoRA Adapters**: Dynamic LoRA adapter discovery and runtime injection are out of scope.

---

## 2. Core System Components

| Module | Purpose |
| :--- | :--- |
| `src/main.rs` | CLI entry point, argument parsing (`--ini`), global configuration loading, and TUI lifecycle. |
| `src/launcher.rs` | Builds command-line arguments for single-model and router modes, tracks active PIDs (`active_pids.txt`), and terminates stray processes. |
| `src/discovery.rs` | Scans `models_dir`, runs pairing heuristics (draft models, `mmproj` vision projectors with precision/quant token stripping), detects GPU hardware topology (CUDA, ROCm, WDDM), and generates `models-preset.ini` with variant filtering. |
| `src/config.rs` | Enforces strict TOML rules, key/value injection guards, context size parsing, and restricted parameter lists. |
| `src/health.rs` | Polling engine for `GET /health`, classifying `HealthState` (`Healthy`, `Loading`, `Unhealthy`, `Recovering`). |
| `src/control.rs` | REST control dispatcher sending cancellation signals to `/v1/chat/completions/control` (triggered via `F4`). |
| `src/setup.rs` | Interactive setup wizard for first-time path configuration. |
| `src/tui/logs.rs` | Concurrent stdout/stderr log consumer, ANSI SGR sequence parser, log ring buffer, and subprocess supervisor thread. |
| `src/tui/app.rs` | Application state machine, model selection, parameter override state, and models folder watcher. |
| `src/tui/ui.rs` | Ratatui rendering engine, layout management, status badge styling, and dialogs. |
| `src/tui/theme.rs`| Hybrid Theme System (Functional Palette + Procedural UI). |
| `src/tui/picker.rs`| Interactive file and directory picker modal. |

---

## 3. Mandatory Development Rules

### Rule 1: Mandatory Theme Adherence
- **NO HARDCODED COLORS**: Hardcoding colors like `Color::Cyan`, `Color::White`, or `Color::Red` in `src/tui/ui.rs` or any UI component is strictly forbidden.
- Always use `state.theme.<property>` or pass a reference to the `Theme` struct.
- Map all new visual elements to an appropriate semantic field in the `Theme` struct.

### Rule 2: Mandatory Server Launch Flags
- `llama-server` **must** always be launched with `--log-colors on`. This is hard-coded in `src/launcher.rs` so that the ANSI log parser receives styled color escape codes.

### Rule 3: Strict TOML Key/Value Rules & Restrictions
- **No Underscores or Leading Dashes**: Keys in model TOML configs must not contain underscores (`_`) or start with a dash (`-`). Violating keys must be rejected/ignored at load time.
- **Injection Guard**: Option values must not start with `--` or `-` followed by alphabetic characters, and must not contain shell metacharacters (`;`, `&`, `|`).
- **Context Size Parsing**: Suffixes for `ctx-size` are strictly restricted to `'k'`/`'K'` (multiplying by 1024). Other suffixes (`'M'`, `'G'`) or non-numeric/negative values must be rejected.
- **Restricted Keys**: Any parameter managed by LlamaHerd (including `variants`, orchestrator settings, and server options) is strictly restricted from arbitrary passthrough to prevent duplicate/conflicting flags.

### Rule 4: Windows & Cross-Platform Compatibility
- When querying process status or health endpoints, always account for Windows socket behaviors:
  - Do NOT overwrite a verified `HEALTHY` status with `UNHEALTHY` due to a transient socket timeout or polling blip.
  - Process termination on Windows uses `taskkill /F /PID <pid> /T` for complete process tree cleanup. On Unix, `process.kill()` is used.
- Paths must be converted safely between platform formats (`Path` and `PathBuf`).

### Rule 5: Keyboard Shortcuts Standard
- **Tab switching**: `F1` (Dashboard), `F2` (Settings), `F3` (Logs). Never use plain numeric keys `1`, `2`, `3` as global shortcuts to avoid intercepting text input.
- **Model selection & variant cycling**:
  - `↑` / `↓` (Left panel): Navigate between base models.
  - `←` / `→` (Left panel): Cycle between available variants for selected model (`[B]`, `[D]`, `[V]`, `[DV]`).
  - `Tab` / `Enter`: Switch focus to Preset Details & Parameters editing (`Esc` to return to Models).
- **Server operations**:
  - `F4`: Cancel active inference generation via REST control signal.
  - `F5`: Start selected preset model.
  - `F6`: Start router mode server.
  - `F7`: Restart active server process.
  - `F8`: Stop running server.
- **Log viewer controls**: `A` / `Space` (auto-scroll toggle), `P` (pause), `W` (wrap toggle), `C` (copy to clipboard).

---

## 4. Quality Gates & Verification

Before submitting any code changes, all of the following quality gates must pass:

1. **Formatting**: `cargo fmt --check`
2. **Linter**: `cargo clippy -- -D warnings` (must be completely clean with zero warnings)
3. **Tests**: `cargo test` (all unit and integration tests must pass)
4. **Documentation**: Whenever adding features, modifying CLI flags, or altering TUI behaviors, update `README.md`, `GEMINI.md`, and all relevant guides in `docs/`.
