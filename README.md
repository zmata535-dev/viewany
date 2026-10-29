# ViewAny

**ViewAny** is a fast, lightweight, and modern image viewer built in Rust with [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

![ViewAny Icon](viewany_icon.png)

## Features

- **Multi-Format Support**: Opens PNG, JPEG, WebP, BMP, GIF, TIFF, ICO, and **SVG/SVGZ** vector graphics.
- **SVG Rendering**: High-quality SVG and SVGZ rendering via [`resvg`](https://github.com/RazrFalcon/resvg) with full system font support. Small icons are automatically upscaled to at least 1024 px for crisp zoom. Modified SVGs are re-exported as embedded-PNG SVGs, preserving the `.svg` extension.
- **Drag & Drop**: Drop any image (including SVG files or SVG text) directly into the application window to view it.
- **Desktop Integration**: Supports Linux "Open With..." file associations and desktop launcher (`viewany.desktop`). SVG MIME types (`image/svg+xml`, `image/svg+xml-compressed`) are registered.
- **Smooth Navigation**: Browse through sibling images in the same folder (including SVGs) using Arrow keys or bottom panel buttons.
- **Zoom & Pan**: Smooth scroll-wheel zooming and keyboard controls (+/-) with 100% reset.
- **Selection Crop**: Click and drag to crop any portion of the image.
- **Transformations**: 90° clockwise rotation and color inversion.
- **History**: Full Undo (<kbd>Ctrl</kbd>+<kbd>Z</kbd>) and Redo (<kbd>Ctrl</kbd>+<kbd>Y</kbd>) stack for edits.
- **Clipboard & File Tools**:
  - Copy image directly to clipboard (<kbd>Ctrl</kbd>+<kbd>C</kbd> or menu).
  - Copy image file path.
  - Open containing folder ("Go to Location") in system file manager.
- **Save As SVG**: Export any loaded image back as an SVG (via embedded PNG data URI).
- **Unobtrusive UI**: Minimalist interface with a right-click context menu and keyboard shortcuts.

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| <kbd>→</kbd> / <kbd>Page Down</kbd> | Next image |
| <kbd>←</kbd> / <kbd>Page Up</kbd> | Previous image |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Open image dialog |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Save modified image |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> | Undo |
| <kbd>Ctrl</kbd> + <kbd>Y</kbd> / <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>Z</kbd> | Redo |
| <kbd>+</kbd> / <kbd>-</kbd> | Zoom in / Zoom out |
| <kbd>Scroll Wheel</kbd> | Zoom in / Zoom out |
| <kbd>Alt</kbd> + <kbd>M</kbd> / <kbd>Right Click</kbd> | Open / Close Context Menu |
| <kbd>Esc</kbd> | Close Menu / Cancel Crop / Exit |

## Building & Installation

Ensure you have Rust installed (via [rustup](https://rustup.rs/)):

```bash
# Clone the repository
git clone https://github.com/zmata535-dev/viewany.git
cd viewany

# Build release binary
cargo build --release

# Run
./target/release/viewany [path-to-image]
```

### Desktop Integration (Linux)

To install ViewAny as your desktop image viewer:

```bash
# Copy binary
cp target/release/viewany ~/.local/bin/

# Copy desktop launcher and icon
cp viewany.desktop ~/.local/share/applications/
cp viewany_icon.png ~/.local/share/pixmaps/viewany.png

# Update desktop application database
update-desktop-database ~/.local/share/applications/
```

## SVG Support Notes

SVG files are rendered at runtime using [`resvg`](https://github.com/RazrFalcon/resvg) — no external tools required. Features include:

- Full CSS, gradients, filters, and clip-path support.
- System fonts used for `<text>` elements (via `fontdb`).
- SVGZ (gzip-compressed SVG) support out of the box.
- Icons smaller than 1024 px are automatically supersampled for crisp display.
- Saving an SVG writes the rasterized content back as an embedded-PNG data URI SVG, preserving the original file extension.

## License

MIT OR Apache-2.0
