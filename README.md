# HOMO: Hoss' Opinionated Markdown Output

A fast, native macOS Markdown viewer with streaming support. Pipe Markdown to it or launch an empty window for quick previews. Built with [cacao](https://github.com/PistonDevelopers/cacao) and [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark).

---

## Features

- **Native macOS GUI** (AppKit/WebView)
- **Live streaming**: Pipe Markdown to stdin and see live updates
- **GitHub-flavored Markdown**: Tables, footnotes, strikethrough, task lists
- **External link handling**: Opens links in your browser

---

## Install

### Prerequisites

- Rust (latest stable, [install here](https://rustup.rs/))
- macOS 11+ (Apple Silicon or Intel)

### Build & Install

```sh
# Clone the repo
$ git clone https://github.com/yourname/homo.git
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

### Launch an empty window

```sh
homo
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

## License

MIT
