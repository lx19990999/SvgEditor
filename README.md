# SVG Editor

A cross-platform SVG editor built with Rust and [egui](https://github.com/emilk/egui). Supports Windows, Linux, and macOS.

## Features

- **SVG Loading & Editing** — Open SVG files, view and edit paths, fill colors, stroke colors and widths
- **Path Editing** — Drag control points (endpoints and bezier handles) to reshape paths
- **Drawing Tool** — Click to add points and create new paths, with close-path support
- **Path Source Editor** — Directly edit SVG path `d` attribute in a text editor
- **Transform** — Per-path translate, scale (with aspect-ratio lock), rotate with configurable pivot
- **Canvas Properties** — Adjustable width, height, and background color
- **Export** — Save as SVG, PNG, or JPG
- **Transparent Background** — Photoshop-style checkerboard pattern for transparent areas
- **DPI Scaling** — Auto-detects screen resolution; manual +/- adjustment (0.5 step)
- **Multi-language** — English and 简体中文, auto-detects system locale, manual switch in toolbar
- **Dark/Light Theme** — Follows system theme by default, manual switch in toolbar
- **HiDPI Support** — Automatic scaling for high-resolution displays
- **Undo/Redo** — Ctrl+Z / Ctrl+Y with up to 100 history states
- **File Drag & Drop** — Drop SVG files onto the window to open
- **Persistent Config** — Saves DPI, language, and theme to `~/.config/svgeditor.json`

## Download

Download the latest release for your platform from the [Releases](../../releases) page.

| Platform | File |
|----------|------|
| Windows  | `svg-editor-windows-amd64.exe` |
| Linux    | `svg-editor-linux-amd64` |
| macOS (ARM) | `svg-editor-macos-arm64` |

## Build from Source

### Prerequisites

- Rust 1.92+ (install via [rustup](https://rustup.rs/))

**Linux additional dependencies:**

```bash
# Debian/Ubuntu
sudo apt-get install libwayland-dev libxkbcommon-dev libgtk-3-dev libssl-dev

# Fedora
sudo dnf install wayland-devel libxkbcommon-devel gtk3-devel openssl-devel
```

### Build

```bash
git clone https://github.com/YOUR_USERNAME/svg-editor.git
cd svg-editor

# Linux (requires wayland/x11 features)
cargo build --release --features "eframe/wayland,eframe/x11"

# Windows / macOS
cargo build --release
```

The binary will be at `target/release/svg-editor` (or `svg-editor.exe` on Windows).

### Run

```bash
# Linux
cargo run --release --features "eframe/wayland,eframe/x11"

# Windows / macOS
cargo run --release
```

## Usage

1. **Open a file** — File → Open, or drag & drop an SVG file onto the window
2. **Select a path** — Click on a path in the canvas or in the left panel path list
3. **Edit properties** — Use the right panel to change fill/stroke colors, stroke width
4. **Edit path source** — Expand "Path Source" in the right panel to edit the SVG `d` attribute directly
5. **Transform** — Expand "Transform" to adjust position, scale, rotation per path
6. **Draw new paths** — Click the ✏ Draw button, click points on canvas, double-click or Enter to finish
7. **Export** — File → Export as PNG / Export as JPG

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Scroll wheel` | Zoom in/out (over canvas) |
| `Middle mouse drag` | Pan canvas |
| `Space + drag` | Pan canvas |
| `Enter` | Finish drawing path |
| `Escape` | Cancel drawing |

## Configuration

Configuration is stored at:

- **Linux/macOS:** `~/.config/svgeditor.json`
- **Windows:** `%USERPROFILE%\.config\svgeditor.json`

```json
{
  "dpi": 1.5,
  "language": "zh-CN",
  "theme": "System"
}
```

## Project Structure

```
src/
├── main.rs          # Entry point, window setup, font loading
├── app.rs           # Main app state, UI layout, toolbar, menus
├── canvas.rs        # Canvas rendering, zoom/pan, drawing tool
├── config.rs        # App config (DPI, language, theme), persistence
├── history.rs       # Undo/redo history
├── i18n.rs          # Translations (en, zh-CN)
├── panels.rs        # Side panels (path list, properties)
├── path_editor.rs   # Control point dragging, hit testing
└── svg_doc.rs       # SVG document model, parsing, export
```

## Bundled Font

This application bundles [Noto Sans CJK SC](https://github.com/notofonts/noto-cjk) for Chinese text rendering support. The font is embedded into the binary at compile time via `include_bytes!`.

- **Font file:** `fonts/NotoSansSC-Regular.ttf` (Noto Sans CJK SC Regular, ~16MB)
- **License:** [OFL-1.1](https://github.com/notofonts/noto-cjk/blob/main/LICENSE)
- **Source:** [googlefonts/noto-cjk](https://github.com/googlefonts/noto-cjk)

## License

MIT

### Third-party licenses

- [egui](https://github.com/emilk/egui) — MIT OR Apache-2.0
- [Noto Sans CJK SC](https://github.com/notofonts/noto-cjk) — OFL-1.1
- [resvg](https://github.com/nickel-org/resvg) — MPL-2.0
- Other dependencies — see `cargo license` for details
