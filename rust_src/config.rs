use std::time::Duration;

pub struct Config;

impl Config {
    pub const IMAGE_SEARCH_ENGINE_URL: &'static str = "https://lens.google.com/uploadbyurl";
    pub const UPLOAD_ENDPOINT: &'static str = "https://uguu.se/upload";
    pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
    pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
    pub const MAX_UPLOAD_RESPONSE_BYTES: u64 = 64 * 1024;
}
