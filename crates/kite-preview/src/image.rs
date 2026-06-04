use crate::traits::{ImageFormat, PreviewContent, PreviewError, PreviewProvider};
use base64::Engine;
use image::GenericImageView;
use std::path::Path;

pub struct KittyImageProvider {
    supported: bool,
}

impl KittyImageProvider {
    pub fn new() -> Self {
        let supported = detect_kitty_support();
        Self { supported }
    }

    pub fn force_enabled() -> Self {
        Self { supported: true }
    }
}

impl PreviewProvider for KittyImageProvider {
    fn supported_extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "ico"]
    }

    fn can_preview(&self, path: &Path) -> bool {
        if !self.supported || !path.is_file() {
            return false;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        self.supported_extensions().contains(&ext.as_str())
    }

    fn preview(&self, path: &Path, width: u16, height: u16) -> Result<PreviewContent, PreviewError> {
        let img = image::open(path)
            .map_err(|e| PreviewError::Other(format!("Failed to open image: {}", e)))?;

        let (_img_w, _img_h) = img.dimensions();

        // Calculate target size in pixels (approximate: 8px per cell width, 16px per cell height)
        let target_w = (width as u32) * 8;
        let target_h = (height as u32) * 16;

        let resized = img.resize(target_w, target_h, image::imageops::FilterType::Triangle);
        let (final_w, final_h) = resized.dimensions();

        let rgba = resized.to_rgba8();
        let raw_data = rgba.into_raw();

        Ok(PreviewContent::KittyImage {
            data: raw_data,
            width: final_w,
            height: final_h,
            format: ImageFormat::Rgba32,
        })
    }

    fn priority(&self) -> u8 {
        70
    }
}

fn detect_kitty_support() -> bool {
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        if term.to_lowercase().contains("kitty") || term.to_lowercase().contains("wezterm") {
            return true;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("kitty") {
            return true;
        }
    }
    // Check KITTY_WINDOW_ID
    std::env::var("KITTY_WINDOW_ID").is_ok()
}

/// Encode image data using Kitty graphics protocol escape sequences.
/// Returns the full escape sequence string to write to the terminal.
pub fn encode_kitty_image(data: &[u8], width: u32, height: u32, format: ImageFormat) -> String {
    let fmt_char = match format {
        ImageFormat::Rgba32 => 32,
        ImageFormat::Rgb24 => 24,
        ImageFormat::Png => 100,
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    let mut output = String::new();

    // Kitty protocol: chunk data into 4096-byte pieces
    let chunk_size = 4096;
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        let more = if is_last { 0 } else { 1 };

        if i == 0 {
            output.push_str(&format!(
                "\x1b_Ga=T,f={},s={},v={},m={};{}\x1b\\",
                fmt_char, width, height, more, chunk
            ));
        } else {
            output.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
        }
    }

    output
}

/// Generate the escape sequence to clear a previously displayed kitty image
pub fn clear_kitty_image() -> &'static str {
    "\x1b_Ga=d;\x1b\\"
}
