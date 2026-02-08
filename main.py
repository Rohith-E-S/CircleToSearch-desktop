import sys
import os
import subprocess
import platform
import shutil
from PyQt6.QtWidgets import QApplication, QWidget
from PyQt6.QtCore import Qt, QPoint, QRect
from PyQt6.QtGui import QPainter, QPen, QColor, QPixmap, QCursor
from config import Config
from search_service import SearchService

class SelectionOverlay(QWidget):
    def __init__(self, screenshot_path, geometry=None):
        super().__init__()
        self.screenshot_path = screenshot_path
        self.dragging = False
        self.drag_start = QPoint()
        self.current_pos = QPoint()
        
        # UI Setup
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.WindowStaysOnTopHint)
        self.setWindowState(Qt.WindowState.WindowFullScreen)
        self.setCursor(Qt.CursorShape.CrossCursor)
        
        # Load background
        self.pixmap = QPixmap(screenshot_path)
        
        if geometry:
            self.setGeometry(geometry)

        # Colors
        self.selection_color = QColor(Config.SELECTION_COLOR)
        self.overlay_color = QColor(Config.OVERLAY_COLOR)

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        
        # Draw background screenshot
        # If the window is fullscreen, we want to draw the pixmap covering the whole window.
        # Assuming the screenshot matches the window size (monitor or virtual desktop).
        painter.drawPixmap(self.rect(), self.pixmap)
        
        # Draw darken overlay
        painter.fillRect(self.rect(), self.overlay_color)
        
        if not self.dragging:
            return

        # Setup Pen
        pen = QPen(self.selection_color)
        pen.setWidth(Config.STROKE_WIDTH)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)

        # Draw the rectangle
        rect = QRect(self.drag_start, self.current_pos).normalized()
        painter.drawRect(rect)

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.dragging = True
            self.drag_start = event.pos()
            self.current_pos = event.pos()
            self.update()

    def mouseMoveEvent(self, event):
        if self.dragging:
            self.current_pos = event.pos()
            self.update()

    def mouseReleaseEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton and self.dragging:
            self.dragging = False
            self.finish_selection()

    def keyPressEvent(self, event):
        if event.key() == Qt.Key.Key_Escape:
            self.cleanup_and_exit()

    def finish_selection(self):
        rect = QRect(self.drag_start, self.current_pos).normalized()
        
        if rect.width() < 5 or rect.height() < 5:
            self.cleanup_and_exit()
            return

        self.hide()
        # On HighDPI screens, the Qt coordinates might differ from the actual pixel coordinates of the screenshot.
        # However, since we are displaying the screenshot in a fullscreen window, the coordinate space of the window
        # should match the visual representation. If the screenshot was taken at native resolution and displayed
        # at logical resolution, there might be a scale factor.
        # For simplicity in this cross-platform script, we assume 1:1 or let the OS scaling handle it, 
        # but robust implementations might need to account for devicePixelRatio().
        
        # If using grim (Wayland), it captures raw pixels. Qt might be using scaled points.
        # We might need to scale the rect.
        scale_factor = self.devicePixelRatio()
        
        x = int(rect.x() * scale_factor)
        y = int(rect.y() * scale_factor)
        w = int(rect.width() * scale_factor)
        h = int(rect.height() * scale_factor)

        SearchService.search(x, y, w, h, self.screenshot_path)
        QApplication.quit()

    def cleanup_and_exit(self):
        if os.path.exists(self.screenshot_path):
            os.remove(self.screenshot_path)
        QApplication.quit()

def capture_screen_grim(path):
    """Capture screen using grim (Wayland/Hyprland)."""
    # Try to capture the currently focused output
    try:
        output = subprocess.check_output(["hyprctl", "monitors", "-j"]).decode()
        import json
        monitors = json.loads(output)
        for m in monitors:
            if m.get('focused'):
                subprocess.run(["grim", "-o", m.get('name'), path], check=True)
                return True
    except (subprocess.CalledProcessError, FileNotFoundError, ImportError):
        pass
    
    # Fallback to capturing the whole screen/region if hyprctl fails or simple grim invocation
    try:
        subprocess.run(["grim", path], check=True)
        return True
    except subprocess.CalledProcessError:
        return False
    except FileNotFoundError:
        return False

def capture_screen_mss(path):
    """Capture screen using mss (Cross-platform)."""
    try:
        import mss
        with mss.mss() as sct:
            # Capture the primary monitor (or all). monitor[0] is 'All' combined.
            # Using monitor 0 is usually safer for a full overlay.
            filename = sct.shot(mon=-1, output=path)
            # mss adds the monitor number to filename usually if multiple shots, but shot() output=path is direct.
            # However, shot() returns the filename created.
            if filename != path and os.path.exists(filename):
                shutil.move(filename, path)
            return True
    except ImportError:
        print("mss library not found. Please install it: pip install mss")
        return False
    except Exception as e:
        print(f"mss capture failed: {e}")
        return False

def main():
    Config.ensure_temp_path()
    app = QApplication(sys.argv)
    
    screenshot_path = os.path.join(Config.TEMP_PATH, "capture.png")
    
    # Strategy:
    # 1. If Linux and Wayland, try grim.
    # 2. If grim fails or other OS, use mss.
    
    captured = False
    is_wayland = os.environ.get("XDG_SESSION_TYPE") == "wayland"
    
    if is_wayland:
        captured = capture_screen_grim(screenshot_path)
    
    if not captured:
        captured = capture_screen_mss(screenshot_path)
    
    if not captured:
        print("Failed to capture screenshot. Ensure 'grim' (Wayland) or 'mss' (X11/Windows) is available.")
        sys.exit(1)

    overlay = SelectionOverlay(screenshot_path)
    overlay.show()
    sys.exit(app.exec())

if __name__ == "__main__":
    main()
