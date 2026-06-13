# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
