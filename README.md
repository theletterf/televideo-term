# Televideo CLI

A high-performance command-line interface for browsing RAI Televideo, the Italian teletext service, written in Rust.

## Features

- **High-quality image rendering** with automatic terminal protocol detection (iTerm2, Kitty, Sixel, or Unicode half-blocks)
- **Full-screen TUI** with fixed header and footer bars
- **Fast navigation** with arrow keys and direct page jumping
- **5-minute caching** for faster page loads
- **Navy blue status bars** with white text, classic teletext style

## Building the project

### Prerequisites

- Rust 1.70 or higher
- A terminal that supports 24-bit true color
- Recommended terminals:
  - iTerm2 (macOS)
  - Terminal.app (macOS)
  - Windows Terminal (Windows)
  - Kitty, Alacritty, or similar (Linux)

### Build from source

```bash
cargo build --release
```

The binary will be at `target/release/televideo`.

### Install globally

```bash
cargo install --path .
```

## Usage

Run the application:

```bash
./target/release/televideo
```

Or if installed globally:

```bash
televideo
```

### Navigation

- **← / →** - Previous/Next page
- **↑ / ↓** - Previous/Next sub-page
- **0-9** - Type a page number (100-899)
- **Enter** - Go to the typed page number
- **Backspace** - Delete last digit of page number
- **Escape** - Clear page number input
- **c** - Clear image cache
- **q** or **Ctrl+C** - Quit the application

## How it works

The application:

1. Fetches PNG images from RAI's Televideo servers
2. Automatically detects terminal capabilities and uses the best available rendering method:
   - iTerm2 inline images protocol (best quality)
   - Kitty graphics protocol
   - Sixel protocol
   - Unicode half-blocks (▄) as fallback
3. Uses `ratatui` for precise TUI layout control (header always at top, footer at bottom)
4. Implements a 5-minute cache to reduce server requests

### URL pattern

```
http://www.televideo.rai.it/televideo/pub/tt4web/Nazionale/16_9_page-{PAGE}[.{PART}].png
```

## Technical details

- **Language**: Rust
- **TUI framework**: `ratatui` (Terminal User Interface library)
- **Image rendering**: `ratatui-image` (automatic protocol detection)
- **Terminal backend**: `crossterm` (cross-platform terminal manipulation)
- **HTTP client**: `reqwest` (blocking mode)
- **Image processing**: `image` crate for PNG decoding

### Why Rust?

The Rust version provides:
- **Precise layout control** - ratatui's constraint-based layout prevents scrolling issues
- **Better performance** - compiled binary with zero runtime overhead
- **Memory safety** - no garbage collection pauses
- **Cross-platform** - works on macOS, Linux, and Windows
- **Superior image quality** - native terminal graphics protocols when available

## License

ISC
