pub mod binary;
pub mod directory;
pub mod image;
pub mod text;
pub mod traits;

use std::path::Path;
use traits::{PreviewContent, PreviewError, PreviewProvider};

pub struct PreviewManager {
    providers: Vec<Box<dyn PreviewProvider>>,
}

impl PreviewManager {
    pub fn new() -> Self {
        let providers: Vec<Box<dyn PreviewProvider>> = vec![
            Box::new(text::TextPreviewProvider::new()),
            Box::new(image::KittyImageProvider::new()),
            Box::new(directory::DirectoryPreviewProvider),
            Box::new(binary::BinaryPreviewProvider),
        ];
        Self { providers }
    }

    pub fn add_provider(&mut self, provider: Box<dyn PreviewProvider>) {
        self.providers.push(provider);
        self.providers.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    pub fn preview(&self, path: &Path, width: u16, height: u16) -> Result<PreviewContent, PreviewError> {
        for provider in &self.providers {
            if provider.can_preview(path) {
                return provider.preview(path, width, height);
            }
        }
        Ok(PreviewContent::Placeholder("No preview available".to_string()))
    }
}

impl Default for PreviewManager {
    fn default() -> Self {
        Self::new()
    }
}
