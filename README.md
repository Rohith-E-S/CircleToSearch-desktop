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

## Launch Setup for Hyprland
1. Download portable binary from **[Releases](../../releases)** to desired location
2. Copy the absolute path of the binary file.
3. Add the following line to your Hyprland configuration file:
   ```ini
   bind = CTRL , PRINT ,exec , /path/to/CircleToSearch
   ```
4. Now `CTRL` + `PRINT` triggers the Circle to Search overlay.

## Launch Setup for GNOME
1. Open **Settings** > **Keyboard** > **View and Customize Shortcuts**.
2. Scroll down and select **Custom Shortcuts**.
3. Click **Add Shortcut** (+).
4. Fill in the details:
   - **Name**: Circle to Search
   - **Command**: `/path/to/CircleToSearch` (Replace with your actual path)
   - **Shortcut**: Set your preferred keys (e.g., `Ctrl` + `Print`)
5. Click **Add**.

## Launch Setup for KDE Plasma
1. Open **System Settings** > **Shortcuts** > **Custom Shortcuts**.
2. Right-click in the list > **New** > **Global Shortcut** > **Command/URL**.
3. Name it **Circle to Search**.
4. In the **Trigger** tab, set your shortcut (e.g., `Ctrl` + `Print`).
5. In the **Action** tab, enter the command:
   `/path/to/CircleToSearch` (Replace with your actual path)
6. Click **Apply**.

## 🖱️ Usage
1. Run the application.
2. **Left-Click & Drag** to select a region.
3. **Release** to search in your default browser.
4. Press **Escape** to cancel.


## Dependencies
- **PyQt6**: UI and image processing.
- **mss**: Screen capture.
- **grim**: (Linux/Wayland only) Recommended for Hyprland/Sway.
