use std::time::Duration;

pub struct Config;

impl Config {
    pub const IMAGE_SEARCH_ENGINE_URL: &'static str = "https://lens.google.com/uploadbyurl";
    pub const UPLOAD_ENDPOINT: &'static str = "https://uguu.se/upload";
    pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
    pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
    pub const MAX_UPLOAD_RESPONSE_BYTES: u64 = 64 * 1024;

    /// Smallest selection, in window points, that is treated as intentional.
    ///
    /// Anything smaller is almost always a mis-click, and uploading it would
    /// publish a meaningless crop to a third-party host.
    pub const MIN_SELECTION_POINTS: f32 = 5.0;
}

/// Version of the running binary, stamped at build time by `build.rs`.
pub fn version() -> &'static str {
    option_env!("CIRCLE_TO_SEARCH_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Short commit hash of the running binary, stamped at build time by `build.rs`.
pub fn commit() -> &'static str {
    option_env!("CIRCLE_TO_SEARCH_COMMIT").unwrap_or("unknown")
}

/// Compile-time guard so the selection floor can never become nonsensical.
const _: () = assert!(Config::MIN_SELECTION_POINTS > 0.0);
const _: () = assert!(Config::MIN_SELECTION_POINTS <= 32.0);

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_always_populated() {
        assert!(!version().is_empty());
        assert!(version().len() < 64, "version should be a short label");
    }
}
