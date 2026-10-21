"""Pure geometry helpers for the selection overlay.

This module deliberately avoids importing Qt so the mapping between widget
coordinates and captured-image pixels can be unit tested without a display
server. It mirrors the Rust implementation in `rust_src/main.rs`.
"""

import math

# Smallest selection, in widget points, that is treated as intentional.
# Anything smaller is almost always a mis-click, and uploading it would
# publish a meaningless crop to a third-party host.
MIN_SELECTION_POINTS = 5.0


def fit_rect(available, image_size):
    """Largest rectangle with the image's aspect ratio that fits in `available`.

    `available` is an ``(x, y, width, height)`` tuple in widget coordinates and
    `image_size` is the ``(width, height)`` of the image in pixels. The result
    uses the same coordinate space as `available`.

    The captured image does not always cover the whole overlay: on multi-monitor
    setups the capture can span the virtual desktop while the overlay covers a
    single screen. Fitting instead of stretching keeps the image undistorted, so
    what the user sees matches what gets cropped.
    """
    avail_x, avail_y, avail_w, avail_h = available
    img_w, img_h = image_size
    if img_w <= 0 or img_h <= 0 or avail_w <= 0 or avail_h <= 0:
        return (avail_x, avail_y, 0, 0)

    scale = min(avail_w / img_w, avail_h / img_h)
    width = img_w * scale
    height = img_h * scale
    return (
        avail_x + (avail_w - width) / 2,
        avail_y + (avail_h - height) / 2,
        width,
        height,
    )


def crop_bounds(selection, image_rect, image_size, min_size=MIN_SELECTION_POINTS):
    """Maps a selection in widget coordinates to pixel bounds in the image.

    `selection` and `image_rect` are ``(x, y, width, height)`` tuples in widget
    coordinates, where `image_rect` is the area the captured image is drawn in.
    `image_size` is the ``(width, height)`` of the image in pixels.

    Returns ``(x, y, width, height)`` in image pixels, or ``None`` when the
    selection is too small to be intentional or falls outside the image.
    """
    sel_x, sel_y, sel_w, sel_h = selection
    rect_x, rect_y, rect_w, rect_h = image_rect
    img_w, img_h = image_size

    if rect_w <= 0 or rect_h <= 0 or img_w <= 0 or img_h <= 0:
        return None

    # Normalise the selection so a bottom-left to top-right drag works.
    if sel_w < 0:
        sel_x += sel_w
        sel_w = -sel_w
    if sel_h < 0:
        sel_y += sel_h
        sel_h = -sel_h

    # Clip the selection to the area the image is drawn in.
    sel_right = sel_x + sel_w
    sel_bottom = sel_y + sel_h
    sel_x = max(sel_x, rect_x)
    sel_y = max(sel_y, rect_y)
    sel_right = min(sel_right, rect_x + rect_w)
    sel_bottom = min(sel_bottom, rect_y + rect_h)
    sel_w = sel_right - sel_x
    sel_h = sel_bottom - sel_y

    if sel_w < min_size or sel_h < min_size:
        return None

    scale_x = img_w / rect_w
    scale_y = img_h / rect_h

    left = math.floor((sel_x - rect_x) * scale_x)
    top = math.floor((sel_y - rect_y) * scale_y)
    right = min(img_w, math.ceil((sel_x + sel_w - rect_x) * scale_x))
    bottom = min(img_h, math.ceil((sel_y + sel_h - rect_y) * scale_y))

    left = max(0, min(left, img_w))
    top = max(0, min(top, img_h))

    if right <= left or bottom <= top:
        return None

    return (left, top, right - left, bottom - top)
