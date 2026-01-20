//! Image loading for markdown rendering.
//!
//! Supports loading images from:
//! - Local file paths
//! - HTTP/HTTPS URLs (async)

use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_IMAGE_WIDTH: i32 = 600;

/// Load an image from a local file path
pub fn load_local_image(path: &str) -> Option<gdk::Texture> {
    let path = Path::new(path);
    if !path.exists() {
        tracing::debug!("Image file not found: {:?}", path);
        return None;
    }

    let file = gio::File::for_path(path);
    match gdk::Texture::from_file(&file) {
        Ok(texture) => {
            tracing::debug!(
                "Loaded local image: {:?} ({}x{})",
                path,
                texture.width(),
                texture.height()
            );
            Some(scale_texture_if_needed(&texture))
        }
        Err(e) => {
            tracing::warn!("Failed to load image {:?}: {}", path, e);
            None
        }
    }
}

/// Load an image from HTTP URL asynchronously and insert it into the buffer
pub fn load_http_image_async(url: String, buffer: gtk4::TextBuffer, offset: i32) {
    tracing::debug!("Starting HTTP image load: {}", url);
    let url_clone = url.clone();

    glib::spawn_future_local(async move {
        tracing::debug!("Fetching HTTP image: {}", url);
        match fetch_image_bytes_async(&url).await {
            Ok(bytes) => {
                if let Some(texture) = load_texture_from_bytes(&bytes) {
                    let scaled = scale_texture_if_needed(&texture);
                    let mut iter = buffer.iter_at_offset(offset);
                    buffer.insert_paintable(&mut iter, &scaled);
                    tracing::debug!(
                        "Inserted HTTP image: {} ({}x{})",
                        url_clone,
                        scaled.width(),
                        scaled.height()
                    );
                } else {
                    tracing::warn!("Failed to parse image from {}", url_clone);
                    let mut iter = buffer.iter_at_offset(offset);
                    buffer.insert(&mut iter, "[Image]");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch image from {}: {}", url, e);
                let mut iter = buffer.iter_at_offset(offset);
                buffer.insert(&mut iter, "[Image]");
            }
        }
    });
}

async fn fetch_image_bytes_async(url: &str) -> Result<Vec<u8>, String> {
    let url = url.to_string();

    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(HTTP_TIMEOUT)
                .build()
                .map_err(|e| e.to_string())?;

            let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
            let bytes = response.bytes().await.map_err(|e| e.to_string())?;
            Ok::<Vec<u8>, String>(bytes.to_vec())
        });

        let _ = tx.send(result);
    });

    rx.await.map_err(|e| e.to_string())?
}

/// Load texture from raw bytes using `GdkTexture` directly
fn load_texture_from_bytes(bytes: &[u8]) -> Option<gdk::Texture> {
    let gbytes = glib::Bytes::from(bytes);
    match gdk::Texture::from_bytes(&gbytes) {
        Ok(texture) => Some(texture),
        Err(e) => {
            tracing::warn!("Failed to decode image: {}", e);
            None
        }
    }
}

/// Scale texture if it exceeds max width, maintaining aspect ratio
// Dimension scaling math uses f64, GTK textures use i32
#[allow(clippy::cast_possible_truncation)]
fn scale_texture_if_needed(texture: &gdk::Texture) -> gdk::Texture {
    let width = texture.width();
    let height = texture.height();

    if width <= MAX_IMAGE_WIDTH {
        return texture.clone();
    }

    let scale = f64::from(MAX_IMAGE_WIDTH) / f64::from(width);
    let new_width = MAX_IMAGE_WIDTH;
    let new_height = (f64::from(height) * scale) as i32;

    scale_texture_with_image_crate(texture, new_width, new_height)
}

/// Scale a texture using the image crate
// Image dimension conversions between i32 (GTK), u32 (image crate), and usize (stride)
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scale_texture_with_image_crate(
    texture: &gdk::Texture,
    new_width: i32,
    new_height: i32,
) -> gdk::Texture {
    let width = texture.width() as u32;
    let height = texture.height() as u32;

    // Download texture to memory (RGBA format)
    let stride = width as usize * 4;
    let mut data = vec![0u8; stride * height as usize];
    texture.download(&mut data, stride);

    // Create image buffer from raw RGBA data
    let img = match image::RgbaImage::from_raw(width, height, data) {
        Some(img) => image::DynamicImage::ImageRgba8(img),
        None => return texture.clone(),
    };

    // Resize using Lanczos3 for quality
    let resized = img.resize(
        new_width as u32,
        new_height as u32,
        image::imageops::FilterType::Lanczos3,
    );

    // Convert back to PNG bytes for Texture::from_bytes
    let mut png_bytes = Vec::new();
    if resized
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .is_err()
    {
        return texture.clone();
    }

    // Create new texture from PNG bytes
    let gbytes = glib::Bytes::from(&png_bytes);
    gdk::Texture::from_bytes(&gbytes).unwrap_or_else(|_| texture.clone())
}

/// Check if a URL is an HTTP/HTTPS URL
pub fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
