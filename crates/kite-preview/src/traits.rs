use ratatui::style::Style;
use std::path::Path;

pub trait PreviewProvider: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn can_preview(&self, path: &Path) -> bool;
    fn preview(&self, path: &Path, width: u16, height: u16) -> Result<PreviewContent, PreviewError>;
    fn priority(&self) -> u8 {
        50
    }
}

#[derive(Debug, Clone)]
pub enum PreviewContent {
    StyledText(Vec<StyledLine>),
    AnsiText(String),
    KittyImage {
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: ImageFormat,
    },
    Placeholder(String),
}

#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
}

#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Rgb24,
    Rgba32,
}

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File too large to preview: {size} bytes")]
    TooLarge { size: u64 },
    #[error("Unsupported format")]
    Unsupported,
    #[error("Preview generation failed: {0}")]
    Other(String),
}
