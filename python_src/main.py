import sys
import os
import subprocess
import shutil
from tempfile import TemporaryDirectory
from PyQt6.QtWidgets import QApplication, QWidget
from PyQt6.QtCore import Qt, QPoint, QRect
from PyQt6.QtGui import QPainter, QPen, QColor, QPixmap, QCursor
from config import Config
from search_service import SearchService
from selection import crop_bounds, fit_rect

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
        if self.pixmap.isNull():
            raise ValueError("Failed to load screenshot")
        
        if geometry:
            self.setGeometry(geometry)

        # Colors
        self.selection_color = QColor(Config.SELECTION_COLOR)
        self.overlay_color = QColor(Config.OVERLAY_COLOR)

    def image_rect(self):
        """Area the captured image is drawn in, preserving its aspect ratio."""
        rect = self.rect()
        return fit_rect(
            (rect.x(), rect.y(), rect.width(), rect.height()),
            (self.pixmap.width(), self.pixmap.height()),
        )

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        
        # Draw background screenshot
        # The screenshot is drawn at its native aspect ratio so that the user
        # selects against exactly what is on their screen.
        painter.fillRect(self.rect(), Qt.GlobalColor.black)
        target = self.image_rect()
        painter.drawPixmap(QRect(int(target[0]), int(target[1]), int(target[2]), int(target[3])), self.pixmap)
        
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
            self.current_pos = event.pos()
            self.finish_selection()

    def keyPressEvent(self, event):
        if event.key() == Qt.Key.Key_Escape:
            self.cleanup_and_exit()

    def finish_selection(self):
        image_rect = self.image_rect()
        drawn = QRect(int(image_rect[0]), int(image_rect[1]), int(image_rect[2]), int(image_rect[3]))
        rect = QRect(self.drag_start, self.current_pos).normalized().intersected(drawn)
        
        bounds = crop_bounds(
            (rect.x(), rect.y(), rect.width(), rect.height()),
            image_rect,
            (self.pixmap.width(), self.pixmap.height()),
        )
        if bounds is None:
            self.cleanup_and_exit()
            return

        self.hide()
        try:
            SearchService.search(*bounds, self.screenshot_path)
        finally:
            self.cleanup_and_exit()

    def cleanup_and_exit(self):
        QApplication.quit()

def capture_screen_grim(path):
    """Capture screen using grim (Wayland/Hyprland)."""
    # Try to capture the currently focused output
    try:
        output = subprocess.check_output(["hyprctl", "monitors", "-j"]).decode()
        import json
        monitors = json.loads(output)
        if isinstance(monitors, list):
            for m in monitors:
                if isinstance(m, dict) and m.get('focused') and isinstance(m.get('name'), str) and m['name']:
                    subprocess.run(["grim", "-o", m['name'], path], check=True)
                    return True
    except (subprocess.CalledProcessError, OSError, ValueError):
        pass
    
    # Fallback to capturing the whole screen/region if hyprctl fails or simple grim invocation
    try:
        subprocess.run(["grim", path], check=True)
        return True
    except (subprocess.CalledProcessError, OSError):
        return False

def capture_screen_mss(path):
    """Capture screen using mss (Cross-platform)."""
    try:
        import mss
    except ImportError:
        print("mss library not found. Please install it: pip install 'mss>=10'")
        return False

    try:
        # mss>=10 exposes MSS directly; older releases only ship the
        # (now deprecated) mss() factory.
        factory = getattr(mss, "MSS", None) or mss.mss
        with factory() as sct:
            # mon=-1 captures the virtual desktop (every monitor combined) as a
            # single image. This interpretation of -1 only exists from mss 10
            # onwards; older releases treat it as the last physical monitor,
            # hence the 'mss>=10' requirement.
            filename = sct.shot(mon=-1, output=path)
            # mss writes the file itself, but be defensive about the name.
            if filename != path and os.path.exists(filename):
                shutil.move(filename, path)
            return True
    except Exception as e:
        print(f"mss capture failed: {e}")
        return False

def main():
    app = QApplication(sys.argv)
    with TemporaryDirectory(prefix="circle-to-search-") as temp_path:
        screenshot_path = os.path.join(temp_path, "capture.png")
        captured = False
        is_wayland = os.environ.get("XDG_SESSION_TYPE") == "wayland"

        if is_wayland:
            captured = capture_screen_grim(screenshot_path)

        if not captured:
            captured = capture_screen_mss(screenshot_path)

        if not captured:
            print("Failed to capture screenshot. Ensure 'grim' (Wayland) or 'mss' (X11/Windows) is available.")
            sys.exit(1)

        try:
            overlay = SelectionOverlay(screenshot_path)
        except ValueError as e:
            print(e)
            sys.exit(1)
        overlay.show()
        sys.exit(app.exec())

if __name__ == "__main__":
    main()
