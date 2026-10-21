# Circle to Search

A minimalist, high-performance, cross-platform "Circle to Search" implementation rewritten in **Rust**. Capture any part of your screen and instantly search for it online using Google Lens.

## ✨ Features
- **Fast & Lightweight**: ~13MB binary (vs ~50MB+ Python), instant startup.
- **Cross-Platform**: Works on Linux (X11 & Wayland), Windows, and macOS.
- **Wayland Support**: Native integration with `grim` for Hyprland/Sway users, capturing the output you are looking at.
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

Selections smaller than a few pixels are ignored, so a stray click never uploads
anything. If the upload or the browser launch fails, the overlay stays open and
shows the reason so you can try again.

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
The original Python implementation is available in `python_src` and requires
Python 3, PyQt6, and mss 10 or newer.

```bash
python3 -m pip install PyQt6 'mss>=10'
python3 python_src/main.py
```

## Privacy and platform notes

Releasing a valid selection uploads that region to the third-party host uguu.se and opens it in Google Lens. Avoid selecting sensitive information. Escape closes the Rust overlay even during a search, but cannot undo data already uploaded.

On wlroots-based Wayland desktops, install `grim` for capture. The focused output is captured on Hyprland and Sway; elsewhere the whole virtual desktop is captured and letterboxed into the overlay, so the image is never stretched. Other Wayland compositors depend on the available capture backend; portal-based capture is not implemented. X11, macOS and Windows use the `xcap` backend, and macOS requires screen-recording permission.

## Development checks

From `rust_src`, run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked`.
From the repository root, run `python3 -B -m unittest discover -s python_src -p 'test_*.py' -v`. The legacy Python tests mock Qt and external services; `python_src/test_selection.py` covers the selection geometry without needing Qt at all.

The Rust and Python selection geometry is kept in sync deliberately: both are
covered by the same test vectors.

## Releasing

Every push to `main` publishes a rolling release tagged
`v<version>-build.<run number>`. To publish a named version:

1. Bump `version` in `rust_src/Cargo.toml`.
2. Tag the commit (`git tag v1.2.0 && git push origin v1.2.0`).

The workflow uses the pushed tag as the release tag, and every binary is
stamped with the version and the commit it was built from.

## Building from source

```bash
cd rust_src
cargo build --release
```

On Linux the build needs the X11/Wayland development packages, for example on
Debian/Ubuntu:

```bash
sudo apt-get install -y pkg-config libx11-dev libxcb1-dev libxrandr-dev \
  libgtk-3-dev libclang-dev libdbus-1-dev libegl-dev libpipewire-0.3-dev \
  libwayland-dev
```
