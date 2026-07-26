//! `LlamaHerd` library containing configuration, model discovery, server launching, and TUI components.

#![forbid(unsafe_code)]
#![allow(clippy::multiple_crate_versions)]

/// Configuration structures and helpers for parsing GGUF/TOML config files.
pub mod config;
/// Direct control dispatcher for sending runtime commands to llama-server.
pub mod control;
/// Heuristics and utilities for scanning directories and discovering model presets.
pub mod discovery;
/// Health monitoring engine for llama-server instances.
pub mod health;
/// Orchestrates running llama-server subprocesses and command line construction.
pub mod launcher;
/// CLI wizard for setting up initial paths and settings.
pub mod setup;
/// Text User Interface components and event loop.
pub mod tui;
