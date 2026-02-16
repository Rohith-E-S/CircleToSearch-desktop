mod config;
mod capture;
mod search_service;

use eframe::egui;
use image::DynamicImage;
use std::sync::{Arc, mpsc};
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
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    );

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
            let texture = cc.egui_ctx.load_texture(
                "screenshot",
                color_image,
                egui::TextureOptions::LINEAR,
            );
            Ok(Box::new(MyApp::new(texture, image)))
        }),
    )
}

struct MyApp {
    texture: egui::TextureHandle,
    image: Arc<DynamicImage>,
    start_pos: Option<egui::Pos2>,
    current_pos: egui::Pos2,
    search_tx: mpsc::Sender<Result<(), String>>,
    search_rx: mpsc::Receiver<Result<(), String>>,
    is_searching: bool,
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
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for search results
        if let Ok(result) = self.search_rx.try_recv() {
            self.is_searching = false;
            match result {
                Ok(_) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(e) => {
                    eprintln!("Search error: {}", e);
                    // Optionally show an error toast/message in UI
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

                if response.drag_started() {
                    self.start_pos = response.interact_pointer_pos();
                }

                if let Some(pos) = response.interact_pointer_pos() {
                    self.current_pos = pos;
                }

                if response.drag_stopped() {
                    if let Some(start) = self.start_pos {
                        let rect = egui::Rect::from_two_pos(start, self.current_pos);
                        // Minimal size check
                        if rect.width() > 5.0 && rect.height() > 5.0 {
                            // Start Search
                            self.is_searching = true;
                            let ppp = ctx.pixels_per_point();

                            let x = (rect.min.x * ppp) as u32;
                            let y = (rect.min.y * ppp) as u32;
                            let w = (rect.width() * ppp) as u32;
                            let h = (rect.height() * ppp) as u32;

                            let image_ref = self.image.clone();
                            let tx = self.search_tx.clone();
                            let ctx_clone = ctx.clone();

                            thread::spawn(move || {
                                let res = search_service::search(x, y, w, h, &image_ref);
                                if let Err(_) = tx.send(res) {
                                    eprintln!("Failed to send search result");
                                }
                                ctx_clone.request_repaint(); // Wake up UI
                            });
                        }
                    }
                    self.start_pos = None;
                }

                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
                // Show Spinner
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                });
            }

            // Draw selection
            if let Some(start) = self.start_pos {
                let rect = egui::Rect::from_two_pos(start, self.current_pos);

                let uv_min = egui::pos2(
                    rect.min.x / screen_rect.width(),
                    rect.min.y / screen_rect.height(),
                );
                let uv_max = egui::pos2(
                    rect.max.x / screen_rect.width(),
                    rect.max.y / screen_rect.height(),
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
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(235, 232, 231)), // Google Blue
                );
            }
        });
    }
}
