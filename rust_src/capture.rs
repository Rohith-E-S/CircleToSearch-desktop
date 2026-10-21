use image::DynamicImage;
use serde::Deserialize;
use std::env;
use std::process::Command;

/// A compositor output (monitor). Both `hyprctl monitors -j` and
/// `swaymsg -t get_outputs` emit objects with these two fields.
#[derive(Deserialize)]
struct Output {
    name: String,
    focused: bool,
}

/// True when the session is a Wayland session.
pub fn is_wayland() -> bool {
    env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland"
        || env::var_os("WAYLAND_DISPLAY").is_some_and(|display| !display.is_empty())
}

/// Captures the screen of the output the user is currently looking at.
///
/// On Wayland the capture is delegated to `grim`, which reads the framebuffer
/// directly. Everywhere else `xcap` is used.
pub fn capture_screen() -> Result<DynamicImage, String> {
    if is_wayland() {
        return capture_screen_grim();
    }

    capture_screen_generic()
}

/// Asks the compositor which output currently has focus.
///
/// Hyprland and Sway both expose this; the JSON shapes are compatible enough
/// that one parser handles both.
fn focused_output() -> Option<String> {
    const COMMANDS: [&[&str]; 2] = [
        &["hyprctl", "monitors", "-j"],
        &["swaymsg", "-t", "get_outputs"],
    ];

    for command in COMMANDS {
        let Ok(output) = Command::new(command[0]).args(&command[1..]).output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let Ok(outputs) = serde_json::from_slice::<Vec<Output>>(&output.stdout) else {
            continue;
        };
        if let Some(focused) = outputs.iter().find(|output| output.focused) {
            if !focused.name.is_empty() {
                return Some(focused.name.clone());
            }
        }
    }

    None
}

fn capture_screen_grim() -> Result<DynamicImage, String> {
    let mut focused_error = None;

    // Capturing only the focused output keeps the image aligned with the
    // fullscreen overlay, which the compositor places on that same output.
    if let Some(name) = focused_output() {
        match run_grim(&["-o", &name, "-"]) {
            Ok(image) => return Ok(image),
            Err(error) => focused_error = Some(error),
        }
    }

    // Fall back to the whole virtual desktop. The overlay letterboxes the
    // image, so the selection still matches what is on screen.
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
    let monitors = xcap::Monitor::all().map_err(|error| error.to_string())?;

    let monitor = monitors
        .iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())
        .ok_or("No screens found")?;

    let image = monitor.capture_image().map_err(|error| error.to_string())?;

    Ok(DynamicImage::ImageRgba8(image))
}

#[cfg(test)]
mod tests {
    use super::Output;

    #[test]
    fn parses_hyprland_and_sway_output_lists() {
        let hyprland: &[u8] =
            br#"[{"id":0,"name":"DP-1","focused":false},{"id":1,"name":"DP-2","focused":true}]"#;
        let sway: &[u8] = br#"[{"name":"eDP-1","active":true,"focused":true,"scale":1.0}]"#;

        for payload in [hyprland, sway] {
            let outputs: Vec<Output> = serde_json::from_slice(payload).expect("valid output list");
            let focused = outputs.iter().find(|output| output.focused);
            assert!(focused.is_some());
            assert!(!focused.unwrap().name.is_empty());
        }
    }

    #[test]
    fn rejects_malformed_output_lists() {
        for payload in [
            &b"not json"[..],
            &b"null"[..],
            &b"{}"[..],
            &b"[null]"[..],
            &br#"[{"focused":true}]"#[..],
            &br#"[{"name":"DP-1"}]"#[..],
        ] {
            let parsed: Result<Vec<Output>, _> = serde_json::from_slice(payload);
            assert!(parsed.is_err(), "expected {payload:?} to be rejected");
        }
    }
}
