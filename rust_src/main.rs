mod capture;
mod config;
mod search_service;

use eframe::egui;
use image::DynamicImage;
use std::sync::{mpsc, Arc};
use std::thread;

fn main() -> Result<(), eframe::Error> {
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

fn selection_bounds(
    start: egui::Pos2,
    end: egui::Pos2,
    screen_rect: egui::Rect,
    image_width: u32,
    image_height: u32,
) -> Option<(u32, u32, u32, u32)> {
    if !start.is_finite()
        || !end.is_finite()
        || !screen_rect.is_finite()
        || !screen_rect.width().is_finite()
        || !screen_rect.height().is_finite()
        || screen_rect.width() <= 0.0
        || screen_rect.height() <= 0.0
        || image_width == 0
        || image_height == 0
    {
        return None;
    }
    let rect = egui::Rect::from_two_pos(start, end).intersect(screen_rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let map = |value: f32, origin: f32, extent: f32, pixels: u32| {
        ((f64::from(value) - f64::from(origin)) / f64::from(extent) * f64::from(pixels))
            .clamp(0.0, f64::from(pixels))
    };
    let x = map(
        rect.min.x,
        screen_rect.min.x,
        screen_rect.width(),
        image_width,
    )
    .floor() as u32;
    let y = map(
        rect.min.y,
        screen_rect.min.y,
        screen_rect.height(),
        image_height,
    )
    .floor() as u32;
    let right = map(
        rect.max.x,
        screen_rect.min.x,
        screen_rect.width(),
        image_width,
    )
    .ceil() as u32;
    let bottom = map(
        rect.max.y,
        screen_rect.min.y,
        screen_rect.height(),
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

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            let screen_rect = ui.max_rect();

            // Draw screenshot background and overlay
            {
                let painter = ui.painter();
                painter.image(
                    self.texture.id(),
                    screen_rect,
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
                let response = ui.interact(screen_rect, ui.id(), egui::Sense::drag());

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
                            screen_rect,
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
                let rect = egui::Rect::from_two_pos(start, self.current_pos).intersect(screen_rect);

                let uv_min = egui::pos2(
                    (rect.min.x - screen_rect.min.x) / screen_rect.width(),
                    (rect.min.y - screen_rect.min.y) / screen_rect.height(),
                );
                let uv_max = egui::pos2(
                    (rect.max.x - screen_rect.min.x) / screen_rect.width(),
                    (rect.max.y - screen_rect.min.y) / screen_rect.height(),
                );

                let painter = ui.painter();
                painter.image(
                    self.texture.id(),
                    rect,
                    egui::Rect::from_min_max(uv_min, uv_max),
                    egui::Color32::WHITE,
                );

                // Draw border
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(235, 232, 231)), // Google Blue
                );
            }

            let mut message = "Drag with the primary mouse button to search. Selected area uploads to uguu.se and Google Lens. Escape closes.".to_string();
            if let Some(error) = &self.search_error {
                message.push_str(&format!("\nSearch failed: {error}\nDraw another area to try again, or press Escape to close."));
            }
            let text = ui.painter().layout(
                message,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
                (screen_rect.width() - 24.0).max(1.0),
            );
            let position = screen_rect.min + egui::vec2(12.0, 12.0);
            ui.painter().rect_filled(
                egui::Rect::from_min_size(position, text.size()).expand(6.0),
                0.0,
                egui::Color32::BLACK,
            );
            ui.painter().galley(position, text, egui::Color32::WHITE);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::selection_bounds;
    use eframe::egui::{pos2, Rect};

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
            Some((0, 0, 1, 1))
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
