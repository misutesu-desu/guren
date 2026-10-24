# Guren

English · [Türkçe](README.tr.md)

A fast photo viewer for Windows, written in Rust with egui. Photos are decoded on background threads and the two photos on each side of the current one are prepared ahead of time. Zoom and pan stay the same when you move to the next photo, so you can compare a burst of shots at the same spot.

Opens JPEG, PNG, WebP, GIF, BMP, TIFF and ICO, and applies the EXIF orientation.

## Download

Get `guren.exe` from the [latest release](https://github.com/misutesu-desu/guren/releases/latest). No installer, just run it.

## Default viewer

Open the ⋯ menu in the top right and choose **Set as default photo viewer**. Guren registers itself for your user account, no admin rights needed, and the Windows Default apps page opens. Windows does not let programs make themselves the default, so pick Guren there to finish. **Remove from Open with** in the same menu undoes the registration. If you move `guren.exe`, it updates the registered path on the next start.

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
