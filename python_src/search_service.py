import os
import json
import urllib.request
import urllib.error
import urllib.parse
import webbrowser
from PyQt6.QtGui import QPixmap
from config import Config

class SearchService:
    @staticmethod
    def search(x, y, width, height, screenshot_path):
        cropped_path = os.path.join(os.path.dirname(screenshot_path), "crop.png")
        try:
            pixmap = QPixmap(screenshot_path)
            if pixmap.isNull():
                raise ValueError("Failed to load screenshot")
            x, y, width, height = int(x), int(y), int(width), int(height)
            if x < 0 or y < 0 or width <= 0 or height <= 0 or x + width > pixmap.width() or y + height > pixmap.height():
                raise ValueError("Invalid crop bounds")
            cropped_pixmap = pixmap.copy(x, y, width, height)
            if cropped_pixmap.isNull() or not cropped_pixmap.save(cropped_path, "PNG"):
                raise ValueError("Failed to save cropped image")
            
            # 2. Upload the image using urllib
            with open(cropped_path, 'rb') as f:
                image_data = f.read()
            
            # Prepare multipart/form-data manually for urllib
            boundary = '----WebKitFormBoundary7MA4YWxkTrZu0gW'
            body = (
                f'--{boundary}\r\n'
                f'Content-Disposition: form-data; name="files[]"; filename="crop.png"\r\n'
                f'Content-Type: image/png\r\n\r\n'
            ).encode('utf-8') + image_data + f'\r\n--{boundary}--\r\n'.encode('utf-8')
            
            headers = {
                'Content-Type': f'multipart/form-data; boundary={boundary}',
                'Content-Length': str(len(body))
            }
            
            req = urllib.request.Request(Config.UPLOAD_ENDPOINT, data=body, headers=headers)
            with urllib.request.urlopen(req, timeout=Config.UPLOAD_TIMEOUT) as response:
                if not 200 <= response.status < 300:
                    raise ValueError(f"Upload failed with HTTP status {response.status}")
                res_data = response.read(Config.MAX_UPLOAD_RESPONSE_BYTES + 1)
                if len(res_data) > Config.MAX_UPLOAD_RESPONSE_BYTES:
                    raise ValueError("Upload response too large")
                data = json.loads(res_data.decode('utf-8'))

            if not isinstance(data, dict) or data.get('success') is not True:
                raise ValueError("Upload service reported failure")
            files = data.get('files')
            if not isinstance(files, list) or not files or not isinstance(files[0], dict):
                raise ValueError("Invalid upload response")
            image_url = files[0].get('url')
            if not isinstance(image_url, str) or not image_url or any(c.isspace() or ord(c) < 32 or ord(c) == 127 or c == '\\' for c in image_url):
                raise ValueError("Invalid uploaded image URL")
            try:
                parsed = urllib.parse.urlsplit(image_url)
                if parsed.scheme != 'https' or not parsed.hostname or parsed.username is not None or parsed.password is not None:
                    raise ValueError
                parsed.port
            except ValueError:
                raise ValueError("Invalid uploaded image URL") from None
            # Encode the image URL as a single query parameter. urlencode uses
            # the same form-urlencoded rules as the Rust implementation, so
            # both builds produce an identical request.
            search_url = Config.IMAGE_SEARCH_ENGINE_URL + "?" + urllib.parse.urlencode({"url": image_url})
            if not webbrowser.open(search_url):
                raise ValueError("Failed to open browser")

        except urllib.error.HTTPError as e:
            print(f"Upload failed with HTTP status {e.code}")
            e.close()
        except (urllib.error.URLError, TimeoutError):
            print("Upload request failed or timed out")
        except (json.JSONDecodeError, UnicodeError):
            print("Invalid upload response")
        except Exception as e:
            print(f"Error during search process: {e}")
        finally:
            for path in (cropped_path, screenshot_path):
                try:
                    os.remove(path)
                except FileNotFoundError:
                    pass
                except OSError:
                    print("Failed to remove temporary image")
