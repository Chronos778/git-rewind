# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Streaming output** — `rewind` and `rewind ask` now stream AI responses in real-time instead of waiting for the full response.
- **`--verbose` / `-v` flag** — Show diagnostic info (provider, API base, model) for troubleshooting.
- **`--no-save` flag** — Skip saving the brief to `.rewind-brief.md`.
- **Secure key entry** — `rewind config set <provider>` now prompts for the key securely if omitted, keeping it out of shell history.
- **CI workflow** — Automated `cargo fmt`, `clippy`, and `test` checks on every push and PR.
- **Unit tests** — 18 tests covering provider parsing, config operations, prompt building, and text truncation.
- **`CHANGELOG.md`** — This file.

### Changed
- **Gemini default model** updated from `gemini-1.5-flash` to `gemini-2.0-flash`.
- **Token estimation** (`rewind estimate`) now uses an improved heuristic (word + character averaging) and shows a disclaimer.
- **`.rewindignore`** now resolves relative to the git repo root, so it works when running `rewind` from a subdirectory.
- **Codebase restructured** — Monolithic `ai.rs` (424 lines) split into 5 focused modules (`config.rs`, `ai/mod.rs`, `ai/client.rs`, `ai/models.rs`, `ai/prompts.rs`).
- **Provider enum** centralizes all provider-specific logic (API bases, default models, env var names), making it trivial to add new providers.
- **Named constants** replace magic numbers for diff limits, token caps, and timeouts.

### Fixed
- **Silent error swallowing** — Failed git commands, config saves, and brief file writes now log warnings to stderr instead of being silently ignored.
- **Zombie process risk** — `run_git_limited` now uses an RAII guard to ensure child processes are cleaned up even on panic.
- **`Cargo.toml`** — Added `rust-version` (MSRV), `homepage`, and `exclude` fields.
