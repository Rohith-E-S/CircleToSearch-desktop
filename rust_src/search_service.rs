use std::process::Command;
use reqwest::blocking::multipart;
use serde::Deserialize;
use crate::config::Config;

#[derive(Deserialize)]
struct UploadResponse {
    success: bool,
    files: Option<Vec<FileResponse>>,
}

#[derive(Deserialize)]
struct FileResponse {
    url: String,
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Stdio;
        // 1. Try xdg-open with stderr silenced to avoid "parental controls" noise
        if Command::new("xdg-open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok() 
        {
            return Ok(());
        }
        
        // 2. Fallback browsers
        let commands = ["google-chrome", "firefox", "chromium", "brave"];
        for cmd in commands {
            if Command::new(cmd)
                .arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .is_ok() 
            {
                return Ok(());
            }
        }
        return Err("Could not open browser with any known command".to_string());
    }

    #[cfg(not(target_os = "linux"))]
    {
        // 1. Try the cross-platform crate first
        if webbrowser::open(url).is_ok() {
            return Ok(());
        }

        // 2. Fallback: OS-specific commands
        #[cfg(target_os = "windows")]
        {
            if Command::new("cmd").args(&["/c", "start", url]).spawn().is_ok() {
                return Ok(());
            }
        }

        #[cfg(target_os = "macos")]
        {
            if Command::new("open").arg(url).spawn().is_ok() {
                return Ok(());
            }
        }

        Err("Could not open browser with any known command".to_string())
    }
}

pub fn search(x: u32, y: u32, width: u32, height: u32, img: &image::DynamicImage) -> Result<(), String> {
    // 1. Crop
    let cropped = img.crop_imm(x, y, width, height);
    
    // Encode to PNG in memory
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped.write_to(&mut buffer, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    let bytes = buffer.into_inner();

    // 2. Upload
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    
    let part = multipart::Part::bytes(bytes)
        .file_name("crop.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .part("files[]", part);

    println!("Uploading to {}...", Config::UPLOAD_ENDPOINT);

    let res = client.post(Config::UPLOAD_ENDPOINT)
        .multipart(form)
        .send()
        .map_err(|e| e.to_string())?;

    let text = res.text().map_err(|e| e.to_string())?;
    println!("Server response: {}", text);
    
    let data: UploadResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    if data.success {
        if let Some(files) = data.files {
            if let Some(file) = files.first() {
                let search_url = format!("{}{}", Config::IMAGE_SEARCH_ENGINE_URL, file.url);
                println!("Opening URL: {}", search_url);
                if let Err(e) = open_browser(&search_url) {
                    return Err(format!("Failed to open browser: {}", e));
                }
                return Ok(());
            }
        }
    }

    Err(format!("Upload failed: {}", text))
}
