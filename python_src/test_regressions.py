"""Regression tests for the legacy Python implementation.

Qt is mocked out so the suite runs headless and offline. The pure geometry
lives in `selection.py` and is covered by `test_selection.py`.
"""

import importlib
import io
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import MagicMock, Mock, mock_open, patch
import urllib.error
import urllib.parse


qt_widgets = types.ModuleType("PyQt6.QtWidgets")
qt_widgets.QApplication = MagicMock()
qt_widgets.QWidget = type("QWidget", (), {})
qt_core = types.ModuleType("PyQt6.QtCore")
qt_core.Qt = MagicMock()
qt_core.QPoint = MagicMock()
qt_core.QRect = MagicMock()
qt_gui = types.ModuleType("PyQt6.QtGui")
for name in ("QPainter", "QPen", "QColor", "QPixmap", "QCursor"):
    setattr(qt_gui, name, MagicMock())
with patch.dict(sys.modules, {
    "PyQt6": types.ModuleType("PyQt6"),
    "PyQt6.QtWidgets": qt_widgets,
    "PyQt6.QtCore": qt_core,
    "PyQt6.QtGui": qt_gui,
}):
    main = importlib.import_module("main")
    search_service = importlib.import_module("search_service")
import build_apps
from config import Config


def fake_overlay():
    """A SelectionOverlay with just enough state for the selection maths."""
    overlay = main.SelectionOverlay.__new__(main.SelectionOverlay)
    overlay.screenshot_path = "capture.png"
    overlay.drag_start = Mock()
    overlay.current_pos = Mock()
    overlay.hide = Mock()
    overlay.image_rect = Mock(return_value=(0.0, 0.0, 800.0, 600.0))
    overlay.pixmap = Mock()
    overlay.pixmap.width.return_value = 2000
    overlay.pixmap.height.return_value = 900
    return overlay


class OverlayTests(unittest.TestCase):
    def test_selection_maps_to_image_pixels(self):
        overlay = fake_overlay()
        rect = Mock()
        rect.x.return_value, rect.y.return_value = 100, 50
        rect.width.return_value, rect.height.return_value = 100, 50

        with patch.object(main, "QRect") as rectangle, \
             patch.object(main.SearchService, "search") as search, \
             patch.object(main.QApplication, "quit"):
            rectangle.return_value.normalized.return_value.intersected.return_value = rect
            overlay.finish_selection()

        search.assert_called_once_with(250, 75, 250, 75, "capture.png")

    def test_tiny_selection_never_uploads(self):
        overlay = fake_overlay()
        rect = Mock()
        rect.x.return_value, rect.y.return_value = 10, 10
        rect.width.return_value, rect.height.return_value = 1, 1

        with patch.object(main, "QRect") as rectangle, \
             patch.object(main.SearchService, "search") as search, \
             patch.object(main.QApplication, "quit") as quit_app:
            rectangle.return_value.normalized.return_value.intersected.return_value = rect
            overlay.finish_selection()

        search.assert_not_called()
        quit_app.assert_called_once()

    def test_release_uses_final_position(self):
        overlay = fake_overlay()
        overlay.dragging = True
        overlay.finish_selection = Mock()
        event = Mock()
        event.button.return_value = main.Qt.MouseButton.LeftButton
        main.SelectionOverlay.mouseReleaseEvent(overlay, event)
        self.assertIs(overlay.current_pos, event.pos.return_value)
        overlay.finish_selection.assert_called_once()


class LifecycleTests(unittest.TestCase):
    def test_temporary_files_removed_on_all_exit_paths(self):
        parents = []
        for outcome in ("escape", "cancel", "close", "capture_failure", "capture_exception", "overlay_failure", "event_failure"):
            with self.subTest(outcome=outcome):
                main.QApplication.quit.reset_mock()
                paths = []

                def capture(path):
                    paths.append(Path(path))
                    parents.append(Path(path).parent)
                    Path(path).write_bytes(b"capture")
                    Path(path).with_name("crop.png").write_bytes(b"crop")
                    if outcome == "capture_exception":
                        raise OSError("capture failed")
                    return outcome != "capture_failure"

                def execute():
                    overlay = fake_overlay()
                    if outcome == "escape":
                        event = Mock()
                        event.key.return_value = main.Qt.Key.Key_Escape
                        main.SelectionOverlay.keyPressEvent(overlay, event)
                    elif outcome == "cancel":
                        rect = Mock()
                        rect.x.return_value, rect.y.return_value = 10, 10
                        rect.width.return_value, rect.height.return_value = 1, 1
                        with patch.object(main, "QRect") as rectangle:
                            rectangle.return_value.normalized.return_value.intersected.return_value = rect
                            overlay.finish_selection()
                    elif outcome == "event_failure":
                        raise RuntimeError("event failure")
                    if outcome in ("escape", "cancel"):
                        main.QApplication.quit.assert_called_once()
                    self.assertTrue(paths[0].parent.exists())
                    return 0

                with patch.dict(os.environ, {"XDG_SESSION_TYPE": "x11"}), patch.object(main, "capture_screen_mss", side_effect=capture), patch.object(main.SelectionOverlay, "__init__", return_value=None) as initialize, patch.object(main.SelectionOverlay, "show", create=True), patch.object(main.QApplication.return_value, "exec", side_effect=execute), patch("builtins.print"):
                    if outcome == "overlay_failure":
                        initialize.side_effect = ValueError("bad image")
                    try:
                        main.main()
                    except (OSError, RuntimeError, ValueError, SystemExit):
                        pass
                self.assertEqual(len(paths), 1)
                self.assertFalse(paths[0].exists())
                self.assertFalse(paths[0].with_name("crop.png").exists())
                self.assertFalse(paths[0].parent.exists())
        self.assertEqual(len(set(parents)), len(parents))


class SearchTests(unittest.TestCase):
    def run_search(self, body=None, status=200, error=None, save=True, null=False, browser_error=None, bounds=(0, 0, 10, 10)):
        with tempfile.TemporaryDirectory() as directory:
            screenshot = Path(directory) / "capture.png"
            crop = Path(directory) / "crop.png"
            screenshot.write_bytes(b"capture")
            pixmap = Mock()
            pixmap.isNull.return_value = null
            pixmap.width.return_value = 20
            pixmap.height.return_value = 20

            def save_crop(path, format):
                self.assertEqual(Path(path), crop)
                self.assertEqual(format, "PNG")
                Path(path).write_bytes(b"crop")
                return save

            pixmap.copy.return_value.isNull.return_value = False
            pixmap.copy.return_value.save.side_effect = save_crop
            response = MagicMock()
            response.status = status
            response.read.return_value = body if body is not None else json.dumps({"success": True, "files": [{"url": "https://images.example/crop.png"}]}).encode()
            with patch.object(search_service, "QPixmap", return_value=pixmap), patch.object(search_service.urllib.request, "urlopen") as upload, patch.object(search_service.webbrowser, "open", return_value=True, side_effect=browser_error) as browser, patch("sys.stdout", new_callable=io.StringIO) as output:
                upload.return_value.__enter__.return_value = response
                upload.side_effect = error
                search_service.SearchService.search(*bounds, str(screenshot))
            self.assertFalse(screenshot.exists())
            self.assertFalse(crop.exists())
            return upload, browser, response, output.getvalue()

    def test_upload_timeout_and_single_encoded_url_parameter(self):
        image_url = "https://images.example/crop%20one.png?token=a%2Bb&other=x=y#fragment"
        upload, browser, response, output = self.run_search(json.dumps({"success": True, "files": [{"url": image_url}]}).encode())
        self.assertEqual(output, "")
        self.assertGreater(upload.call_args.kwargs["timeout"], 0)
        self.assertEqual(upload.call_args.kwargs["timeout"], Config.UPLOAD_TIMEOUT)
        request = upload.call_args.args[0]
        self.assertEqual(request.full_url, Config.UPLOAD_ENDPOINT)
        self.assertIn(b'name="files[]"; filename="crop.png"', request.data)
        self.assertIn(b"\r\n\r\ncrop\r\n", request.data)
        url = urllib.parse.urlsplit(browser.call_args.args[0])
        self.assertEqual(url.path, "/uploadbyurl")
        self.assertEqual(urllib.parse.parse_qsl(url.query), [("url", image_url)])
        self.assertEqual(url.fragment, "")
        response.read.assert_called_once_with(Config.MAX_UPLOAD_RESPONSE_BYTES + 1)

    def test_invalid_responses_never_open_browser(self):
        bodies = [b"not json", b"\xff", b"[]", b"null", b"{}", b'{"success":false,"error":"private"}', b'{"success":"true","files":[{"url":"https://images.example/a"}]}']
        for files in (None, [], {}, [None], [{}], [{"url": 3}]):
            bodies.append(json.dumps({"success": True, "files": files}).encode())
        for url in ("", "not a URL", "/crop.png", "//images.example/a", "http://images.example/a", "file:///crop.png", "javascript:alert(1)", "https://", "https://user:secret@images.example/a", "https://images.example:bad/a", "https://images.example:65536/a", "https://[broken/a", "https://images.example/\na", "https://images.example/ a", "https://images.example\\evil/a"):
            bodies.append(json.dumps({"success": True, "files": [{"url": url}]}).encode())
        bodies.append(b" " * (64 * 1024 + 1))
        for body in bodies:
            with self.subTest(body=body[:120]):
                _, browser, _, output = self.run_search(body)
                browser.assert_not_called()
                self.assertTrue(output)
                self.assertNotIn("private", output)
                self.assertNotIn("secret", output)

    def test_network_and_encoding_failures_cleanup(self):
        for error in (TimeoutError("timeout"), urllib.error.URLError("offline"), urllib.error.HTTPError(Config.UPLOAD_ENDPOINT, 503, "unavailable", {}, None), OSError("read failure")):
            with self.subTest(error=error):
                _, browser, _, output = self.run_search(error=error)
                browser.assert_not_called()
                self.assertTrue(output)
        for status in (302, 400, 500):
            _, browser, _, output = self.run_search(status=status)
            browser.assert_not_called()
            self.assertIn(str(status), output)
        for options in ({"save": False}, {"null": True}, {"bounds": (0, 0, 0, 10)}, {"bounds": (-1, 0, 10, 10)}, {"bounds": (15, 0, 10, 10)}):
            upload, browser, _, output = self.run_search(**options)
            upload.assert_not_called()
            browser.assert_not_called()
            self.assertTrue(output)
        self.run_search(browser_error=OSError("browser failed"))


def fake_mss_module(with_mss_class=True):
    """A stand-in for the `mss` package that records how it was called."""
    module = types.ModuleType("mss")
    calls = []

    class FakeSct:
        def __enter__(self):
            return self

        def __exit__(self, *args):
            return False

        def shot(self, mon=None, output=None):
            calls.append((mon, output))
            Path(output).write_bytes(b"png")
            return output

    factory = FakeSct
    if with_mss_class:
        module.MSS = factory
    else:
        module.mss = factory
    module.calls = calls
    return module


class CaptureTests(unittest.TestCase):
    def test_grim_falls_back_for_malformed_monitor_data(self):
        for data in (b"not json", b"\xff", b"null", b"{}", b"[null]", b'[{"focused":true}]'):
            with self.subTest(data=data), patch.object(main.subprocess, "check_output", return_value=data), patch.object(main.subprocess, "run") as run:
                self.assertTrue(main.capture_screen_grim("capture.png"))
                run.assert_called_once_with(["grim", "capture.png"], check=True)

    def test_capture_command_errors_are_handled(self):
        with patch.object(main.subprocess, "check_output", side_effect=PermissionError()), patch.object(main.subprocess, "run", side_effect=PermissionError()):
            self.assertFalse(main.capture_screen_grim("capture.png"))
        with patch.dict(sys.modules, {"mss": None}), patch("builtins.print"):
            self.assertFalse(main.capture_screen_mss("capture.png"))

    def test_mss_captures_the_virtual_desktop(self):
        for with_mss_class in (True, False):
            with self.subTest(with_mss_class=with_mss_class):
                module = fake_mss_module(with_mss_class)
                with tempfile.TemporaryDirectory() as directory:
                    path = os.path.join(directory, "capture.png")
                    with patch.dict(sys.modules, {"mss": module}):
                        self.assertTrue(main.capture_screen_mss(path))
                    # mon=-1 is the virtual desktop, i.e. every monitor combined.
                    self.assertEqual(module.calls, [(-1, path)])
                    self.assertTrue(os.path.exists(path))


class BuildTests(unittest.TestCase):
    def test_script_fails_nonzero_for_install_and_build_errors(self):
        for stage in ("install", "build"):
            error = subprocess.CalledProcessError(2, stage)
            calls = [error] if stage == "install" else [None, error]
            with self.subTest(stage=stage), patch.object(subprocess, "check_call", side_effect=calls) as command, patch("builtins.open", mock_open()), patch("sys.stdout", new_callable=io.StringIO) as output:
                with self.assertRaises((subprocess.CalledProcessError, SystemExit)) as raised:
                    runpy.run_path(build_apps.__file__, run_name="__main__")
                if isinstance(raised.exception, SystemExit):
                    self.assertNotEqual(raised.exception.code, 0)
                self.assertNotIn("BUILD SUCCESSFUL", output.getvalue())
                self.assertEqual(command.call_count, len(calls))

    def test_mss_version_floor_is_pinned(self):
        source = Path(build_apps.__file__).read_text()
        self.assertIn("mss>=10", source)


if __name__ == "__main__":
    unittest.main()
