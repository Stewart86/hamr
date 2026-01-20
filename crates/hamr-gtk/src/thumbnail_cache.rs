//! Platform-agnostic image cache with background generation.
//!
//! Provides lazy image resizing for thumbnails and previews, running on a
//! background thread to avoid blocking the UI. Images are cached to disk
//! using MD5 hash of path + mtime + size for cache invalidation.
//!
//! Supports `HiDPI` displays by scaling cache sizes based on display scale factor.
//!
//! Two cache tiers:
//! - `thumbnail_cache()`: Small icons for result lists (128px base)
//! - `preview_cache()`: Larger images for preview panels (512px base)

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use gtk4::glib;
use tracing::{debug, error, warn};

/// Base thumbnail size in pixels (scaled by display factor)
const THUMBNAIL_BASE_SIZE: u32 = 128;

/// Base preview size in pixels (scaled by display factor)
const PREVIEW_BASE_SIZE: u32 = 512;

/// Cache directory names under XDG cache
const THUMBNAIL_CACHE_DIR: &str = "hamr/thumbnails";
const PREVIEW_CACHE_DIR: &str = "hamr/previews";

thread_local! {
    static THUMBNAIL_CACHE: RefCell<Option<ImageCache>> = const { RefCell::new(None) };
    static PREVIEW_CACHE: RefCell<Option<ImageCache>> = const { RefCell::new(None) };
    static SCALE_FACTOR: RefCell<f64> = const { RefCell::new(1.0) };
}

/// Set the display scale factor for cache size calculations.
/// Call this when the display scale changes (e.g., on window map or monitor change).
pub fn set_scale_factor(scale: f64) {
    SCALE_FACTOR.with(|sf| {
        *sf.borrow_mut() = scale.max(1.0);
    });
}

/// Get the current display scale factor.
pub fn get_scale_factor() -> f64 {
    SCALE_FACTOR.with(|sf| *sf.borrow())
}

/// Get the global thumbnail cache instance (128px base, for result list icons).
// Scaled size calculation: f64 * f64 -> u32 for image dimensions
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn thumbnail_cache() -> ImageCache {
    THUMBNAIL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let scale = get_scale_factor();
        let size = (f64::from(THUMBNAIL_BASE_SIZE) * scale) as u32;

        if cache.is_none() {
            *cache = Some(ImageCache::new(THUMBNAIL_CACHE_DIR, size));
        }
        cache.as_ref().unwrap().clone()
    })
}

/// Get the global preview cache instance (512px base, for preview panels).
// Scaled size calculation: f64 * f64 -> u32 for image dimensions
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn preview_cache() -> ImageCache {
    PREVIEW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let scale = get_scale_factor();
        let size = (f64::from(PREVIEW_BASE_SIZE) * scale) as u32;

        if cache.is_none() {
            *cache = Some(ImageCache::new(PREVIEW_CACHE_DIR, size));
        }
        cache.as_ref().unwrap().clone()
    })
}

/// Shared cache state
struct CacheState {
    /// Paths currently being processed (includes size in key)
    pending: HashSet<String>,
    /// Completed images
    ready: HashSet<String>,
}

/// Platform-agnostic image cache with background generation.
///
/// Usage:
/// 1. Call `get()` with an image path
/// 2. If returns `Some(path)`, use the cached image
/// 3. If returns `None`, image is being generated in background
/// 4. The callback is invoked on main thread when ready
#[derive(Clone)]
pub struct ImageCache {
    cache_dir: PathBuf,
    state: Arc<Mutex<CacheState>>,
    target_size: u32,
}

impl ImageCache {
    /// Create a new image cache with the given directory name and target size.
    pub fn new(cache_dir_name: &str, target_size: u32) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(cache_dir_name);

        if let Err(e) = fs::create_dir_all(&cache_dir) {
            warn!("Failed to create image cache dir: {}", e);
        }

        Self {
            cache_dir,
            state: Arc::new(Mutex::new(CacheState {
                pending: HashSet::new(),
                ready: HashSet::new(),
            })),
            target_size,
        }
    }

    /// Get a cached/resized image for the given path.
    ///
    /// Returns:
    /// - `Some(path)` if image is cached and valid
    /// - `None` if image needs generation (queued in background)
    ///
    /// The `on_ready` callback is invoked on the main thread when the image
    /// is ready, receiving the path to the generated cached image.
    pub fn get<F>(&self, path: &Path, on_ready: F) -> Option<PathBuf>
    where
        F: FnOnce(PathBuf) + Send + 'static,
    {
        let cache_key = self.cache_key(path);

        // Check if already cached on disk
        if let Some(cached_path) = self.get_cached(path) {
            return Some(cached_path);
        }

        // Check if already pending
        {
            let state = self.state.lock().unwrap();
            if state.pending.contains(&cache_key) {
                return None;
            }
        }

        // Mark as pending and spawn background generation
        {
            let mut state = self.state.lock().unwrap();
            state.pending.insert(cache_key.clone());
        }

        self.spawn_generation(path.to_path_buf(), on_ready);
        None
    }

    /// Create a unique cache key for path + size combination.
    fn cache_key(&self, path: &Path) -> String {
        format!("{}:{}", path.to_string_lossy(), self.target_size)
    }

    /// Check if a cached image exists and is valid.
    fn get_cached(&self, path: &Path) -> Option<PathBuf> {
        let cached_path = self.cached_path(path);

        if !cached_path.exists() {
            return None;
        }

        // Validate cache: check if source file was modified after cached version
        if let (Ok(src_meta), Ok(cached_meta)) = (path.metadata(), cached_path.metadata()) {
            let src_mtime = src_meta.modified().ok()?;
            let cached_mtime = cached_meta.modified().ok()?;

            if src_mtime > cached_mtime {
                debug!("Image cache stale for {:?}", path);
                return None;
            }
        }

        Some(cached_path)
    }

    /// Compute the cached image path for a source image.
    fn cached_path(&self, path: &Path) -> PathBuf {
        let mtime = path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());

        // Include target size in hash so different cache tiers don't collide
        let cache_key = format!("{}:{}:{}", path.to_string_lossy(), mtime, self.target_size);
        let hash = md5_hash(&cache_key);

        self.cache_dir.join(format!("{hash}.png"))
    }

    /// Spawn background image generation.
    fn spawn_generation<F>(&self, path: PathBuf, on_ready: F)
    where
        F: FnOnce(PathBuf) + Send + 'static,
    {
        let cache = self.clone();
        let target_size = self.target_size;

        gtk4::gio::spawn_blocking(move || {
            let result = generate_resized_image(&path, &cache.cache_dir, target_size);

            let cache_key = cache.cache_key(&path);

            // Update state
            {
                let mut state = cache.state.lock().unwrap();
                state.pending.remove(&cache_key);
                if result.is_some() {
                    state.ready.insert(cache_key);
                }
            }

            // Notify on main thread with the cached image path
            if let Some(cached_path) = result {
                glib::idle_add_once(move || {
                    on_ready(cached_path);
                });
            }
        });
    }
}

/// Generate a resized image for the given path.
fn generate_resized_image(src_path: &Path, cache_dir: &Path, size: u32) -> Option<PathBuf> {
    use image::GenericImageView;
    use image::imageops::FilterType;

    debug!(
        "Generating cached image for {:?} at size {}",
        src_path, size
    );

    let img = match image::open(src_path) {
        Ok(img) => img,
        Err(e) => {
            error!("Failed to open image {:?}: {}", src_path, e);
            return None;
        }
    };

    // Only resize if larger than target
    let (width, height) = img.dimensions();
    let resized = if width > size || height > size {
        // Use Lanczos3 for higher quality when downscaling
        img.resize(size, size, FilterType::Lanczos3)
    } else {
        img
    };

    // Compute output path (include size in hash)
    let mtime = src_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());

    let cache_key = format!("{}:{}:{}", src_path.to_string_lossy(), mtime, size);
    let hash = md5_hash(&cache_key);
    let cached_path = cache_dir.join(format!("{hash}.png"));

    if let Err(e) = resized.save(&cached_path) {
        error!("Failed to save cached image {:?}: {}", cached_path, e);
        return None;
    }

    debug!("Generated cached image: {:?}", cached_path);
    Some(cached_path)
}

/// Compute MD5 hash of a string, returning hex string.
fn md5_hash(input: &str) -> String {
    use std::fmt::Write;

    let digest = md5_digest(input.as_bytes());

    let mut result = String::with_capacity(32);
    for byte in digest {
        write!(result, "{byte:02x}").unwrap();
    }
    result
}

/// MD5 digest computation.
// Standard algorithm implementation - splitting would fragment cohesive cryptographic rounds
#[allow(clippy::many_single_char_names, clippy::too_many_lines)]
fn md5_digest(input: &[u8]) -> [u8; 16] {
    const K: [u32; 64] = [
        0xd76a_a478,
        0xe8c7_b756,
        0x2420_70db,
        0xc1bd_ceee,
        0xf57c_0faf,
        0x4787_c62a,
        0xa830_4613,
        0xfd46_9501,
        0x6980_98d8,
        0x8b44_f7af,
        0xffff_5bb1,
        0x895c_d7be,
        0x6b90_1122,
        0xfd98_7193,
        0xa679_438e,
        0x49b4_0821,
        0xf61e_2562,
        0xc040_b340,
        0x265e_5a51,
        0xe9b6_c7aa,
        0xd62f_105d,
        0x0244_1453,
        0xd8a1_e681,
        0xe7d3_fbc8,
        0x21e1_cde6,
        0xc337_07d6,
        0xf4d5_0d87,
        0x455a_14ed,
        0xa9e3_e905,
        0xfcef_a3f8,
        0x676f_02d9,
        0x8d2a_4c8a,
        0xfffa_3942,
        0x8771_f681,
        0x6d9d_6122,
        0xfde5_380c,
        0xa4be_ea44,
        0x4bde_cfa9,
        0xf6bb_4b60,
        0xbebf_bc70,
        0x289b_7ec6,
        0xeaa1_27fa,
        0xd4ef_3085,
        0x0488_1d05,
        0xd9d4_d039,
        0xe6db_99e5,
        0x1fa2_7cf8,
        0xc4ac_5665,
        0xf429_2244,
        0x432a_ff97,
        0xab94_23a7,
        0xfc93_a039,
        0x655b_59c3,
        0x8f0c_cc92,
        0xffef_f47d,
        0x8584_5dd1,
        0x6fa8_7e4f,
        0xfe2c_e6e0,
        0xa301_4314,
        0x4e08_11a1,
        0xf753_7e82,
        0xbd3a_f235,
        0x2ad7_d2bb,
        0xeb86_d391,
    ];

    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    let mut h0: u32 = 0x6745_2301;
    let mut h1: u32 = 0xefcd_ab89;
    let mut h2: u32 = 0x98ba_dcfe;
    let mut h3: u32 = 0x1032_5476;

    let mut msg = input.to_vec();
    let original_len_bits = (msg.len() as u64) * 8;

    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }

    msg.extend_from_slice(&original_len_bits.to_le_bytes());

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, bytes) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])).rotate_left(S[i]),
            );
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
    }

    let mut result = [0u8; 16];
    result[0..4].copy_from_slice(&h0.to_le_bytes());
    result[4..8].copy_from_slice(&h1.to_le_bytes());
    result[8..12].copy_from_slice(&h2.to_le_bytes());
    result[12..16].copy_from_slice(&h3.to_le_bytes());
    result
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod tests {
    use super::*;

    #[test]
    fn test_md5_hash() {
        assert_eq!(md5_hash(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hash("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_scale_factor() {
        set_scale_factor(2.0);
        assert_eq!(get_scale_factor(), 2.0);

        // Scale factor should not go below 1.0
        set_scale_factor(0.5);
        assert_eq!(get_scale_factor(), 1.0);
    }
}
