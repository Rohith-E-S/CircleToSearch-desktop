"""Tests for the pure geometry helpers.

These need no Qt and no display server. The test vectors deliberately match
the Rust tests in `rust_src/main.rs` so both implementations stay in sync.
"""

import unittest

from selection import MIN_SELECTION_POINTS, crop_bounds, fit_rect


class FitRectTests(unittest.TestCase):
    def test_letterboxes_to_preserve_aspect_ratio(self):
        rect = fit_rect((0.0, 0.0, 1000.0, 1000.0), (200, 100))
        # Vertically centred.
        self.assertEqual(rect, (0.0, 250.0, 1000.0, 500.0))
        self.assertEqual(rect[1], 250.0)
        self.assertEqual(rect[1] + rect[3], 750.0)

    def test_letterbox_is_a_noop_for_matching_aspect_ratios(self):
        available = (0.0, 0.0, 800.0, 600.0)
        self.assertEqual(fit_rect(available, (1600, 1200)), available)

    def test_letterbox_handles_degenerate_inputs(self):
        available = (0.0, 0.0, 100.0, 100.0)
        for size in [(0, 100), (100, 0), (0, 0)]:
            rect = fit_rect(available, size)
            self.assertEqual(rect[2], 0.0)
            self.assertEqual(rect[3], 0.0)
        self.assertEqual(fit_rect((0.0, 0.0, 0.0, 0.0), (100, 100))[2], 0.0)


class CropBoundsTests(unittest.TestCase):
    def test_maps_nonzero_origin(self):
        self.assertEqual(
            crop_bounds((140.0, -30.0, 160.0, 80.0), (100.0, -50.0, 400.0, 200.0), (800, 600)),
            (80, 60, 320, 240),
        )

    def test_reversed_drags_produce_the_same_bounds(self):
        forward = crop_bounds((140.0, -30.0, 160.0, 80.0), (100.0, -50.0, 400.0, 200.0), (800, 600))
        reversed_drag = crop_bounds((300.0, 50.0, -160.0, -80.0), (100.0, -50.0, 400.0, 200.0), (800, 600))
        self.assertEqual(forward, (80, 60, 320, 240))
        self.assertEqual(reversed_drag, forward)

    def test_scales_each_axis_and_rounds_outward(self):
        self.assertEqual(
            crop_bounds((10.25, 20.5, 20.0, 40.0), (0.0, 0.0, 100.0, 200.0), (200, 100)),
            (20, 10, 41, 21),
        )

    def test_clips_to_image_edges(self):
        image_rect = (10.0, 20.0, 100.0, 200.0)
        self.assertEqual(
            crop_bounds((-100.0, -100.0, 1100.0, 1100.0), image_rect, (320, 240)),
            (0, 0, 320, 240),
        )
        self.assertEqual(
            crop_bounds((60.0, 120.0, 940.0, 880.0), image_rect, (320, 240)),
            (160, 120, 160, 120),
        )
        self.assertEqual(
            crop_bounds((10.0, 20.0, 100.0, 200.0), image_rect, (2**31, 2**31)),
            (0, 0, 2**31, 2**31),
        )

    def test_rejects_selections_below_the_floor(self):
        image_rect = (0.0, 0.0, 100.0, 100.0)
        floor = MIN_SELECTION_POINTS
        self.assertIsNone(
            crop_bounds((10.0, 10.0, floor - 0.1, 40.0), image_rect, (100, 100))
        )
        self.assertIsNone(
            crop_bounds((10.0, 10.0, 40.0, floor - 0.1), image_rect, (100, 100))
        )
        # Exactly at the floor is accepted (1 point maps to 1 pixel here).
        self.assertEqual(
            crop_bounds((10.0, 10.0, floor, 40.0), image_rect, (100, 100)),
            (10, 10, 5, 40),
        )

    def test_rejects_empty_and_outside_selections(self):
        image_rect = (0.0, 0.0, 100.0, 100.0)
        for selection in [
            (10.0, 10.0, 0.0, 0.0),
            (10.0, 10.0, 0.0, 10.0),
            (10.0, 10.0, 10.0, 0.0),
            (-20.0, -20.0, 10.0, 10.0),
            (100.0, 10.0, 20.0, 10.0),
        ]:
            self.assertIsNone(crop_bounds(selection, image_rect, (100, 100)))

    def test_rejects_degenerate_image_rects_and_sizes(self):
        image_rect = (0.0, 0.0, 100.0, 100.0)
        for rect in [(0.0, 0.0, 0.0, 100.0), (0.0, 0.0, 100.0, 0.0)]:
            self.assertIsNone(crop_bounds((10.0, 10.0, 50.0, 50.0), rect, (100, 100)))
        for size in [(0, 100), (100, 0), (0, 0)]:
            self.assertIsNone(crop_bounds((10.0, 10.0, 50.0, 50.0), image_rect, size))


if __name__ == "__main__":
    unittest.main()
