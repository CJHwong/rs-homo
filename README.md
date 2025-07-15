# HOMO: Hoss' Opinionated Markdown Output

A fast, native macOS Markdown viewer with streaming support. Pipe Markdown to it, open a file directly, or launch an empty window for quick previews. Built with [cacao](https://github.com/PistonDevelopers/cacao) and [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark).

## Screenshot

![HOMO App Screenshot](./screenshots/screenshot.jpg)

---

## Features

- **Native macOS GUI** (AppKit/WebView)
- **Live streaming**: Pipe Markdown to stdin and see live updates
- **Open files directly**: Pass a markdown file as an argument to view it instantly
- **GitHub-flavored Markdown**: Tables, footnotes, strikethrough, task lists
- **External link handling**: Opens links in your browser

---

## Install

### Download Pre-built Binary (Recommended)

Download the latest release for your platform from the [releases page](https://github.com/yourusername/homo/releases):

- **macOS (Intel)**: `homo-macos-amd64`
- **macOS (Apple Silicon)**: `homo-macos-arm64`

Then make it executable and add to your PATH:

```sh
chmod +x homo-macos-arm64
sudo mv homo-macos-arm64 /usr/local/bin/homo
```

### Build from Source

#### Prerequisites

- Rust (latest stable, [install here](https://rustup.rs/))
- macOS 11+ (Apple Silicon or Intel)

#### Build & Install

```sh
# Clone the repo
$ git clone https://github.com/CJHwong/homo.git
$ cd homo

# Build the app
$ cargo build --release

# Optionally, install to your Cargo bin directory
$ cargo install --path .
```

---

## Usage

### Pipe Markdown to the app

```sh
echo '# Hello, world!' | homo
```

Or stream a file:

```sh
tail -f README.md | homo
```

### Open a Markdown file directly

```sh
homo README.md
```

---

## Development

### Project Structure

- `src/main.rs` — Entry point, handles GUI/streaming mode
- `src/gui/` — GUI logic (window, view, delegate)
- `src/markdown/` — Markdown parsing utilities
- `src/streaming.rs` — Streaming logic for stdin → GUI
- `src/error.rs` — Unified error type

### Running Locally

```sh
cargo run
```

To test streaming mode:

```sh
echo '## Streaming test' | cargo run --
```

---

## User Preferences

HOMO stores user preferences (font size, font family, theme) in macOS UserDefaults. These preferences persist across app launches.

### Managing Preferences

View current preferences:

```bash
defaults read homo StylePreferences
```

Reset preferences to defaults:

```bash
defaults delete homo StylePreferences
```

### Available Preferences

- **Font Family**: System, Menlo, Monaco, Helvetica
- **Font Size**: Adjustable via menu (⌘+ / ⌘- / ⌘0)
- **Theme**: Light, Dark, System (follows system preference)

---

## License

MIT
