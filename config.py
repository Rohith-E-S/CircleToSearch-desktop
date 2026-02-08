import os
import tempfile

class Config:
    # Search Engine Configuration
    IMAGE_SEARCH_ENGINE_URL = "https://lens.google.com/uploadbyurl?url="
    UPLOAD_ENDPOINT = "https://uguu.se/upload"

    # UI Styling
    SELECTION_COLOR = "#4285F4"  # Google Blue
    OVERLAY_COLOR = "#66000000"  # Semi-transparent black
    STROKE_WIDTH = 1

    # Paths
    TEMP_PATH = os.path.join(tempfile.gettempdir(), "circle-to-search")

    @classmethod
    def ensure_temp_path(cls):
        if not os.path.exists(cls.TEMP_PATH):
            os.makedirs(cls.TEMP_PATH)
