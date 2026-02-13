use std::path::Path;
use std::process::Command;
use std::env;
use serde::Deserialize;

#[derive(Deserialize)]
struct Monitor {
    name: String,
    focused: bool,
}

pub fn capture_screen(path: &Path) -> Result<(), String> {
    let is_wayland = env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland";

    if is_wayland {
        if capture_screen_grim(path).is_ok() {
            return Ok(());
        }
    }

    capture_screen_generic(path)
}

fn capture_screen_grim(path: &Path) -> Result<(), String> {
    // Try to capture focused output using hyprctl
    let output = Command::new("hyprctl")
        .args(&["monitors", "-j"])
        .output();

    if let Ok(output) = output {
        if let Ok(monitors) = serde_json::from_slice::<Vec<Monitor>>(&output.stdout) {
            for m in monitors {
                if m.focused {
                    let status = Command::new("grim")
                        .args(&["-o", &m.name, path.to_str().unwrap()])
                        .status();
                    if let Ok(status) = status {
                        if status.success() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    // Fallback to default grim
    let status = Command::new("grim")
        .arg(path.to_str().unwrap())
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("grim failed".to_string())
    }
}

fn capture_screen_generic(path: &Path) -> Result<(), String> {
    let screens = screenshots::Screen::all().map_err(|e| e.to_string())?;
    
    // For simplicity, we capture the first screen or the main one. 
    // The Python code with mss captured "all" combined (-1).
    // screenshots crate captures per screen.
    // Handling multi-monitor "combined" image is complex (requires stitching).
    // For now, let's capture the primary screen or the first one found.
    // This is a slight divergence but safer for a first iteration.
    
    if let Some(screen) = screens.first() {
        let image = screen.capture().map_err(|e| e.to_string())?;
        image.save(path).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No screens found".to_string())
    }
}
