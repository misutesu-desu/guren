# Guren

English · [Türkçe](README.tr.md)

A fast photo viewer for Windows, written in Rust with egui. Photos are decoded on background threads and the two photos on each side of the current one are prepared ahead of time. Zoom and pan stay the same when you move to the next photo, so you can compare a burst of shots at the same spot.

Opens JPEG, PNG, WebP, GIF, BMP, TIFF and ICO, and applies the EXIF orientation.

## Download

Get `guren.exe` from the [latest release](https://github.com/misutesu-desu/guren/releases/latest). No installer, just run it.

## Building

```bash
cargo build --release
```

The binary ends up in `target/release/guren.exe`. Pass a file or folder as an argument, drop one on the window or pick one with Ctrl+O. Opening a file lines up every photo in its folder.

## Shortcuts

| Key | Action |
|---|---|
| → · Space · Page Down | Next |
| ← · Backspace · Page Up | Previous |
| Home · End | First · last |
| Mouse wheel | Zoom towards the cursor |
| Drag · ↑ ↓ | Pan |
| + · − | Zoom in · out |
| 0 | Fit to window |
| 1 | Actual size |
| Double click | Toggle between fit and actual size |
| F · F11 · Esc | Full screen |
| T | Theme: system, dark, light |
| L | Language: English, Turkish |
| Ctrl+O | Open |

The back and forward mouse buttons also switch photos. The interface starts in Turkish when the system language is Turkish and in English otherwise.
