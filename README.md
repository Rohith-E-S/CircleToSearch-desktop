# Circle to Search

A minimalist, cross-platform "Circle to Search" implementation. Capture any part of your screen and instantly search for it online.

## 🚀 Quick Start
Download the latest portable binary for your OS from the **[Releases](../../releases)** page.
- **Linux**: `CircleToSearch` (Requires `grim` for Wayland)
- **Windows**: `CircleToSearch.exe`

## 🛠️ Run from Source
If you prefer running the script directly:
```bash
pip install -r requirements.txt
python main.py
```

## 🖱️ Usage
1. Run the application.
2. **Left-Click & Drag** to select a region.
3. **Release** to search in your default browser.
4. Press **Escape** to cancel.

## 🏗️ Build from Source
To generate your own optimized binary:
```bash
python build_apps.py
```

## Dependencies
- **PyQt6**: UI and image processing.
- **mss**: Screen capture.
- **grim**: (Linux/Wayland only) Recommended for Hyprland/Sway.