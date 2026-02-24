<p align="center">
  <img src="assets/icon-512.png" width="128" height="128" alt="Moterm Icon">
  <h1 align="center">Moterm</h1>
  <p align="center">A minimal, blazing-fast terminal emulator written in Rust</p>
</p>

<p align="center">
  <a href="https://github.com/longzhi/moterm/releases"><img src="https://img.shields.io/github/v/release/longzhi/moterm?style=flat-square" alt="Release"></a>
  <a href="https://github.com/longzhi/moterm/blob/main/LICENSE"><img src="https://img.shields.io/github/license/longzhi/moterm?style=flat-square" alt="License"></a>
  <a href="https://github.com/longzhi/moterm/actions"><img src="https://img.shields.io/github/actions/workflow/status/longzhi/moterm/ci.yml?style=flat-square" alt="CI"></a>
</p>

---

## Why Moterm?

Most modern terminals are feature-rich but heavy. Moterm takes the opposite approach: **do less, but do it well.**

- **~710KB** binary (vs 100MB+ for Ghostty/WezTerm)
- **CPU-only rendering** — no GPU required, minimal resource usage
- **Starts instantly**, stays out of your way
- **Zero config needed** — works great out of the box, customize if you want

## Features

| Feature | Details |
|---------|---------|
| **Colors** | 256 + truecolor |
| **Cursor styles** | Block, beam, underline (auto-switches for vim, etc.) |
| **Copy/Paste** | Cmd+C / Cmd+V with bracketed paste |
| **Search** | Cmd+F with match highlighting |
| **URL detection** | Cmd+click to open links |
| **Font zoom** | Cmd+= / Cmd+- / Cmd+0 |
| **Selection** | Click-drag, double-click word, triple-click line, Cmd+A |
| **Scrollback** | 2000 lines + mouse wheel |
| **Config file** | TOML-based, optional |
| **Nerd Font** | Auto-detects installed Nerd Fonts for prompt icons |

## Installation

### Homebrew (macOS)

```bash
brew tap longzhi/tap
brew install --cask moterm
```

This installs `Moterm.app` to `/Applications` with the app icon, Launchpad, and Dock support.

### From source

```bash
git clone https://github.com/longzhi/moterm.git
cd moterm
cargo build --release
cp target/release/moterm /usr/local/bin/
```

### Requirements

- macOS 12+ (Linux support planned)
- Rust 1.70+ (for building from source)
- A monospace font (auto-detects system fonts, prefers Nerd Fonts)

## Configuration

Moterm works with zero configuration. Optionally, create `~/.config/moterm/config.toml`:

```toml
[font]
family = "FiraCode Nerd Font Mono"  # optional, auto-detects if omitted
size = 14                            # logical points (scaled for HiDPI)

[window]
width = 960
height = 600

[cursor]
style = "block"  # block | beam | underline

[colors]
background = "#1e1e2e"
foreground = "#cdd6f4"
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+C` | Copy selection |
| `Cmd+V` | Paste (bracketed) |
| `Cmd+A` | Select all |
| `Cmd+F` | Search (Enter: next, Cmd+G: next, Cmd+Shift+G: prev, Esc: close) |
| `Cmd+=` | Zoom in |
| `Cmd+-` | Zoom out |
| `Cmd+0` | Reset zoom |
| `Cmd+K` | Clear scrollback |
| `Cmd+Q` | Quit |
| `Cmd+Click` | Open URL |
| `Double-click` | Select word |
| `Triple-click` | Select line |
| `Shift+PageUp/Down` | Scroll by page |

## Architecture

```
winit (window)  →  softbuffer (CPU pixel buffer)
fontdue (glyph rasterization + LRU cache)
vte (escape sequence parsing)
libc forkpty (PTY management)
polling (async I/O)
```

No GPU. No async runtime. No heavy GUI framework. Just pixels and file descriptors.

## Performance

| Metric | Moterm | Ghostty | Alacritty |
|--------|--------|---------|-----------|
| Binary size | ~710KB | ~30MB | ~10MB |
| Cold start memory | ~10MB | ~100MB | ~30MB |
| Dependencies | 8 | 100+ | 50+ |

## Roadmap

- [ ] Linux support (X11/Wayland)
- [ ] Tab support
- [ ] Split panes
- [ ] Ligature support
- [ ] Custom color schemes
- [ ] Clickable file paths
- [ ] Session persistence

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a PR.

## License

MIT — see [LICENSE](LICENSE) for details.

## Acknowledgments

Built on the shoulders of:
- [winit](https://github.com/rust-windowing/winit) — Cross-platform window management
- [softbuffer](https://github.com/rust-windowing/softbuffer) — CPU-based pixel buffer
- [fontdue](https://github.com/mooman219/fontdue) — Font rasterization
- [vte](https://github.com/alacritty/vte) — Terminal escape sequence parser
