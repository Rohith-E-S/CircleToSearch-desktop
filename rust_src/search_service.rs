use crate::config::Config;
use reqwest::blocking::{multipart, Client, Response};
use reqwest::Url;
use serde::Deserialize;
use std::io::Read;

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
    webbrowser::open(url).map_err(|_| "Failed to open browser".to_string())
}

fn crop_image(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    img: &image::DynamicImage,
) -> Result<image::DynamicImage, String> {
    if width == 0
        || height == 0
        || x >= img.width()
        || y >= img.height()
        || width > img.width() - x
        || height > img.height() - y
    {
        return Err("Invalid crop bounds".to_string());
    }
    Ok(img.crop_imm(x, y, width, height))
}

fn parse_upload_response(body: &[u8]) -> Result<Url, String> {
    let data: UploadResponse =
        serde_json::from_slice(body).map_err(|_| "Invalid upload response".to_string())?;
    if !data.success {
        return Err("Upload service reported failure".to_string());
    }
    let file = data
        .files
        .as_ref()
        .and_then(|files| files.first())
        .ok_or_else(|| "Upload response contained no files".to_string())?;
    let image_url = Url::parse(&file.url).map_err(|_| "Invalid uploaded image URL".to_string())?;
    if image_url.scheme() != "https"
        || image_url.host_str().is_none()
        || !image_url.username().is_empty()
        || image_url.password().is_some()
    {
        return Err("Invalid uploaded image URL".to_string());
    }
    let mut search_url = Url::parse(Config::IMAGE_SEARCH_ENGINE_URL)
        .map_err(|_| "Invalid search engine URL".to_string())?;
    search_url
        .query_pairs_mut()
        .append_pair("url", image_url.as_str());
    Ok(search_url)
}

fn read_upload_response(response: Response) -> Result<Url, String> {
    let response = response
        .error_for_status()
        .map_err(|error| match error.status() {
            Some(status) => format!("Upload failed with HTTP status {}", status.as_u16()),
            None => "Upload request failed".to_string(),
        })?;
    if !response.status().is_success() {
        return Err("Upload did not return a successful HTTP status".to_string());
    }
    if response
        .content_length()
        .is_some_and(|length| length > Config::MAX_UPLOAD_RESPONSE_BYTES)
    {
        return Err("Upload response too large".to_string());
    }
    let mut body = Vec::new();
    response
        .take(Config::MAX_UPLOAD_RESPONSE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|_| "Failed to read upload response".to_string())?;
    if body.len() as u64 > Config::MAX_UPLOAD_RESPONSE_BYTES {
        return Err("Upload response too large".to_string());
    }
    parse_upload_response(&body)
}

fn upload_image(client: &Client, endpoint: &str, bytes: Vec<u8>) -> Result<Url, String> {
    let part = multipart::Part::bytes(bytes)
        .file_name("crop.png")
        .mime_str("image/png")
        .map_err(|_| "Failed to prepare upload".to_string())?;
    let response = client
        .post(endpoint)
        .multipart(multipart::Form::new().part("files[]", part))
        .send()
        .map_err(|_| "Upload request failed".to_string())?;
    read_upload_response(response)
}

pub fn search(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    img: &image::DynamicImage,
) -> Result<(), String> {
    let cropped = crop_image(x, y, width, height, img)?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|_| "Failed to encode cropped image".to_string())?;
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .connect_timeout(Config::CONNECT_TIMEOUT)
        .timeout(Config::REQUEST_TIMEOUT)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Failed to initialize upload client".to_string())?;
    let search_url = upload_image(&client, Config::UPLOAD_ENDPOINT, buffer.into_inner())?;
    open_browser(search_url.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn rejects_invalid_crops_before_any_side_effects() {
        let image = image::DynamicImage::new_rgba8(4, 3);
        for (x, y, width, height) in [
            (0, 0, 0, 1),
            (0, 0, 1, 0),
            (4, 0, 1, 1),
            (0, 3, 1, 1),
            (3, 0, 2, 1),
            (0, 2, 1, 2),
            (u32::MAX, 0, 2, 1),
            (0, u32::MAX, 1, 2),
            (1, 0, u32::MAX, 1),
            (0, 1, 1, u32::MAX),
        ] {
            assert_eq!(
                search(x, y, width, height, &image),
                Err("Invalid crop bounds".to_string())
            );
        }
        let empty = image::DynamicImage::new_rgba8(0, 0);
        assert!(crop_image(0, 0, 1, 1, &empty).is_err());
    }

    #[test]
    fn accepts_exact_crop_edges() {
        let image = image::DynamicImage::new_rgba8(4, 3);
        let whole = crop_image(0, 0, 4, 3, &image).unwrap();
        assert_eq!((whole.width(), whole.height()), (4, 3));
        let corner = crop_image(3, 2, 1, 1, &image).unwrap();
        assert_eq!((corner.width(), corner.height()), (1, 1));
    }

    fn upload_body(url: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "success": true,
            "files": [{ "url": url }]
        }))
        .unwrap()
    }

    #[test]
    fn encodes_image_url_as_a_single_query_parameter() {
        let image_url = "https://images.example/crop%20one.png?token=a%2Bb&other=x=y#fragment";
        let url = parse_upload_response(&upload_body(image_url)).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("lens.google.com"));
        assert_eq!(url.path(), "/uploadbyurl");
        assert_eq!(url.fragment(), None);
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("url".into(), image_url.into())]
        );
    }

    #[test]
    fn rejects_invalid_image_urls_without_disclosing_them() {
        for url in [
            "",
            "not a URL",
            "/crop.png",
            "//images.example/crop.png",
            "http://images.example/crop.png",
            "file:///crop.png",
            "javascript:alert(1)",
            "https://",
            "https://user:secret@images.example/crop.png",
        ] {
            assert_eq!(
                parse_upload_response(&upload_body(url)).unwrap_err(),
                "Invalid uploaded image URL"
            );
        }
    }

    #[test]
    fn rejects_upload_errors_without_disclosing_response_bodies() {
        for body in [
            r#"{"success":false,"error":"private server details"}"#,
            r#"{"success":false,"files":[{"url":"https://images.example/private"}]}"#,
            r#"{"success":true}"#,
            r#"{"success":true,"files":null}"#,
            r#"{"success":true,"files":[]}"#,
            r#"{"success":true,"files":[{}]}"#,
            r#"{"success":"private server details"}"#,
            "private server details",
        ] {
            let error = parse_upload_response(body.as_bytes()).unwrap_err();
            assert!(!error.contains("private"));
            assert!(!error.contains("https://"));
        }
    }

    fn mock_upload(response: Vec<u8>, delay: Duration, timeout: Duration) -> Result<Url, String> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/upload", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                headers.push(byte[0]);
                assert!(headers.len() < 8192);
            }
            let headers = String::from_utf8(headers).unwrap().to_ascii_lowercase();
            assert!(headers.starts_with("post /upload http/1.1\r\n"));
            assert!(headers.contains("multipart/form-data"));
            let length: u64 = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            std::io::copy(&mut (&mut stream).take(length), &mut std::io::sink()).unwrap();
            thread::sleep(delay);
            let _ = stream.write_all(&response);
        });
        let client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(1))
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let result = upload_image(&client, &endpoint, vec![0]);
        server.join().unwrap();
        result
    }

    #[test]
    fn handles_local_http_status_and_upload_responses() {
        for (status, body, expected) in [
            (
                "200 OK",
                upload_body("https://images.example/crop.png"),
                None,
            ),
            (
                "500 Internal Server Error",
                b"private server details".to_vec(),
                Some("Upload failed with HTTP status 500"),
            ),
            (
                "302 Found",
                Vec::new(),
                Some("Upload did not return a successful HTTP status"),
            ),
            (
                "200 OK",
                b"{\"success\":false}".to_vec(),
                Some("Upload service reported failure"),
            ),
            (
                "200 OK",
                b"not json".to_vec(),
                Some("Invalid upload response"),
            ),
        ] {
            let mut response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            response.extend(body);
            let result = mock_upload(response, Duration::ZERO, Duration::from_secs(2));
            match expected {
                Some(error) => assert_eq!(result.unwrap_err(), error),
                None => assert!(result.is_ok()),
            }
        }
    }

    #[test]
    fn bounds_responses_with_and_without_content_length() {
        for header in [
            format!(
                "Content-Length: {}\r\n",
                Config::MAX_UPLOAD_RESPONSE_BYTES + 1
            ),
            String::new(),
        ] {
            let mut response =
                format!("HTTP/1.1 200 OK\r\n{header}Connection: close\r\n\r\n").into_bytes();
            response.extend(vec![b' '; Config::MAX_UPLOAD_RESPONSE_BYTES as usize + 1]);
            assert_eq!(
                mock_upload(response, Duration::ZERO, Duration::from_secs(2)).unwrap_err(),
                "Upload response too large"
            );
        }
    }

    #[test]
    fn upload_request_times_out_offline() {
        assert_eq!(
            mock_upload(
                Vec::new(),
                Duration::from_millis(500),
                Duration::from_millis(100)
            )
            .unwrap_err(),
            "Upload request failed"
        );
    }
}
