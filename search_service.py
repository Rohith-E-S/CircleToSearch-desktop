import os
import json
import urllib.request
import urllib.parse
import webbrowser
from PyQt6.QtGui import QPixmap
from config import Config

class SearchService:
    @staticmethod
    def search(x, y, width, height, screenshot_path):
        try:
            # 1. Crop the image using PyQt6 (already a dependency)
            pixmap = QPixmap(screenshot_path)
            # Crop the pixmap
            cropped_pixmap = pixmap.copy(int(x), int(y), int(width), int(height))
            
            cropped_path = os.path.join(Config.TEMP_PATH, "crop.png")
            cropped_pixmap.save(cropped_path, "PNG")
            
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
            with urllib.request.urlopen(req) as response:
                res_data = response.read().decode('utf-8')
                data = json.loads(res_data)
                
                if data.get('success') and data.get('files'):
                    image_url = data['files'][0]['url']
                    
                    # 3. Open in Browser
                    search_url = f"{Config.IMAGE_SEARCH_ENGINE_URL}{image_url}"
                    webbrowser.open(search_url)
                else:
                    print(f"Upload failed: {data}")

            # 4. Cleanup
            if os.path.exists(screenshot_path):
                os.remove(screenshot_path)
            if os.path.exists(cropped_path):
                os.remove(cropped_path)
                
        except Exception as e:
            print(f"Error during search process: {e}")
