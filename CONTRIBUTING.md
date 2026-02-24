# Contributing to Moterm

Thanks for your interest in contributing! Here's how to get started.

## Getting Started

1. Fork the repo
2. Clone your fork: `git clone https://github.com/YOUR_NAME/moterm.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run checks: `cargo check && cargo clippy && cargo test`
6. Commit: `git commit -m "Add my feature"`
7. Push: `git push origin my-feature`
8. Open a Pull Request

## Development Setup

```bash
# Install Rust (if you haven't)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build

# Run in debug mode
cargo run

# Build release
cargo build --release
```

## Code Style

- Follow standard Rust conventions (`cargo fmt` and `cargo clippy`)
- Keep dependencies minimal — think twice before adding a new crate
- Prefer clarity over cleverness
- Comments in Chinese or English are both fine

## What to Work On

Check the [Issues](https://github.com/longzhi/moterm/issues) page. Good first issues are labeled `good first issue`.

### Priority areas:
- **Linux support** — X11 and Wayland compatibility
- **Missing VTE sequences** — better compatibility with programs like htop, tmux, vim
- **Performance** — rendering optimization, memory reduction
- **Tests** — unit tests for terminal state, VTE handling, URL detection

## Architecture Overview

```
src/
├── main.rs          # Event loop, window management
├── config.rs        # TOML config loading
├── pty.rs           # PTY creation and I/O
├── terminal.rs      # Terminal state (grid, cursor, scrollback)
├── vte_handler.rs   # Escape sequence handling (via vte crate)
├── renderer.rs      # Pixel rendering (fontdue + softbuffer)
├── font.rs          # Font loading and discovery
├── input.rs         # Keyboard → escape sequence mapping
├── clipboard.rs     # System clipboard (pbcopy/pbpaste)
├── search.rs        # Cmd+F search
├── url.rs           # URL detection
└── color.rs         # Color definitions and conversion
```

## Design Principles

1. **Minimal dependencies** — every crate added must justify its weight
2. **CPU rendering only** — no GPU, no wgpu, no OpenGL
3. **Event-driven** — zero CPU when idle
4. **Small binary** — target <1MB release build
5. **macOS first** — Linux support is planned but macOS is the primary target

## Pull Request Guidelines

- One feature/fix per PR
- Keep PRs small and focused
- Include a clear description of what changed and why
- Update README.md if you add user-facing features
- Update CHANGELOG.md under `[Unreleased]`

## Reporting Bugs

Open an issue with:
- macOS version
- Moterm version (`moterm --version` or commit hash)
- Steps to reproduce
- Expected vs actual behavior
- Terminal output / screenshots if relevant

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
