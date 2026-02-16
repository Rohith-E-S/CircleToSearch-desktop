use std::path::PathBuf;
use std::env;

pub struct Config;

impl Config {
    pub const IMAGE_SEARCH_ENGINE_URL: &'static str = "https://lens.google.com/uploadbyurl?url=";
    pub const UPLOAD_ENDPOINT: &'static str = "https://uguu.se/upload";

    // pub fn temp_path() -> PathBuf {
    //     let mut path = env::temp_dir();
    //     path.push("circle-to-search");
    //     if !path.exists() {
    //         std::fs::create_dir_all(&path).unwrap_or_default();
    //     }
    //     path
    // }
}
