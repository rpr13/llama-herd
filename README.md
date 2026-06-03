# 🦙 Llama-Herd (`llama-herd`)

> [!IMPORTANT]
> 🤖✨ **AI-First Software**: This project was designed and built with **very heavy use of agentic AI coding assistants** (including Antigravity, Gemini, and Claude) working in partnership with human developers to coordinate and route local Large Language Models (including many models sourced from Hugging Face). 🦾💻

<p align="center">
  <a href="https://github.com/rpr13/llama-herd/actions"><img src="https://github.com/rpr13/llama-herd/actions/workflows/rust.yml/badge.svg" alt="Build Status"></a>
  <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-blue?logo=linux&logoColor=white" alt="Platforms">
  <img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue" alt="License">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Built_with-Agentic_AI-blueviolet?logo=googlegemini&logoColor=white" alt="AI Assisted">
  <img src="https://img.shields.io/badge/Powered_by-llama.cpp-yellow?logo=openllama&logoColor=black" alt="llama.cpp">
  <img src="https://img.shields.io/badge/API-OpenAI%20Compatible-darkgreen?logo=openai&logoColor=white" alt="OpenAI Compatible">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024-orange?logo=rust&logoColor=white" alt="Rust 2024">
  <img src="https://img.shields.io/badge/UI-Ratatui-red?logo=terminal&logoColor=white" alt="Ratatui">
  <img src="https://img.shields.io/badge/Linter-Clippy-yellow?logo=rust&logoColor=white" alt="Clippy Enforced">
  <img src="https://img.shields.io/badge/Code%20Style-Prettier-ff69b4?logo=prettier&logoColor=white" alt="Prettier">
</p>

**Llama-Herd** is a high-performance, native Rust Terminal User Interface (TUI) and Command Line Interface (CLI) companion for orchestrating, pairing, and routing local Large Language Model (LLM) services driven by `llama.cpp`'s `llama-server`.

---

## Why Llama-Herd?

Running local LLMs via raw `llama-server` command lines is often brittle and manually tedious. `Llama-Herd` orchestrates the lifecycle of your models automatically: it auto-discovers GGUFs, applies naming heuristics to pair draft and vision sub-models, enforces strict TOML parameter safety, parses and displays ANSI log streams live in Ratatui, and routes request traffic dynamically in Multi-Model Router modes.

### Key Capabilities

- 🛠️ **Auto-Discovery & Pairing**: Instantly pairs base models with compatible speculative drafts or vision projectors using filename heuristics, providing interactive selector menus directly on the Dashboard.
- 🔒 **Configuration & Option Safety**: Enforces strict TOML key/value validation, filtering out option/command injection flags, restricting context size suffixes strictly to `'k'`/`'K'`, and performing safe sequential port searches (retrying up to +10).
- ⚙️ **Robust Process Orchestration**: Spawns and monitors `llama-server` subprocesses, tracking active PIDs in a configuration-resident `active_pids.txt` file and terminating zombie instances cleanly using the `sysinfo` library.
- 🖥️ **Interactive Control Center**: Provides a rich Ratatui TUI dashboard showing loaded status, resource allocations, and real-time logs.
- 🎨 **ANSI Log Parser**: Streams subprocess logs directly into terminal frames, parsing graphic coloring codes into styled spans with scroll, pause, and export controls.
- 🎨 **Customizable TUI Theme System**: Features a hybrid functional palette and procedural UI behavior system for a personalized dashboard experience.
- 🔀 **Dynamic Preset Routing**: Coordinates on-demand model loading and routing configurations (governed by generated `models-preset.ini` files).
- 🔄 **Dynamic Hot Reloading**: Automatically scans the models directory at runtime with active-writes stability checks (deferring updates during downloads) and alerts users with TUI warning bars when edits are unsaved.

---

## Quick Start

### Prerequisites

1. **Rust Toolchain**: Install stable Rust using [rustup](https://rustup.rs/) (v1.70+ recommended).
2. **llama-server**: Install `llama.cpp` and build the `llama-server` binary.

### Installation

```bash
# Clone the repository
git clone https://github.com/rpr13/llama-herd.git
cd llama-herd

# Install the binary globally in your Cargo path
cargo install --path .
```

### Execution Commands

```bash
# Start LlamaHerd. An interactive wizard will guide you on first launch.
llama-herd

# Generate models-preset.ini dynamically inside your models directory and exit immediately
llama-herd --ini
```

---

## Configuration Snippet

Llama-Herd reads custom parameters from `.toml` files matching your `.gguf` models (e.g. `Qwen2.5-7B-Instruct.toml` for `Qwen2.5-7B-Instruct.gguf`).

```toml
# Llama-Herd Orchestration Settings
[llama-herd]
is-default = true
draft = "Qwen2.5-1.5B-Instruct.gguf"

# llama-server long option parameters (e.g. --ctx-size, --slot-prompt-similarity, and --spec-type)
[llama-server-long]
ctx-size = "32k"
ngl = "auto"
spec-type = "draft-mtp"
slot-prompt-similarity = 0.5

# llama-server short option parameters (e.g. -sps 0.6)
[llama-server-short]
sps = 0.6
```

---

## Deep Dives & Reference

For comprehensive details on how to design, configure, or optimize Llama-Herd, refer to:

- 🏛️ **[Architecture & System Design](docs/architecture.md)**: Visual flowchart, directory structures, and module architecture breakdown.
- ⚙️ **[Configuration & Performance Optimization](docs/configuration.md)**: Global settings, model config keys list, API routing endpoints, and tuning tips.
- 🎨 **[TUI Theming & Customization](docs/theming.md)**: Design principles, functional palette schema, and procedural UI behaviors.

---

## Credits & Acknowledgments

- **llama.cpp**: The core execution engine for high-performance GGUF quantization inference.
- **Ratatui**: The excellent Terminal UI framework enabling the interactive dashboard interface.
- **Crossterm**: Safe, cross-platform terminal control backend.
- **serde & toml**: Type-safe configuration loading, serialization, and validation.
- **regex**: Powering log parsing, ANSI graphic sequence stripping, and tokenizer heuristic matching.
- **arboard**: Cross-platform system clipboard access for log copying.
- **sysinfo**: Local system checks and subprocess health metrics.
- **AI Assistants**: Deepmind's Antigravity agent, Google's Gemini, and Anthropic's Claude, which were heavily involved in the codebase design, refactoring, and testing.
