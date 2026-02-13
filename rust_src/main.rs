mod config;
mod capture;
mod search_service;

use eframe::egui;
use std::path::PathBuf;
use config::Config;

fn main() -> Result<(), eframe::Error> {
    let temp_path = Config::temp_path();
    let screenshot_path = temp_path.join("capture.png");

    // 1. Capture Screen
    if let Err(e) = capture::capture_screen(&screenshot_path) {
        eprintln!("Failed to capture screen: {}", e);
        std::process::exit(1);
    }

    // 2. Load Image
    let image = image::open(&screenshot_path).expect("Failed to open screenshot");
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
            Ok(Box::new(MyApp::new(texture, screenshot_path)))
        }),
    )
}

struct MyApp {
    texture: egui::TextureHandle,
    screenshot_path: PathBuf,
    start_pos: Option<egui::Pos2>,
    current_pos: egui::Pos2,
}

impl MyApp {
    fn new(texture: egui::TextureHandle, path: PathBuf) -> Self {
        Self {
            texture,
            screenshot_path: path,
            start_pos: None,
            current_pos: egui::Pos2::ZERO,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_frame = egui::Frame::none()
            .fill(egui::Color32::BLACK)
            .inner_margin(egui::Margin::same(0.0));

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            let screen_rect = ui.max_rect();
            
            // Draw screenshot background
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

            // Handle Input
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
                        // Perform search
                        // Map rect to image coordinates
                        // Assuming the image fills the screen, screen coordinates == image coordinates
                        // But need to handle scaling if any. 
                        // eframe pixels are logical points.
                        // image is physical pixels.
                        // We need pixels_per_point.
                        let ppp = ctx.pixels_per_point();
                        
                        let x = (rect.min.x * ppp) as u32;
                        let y = (rect.min.y * ppp) as u32;
                        let w = (rect.width() * ppp) as u32;
                        let h = (rect.height() * ppp) as u32;
                        
                        if let Err(e) = search_service::search(x, y, w, h, &self.screenshot_path) {
                            eprintln!("Search error: {}", e);
                        }
                        
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                self.start_pos = None;
            }
            
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Draw selection
            if let Some(start) = self.start_pos {
                let rect = egui::Rect::from_two_pos(start, self.current_pos);
                
                // Clear the overlay inside the selection
                // Actually, we can't "clear" easily in immediate mode painter order without layers.
                // Instead, we just draw the overlay *around* the rect, or we draw the "clear" rect 
                // by re-drawing the image part inside the rect.
                
                // Re-draw the original image inside the selection rect to make it look "bright"
                // mapping the UV coords.
                let uv_min = egui::pos2(
                    rect.min.x / screen_rect.width(),
                    rect.min.y / screen_rect.height(),
                );
                let uv_max = egui::pos2(
                    rect.max.x / screen_rect.width(),
                    rect.max.y / screen_rect.height(),
                );
                
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
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(66, 133, 244)), // Google Blue
                );
            }
        });
    }
}
