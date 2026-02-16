use std::process::Command;
use std::env;
use serde::Deserialize;
use image::DynamicImage;

#[derive(Deserialize)]
struct Monitor {
    name: String,
    focused: bool,
}

pub fn capture_screen() -> Result<DynamicImage, String> {
    let is_wayland = env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland";

    if is_wayland {
        if let Ok(img) = capture_screen_grim() {
            return Ok(img);
        }
    }

    capture_screen_generic()
}

fn capture_screen_grim() -> Result<DynamicImage, String> {
    // Try to capture focused output using hyprctl
    let output = Command::new("hyprctl")
        .args(&["monitors", "-j"])
        .output();

    if let Ok(output) = output {
        if let Ok(monitors) = serde_json::from_slice::<Vec<Monitor>>(&output.stdout) {
            for m in monitors {
                if m.focused {
                    let output = Command::new("grim")
                        .args(&["-o", &m.name, "-"])
                        .output();
                    
                    if let Ok(output) = output {
                        if output.status.success() {
                            return image::load_from_memory(&output.stdout).map_err(|e| e.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback to default grim
    let output = Command::new("grim")
        .arg("-")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        image::load_from_memory(&output.stdout).map_err(|e| e.to_string())
    } else {
        Err("grim failed".to_string())
    }
}

fn capture_screen_generic() -> Result<DynamicImage, String> {
    let screens = screenshots::Screen::all().map_err(|e| e.to_string())?;
    
    if let Some(screen) = screens.first() {
        let image = screen.capture().map_err(|e| e.to_string())?;
        let width = image.width();
        let height = image.height();
        let data = image.into_raw();
        
        let new_image = image::RgbaImage::from_raw(width, height, data)
            .ok_or("Failed to create image buffer")?;
            
        Ok(DynamicImage::ImageRgba8(new_image))
    } else {
        Err("No screens found".to_string())
    }
}
