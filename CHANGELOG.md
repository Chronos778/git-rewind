# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.6.0](https://github.com/Chronos778/git-rewind/compare/v1.5.0...v1.6.0) - 2026-06-19

### Added

- implement initial CLI structure and core commands for AI-powered repository analysis and commit message generation
- show cached model in diagnostic output

### Fixed

- resolve github actions ci type-check errors and unused warnings
- resolve compilation error due to missing api_base parameter in get_cached_model
- make json flag global and eliminate frozen spinner rendering on api errors
- resolve data loss config overwrite and stale model cache bugs
- resolve orphaned spinner on streaming termination, fix model UI state desync, and safely ignore empty API keys
- ensure JSON formatting integrity across all commands and resolve configuration state leak during first-time interactive setup
- append custom system_prompt to base instructions rather than completely overwriting the agent's identity
- invalidate model cache when API Base URL changes to prevent model mismatch on proxy switching
- correctly prioritize all environment variables above config file keys across providers
- prevent local .rewindrc settings from leaking into global config.json during model cache persistence
- prevent local .rewindrc settings from leaking into global config.json on mutation
- prevent API payload bloat from massive untracked git status output
- print warning when config json parsing fails instead of silently ignoring it
- safely trim trailing slashes from API base globally instead of just in models
- correctly display keys loaded from environment variables in config show command
- refactor git diff size truncation to prevent API bloat on minified files
- print telemetry to stderr when using json flag
- correctly flag conflicted files in git status output
- write info message to stderr to prevent json output corruption
- enforce correct provider priority in ensure_configured
- enforce correct provider priority in api_call
- exclude .rewind-brief.md from diff generation
- resolve .rewindrc from repository root
- prevent double slashes in API base URLs
- optimize large diff generation and include untracked files
- resolve rewind history from repository root

### Other

- run cargo fmt to fix CI pipeline formatting errors
- apply cargo fmt and make key masking robust

## [1.5.0](https://github.com/Chronos778/git-rewind/compare/v1.4.0...v1.5.0) - 2026-06-13

### Added

- implement CLI command structure and AI repository analysis logic in main.rs

## [1.4.0](https://github.com/Chronos778/git-rewind/compare/v1.3.1...v1.4.0) - 2026-06-13

### Added

- implement repository state extraction with pathspec-based diff filtering and ignore support
- implement AI client with support for streaming, retries, and dynamic model discovery
- implement persistent configuration management with support for API keys, custom models, and local overrides
- implement AI client for LLM interaction with streaming, retries, and configuration management

### Other

- enable reqwest feature for self_update dependency
- enable blocking feature for reqwest dependency
- Merge branch 'main' of https://github.com/Chronos778/git-rewind

## [1.3.1](https://github.com/Chronos778/git-rewind/compare/v1.3.0...v1.3.1) - 2026-06-13

### Other

- update reqwest to use rustls-tls and disable default features
- document system-prompt configuration and per-project .rewindrc support

## [1.3.0](https://github.com/Chronos778/git-rewind/compare/v1.2.2...v1.3.0) - 2026-06-13

### Added

- add CI workflow, AI prompt logic, and user configuration management
- implement persistent configuration management and model caching for AI providers
- implement core AI modules, configuration management, and robust release downloading for git-rewind
- implement multi-provider LLM infrastructure with automatic key detection and release workflows

### Other

- add GitHub Actions workflow and update self_update dependency
- downgrade self_update to version 0.41.0 in Cargo.toml
- add GitHub Actions workflow and update MSRV to 1.78

## [1.2.2](https://github.com/Chronos778/git-rewind/compare/v1.2.1...v1.2.2) - 2026-06-10

### Fixed
- Resolved an issue where the installation script would incorrectly download checksum files instead of the release binary.

## [1.2.1](https://github.com/Chronos778/git-rewind/compare/v1.2.0...v1.2.1) - 2026-06-10

### Fixed
- CI pipeline adjustments for binary releases.

## [1.2.0](https://github.com/Chronos778/git-rewind/compare/v1.1.4...v1.2.0) - 2026-06-10

### Added
- **AI Repository Analysis:** The core `rewind` command now analyzes your git repository and generates a concise `.rewind-brief.md` summary of recent changes.
- **Auto-Commit Messages:** Use `rewind commit` to instantly generate conventional git commit messages based on your staged diffs.
- **Interactive Queries:** Use `rewind ask "query"` to ask questions specifically about your codebase context.
- **Multi-Provider Support:** Automatically discover and utilize the best available models from Groq, Google Gemini, or OpenAI using your API key.
- **Easy Installation:** Added cross-platform installation scripts and pre-compiled binaries for macOS, Linux, and Windows.
