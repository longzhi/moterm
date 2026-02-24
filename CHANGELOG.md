# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-02-24

### Added
- Initial release
- PTY support with shell spawning (zsh/bash)
- VTE escape sequence parsing (colors, cursor movement, screen clearing)
- CPU-only rendering via softbuffer + fontdue
- 256 color + truecolor support
- Cursor styles: block, beam, underline (auto-switching via DECSCUSR)
- Copy (Cmd+C) and paste (Cmd+V) with bracketed paste
- Select all (Cmd+A)
- Double-click word selection, triple-click line selection
- Mouse drag selection
- Search (Cmd+F) with match highlighting
- URL detection with Cmd+click to open
- Font zoom: Cmd+= / Cmd+- / Cmd+0
- Clear scrollback: Cmd+K
- Quit: Cmd+Q
- Dynamic window title via OSC 0/2
- Bell support (system beep)
- Scrollback buffer (2000 lines) with mouse wheel scrolling
- Configuration file: ~/.config/moterm/config.toml
- Auto-detect Nerd Fonts for prompt icons
- TERM_PROGRAM=moterm environment variable
- ~710KB release binary

[Unreleased]: https://github.com/longzhi/moterm/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/longzhi/moterm/releases/tag/v0.1.0
