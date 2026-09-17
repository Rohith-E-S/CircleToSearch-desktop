# Circle to Search

A minimalist, high-performance, cross-platform "Circle to Search" implementation rewritten in **Rust**. Capture any part of your screen and instantly search for it online using Google Lens.

## ✨ Features
- **Fast & Lightweight**: ~13MB binary (vs ~50MB+ Python), instant startup.
- **Cross-Platform**: Works on Linux (X11 & Wayland), Windows, and macOS.
- **Wayland Support**: Native integration with `grim` for Hyprland/Sway users.
- **Async Search**: Non-blocking image upload with a visual loading spinner.
- **Minimal Dependencies**: No heavy Python runtime required for the end user.

## 🚀 Installation & Usage

### 📥 Download Binary
**No installation or building required.**
Simply download the latest portable binary for your OS from the **[Releases](../../releases)** page.

### 🖱️ Usage
1. Run the application (bind it to a shortcut like `Ctrl + Print` or `Super + S`).
2. **Left-Click & Drag** to select a region on your screen.
3. **Release** to automatically upload and search in your default browser.
4. Press **Escape** to cancel.

## 🔧 Integration Setup

### Hyprland (Linux)
Add this to your `hyprland.conf`:
```ini
bind = CTRL, PRINT, exec, /path/to/circle-to-search
```

### GNOME (Linux)
1. **Settings** > **Keyboard** > **View and Customize Shortcuts** > **Custom Shortcuts**.
2. Add new:
   - **Command**: `/path/to/circle-to-search`
   - **Shortcut**: `Ctrl + Print`

### KDE Plasma (Linux)
1. **System Settings** > **Shortcuts** > **Custom Shortcuts**.
2. Create new **Global Shortcut** > **Command/URL**.
3. **Action**: `/path/to/circle-to-search`

### Windows
Create a shortcut to the `.exe` and assign a hotkey in the shortcut properties.

---

## 🐍 Legacy Python Version
The original Python implementation is available in `python_src` and requires Python 3, PyQt6, and mss.

```bash
python3 -m pip install PyQt6 mss
python3 python_src/main.py
```

## Privacy and platform notes

Releasing a valid selection uploads that region to the third-party host uguu.se and opens it in Google Lens. Avoid selecting sensitive information. Escape closes the Rust overlay even during a search, but cannot undo data already uploaded.

On wlroots-based Wayland desktops, install `grim` for capture. Other Wayland compositors depend on the available capture backend; portal-based capture is not implemented. Capture and fullscreen-window placement may differ on multi-monitor setups. macOS requires screen-recording permission.

## Development checks

From `rust_src`, run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked`.
From the repository root, run `python3 -B -m unittest discover -s python_src -p 'test_*.py' -v`; these legacy tests mock Qt and external services.
