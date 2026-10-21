mod capture;
mod config;
mod search_service;

use crate::config::Config;
use eframe::egui;
use image::DynamicImage;
use std::sync::{mpsc, Arc};
use std::thread;

fn main() -> Result<(), eframe::Error> {
    eprintln!(
        "Circle to Search {} ({})",
        config::version(),
        config::commit()
    );

    // 1. Capture Screen (In Memory)
    let image = match capture::capture_screen() {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to capture screen: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Prepare for Display
    let size = [image.width() as usize, image.height() as usize];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

    // 3. Setup Window Options
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };

    // 4. Run App
    eframe::run_native(
        "Circle to Search",
        options,
        Box::new(move |cc| {
            // Create texture from image
            let texture =
                cc.egui_ctx
                    .load_texture("screenshot", color_image, egui::TextureOptions::LINEAR);
            Ok(Box::new(MyApp::new(texture, image)))
        }),
    )
}

/// The area the captured image is drawn in, preserving its aspect ratio.
///
/// The capture does not always cover the whole window: on multi-monitor
/// setups the compositor may place the overlay on a single output while the
/// capture spans the virtual desktop. Letterboxing keeps the image undistorted
/// so that what the user sees matches what gets cropped.
fn fit_image_rect(available: egui::Rect, image_width: u32, image_height: u32) -> egui::Rect {
    if image_width == 0
        || image_height == 0
        || !available.is_finite()
        || available.width() <= 0.0
        || available.height() <= 0.0
    {
        return egui::Rect::from_min_size(available.min, egui::vec2(0.0, 0.0));
    }

    let scale =
        (available.width() / image_width as f32).min(available.height() / image_height as f32);
    egui::Rect::from_center_size(
        available.center(),
        egui::vec2(image_width as f32 * scale, image_height as f32 * scale),
    )
}

/// Texture coordinates for `rect`, given that the image is drawn at `image_rect`.
fn uv_rect(rect: egui::Rect, image_rect: egui::Rect) -> egui::Rect {
    let size = image_rect.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    }
    egui::Rect::from_min_max(
        egui::pos2(
            (rect.min.x - image_rect.min.x) / size.x,
            (rect.min.y - image_rect.min.y) / size.y,
        ),
        egui::pos2(
            (rect.max.x - image_rect.min.x) / size.x,
            (rect.max.y - image_rect.min.y) / size.y,
        ),
    )
}

/// Maps a drag, given in window points, to pixel bounds in the captured image.
fn selection_bounds(
    start: egui::Pos2,
    end: egui::Pos2,
    image_rect: egui::Rect,
    image_width: u32,
    image_height: u32,
) -> Option<(u32, u32, u32, u32)> {
    if !start.is_finite()
        || !end.is_finite()
        || !image_rect.is_finite()
        || !image_rect.width().is_finite()
        || !image_rect.height().is_finite()
        || image_rect.width() <= 0.0
        || image_rect.height() <= 0.0
        || image_width == 0
        || image_height == 0
    {
        return None;
    }
    let rect = egui::Rect::from_two_pos(start, end).intersect(image_rect);
    if rect.width() < Config::MIN_SELECTION_POINTS || rect.height() < Config::MIN_SELECTION_POINTS {
        return None;
    }
    let map = |value: f32, origin: f32, extent: f32, pixels: u32| {
        ((f64::from(value) - f64::from(origin)) / f64::from(extent) * f64::from(pixels))
            .clamp(0.0, f64::from(pixels))
    };
    let x = map(
        rect.min.x,
        image_rect.min.x,
        image_rect.width(),
        image_width,
    )
    .floor() as u32;
    let y = map(
        rect.min.y,
        image_rect.min.y,
        image_rect.height(),
        image_height,
    )
    .floor() as u32;
    let right = map(
        rect.max.x,
        image_rect.min.x,
        image_rect.width(),
        image_width,
    )
    .ceil() as u32;
    let bottom = map(
        rect.max.y,
        image_rect.min.y,
        image_rect.height(),
        image_height,
    )
    .ceil() as u32;
    if right <= x || bottom <= y {
        return None;
    }
    Some((x, y, right - x, bottom - y))
}

struct MyApp {
    texture: egui::TextureHandle,
    image: Arc<DynamicImage>,
    start_pos: Option<egui::Pos2>,
    current_pos: egui::Pos2,
    search_tx: mpsc::Sender<Result<(), String>>,
    search_rx: mpsc::Receiver<Result<(), String>>,
    is_searching: bool,
    search_error: Option<String>,
}

impl MyApp {
    fn new(texture: egui::TextureHandle, image: DynamicImage) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            texture,
            image: Arc::new(image),
            start_pos: None,
            current_pos: egui::Pos2::ZERO,
            search_tx: tx,
            search_rx: rx,
            is_searching: false,
            search_error: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Check for search results
        if let Ok(result) = self.search_rx.try_recv() {
            self.is_searching = false;
            match result {
                Ok(()) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                Err(e) => {
                    eprintln!("Search error: {}", e);
                    self.search_error = Some(e);
                }
            }
        }

        let panel_frame = egui::Frame::none()
            .fill(egui::Color32::BLACK)
            .inner_margin(egui::Margin::same(0.0));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let screen_rect = ui.max_rect();
                let image_rect =
                    fit_image_rect(screen_rect, self.image.width(), self.image.height());

                // Draw screenshot background and overlay
                {
                    let painter = ui.painter();
                    painter.image(
                        self.texture.id(),
                        image_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    // Draw overlay (semi-transparent black)
                    painter.rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 100),
                    );
                }

                // Handle Input (only if not searching)
                if !self.is_searching {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    let response = ui.interact(image_rect, ui.id(), egui::Sense::drag());

                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.start_pos = ctx.input(|i| i.pointer.press_origin());
                        self.search_error = None;
                    }

                    if let Some(pos) = response.interact_pointer_pos() {
                        self.current_pos = pos;
                    }

                    if response.drag_stopped_by(egui::PointerButton::Primary) {
                        if let Some(start) = self.start_pos {
                            if let Some((x, y, w, h)) = selection_bounds(
                                start,
                                self.current_pos,
                                image_rect,
                                self.image.width(),
                                self.image.height(),
                            ) {
                                self.is_searching = true;

                                let image_ref = self.image.clone();
                                let tx = self.search_tx.clone();
                                let ctx_clone = ctx.clone();

                                thread::spawn(move || {
                                    let res = search_service::search(x, y, w, h, &image_ref);
                                    if tx.send(res).is_err() {
                                        eprintln!("Failed to send search result");
                                    }
                                    ctx_clone.request_repaint(); // Wake up UI
                                });
                            }
                        }
                        self.start_pos = None;
                    }
                } else {
                    // Show Spinner
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                }

                // Draw selection
                if let Some(start) = self.start_pos {
                    let rect =
                        egui::Rect::from_two_pos(start, self.current_pos).intersect(image_rect);

                    if rect.width() > 0.0 && rect.height() > 0.0 {
                        let painter = ui.painter();
                        painter.image(
                            self.texture.id(),
                            rect,
                            uv_rect(rect, image_rect),
                            egui::Color32::WHITE,
                        );

                        // Draw border
                        painter.rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(235, 232, 231)), // Google Blue
                        );
                    }
                }

                self.draw_status(ui, image_rect);
            });
    }
}

impl MyApp {
    /// Draws the hint (or the error) in the bottom-left corner of the image.
    ///
    /// The hint is intentionally unobtrusive: it is plain text with no
    /// background, so it does not cover the area the user is selecting.
    fn draw_status(&self, ui: &egui::Ui, image_rect: egui::Rect) {
        let (message, is_error) = match &self.search_error {
            Some(error) => (
                format!(
                    "Search failed: {error}\nDraw another area to try again, or press Escape to close."
                ),
                true,
            ),
            None => (
                "Drag to select. The selection is uploaded to uguu.se and opened in Google Lens."
                    .to_string(),
                false,
            ),
        };

        let text = ui.painter().layout(
            message,
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
            (image_rect.width() - 24.0).max(1.0),
        );

        let position = egui::pos2(
            image_rect.min.x + 12.0,
            (image_rect.max.y - text.size().y - 12.0).max(image_rect.min.y + 12.0),
        );

        if is_error {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(position, text.size()).expand(6.0),
                0.0,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 200),
            );
        }

        ui.painter().galley(position, text, egui::Color32::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::{fit_image_rect, selection_bounds, uv_rect};
    use crate::config::Config;
    use eframe::egui::{pos2, Rect};

    #[test]
    fn letterboxes_to_preserve_aspect_ratio() {
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 1000.0));
        let rect = fit_image_rect(available, 200, 100);
        assert_eq!(rect.width(), 1000.0);
        assert_eq!(rect.height(), 500.0);
        // Vertically centred.
        assert_eq!(rect.min.y, 250.0);
        assert_eq!(rect.max.y, 750.0);
    }

    #[test]
    fn letterbox_is_a_noop_for_matching_aspect_ratios() {
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));
        let rect = fit_image_rect(available, 1600, 1200);
        assert_eq!(rect.min, available.min);
        assert_eq!(rect.max, available.max);
    }

    #[test]
    fn letterbox_handles_degenerate_inputs() {
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        for (width, height) in [(0, 100), (100, 0), (0, 0)] {
            let rect = fit_image_rect(available, width, height);
            assert_eq!(rect.width(), 0.0);
            assert_eq!(rect.height(), 0.0);
        }
        let empty = Rect::from_min_max(pos2(0.0, 0.0), pos2(0.0, 0.0));
        assert_eq!(fit_image_rect(empty, 100, 100).width(), 0.0);
    }

    #[test]
    fn uv_rect_maps_the_image_rect_to_the_full_texture() {
        let image = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 200.0));
        let uv = uv_rect(image, image);
        assert_eq!(uv.min, pos2(0.0, 0.0));
        assert_eq!(uv.max, pos2(1.0, 1.0));

        let half = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 100.0));
        let uv = uv_rect(half, image);
        assert_eq!(uv.min, pos2(0.0, 0.0));
        assert_eq!(uv.max, pos2(0.5, 0.5));
    }

    #[test]
    fn maps_nonzero_origin_and_reversed_drags() {
        let screen = Rect::from_min_max(pos2(100.0, -50.0), pos2(500.0, 150.0));
        let start = pos2(140.0, -30.0);
        let end = pos2(300.0, 50.0);
        assert_eq!(
            selection_bounds(start, end, screen, 800, 600),
            Some((80, 60, 320, 240))
        );
        assert_eq!(
            selection_bounds(end, start, screen, 800, 600),
            Some((80, 60, 320, 240))
        );
    }

    #[test]
    fn scales_each_axis_and_rounds_outward() {
        let screen = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 200.0));
        assert_eq!(
            selection_bounds(pos2(10.25, 20.5), pos2(30.25, 60.5), screen, 200, 100),
            Some((20, 10, 41, 21))
        );
        assert_eq!(
            selection_bounds(pos2(1.0, 1.0), pos2(1.1, 1.1), screen, 10, 10),
            None,
            "a sub-threshold drag must not produce a crop"
        );
    }

    #[test]
    fn rejects_selections_below_the_floor() {
        let screen = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let floor = Config::MIN_SELECTION_POINTS;
        // Just under the floor on either axis.
        assert_eq!(
            selection_bounds(
                pos2(10.0, 10.0),
                pos2(10.0 + floor - 0.1, 50.0),
                screen,
                100,
                100
            ),
            None
        );
        assert_eq!(
            selection_bounds(
                pos2(10.0, 10.0),
                pos2(50.0, 10.0 + floor - 0.1),
                screen,
                100,
                100
            ),
            None
        );
        // Exactly at the floor is accepted (1 point maps to 1 pixel here).
        assert_eq!(
            selection_bounds(pos2(10.0, 10.0), pos2(10.0 + floor, 50.0), screen, 100, 100),
            Some((10, 10, 5, 40))
        );
    }

    #[test]
    fn clips_to_image_edges() {
        let screen = Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 220.0));
        assert_eq!(
            selection_bounds(pos2(-100.0, -100.0), pos2(1000.0, 1000.0), screen, 320, 240),
            Some((0, 0, 320, 240))
        );
        assert_eq!(
            selection_bounds(pos2(60.0, 120.0), pos2(1000.0, 1000.0), screen, 320, 240),
            Some((160, 120, 160, 120))
        );
        assert_eq!(
            selection_bounds(screen.min, screen.max, screen, u32::MAX, u32::MAX),
            Some((0, 0, u32::MAX, u32::MAX))
        );
    }

    #[test]
    fn rejects_empty_and_outside_selections() {
        let screen = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        for (start, end) in [
            (pos2(10.0, 10.0), pos2(10.0, 10.0)),
            (pos2(10.0, 10.0), pos2(10.0, 20.0)),
            (pos2(10.0, 10.0), pos2(20.0, 10.0)),
            (pos2(-20.0, -20.0), pos2(-10.0, -10.0)),
            (pos2(100.0, 10.0), pos2(120.0, 20.0)),
        ] {
            assert_eq!(selection_bounds(start, end, screen, 100, 100), None);
        }
        for (width, height) in [(0, 100), (100, 0), (0, 0)] {
            assert_eq!(
                selection_bounds(screen.min, screen.max, screen, width, height),
                None
            );
        }
    }

    #[test]
    fn rejects_nonfinite_inputs_and_invalid_screens() {
        let screen = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for invalid in [pos2(value, 10.0), pos2(10.0, value)] {
                assert_eq!(
                    selection_bounds(invalid, screen.max, screen, 100, 100),
                    None
                );
                assert_eq!(
                    selection_bounds(screen.min, invalid, screen, 100, 100),
                    None
                );
                for invalid_screen in [
                    Rect::from_min_max(invalid, screen.max),
                    Rect::from_min_max(screen.min, invalid),
                ] {
                    assert_eq!(
                        selection_bounds(screen.min, screen.max, invalid_screen, 100, 100),
                        None
                    );
                }
            }
        }
        for invalid in [
            Rect::from_min_max(pos2(0.0, 0.0), pos2(0.0, 100.0)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 0.0)),
            Rect::from_min_max(screen.max, screen.min),
            Rect::from_min_max(pos2(-f32::MAX, 0.0), pos2(f32::MAX, 100.0)),
        ] {
            assert_eq!(
                selection_bounds(screen.min, screen.max, invalid, 100, 100),
                None
            );
        }
    }
}
