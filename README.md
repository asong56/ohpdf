# OhPDF

**A PDF tool that lives on your computer.**

No account. No uploads. No subscription. Download, double-click, done.

*No upload. No account. Just PDF.*

---

## Why OhPDF?

The typical path to merge three PDFs: search online → sit through ads → upload your files (privacy?) → prompted to sign up → sign up → hit a file-size limit → give up.

The OhPDF path: double-click → drop files → done.

---

## Features

**MVP (Phase 1)**

| Feature | Description |
|---------|-------------|
| Merge | Combine multiple PDFs into one |
| Split | Split by page range |
| Compress | Reduce file size |
| Encrypt | Set an open password |
| Decrypt | Remove a password |
| Delete Pages | Remove specific pages |
| Extract Pages | Save a page range as a new file |
| Rotate Pages | Rotate specific pages |
| Reorder Pages | Drag and drop to rearrange pages |
| PDF to Images | Export each page as PNG, with optional DPI |
| Images to PDF | Combine PNG / JPG files into a PDF |
| Add Watermark | Semi-transparent text watermark on every page |

---

## Building

### Prerequisites

- [Rust](https://rustup.rs/) 1.77+
- [MuPDF](https://mupdf.com/) development libraries (see platform notes below)

### macOS

```bash
brew install mupdf
cargo build --release
```

### Linux (Ubuntu / Debian)

```bash
sudo apt install libmupdf-dev libwebkit2gtk-4.1-dev
cargo build --release
```

### Windows

```bash
# Install mupdf via vcpkg
vcpkg install mupdf
cargo build --release
```

The output binary is at `target/release/ohpdf` (macOS / Linux) or `target\release\ohpdf.exe` (Windows).

---

## Usage

```
ohpdf          # launch the graphical interface
```

Or just double-click the executable.

### File Picker

Click anywhere in the drop zone to open a native OS file dialog. Drag-and-drop also opens the dialog as a fallback (browsers do not expose real filesystem paths from dropped files, so a native dialog is always used to ensure operations get the correct absolute path).

### Output Files

Output is saved in the same directory as the source file, with a suffix appended automatically:

```
report.pdf  →  report_merged.pdf
report.pdf  →  report_compressed.pdf
```

---

## Architecture

```
Rust
 ├── MuPDF FFI    — PDF engine, statically linked
 ├── src/pdf/     — all PDF operations
 └── src/ipc.rs   — communication with the frontend

wry + tao         — native window + system WebView

ui/index.html     — all UI, single file, no framework, no build step
```

**Only 7 dependencies:** `tao` · `wry` · `mupdf` · `serde` · `serde_json` · `tokio` · `rfd`

### Binary Size Target

| Platform | Target |
|----------|--------|
| Windows (.exe) | < 15 MB |
| macOS | < 15 MB |
| Linux | < 15 MB |

No installer, no registry writes — delete the binary to uninstall.

---

## Design Principles

**Task first, UI second.** Users come to OhPDF to get something done. The interface's job is to make that as fast as possible, then get out of the way.

**Local is a promise, not a feature.** Not "supports local processing" — *only* local processing.

**Restraint over accumulation.** Before adding any feature, ask: in what real situation does a regular person actually need this?

---

## Roadmap

- [x] Phase 1: Core PDF operations (MVP)
- [ ] Phase 2: Annotations, form filling, digital signatures
- [ ] Phase 3: Localization, keyboard shortcuts

---

## Contributing

PRs welcome. Before opening an issue, please confirm:

1. Does this functionality already exist?
2. Does a typical user (someone who occasionally works with PDFs) actually need it?

---

## License

MIT
