# Circle to Search

A minimalist, high-performance, cross-platform "Circle to Search" implementation rewritten in **Rust**. Capture any part of your screen and instantly search for it online using Google Lens.

## ✨ Features
- **Fast & Lightweight**: ~13MB binary (vs ~50MB+ Python), instant startup.
- **Cross-Platform**: Works on Linux (X11 & Wayland), Windows, and macOS.
- **Wayland Support**: Native integration with `grim` for Hyprland/Sway users.
- **Minimal Dependencies**: No heavy Python runtime required for the end user.

## 🚀 Installation & Usage

### 📥 Download Binary
Download the latest portable binary for your OS from the **[Releases](../../releases)** page.

### 🛠️ Build from Source (Rust)
Prerequisites: `cargo` (Rust toolchain).

```bash
cd rust_app
cargo build --release
```
The binary will be located at `rust_app/target/release/circle-to-search`.

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
The original Python implementation is still available in the root directory.


