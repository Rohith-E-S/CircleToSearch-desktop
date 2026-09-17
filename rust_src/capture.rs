use image::DynamicImage;
use serde::Deserialize;
use std::env;
use std::process::Command;

#[derive(Deserialize)]
struct Monitor {
    name: String,
    focused: bool,
}

pub fn capture_screen() -> Result<DynamicImage, String> {
    let is_wayland = env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland"
        || env::var_os("WAYLAND_DISPLAY").is_some_and(|display| !display.is_empty());

    if is_wayland {
        return capture_screen_grim().or_else(|grim_error| {
            capture_screen_generic().map_err(|generic_error| {
                format!("{grim_error}; generic capture also failed: {generic_error}")
            })
        });
    }

    capture_screen_generic()
}

fn capture_screen_grim() -> Result<DynamicImage, String> {
    let mut focused_error = None;
    if let Ok(output) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
        if output.status.success() {
            if let Ok(monitors) = serde_json::from_slice::<Vec<Monitor>>(&output.stdout) {
                if let Some(monitor) = monitors.iter().find(|monitor| monitor.focused) {
                    match run_grim(&["-o", &monitor.name, "-"]) {
                        Ok(image) => return Ok(image),
                        Err(error) => focused_error = Some(error),
                    }
                }
            }
        }
    }

    run_grim(&["-"]).map_err(|error| match focused_error {
        Some(focused) => format!("Focused output: {focused}; default capture: {error}"),
        None => error,
    })
}

fn run_grim(args: &[&str]) -> Result<DynamicImage, String> {
    let output = Command::new("grim").args(args).output().map_err(|error| {
        format!("Cannot run grim: {error}. Install grim and ensure it is on PATH.")
    })?;

    if output.status.success() {
        image::load_from_memory(&output.stdout)
            .map_err(|error| format!("Invalid image from grim: {error}"))
    } else {
        Err(format!(
            "grim failed ({}): {}. Check that your Wayland compositor supports grim screen capture and the selected output is available.",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
